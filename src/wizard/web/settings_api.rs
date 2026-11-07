//! Authenticated asynchronous settings operations and protected save tickets.

use crate::wizard::web::{
    Flow, Services, Session,
    form::{self, Event, FormProcess},
    keychain::CreatedItem,
    random_id,
    settings::{self, Draft, Operation},
};
use anyhow::{Result, anyhow, bail};
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::Instant;
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

pub(super) struct Task {
    pub worker: FormProcess,
    revision: String,
    pending: Option<Ready>,
}
pub(super) struct Ready {
    ticket: String,
    name: String,
    revision: String,
    document: Zeroizing<String>,
    preview: String,
    secrets: Vec<(String, Zeroizing<String>)>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiscoverRequest {
    email: String,
    revision: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckRequest {
    draft: Draft,
    revision: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PollRequest {
    generation: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommitRequest {
    ticket: String,
    revision: String,
}

fn revision_valid(revision: &str) -> bool {
    !revision.is_empty()
        && revision.len() <= 64
        && revision
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
}

impl<E: Services> Session<E> {
    pub(super) fn settings_dispatch(
        &self,
        method: &str,
        path: &str,
        bytes: &[u8],
    ) -> Result<Value> {
        if method != "POST" || !self.live() {
            bail!("Session expired or invalid request");
        }
        let mut flow = self
            .flow
            .try_lock()
            .map_err(|_| anyhow!("Another settings operation is running"))?;
        if !self.live() {
            bail!("Session expired");
        }
        if path != "/api/settings/result"
            && flow
                .settings_outcome
                .as_ref()
                .is_some_and(|(_, _, outcome)| outcome["state"] == "recovery_required")
        {
            bail!(
                "復旧確認が必要です。再実行せず、起動元のターミナルと設定ファイルを確認してください。"
            );
        }
        match path {
            "/api/settings/discover" => {
                let request: DiscoverRequest = serde_json::from_slice(bytes)
                    .map_err(|_| anyhow!("Invalid discovery request"))?;
                if !revision_valid(&request.revision) {
                    bail!("Invalid draft revision");
                }
                if flow.settings_task.is_some() || flow.wizard.is_some() {
                    bail!("Cancel the previous operation first");
                }
                flow.settings_ready = None;
                flow.ready = None;
                flow.verified = None;
                flow.wizard_error = None;
                if !settings::valid_email(&request.email) {
                    return Ok(
                        json!({"state":"invalid","field":"email","error":"メールアドレスを入力してください。"}),
                    );
                }
                let payload = Zeroizing::new(serde_json::to_vec(&Operation::Discover {
                    email: request.email,
                })?);
                flow.generation = random_id()?;
                flow.settings_task = Some(Task {
                    worker: FormProcess::settings(&payload)?,
                    revision: request.revision,
                    pending: None,
                });
                self.settings_activity()?;
                Ok(json!({"state":"working","generation":flow.generation}))
            }
            "/api/settings/check" => {
                let request: CheckRequest =
                    serde_json::from_slice(bytes).map_err(|_| anyhow!("Invalid settings draft"))?;
                if !revision_valid(&request.revision) {
                    bail!("Invalid draft revision");
                }
                if flow.settings_task.is_some() || flow.wizard.is_some() {
                    bail!("Cancel the previous operation first");
                }
                flow.settings_ready = None;
                flow.ready = None;
                flow.verified = None;
                flow.wizard_error = None;
                if let Err(error) = request.draft.validate() {
                    return Ok(json!({"state":"invalid","field":error.field,"error":error.error}));
                }
                if flow.saved.is_some() {
                    flow.target
                        .refresh()
                        .map_err(|_| anyhow!("Cannot safely refresh the configuration"))?;
                }
                if flow.target.names().contains(&request.draft.name) {
                    return Ok(
                        json!({"state":"invalid","field":"name","error":"このアカウント名は既に登録されています。別の名前を指定してください。"}),
                    );
                }
                let entries: Vec<String> = request
                    .draft
                    .active_credentials()
                    .iter()
                    .map(|_| random_id().map(|id| format!("himalaya-web-{id}")))
                    .collect::<Result<_>>()?;
                let secrets = request
                    .draft
                    .active_credentials()
                    .into_iter()
                    .zip(&entries)
                    .filter(|(credential, _)| credential.source == "new")
                    .map(|(credential, entry)| (entry.clone(), credential.value.clone()))
                    .collect();
                let document = Zeroizing::new(
                    flow.target
                        .render(&request.draft.name, request.draft.account(&entries, false))
                        .map_err(|_| anyhow!("Cannot render the new account"))?,
                );
                let preview = form::preview(&document)
                    .map_err(|_| anyhow!("Cannot produce a private preview"))?;
                let ready = Ready {
                    ticket: random_id()?,
                    name: request.draft.name.clone(),
                    revision: request.revision.clone(),
                    document,
                    preview,
                    secrets,
                };
                let payload = Zeroizing::new(serde_json::to_vec(&Operation::Check {
                    draft: Box::new(request.draft),
                    entries,
                })?);
                flow.generation = random_id()?;
                flow.settings_task = Some(Task {
                    worker: FormProcess::settings(&payload)?,
                    revision: request.revision,
                    pending: Some(ready),
                });
                self.settings_activity()?;
                Ok(json!({"state":"working","generation":flow.generation}))
            }
            "/api/settings/poll" => {
                let request: PollRequest =
                    serde_json::from_slice(bytes).map_err(|_| anyhow!("Invalid poll request"))?;
                if request.generation != flow.generation {
                    return Ok(json!({"state":"stale"}));
                }
                if let Some(ready) = &flow.settings_ready {
                    return Ok(
                        json!({"state":"ready","ticket":ready.ticket,"revision":ready.revision,"name":ready.name,"preview":ready.preview}),
                    );
                }
                if let Some(error) = &flow.wizard_error {
                    return Ok(json!({"state":"invalid","field":"connection","error":error}));
                }
                let Some(task) = flow.settings_task.as_mut() else {
                    return Ok(json!({"state":"idle"}));
                };
                match task.worker.poll() {
                    Ok(Event::Pending) => Ok(json!({"state":"working"})),
                    Ok(Event::Data(data)) => {
                        let mut task = flow.settings_task.take().expect("active task");
                        if task.pending.is_none() {
                            return Ok(
                                json!({"state":"discovered","revision":task.revision,"result":data}),
                            );
                        }
                        if data["ok"] == true {
                            let ready = task.pending.take().expect("checked draft");
                            let result = json!({"state":"ready","ticket":ready.ticket,"revision":ready.revision,"name":ready.name,"preview":ready.preview});
                            flow.settings_ready = Some(ready);
                            Ok(result)
                        } else {
                            Ok(
                                json!({"state":"invalid","revision":task.revision,"field":data["field"].as_str().unwrap_or("connection"),"error":data["error"].as_str().unwrap_or("接続確認に失敗しました。設定を修正して再試行してください。")}),
                            )
                        }
                    }
                    _ => {
                        flow.settings_task = None;
                        Ok(
                            json!({"state":"invalid","field":"connection","error":"設定確認に失敗しました。入力を修正して再試行してください。"}),
                        )
                    }
                }
            }
            "/api/settings/cancel" => {
                let request: PollRequest = serde_json::from_slice(bytes)
                    .map_err(|_| anyhow!("Invalid cancellation request"))?;
                if request.generation != flow.generation {
                    return Ok(json!({"state":"stale"}));
                }
                flow.settings_task = None;
                flow.settings_ready = None;
                flow.wizard_error = None;
                flow.generation = random_id()?;
                self.settings_activity()?;
                Ok(json!({"state":"idle"}))
            }
            "/api/settings/save" => {
                let request: CommitRequest =
                    serde_json::from_slice(bytes).map_err(|_| anyhow!("Invalid save request"))?;
                let ready = flow
                    .settings_ready
                    .as_ref()
                    .ok_or_else(|| anyhow!("変更後の設定で接続確認を行ってください。"))?;
                if ready.revision != request.revision
                    || !bool::from(ready.ticket.as_bytes().ct_eq(request.ticket.as_bytes()))
                {
                    bail!("設定が変更されています。接続確認をやり直してください。");
                }
                let ready = flow.settings_ready.take().expect("checked settings");
                let mut recovery_required = false;
                let result = self.save_settings(&mut flow, ready, &mut recovery_required);
                let outcome = match &result {
                    Ok(value) => value.clone(),
                    Err(error) => {
                        json!({"state":if recovery_required {"recovery_required"} else {"failed"},"error":error.to_string()})
                    }
                };
                flow.settings_outcome = Some((request.ticket, request.revision, outcome));
                result
            }
            "/api/settings/result" => {
                let request: CommitRequest = serde_json::from_slice(bytes)
                    .map_err(|_| anyhow!("Invalid outcome request"))?;
                if let Some((ticket, revision, outcome)) = &flow.settings_outcome
                    && revision == &request.revision
                    && bool::from(ticket.as_bytes().ct_eq(request.ticket.as_bytes()))
                {
                    return Ok(outcome.clone());
                }
                if flow.settings_ready.as_ref().is_some_and(|ready| {
                    ready.revision == request.revision
                        && bool::from(ready.ticket.as_bytes().ct_eq(request.ticket.as_bytes()))
                }) {
                    // If reconciliation overtakes a delayed save, revoke its
                    // capability before acknowledging that nothing was saved.
                    flow.settings_ready = None;
                    let outcome = json!({"state":"aborted","error":"保存要求は取り消されました。接続確認からやり直してください。"});
                    flow.settings_outcome =
                        Some((request.ticket, request.revision, outcome.clone()));
                    return Ok(outcome);
                }
                bail!("保存結果を確定できません。設定ファイルを確認してください。")
            }
            _ => bail!("Unsupported settings operation"),
        }
    }

    fn settings_activity(&self) -> Result<()> {
        *self
            .last_activity
            .lock()
            .map_err(|_| anyhow!("Session unavailable"))? = Instant::now();
        Ok(())
    }

    fn rollback_settings(&self, items: Vec<(String, CreatedItem)>) -> bool {
        let mut failed = false;
        for (entry, item) in items {
            if (item.rollback)().is_err() {
                self.recovery_warning(
                    &entry,
                    "owned credential cleanup failed; configuration was not written",
                );
                failed = true;
            }
        }
        failed
    }

    fn save_settings(
        &self,
        flow: &mut Flow,
        ready: Ready,
        recovery_required: &mut bool,
    ) -> Result<Value> {
        let mut created = vec![];
        for (entry, secret) in &ready.secrets {
            if !self.live() {
                *recovery_required = self.rollback_settings(created);
                bail!("Session expired; configuration was not written");
            }
            match self.services.create(entry, secret) {
                Ok(item) => created.push((entry.clone(), item)),
                Err(_) => {
                    *recovery_required = true;
                    self.recovery_warning(entry,"credential creation failed or timed out; creation outcome may be uncertain; configuration was not written");
                    self.rollback_settings(created);
                    bail!(
                        "Keychainへの保存が完了しませんでした。設定ファイルは変更していません。ターミナルの復旧情報を確認してください。"
                    );
                }
            }
        }
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
        self.finish_settings_save(result, may_have_written, created, recovery_required)?;
        flow.saved = Some(ready.name.clone());
        self.settings_activity()?;
        Ok(json!({"state":"saved","saved":ready.name}))
    }

    fn finish_settings_save(
        &self,
        result: Result<()>,
        may_have_written: bool,
        created: Vec<(String, CreatedItem)>,
        recovery_required: &mut bool,
    ) -> Result<()> {
        if result.is_err() {
            if may_have_written {
                *recovery_required = true;
                self.recovery_warning(
                    "settings save",
                    "configuration write may be partial; credentials retained for recovery",
                );
                bail!(
                    "保存が一部だけ完了した可能性があります。再実行せず設定ファイルを確認してください。"
                );
            }
            let failed = self.rollback_settings(created);
            if failed {
                *recovery_required = true;
                bail!(
                    "設定ファイルは変更していません。新規Keychain項目の後処理に失敗したため、ターミナルの復旧情報を確認してください。"
                );
            }
            bail!(
                "保存できませんでした。設定ファイルの変更・アクセス権・ACLを確認して再試行してください。"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wizard::{configure::SetupTarget, web::draft};
    use std::{
        fs,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };
    use tempfile::{TempDir, tempdir};
    #[derive(Clone, Default)]
    struct Fake {
        creates: Arc<AtomicUsize>,
        rollbacks: Arc<AtomicUsize>,
        fail_at: usize,
        cleanup_failure: bool,
    }
    impl Services for Fake {
        fn read(&self, _: &str) -> Result<Zeroizing<String>> {
            bail!("unexpected credential read")
        }
        fn check(&self, _: &draft::Draft, _: &str, _: &dyn Fn() -> bool) -> Result<()> {
            bail!("unexpected legacy provider check")
        }
        fn create(&self, _: &str, password: &str) -> Result<CreatedItem> {
            assert_eq!(password, "fixture-password");
            let n = self.creates.fetch_add(1, Ordering::SeqCst) + 1;
            if n == self.fail_at {
                bail!("uncertain creation");
            }
            let fake = self.clone();
            Ok(CreatedItem {
                rollback: Box::new(move || {
                    fake.rollbacks.fetch_add(1, Ordering::SeqCst);
                    if fake.cleanup_failure {
                        bail!("cleanup failed");
                    }
                    Ok(())
                }),
            })
        }
    }
    fn setup(fake: Fake, count: usize) -> (TempDir, Session<Fake>) {
        let root = tempdir().unwrap();
        let target = SetupTarget::open(&[root.path().join("config.toml")]).unwrap();
        let session = Session::new("127.0.0.1:12345".into(), target, fake).unwrap();
        session.flow.lock().unwrap().settings_ready = Some(Ready {
            ticket: "test-ticket".into(),
            name: "fixture".into(),
            revision: "17".into(),
            document: Zeroizing::new("[accounts.fixture]\nmaildir.root = \"/fixture\"\n".into()),
            preview: "redacted".into(),
            secrets: (0..count)
                .map(|n| {
                    (
                        format!("himalaya-web-fixture-{n}"),
                        Zeroizing::new("fixture-password".into()),
                    )
                })
                .collect(),
        });
        (root, session)
    }
    fn save(session: &Session<Fake>, ticket: &str, revision: &str) -> Result<Value> {
        session.dispatch(
            "POST",
            "/api/settings/save",
            &serde_json::to_vec(&json!({"ticket":ticket,"revision":revision})).unwrap(),
        )
    }
    #[test]
    fn only_current_revision_ticket_can_save_and_is_single_use() {
        let fake = Fake::default();
        let (root, session) = setup(fake.clone(), 2);
        assert!(save(&session, "test-ticket", "18").is_err());
        assert!(save(&session, "wrong", "17").is_err());
        assert_eq!(fake.creates.load(Ordering::SeqCst), 0);
        assert_eq!(
            save(&session, "test-ticket", "17").unwrap()["saved"],
            "fixture"
        );
        assert!(save(&session, "test-ticket", "17").is_err());
        assert_eq!(fake.creates.load(Ordering::SeqCst), 2);
        assert_eq!(fake.rollbacks.load(Ordering::SeqCst), 0);
        assert!(
            !fs::read_to_string(root.path().join("config.toml"))
                .unwrap()
                .contains("fixture-password")
        );
    }
    #[test]
    fn second_creation_failure_rolls_back_only_prior_owned_item() {
        let fake = Fake {
            fail_at: 2,
            ..Default::default()
        };
        let (root, session) = setup(fake.clone(), 2);
        assert!(save(&session, "test-ticket", "17").is_err());
        assert_eq!(fake.creates.load(Ordering::SeqCst), 2);
        assert_eq!(fake.rollbacks.load(Ordering::SeqCst), 1);
        assert!(!root.path().join("config.toml").exists());
    }
    #[test]
    fn rollback_attempts_continue_after_a_cleanup_failure() {
        let fake = Fake {
            fail_at: 3,
            cleanup_failure: true,
            ..Default::default()
        };
        let (root, session) = setup(fake.clone(), 3);
        assert!(save(&session, "test-ticket", "17").is_err());
        assert_eq!(fake.rollbacks.load(Ordering::SeqCst), 2);
        assert!(!root.path().join("config.toml").exists());
    }
    #[test]
    fn expired_sessions_cannot_create_and_legacy_routes_cannot_cross_settings_flow() {
        let fake = Fake::default();
        let (_root, session) = setup(fake.clone(), 1);
        for path in [
            "/api/test",
            "/api/save",
            "/api/wizard/start",
            "/api/wizard/cancel",
        ] {
            assert!(session.dispatch("POST", path, b"{}").is_err());
        }
        assert!(session.dispatch("GET", "/api/status", b"").is_ok());
        session.revoke();
        assert!(save(&session, "test-ticket", "17").is_err());
        assert_eq!(fake.creates.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn poll_and_rejected_save_do_not_keep_session_alive() {
        let (_root, session) = setup(Fake::default(), 0);
        let before = *session.last_activity.lock().unwrap();
        assert!(
            session
                .dispatch("POST", "/api/settings/poll", br#"{"generation":"wrong"}"#)
                .is_ok()
        );
        assert!(save(&session, "wrong", "17").is_err());
        assert_eq!(*session.last_activity.lock().unwrap(), before);
    }

    #[test]
    fn reconciliation_revokes_a_delayed_save_and_reports_completed_outcome() {
        let fake = Fake::default();
        let (_root, session) = setup(fake.clone(), 1);
        let request = br#"{"ticket":"test-ticket","revision":"17"}"#;
        assert_eq!(
            session
                .dispatch("POST", "/api/settings/result", request)
                .unwrap()["state"],
            "aborted"
        );
        assert!(save(&session, "test-ticket", "17").is_err());
        assert_eq!(fake.creates.load(Ordering::SeqCst), 0);
        let (_root, session) = setup(fake.clone(), 1);
        save(&session, "test-ticket", "17").unwrap();
        assert_eq!(
            session
                .dispatch("POST", "/api/settings/result", request)
                .unwrap()["saved"],
            "fixture"
        );
        assert_eq!(fake.creates.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn asynchronous_completion_is_generation_bound_and_cancel_discards_ready() {
        let (_root, session) = setup(Fake::default(), 0);
        {
            let mut flow = session.flow.lock().unwrap();
            let ready = flow.settings_ready.take();
            flow.generation = "current".into();
            flow.settings_task = Some(Task {
                worker: FormProcess::fixture(
                    "printf '%s\\n' '{\"type\":\"data\",\"data\":{\"ok\":true}}'",
                ),
                revision: "17".into(),
                pending: ready,
            });
        }
        assert_eq!(
            session
                .dispatch("POST", "/api/settings/poll", br#"{"generation":"old"}"#)
                .unwrap()["state"],
            "stale"
        );
        let start = Instant::now();
        loop {
            let reply = session
                .dispatch("POST", "/api/settings/poll", br#"{"generation":"current"}"#)
                .unwrap();
            if reply["state"] == "ready" {
                assert_eq!(reply["revision"], "17");
                break;
            }
            assert!(start.elapsed() < std::time::Duration::from_secs(2));
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        session
            .dispatch(
                "POST",
                "/api/settings/cancel",
                br#"{"generation":"current"}"#,
            )
            .unwrap();
        assert!(save(&session, "test-ticket", "17").is_err());
        assert!(session.flow.lock().unwrap().settings_task.is_none());
    }

    #[test]
    fn settings_watchdog_drops_pending_secret_without_browser_poll() {
        let (_root, session) = setup(Fake::default(), 1);
        {
            let mut flow = session.flow.lock().unwrap();
            let ready = flow.settings_ready.take();
            let mut worker = FormProcess::fixture("exec /bin/sleep 60");
            worker.expire_for_test();
            flow.settings_task = Some(Task {
                worker,
                revision: "17".into(),
                pending: ready,
            });
        }
        session.watchdog();
        let flow = session.flow.lock().unwrap();
        assert!(flow.settings_task.is_none());
        assert!(flow.settings_ready.is_none());
    }

    #[test]
    fn partial_write_requires_recovery_and_retains_created_credentials() {
        let fake = Fake::default();
        let (_root, session) = setup(fake.clone(), 0);
        let item = fake
            .create("himalaya-web-owned-fixture", "fixture-password")
            .unwrap();
        let mut recovery = false;
        assert!(
            session
                .finish_settings_save(
                    Err(anyhow!("injected partial append")),
                    true,
                    vec![("himalaya-web-owned-fixture".into(), item)],
                    &mut recovery
                )
                .is_err()
        );
        assert!(recovery);
        assert_eq!(fake.rollbacks.load(Ordering::SeqCst), 0);
        let fake = Fake {
            fail_at: 1,
            ..Default::default()
        };
        let (_root, session) = setup(fake, 1);
        assert!(save(&session, "test-ticket", "17").is_err());
        assert_eq!(
            session
                .dispatch(
                    "POST",
                    "/api/settings/result",
                    br#"{"ticket":"test-ticket","revision":"17"}"#
                )
                .unwrap()["state"],
            "recovery_required"
        );
    }
}
