//! # Secret prompts
//!
//! The credential prompts the backend wizards share, delegating to
//! pimalaya-cli's OS-aware pickers.
//!
//! One offers the OS keyrings and the other the OAuth 2.0 token brokers,
//! both also taking a custom command or a raw value. A known provider or
//! broker yields an argv command, a custom one a shell string.
//!
//! Himalaya only reads a secret, never stores one, so a value that is not
//! there yet surfaces when the account is tested right after.

use crate::wizard::prompt;
use anyhow::{Result, bail};
use pimalaya_cli::wizard::keyring::{self, SecretChoice};
use pimalaya_cli::wizard::keyring::{KeyringProvider, TokenBroker};
use pimalaya_config::{command::CommandConfig, secret::Secret};

/// Prompts for a password through the shared keyring picker.
///
/// The default seeds the keyring entry, which is then used verbatim, so a
/// secret already there is read exactly as it is named.
pub fn configure_password(label: &str, key_default: &str) -> Result<Secret> {
    if prompt::is_web() {
        return form_secret(label, key_default, false);
    }
    to_secret(keyring::prompt_secret(label, key_default)?)
}

/// Prompts for an API token through the shared token picker.
///
/// The picker offers the OS keyrings, for a token generated on the
/// provider, and the OAuth 2.0 brokers, which refresh and print a fresh
/// one on every read.
///
/// The default seeds the keyring entry or the broker account handle.
pub fn configure_token(label: &str, key_default: &str, oauth: bool) -> Result<Secret> {
    if prompt::is_web() {
        return form_secret(label, key_default, oauth);
    }
    to_secret(keyring::prompt_token(label, key_default, oauth)?)
}

/// Reuses the framework's provider inventory and command construction.
fn form_secret(label: &str, key_default: &str, oauth: bool) -> Result<Secret> {
    let providers = KeyringProvider::available();
    let brokers = if oauth {
        TokenBroker::available()
    } else {
        vec![]
    };
    let describe = |name: &str, installed: bool| {
        if installed {
            name.to_string()
        } else {
            format!("{name}, not found on PATH")
        }
    };
    let mut choices: Vec<String> = providers
        .iter()
        .map(|p| describe(p.name(), p.installed()))
        .collect();
    choices.extend(brokers.iter().map(|b| describe(b.name(), b.installed())));
    choices.extend([
        "Custom shell command".into(),
        "Store raw in the configuration (plaintext, NOT recommended)".into(),
    ]);
    let selected = prompt::item(format!("{label} strategy:"), choices.clone(), None)?;
    let index = choices
        .iter()
        .position(|c| c == &selected)
        .expect("selected strategy");
    if let Some(provider) = providers.get(index) {
        let key = prompt::text(
            format!("{label} keyring entry:"),
            Some(key_default.to_owned()),
        )?;
        return command_secret(provider.read_command(None, &key));
    }
    if let Some(broker) = index
        .checked_sub(providers.len())
        .and_then(|i| brokers.get(i))
    {
        let account = prompt::text(format!("{label} account:"), Some(key_default.to_owned()))?;
        return command_secret(broker.read_command(&account));
    }
    if index == providers.len() + brokers.len() {
        if !prompt::bool(
            "This custom command will run locally with your user permissions during connection testing. Continue?",
            false,
        )? {
            bail!("Custom command cancelled");
        }
        let line = prompt::text(format!("{label} shell command:"), None::<String>)?;
        return shell_secret(&line);
    }
    if !prompt::bool(
        "The secret will be saved in plaintext in the configuration file. Keychain or a token broker is recommended. Continue?",
        false,
    )? {
        bail!("Plaintext storage cancelled");
    }
    Ok(Secret::Raw(prompt::password(
        format!("{label}:"),
        format!("Confirm {label}:"),
    )?))
}

/// Turns a picker choice into the [`Secret`] the configuration stores.
fn to_secret(choice: SecretChoice) -> Result<Secret> {
    Ok(match choice {
        SecretChoice::Command(argv) => command_secret(argv)?,
        SecretChoice::Shell(line) => shell_secret(&line)?,
        SecretChoice::Raw(secret) => Secret::Raw(secret),
    })
}

/// Builds a [`Secret::Command`] from an argv (program + arguments, no
/// shell), the form a known keyring provider or token broker yields. It
/// serializes back as a TOML array.
fn command_secret(argv: Vec<String>) -> Result<Secret> {
    let Some((program, args)) = argv.split_first() else {
        bail!("Empty command for secret");
    };

    Ok(Secret::Command(CommandConfig::Argv {
        program: program.clone(),
        args: args.to_vec(),
    }))
}

/// Builds a [`Secret::Command`] from a shell command line, the fallback
/// form a user typed by hand. It serializes back as a TOML string.
fn shell_secret(line: &str) -> Result<Secret> {
    let line = line.trim();
    if line.is_empty() {
        bail!("Empty shell command for secret");
    }

    Ok(Secret::Command(CommandConfig::Shell(line.to_owned())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_command_secret_is_rejected() {
        assert!(command_secret(Vec::new()).is_err());
    }

    #[test]
    fn blank_shell_secret_is_rejected() {
        assert!(shell_secret("   ").is_err());
    }
}
