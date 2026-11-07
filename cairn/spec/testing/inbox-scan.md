---
cairn: spec
capability: inbox-scan-testing
status: current
---

# Bounded receipt-scan verification

Verified on 2026-10-18, macOS 27.0.1 arm64, Rust 1.95.0 and Node 26.0.0. This report concerns the fork's scan and the local OpenMausBot wrapper, not registration of a live bot or delivery of notifications.

## Synthetic verification

- Root defaults: 333 unit tests and six message_parse integration tests passed; five existing opt-in tests remain ignored. Strict all-target clippy, formatting and diff checks passed.
- `scripts/inbox_scan_smoke.py`: 23 loopback IMAP checks passed. Covered fixed batch replay, sparse UIDs, partial continuation past old/out-of-order INTERNALDATE, independent next-UID rounds, empty ranges, downtime lookback extension, wrong scope/window/kind, UIDVALIDITY, missing rows/dates, oversized summaries, output budgets, future dates across partial batches, historical as_of recovery, future dates outside an upper-day search and error-response byte bounds. Wire assertions prohibit SELECT, body fetch and mutation commands.
- Existing inbox smoke: 21 checks passed. Existing terminal security smoke: 17 checks passed.
- OpenMausBot wrapper: 32 Node tests passed, including typed process limits, account isolation, aggregate byte limits, pending replay/ack/crash, invalid state, ownership/mode/link checks and metadata-only state. SIGTERM-ignoring helpers with ignored stdio are killed on TIMEOUT and OUTPUT_LIMIT even if the direct child exits first; inherited-pipe helpers are covered separately. Targeted oxlint, JS syntax and skill-format validation passed.
- IMAP-only modern-TLS and Maildir-only reduced feature checks passed. Existing reduced-build dead-code/unused warnings outside this change remain.

Two Astra agents participated: one implemented the disjoint wrapper slice and the main agent reviewed and executed its tests; a second independently reviewed the Rust scan, wrapper and live-check script without actual mail/credential access. Findings addressed: future-dated UID checkpoint loss, oversized error fallback, descendant cleanup after early parent closure and assertion of the final delta acknowledgement. Final independent review reported no outstanding actionable findings in that scope.

## Actual accounts and runtime

Existing production account configuration and its Keychain-backed secret commands were reused, without reading or emitting their values. Config inode 128183892, size 2382, mtime 1792246714 and mode 0600 remained unchanged. No account was added, removed or edited.

Read-only `inbox --since` through the wrapper, including formerly excessive `--page-size 100 --max-pages 100` input, completed all four accounts. Input was clamped to at most 50 summaries per fetch and four fetches. One 30-minute round returned two matches in 8.124 seconds; one six-hour round returned 18 in 6.685 seconds. The installed-binary repetition completed in 6.663 and 12.016 seconds respectively. Serialized output was 5156 and 13895 bytes, below 1 MiB. Each account reports its own exact since/as_of; distinct rounds do not share one atomic cross-server timestamp.

Independent-account six-hour monitoring used five-message single-fetch batches in an isolated temporary private state directory. The live account names and per-account counts are omitted from this public report:

All four configured accounts completed the replay, acknowledgement, continuation and checkpoint checks. The verification exercised both empty and multi-batch results; live mailbox counts are intentionally not published.

Every account passed identical unacknowledged replay, replay from a newly started Node process, explicit/idempotent ack, continuation to complete coverage with no duplicate IDs within that snapshot, and a following checkpoint poll with zero new UID candidates in the observed round. The installed binary passed this workflow again. Later rounds have their own rolling cutoff and can return different match counts; those counts are not a claim of identical mailbox state. Final-delta acknowledgements are asserted in the reusable live-check script.

Only counts, time bounds, completion and safe failure metadata were emitted to the verification transcript. Subjects, senders, bodies, attachments, credential values and actual message UIDs were not emitted or saved. The ephemeral monitor files contained only cursor/checkpoint/request/fingerprint metadata, were 0600 in a 0700 directory and were removed on verification exit. Existing production monitoring state and live bot conversations/instructions/automations were not changed. No SMTP session, send/delete/Seen action or test-mail delivery was used.

The installed binary matched the locally built binary during verification. Machine-specific installation paths and binary hashes are intentionally omitted from this public report.

## Limits of this evidence

The checks do not establish notification delivery by a running bot, arrival of a new real message during the test, long-running unattended operation, automatic recovery from changed UIDVALIDITY, or MPRG authentication in Himalaya. Candidate UID SEARCH materializes its match set; fetch/output limits are not a hard process RAM limit. The wrapper enforces bounded process attempts. Non-IMAP resumable scans remain explicitly unsupported; the existing bounded legacy listing remains available. No upstream PR, commit or push was performed.
