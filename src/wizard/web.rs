//! # Local browser setup
//!
//! Renders the shared wizard as authenticated, ephemeral local Web forms.
//! Existing config contents and generated credential values stay server-side.

pub mod draft;
pub mod form;
pub mod http;
pub mod keychain;
pub mod settings;
mod settings_api;
#[cfg(test)]
mod tests;
pub mod worker;

use std::{
    io::{Write, stderr},
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::{Result, anyhow, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use crate::wizard::{
    configure::SetupTarget,
    web::{
        draft::{Credential, Draft},
        keychain::CreatedItem,
    },
};

/// The platform boundary, replaced by isolated fakes in adversarial tests.
pub trait Services: Send + Sync + 'static {
    /// Reads one explicitly named credential without exposing it to the UI.
    fn read(&self, entry: &str) -> Result<Zeroizing<String>>;
    /// Authenticates both fixed endpoints without fetching or sending mail.
    fn check(&self, draft: &Draft, password: &str, cancelled: &dyn Fn() -> bool) -> Result<()>;
    /// Creates a new credential with an exact rollback capability.
    fn create(&self, entry: &str, password: &str) -> Result<CreatedItem>;
}

/// The production native-Keychain and bounded-provider adapter.
pub struct SystemServices;

impl Services for SystemServices {
    fn read(&self, entry: &str) -> Result<Zeroizing<String>> {
        keychain::read_bounded(entry)
    }
    fn check(&self, draft: &Draft, password: &str, cancelled: &dyn Fn() -> bool) -> Result<()> {
        worker::check(draft, password, cancelled)
    }
    fn create(&self, entry: &str, password: &str) -> Result<CreatedItem> {
        keychain::create_bounded(entry, password)
    }
}

/// The authenticated lifetime and serialized account-setup state.
pub struct Session<E: Services> {
    /// Exact listener host and port, never an arbitrary DNS hostname.
    pub authority: String,
    /// The launch-specific bearer capability, never included in API responses.
    pub token: Zeroizing<String>,
    /// Whether finish/cancel or process shutdown has revoked the capability.
    pub closed: AtomicBool,
    started: Instant,
    last_activity: Mutex<Instant>,
    flow: Mutex<Flow>,
    commit: Mutex<()>,
    operations: Mutex<usize>,
    settled: Condvar,
    recovery: Mutex<Vec<String>>,
    services: E,
    idle_limit: Duration,
    absolute_limit: Duration,
}

struct Flow {
    target: SetupTarget,
    verified: Option<Verified>,
    saved: Option<String>,
    wizard: Option<form::FormProcess>,
    ready: Option<form::Ready>,
    wizard_error: Option<String>,
    generation: String,
    settings_task: Option<settings_api::Task>,
    settings_ready: Option<settings_api::Ready>,
    settings_outcome: Option<(String, String, Value)>,
}

struct Verified {
    ticket: String,
    draft: Draft,
    password: Zeroizing<String>,
    entry: String,
    document: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveRequest {
    ticket: String,
}

impl<E: Services> Session<E> {
    /// Constructs a session around a prevalidated, fixed configuration target.
    pub(super) fn new(authority: String, target: SetupTarget, services: E) -> Result<Self> {
        Ok(Self {
            authority,
            token: Zeroizing::new(random_id()?),
            closed: AtomicBool::new(false),
            started: Instant::now(),
            last_activity: Mutex::new(Instant::now()),
            flow: Mutex::new(Flow {
                target,
                verified: None,
                saved: None,
                wizard: None,
                ready: None,
                wizard_error: None,
                generation: String::new(),
                settings_task: None,
                settings_ready: None,
                settings_outcome: None,
            }),
            commit: Mutex::new(()),
            operations: Mutex::new(0),
            settled: Condvar::new(),
            recovery: Mutex::new(Vec::new()),
            services,
            idle_limit: Duration::from_secs(600),
            absolute_limit: Duration::from_secs(1800),
        })
    }

    /// Checks both expiry limits without allowing unauthenticated keep-alives.
    pub fn live(&self) -> bool {
        !self.closed.load(Ordering::SeqCst)
            && self.started.elapsed() < self.absolute_limit
            && self
                .last_activity
                .lock()
                .is_ok_and(|last| last.elapsed() < self.idle_limit)
    }

    /// Verifies a capability without a timing-dependent byte comparison.
    pub fn authorized(&self, supplied: &str) -> bool {
        self.live() && bool::from(self.token.as_bytes().ct_eq(supplied.as_bytes()))
    }

    /// Ends a stalled wizard independently of browser polling.
    pub fn watchdog(&self) {
        if let Ok(mut flow) = self.flow.try_lock()
            && flow
                .settings_task
                .as_ref()
                .is_some_and(|t| t.worker.expired())
        {
            flow.settings_task = None;
            flow.wizard_error = Some(
                "設定確認がタイムアウトしました。入力内容を確認して再試行してください。".into(),
            );
        }
        if let Ok(mut flow) = self.flow.try_lock()
            && flow.wizard.as_ref().is_some_and(form::FormProcess::expired)
        {
            flow.wizard = None;
            flow.wizard_error =
                Some("設定処理がタイムアウトしました。最初からやり直してください。".into());
        }
    }

    /// Revokes only after any already-committed save has a known outcome.
    pub fn revoke(&self) {
        let _commit = self
            .commit
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        self.closed.store(true, Ordering::SeqCst);
    }

    /// Retains ownership of queued/running work even if HTTP is cancelled.
    pub fn begin_work(self: &Arc<Self>) -> Work<E> {
        *self
            .operations
            .lock()
            .unwrap_or_else(|error| error.into_inner()) += 1;
        Work {
            session: self.clone(),
        }
    }

    /// Waits for owned work and makes recovery warnings survive socket closure.
    pub fn settle(&self) -> Result<()> {
        if let Ok(mut flow) = self.flow.lock() {
            flow.wizard = None;
            flow.ready = None;
            flow.settings_task = None;
            flow.settings_ready = None;
        }
        let mut active = self
            .operations
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        while *active != 0 {
            active = self
                .settled
                .wait(active)
                .unwrap_or_else(|error| error.into_inner());
        }
        let warnings = self
            .recovery
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if !warnings.is_empty() {
            bail!(
                "Setup ended with a recovery warning; inspect the non-secret Keychain item information in the terminal before retrying"
            );
        }
        if let Ok(flow) = self.flow.lock()
            && let Some(name) = &flow.saved
        {
            let _ = writeln!(stderr(), "Account `{name}` saved; local setup stopped");
        }
        Ok(())
    }

    fn recovery_warning(&self, entry: &str, condition: &str) {
        let warning =
            format!("Setup recovery: {condition}; Keychain item: {entry}. No password is printed");
        self.recovery
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(warning.clone());
        let _ = writeln!(stderr(), "{warning}");
    }

    /// Executes an authenticated route, serializing all credential side effects.
    pub fn dispatch(&self, method: &str, path: &str, bytes: &[u8]) -> Result<Value> {
        if path.starts_with("/api/settings/") {
            return self.settings_dispatch(method, path, bytes);
        }
        if !self.live() {
            bail!("Session expired; restart browser setup");
        }
        let mut flow = self
            .flow
            .try_lock()
            .map_err(|_| anyhow!("Another setup operation is running"))?;
        if !self.live() {
            bail!("Session expired; restart browser setup");
        }
        if (flow.settings_task.is_some()
            || flow.settings_ready.is_some()
            || flow
                .settings_outcome
                .as_ref()
                .is_some_and(|(_, _, outcome)| outcome["state"] == "recovery_required"))
            && path != "/api/status"
            && path != "/api/finish"
        {
            bail!("Cancel the current settings operation first");
        }
        if !path.starts_with("/api/wizard/") && path != "/api/status" {
            *self
                .last_activity
                .lock()
                .map_err(|_| anyhow!("Session unavailable"))? = Instant::now();
        }
        match (method, path) {
            ("GET", "/api/status") => Ok(
                json!({"accounts": flow.target.names(), "saved": flow.saved,"backends":settings::backends(),"sources":settings::sources(),"legacy_mprg":settings::legacy_metadata()}),
            ),
            ("POST", "/api/finish") => {
                let _: Empty =
                    serde_json::from_slice(bytes).map_err(|_| anyhow!("Invalid finish request"))?;
                flow.verified = None;
                flow.ready = None;
                flow.wizard = None;
                flow.settings_task = None;
                flow.settings_ready = None;
                self.revoke();
                Ok(json!({"closed": true}))
            }
            ("POST", "/api/wizard/start") => {
                let _: Empty =
                    serde_json::from_slice(bytes).map_err(|_| anyhow!("Invalid start request"))?;
                if flow.wizard.is_some() || flow.ready.is_some() {
                    bail!("Finish or cancel the current account first");
                }
                if flow.saved.is_some() {
                    flow.target
                        .refresh()
                        .map_err(|_| anyhow!("Cannot safely refresh the saved configuration"))?;
                }
                flow.verified = None;
                flow.wizard_error = None;
                flow.generation = random_id()?;
                flow.wizard = Some(form::FormProcess::start()?);
                *self
                    .last_activity
                    .lock()
                    .map_err(|_| anyhow!("Session unavailable"))? = Instant::now();
                Ok(json!({"state":"working", "generation":flow.generation}))
            }
            ("POST", "/api/wizard/poll") => {
                let _: Empty =
                    serde_json::from_slice(bytes).map_err(|_| anyhow!("Invalid poll request"))?;
                if let Some(ready) = &flow.ready {
                    return Ok(
                        json!({"state":"ready", "ticket":ready.ticket, "name":ready.name, "preview":ready.preview}),
                    );
                }
                if let Some(error) = &flow.wizard_error {
                    bail!("{error}");
                }
                let Some(worker) = flow.wizard.as_mut() else {
                    return Ok(json!({"state":"idle"}));
                };
                match worker.poll() {
                    Ok(form::Event::Data(_)) => {
                        flow.wizard = None;
                        bail!("Unexpected worker response");
                    }
                    Ok(form::Event::Pending) => Ok(json!({"state":"working"})),
                    Ok(form::Event::Question(question)) => Ok(
                        json!({"state":"question", "generation":flow.generation, "question":question}),
                    ),
                    Ok(form::Event::Complete(base, generated)) => {
                        flow.wizard = None;
                        let names = flow.target.names();
                        let mut name = base.clone();
                        let mut index = 2;
                        while names.contains(&name) {
                            name = format!("{base}-{index}");
                            index += 1;
                        }
                        let document = Zeroizing::new(
                            flow.target
                                .render_generated(&name, &base, &generated)
                                .map_err(|_| anyhow!("Cannot render the new account"))?,
                        );
                        let preview = form::preview(&document)
                            .map_err(|_| anyhow!("Cannot render a safe preview"))?;
                        let ticket = random_id()?;
                        let result = json!({"state":"ready", "ticket":ticket, "name":name, "preview":preview});
                        flow.ready = Some(form::Ready {
                            ticket,
                            name,
                            document,
                            preview,
                        });
                        Ok(result)
                    }
                    Err(error) => {
                        flow.wizard = None;
                        flow.wizard_error = Some(error.to_string());
                        Err(error)
                    }
                }
            }
            ("POST", "/api/wizard/answer") => {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Submission {
                    generation: String,
                    id: u64,
                    value: Zeroizing<String>,
                }
                let answer: Submission = serde_json::from_slice(bytes)
                    .map_err(|_| anyhow!("Invalid form submission"))?;
                if answer.generation != flow.generation {
                    bail!("This wizard has been replaced; restart the form");
                }
                let payload = Zeroizing::new(serde_json::to_vec(
                    &json!({"id":answer.id, "value":answer.value.as_str()}),
                )?);
                let worker = flow
                    .wizard
                    .as_mut()
                    .ok_or_else(|| anyhow!("No wizard is running"))?;
                worker.answer(&payload)?;
                *self
                    .last_activity
                    .lock()
                    .map_err(|_| anyhow!("Session unavailable"))? = Instant::now();
                Ok(json!({"state":"working"}))
            }
            ("POST", "/api/wizard/cancel") => {
                let _: Empty =
                    serde_json::from_slice(bytes).map_err(|_| anyhow!("Invalid cancel request"))?;
                flow.wizard = None;
                flow.ready = None;
                flow.wizard_error = None;
                flow.generation = random_id()?;
                *self
                    .last_activity
                    .lock()
                    .map_err(|_| anyhow!("Session unavailable"))? = Instant::now();
                Ok(json!({"state":"idle"}))
            }
            ("POST", "/api/wizard/save") => {
                let request: SaveRequest =
                    serde_json::from_slice(bytes).map_err(|_| anyhow!("Invalid save request"))?;
                let ready = flow
                    .ready
                    .as_ref()
                    .ok_or_else(|| anyhow!("Complete the connection checks before saving"))?;
                if !bool::from(ready.ticket.as_bytes().ct_eq(request.ticket.as_bytes())) {
                    bail!("This save ticket is stale");
                }
                let ready = flow.ready.take().expect("verified account");
                let mut may_have_written = false;
                let result = if let Ok(_commit) = self.commit.lock() {
                    flow.target
                        .save(&ready.document, &mut may_have_written, &|| {
                            if self.live() {
                                Ok(())
                            } else {
                                bail!("Session expired")
                            }
                        })
                } else {
                    Err(anyhow!("Session unavailable"))
                };
                if result.is_err() {
                    if may_have_written {
                        self.recovery_warning("no new Keychain item", "configuration write may be partial; inspect the target before retrying");
                        bail!(
                            "Configuration write may be partial; inspect the target before retrying"
                        );
                    }
                    bail!(
                        "Configuration was not written; check for changed contents, permissions or ACLs"
                    );
                }
                flow.saved = Some(ready.name.clone());
                *self
                    .last_activity
                    .lock()
                    .map_err(|_| anyhow!("Session unavailable"))? = Instant::now();
                Ok(json!({"state":"saved", "saved":ready.name}))
            }
            ("POST", "/api/test") => {
                if flow.wizard.is_some() || flow.ready.is_some() {
                    bail!("A form wizard is already running");
                }
                if flow.saved.is_some() {
                    bail!("Account already saved; finish and restart to add another");
                }
                flow.verified = None;
                let mut draft: Draft =
                    serde_json::from_slice(bytes).map_err(|_| anyhow!("Invalid account form"))?;
                draft.validate()?;
                let entry = match &draft.credential {
                    Credential::New { .. } => format!("himalaya-web-{}", random_id()?),
                    Credential::Existing { entry } => entry.clone(),
                };
                let document = flow
                    .target
                    .render(&draft.name, draft.account(&entry))
                    .map_err(|_| anyhow!("Account name exists or draft cannot be rendered"))?;
                if !self.live() {
                    bail!("Session expired; restart browser setup");
                }
                let password = match &draft.credential {
                    Credential::New { password } => password.clone(),
                    Credential::Existing { entry } => self.services.read(entry)?,
                };
                if password.len() != 16 || !password.bytes().all(|b| b.is_ascii_alphanumeric()) {
                    bail!(
                        "Keychain app password must contain exactly 16 ASCII letters or numbers without separators"
                    );
                }
                if !self.live() {
                    bail!("Session expired; restart browser setup");
                }
                self.services.check(&draft, &password, &|| !self.live())?;
                if !self.live() {
                    bail!("Session expired; restart browser setup");
                }
                let ticket = random_id()?;
                let result = json!({"ok": true, "ticket": ticket, "preview": document});
                // NOTE: retain only one secret copy for the checked draft.
                draft.credential = match draft.credential {
                    Credential::New { .. } => Credential::New {
                        password: Zeroizing::new(String::new()),
                    },
                    existing => existing,
                };
                flow.verified = Some(Verified {
                    ticket,
                    draft,
                    password,
                    entry,
                    document,
                });
                Ok(result)
            }
            ("POST", "/api/save") => {
                let request: SaveRequest =
                    serde_json::from_slice(bytes).map_err(|_| anyhow!("Invalid save request"))?;
                let verified = flow
                    .verified
                    .as_ref()
                    .ok_or_else(|| anyhow!("Test the account before saving"))?;
                if !bool::from(verified.ticket.as_bytes().ct_eq(request.ticket.as_bytes())) {
                    bail!("Account check is stale; test again");
                }
                let verified = flow.verified.take().expect("checked draft");
                if !self.live() {
                    bail!("Session expired; restart browser setup");
                }
                let created = match &verified.draft.credential {
                    Credential::New { .. } => {
                        match self.services.create(&verified.entry, &verified.password) {
                            Ok(item) => Some(item),
                            Err(error) => {
                                self.recovery_warning(&verified.entry, "credential creation failed or timed out; creation outcome may be uncertain; configuration was not written");
                                return Err(error);
                            }
                        }
                    }
                    Credential::Existing { entry } => {
                        let current = self.services.read(entry)?;
                        if !bool::from(current.as_bytes().ct_eq(verified.password.as_bytes())) {
                            bail!("Keychain item changed after the check; test again");
                        }
                        None
                    }
                };
                let mut may_have_written = false;
                let result = if let Ok(_commit) = self.commit.lock() {
                    flow.target
                        .save(&verified.document, &mut may_have_written, &|| {
                            if self.live() {
                                Ok(())
                            } else {
                                Err(anyhow!("Session expired"))
                            }
                        })
                } else {
                    Err(anyhow!("Session expired"))
                };
                if result.is_err() {
                    if may_have_written {
                        self.recovery_warning(&verified.entry, "configuration write may be partial; credential retained for inspection");
                        bail!(
                            "Configuration write may be partial; the new Keychain item was retained for recovery; inspect the config before retrying"
                        );
                    }
                    if let Some(item) = created
                        && (item.rollback)().is_err()
                    {
                        self.recovery_warning(
                            &verified.entry,
                            "configuration was not written but owned-item cleanup failed",
                        );
                        bail!(
                            "Configuration was not written; newly created Keychain item cleanup failed; inspect Keychain before retrying"
                        );
                    }
                    bail!(
                        "Configuration was not written; it changed or failed the permission/ACL checks; test again after checking the target"
                    );
                }
                flow.saved = Some(verified.draft.name.clone());
                Ok(json!({"saved": verified.draft.name}))
            }
            _ => bail!("Unsupported setup route"),
        }
    }
}

/// Keeps a blocking operation owned until its result and recovery state settle.
pub struct Work<E: Services> {
    session: Arc<Session<E>>,
}

impl<E: Services> Drop for Work<E> {
    fn drop(&mut self) {
        let mut active = self
            .session
            .operations
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        *active -= 1;
        self.session.settled.notify_all();
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Empty {}

/// Generates an OS-random 256-bit hexadecimal capability or item identifier.
pub fn random_id() -> Result<String> {
    let mut bytes = Zeroizing::new([0u8; 32]);
    getrandom::fill(bytes.as_mut()).map_err(|_| anyhow!("Secure randomness unavailable"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// Opens the browser and serves only one short-lived, explicitly requested UI.
pub fn execute(paths: &[PathBuf]) -> Result<()> {
    let target = SetupTarget::open(paths).map_err(|_| {
        anyhow!("Cannot safely read the setup configuration; check its path, format and file type")
    })?;
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?
        .block_on(async move {
            let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
            let authority = listener.local_addr()?.to_string();
            let session = Arc::new(Session::new(authority, target, SystemServices)?);
            let bootstrap = Zeroizing::new(format!(
                "http://{}/#{}",
                session.authority,
                session.token.as_str()
            ));
            open::that(bootstrap.as_str()).map_err(|_| {
                anyhow!("Cannot open the setup browser; no setup session was left running")
            })?;
            eprintln!("Local account setup opened; finish in the browser or press Ctrl-C to stop");
            http::serve(listener, session).await
        })
}
