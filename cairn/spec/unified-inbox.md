---
cairn: spec
capability: unified-inbox
status: current
---

# Fork-only unified inbox

### Requirement: Account identity and routing

`inbox list` SHALL merge inbox summaries from all configured accounts in deterministic order. Global `-a NAME` SHALL select one account. No default account SHALL be required for an all-account listing. Each entry SHALL retain account, resolved mailbox and backend message ID. Equal IDs in different accounts SHALL remain distinct. `-m NAME` SHALL resolve each account's mailbox alias. Backend selection SHALL follow global `--backend`.

### Requirement: Configurable receipt windows

`--since` SHALL accept a positive checked integer duration with unit `s`, `m`, `h` or `d`. Global `[inbox] since` SHALL provide an optional default, CLI `--since` SHALL override it and `--all-time` SHALL disable it. Filtering SHALL use one command-start timestamp and an inclusive interval from cutoff through that timestamp. Future receipt timestamps SHALL be excluded. Read and unread messages SHALL both qualify.

Receipt filtering SHALL use backend receipt metadata: IMAP INTERNALDATE, JMAP receivedAt, Gmail internalDate or Graph receivedDateTime. Missing receipt metadata SHALL exclude a row from a recent listing and be counted explicitly, never replaced by a sender Date or filesystem modification time. Unfiltered rows lacking receipt metadata MAY use the existing envelope date for sorting, with the table identifying that time basis. Normal envelope JSON SHALL retain its existing shape; unified output SHALL expose receipt metadata separately.

### Requirement: Read-only summary retrieval

Inbox listing SHALL not fetch message bodies, send mail, delete messages or mutate Seen flags. IMAP summary listing SHALL use EXAMINE and fetch receipt metadata alongside headers, flags, IDs and sizes.

### Requirement: Coverage and bounded scanning

Each account SHALL be scanned sequentially up to configurable positive bounds. Defaults SHALL be 100 pages of 100 summaries, with maximum allowed settings of 10000 pages and 500 summaries per page. Scanning SHALL not stop merely because a fetched receipt timestamp is old or a page is shorter than requested. Empty pages SHALL establish exhaustion. Graph listings SHALL follow server next links rather than calculate offsets and SHALL reject next links outside the HTTPS graph.microsoft.com origin.

Account errors SHALL not discard summaries from successful accounts. Plain output SHALL sanitize untrusted cells and warnings. JSON SHALL emit one structured object with entries, per-account coverage, cutoff and start timestamps, and a complete flag. Errors, bounds, missing receipt metadata in a filtered scan or repeated IDs within an account SHALL mark coverage incomplete. Partial output SHALL remain a successful command invocation; strict consumers SHALL check complete. Repeated IDs SHALL be deduplicated only within an account and explicitly counted.

Coverage SHALL not promise an atomic snapshot across servers or changing mailboxes. Concurrent mailbox changes MAY shift pages even when no overlap is detected.

### Requirement: Fork isolation

This capability and subsequent custom features SHALL remain only in SORALNV/himalaya unless the user changes that policy. Existing upstream security PRs SHALL not receive custom features.
