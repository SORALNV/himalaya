//! Typed, editable account settings and bounded discovery/check operations.

use anyhow::{Result, anyhow, bail};
use pimalaya_cli::wizard::keyring::{KeyringProvider, TokenBroker};
use pimalaya_config::{command::CommandConfig, secret::Secret};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    io::{BufRead, Read, Write, stdin, stdout},
    path::Path,
};
use url::Url;
use zeroize::Zeroizing;

use crate::{
    account::check,
    config::*,
    wizard::{
        prompt::CheckConfig,
        search::{self, DiscoveredKind},
    },
};

/// A field-specific, static error safe to return to a browser.
#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    pub field: &'static str,
    pub error: &'static str,
}

fn invalid(field: &'static str, error: &'static str) -> FieldError {
    FieldError { field, error }
}

/// A credential source chosen explicitly by the user.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Credential {
    pub source: String,
    #[serde(default)]
    pub value: Zeroizing<String>,
    #[serde(default)]
    pub confirm: Zeroizing<String>,
    #[serde(default)]
    pub entry: String,
    #[serde(default)]
    pub approved: bool,
}

impl Default for Credential {
    fn default() -> Self {
        Self {
            source: "new".into(),
            value: Zeroizing::new(String::new()),
            confirm: Zeroizing::new(String::new()),
            entry: String::new(),
            approved: false,
        }
    }
}

impl Credential {
    fn validate(&self, field: &'static str) -> std::result::Result<(), FieldError> {
        if self.value.len() > 4096
            || self.confirm.len() > 4096
            || self.entry.len() > 1024
            || self.value.chars().any(char::is_control)
            || self.entry.chars().any(char::is_control)
        {
            return Err(invalid(
                field,
                "認証情報が長すぎるか、使用できない文字を含んでいます。",
            ));
        }
        match self.source.as_str() {
            "new" if self.value.is_empty() => Err(invalid(
                field,
                "パスワードまたはトークンを入力してください。",
            )),
            "raw" if !self.approved => Err(invalid(field, "平文保存の警告を確認してください。")),
            "raw" if self.value.is_empty() || self.value.as_str() != self.confirm.as_str() => {
                Err(invalid(field, "秘密の入力と確認が一致しません。"))
            }
            "custom" if !self.approved || self.entry.trim().is_empty() => Err(invalid(
                field,
                "コマンドを入力し、実行の警告を確認してください。",
            )),
            "security" | "pass" | "ortie" | "pizauth" | "oama" if self.entry.is_empty() => {
                Err(invalid(
                    field,
                    "保存済みの項目名または管理ツールのアカウント名を入力してください。",
                ))
            }
            "new" | "raw" | "custom" | "security" | "pass" | "ortie" | "pizauth" | "oama" => Ok(()),
            _ => Err(invalid(field, "認証情報の取得方法を選んでください。")),
        }
    }

    fn secret(&self, entry: &str, checking: bool) -> Secret {
        if self.source == "raw" || self.source == "new" && checking {
            return Secret::Raw(self.value.to_string().into());
        }
        let command = match self.source.as_str() {
            "new" => CommandConfig::Argv {
                program: "/usr/bin/security".into(),
                args: vec![
                    "find-generic-password".into(),
                    "-a".into(),
                    entry.into(),
                    "-s".into(),
                    entry.into(),
                    "-w".into(),
                ],
            },
            "custom" => CommandConfig::Shell(self.entry.clone()),
            source => {
                let argv = match source {
                    "security" => KeyringProvider::Security.read_command(None, &self.entry),
                    "pass" => KeyringProvider::Pass.read_command(None, &self.entry),
                    "ortie" => TokenBroker::Ortie.read_command(&self.entry),
                    "pizauth" => TokenBroker::Pizauth.read_command(&self.entry),
                    "oama" => TokenBroker::Oama.read_command(&self.entry),
                    _ => unreachable!("validated credential source"),
                };
                CommandConfig::Argv {
                    program: argv[0].clone(),
                    args: argv[1..].to_vec(),
                }
            }
        };
        Secret::Command(command)
    }
}

/// Editable receive/send settings; transport and authentication remain typed.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Connection {
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default = "tls")]
    pub encryption: String,
    #[serde(default)]
    pub login: String,
    #[serde(default = "plain")]
    pub mechanism: String,
    #[serde(default)]
    pub credential: Credential,
}
fn tls() -> String {
    "tls".into()
}
fn plain() -> String {
    "plain".into()
}
impl Default for Connection {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 993,
            encryption: tls(),
            login: String::new(),
            mechanism: plain(),
            credential: Credential::default(),
        }
    }
}

impl Connection {
    fn validate(
        &self,
        field: &'static str,
        credential: bool,
    ) -> std::result::Result<(), FieldError> {
        if self.host.is_empty()
            || self.host.len() > 253
            || self
                .host
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
            || self.host.contains(['/', '@', '?', '#', '\\', '%'])
            || !["tls", "starttls"].contains(&self.encryption.as_str())
            || self.port == 0
        {
            return Err(invalid(
                field,
                "サーバー名・ポート・暗号化方式（TLSまたはSTARTTLS）を確認してください。",
            ));
        }
        let url = self.url("imap");
        if Url::parse(&url).ok().is_none_or(|u| {
            u.host_str()
                .is_none_or(|h| !h.eq_ignore_ascii_case(&self.host))
                || u.port() != Some(self.port)
                || !u.username().is_empty()
                || u.password().is_some()
        }) {
            return Err(invalid(
                field,
                "有効なサーバー名を入力してください。URLではなくホスト名だけを指定します。",
            ));
        }
        if ![
            "plain",
            "login",
            "scram-sha-256",
            "oauthbearer",
            "xoauth2",
            "anonymous",
        ]
        .contains(&self.mechanism.as_str())
        {
            return Err(invalid(field, "認証方式を選んでください。"));
        }
        if credential && self.mechanism != "anonymous" {
            if !valid_login(&self.login) {
                return Err(invalid(field, "ログイン名を入力してください。"));
            }
            self.credential.validate(if field == "smtp" {
                "smtp-credential"
            } else {
                "credential"
            })?;
        }
        Ok(())
    }
    fn url(&self, protocol: &str) -> String {
        format!(
            "{}{}://{}:{}",
            protocol,
            if self.encryption == "tls" { "s" } else { "" },
            self.host,
            self.port
        )
    }
    fn sasl(&self, entry: &str, checking: bool) -> SaslConfig {
        let secret = || self.credential.secret(entry, checking);
        match self.mechanism.as_str() {
            "plain" => SaslConfig::Plain(SaslPlainConfig {
                authzid: None,
                authcid: self.login.clone(),
                passwd: secret(),
            }),
            "login" => SaslConfig::Login(SaslLoginConfig {
                username: self.login.clone(),
                password: secret(),
            }),
            "scram-sha-256" => SaslConfig::ScramSha256(SaslScramSha256Config {
                username: self.login.clone(),
                password: secret(),
            }),
            "oauthbearer" => SaslConfig::Oauthbearer(SaslOauthbearerConfig {
                username: self.login.clone(),
                token: secret(),
            }),
            "xoauth2" => SaslConfig::Xoauth2(SaslXoauth2Config {
                username: self.login.clone(),
                token: secret(),
            }),
            "anonymous" => SaslConfig::Anonymous(SaslAnonymousConfig { message: None }),
            _ => unreachable!("validated mechanism"),
        }
    }
}

fn valid_login(s: &str) -> bool {
    !s.is_empty() && s.len() <= 1024 && !s.chars().any(char::is_control)
}

/// One complete draft, never a raw TOML or arbitrary command request.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    pub name: String,
    #[serde(default)]
    pub email: String,
    pub backend: String,
    #[serde(default)]
    pub receive: Connection,
    #[serde(default)]
    pub smtp: Connection,
    #[serde(default)]
    pub smtp_enabled: bool,
    #[serde(default = "yes")]
    pub smtp_shared: bool,
    #[serde(default)]
    pub server: String,
    #[serde(default = "me")]
    pub user_id: String,
    #[serde(default)]
    pub root: String,
    /// Plain labels grouping the account, trimmed and deduplicated on save.
    #[serde(default)]
    pub groups: Vec<String>,
    #[serde(default)]
    pub destination_confirmed: bool,
    #[serde(default)]
    pub legacy_mprg: Option<LegacyDraft>,
}

/// Separate explicit approval for obsolete TLS and initial pin trust.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyDraft {
    pub imap_sha256: String,
    #[serde(default)]
    pub smtp_sha256: String,
    pub approved: bool,
}

/// Public pin observed during credential-free diagnosis, never silently updated.
pub fn legacy_metadata() -> Value {
    json!({"available":cfg!(all(feature="legacy-mprg",target_os="macos")),"host":"mail.mprg.cs.chubu.ac.jp","observed_sha256":"de1b5b799cbadd8f56540697340381683008435a4bd42488fb71c0f1123ddd4c","observed_date":"2026-10-18"})
}
fn yes() -> bool {
    true
}
fn me() -> String {
    "me".into()
}

/// Backend choices reflect compiled capabilities, not a provider allowlist.
pub fn backends() -> Vec<&'static str> {
    [
        ("imap", cfg!(feature = "imap")),
        ("jmap", cfg!(feature = "jmap")),
        ("gmail", cfg!(feature = "gmail")),
        ("msgraph", cfg!(feature = "msgraph")),
        ("maildir", cfg!(feature = "maildir")),
        ("m2dir", cfg!(feature = "m2dir")),
        ("mbox", cfg!(feature = "mbox")),
    ]
    .into_iter()
    .filter_map(|(name, on)| on.then_some(name))
    .collect()
}

pub fn sources() -> Value {
    json!([
        {"id":"new","label":"入力した秘密をKeychainに保存","installed":true},
        {"id":"security","label":"保存済みのKeychain項目","installed":KeyringProvider::Security.installed()},
        {"id":"pass","label":"pass（パスワードストア）","installed":KeyringProvider::Pass.installed()},
        {"id":"ortie","label":"Ortie（OAuthトークン管理）","installed":TokenBroker::Ortie.installed()},
        {"id":"pizauth","label":"pizauth（OAuthトークン管理）","installed":TokenBroker::Pizauth.installed()},
        {"id":"oama","label":"oama（OAuthトークン管理）","installed":TokenBroker::Oama.installed()},
        {"id":"custom","label":"任意コマンド（詳細）","installed":true},
        {"id":"raw","label":"平文で保存（非推奨）","installed":true}
    ])
}

impl Draft {
    pub fn validate(&self) -> std::result::Result<(), FieldError> {
        if self.name.is_empty()
            || self.name.len() > 64
            || !self
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        {
            return Err(invalid(
                "name",
                "アカウント名を半角英数字・ハイフン・アンダースコアで入力してください。",
            ));
        }
        let local = ["maildir", "m2dir", "mbox"].contains(&self.backend.as_str());
        if !backends().contains(&self.backend.as_str()) {
            return Err(invalid(
                "backend",
                "この実行ファイルで利用できる接続方式を選んでください。",
            ));
        }
        if !local && !valid_email(&self.email) {
            return Err(invalid("email", "メールアドレスを入力してください。"));
        }
        if self.groups.len() > 8
            || self.groups.iter().any(|g| {
                g.is_empty() || g.len() > 32 || g.chars().any(char::is_control) || g.contains(',')
            })
        {
            return Err(invalid(
                "groups",
                "グループは32文字以内・カンマを含まない名前で、8個までにしてください。",
            ));
        }
        if !self.destination_confirmed {
            return Err(invalid("destination", "表示した接続先を確認してください。"));
        }
        if let Some(legacy) = &self.legacy_mprg {
            if self.backend != "imap" || !legacy.approved {
                return Err(invalid(
                    "legacy",
                    "旧TLSと証明書の初回固定のリスクを確認し、IMAP接続で明示的に許可してください。",
                ));
            }
            let receive = Url::parse(&self.receive.url("imap"))
                .map_err(|_| invalid("legacy", "研究室メールの接続先を確認してください。"))?;
            if self
                .tls(false)
                .validate_endpoint(&receive, self.receive.encryption == "starttls")
                .is_err()
            {
                return Err(invalid(
                    "legacy",
                    "互換モードはMPRG受信143/STARTTLSと有効な証明書SHA-256の組合せ専用です。",
                ));
            }
            if self.smtp_enabled {
                let smtp = Url::parse(&self.smtp.url("smtp"))
                    .map_err(|_| invalid("legacy", "送信接続先を確認してください。"))?;
                if self
                    .tls(true)
                    .validate_endpoint(&smtp, self.smtp.encryption == "starttls")
                    .is_err()
                {
                    return Err(invalid(
                        "legacy",
                        "互換モードはMPRG送信587/STARTTLSと有効な証明書SHA-256の組合せ専用です。",
                    ));
                }
            }
        }
        match self.backend.as_str() {
            "imap" => {
                self.receive.validate("receive", true)?;
                if self.smtp_enabled {
                    self.smtp.validate("smtp", !self.smtp_shared)?;
                }
            }
            "jmap" => {
                let parsed = Url::parse(&self.server)
                    .map_err(|_| invalid("server", "HTTPSのJMAPサーバーURLを入力してください。"))?;
                if parsed.scheme() != "https"
                    || parsed.host_str().is_none()
                    || !parsed.username().is_empty()
                    || parsed.password().is_some()
                    || parsed.query().is_some()
                    || parsed.fragment().is_some()
                    || self.server.len() > 2048
                    || self
                        .server
                        .chars()
                        .any(|c| c.is_control() || c.is_whitespace())
                    || self.server.contains('\\')
                {
                    return Err(invalid(
                        "server",
                        "JMAPには認証情報やクエリを含まないHTTPS URLを指定してください。",
                    ));
                }
                if !["basic", "bearer", "header"].contains(&self.receive.mechanism.as_str()) {
                    return Err(invalid("mechanism", "JMAPの認証方式を選んでください。"));
                }
                if self.receive.mechanism == "basic" && !valid_login(&self.receive.login) {
                    return Err(invalid("login", "ログイン名を入力してください。"));
                }
                self.receive.credential.validate("credential")?;
            }
            "gmail" | "msgraph" => {
                if !valid_login(&self.user_id) {
                    return Err(invalid(
                        "user-id",
                        "ユーザーIDを入力してください（通常はme）。",
                    ));
                }
                self.receive.credential.validate("credential")?;
            }
            _ => {
                if self.root.is_empty()
                    || self.root.len() > 4096
                    || !Path::new(&self.root).is_absolute()
                    || self.root.chars().any(char::is_control)
                {
                    return Err(invalid(
                        "root",
                        "既存メールフォルダーの絶対パスを入力してください。",
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn active_credentials(&self) -> Vec<&Credential> {
        let mut credentials = vec![];
        if ["jmap", "gmail", "msgraph"].contains(&self.backend.as_str())
            || self.backend == "imap" && self.receive.mechanism != "anonymous"
        {
            credentials.push(&self.receive.credential);
        }
        if self.backend == "imap"
            && self.smtp_enabled
            && !self.smtp_shared
            && self.smtp.mechanism != "anonymous"
        {
            credentials.push(&self.smtp.credential);
        }
        credentials
    }

    fn tls(&self, smtp: bool) -> TlsConfig {
        TlsConfig {
            provider: self
                .legacy_mprg
                .as_ref()
                .map(|legacy| TlsProviderConfig::LegacyMprg {
                    sha256: if smtp {
                        legacy.smtp_sha256.clone()
                    } else {
                        legacy.imap_sha256.clone()
                    },
                }),
            ..Default::default()
        }
    }

    /// Builds a typed account, never resolving an existing command here.
    pub fn account(&self, entries: &[String], checking: bool) -> AccountConfig {
        let mut account = AccountConfig {
            email: (!self.email.is_empty()).then(|| self.email.clone()),
            groups: {
                let mut groups: Vec<String> = Vec::new();
                for label in &self.groups {
                    let label = label.trim();
                    if !label.is_empty() && !groups.iter().any(|g| g == label) {
                        groups.push(label.to_owned());
                    }
                }
                groups
            },
            ..Default::default()
        };
        let entry = |index| entries.get(index).map(String::as_str).unwrap_or("");
        let credential = || self.receive.credential.secret(entry(0), checking);
        match self.backend.as_str() {
            "imap" => {
                let sasl = self.receive.sasl(entry(0), checking);
                account.imap = Some(ImapConfig {
                    server: self.receive.url("imap"),
                    tls: self.tls(false),
                    proxy: None,
                    starttls: self.receive.encryption == "starttls",
                    alpn: vec!["imap".into()],
                    sasl: Some(sasl.clone()),
                    sasl_ir: None,
                    id: Default::default(),
                    sort: Default::default(),
                });
                if self.smtp_enabled {
                    let smtp_sasl = if self.smtp_shared {
                        sasl
                    } else {
                        self.smtp.sasl(
                            entry(usize::from(self.receive.mechanism != "anonymous")),
                            checking,
                        )
                    };
                    account.smtp = Some(SmtpConfig {
                        server: self.smtp.url("smtp"),
                        tls: self.tls(true),
                        proxy: None,
                        starttls: self.smtp.encryption == "starttls",
                        alpn: vec!["smtp".into()],
                        sasl: Some(smtp_sasl),
                    });
                }
            }
            "jmap" => {
                account.jmap = Some(JmapConfig {
                    server: self.server.clone(),
                    tls: Default::default(),
                    proxy: None,
                    alpn: vec!["http/1.1".into()],
                    auth: match self.receive.mechanism.as_str() {
                        "basic" => JmapAuthConfig::Basic {
                            username: self.receive.login.clone(),
                            password: credential(),
                        },
                        "header" => JmapAuthConfig::Header(credential()),
                        _ => JmapAuthConfig::Bearer {
                            token: credential(),
                        },
                    },
                    identity_id: None,
                    drafts_mailbox_id: None,
                    sent_mailbox_id: None,
                });
            }
            "gmail" => {
                account.gmail = Some(GmailConfig {
                    user_id: self.user_id.clone(),
                    tls: Default::default(),
                    proxy: None,
                    alpn: vec!["http/1.1".into()],
                    auth: GmailAuthConfig {
                        token: credential(),
                    },
                });
            }
            "msgraph" => {
                account.msgraph = Some(MsgraphConfig {
                    user_id: self.user_id.clone(),
                    tls: Default::default(),
                    proxy: None,
                    alpn: vec!["http/1.1".into()],
                    auth: MsgraphAuthConfig {
                        token: credential(),
                    },
                });
            }
            "maildir" => {
                account.maildir = Some(MaildirConfig {
                    root: self.root.clone().into(),
                    keywords: Default::default(),
                })
            }
            "m2dir" => {
                account.m2dir = Some(M2dirConfig {
                    root: self.root.clone().into(),
                })
            }
            "mbox" => {
                account.mbox = Some(MboxConfig {
                    root: self.root.clone().into(),
                    inbox: None,
                    format: Default::default(),
                    thunderbird: false,
                    lock: Default::default(),
                })
            }
            _ => unreachable!("validated backend"),
        }
        account
    }
}

pub fn valid_email(email: &str) -> bool {
    email.len() <= 254
        && email.is_ascii()
        && !email
            .bytes()
            .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty()
                && !local.contains(':')
                && !domain.contains('@')
                && domain.contains('.')
                && domain
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
        })
}

/// Only a server-built worker operation crosses the private pipe.
#[derive(Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "lowercase", deny_unknown_fields)]
pub enum Operation {
    Discover {
        email: String,
    },
    Check {
        draft: Box<Draft>,
        entries: Vec<String>,
    },
}

pub fn execute_worker() -> Result<()> {
    let mut bytes = Zeroizing::new(Vec::new());
    stdin().lock().take(262145).read_until(b'\n', &mut bytes)?;
    if bytes.len() > 262144 || !bytes.ends_with(b"\n") {
        bail!("Invalid private settings input");
    }
    let operation: Operation =
        serde_json::from_slice(&bytes).map_err(|_| anyhow!("Invalid settings operation"))?;
    let result = match operation {
        Operation::Discover { email } => {
            if !valid_email(&email) {
                json!({"field":"email","error":"メールアドレスを確認してください。"})
            } else {
                match search::search(&email) {
                    Ok(found) => {
                        let choices:Vec<Value> = found.into_iter().filter_map(|found| match found.kind {
                    DiscoveredKind::ImapSmtp {imap,smtp} if backends().contains(&"imap") => {
                        let encryption = |s| match s { io_pim_discovery::compose::config::DiscoverySecurity::Tls=>"tls",io_pim_discovery::compose::config::DiscoverySecurity::Starttls=>"starttls",_=>"unsupported" };
                        Some(json!({"backend":"imap","label":format!("IMAP / SMTP · {}",imap.host),"receive":{"host":imap.host,"port":imap.port,"encryption":encryption(imap.security),"login":found.username.unwrap_or_else(||email.clone())},"smtp":smtp.map(|s|json!({"host":s.host,"port":s.port,"encryption":encryption(s.security),"login":email} ))}))
                    }
                    DiscoveredKind::Jmap(url) if backends().contains(&"jmap") => Some(json!({"backend":"jmap","label":"JMAP","server":url})),
                    DiscoveredKind::Gmail if backends().contains(&"gmail") => Some(json!({"backend":"gmail","label":"Gmail API（OAuthトークン）"})),
                    DiscoveredKind::Msgraph if backends().contains(&"msgraph") => Some(json!({"backend":"msgraph","label":"Microsoft Graph（OAuthトークン）"})),
                    _=>None,
                }).collect();
                        json!({"choices":choices})
                    }
                    Err(_) => {
                        json!({"field":"email","error":"自動取得できませんでした。接続設定を手入力できます。"})
                    }
                }
            }
        }
        Operation::Check { draft, entries } => {
            if let Err(error) = draft.validate() {
                serde_json::to_value(error)?
            } else if entries.len() != draft.active_credentials().len()
                || entries.iter().any(|e| {
                    !e.starts_with("himalaya-web-")
                        || e.len() != 77
                        || !e[13..].bytes().all(|b| b.is_ascii_hexdigit())
                })
            {
                bail!("Invalid private credential references");
            } else {
                let mut account = draft.account(&entries, true);
                if account.prepare().is_err() {
                    json!({"field":"credential","error":"認証情報を取得できません。項目名・ツールの導入・アクセス許可を確認してください。"})
                } else if !safe_http_auth(&account) {
                    json!({"field":"credential","error":"HTTP認証情報に使用できない文字が含まれています。"})
                } else if draft.backend == "imap" {
                    let mut resolver = pimalaya_config::secret::SecretResolver::new();
                    let received = check::connect_imap(
                        account.imap.as_ref().expect("IMAP draft"),
                        &mut resolver,
                    );
                    if let Err(error) = received {
                        connection_error("receive", &error)
                    } else if let Some(error) = account
                        .smtp
                        .as_ref()
                        .and_then(|smtp| check::connect_smtp(smtp, &mut resolver).err())
                    {
                        connection_error("smtp", &error)
                    } else {
                        json!({"ok":true})
                    }
                } else if check::test_account(&account).is_err() {
                    json!({"field":"connection","error":"接続を確認できません。接続先・認証情報・利用条件を確認してください。"})
                } else {
                    json!({"ok":true})
                }
            }
        }
    };
    let output = Zeroizing::new(serde_json::to_vec(&json!({"type":"data","data":result}))?);
    if output.len() > 60000 {
        bail!("Settings output exceeds limit");
    }
    let mut out = stdout().lock();
    out.write_all(&output)?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}

fn connection_error(field: &'static str, error: &anyhow::Error) -> Value {
    let pin_error = error
        .chain()
        .any(|cause| cause.to_string().contains("Legacy TLS leaf pin mismatch"));
    let handshake_error = error.chain().any(|cause| {
        cause
            .to_string()
            .contains("Pinned legacy TLS handshake failed")
    });
    let message = if pin_error {
        "旧TLSの証明書指紋が一致しません。認証送信前に中止しました。証明書の変更を明示的に確認してください。"
    } else if handshake_error {
        "旧TLS接続に失敗しました。パスワード認証前のエラーです。接続先と互換モードを確認してください。"
    } else if field == "receive" {
        "受信接続に失敗しました。サーバー・ログイン名・認証情報を確認してください。"
    } else {
        "送信接続に失敗しました。SMTP設定または認証情報を確認してください。"
    };
    json!({"field":field,"error":message})
}

// Commands are resolved before this check. A custom Authorization value must
// never introduce HTTP header lines, including when it came from a broker.
fn safe_http_auth(account: &AccountConfig) -> bool {
    use secrecy::ExposeSecret;
    let safe = |secret: &Secret| match secret {
        Secret::Raw(value) => !value.expose_secret().chars().any(char::is_control),
        _ => false,
    };
    account.jmap.as_ref().is_none_or(|jmap| match &jmap.auth {
        JmapAuthConfig::Basic { .. } => true,
        JmapAuthConfig::Bearer { token } | JmapAuthConfig::Header(token) => safe(token),
    }) && account
        .gmail
        .as_ref()
        .is_none_or(|gmail| safe(&gmail.auth.token))
        && account
            .msgraph
            .as_ref()
            .is_none_or(|graph| safe(&graph.auth.token))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn draft() -> Draft {
        serde_json::from_value(json!({"name":"university","email":"student@university.example","backend":"imap","destination_confirmed":true,"receive":{"host":"imap.university.example","port":993,"login":"student@university.example","credential":{"source":"new","value":"長い通常のパスワード🔑"}}})).unwrap()
    }

    #[test]
    fn legacy_web_consent_is_required_and_binds_both_endpoint_pins() {
        let mut d = draft();
        d.receive.host = "mail.mprg.cs.chubu.ac.jp".into();
        d.receive.port = 143;
        d.receive.encryption = "starttls".into();
        d.legacy_mprg = Some(LegacyDraft {
            imap_sha256: "ab".repeat(32),
            smtp_sha256: "cd".repeat(32),
            approved: false,
        });
        assert_eq!(d.validate().unwrap_err().field, "legacy");
        d.legacy_mprg.as_mut().unwrap().approved = true;
        if !cfg!(all(feature = "legacy-mprg", target_os = "macos")) {
            assert!(d.validate().is_err());
            return;
        }
        d.validate().unwrap();
        let account = d.account(&["fixture-entry".into()], false);
        let text = toml::to_string(&account).unwrap();
        assert!(text.contains("legacy-mprg") && text.contains(&"ab".repeat(32)));
        d.smtp_enabled = true;
        d.smtp.host = "mail.mprg.cs.chubu.ac.jp".into();
        d.smtp.port = 587;
        d.smtp.encryption = "starttls".into();
        d.validate().unwrap();
        let text = toml::to_string(&d.account(&["fixture-entry".into()], false)).unwrap();
        assert!(text.contains(&"cd".repeat(32)));
        d.smtp.host = "smtp.other.example".into();
        assert!(d.validate().is_err());
        d.smtp_enabled = false;
        d.receive.encryption = "tls".into();
        assert!(d.validate().is_err());
    }

    #[test]
    fn legacy_errors_do_not_echo_raw_provider_or_helper_output() {
        let error = anyhow!("Legacy TLS leaf pin mismatch; PRIVATE_TEST_MARKER");
        let result = connection_error("receive", &error);
        assert!(result["error"].as_str().unwrap().contains("認証送信前"));
        assert!(!result.to_string().contains("PRIVATE_TEST_MARKER"));
        let result = connection_error(
            "smtp",
            &anyhow!("Pinned legacy TLS handshake failed before authentication"),
        );
        assert!(
            result["error"]
                .as_str()
                .unwrap()
                .contains("パスワード認証前")
        );
    }
    #[test]
    fn generic_servers_and_unicode_credentials_are_not_provider_restricted() {
        let d = draft();
        d.validate().unwrap();
        let saved = d.account(&["himalaya-web-example".into()], false);
        let text = toml::to_string(&saved).unwrap();
        assert!(text.contains("imap.university.example"));
        assert!(!text.contains("長い通常"));
        assert!(text.contains("himalaya-web-example"));
        assert!(
            toml::to_string(&d.account(&[], true))
                .unwrap()
                .contains("長い通常")
        );
    }
    #[test]
    fn destination_consent_and_strict_transport_are_required() {
        let mut d = draft();
        d.destination_confirmed = false;
        assert_eq!(d.validate().unwrap_err().field, "destination");
        d.destination_confirmed = true;
        for host in [
            "user@evil.example",
            "mail.example:444",
            "mail.example/path",
            "mail.example\n",
            "mail.example%00",
            "https://mail.example",
        ] {
            d.receive.host = host.into();
            assert!(d.validate().is_err(), "{host}");
        }
        d.receive.host = "imap.example.test".into();
        d.receive.encryption = "none".into();
        assert!(d.validate().is_err());
    }
    #[test]
    fn custom_and_raw_require_separate_explicit_consent() {
        let mut d = draft();
        d.receive.credential.source = "custom".into();
        d.receive.credential.entry = "printf example".into();
        assert!(d.validate().is_err());
        d.receive.credential.approved = true;
        d.validate().unwrap();
        d.receive.credential.source = "raw".into();
        assert!(d.validate().is_err());
        d.receive.credential.confirm = d.receive.credential.value.clone();
        d.validate().unwrap();
    }
    #[test]
    fn smtp_independent_and_shared_credentials_map_to_correct_entries() {
        let mut d = draft();
        d.smtp_enabled = true;
        d.smtp_shared = false;
        d.smtp = Connection {
            host: "smtp.example.test".into(),
            port: 587,
            encryption: "starttls".into(),
            login: "sender@example.test".into(),
            credential: Credential {
                value: Zeroizing::new("smtp-password".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        d.validate().unwrap();
        assert_eq!(d.active_credentials().len(), 2);
        let text =
            toml::to_string(&d.account(&["receive-entry".into(), "send-entry".into()], false))
                .unwrap();
        assert!(text.contains("receive-entry") && text.contains("send-entry"));
        d.smtp_shared = true;
        assert_eq!(d.active_credentials().len(), 1);
        let text = toml::to_string(&d.account(&["receive-entry".into()], false)).unwrap();
        assert!(!text.contains("sender@example.test"));
    }
    #[test]
    fn local_draft_needs_no_email_or_credential() {
        let mut d = draft();
        d.backend = "maildir".into();
        d.email.clear();
        d.root = "/tmp/disposable-mail".into();
        d.receive.credential.value.clear();
        d.validate().unwrap();
        assert!(d.active_credentials().is_empty());
        assert!(d.account(&[], false).maildir.is_some());
        d.root = "relative".into();
        assert!(d.validate().is_err());
    }
    #[test]
    fn jmap_rejects_url_userinfo_and_http_header_control_characters() {
        let mut d = draft();
        d.backend = "jmap".into();
        d.receive.mechanism = "header".into();
        for server in [
            "http://mail.example/jmap",
            "https://user:pass@mail.example/jmap",
            "https://mail.example/jmap?secret=bad",
            "https://mail.example/\njmap",
        ] {
            d.server = server.into();
            assert!(d.validate().is_err());
        }
        d.server = "https://mail.example/jmap".into();
        d.validate().unwrap();
        let mut account = d.account(&[], true);
        account.jmap.as_mut().unwrap().auth =
            JmapAuthConfig::Header(Secret::Raw("Bearer token\r\nInjected: yes".into()));
        assert!(!safe_http_auth(&account));
        account.jmap.as_mut().unwrap().auth =
            JmapAuthConfig::Header(Secret::Raw("Bearer token".into()));
        assert!(safe_http_auth(&account));
    }
    #[test]
    fn extra_worker_and_config_fields_are_not_accepted() {
        let mut value = serde_json::to_value(draft()).unwrap();
        value["config_path"] = json!("/tmp/unwanted");
        assert!(serde_json::from_value::<Draft>(value).is_err());
        assert!(
            serde_json::from_value::<Operation>(json!({"action":"shell","command":"true"}))
                .is_err()
        );
    }
}
