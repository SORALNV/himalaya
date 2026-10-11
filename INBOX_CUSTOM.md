# Fork-only unified inbox

This extension lives only in SORALNV/himalaya. It is not part of the upstream security PRs.

Use the existing configuration file, with as many `[accounts.NAME]` blocks as needed. No default account is required for a merged listing.

For this local checkout and the existing private test configuration, define the helper in your terminal:

```sh
himalaya_test() {
  /Users/sora/Code/himalaya/target/debug/himalaya \
    -c /Users/sora/.config/himalaya-security-test/config.toml "$@"
}
```

Then replace `himalaya` with `himalaya_test` in the examples below. The helper does not contain a password and does not change the configuration.

```sh
# All accounts, all available inbox summaries
himalaya inbox list

# All accounts, received in the last 30 minutes
himalaya inbox list --since 30m

# One account, received in the last two hours
himalaya -a work inbox list --since 2h

# Machine-readable entries and per-account coverage
himalaya --json inbox list --since 1d
```

Accepted duration units are seconds (`s`), minutes (`m`), hours (`h`) and days (`d`). Durations must be positive integers. A day means 24 hours, not a calendar day. The window includes both its cutoff and the command-start time; future timestamps are excluded. Seen and unseen messages both qualify: this is a received-time window, not an unread-only filter or a checkpoint since the previous run.

Set a persistent default in the global configuration, outside any account block:

```toml
[inbox]
since = "30m"
```

`--since` overrides this value. `--all-time` disables it. `-m NAME` selects another mailbox in every selected account; otherwise each account's inbox alias applies. Each result includes its account, resolved mailbox and original message ID. To read a result, pass the same account, mailbox and ID to `message read`; never treat a bare ID as globally unique.

Recent filtering uses IMAP INTERNALDATE, JMAP receivedAt, Gmail internalDate or Microsoft Graph receivedDateTime. It never substitutes the sender's Date header. These are backend-assigned receipt metadata, not cryptographic proof of delivery; imported mail may carry an assigned historical receipt timestamp. Local stores do not expose this metadata here. Their unfiltered summaries work, but recent filtering excludes rows without receipt metadata and reports the exclusion. In unfiltered output, missing receipt timestamps use the existing envelope date for sorting and the table labels that basis explicitly.

The command retrieves summaries only. IMAP listing opens the mailbox read-only using EXAMINE and fetches no body. It does not send mail, delete messages or change Seen flags.

## Bounds and partial results

Each account is scanned sequentially, up to 100 pages of 100 summaries by default. Change these using `--max-pages N` (1 through 10000) and `--page-size N` (1 through 500, compatible with Gmail's request limit). Scanning does not stop at the first old timestamp: message sequence order need not be receipt order. A short page is not assumed to be the end; an empty page establishes exhaustion. Large mailboxes can require more time and network traffic, especially REST backends that replay page tokens.

Plain output warns about account failures, scan bounds, missing receipt timestamps and overlapping pages. JSON returns one object containing `entries`, per-account `accounts`, `as_of`, `since` and `complete`. Partial output deliberately exits successfully so it remains consumable; scripts requiring full coverage must reject `complete: false`. Errors before scanning, such as an unknown selected account or an invalid duration, exit unsuccessfully.

`complete` describes scan coverage, not an atomic snapshot across servers. Concurrent arrivals, deletions or moves can shift pages while scanning. Detected overlap is deduplicated within its account and marks coverage incomplete; undetected concurrent changes remain possible. Retry when stable snapshot-like results are needed.

## Verification

```sh
cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
cargo build --all-features
python3 scripts/inbox_smoke.py
python3 scripts/security_smoke.py
```

The inbox smoke test uses temporary synthetic Maildirs and a loopback PREAUTH IMAP fixture. It checks merged and selected routing, colliding IDs, received-versus-sender dates, pagination past an old message, read-only fetches, configured windows, explicit truncation, partial failures and terminal sanitization. It does not authenticate to a public mailbox.
