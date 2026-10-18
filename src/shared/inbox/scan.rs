//! Fixed UID snapshots for read-only, acknowledged inbox consumers.

use std::{collections::BTreeMap, fmt, path::PathBuf, time::Duration};

use anyhow::Result;
use base64::{Engine as _, prelude::BASE64_URL_SAFE_NO_PAD};
use chrono::{DateTime, TimeDelta, Utc};
use clap::Parser;
use io_imap::{
    client::ImapClient as _,
    rfc3501::{fetch::ImapMessageFetchOptions, search::ImapMessageSearchOptions},
    types::{
        core::Vec1, datetime::NaiveDate as ImapDate, fetch::MessageDataItem, search::SearchKey,
    },
};
use pimalaya_cli::printer::Printer;
use pimalaya_config::toml::TomlConfig;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{InboxEntry, parse_window};
use crate::{
    account::context::Account,
    backend::Backend,
    config::{Config, SaslConfig},
    imap::{
        backend::{build_item_names, envelope_from, parse_mailbox, uid_sequence_set},
        client::{ImapClient, parse_imap_server},
    },
    shared::mailbox::arg::MailboxArg,
};

/// Fetch one bounded batch; continuations retain their original receipt window.
#[derive(Debug, Parser)]
pub struct InboxScanCommand {
    #[command(flatten)]
    pub mailbox: MailboxArg,
    /// Receipt lookback (INTERNALDATE), not the sender's Date header.
    #[arg(long, value_parser = parse_window)]
    pub since: Option<TimeDelta>,
    #[arg(long, conflicts_with = "since")]
    pub all_time: bool,
    #[arg(long, default_value = "25", value_parser = clap::value_parser!(u32).range(1..=100))]
    pub page_size: u32,
    #[arg(long, default_value = "4", value_parser = clap::value_parser!(u32).range(1..=4))]
    pub max_pages: u32,
    #[arg(long, default_value = "131072", value_parser = clap::value_parser!(u32).range(8192..=131072))]
    pub max_bytes: u32,
    /// Continue or replay a fixed snapshot; opaque metadata, not a credential.
    #[arg(long, conflicts_with = "after")]
    pub cursor: Option<String>,
    /// Begin a new UID round after a fully acknowledged checkpoint.
    #[arg(long)]
    pub after: Option<String>,
    /// Fix the upper receipt timestamp for a new snapshot.
    #[arg(long)]
    pub as_of: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Cursor {
    version: u8,
    checkpoint: bool,
    scope: String,
    window_seconds: Option<i64>,
    since: Option<DateTime<Utc>>,
    as_of: DateTime<Utc>,
    uid_validity: u32,
    upper: u32,
    last: u32,
    #[serde(default)]
    deferred_from: Option<u32>,
}

#[derive(Debug, Serialize)]
struct SafeError {
    code: &'static str,
}

#[derive(Debug, Serialize)]
struct ScanOutput {
    version: u8,
    account: String,
    mailbox: String,
    since: Option<DateTime<Utc>>,
    as_of: DateTime<Utc>,
    complete: bool,
    scanned: usize,
    matched: usize,
    uid_validity: u32,
    snapshot_uid: u32,
    deferred: bool,
    cursor: String,
    next_cursor: Option<String>,
    checkpoint: Option<String>,
    reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<SafeError>,
    entries: Vec<InboxEntry>,
}

impl fmt::Display for ScanOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "complete={} deferred={} scanned={} matched={}",
            self.complete, self.deferred, self.scanned, self.matched
        )?;
        if let Some(reason) = self.reason {
            write!(f, " reason={reason}")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct ScanError(&'static str);
type ScanResult<T> = std::result::Result<T, ScanError>;

impl Cursor {
    fn encode(&self) -> String {
        BASE64_URL_SAFE_NO_PAD.encode(serde_json::to_vec(self).expect("cursor metadata serializes"))
    }

    fn decode(raw: &str, checkpoint: bool) -> ScanResult<Self> {
        if raw.len() > 8192 {
            return Err(ScanError("INVALID_CURSOR"));
        }
        let bytes = BASE64_URL_SAFE_NO_PAD
            .decode(raw)
            .map_err(|_| ScanError("INVALID_CURSOR"))?;
        let value: Self =
            serde_json::from_slice(&bytes).map_err(|_| ScanError("INVALID_CURSOR"))?;
        if value.version != 1
            || value.checkpoint != checkpoint
            || value.scope.len() != 64
            || value.uid_validity == 0
            || value.last > value.upper
            || (checkpoint && value.last != value.upper)
            || (checkpoint && value.deferred_from.is_some())
            || value
                .deferred_from
                .is_some_and(|u| u == 0 || u > value.last)
            || value
                .window_seconds
                .is_some_and(|n| n <= 0 || TimeDelta::try_seconds(n).is_none())
            || value.since.is_some_and(|since| since > value.as_of)
            || value.window_seconds.is_some() != value.since.is_some()
            || value.as_of > Utc::now() + TimeDelta::seconds(30)
        {
            return Err(ScanError("INVALID_CURSOR"));
        }
        Ok(value)
    }
}

impl InboxScanCommand {
    pub fn execute(
        self,
        printer: &mut impl Printer,
        paths: &[PathBuf],
        selected: Option<&str>,
        backend: Backend,
    ) -> Result<()> {
        let fallback = selected.filter(|s| s.len() <= 512).unwrap_or("").to_owned();
        let max_bytes = self.max_bytes as usize;
        let mut output = match self.scan(paths, selected, backend) {
            Ok(output) => output,
            Err(ScanError(code)) => ScanOutput {
                version: 1,
                account: fallback,
                mailbox: String::new(),
                since: None,
                as_of: Utc::now(),
                complete: false,
                scanned: 0,
                matched: 0,
                uid_validity: 0,
                snapshot_uid: 0,
                deferred: false,
                cursor: String::new(),
                next_cursor: None,
                checkpoint: None,
                reason: Some(code),
                error: Some(SafeError { code }),
                entries: vec![],
            },
        };
        if serde_json::to_vec_pretty(&output)?.len() + 1 > max_bytes {
            output.entries.clear();
            output.account.clear();
            output.mailbox.clear();
            output.cursor.clear();
            output.next_cursor = None;
            output.checkpoint = None;
            output.complete = false;
            output.scanned = 0;
            output.matched = 0;
            output.error = Some(SafeError {
                code: "SCAN_FAILED",
            });
            output.reason = Some("OUTPUT_BUDGET");
        }
        printer.out(output)
    }

    fn scan(
        self,
        paths: &[PathBuf],
        selected: Option<&str>,
        backend: Backend,
    ) -> ScanResult<ScanOutput> {
        if !backend.allows_imap() {
            return Err(ScanError("UNSUPPORTED_BACKEND"));
        }
        let selected = selected
            .filter(|s| !s.is_empty())
            .ok_or(ScanError("ACCOUNT_REQUIRED"))?;
        let mut config = Config::from_paths_or_default(paths)
            .map_err(|_| ScanError("CONFIG_ERROR"))?
            .ok_or(ScanError("CONFIG_ERROR"))?;
        if selected == "default" && config.accounts.values().filter(|a| a.default).count() != 1 {
            return Err(ScanError("ACCOUNT_REQUIRED"));
        }
        let (name, mut account_config) = if selected == "default" {
            config.take_default_account()
        } else {
            config.take_named_account(selected)
        }
        .ok_or(ScanError("CONFIG_ERROR"))?;
        if name.len() > 512 {
            return Err(ScanError("CONFIG_ERROR"));
        }
        let configured_window = config.inbox.since.as_deref().map(parse_window).transpose();
        if backend == Backend::Auto
            && [
                (Backend::Maildir, account_config.maildir.is_some()),
                (Backend::M2dir, account_config.m2dir.is_some()),
                (Backend::Mbox, account_config.mbox.is_some()),
                (Backend::Pimdir, account_config.pimdir.is_some()),
                (Backend::Jmap, account_config.jmap.is_some()),
                (Backend::Gmail, account_config.gmail.is_some()),
                (Backend::Msgraph, account_config.msgraph.is_some()),
            ]
            .iter()
            .any(|(backend, configured)| *configured && Backend::COMPILED.contains(backend))
        {
            return Err(ScanError("UNSUPPORTED_BACKEND"));
        }
        let imap = account_config
            .imap
            .take()
            .ok_or(ScanError("UNSUPPORTED_BACKEND"))?;
        config.accounts.clear();
        let account = Account::from(config).merge(Account::from(account_config));
        let mailbox = self.mailbox.resolve(&account);
        if mailbox.len() > 1024 {
            return Err(ScanError("CONFIG_ERROR"));
        }
        // Bind only endpoint and non-secret authentication identities, never a secret resolver.
        let mut endpoint =
            parse_imap_server(&imap.server).map_err(|_| ScanError("CONFIG_ERROR"))?;
        let _ = endpoint.set_password(None);
        let _ = endpoint.set_username("");
        let identity: (&str, &str, &str) = match &imap.sasl {
            Some(SaslConfig::Plain(c)) => ("plain", &c.authcid, c.authzid.as_deref().unwrap_or("")),
            Some(SaslConfig::Login(c)) => ("login", &c.username, ""),
            Some(SaslConfig::Oauthbearer(c)) => ("oauthbearer", &c.username, ""),
            Some(SaslConfig::Xoauth2(c)) => ("xoauth2", &c.username, ""),
            Some(SaslConfig::ScramSha256(c)) => ("scram", &c.username, ""),
            _ => ("anonymous", "", ""),
        };
        let digest = Sha256::digest(
            serde_json::to_vec(&(&name, &mailbox, endpoint.as_str(), identity))
                .map_err(|_| ScanError("CONFIG_ERROR"))?,
        );
        let scope: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        let prior = self
            .cursor
            .as_deref()
            .map(|c| Cursor::decode(c, false))
            .or_else(|| self.after.as_deref().map(|c| Cursor::decode(c, true)))
            .transpose()?;
        if let Some(prior) = &prior {
            if prior.scope != scope {
                return Err(ScanError("CURSOR_SCOPE_CHANGED"));
            }
            if (self.all_time && prior.window_seconds.is_some())
                || self
                    .since
                    .is_some_and(|w| Some(w.num_seconds()) != prior.window_seconds)
                || (self.cursor.is_some() && self.as_of.is_some_and(|t| t != prior.as_of))
            {
                return Err(ScanError("INVALID_CURSOR"));
            }
        }
        let window_seconds = match &prior {
            Some(prior) => prior.window_seconds,
            None if self.all_time => None,
            None => match self.since {
                Some(window) => Some(window.num_seconds()),
                None => configured_window
                    .map_err(|_| ScanError("CONFIG_ERROR"))?
                    .map(|w| w.num_seconds()),
            },
        };
        let as_of = if self.cursor.is_some() {
            prior.as_ref().unwrap().as_of
        } else {
            self.as_of.unwrap_or_else(Utc::now)
        };
        if as_of > Utc::now() + TimeDelta::seconds(30)
            || prior
                .as_ref()
                .is_some_and(|p| self.after.is_some() && as_of < p.as_of)
        {
            return Err(ScanError("INVALID_CURSOR"));
        }
        let mut client = ImapClient::new(imap).map_err(|_| ScanError("SCAN_FAILED"))?;
        client
            .stream
            .set_read_timeout(Some(Duration::from_secs(8)))
            .map_err(|_| ScanError("SCAN_FAILED"))?;
        client.stream.stop_retrying();
        let selected = client
            .examine(
                parse_mailbox(&mailbox).map_err(|_| ScanError("CONFIG_ERROR"))?,
                Default::default(),
            )
            .map_err(|_| ScanError("SCAN_FAILED"))?;
        let uid_validity = selected
            .uid_validity
            .ok_or(ScanError("MISSING_UID_METADATA"))?
            .get();
        let high = selected
            .uid_next
            .ok_or(ScanError("MISSING_UID_METADATA"))?
            .get()
            - 1;
        if prior
            .as_ref()
            .is_some_and(|p| p.uid_validity != uid_validity)
        {
            return Err(ScanError("UIDVALIDITY_CHANGED"));
        }
        if prior.as_ref().is_some_and(|p| p.upper > high) {
            return Err(ScanError("UID_STATE_REGRESSED"));
        }
        // UIDNEXT precedes the default time boundary, so mail arriving during authentication is not skipped.
        let as_of = if self.cursor.is_none() && self.as_of.is_none() {
            Utc::now()
        } else {
            as_of
        };
        let since = if self.cursor.is_some() {
            prior.as_ref().unwrap().since
        } else {
            window_seconds
                .map(|s| {
                    as_of
                        .checked_sub_signed(TimeDelta::seconds(s))
                        .ok_or(ScanError("INVALID_WINDOW"))
                })
                .transpose()?
                .map(|s| prior.as_ref().map_or(s, |p| s.min(p.as_of)))
        };
        let mut position = Cursor {
            version: 1,
            checkpoint: false,
            scope,
            window_seconds,
            since,
            as_of,
            uid_validity,
            upper: if self.cursor.is_some() {
                prior.as_ref().unwrap().upper
            } else {
                high
            },
            last: prior.as_ref().map_or(0, |p| p.last),
            deferred_from: if self.cursor.is_some() {
                prior.as_ref().unwrap().deferred_from
            } else {
                None
            },
        };
        let mut output = ScanOutput {
            version: 1,
            account: name.clone(),
            mailbox: mailbox.clone(),
            since,
            as_of,
            complete: false,
            scanned: 0,
            matched: 0,
            uid_validity,
            snapshot_uid: position.upper,
            deferred: position.deferred_from.is_some(),
            cursor: position.encode(),
            next_cursor: None,
            checkpoint: None,
            reason: None,
            error: None,
            entries: vec![],
        };
        let candidates = if position.last >= position.upper {
            vec![]
        } else {
            let range = format!("{}:{}", position.last + 1, position.upper)
                .parse()
                .map_err(|_| ScanError("INVALID_CURSOR"))?;
            let mut keys = vec![SearchKey::Uid(range)];
            // IMAP date searches discard time/zone. Widen by a day, then filter exact INTERNALDATE.
            if let Some(since) = since {
                let date = since
                    .date_naive()
                    .pred_opt()
                    .ok_or(ScanError("INVALID_WINDOW"))?;
                keys.push(SearchKey::Since(
                    ImapDate::try_from(date).map_err(|_| ScanError("INVALID_WINDOW"))?,
                ));
            }
            // Future INTERNALDATE UIDs must be observed, so no upper date predicate may hide them.
            let mut uids: Vec<u32> = client
                .search(
                    Vec1::try_from(keys).map_err(|_| ScanError("SCAN_FAILED"))?,
                    ImapMessageSearchOptions { uid: true },
                )
                .map_err(|_| ScanError("SCAN_FAILED"))?
                .into_iter()
                .map(|u| u.get())
                .collect();
            if uids
                .iter()
                .any(|u| *u <= position.last || *u > position.upper)
            {
                return Err(ScanError("INVALID_UID_RESPONSE"));
            }
            uids.sort_unstable();
            uids.dedup();
            uids
        };
        let limit = (self.page_size * self.max_pages) as usize;
        'pages: for chunk in candidates
            .iter()
            .take(limit)
            .copied()
            .collect::<Vec<_>>()
            .chunks(self.page_size as usize)
        {
            let rows = client.fetch(
                uid_sequence_set(chunk.to_vec()).map_err(|_| ScanError("SCAN_FAILED"))?,
                build_item_names(false),
                ImapMessageFetchOptions {
                    uid: true,
                    ..Default::default()
                },
            );
            let Ok(rows) = rows else {
                output.reason = Some("SCAN_FAILED");
                break;
            };
            let mut envelopes = BTreeMap::new();
            for (seq, items) in rows {
                let items: Vec<_> = items.into_iter().collect();
                let uid = items.iter().find_map(|i| {
                    if let MessageDataItem::Uid(uid) = i {
                        Some(uid.get())
                    } else {
                        None
                    }
                });
                let Some(uid) = uid.filter(|u| chunk.contains(u)) else {
                    output.reason = Some("INVALID_UID_RESPONSE");
                    break 'pages;
                };
                if envelopes
                    .insert(uid, envelope_from(seq.get(), items))
                    .is_some()
                {
                    output.reason = Some("INVALID_UID_RESPONSE");
                    break 'pages;
                }
            }
            for uid in chunk {
                let Some(envelope) = envelopes.remove(uid) else {
                    output.reason = Some("MISSING_FETCH_ROW");
                    break 'pages;
                };
                let Some(received) = envelope.received_at else {
                    output.reason = Some("MISSING_RECEIVED_AT");
                    break 'pages;
                };
                if received > as_of {
                    position.deferred_from =
                        Some(position.deferred_from.map_or(*uid, |u| u.min(*uid)));
                }
                let matches = received <= as_of && since.is_none_or(|s| received >= s);
                if matches {
                    output.entries.push(InboxEntry {
                        account: name.clone(),
                        mailbox: mailbox.clone(),
                        received_at: Some(received),
                        envelope,
                    });
                    // Reserve the bounded final continuation/checkpoint and status fields.
                    if serde_json::to_vec_pretty(&output)
                        .map_err(|_| ScanError("SCAN_FAILED"))?
                        .len()
                        + 2048
                        > self.max_bytes as usize
                    {
                        output.entries.pop();
                        output.reason = Some(if output.entries.is_empty() {
                            "ENTRY_TOO_LARGE"
                        } else {
                            "OUTPUT_BUDGET"
                        });
                        break 'pages;
                    }
                }
                position.last = *uid;
                output.scanned += 1;
            }
        }
        output.matched = output.entries.len();
        output.deferred = position.deferred_from.is_some();
        output.complete = output.reason.is_none() && output.scanned == candidates.len();
        if output.complete {
            // Revisit a future-dated UID on the next round instead of acknowledging past it.
            position.upper = position.deferred_from.map_or(position.upper, |u| u - 1);
            position.last = position.upper;
            position.deferred_from = None;
            position.checkpoint = true;
            output.checkpoint = Some(position.encode());
        } else {
            output.reason.get_or_insert("PAGE_LIMIT");
            output.next_cursor = Some(position.encode());
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cursor() -> Cursor {
        Cursor {
            version: 1,
            checkpoint: false,
            scope: "a".repeat(64),
            window_seconds: Some(1800),
            since: Some(Utc::now() - TimeDelta::minutes(30)),
            as_of: Utc::now(),
            uid_validity: 1,
            upper: 10,
            last: 2,
            deferred_from: None,
        }
    }

    #[test]
    fn cursor_roundtrip_and_validation() {
        let mut c = cursor();
        assert_eq!(Cursor::decode(&c.encode(), false).unwrap().last, 2);
        assert!(Cursor::decode(&c.encode(), true).is_err());
        c.last = 11;
        assert!(Cursor::decode(&c.encode(), false).is_err());
        assert!(Cursor::decode(&"x".repeat(8193), false).is_err());
        assert!(Cursor::decode("not-base64!", false).is_err());
    }

    #[test]
    fn checkpoint_requires_complete_snapshot() {
        let mut c = cursor();
        c.checkpoint = true;
        assert!(Cursor::decode(&c.encode(), true).is_err());
        c.last = c.upper;
        assert!(Cursor::decode(&c.encode(), true).is_ok());
        c.since = Some(c.as_of + TimeDelta::seconds(1));
        assert!(Cursor::decode(&c.encode(), true).is_err());
    }
}
