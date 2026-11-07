---
cairn: delta
change: web-account-management
---

## ADDED Requirements

### Existing account management

The local Web UI SHALL permit selecting, editing and removing an existing account only through explicit authenticated operations against the fixed configuration target. Current credentials SHALL remain server-held by default. List/load SHALL never resolve or reveal secrets or arbitrary credential commands. Unrelated accounts, defaults, unknown/inherited settings and comments SHALL not be silently lost. Removal SHALL affect only the selected configuration entry and require explicit target confirmation; default replacement SHALL be selected explicitly when needed.

### Recoverable targeted replacement

Before edit/removal, a private local backup SHALL preserve the validated original bytes. The replacement SHALL be snapshot-bound, concurrency-aware, permissions/ACL-safe and durable, without blind truncation or arbitrary browser-selected paths. Failure or uncertain persistence SHALL be reported without unsafe automatic retry. Keychain items, remote mail/accounts and local messages SHALL never be deleted as a side effect of account removal.

## MODIFIED Requirements

### Editable fork-only account workspace

Replace the current add-only limitation with explicit existing-account management while preserving typed validation, protected saves, credential privacy and local session boundaries. Streamlined launch/interaction behavior SHALL be specified after clarification of the user's repeated-click concern.

## REMOVED Requirements

Remove only the prohibition on explicitly requested existing-account edits/removal. Do not remove HTTP authentication, secret redaction, save guards or confirmation requirements by assumption.
