//! Fork-only multi-account inbox and receipt-time filtering.

use std::{collections::HashSet, fmt, path::PathBuf};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, FixedOffset, TimeDelta, Utc};
use clap::{Parser, Subcommand};
use pimalaya_cli::{
    printer::Printer,
    table::{Cell, Table, sanitize},
};
use pimalaya_config::toml::TomlConfig;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    backend::Backend,
    config::Config,
    email::envelope::Envelope,
    shared::{client::EmailClient, envelope::list::format_addresses, mailbox::arg::MailboxArg},
};

/// Fork-only inbox commands.
#[derive(Debug, Subcommand)]
pub enum InboxCommand {
    /// Merge configured inboxes; global `-a NAME` selects one account.
    List(InboxListCommand),
}

/// List summaries without fetching bodies or changing Seen flags.
#[derive(Debug, Parser)]
pub struct InboxListCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    /// Receipt window, such as 30m, 2h or 1d; overrides inbox.since.
    #[arg(long, value_parser = parse_window)]
    pub since: Option<TimeDelta>,
    /// Disable the configured receipt window.
    #[arg(long, conflicts_with = "since")]
    pub all_time: bool,
    /// Maximum pages scanned per account; reaching this is reported.
    #[arg(long, default_value = "100", value_parser = clap::value_parser!(u32).range(1..=10000))]
    pub max_pages: u32,
    /// Summaries fetched per page (not an output limit).
    #[arg(long, default_value = "100", value_parser = clap::value_parser!(u32).range(1..=500))]
    pub page_size: u32,
}

fn parse_window(raw: &str) -> Result<TimeDelta, String> {
    if !raw.is_ascii() {
        return Err("Use an ASCII duration such as 30m".into());
    }
    let (digits, unit) = raw.split_at(raw.len().saturating_sub(1));
    let multiplier = match unit {
        "s" => 1,
        "m" => 60,
        "h" => 3600,
        "d" => 86400,
        _ => return Err("Use a positive duration such as 30m, 2h or 1d".into()),
    };
    let seconds = digits
        .parse::<i64>()
        .ok()
        .filter(|v| *v > 0)
        .and_then(|v| v.checked_mul(multiplier));
    seconds
        .and_then(TimeDelta::try_seconds)
        .ok_or_else(|| "Duration must be positive and within range".into())
}

fn within_window(
    received: DateTime<FixedOffset>,
    since: DateTime<Utc>,
    as_of: DateTime<Utc>,
) -> bool {
    received >= since && received <= as_of
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct InboxEntry {
    pub account: String,
    pub mailbox: String,
    pub received_at: Option<DateTime<FixedOffset>>,
    pub envelope: Envelope,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct AccountCoverage {
    pub account: String,
    pub scanned: usize,
    pub exhausted: bool,
    pub missing_received_at: usize,
    pub repeated_ids: usize,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct InboxOutput {
    pub as_of: DateTime<Utc>,
    pub since: Option<DateTime<Utc>>,
    /// False when any account failed, lacked receipt metadata or hit a bound.
    pub complete: bool,
    pub accounts: Vec<AccountCoverage>,
    pub entries: Vec<InboxEntry>,
}

impl InboxCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        paths: &[PathBuf],
        selected: Option<&str>,
        backend: Backend,
    ) -> Result<()> {
        let Self::List(cmd) = self;
        let mut config = Config::from_paths_or_default(paths)?.context("No configuration found")?;
        let mut names: Vec<String> = config.accounts.keys().cloned().collect();
        names.sort();
        if let Some(name) = selected {
            if name.is_empty() || name == "default" {
                names.retain(|name| config.accounts[name].default);
                if names.len() != 1 {
                    bail!("Select an explicit account with -a NAME");
                }
            } else {
                if !config.accounts.contains_key(name) {
                    bail!("Unknown account: {}", sanitize(name));
                }
                names = vec![name.to_owned()];
            }
        }
        if names.is_empty() {
            bail!("No configured accounts");
        }
        let window = if cmd.all_time {
            None
        } else {
            match cmd.since {
                Some(window) => Some(window),
                None => config
                    .inbox
                    .since
                    .as_deref()
                    .map(parse_window)
                    .transpose()
                    .map_err(anyhow::Error::msg)?,
            }
        };
        let as_of = Utc::now();
        let since = window
            .map(|window| {
                as_of
                    .checked_sub_signed(window)
                    .context("Receipt window exceeds the supported calendar range")
            })
            .transpose()?;
        // NOTE: retain a single parsed configuration, move selected credentials
        // into their clients and clone only the credential-free global defaults.
        let accounts: Vec<_> = names
            .iter()
            .map(|name| {
                config
                    .take_named_account(name)
                    .context("Account disappeared")
            })
            .collect::<Result<_>>()?;
        config.accounts.clear();
        let mut output = InboxOutput {
            as_of,
            since,
            complete: true,
            accounts: vec![],
            entries: vec![],
        };
        for (name, account_config) in accounts {
            let mut coverage = AccountCoverage {
                account: name.clone(),
                scanned: 0,
                exhausted: false,
                missing_received_at: 0,
                repeated_ids: 0,
                error: None,
            };
            let result = (|| -> Result<()> {
                let (account, mut client) =
                    EmailClient::new(config.clone(), &name, account_config, backend)?;
                let mailbox = client.resolve_mailbox_id(&cmd.mailbox.resolve(&account))?;
                let mut ids = HashSet::new();
                for page in 1..=cmd.max_pages {
                    let envelopes =
                        client.list_envelopes(&mailbox, Some(page), Some(cmd.page_size), false)?;
                    let count = envelopes.len();
                    coverage.scanned += count;
                    for envelope in envelopes {
                        // NOTE: deduplicate overlapping pages only within this account.
                        if !ids.insert(envelope.id.clone()) {
                            coverage.repeated_ids += 1;
                            continue;
                        }
                        if let Some(cutoff) = since {
                            let Some(received) = envelope.received_at else {
                                coverage.missing_received_at += 1;
                                continue;
                            };
                            if !within_window(received, cutoff, as_of) {
                                continue;
                            }
                        }
                        output.entries.push(InboxEntry {
                            account: name.clone(),
                            mailbox: mailbox.clone(),
                            received_at: envelope.received_at,
                            envelope,
                        });
                    }
                    if count == 0 {
                        coverage.exhausted = true;
                        break;
                    }
                }
                Ok(())
            })();
            if let Err(error) = result {
                coverage.error = Some(error.to_string());
            }
            output.complete &= coverage.exhausted
                && coverage.error.is_none()
                && coverage.missing_received_at == 0
                && coverage.repeated_ids == 0;
            output.accounts.push(coverage);
        }
        output.entries.sort_by(|a, b| {
            b.received_at
                .or(b.envelope.date)
                .cmp(&a.received_at.or(a.envelope.date))
                .then(a.account.cmp(&b.account))
                .then(a.mailbox.cmp(&b.mailbox))
                .then(a.envelope.id.cmp(&b.envelope.id))
        });
        printer.out(output)
    }
}

impl fmt::Display for InboxOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for account in &self.accounts {
            if account.repeated_ids > 0 {
                writeln!(
                    f,
                    "WARNING [{}]: overlapping pages; mailbox may have changed, retry the scan",
                    sanitize(&account.account)
                )?;
            }
            if let Some(error) = &account.error {
                writeln!(
                    f,
                    "WARNING [{}]: {}",
                    sanitize(&account.account),
                    sanitize(error)
                )?;
            }
            if !account.exhausted && account.error.is_none() {
                writeln!(
                    f,
                    "WARNING [{}]: incomplete scan; increase --max-pages",
                    sanitize(&account.account)
                )?;
            }
            if account.missing_received_at > 0 {
                writeln!(
                    f,
                    "WARNING [{}]: {} summaries lack receipt metadata and were excluded",
                    sanitize(&account.account),
                    account.missing_received_at
                )?;
            }
        }
        let mut table = Table::new();
        table.set_header([
            "ACCOUNT",
            "MAILBOX",
            "ID",
            "SUBJECT",
            "FROM",
            "TIME",
            "TIME BASIS",
        ]);
        for entry in &self.entries {
            let env = &entry.envelope;
            let time = entry
                .received_at
                .or(env.date)
                .map(|d| d.to_rfc3339())
                .unwrap_or_default();
            table.add_row([
                Cell::new(sanitize(&entry.account)),
                Cell::new(sanitize(&entry.mailbox)),
                Cell::new(sanitize(&env.id)),
                Cell::new(sanitize(&env.subject)),
                Cell::new(sanitize(&format_addresses(&env.from))),
                Cell::new(time),
                Cell::new(if entry.received_at.is_some() {
                    "receipt"
                } else {
                    "sender / unknown"
                }),
            ]);
        }
        writeln!(f, "{table}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn durations_are_positive_checked_and_explicit() {
        for (raw, seconds) in [("30m", 1800), ("2h", 7200), ("1d", 86400), ("1s", 1)] {
            assert_eq!(parse_window(raw).unwrap().num_seconds(), seconds);
        }
        for raw in ["", "0m", "-1h", "30", "1w", "999999999999999999d", "1.5h"] {
            assert!(parse_window(raw).is_err(), "{raw}");
        }
    }

    #[test]
    fn unicode_durations_never_panic() {
        for raw in ["１分", "30分", "é", "💌"] {
            assert!(parse_window(raw).is_err());
        }
    }

    #[test]
    fn receipt_boundaries_are_inclusive_and_timezone_independent() {
        let as_of = DateTime::parse_from_rfc3339("2026-10-11T12:00:00+09:00")
            .unwrap()
            .with_timezone(&Utc);
        let since = as_of - TimeDelta::minutes(30);
        assert!(within_window(since.fixed_offset(), since, as_of));
        assert!(within_window(as_of.fixed_offset(), since, as_of));
        assert!(within_window(
            DateTime::parse_from_rfc3339("2026-10-10T20:45:00-06:00").unwrap(),
            since,
            as_of
        ));
        assert!(!within_window(
            (since - TimeDelta::nanoseconds(1)).fixed_offset(),
            since,
            as_of
        ));
        assert!(!within_window(
            (as_of + TimeDelta::nanoseconds(1)).fixed_offset(),
            since,
            as_of
        ));
    }
}
