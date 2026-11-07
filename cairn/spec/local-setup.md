---
cairn: spec
capability: local-setup
status: current
---

# Local browser account settings

### Requirement: Editable fork-only account workspace

The macOS fork SHALL offer `configure --web` with wizard, IMAP, SMTP and TLS enabled. Email entry SHALL be immediately visible alongside existing account names and grouped editable connection/authentication fields. Explicit discovery SHALL reuse terminal service search and populate editable choices; manual endpoints/local roots SHALL be available without a provider allowlist. Compiled IMAP/SMTP, JMAP, Gmail API, Graph and local-store choices SHALL remain available. Specialist credentials SHALL remain under details. Non-secret drafts SHALL survive failed checks for in-place correction. The UI SHALL add accounts without editing/removing existing accounts, selecting arbitrary save paths or exposing message operations or native OAuth grants. The supplied per-user launcher SHALL run only the installed fork against the explicit integration config. File-source previews SHALL clearly state that they are not authenticated applications.

### Requirement: Local requests authenticate and remain bounded

The session SHALL bind an ephemeral 127.0.0.1 port, generate a fresh OS-random 256-bit capability, remove its bootstrap fragment and require a constant-time-checked custom header. Exact Host and POST Origin, rejected foreign/null origins/preflights, bounded concurrency/framing/headers/read time and no GET side effects SHALL remain enforced. Settings JSON SHALL be at most 128 KiB, private settings input 256 KiB and worker output 64 KiB; legacy requests SHALL retain their smaller limits. Browser inputs SHALL NOT select executables, worker argv or config paths. Only the fixed executable's private workers SHALL perform discovery/checks without loading the existing config. Work SHALL time out after 45 seconds independently of polling.

### Requirement: Typed drafts bind consent, checks and saving

An explicit check SHALL validate typed backend, server, authentication and credential fields before side effects. Remote transports SHALL require TLS/STARTTLS or HTTPS with certificate validation, no server userinfo, unambiguous hosts/ports and explicit visible destination confirmation. The explicit macOS MPRG compatibility profile MAY replace modern CA/hostname validation with same-connection leaf pin verification under the separate legacy-mprg requirements; ordinary endpoints SHALL remain unchanged. JMAP URLs SHALL reject credentials, query/fragment, whitespace and backslashes; resolved HTTP auth SHALL reject control characters. JMAP checks SHALL not follow redirects to unconfirmed destinations. API targets SHALL be fixed service endpoints. Local roots SHALL be absolute paths.

Draft revisions and server generations SHALL prevent stale results/edits from enabling save or overwriting current fields. Every edit SHALL invalidate the ticket/destination consent; authentication or commands SHALL never replay implicitly. A successful worker SHALL authorize only the exact parent-retained typed document via a one-use revision-bound ticket. Rejected requests SHALL have no credential/provider/save side effects. Legacy compatibility flows SHALL not interfere with active/new-ready/recovery-required settings state.

### Requirement: Secrets and advanced sources require explicit handling

Assets SHALL be bundled with restrictive CSP, anti-framing, no-store and no-referrer headers, and untrusted text SHALL never become HTML. The page SHALL use no analytics, external assets, service workers or persistent credential storage. Submitted secrets SHALL not appear in logs, previews, errors or answer history. Password inputs SHALL clear after submission/source changes; inactive credential values SHALL not be submitted. Private IPC SHALL not use secret URLs, arguments, environment values or temporary files. Memory erasure is best effort.

New passwords/tokens SHALL accept bounded UTF-8 text and remain transient until explicit save creates a dedicated server-named Keychain item. Existing Keychain/pass/broker choices SHALL use framework inventories/command builders, retain missing tools as labelled choices and never enumerate/overwrite/delete existing items. A broker MAY refresh or request authentication before save; the UI SHALL not claim to provide OAuth grants or token refresh for fixed stored tokens. Custom commands SHALL require explicit local-execution approval; source/command changes SHALL reset approval. Raw storage SHALL require separate approval and matching masked repeat input; only that choice permits raw secrets in saved TOML. Previews SHALL redact entire credential/proxy branches, including commands.

Checks SHALL clone typed configs without serde re-expansion, resolve commands under ten-second/8-KiB-output/null-stderr bounds and preserve saved references. Workers SHALL retain only the documented local-context environment allowlist, not proxy/logging controls. Checks MAY read protocol/session/profile metadata but SHALL not fetch message bodies, send or delete mail or bypass organization policy.

### Requirement: Owned lifetimes and guarded persistence

Finish, Ctrl-C, SIGTERM, SIGHUP, ten minutes without accepted user action or thirty total minutes SHALL stop the session. Status/poll/outcome/rejected requests SHALL not refresh idle expiry. Cancel SHALL drop pending work/secrets/ready tickets. Every worker termination path SHALL kill its owned group and reap the direct child. Pipe reads/writes/output EOF SHALL remain nonblocking and deadline-bounded. Deliberately detached commands are outside this cleanup guarantee.

The fixed target SHALL retain the terminal descriptor/snapshot writer: unchanged existing bytes/comments/defaults, private modes, macOS ACL refusal, regular/nonblocking descriptor checks, original device/inode/content checks, unchanged-symlink compatibility, advisory exclusive append lock and live checks coordinated with revocation at commit. Refresh after success SHALL permit another account on the same target.

Multi-credential save SHALL create only server-generated new Keychain items, recheck session validity between creations/commit and retain native persistent identities. Pre-write failure SHALL attempt cleanup of every exactly owned new item even after cleanup errors. Uncertain creation SHALL never trigger deletion by guessed name. Possible partial config writes SHALL retain created credentials. Uncertain creation/cleanup or partial writes SHALL produce recovery-required outcomes and block ordinary retry routes/controls. Save-result reconciliation SHALL bind ticket/revision, return retained outcomes or revoke a not-yet-started ticket before reporting abortion. Unknown outcomes SHALL not unlock retry or claim finish; finish SHALL require acknowledgement. External credential tools and config saving are not a transaction.

### Requirement: Evidence-backed fork release

Release SHALL require isolated HTTP authorization/framing, typed schema/transport/consent, redaction, revision/generation, cancellation/watchdog, output/deadline/process cleanup, stale/concurrent saves, multi-item rollback, outcome/recovery and guarded filesystem coverage, plus terminal security/inbox regressions and actual-browser local/synthetic fixtures. Native Keychain tests SHALL use disposable items only; real-provider checks require user action. Same-user malware, privileged extensions, root, untrusted parents, non-cooperating writers and malicious detached commands SHALL be documented limitations. The feature SHALL stay in SORALNV/himalaya and separate from upstream security PRs.
