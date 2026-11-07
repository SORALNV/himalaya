---
cairn: tasks
change: local-setup-web-ui
---

- [x] Inspect current fork, wizard save guards, Keychain flow and existing setup failures.
- [x] Define initial feature scope, threat model, explicit exclusions and release gates.
- [x] Obtain human review of this proposal before implementation (approved 2026-10-17).
- [x] Read shared contribution/architecture requirements before source changes; select and review maintained HTTP and native Keychain dependencies and feature gates.
- [x] Extract the validated config snapshot/append boundary for reuse without weakening existing CLI protections.
- [x] Implement typed Gmail/Google Workspace and iCloud drafts, provider/transport allowlists and reference-only previews.
- [x] Implement native Keychain storage/reference handling, new-item ownership tracking, cleanup and classified partial-failure reporting.
- [x] Implement loopback-only lifecycle, private browser bootstrap, capability authorization, Host/Origin/CSRF checks and bounded HTTP processing.
- [x] Implement bundled Japanese setup UI with restrictive headers and no external resources or persistent browser storage.
- [x] Connect explicit draft tests and saves with stale-draft/concurrency/session-expiry protection; do not expose mail operations or arbitrary command execution.
- [x] Add isolated HTTP, redaction, provider-policy, Keychain-fake and filesystem attack tests, including negative side-effect assertions.
- [x] Run disposable native Keychain tests, terminal-wizard regressions, security/inbox smoke suites and supported feature builds; record the pre-existing TLS-free build limitation.
- [x] Perform real-browser fixture end-to-end tests; leave real-provider authentication to an explicit user-operated check.
- [x] Review credential handling and attack boundaries independently with Astra; fix findings and report verification limits.
- [x] Fold the delta into current specs, add a dated log, document usage, and mark landed after the implemented behavior passes the release gate.
