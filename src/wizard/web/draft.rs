//! # Setup drafts
//!
//! Converts a restricted browser form into fixed, encrypted provider presets.

use anyhow::{Result, bail};
use pimalaya_config::{command::CommandConfig, secret::Secret};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::config::{AccountConfig, ImapConfig, SaslConfig, SaslPlainConfig, SmtpConfig};

/// A provider whose endpoints cannot be supplied by the browser.
#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// Gmail, including custom Google Workspace domains.
    Gmail,
    /// Apple iCloud Mail.
    Icloud,
}

/// The bounded account form submitted for explicit authentication testing.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Draft {
    /// The provider preset.
    pub provider: Provider,
    /// A new, non-conflicting account table name.
    pub name: String,
    /// The full mailbox address.
    pub email: String,
    /// Optional iCloud IMAP login, distinct from its SMTP address.
    #[serde(default)]
    pub imap_login: String,
    /// Either a transient password or an existing named Keychain item.
    pub credential: Credential,
}

/// Credentials accepted by the setup form, never returned to the browser.
#[derive(Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Credential {
    /// A new app password to save only after explicit confirmation.
    New {
        /// The transient password, cleared on drop.
        password: Zeroizing<String>,
    },
    /// A specific existing item; no Keychain listing is performed.
    Existing {
        /// The account/service name used for both Keychain attributes.
        entry: String,
    },
}

impl Draft {
    /// Validates identity and normalizes only app-password separators.
    pub fn validate(&mut self) -> Result<()> {
        if self.name.is_empty()
            || self.name.len() > 64
            || !self
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            bail!("Account name must use 1-64 ASCII letters, numbers, hyphens or underscores");
        }
        if self.email.len() > 254
            || !self.email.is_ascii()
            || self
                .email
                .bytes()
                .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
        {
            bail!("Enter a full ASCII email address without whitespace");
        }
        let Some((local, domain)) = self.email.split_once('@') else {
            bail!("Enter a full email address");
        };
        if local.is_empty()
            || domain.is_empty()
            || domain.contains('@')
            || !domain.contains('.')
            || !domain
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
            || !local
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".!#$%&'*+-/=?^_`{|}~".contains(&b))
        {
            bail!("Enter a valid email address");
        }
        match self.provider {
            Provider::Gmail => {
                if !self.imap_login.is_empty() && self.imap_login != self.email {
                    bail!("Gmail login must equal the full account address");
                }
                self.imap_login = self.email.clone();
            }
            Provider::Icloud => {
                if !["icloud.com", "me.com", "mac.com"]
                    .contains(&domain.to_ascii_lowercase().as_str())
                {
                    bail!(
                        "The initial iCloud preset requires an icloud.com, me.com or mac.com address"
                    );
                }
                if self.imap_login.is_empty() {
                    self.imap_login = local.to_string();
                }
                if self.imap_login.len() > 254
                    || !self.imap_login.is_ascii()
                    || self
                        .imap_login
                        .bytes()
                        .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
                {
                    bail!("Invalid iCloud IMAP login");
                }
            }
        }
        match &mut self.credential {
            Credential::New { password } => {
                let normalized: String = password
                    .chars()
                    .filter(|c| *c != ' ' && !matches!((self.provider, c), (Provider::Icloud, '-')))
                    .collect();
                *password = Zeroizing::new(normalized);
                if password.len() != 16 || !password.bytes().all(|b| b.is_ascii_alphanumeric()) {
                    bail!("Enter the 16-character app password, not your normal login password");
                }
            }
            Credential::Existing { entry } => {
                if entry.is_empty()
                    || entry.len() > 128
                    || !entry
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b"-_.@".contains(&b))
                {
                    bail!("Invalid Keychain item name");
                }
            }
        }
        Ok(())
    }

    /// Builds a provider account with only a safe Keychain reference.
    pub fn account(&self, entry: &str) -> AccountConfig {
        let password = Secret::Command(CommandConfig::Argv {
            program: "/usr/bin/security".into(),
            args: vec![
                "find-generic-password".into(),
                "-a".into(),
                entry.into(),
                "-s".into(),
                entry.into(),
                "-w".into(),
            ],
        });
        self.account_with_secret(password)
    }

    /// Builds a fixed endpoint configuration for an in-memory secret.
    pub fn account_with_secret(&self, password: Secret) -> AccountConfig {
        let (imap_server, smtp_server) = match self.provider {
            Provider::Gmail => ("imaps://imap.gmail.com:993", "smtp://smtp.gmail.com:587"),
            Provider::Icloud => (
                "imaps://imap.mail.me.com:993",
                "smtp://smtp.mail.me.com:587",
            ),
        };
        AccountConfig {
            email: Some(self.email.clone()),
            imap: Some(ImapConfig {
                server: imap_server.into(),
                tls: Default::default(),
                proxy: None,
                starttls: false,
                alpn: vec!["imap".into()],
                sasl: Some(SaslConfig::Plain(SaslPlainConfig {
                    authzid: None,
                    authcid: self.imap_login.clone(),
                    passwd: password.clone(),
                })),
                sasl_ir: None,
                id: Default::default(),
                sort: Default::default(),
            }),
            smtp: Some(SmtpConfig {
                server: smtp_server.into(),
                tls: Default::default(),
                proxy: None,
                starttls: true,
                alpn: vec!["smtp".into()],
                sasl: Some(SaslConfig::Plain(SaslPlainConfig {
                    authzid: None,
                    authcid: self.email.clone(),
                    passwd: password,
                })),
            }),
            ..Default::default()
        }
    }
}
