//! Structured forms over the actual discovery and authentication wizard.

use std::{
    collections::HashMap,
    env,
    io::{BufRead, Read, Write, stdin, stdout},
    os::{fd::AsRawFd, unix::process::CommandExt},
    path::PathBuf,
    process::{Child, ChildStdin, ChildStdout, Command, ExitStatus, Stdio},
    sync::{
        LazyLock, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::{Result, anyhow, bail};
use pimalaya_config::command::CommandConfig;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use zeroize::Zeroizing;

use crate::wizard::discover;
use pimalaya_config::secret::Secret;

const MAX_MESSAGE: usize = 65536;
static ENABLED: AtomicBool = AtomicBool::new(false);
static NEXT: AtomicU64 = AtomicU64::new(1);
static RESOLVED: LazyLock<Mutex<HashMap<CommandConfig, SecretString>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Identifies only the explicitly launched private form worker.
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// One public form question, never containing a previous secret answer.
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    /// Monotonic identifier preventing duplicate or stale answers.
    pub id: u64,
    /// One of text, password, select or confirm.
    pub kind: String,
    /// Original wizard question.
    pub label: String,
    /// Text default or selection index; never a secret.
    pub default: Option<String>,
    /// Original wizard display labels.
    pub choices: Vec<String>,
}

impl Question {
    fn validate_answer(&self, value: &str) -> Result<()> {
        if value.len() > 4096 || value.contains('\0') {
            bail!("Answer exceeds the allowed size");
        }
        match self.kind.as_str() {
            "confirm" if value != "true" && value != "false" => bail!("Choose yes or no"),
            "select"
                if value
                    .parse::<usize>()
                    .ok()
                    .is_none_or(|i| i >= self.choices.len()) =>
            {
                bail!("Choose a listed option")
            }
            "text" | "password" | "select" | "confirm" => Ok(()),
            _ => bail!("Invalid question type"),
        }
    }
}

/// Requests one form field through bounded private stdio.
pub fn ask(
    kind: &str,
    label: &str,
    default: Option<String>,
    choices: Vec<String>,
) -> Result<String> {
    let question = Question {
        id: NEXT.fetch_add(1, Ordering::Relaxed),
        kind: kind.into(),
        label: label.into(),
        default,
        choices,
    };
    if question.id > 256 {
        bail!("Wizard question limit reached");
    }
    emit(&json!({"type":"question", "question":question}))?;
    let mut line = Zeroizing::new(Vec::new());
    stdin().lock().take(8193).read_until(b'\n', &mut line)?;
    if line.len() > 8192 || !line.ends_with(b"\n") {
        bail!("Invalid private answer framing");
    }
    let answer: Answer =
        serde_json::from_slice(&line).map_err(|_| anyhow!("Invalid private answer"))?;
    if answer.id != question.id {
        bail!("Stale form answer");
    }
    question.validate_answer(&answer.value)?;
    if question.id == 1 && answer.value.contains("://") {
        let parsed = url::Url::parse(&answer.value).map_err(|_| anyhow!("Invalid server URL"))?;
        if !parsed.username().is_empty() || parsed.password().is_some() {
            bail!("Do not put credentials in a server URL");
        }
    }
    Ok(answer.value.to_string())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Answer {
    id: u64,
    value: Zeroizing<String>,
}

fn emit(value: &Value) -> Result<()> {
    let mut bytes = Zeroizing::new(serde_json::to_vec(value)?);
    if bytes.len() >= MAX_MESSAGE {
        bail!("Wizard response exceeds limit");
    }
    bytes.push(b'\n');
    let mut out = stdout().lock();
    out.write_all(&bytes)?;
    out.flush()?;
    Ok(())
}

/// Runs the original wizard without loading or saving an existing config.
pub fn execute() -> Result<()> {
    ENABLED.store(true, Ordering::Relaxed);
    match discover::run() {
        Ok((name, account)) => {
            let document = Zeroizing::new(account.render(&name)?);
            emit(&json!({"type":"complete", "name":name, "document":document.as_str()}))
        }
        Err(_) => emit(
            &json!({"type":"error", "message":"設定または接続確認に失敗しました。入力・認証情報の保存先・サービスの利用条件を確認し、最初からやり直してください。"}),
        ),
    }
}

/// Owns one fixed worker, its private channel and process group.
pub struct FormProcess {
    child: Child,
    input: ChildStdin,
    output: ChildStdout,
    buffer: Zeroizing<Vec<u8>>,
    question: Option<Question>,
    last_question: u64,
    started: Instant,
    waiting_since: Option<Instant>,
    pending_input: Zeroizing<Vec<u8>>,
    input_offset: usize,
}

/// A finished account retained only by the server until explicit save.
pub struct Ready {
    /// One-use save capability.
    pub ticket: String,
    /// New account table name.
    pub name: String,
    /// Actual generated document; may contain explicitly selected raw secrets.
    pub document: Zeroizing<String>,
    /// Credential-redacted document shown to the browser.
    pub preview: String,
}

/// The next private event, separated from public browser responses.
pub enum Event {
    /// No worker output yet.
    Pending,
    /// A public form question.
    Question(Question),
    /// A newly authenticated account, never sent directly to the browser.
    Complete(String, Zeroizing<String>),
    /// A bounded structured settings result, private until parent validation.
    Data(Value),
}

impl FormProcess {
    #[cfg(test)]
    pub(super) fn expire_for_test(&mut self) {
        self.waiting_since = Some(Instant::now() - Duration::from_secs(46));
    }
    #[cfg(test)]
    pub(super) fn process_id(&self) -> u32 {
        self.child.id()
    }
    #[cfg(test)]
    pub(super) fn fixture(script: &str) -> Self {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", script]);
        Self::launch(command).unwrap()
    }
    /// Launches only this executable's form worker, with no user argv.
    pub fn start() -> Result<Self> {
        Self::spawn(executable()?, "--web-form-worker")
    }

    /// Starts only the fixed settings helper and queues bounded private input.
    pub fn settings(payload: &[u8]) -> Result<Self> {
        if payload.len() > 262143 {
            bail!("Private settings input exceeds limit");
        }
        let mut process = Self::spawn(executable()?, "--web-settings-worker")?;
        process.pending_input.extend_from_slice(payload);
        process.pending_input.push(b'\n');
        Ok(process)
    }

    fn spawn(executable: PathBuf, flag: &str) -> Result<Self> {
        let mut command = Command::new(executable);
        command
            .args(["--log-level", "off", "configure", flag])
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0);
        for name in [
            "HOME",
            "PATH",
            "USER",
            "LOGNAME",
            "TMPDIR",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_RUNTIME_DIR",
            "SSH_AUTH_SOCK",
        ] {
            if let Some(value) = env::var_os(name) {
                command.env(name, value);
            }
        }
        Self::launch(command)
    }

    fn launch(mut command: Command) -> Result<Self> {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0);
        let mut child = command
            .spawn()
            .map_err(|_| anyhow!("Cannot start configuration worker"))?;
        let input = child.stdin.take().expect("piped input");
        let output = child.stdout.take().expect("piped output");
        for fd in [input.as_raw_fd(), output.as_raw_fd()] {
            // SAFETY: both file descriptors are live owned pipes.
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
            {
                // SAFETY: this child was spawned in its own process group.
                unsafe {
                    libc::kill(-(child.id() as i32), libc::SIGKILL);
                }
                let _ = child.wait();
                bail!("Cannot initialize private worker pipes");
            }
        }
        Ok(Self {
            child,
            input,
            output,
            buffer: Zeroizing::new(Vec::new()),
            question: None,
            last_question: 0,
            started: Instant::now(),
            waiting_since: Some(Instant::now()),
            pending_input: Zeroizing::new(Vec::new()),
            input_offset: 0,
        })
    }

    /// Bounds non-interactive work even when the browser stops polling.
    pub fn expired(&self) -> bool {
        self.started.elapsed() > Duration::from_secs(1800)
            || self
                .waiting_since
                .is_some_and(|t| t.elapsed() > Duration::from_secs(45))
    }

    /// Reads bounded output without blocking an HTTP or shutdown thread.
    pub fn poll(&mut self) -> Result<Event> {
        if self.expired() {
            bail!("設定処理がタイムアウトしました。最初からやり直してください。");
        }
        while self.input_offset < self.pending_input.len() {
            match self.input.write(&self.pending_input[self.input_offset..]) {
                Ok(0) => bail!("Settings worker input closed"),
                Ok(n) => self.input_offset += n,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => bail!("Settings worker input failed"),
            }
        }
        if !self.pending_input.is_empty() && self.input_offset == self.pending_input.len() {
            self.pending_input.clear();
            self.input_offset = 0;
        }
        if let Some(question) = &self.question {
            return Ok(Event::Question(question.clone()));
        }
        let mut chunk = Zeroizing::new([0u8; 4096]);
        let mut eof = false;
        loop {
            match self.output.read(chunk.as_mut()) {
                Ok(0) => {
                    eof = true;
                    break;
                }
                Ok(n) => {
                    if self.buffer.len() + n > MAX_MESSAGE {
                        bail!("Worker response exceeds limit");
                    }
                    self.buffer.extend_from_slice(&chunk[..n]);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => bail!("Configuration worker channel failed"),
            }
        }
        if let Some(end) = self.buffer.iter().position(|b| *b == b'\n') {
            let value: Value = serde_json::from_slice(&self.buffer[..end])
                .map_err(|_| anyhow!("Invalid configuration worker response"))?;
            self.buffer.drain(..=end);
            match value["type"].as_str() {
                Some("data") => return Ok(Event::Data(value["data"].clone())),
                Some("question") => {
                    let question: Question = serde_json::from_value(value["question"].clone())
                        .map_err(|_| anyhow!("Invalid worker question"))?;
                    if question.id != self.last_question + 1
                        || question.id > 256
                        || question.label.len() > 4096
                        || question.choices.len() > 256
                        || question.kind == "password" && question.default.is_some()
                    {
                        bail!("Invalid worker question limits");
                    }
                    self.waiting_since = None;
                    self.last_question = question.id;
                    self.question = Some(question.clone());
                    return Ok(Event::Question(question));
                }
                Some("complete") => {
                    let name = value["name"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing account name"))?
                        .to_string();
                    let document = value["document"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Missing generated account"))?;
                    return Ok(Event::Complete(name, Zeroizing::new(document.into())));
                }
                _ => bail!(
                    "設定または接続確認に失敗しました。入力・認証情報の保存先・サービスの利用条件を確認してください。"
                ),
            }
        }
        if eof {
            bail!("設定処理が終了しました。最初からやり直してください。");
        }
        Ok(Event::Pending)
    }

    /// Submits exactly one bounded answer to the outstanding question.
    pub fn answer(&mut self, bytes: &[u8]) -> Result<()> {
        let answer: Answer =
            serde_json::from_slice(bytes).map_err(|_| anyhow!("Invalid form answer"))?;
        let question = self
            .question
            .as_ref()
            .ok_or_else(|| anyhow!("No question is awaiting an answer"))?;
        if answer.id != question.id {
            bail!("This question is stale; reload its state");
        }
        question.validate_answer(&answer.value)?;
        if question.id == 1 && answer.value.contains("://") {
            let url = url::Url::parse(&answer.value).map_err(|_| anyhow!("Invalid server URL"))?;
            if !url.username().is_empty() || url.password().is_some() {
                bail!("Do not put credentials in a server URL");
            }
        }
        let mut line = Zeroizing::new(bytes.to_vec());
        line.push(b'\n');
        self.input
            .write_all(&line)
            .map_err(|_| anyhow!("Cannot submit the answer; restart configuration"))?;
        self.question = None;
        self.waiting_since = Some(Instant::now());
        Ok(())
    }
}

impl Drop for FormProcess {
    fn drop(&mut self) {
        // SAFETY: the live, unreaped child owns this process-group identifier.
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGKILL);
        }
        let _ = self.child.wait();
    }
}

#[cfg(not(test))]
fn executable() -> Result<PathBuf> {
    Ok(env::current_exe()?)
}
#[cfg(test)]
fn executable() -> Result<PathBuf> {
    env::var_os("HIMALAYA_WEB_TEST_BINARY")
        .map(Into::into)
        .ok_or_else(|| anyhow!("Set HIMALAYA_WEB_TEST_BINARY for explicit form-worker tests"))
}

/// Redacts all credential and proxy branches, including custom commands.
pub fn preview(document: &str) -> Result<String> {
    let mut value: toml::Value = toml::from_str(document)?;
    fn redact(value: &mut toml::Value) {
        match value {
            toml::Value::Table(table) => {
                for (key, value) in table.iter_mut() {
                    if [
                        "sasl", "auth", "password", "passwd", "token", "secret", "proxy",
                    ]
                    .contains(&key.as_str())
                    {
                        *value = toml::Value::String("<認証情報は非表示>".into());
                    } else {
                        redact(value);
                    }
                }
            }
            toml::Value::Array(values) => {
                for value in values {
                    redact(value);
                }
            }
            _ => {}
        }
    }
    redact(&mut value);
    Ok(toml::to_string_pretty(&value)?)
}

/// Resolves a command with bounded output, retaining no browser-visible value.
pub fn resolve_secret(secret: &mut Secret) -> Result<()> {
    let Secret::Command(command) = secret else {
        return Ok(());
    };
    let mut cache = RESOLVED
        .lock()
        .map_err(|_| anyhow!("Credential cache unavailable"))?;
    if !cache.contains_key(command) {
        cache.insert(command.clone(), resolve_command(command)?);
    }
    *secret = Secret::Raw(cache[command].clone());
    Ok(())
}

fn resolve_command(config: &CommandConfig) -> Result<SecretString> {
    let mut child = config
        .to_command()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| anyhow!("Cannot start credential command"))?;
    let result = (|| {
        let mut output = child.stdout.take().expect("piped credential output");
        let fd = output.as_raw_fd();
        // SAFETY: output owns a live pipe descriptor.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            bail!("Cannot initialize credential output");
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut bytes = Zeroizing::new(Vec::new());
        let mut chunk = Zeroizing::new([0u8; 1024]);
        let mut status: Option<ExitStatus> = None;
        let mut eof = false;
        loop {
            loop {
                match output.read(chunk.as_mut()) {
                    Ok(0) => {
                        eof = true;
                        break;
                    }
                    Ok(n) => {
                        if bytes.len() + n > 8192 {
                            bail!("Credential command output exceeds limit");
                        }
                        bytes.extend_from_slice(&chunk[..n]);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => bail!("Credential command output failed"),
                }
            }
            if let Some(status) = status
                && eof
            {
                if !status.success() {
                    bail!("Credential command failed; check its entry and access permissions");
                }
                let value = String::from_utf8_lossy(&bytes);
                return Ok(value.lines().next().unwrap_or_default().to_string().into());
            }
            if status.is_none() {
                status = child.try_wait()?;
            }
            if Instant::now() >= deadline {
                bail!("Credential command timed out");
            }
            thread::sleep(Duration::from_millis(20));
        }
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}

#[cfg(test)]
mod tests {
    use crate::wizard::web::form::{Event, FormProcess, Question, preview, resolve_command};
    use pimalaya_config::command::CommandConfig;
    use secrecy::ExposeSecret;
    use serde_json::json;
    use std::{
        process::Command,
        thread,
        time::{Duration, Instant},
    };

    fn fixture(script: &str) -> FormProcess {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", script]);
        FormProcess::launch(command).unwrap()
    }

    #[test]
    fn redacts_raw_tokens_commands_and_proxy_credentials() {
        let source = r#"[accounts.test]
email = "user@example.test"
imap.server = "imaps://mail.example.test:993"
imap.sasl.plain.passwd.raw = "RAW_SECRET_MARKER"
smtp.sasl.xoauth2.token.command = ["printf", "COMMAND_SECRET_MARKER"]
gmail.auth.token.raw = "TOKEN_SECRET_MARKER"
proxy.password.raw = "PROXY_SECRET_MARKER"
"#;
        let output = preview(source).unwrap();
        assert!(!output.contains("SECRET_MARKER"));
        assert!(output.contains("mail.example.test"));
        assert!(output.contains("認証情報は非表示"));
    }

    #[test]
    fn local_account_round_trips_for_private_authentication_check() {
        use crate::config::{AccountConfig, MaildirConfig};
        use crate::wizard::prompt::CheckConfig;
        let mut account = AccountConfig {
            maildir: Some(MaildirConfig {
                root: "/tmp/$LITERAL-fixture".into(),
                keywords: Default::default(),
            }),
            ..Default::default()
        };
        account.prepare().unwrap();
        assert_eq!(
            account.maildir.unwrap().root.to_str(),
            Some("/tmp/$LITERAL-fixture")
        );
    }

    #[test]
    fn typed_answers_refuse_out_of_range_and_invalid_values() {
        let mut question = Question {
            id: 1,
            kind: "select".into(),
            label: "choice".into(),
            default: None,
            choices: vec!["a".into(), "b".into()],
        };
        for answer in ["-1", "2", "bogus", "0\0"] {
            assert!(question.validate_answer(answer).is_err());
        }
        assert!(question.validate_answer("1").is_ok());
        question.kind = "confirm".into();
        assert!(question.validate_answer("true").is_ok());
        assert!(question.validate_answer("yes").is_err());
        question.kind = "password".into();
        assert!(question.validate_answer(&"x".repeat(4097)).is_err());
        assert!(
            question
                .validate_answer("arbitrary-length-token-非ASCII")
                .is_ok()
        );
    }

    #[test]
    fn stalled_and_partial_worker_output_is_nonblocking_and_reaped() {
        let mut worker = fixture("printf '{'; exec /bin/sleep 60");
        let pid = worker.child.id();
        let start = Instant::now();
        for _ in 0..10 {
            assert!(matches!(worker.poll().unwrap(), Event::Pending));
        }
        assert!(start.elapsed() < Duration::from_secs(1));
        worker.waiting_since = Some(Instant::now() - Duration::from_secs(46));
        assert!(worker.poll().is_err());
        drop(worker);
        // SAFETY: kill with signal zero only checks existence of our owned pid.
        assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
    }

    #[test]
    fn questions_and_answers_are_single_use_and_keep_secrets_private() {
        let mut worker = fixture(
            "printf '%s\\n' '{\"type\":\"question\",\"question\":{\"id\":1,\"kind\":\"password\",\"label\":\"Password\",\"default\":null,\"choices\":[]}}'; read ignored; exec /bin/sleep 60",
        );
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if matches!(worker.poll().unwrap(), Event::Question(_)) {
                break;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            worker
                .answer(br#"{"id":2,"value":"SECRET_MARKER"}"#)
                .is_err()
        );
        worker
            .answer(br#"{"id":1,"value":"SECRET_MARKER"}"#)
            .unwrap();
        assert!(
            worker
                .answer(br#"{"id":1,"value":"SECRET_MARKER"}"#)
                .is_err()
        );
        assert!(!String::from_utf8_lossy(&worker.buffer).contains("SECRET_MARKER"));
    }

    #[test]
    fn oversized_worker_output_is_rejected() {
        let mut worker = fixture("/usr/bin/yes x");
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if worker.poll().is_err() {
                break;
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn credential_commands_have_bounded_output_and_redacted_errors() {
        let secret = resolve_command(&CommandConfig::Shell(
            "printf 'temporary-test-value\\nsecond\\n'".into(),
        ))
        .unwrap();
        assert_eq!(secret.expose_secret(), "temporary-test-value");
        let error = resolve_command(&CommandConfig::Shell(
            "printf SECRET_MARKER; printf SECRET_MARKER >&2; exit 7".into(),
        ))
        .unwrap_err();
        assert!(!error.to_string().contains("SECRET_MARKER"));
        let start = Instant::now();
        assert!(
            resolve_command(&CommandConfig::Shell(
                "exec /usr/bin/yes SECRET_MARKER".into()
            ))
            .is_err()
        );
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn bounded_resolution_preserves_saved_command_and_memoizes_checks() {
        use pimalaya_config::secret::Secret;
        let command = json!({"token":{"command":["/usr/bin/printf","fixture-secret\\n"]}});
        let mut resolved: Secret = serde_json::from_value(command["token"].clone()).unwrap();
        crate::wizard::web::form::resolve_secret(&mut resolved).unwrap();
        assert!(command["token"]["command"].is_array());
        let Secret::Raw(value) = resolved else {
            panic!("expected resolved secret");
        };
        assert_eq!(value.expose_secret(), "fixture-secret");
    }

    #[test]
    fn credential_output_waits_for_delayed_descendant_eof() {
        let start = Instant::now();
        let value = resolve_command(&CommandConfig::Shell(
            "(/bin/sleep 0.1; printf delayed-fixture-value) & exit 0".into(),
        ))
        .unwrap();
        assert_eq!(value.expose_secret(), "delayed-fixture-value");
        assert!(start.elapsed() >= Duration::from_millis(80));
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn dropping_worker_stops_descendants_holding_inherited_output() {
        let mut worker = fixture(
            "/bin/sleep 60 & child=$!; printf '{\"type\":\"question\",\"question\":{\"id\":1,\"kind\":\"text\",\"label\":\"%s\",\"default\":null,\"choices\":[]}}\\n' \"$child\"; wait",
        );
        let deadline = Instant::now() + Duration::from_secs(2);
        let descendant: i32 = loop {
            if let Event::Question(question) = worker.poll().unwrap() {
                break question.label.parse().unwrap();
            }
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        };
        drop(worker);
        for _ in 0..100 {
            // SAFETY: signal zero only checks our fixture child's existence.
            if unsafe { libc::kill(descendant, 0) } == -1 {
                return;
            }
            thread::sleep(Duration::from_millis(5));
        }
        let state = Command::new("/bin/ps")
            .args(["-o", "stat=", "-p", &descendant.to_string()])
            .output()
            .unwrap();
        assert!(
            String::from_utf8_lossy(&state.stdout)
                .trim()
                .starts_with('Z'),
            "descendant must be gone or only awaiting OS reaping"
        );
    }
}
