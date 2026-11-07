---
cairn: delta
change: mprg-legacy-transport
---

## ADDED Requirements

### Explicit per-service legacy exception

An explicitly approved compatibility profile MAY negotiate old TLS only for the configured MPRG service. It SHALL NOT activate from a network failure alone or alter other accounts, process/OS-wide TLS policy or certificate trust. Exact configured leaf identity SHALL be checked before authentication; pin changes SHALL require user action.

### Compatibility is not security equivalence

The UI and documentation SHALL disclose obsolete TLS, weak server parameters and unauthenticated initial pin-bootstrap risks. Login codes and mailbox credentials remain sensitive. Password input SHALL be local and bounded; validation SHALL not mutate messages or silently downgrade to plaintext.

## MODIFIED Requirements

For the explicitly approved MPRG profile only, the modern-TLS minimum is replaced by a narrow pinned compatibility transport. Every ordinary endpoint and Web request/session/persistence/secret boundary remains unchanged. This supersedes earlier proposal assumptions forbidding all MPRG legacy transport, not general security defaults.

## REMOVED Requirements

None outside the explicitly authorized per-service TLS-version exception. No secret handling, pin checking, guarded persistence or message-mutation protection is removed.
