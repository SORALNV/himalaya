---
cairn: log
change: unified-inbox
landed: 2026-10-11
---

# Fork-only unified inbox and recent windows

Added `inbox list`, merging all configured accounts unless global `-a` selects one. Entries retain account, mailbox and backend ID, and equal IDs across accounts are preserved. Positive `s`, `m`, `h` and `d` durations are accepted by `--since`; global `[inbox] since` supplies a default and `--all-time` disables it. Filtering uses backend receipt metadata and a fixed inclusive time interval, never silently substituting sender Date. Local summaries lacking receipt metadata remain available unfiltered but are explicitly excluded from recent windows.

Added bounded per-account scanning, explicit failure and missing-timestamp reporting, overlap detection, deterministic merged sorting and structured JSON coverage. Empty pages establish exhaustion; old timestamps and short pages do not prematurely terminate a scan. Partial results are consumable with a successful exit, with `complete: false` for strict consumers. Coverage is not an atomic snapshot of changing servers. IMAP summary listing uses read-only EXAMINE and requests INTERNALDATE. JMAP requests receivedAt, Gmail preserves internalDate and Graph preserves receivedDateTime. Graph pagination follows next links with an HTTPS origin guard. Legacy envelope JSON is unchanged.

Updated commands/config specs, added the unified-inbox capability and usage documentation, and recorded the user's fork-only policy in AGENTS.md. No upstream feature PR was created and existing upstream security PR branches were not modified.

Verification on macOS arm64: all-features tests passed with 261 unit successes, six integration successes and one pre-existing ignore; all-features/all-targets Clippy passed with warnings denied; formatting and diff checks passed. Reduced IMAP/SMTP and JMAP builds passed, and the Sieve-only suite passed 64 tests, retaining existing reduced-feature warnings. The new synthetic inbox smoke script passed 21 checks and the existing security smoke script passed 17 checks. These cover account scopes, ID collisions, duration parsing/default overrides, receipt-versus-sender dates, window boundaries and time zones, bounded scans, partial failures, read-only IMAP metadata and terminal sanitization. REST receipt extraction and Graph paging-origin validation have unit coverage; live REST-provider testing was not performed.

The existing private configuration parsed successfully. A bounded read-only Gmail IMAP recent-window invocation authenticated, obtained receipt metadata and exhausted the selected inbox without coverage errors. No body was fetched, no message was sent and no credentials or private mail were added to fixtures or this log.

OpenAI Codex implemented and verified this feature. No independent subagent or human review of the code is claimed.
