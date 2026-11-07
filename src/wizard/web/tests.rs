//! # Setup boundary tests
//!
//! Uses disposable files and fake credentials/providers, never user mail.

use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    slice::from_ref,
    sync::{
        Arc, Barrier,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::{Result, bail};
use hyper::{Request, StatusCode};
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};
use zeroize::Zeroizing;

use crate::{
    config::Config,
    wizard::{
        configure::SetupTarget,
        web::{
            Services, Session,
            draft::Draft,
            http::{authorize, serve},
            keychain::{self, CreatedItem},
            random_id,
        },
    },
};

const PASSWORD: &str = "abcdefghijklmnop";
const INITIAL: &str = "# keep this comment\n[accounts.personal]\ndefault = true\nemail = \"personal@example.test\"\nmaildir.root = \"/unused\"\n";

#[derive(Clone, Default)]
struct Fake {
    reads: Arc<AtomicUsize>,
    checks: Arc<AtomicUsize>,
    creates: Arc<AtomicUsize>,
    rollbacks: Arc<AtomicUsize>,
    fail_check: Arc<AtomicBool>,
    fail_cleanup: Arc<AtomicBool>,
    changed: Arc<AtomicBool>,
    fail_create: Arc<AtomicBool>,
    create_pause: Option<Arc<(Barrier, Barrier)>>,
}

impl Services for Fake {
    fn read(&self, _: &str) -> Result<Zeroizing<String>> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        Ok(Zeroizing::new(
            if self.changed.load(Ordering::SeqCst) {
                "ponmlkjihgfedcba"
            } else {
                PASSWORD
            }
            .into(),
        ))
    }
    fn check(&self, _: &Draft, password: &str, _: &dyn Fn() -> bool) -> Result<()> {
        self.checks.fetch_add(1, Ordering::SeqCst);
        assert_eq!(password, PASSWORD);
        if self.fail_check.load(Ordering::SeqCst) {
            bail!("Provider authentication failed");
        }
        Ok(())
    }
    fn create(&self, entry: &str, password: &str) -> Result<CreatedItem> {
        self.creates.fetch_add(1, Ordering::SeqCst);
        assert!(entry.starts_with("himalaya-web-"));
        assert_eq!(password, PASSWORD);
        if let Some(pause) = &self.create_pause {
            pause.0.wait();
            pause.1.wait();
        }
        if self.fail_create.load(Ordering::SeqCst) {
            bail!("Simulated uncertain creation timeout");
        }
        let fake = self.clone();
        Ok(CreatedItem {
            rollback: Box::new(move || {
                fake.rollbacks.fetch_add(1, Ordering::SeqCst);
                if fake.fail_cleanup.load(Ordering::SeqCst) {
                    bail!("Cleanup failed");
                }
                Ok(())
            }),
        })
    }
}

fn setup(initial: Option<&str>) -> (TempDir, Session<Fake>, Fake) {
    let root = tempdir().unwrap();
    let path = root.path().join("config.toml");
    if let Some(text) = initial {
        fs::write(&path, text).unwrap();
    }
    let target = SetupTarget::open(&[path]).unwrap();
    let fake = Fake::default();
    let session = Session::new("127.0.0.1:12345".into(), target, fake.clone()).unwrap();
    (root, session, fake)
}

fn form() -> Value {
    json!({"provider":"gmail","name":"university","email":"student@university.example","credential":{"kind":"new","password":PASSWORD}})
}

fn check(session: &Session<Fake>, form: Value) -> Value {
    session
        .dispatch("POST", "/api/test", &serde_json::to_vec(&form).unwrap())
        .unwrap()
}

fn save(session: &Session<Fake>, checked: &Value) -> Result<Value> {
    session.dispatch(
        "POST",
        "/api/save",
        &serde_json::to_vec(&json!({"ticket": checked["ticket"]})).unwrap(),
    )
}

#[test]
fn preview_is_reference_only_and_save_preserves_existing_bytes() {
    let (root, session, fake) = setup(Some(INITIAL));
    let checked = check(&session, form());
    assert!(!checked.to_string().contains(PASSWORD));
    assert_eq!(fake.creates.load(Ordering::SeqCst), 0);
    assert_eq!(
        fs::read_to_string(root.path().join("config.toml")).unwrap(),
        INITIAL
    );
    assert_eq!(save(&session, &checked).unwrap()["saved"], "university");
    let text = fs::read_to_string(root.path().join("config.toml")).unwrap();
    assert!(text.starts_with(INITIAL));
    assert!(!text.contains(PASSWORD));
    assert_eq!(text.matches("default = true").count(), 1);
    let parsed: Config = toml::from_str(&text).unwrap();
    assert_eq!(parsed.accounts.len(), 2);
    assert_eq!(fake.checks.load(Ordering::SeqCst), 1);
    assert_eq!(fake.creates.load(Ordering::SeqCst), 1);
    assert_eq!(fake.rollbacks.load(Ordering::SeqCst), 0);
}

#[test]
fn first_account_has_private_modes_and_default() {
    use std::os::unix::fs::PermissionsExt;
    let (root, session, _) = setup(None);
    save(&session, &check(&session, form())).unwrap();
    let path = root.path().join("config.toml");
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(fs::read_to_string(path).unwrap().contains("default = true"));
}

#[test]
fn save_without_check_and_duplicate_save_do_not_create_credentials() {
    let (root, session, fake) = setup(None);
    assert!(
        session
            .dispatch("POST", "/api/save", b"{\"ticket\":\"fake\"}")
            .is_err()
    );
    assert_eq!(fake.creates.load(Ordering::SeqCst), 0);
    let checked = check(&session, form());
    save(&session, &checked).unwrap();
    let before = fs::read(root.path().join("config.toml")).unwrap();
    assert!(save(&session, &checked).is_err());
    assert_eq!(fake.creates.load(Ordering::SeqCst), 1);
    assert_eq!(fs::read(root.path().join("config.toml")).unwrap(), before);
}

#[test]
fn a_stale_ticket_cannot_consume_the_current_check() {
    let (_root, session, fake) = setup(None);
    let old = check(&session, form());
    let current = check(&session, form());
    assert!(save(&session, &old).is_err());
    assert_eq!(fake.creates.load(Ordering::SeqCst), 0);
    save(&session, &current).unwrap();
}

#[test]
fn failed_check_invalidates_previous_success_without_writing() {
    let (root, session, fake) = setup(None);
    let old = check(&session, form());
    fake.fail_check.store(true, Ordering::SeqCst);
    assert!(
        session
            .dispatch("POST", "/api/test", &serde_json::to_vec(&form()).unwrap())
            .is_err()
    );
    assert!(save(&session, &old).is_err());
    assert!(!root.path().join("config.toml").exists());
    assert_eq!(fake.creates.load(Ordering::SeqCst), 0);
}

#[test]
fn changed_configuration_rolls_back_only_new_item() {
    let (root, session, fake) = setup(Some(INITIAL));
    let checked = check(&session, form());
    let changed = format!("{INITIAL}# user edit\n");
    fs::write(root.path().join("config.toml"), &changed).unwrap();
    assert!(save(&session, &checked).is_err());
    assert_eq!(
        fs::read_to_string(root.path().join("config.toml")).unwrap(),
        changed
    );
    assert_eq!(fake.creates.load(Ordering::SeqCst), 1);
    assert_eq!(fake.rollbacks.load(Ordering::SeqCst), 1);
}

#[test]
fn replacement_with_identical_bytes_is_refused() {
    let (root, session, fake) = setup(Some(INITIAL));
    let checked = check(&session, form());
    let replacement = root.path().join("replacement.toml");
    fs::write(&replacement, INITIAL).unwrap();
    fs::rename(replacement, root.path().join("config.toml")).unwrap();
    assert!(save(&session, &checked).is_err());
    assert_eq!(fake.rollbacks.load(Ordering::SeqCst), 1);
    assert_eq!(
        fs::read_to_string(root.path().join("config.toml")).unwrap(),
        INITIAL
    );
}

#[test]
fn new_target_race_preserves_the_other_file_and_reports_cleanup_failure() {
    let (root, session, fake) = setup(None);
    let checked = check(&session, form());
    fs::write(root.path().join("config.toml"), "other writer").unwrap();
    fake.fail_cleanup.store(true, Ordering::SeqCst);
    let error = save(&session, &checked).unwrap_err().to_string();
    assert!(error.contains("cleanup failed"));
    assert!(!error.contains(PASSWORD));
    assert_eq!(
        fs::read_to_string(root.path().join("config.toml")).unwrap(),
        "other writer"
    );
}

#[test]
fn existing_items_are_never_created_or_deleted_and_changes_require_retest() {
    let (_root, session, fake) = setup(None);
    let mut existing = form();
    existing["credential"] = json!({"kind":"existing","entry":"himalaya-fixture"});
    let checked = check(&session, existing);
    fake.changed.store(true, Ordering::SeqCst);
    assert!(save(&session, &checked).is_err());
    assert_eq!(fake.reads.load(Ordering::SeqCst), 2);
    assert_eq!(fake.creates.load(Ordering::SeqCst), 0);
    assert_eq!(fake.rollbacks.load(Ordering::SeqCst), 0);
}

#[test]
fn existing_item_save_uses_only_the_named_reference() {
    let (root, session, fake) = setup(None);
    let mut existing = form();
    existing["credential"] = json!({"kind":"existing","entry":"himalaya-fixture"});
    save(&session, &check(&session, existing)).unwrap();
    let text = fs::read_to_string(root.path().join("config.toml")).unwrap();
    assert!(text.contains("himalaya-fixture"));
    assert!(!text.contains(PASSWORD));
    assert_eq!(fake.creates.load(Ordering::SeqCst), 0);
    assert_eq!(fake.rollbacks.load(Ordering::SeqCst), 0);
}

#[test]
fn arbitrary_commands_and_provider_overrides_are_rejected_before_side_effects() {
    for (key, value) in [
        ("server", json!("imaps://evil.invalid")),
        ("proxy", json!("http://evil.invalid")),
        ("path", json!("/tmp/elsewhere")),
        ("command", json!(["sh", "-c", "touch marker"])),
    ] {
        let (_root, session, fake) = setup(None);
        let mut input = form();
        input[key] = value;
        assert!(
            session
                .dispatch("POST", "/api/test", &serde_json::to_vec(&input).unwrap())
                .is_err()
        );
        assert_eq!(
            fake.reads.load(Ordering::SeqCst)
                + fake.checks.load(Ordering::SeqCst)
                + fake.creates.load(Ordering::SeqCst),
            0
        );
    }
}

#[test]
fn duplicate_names_and_malicious_labels_never_reach_keychain_or_provider() {
    for name in [
        "personal",
        "<img src=x onerror=alert(1)>",
        "../../config",
        "a\nb",
        "",
    ] {
        let (_root, session, fake) = setup(Some(INITIAL));
        let mut input = form();
        input["name"] = json!(name);
        input["credential"] = json!({"kind":"existing","entry":"fixture"});
        assert!(
            session
                .dispatch("POST", "/api/test", &serde_json::to_vec(&input).unwrap())
                .is_err()
        );
        assert_eq!(
            fake.reads.load(Ordering::SeqCst) + fake.checks.load(Ordering::SeqCst),
            0
        );
    }
}

#[test]
fn status_never_resolves_existing_secret_commands_or_returns_config() {
    let root = tempdir().unwrap();
    let marker = root.path().join("marker");
    let config = format!(
        "[accounts.existing]\nimap.server = \"example.test\"\nimap.sasl.plain.username = \"not-returned@example.test\"\nimap.sasl.plain.password.command = [\"/usr/bin/touch\", {:?}]\n",
        marker.to_str().unwrap()
    );
    let (_root, session, fake) = setup(Some(&config));
    let status = session
        .dispatch("GET", "/api/status", b"")
        .unwrap()
        .to_string();
    assert!(!marker.exists());
    assert!(!status.contains("command"));
    assert!(!status.contains("not-returned"));
    assert_eq!(fake.reads.load(Ordering::SeqCst), 0);
}

#[test]
fn expiry_and_finish_revoke_all_later_side_effects() {
    let (_root, mut session, fake) = setup(None);
    let checked = check(&session, form());
    session.absolute_limit = Duration::ZERO;
    assert!(!session.authorized(&session.token));
    assert!(save(&session, &checked).is_err());
    assert_eq!(fake.creates.load(Ordering::SeqCst), 0);
    let (_root2, session, fake) = setup(None);
    let checked = check(&session, form());
    session.dispatch("POST", "/api/finish", b"{}").unwrap();
    assert!(save(&session, &checked).is_err());
    assert_eq!(fake.creates.load(Ordering::SeqCst), 0);
}

#[test]
fn concurrent_operation_is_refused_without_side_effects() {
    let (_root, session, fake) = setup(None);
    let _guard = session.flow.lock().unwrap();
    assert!(
        session
            .dispatch("POST", "/api/test", &serde_json::to_vec(&form()).unwrap())
            .is_err()
    );
    assert_eq!(fake.checks.load(Ordering::SeqCst), 0);
}

#[test]
fn provider_presets_normalize_passwords_and_separate_icloud_logins() {
    let mut input = form();
    input["credential"]["password"] = json!("abcd efgh ijkl mnop");
    let mut draft: Draft = serde_json::from_value(input).unwrap();
    draft.validate().unwrap();
    let account = draft.account("fixture");
    assert_eq!(account.imap.unwrap().server, "imaps://imap.gmail.com:993");
    let mut input = form();
    input["provider"] = json!("icloud");
    input["email"] = json!("student@icloud.com");
    input["credential"]["password"] = json!("abcd-efgh-ijkl-mnop");
    let mut draft: Draft = serde_json::from_value(input).unwrap();
    draft.validate().unwrap();
    assert_eq!(draft.imap_login, "student");
    let account = draft.account("fixture");
    assert_eq!(
        account.smtp.as_ref().unwrap().server,
        "smtp://smtp.mail.me.com:587"
    );
    assert!(account.smtp.unwrap().starttls);
    assert_eq!(account.imap.unwrap().server, "imaps://imap.mail.me.com:993");
}

#[test]
fn malformed_addresses_normal_passwords_and_icloud_custom_domains_are_rejected() {
    for email in [
        "",
        "user",
        "student@evil\n.example",
        "student@example.test@evil.test",
        "student @example.test",
    ] {
        let mut input = form();
        input["email"] = json!(email);
        let mut draft: Draft = serde_json::from_value(input).unwrap();
        assert!(draft.validate().is_err());
    }
    let mut input = form();
    input["credential"]["password"] = json!("normal-password");
    let mut draft: Draft = serde_json::from_value(input).unwrap();
    assert!(draft.validate().is_err());
    let mut input = form();
    input["provider"] = json!("icloud");
    let mut draft: Draft = serde_json::from_value(input).unwrap();
    assert!(draft.validate().is_err());
}

#[test]
fn capability_has_256_random_bits_and_is_launch_specific() {
    let first = random_id().unwrap();
    assert_eq!(first.len(), 64);
    assert!(first.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_ne!(first, random_id().unwrap());
    let (_root, one, _) = setup(None);
    let (_root2, two, _) = setup(None);
    assert!(!two.authorized(&one.token));
}

fn request(session: &Session<Fake>) -> hyper::http::request::Builder {
    Request::builder()
        .method("POST")
        .uri("/api/test")
        .header("host", &session.authority)
        .header("origin", format!("http://{}", session.authority))
        .header("x-himalaya-setup-token", session.token.as_str())
        .header("content-type", "application/json")
        .header("content-length", "2")
}

#[test]
fn host_origin_capability_and_framing_are_all_required() {
    let (_root, session, fake) = setup(None);
    assert!(authorize(&request(&session).body(()).unwrap(), &session).is_ok());
    for (header, value, status) in [
        ("host", "evil.test", StatusCode::BAD_REQUEST),
        ("origin", "https://evil.test", StatusCode::FORBIDDEN),
        ("origin", "null", StatusCode::FORBIDDEN),
        ("x-himalaya-setup-token", "wrong", StatusCode::UNAUTHORIZED),
        (
            "content-type",
            "text/plain",
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        ("content-length", "999999", StatusCode::PAYLOAD_TOO_LARGE),
        ("transfer-encoding", "chunked", StatusCode::BAD_REQUEST),
    ] {
        let mut req = request(&session).body(()).unwrap();
        req.headers_mut().insert(header, value.parse().unwrap());
        assert_eq!(authorize(&req, &session), Err(status));
    }
    for header in [
        "host",
        "origin",
        "x-himalaya-setup-token",
        "content-length",
        "content-type",
    ] {
        let mut req = request(&session).body(()).unwrap();
        req.headers_mut().remove(header);
        assert!(authorize(&req, &session).is_err());
    }
    for header in ["host", "origin", "x-himalaya-setup-token", "content-length"] {
        let mut req = request(&session).body(()).unwrap();
        let val = req.headers()[header].clone();
        req.headers_mut().append(header, val);
        assert!(authorize(&req, &session).is_err());
    }
    assert_eq!(
        fake.reads.load(Ordering::SeqCst)
            + fake.checks.load(Ordering::SeqCst)
            + fake.creates.load(Ordering::SeqCst),
        0
    );
}

#[test]
fn static_assets_disclose_no_capability_and_get_cannot_mutate() {
    let (_root, session, _) = setup(None);
    let public = Request::builder()
        .uri("/")
        .header("host", &session.authority)
        .body(())
        .unwrap();
    assert!(authorize(&public, &session).is_ok());
    for uri in [
        "/api/save",
        "/api/test",
        "/api/finish",
        "/?token=secret",
        "http://evil.test/",
    ] {
        let req = Request::builder()
            .uri(uri)
            .header("host", &session.authority)
            .header("x-himalaya-setup-token", session.token.as_str())
            .body(())
            .unwrap();
        assert!(authorize(&req, &session).is_err());
    }
    let html = include_str!("assets/index.html");
    let js = include_str!("assets/app.js");
    assert!(!html.contains(session.token.as_str()));
    assert!(!js.contains("innerHTML"));
    assert!(!js.contains("localStorage"));
    assert!(!js.contains("sessionStorage"));
    assert!(js.contains("history.replaceState"));
}

#[test]
fn multiple_config_targets_are_not_ambiguous() {
    assert!(SetupTarget::open(&["one.toml".into(), "two.toml".into()]).is_err());
}

#[test]
fn cooperating_independent_targets_cannot_append_duplicate_accounts() {
    use std::{sync::Barrier, thread};
    for _ in 0..12 {
        let root = tempdir().unwrap();
        let path = root.path().join("config.toml");
        fs::write(&path, INITIAL).unwrap();
        let one = SetupTarget::open(from_ref(&path)).unwrap();
        let two = SetupTarget::open(from_ref(&path)).unwrap();
        let mut draft: Draft = serde_json::from_value(form()).unwrap();
        draft.validate().unwrap();
        let document = one.render(&draft.name, draft.account("fixture")).unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let threads: Vec<_> = [one, two]
            .into_iter()
            .map(|target| {
                let barrier = barrier.clone();
                let document = document.clone();
                thread::spawn(move || {
                    barrier.wait();
                    target.save(&document, &mut false, &|| Ok(())).is_ok()
                })
            })
            .collect();
        let successes = threads
            .into_iter()
            .map(|thread| usize::from(thread.join().unwrap()))
            .sum::<usize>();
        assert_eq!(successes, 1);
        let text = fs::read_to_string(path).unwrap();
        let parsed: Config = toml::from_str(&text).unwrap();
        assert_eq!(parsed.accounts.len(), 2);
        assert_eq!(text.matches("[accounts.university]").count(), 1);
    }
}

#[test]
fn revocation_after_validation_prevents_chmod_and_generated_writes() {
    use std::{os::unix::fs::PermissionsExt, sync::Barrier, thread};
    let (root, session, _) = setup(Some(INITIAL));
    let path = root.path().join("config.toml");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    let target = SetupTarget::open(from_ref(&path)).unwrap();
    let session = Arc::new(session);
    let ready = Arc::new(Barrier::new(2));
    let resume = Arc::new(Barrier::new(2));
    let worker_session = session.clone();
    let worker_ready = ready.clone();
    let worker_resume = resume.clone();
    let thread = thread::spawn(move || {
        let calls = AtomicUsize::new(0);
        let mut wrote = false;
        let result = target.save("[accounts.new]\n", &mut wrote, &|| {
            if calls.fetch_add(1, Ordering::SeqCst) == 1 {
                worker_ready.wait();
                worker_resume.wait();
            }
            if worker_session.live() {
                Ok(())
            } else {
                bail!("Revoked before commit")
            }
        });
        (result.is_err(), wrote)
    });
    ready.wait();
    session.revoke();
    resume.wait();
    assert_eq!(thread.join().unwrap(), (true, false));
    assert_eq!(fs::read_to_string(&path).unwrap(), INITIAL);
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o644
    );
}

#[test]
fn revocation_waits_for_the_commit_boundary_before_acknowledging_shutdown() {
    use std::{sync::mpsc, thread};
    let (_root, session, _) = setup(None);
    let session = Arc::new(session);
    let commit = session.commit.lock().unwrap();
    let other = session.clone();
    let (send, receive) = mpsc::channel();
    let thread = thread::spawn(move || {
        other.revoke();
        send.send(()).unwrap();
    });
    assert!(receive.recv_timeout(Duration::from_millis(30)).is_err());
    assert!(!session.closed.load(Ordering::SeqCst));
    drop(commit);
    receive.recv_timeout(Duration::from_secs(1)).unwrap();
    thread.join().unwrap();
    assert!(!session.live());
}

async fn raw_http(authority: String, request: String) -> String {
    tokio::task::spawn_blocking(move || {
        let mut socket = TcpStream::connect(authority).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(8)))
            .unwrap();
        socket.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        socket.read_to_string(&mut response).unwrap();
        response
    })
    .await
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_http_parser_rejects_attacks_without_side_effects_and_finish_stops_listener() {
    let (root, mut session, fake) = setup(None);
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    session.authority = listener.local_addr().unwrap().to_string();
    let session = Arc::new(session);
    let task = tokio::spawn(serve(listener, session.clone()));
    for extra in [
        "",
        "Origin: https://evil.test\r\n",
        "Origin: null\r\n",
        "Origin: http://127.0.0.1:1\r\n",
    ] {
        let reply = raw_http(session.authority.clone(), format!("POST /api/test HTTP/1.1\r\nHost: {}\r\n{extra}Content-Type: application/json\r\nContent-Length: 2\r\n\r\n{{}}", session.authority)).await;
        assert!(!reply.starts_with("HTTP/1.1 200"));
    }
    for headers in [
        "Host: evil.test\r\n".to_string(),
        format!("Host: {0}\r\nHost: {0}\r\n", session.authority),
        format!(
            "Host: {}\r\nTransfer-Encoding: chunked\r\nContent-Length: 2\r\n",
            session.authority
        ),
    ] {
        let reply = raw_http(session.authority.clone(), format!("POST /api/test HTTP/1.1\r\n{headers}Origin: http://{}\r\nContent-Type: application/json\r\n\r\n", session.authority)).await;
        assert!(!reply.starts_with("HTTP/1.1 200"));
    }
    assert!(!root.path().join("config.toml").exists());
    assert_eq!(
        fake.reads.load(Ordering::SeqCst)
            + fake.checks.load(Ordering::SeqCst)
            + fake.creates.load(Ordering::SeqCst),
        0
    );
    let reply = raw_http(
        session.authority.clone(),
        format!("GET / HTTP/1.1\r\nHost: {}\r\n\r\n", session.authority),
    )
    .await;
    assert!(reply.starts_with("HTTP/1.1 200"));
    assert!(reply.contains("content-security-policy:"));
    assert!(reply.contains("cache-control: no-store"));
    assert!(!reply.contains(session.token.as_str()));
    let reply = raw_http(session.authority.clone(), format!("POST /api/finish HTTP/1.1\r\nHost: {0}\r\nOrigin: http://{0}\r\nX-Himalaya-Setup-Token: {1}\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n{{}}", session.authority, session.token.as_str())).await;
    assert!(reply.contains("\"closed\":true"));
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(TcpStream::connect(&session.authority).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unauthorized_requests_do_not_extend_idle_lifetime() {
    let (_root, mut session, fake) = setup(None);
    session.idle_limit = Duration::from_millis(100);
    let last = *session.last_activity.lock().unwrap();
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    session.authority = listener.local_addr().unwrap().to_string();
    let session = Arc::new(session);
    let task = tokio::spawn(serve(listener, session.clone()));
    let _ = raw_http(
        session.authority.clone(),
        format!(
            "GET /api/status HTTP/1.1\r\nHost: {}\r\n\r\n",
            session.authority
        ),
    )
    .await;
    assert_eq!(*session.last_activity.lock().unwrap(), last);
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(fake.checks.load(Ordering::SeqCst), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn aborted_http_cannot_discard_creation_or_cleanup_recovery_outcomes() {
    for creation_fails in [true, false] {
        let (root, mut session, fake) = setup(None);
        let pause = Arc::new((Barrier::new(2), Barrier::new(2)));
        session.services.create_pause = Some(pause.clone());
        fake.fail_create.store(creation_fails, Ordering::SeqCst);
        fake.fail_cleanup.store(!creation_fails, Ordering::SeqCst);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        session.authority = listener.local_addr().unwrap().to_string();
        let session = Arc::new(session);
        let checked = check(&session, form());
        let body = json!({"ticket":checked["ticket"]}).to_string();
        let request = format!(
            "POST /api/save HTTP/1.1\r\nHost: {0}\r\nOrigin: http://{0}\r\nX-Himalaya-Setup-Token: {1}\r\nContent-Type: application/json\r\nContent-Length: {2}\r\n\r\n{3}",
            session.authority,
            session.token.as_str(),
            body.len(),
            body
        );
        let authority = session.authority.clone();
        let server = tokio::spawn(serve(listener, session.clone()));
        let client = tokio::task::spawn_blocking(move || {
            let mut socket = TcpStream::connect(authority).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            socket.write_all(request.as_bytes()).unwrap();
            let mut response = String::new();
            let _ = socket.read_to_string(&mut response);
            assert!(!response.contains("\"saved\""));
        });
        let ready = pause.clone();
        tokio::task::spawn_blocking(move || ready.0.wait())
            .await
            .unwrap();
        session.revoke();
        tokio::time::sleep(Duration::from_millis(250)).await;
        assert!(
            !server.is_finished(),
            "shutdown must retain the running credential operation"
        );
        assert_eq!(*session.operations.lock().unwrap(), 1);
        pause.1.wait();
        assert!(
            tokio::time::timeout(Duration::from_secs(3), server)
                .await
                .unwrap()
                .unwrap()
                .is_err()
        );
        client.await.unwrap();
        assert_eq!(*session.operations.lock().unwrap(), 0);
        let warnings = session.recovery.lock().unwrap();
        assert_eq!(warnings.len(), 1);
        assert!(!warnings[0].contains(PASSWORD));
        assert!(warnings[0].contains("himalaya-web-"));
        assert!(warnings[0].contains(if creation_fails {
            "creation"
        } else {
            "cleanup failed"
        }));
        assert!(!root.path().join("config.toml").exists());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn listener_error_revokes_and_settles_an_inflight_credential_creation() {
    use crate::wizard::web::http::SetupListener;
    use std::{io, net::SocketAddr};
    use tokio::{
        net::{TcpListener, TcpStream as AsyncTcpStream},
        sync::Notify,
    };

    struct FaultListener {
        inner: TcpListener,
        fail: Arc<Notify>,
    }
    impl SetupListener for FaultListener {
        async fn accept(&self) -> io::Result<(AsyncTcpStream, SocketAddr)> {
            tokio::select! {
                _ = self.fail.notified() => Err(io::Error::other("Injected accept failure")),
                connection = self.inner.accept() => connection,
            }
        }
    }
    let (root, mut session, fake) = setup(None);
    let pause = Arc::new((Barrier::new(2), Barrier::new(2)));
    session.services.create_pause = Some(pause.clone());
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    session.authority = listener.local_addr().unwrap().to_string();
    let fail = Arc::new(Notify::new());
    let session = Arc::new(session);
    let checked = check(&session, form());
    let body = json!({"ticket":checked["ticket"]}).to_string();
    let request = format!(
        "POST /api/save HTTP/1.1\r\nHost: {0}\r\nOrigin: http://{0}\r\nX-Himalaya-Setup-Token: {1}\r\nContent-Type: application/json\r\nContent-Length: {2}\r\n\r\n{3}",
        session.authority,
        session.token.as_str(),
        body.len(),
        body
    );
    let authority = session.authority.clone();
    let server = tokio::spawn(serve(
        FaultListener {
            inner: listener,
            fail: fail.clone(),
        },
        session.clone(),
    ));
    let client = tokio::task::spawn_blocking(move || {
        let mut socket = TcpStream::connect(authority).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        socket.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        let _ = socket.read_to_string(&mut response);
        assert!(!response.contains("\"saved\""));
    });
    let ready = pause.clone();
    tokio::task::spawn_blocking(move || ready.0.wait())
        .await
        .unwrap();
    fail.notify_one();
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(!session.live());
    assert!(!server.is_finished());
    pause.1.wait();
    let error = tokio::time::timeout(Duration::from_secs(3), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("Injected accept failure"));
    client.await.unwrap();
    assert_eq!(*session.operations.lock().unwrap(), 0);
    assert_eq!(fake.creates.load(Ordering::SeqCst), 1);
    assert_eq!(fake.rollbacks.load(Ordering::SeqCst), 1);
    assert!(!root.path().join("config.toml").exists());
}

#[test]
#[ignore = "Creates only a unique disposable native Keychain item; run explicitly"]
fn disposable_native_keychain_round_trip() {
    keychain::noninteractive().unwrap();
    let entry = format!("himalaya-web-fixture-{}", random_id().unwrap());
    let owned = keychain::create(&entry, PASSWORD).unwrap();
    let read = keychain::read(&entry);
    let duplicate = keychain::create(&entry, "ponmlkjihgfedcba");
    let cleanup = (owned.rollback)();
    assert_eq!(read.unwrap().as_str(), PASSWORD);
    assert!(duplicate.is_err());
    cleanup.unwrap();
    assert!(keychain::read(&entry).is_err());
}

#[test]
#[ignore = "Explicit native helper IPC test; requires HIMALAYA_WEB_TEST_BINARY"]
fn bounded_native_keychain_round_trip() {
    for password in [
        PASSWORD.to_string(),
        "通常の長さが異なるパスワード🔑".into(),
        "x".repeat(4096),
    ] {
        let entry = format!("himalaya-web-fixture-{}", random_id().unwrap());
        let owned = keychain::create_bounded(&entry, &password).unwrap();
        let read = keychain::read_bounded(&entry);
        let cleanup = (owned.rollback)();
        assert_eq!(read.unwrap().as_str(), password);
        cleanup.unwrap();
        assert!(keychain::read_bounded(&entry).is_err());
    }
}

#[test]
#[ignore = "Manual browser fixture; isolated target and fake services only"]
fn browser_fixture() {
    let (root, mut session, _) = setup(Some(INITIAL));
    session.token = Zeroizing::new("a".repeat(64));
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        session.authority = listener.local_addr().unwrap().to_string();
        println!(
            "Fixture origin: http://{} (test-only capability is 64 lowercase a characters)",
            session.authority
        );
        let start = Instant::now();
        let session = Arc::new(session);
        serve(listener, session.clone()).await.unwrap();
        if session.flow.lock().unwrap().saved.is_some() {
            let text = fs::read_to_string(root.path().join("config.toml")).unwrap();
            assert!(text.starts_with(INITIAL));
            assert!(!text.contains(PASSWORD));
            let config: Config = toml::from_str(&text).unwrap();
            assert!(config.accounts.len() >= 2);
            assert_eq!(text.matches("default = true").count(), 1);
        }
        assert!(start.elapsed() < Duration::from_secs(1801));
    });
}

#[test]
fn form_preview_redacts_but_explicit_save_preserves_selected_credential() {
    use crate::{
        config::{AccountConfig, GmailAuthConfig, GmailConfig},
        wizard::web::form,
    };
    use pimalaya_config::secret::Secret;
    let (root, session, fake) = setup(Some(INITIAL));
    let account = AccountConfig {
        gmail: Some(GmailConfig {
            user_id: "me".into(),
            tls: Default::default(),
            proxy: None,
            alpn: vec!["http/1.1".into()],
            auth: GmailAuthConfig {
                token: Secret::Raw("RAW_BROWSER_TEST_SECRET".into()),
            },
        }),
        ..Default::default()
    };
    let mut flow = session.flow.lock().unwrap();
    let document = flow.target.render("other", account).unwrap();
    let preview = form::preview(&document).unwrap();
    flow.ready = Some(form::Ready {
        ticket: "ticket".into(),
        name: "other".into(),
        document: Zeroizing::new(document),
        preview,
    });
    drop(flow);
    let poll = session.dispatch("POST", "/api/wizard/poll", b"{}").unwrap();
    assert!(!poll.to_string().contains("RAW_BROWSER_TEST_SECRET"));
    assert!(
        session
            .dispatch("POST", "/api/wizard/save", br#"{"ticket":"wrong"}"#)
            .is_err()
    );
    assert_eq!(
        fs::read_to_string(root.path().join("config.toml")).unwrap(),
        INITIAL
    );
    let saved = session
        .dispatch("POST", "/api/wizard/save", br#"{"ticket":"ticket"}"#)
        .unwrap();
    assert!(!saved.to_string().contains("RAW_BROWSER_TEST_SECRET"));
    assert!(
        session
            .dispatch("POST", "/api/wizard/save", br#"{"ticket":"ticket"}"#)
            .is_err()
    );
    let written = fs::read_to_string(root.path().join("config.toml")).unwrap();
    assert!(written.starts_with(INITIAL));
    assert!(written.contains("RAW_BROWSER_TEST_SECRET"));
    assert_eq!(written.matches("default = true").count(), 1);
    assert_eq!(fake.reads.load(Ordering::SeqCst), 0);
    assert_eq!(fake.creates.load(Ordering::SeqCst), 0);
}

#[test]
fn form_polls_and_rejected_answers_do_not_extend_idle_lifetime() {
    use crate::wizard::web::form::FormProcess;
    let (_root, mut session, _) = setup(None);
    session.idle_limit = Duration::from_millis(60);
    session.flow.lock().unwrap().wizard = Some(FormProcess::fixture("exec /bin/sleep 60"));
    let before = *session.last_activity.lock().unwrap();
    for _ in 0..8 {
        session.dispatch("GET", "/api/status", b"").unwrap();
        session.dispatch("POST", "/api/wizard/poll", b"{}").unwrap();
        assert!(
            session
                .dispatch(
                    "POST",
                    "/api/wizard/answer",
                    br#"{"generation":"wrong","id":1,"value":"secret"}"#
                )
                .is_err()
        );
    }
    assert_eq!(*session.last_activity.lock().unwrap(), before);
    std::thread::sleep(Duration::from_millis(70));
    assert!(!session.live());
    assert!(session.dispatch("POST", "/api/wizard/poll", b"{}").is_err());
    session.revoke();
    session.settle().unwrap();
    assert!(session.flow.lock().unwrap().wizard.is_none());
}

#[test]
fn finish_and_watchdog_stop_workers_without_waiting_for_provider_io() {
    use crate::wizard::web::form::FormProcess;
    let (_root, session, _) = setup(None);
    session.flow.lock().unwrap().wizard =
        Some(FormProcess::fixture("printf '{'; exec /bin/sleep 60"));
    let start = Instant::now();
    session.dispatch("POST", "/api/finish", b"{}").unwrap();
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(session.flow.lock().unwrap().wizard.is_none());
    assert!(!session.live());
}

#[test]
fn every_form_route_requires_the_same_capability_and_origin() {
    let (_root, session, fake) = setup(None);
    for path in [
        "wizard/start",
        "wizard/poll",
        "wizard/answer",
        "wizard/save",
        "wizard/cancel",
        "settings/discover",
        "settings/check",
        "settings/poll",
        "settings/cancel",
        "settings/save",
        "settings/result",
    ] {
        let request = |origin: &str, token: &str| {
            Request::builder()
                .method("POST")
                .uri(format!("/api/{path}"))
                .header("Host", &session.authority)
                .header("Origin", origin)
                .header("X-Himalaya-Setup-Token", token)
                .header("Content-Type", "application/json")
                .header("Content-Length", "2")
                .body(())
                .unwrap()
        };
        let origin = format!("http://{}", session.authority);
        assert!(authorize(&request(&origin, &session.token), &session).is_ok());
        assert_eq!(
            authorize(&request(&origin, "wrong"), &session),
            Err(StatusCode::UNAUTHORIZED)
        );
        assert_eq!(
            authorize(
                &request("https://foreign.example", &session.token),
                &session
            ),
            Err(StatusCode::FORBIDDEN)
        );
    }
    assert!(session.flow.lock().unwrap().wizard.is_none());
    assert_eq!(
        fake.reads.load(Ordering::SeqCst)
            + fake.checks.load(Ordering::SeqCst)
            + fake.creates.load(Ordering::SeqCst),
        0
    );
}

#[test]
fn settings_body_bounds_allow_long_escaped_credentials_but_reject_excess() {
    let (_root, session, _) = setup(None);
    for (length, accepted) in [(33285, true), (131072, true), (131073, false)] {
        let request = Request::builder()
            .method("POST")
            .uri("/api/settings/check")
            .header("Host", &session.authority)
            .header("Origin", format!("http://{}", session.authority))
            .header("X-Himalaya-Setup-Token", &*session.token)
            .header("Content-Type", "application/json")
            .header("Content-Length", length.to_string())
            .body(())
            .unwrap();
        assert_eq!(authorize(&request, &session).is_ok(), accepted);
    }
}

#[test]
fn generated_block_renaming_preserves_literals_and_default_rules() {
    let (_root, session, _) = setup(None);
    let source = "[accounts.base]\nemail = \"literal$UNDEFINED@example.test\"\nmaildir.root = \"/tmp/$UNDEFINED\"\n";
    let flow = session.flow.lock().unwrap();
    let rendered = flow
        .target
        .render_generated("renamed", "base", source)
        .unwrap();
    assert!(rendered.contains("literal$UNDEFINED@example.test"));
    assert!(rendered.contains("/tmp/$UNDEFINED"));
    assert_eq!(rendered.matches("default = true").count(), 1);
    let parsed: toml::Value = toml::from_str(&rendered).unwrap();
    assert!(parsed["accounts"].get("renamed").is_some());
    assert!(
        flow.target
            .render_generated(
                "renamed",
                "base",
                "[accounts.base]\n[accounts.base.maildir]\nroot = '/tmp/fixture'\n"
            )
            .is_err()
    );
    assert!(
        flow.target
            .render_generated("renamed", "other", source)
            .is_err()
    );
    assert!(
        flow.target
            .render_generated("renamed", "base", "[accounts.base]\ndefault = true\n")
            .is_err()
    );
}

#[test]
fn watchdog_stops_expired_worker_without_any_browser_poll() {
    use crate::wizard::web::form::FormProcess;
    let (_root, session, _) = setup(None);
    let mut worker = FormProcess::fixture("exec /bin/sleep 60");
    worker.expire_for_test();
    session.flow.lock().unwrap().wizard = Some(worker);
    session.watchdog();
    let flow = session.flow.lock().unwrap();
    assert!(flow.wizard.is_none());
    assert!(flow.wizard_error.as_ref().unwrap().contains("タイムアウト"));
}

#[test]
fn http_timer_stops_stalled_worker_without_requests() {
    use crate::wizard::web::form::FormProcess;
    let (_root, session, _) = setup(None);
    let mut worker = FormProcess::fixture("exec /bin/sleep 60");
    worker.expire_for_test();
    session.flow.lock().unwrap().wizard = Some(worker);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        let session = Arc::new(session);
        let task = tokio::spawn(serve(listener, session.clone()));
        let deadline = Instant::now() + Duration::from_secs(2);
        while session.flow.lock().unwrap().wizard.is_some() {
            assert!(
                Instant::now() < deadline,
                "HTTP timer must enforce the watchdog without polls"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert!(session.flow.lock().unwrap().wizard_error.is_some());
        session.revoke();
        task.await.unwrap().unwrap();
    });
}

#[test]
#[ignore = "Isolated process required: external TERM/HUP teardown test"]
fn termination_fixture() {
    use crate::wizard::web::form::FormProcess;
    let (_root, mut session, _) = setup(None);
    session.token = Zeroizing::new("a".repeat(64));
    let worker = FormProcess::fixture("exec /bin/sleep 60");
    let worker_pid = worker.process_id();
    session.flow.lock().unwrap().wizard = Some(worker);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async move {
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .unwrap();
        session.authority = listener.local_addr().unwrap().to_string();
        let session = Arc::new(session);
        let task = tokio::spawn(serve(listener, session.clone()));
        tokio::time::sleep(Duration::from_millis(100)).await;
        println!("TERMINATION_FIXTURE_READY {worker_pid}");
        task.await.unwrap().unwrap();
        assert!(!session.live());
        assert!(session.flow.lock().unwrap().wizard.is_none());
        println!("TERMINATION_FIXTURE_SETTLED");
    });
}
