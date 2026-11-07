---
cairn: delta
change: account-settings-ux
---

## ADDED Requirements

### Editable account-settings workspace

The Web UI SHALL expose email input immediately and group connection and authentication settings into an editable form alongside existing account metadata. Ordinary labels SHALL explain the user's action; specialist controls SHALL remain accessible under details. Errors SHALL allow in-place correction of non-secret values. Launching a source HTML file SHALL NOT appear to be a functional authenticated setup session.

## MODIFIED Requirements

### Explicit testing and saving of typed drafts

The UI SHALL not depend on one-question terminal ordering. Test/save actions SHALL bind to the exact typed draft, invalidate on edits and never implicitly replay authentication or custom commands. A user-selected new password MAY be persisted only through a dedicated create-only Keychain path on explicit save, with bounded operations and exact rollback safeguards. Existing account content and permissions SHALL remain protected.

## REMOVED Requirements

No security boundaries are removed. No compiled provider/backend/authentication option is removed. The approved manual destination fields replace the browser no-manual-remote-fields limitation with explicit destination and encrypted-transport validation.
