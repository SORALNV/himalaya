//! Shared wizard questions with terminal and structured Web adapters.

use std::fmt;

use crate::config::{
    AccountConfig, ImapConfig, JmapAuthConfig, JmapConfig, ProxyConfig, SaslConfig, SmtpConfig,
};
use anyhow::{Result, bail};
use pimalaya_config::secret::Secret;
use secrecy::{ExposeSecret, SecretString};

/// Whether this process is the private structured-form worker.
pub fn is_web() -> bool {
    #[cfg(all(target_os = "macos", feature = "imap", feature = "smtp"))]
    return crate::wizard::web::form::enabled();
    #[cfg(not(all(target_os = "macos", feature = "imap", feature = "smtp")))]
    false
}

/// Resolves credential commands with bounded output only for Web checks.
pub fn for_check<T: CheckConfig>(value: &T) -> Result<T> {
    let mut prepared = value.clone();
    if is_web() {
        prepared.prepare()?;
    }
    Ok(prepared)
}

/// A typed check clone preserving literal paths, usernames and save sources.
pub trait CheckConfig: Clone {
    /// Resolves command secrets only in the authentication-check clone.
    fn prepare(&mut self) -> Result<()>;
}

fn prepare_secret(secret: &mut Secret) -> Result<()> {
    #[cfg(all(target_os = "macos", feature = "imap", feature = "smtp"))]
    {
        crate::wizard::web::form::resolve_secret(secret)?;
    }
    #[cfg(not(all(target_os = "macos", feature = "imap", feature = "smtp")))]
    {
        let _ = secret;
    }
    Ok(())
}

fn prepare_sasl(sasl: &mut Option<SaslConfig>) -> Result<()> {
    match sasl {
        Some(SaslConfig::Login(c)) => prepare_secret(&mut c.password),
        Some(SaslConfig::Plain(c)) => prepare_secret(&mut c.passwd),
        Some(SaslConfig::Oauthbearer(c)) => prepare_secret(&mut c.token),
        Some(SaslConfig::Xoauth2(c)) => prepare_secret(&mut c.token),
        Some(SaslConfig::ScramSha256(c)) => prepare_secret(&mut c.password),
        _ => Ok(()),
    }
}

fn prepare_proxy(proxy: &mut Option<ProxyConfig>) -> Result<()> {
    if let Some(proxy) = proxy
        && let Some(password) = &mut proxy.password
    {
        prepare_secret(password)?;
    }
    Ok(())
}

impl CheckConfig for ImapConfig {
    fn prepare(&mut self) -> Result<()> {
        prepare_sasl(&mut self.sasl)?;
        prepare_proxy(&mut self.proxy)
    }
}
impl CheckConfig for SmtpConfig {
    fn prepare(&mut self) -> Result<()> {
        prepare_sasl(&mut self.sasl)?;
        prepare_proxy(&mut self.proxy)
    }
}
impl CheckConfig for JmapConfig {
    fn prepare(&mut self) -> Result<()> {
        match &mut self.auth {
            JmapAuthConfig::Header(value) => prepare_secret(value)?,
            JmapAuthConfig::Bearer { token } => prepare_secret(token)?,
            JmapAuthConfig::Basic { password, .. } => prepare_secret(password)?,
        }
        prepare_proxy(&mut self.proxy)
    }
}
impl CheckConfig for AccountConfig {
    fn prepare(&mut self) -> Result<()> {
        if let Some(config) = &mut self.imap {
            config.prepare()?;
        }
        if let Some(config) = &mut self.smtp {
            config.prepare()?;
        }
        if let Some(config) = &mut self.jmap {
            config.prepare()?;
        }
        if let Some(config) = &mut self.gmail {
            prepare_secret(&mut config.auth.token)?;
            prepare_proxy(&mut config.proxy)?;
        }
        if let Some(config) = &mut self.msgraph {
            prepare_secret(&mut config.auth.token)?;
            prepare_proxy(&mut config.proxy)?;
        }
        prepare_proxy(&mut self.proxy)
    }
}

/// Asks a typed question through the private worker channel.
fn ask(kind: &str, label: &str, default: Option<String>, choices: Vec<String>) -> Result<String> {
    #[cfg(all(target_os = "macos", feature = "imap", feature = "smtp"))]
    return crate::wizard::web::form::ask(kind, label, default, choices);
    #[cfg(not(all(target_os = "macos", feature = "imap", feature = "smtp")))]
    {
        let _ = (kind, label, default, choices);
        bail!("Web form transport is not available in this build")
    }
}

/// Prompts for text with the same default in either interface.
pub fn text<T: AsRef<str>>(label: T, default: Option<T>) -> Result<String> {
    if is_web() {
        return ask(
            "text",
            label.as_ref(),
            default.map(|s| s.as_ref().into()),
            vec![],
        );
    }
    Ok(pimalaya_cli::prompt::text(label, default)?)
}

/// Prompts for a yes/no confirmation.
pub fn bool(label: impl AsRef<str>, default: bool) -> Result<bool> {
    if is_web() {
        return Ok(ask("confirm", label.as_ref(), Some(default.to_string()), vec![])? == "true");
    }
    Ok(pimalaya_cli::prompt::bool(label, default)?)
}

/// Prompts for the original typed selection, exposing only display labels.
pub fn item<T: fmt::Display + Eq>(
    label: impl AsRef<str>,
    items: impl IntoIterator<Item = T>,
    default: Option<T>,
) -> Result<T> {
    let items: Vec<T> = items.into_iter().collect();
    if is_web() {
        let selected = default
            .as_ref()
            .and_then(|value| items.iter().position(|i| i == value));
        let answer = ask(
            "select",
            label.as_ref(),
            selected.map(|i| i.to_string()),
            items.iter().map(ToString::to_string).collect(),
        )?;
        let index: usize = answer.parse()?;
        return items
            .into_iter()
            .nth(index)
            .ok_or_else(|| anyhow::anyhow!("Invalid choice"));
    }
    Ok(pimalaya_cli::prompt::item(label, items, default)?)
}

/// Prompts for a masked value and its confirmation without reflecting it.
pub fn password(label: impl AsRef<str>, confirm: impl AsRef<str>) -> Result<SecretString> {
    if is_web() {
        let first: SecretString = ask("password", label.as_ref(), None, vec![])?.into();
        let second: SecretString = ask("password", confirm.as_ref(), None, vec![])?.into();
        if first.expose_secret() != second.expose_secret() {
            bail!("Password confirmation did not match");
        }
        return Ok(first);
    }
    Ok(pimalaya_cli::prompt::password(label, confirm)?)
}
