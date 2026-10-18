---
cairn: spec
capability: inbox-scan-publication-testing
status: current
---

# Published bounded inbox scan

The published changes are restricted to the fork-only IMAP receipt scanner, its dispatch and internal adapters, the IMAP SHA-256 feature dependency, documentation and synthetic fixtures. Unrelated local Web setup and legacy transport work is not part of this publication. Upstream security PR branches are unchanged.

The exact staged source tree was exported to an isolated directory and checked with its own locked dependencies, without the unrelated dirty worktree changes. Offline default cargo check passed; its default unit suite passed 263 tests with one pre-existing ignored test. The larger operational worktree has additional local feature tests and is not the source of this published-tree test count.

The scanner's synthetic loopback fixture passes 23 checks: receipt timestamps rather than sender Date, fixed snapshots, sparse UIDs, resumed prefixes, incremental and empty ranges, downtime coverage, wrong scope/window/kind rejection, UIDVALIDITY changes, missing rows/dates, entry/output/error bounds, deferred future receipts and no message mutation. The existing inbox and security fixtures pass 21 and 17 checks. The paired wrapper's independently run suite passes 32 checks, including acknowledged state, private metadata storage, account isolation, process deadlines and surviving helper cleanup. An independent Astra review found no outstanding actionable findings after the reviewed fixes.

Authorized real-provider verification completed all four existing accounts for 30-minute and six-hour receipt windows. Small-batch scans completed, unacknowledged results replayed identically across process restarts, acknowledgements were idempotent and following incremental rounds completed. Only aggregate coverage metadata was recorded for publication; no account identifiers, mailbox content, subjects, sender addresses, credentials, host-private configuration identifiers or message UIDs are included here. These results do not establish bot notification delivery, arrival of a new real test message or long-running unattended reliability.

SEARCH materializes the candidate UID set. FETCH and serialized-output bounds are not a hard search-memory bound. Non-IMAP resumable scans are explicitly unsupported. Consumers must check complete, respect deferred coverage, retain continuation and acknowledge processing separately.
