---
cairn: delta
change: local-setup-web-ui
---

## ADDED Requirements

### Requirement: Explicit fork-only local setup session

The macOS fork SHALL offer an explicitly launched `configure --web` setup session for adding Gmail/Google Workspace and iCloud accounts. It SHALL bind only an ephemeral `127.0.0.1` port, close on authenticated finish/cancel, enforce authenticated-idle and absolute lifetimes, and leave the existing terminal wizard available. Unsupported builds/platforms SHALL fail clearly without starting a listener. The UI SHALL NOT expose mail content, send/delete operations, arbitrary servers, raw TOML editing, account replacement or native OAuth grants in its initial implementation.

### Requirement: Local setup authenticates browser requests

The session SHALL generate a cryptographically random per-launch capability with at least 256 bits of entropy, bootstrap it without a query/path/log disclosure, and require it through a custom header for all non-public API requests. It SHALL enforce the exact listener Host and state-changing Origin, reject foreign/null origins and cross-origin preflights, and never treat CORS, a loopback peer or a port number as sufficient authorization. Public resources SHALL NOT disclose capabilities or configuration data. Request framing, sizes, deadlines, concurrency and expired-session work SHALL be bounded. GET SHALL NOT mutate state, connect to a provider or read a Keychain secret.

### Requirement: Local setup keeps credentials out of persistent web state

The UI SHALL use bundled local assets, restrictive security headers, untrusted-text rendering and no browser persistent storage, analytics or external asset/network origins. New passwords SHALL be stored through native macOS Keychain APIs, never raw TOML, process arguments, environment variables, URLs, logs or response bodies. Saved credentials SHALL NOT be returned to the browser. Referencing a named existing item SHALL NOT enumerate, overwrite or delete existing Keychain items or execute arbitrary existing-config helper commands.

### Requirement: Local setup tests only typed provider drafts

An explicit connection check SHALL authenticate an in-memory typed draft only against fixed Gmail/iCloud provider endpoints with certificate validation and required transport encryption. It SHALL NOT accept arbitrary endpoint/proxy/command overrides, follow endpoint redirects, send messages or fetch mail contents. Save eligibility SHALL apply to the exact checked draft. Errors SHALL be classified and redacted rather than exposing raw secrets or backend output.

### Requirement: Local setup reuses guarded config persistence

The UI SHALL resolve a single save target before serving, refuse browser-selected paths and ambiguous multi-file targets, and share the terminal wizard's validated snapshot/append implementation. It SHALL preserve existing bytes and defaults, refuse duplicate names and conflicting saves, and keep Unix private modes, macOS ACL refusal, regular-descriptor/nonblocking checks, retained original identity/content checks and unchanged-symlink compatibility. Cooperating CLI/web writers SHALL lock the opened descriptor before revalidation through append. Save SHALL recheck session validity after validation and coordinate its commit with revocation; already-committed work SHALL resolve before shutdown is acknowledged. Requests rejected at admission SHALL NOT initiate credential, provider or save operations. Exact owned-item cleanup MAY complete after revocation. Native credential operations SHALL have enforceable deadlines, using persistent item identity for cross-process rollback. A creation timeout, cleanup failure or partial config write SHALL be reported explicitly, without claiming a transaction or deleting pre-existing items.

### Requirement: Local setup has adversarial and end-to-end release gates

Delivery SHALL require isolated tests for authentication, origins/Host/rebinding, framing/resource limits, XSS, redaction, provider allowlists, duplicate/stale/expired work, credential cleanup and filesystem races, plus existing CLI security/inbox regressions and real-browser fixture verification. Native credential tests SHALL use disposable owned entries. Real-provider authentication SHALL require an explicit user action, and documentation SHALL state the remaining same-user/browser/root and trusted-parent limitations. The customization SHALL remain fork-only and separate from upstream security PRs.
