---
cairn: spec
capability: local-setup-web-ui-testing
status: current
---

# Editable account-settings verification

Date: 2026-10-17, macOS aarch64. Working tree over 4bff0fcc739683b9d492f34354a1bbb41299b0d9 on codex/unified-inbox. The user approved a single editable account-settings form after rejecting provider restriction, terminal embedding and sequential questions. No production credential or real provider authentication was used.

## Automated evidence

| Check | Result |
| --- | --- |
| Default `cargo test --offline --quiet -- --test-threads=1` | 324 unit and 6 integration tests passed; 5 opt-in/pre-existing tests ignored |
| Default strict clippy, fmt and diff checks | `cargo clippy --offline --all-targets -- -D warnings`, `cargo fmt --check`, `git diff --check` passed |
| JavaScript syntax | Node --check passed for the app and updated opt-in browser smoke script |
| Terminal security/inbox regressions | All 17 security checks and 21 inbox checks passed against the built default executable |
| Signals | Isolated SIGTERM/SIGHUP fixture passed, with settled shutdown and no surviving owned worker |
| New typed/settings coverage | Arbitrary service domains, UTF-8 secrets, encrypted transport, ambiguous host refusal, JMAP URL/header controls, explicit destination/custom/raw consent, independent/shared SMTP credentials, local stores, unknown fields, generation-bound async results, cancellation and no-poll watchdog passed |
| Save and recovery | Revision/ticket binding, one-use save, second-item create failure, continued cleanup after a cleanup error, expired sessions, legacy flow exclusion, outcome reconciliation overtaking a delayed save, possible-partial-write credential retention and recovery-required outcomes passed |
| Transport sizing | 33,285-byte and 128-KiB declared settings bodies accepted at the authorization boundary; excess refused |
| Native Keychain IPC | Opt-in bounded helper test passed 16-character, variable-length Japanese/emoji and 4096-byte values using unique disposable items; exact owned-item deletion and subsequent missing-item reads passed |
| Reduced builds | wizard/JMAP/rustls and wizard/IMAP/SMTP/Maildir/rustls checks completed; unrelated existing unused/dead-code warnings are not strict-clippy claims |

An initial parallel full run hit an unrelated pimdir fixture `Owned` race at src/pimdir/backend.rs:1907. The exact test passed on immediate rerun; the complete serial suite then passed. No adjacent pimdir source was changed.

## Actual browser evidence

CUA operated the actual bundled page and authenticated HTTP boundary using the explicitly ignored Rust browser fixture. Its config was disposable with a synthetic personal account, and its public test-only capability is 64 lowercase a characters. Production capabilities were never printed.

With the actual default binary as the private worker:

- Email is immediately visible and existing names appear in the sidebar.
- A missing destination acknowledgement produces an inline error.
- A disposable local-store check completes; editing the root invalidates save and clears acknowledgement.
- An invalid root fails without losing other input. Correcting it allows check and save.
- Two local accounts were saved consecutively in the same session, preserving original comments/defaults.
- The final installed-source version repeated two consecutive local saves and acknowledged finish. Its browser warning/error log was empty.
- Credential-source changes clear masked input in the final version.

With the synthetic typed worker in scripts/web_settings_fixture.sh:

- Explicit discovery populated editable IMAP/SMTP fields and offered IMAP/SMTP, JMAP, Gmail API and Graph.
- Editing while discovery was active preserved the manual field instead of applying a stale result.
- Advanced source lists included missing external brokers and explicit raw/custom warnings.
- Raw-to-custom switching cleared approval; command edits also cleared approval.
- A dummy raw-token check returned a redacted preview and cleared masked fields.
- A narrowly scoped intercepted save response simulated recovery_required: editing/check/save/add stayed disabled, retry guidance was absent, and finish remained available and required acknowledgement. Interception was cleared and the fixture closed; no provider or Keychain operation was issued by this synthetic scenario.
- At 390px width the page had no horizontal overflow. The temporary viewport was reset.

These are local/synthetic checks, not live Gmail/iCloud/JMAP/Graph tests. The updated standalone Playwright smoke script was syntax-checked but was not executed this turn; browser interactions above used CUA. Local assets and lack of persistent browser storage are additionally verified by source inspection, not claimed as a separate runtime network/storage audit.

The screenshot screenshots/account-settings-web-ui.jpg is the final blank fixture page with dummy metadata only. Test servers finished successfully and their browser tabs were closed or left for automatic cleanup.

## Independent review and installation

Astra independently reviewed source only. Findings concerning consent reuse, aggregate IPC sizing, save-timeout reconciliation, partial-write outcome classification, HTTP auth controls and compatibility-flow exclusion were addressed. Its final source review reported no actionable blockers within the agreed threat model. Test execution was performed by the main agent, not independently by Astra.

The built and installed default binaries matched. The one-click launcher opened the localhost application; email entry, existing account names and disabled-until-checked saving were verified without submitting data. The integration configuration remained unchanged across installation. Machine-specific paths, account metadata and binary hashes are omitted from this public report.

## Limits

No non-macOS cross-build or cargo-deny scan is claimed. Token brokers, OAuth provisioning, organizational policy and user-item Keychain permissions remain external. Existing credential stores are not enumerated. Same-user malware, privileged extensions, root, untrusted parents, hostile non-cooperating writers and malicious commands deliberately detaching remain outside the boundary. External credential operations are not transactional with config saving; process/browser zeroization is best effort.
