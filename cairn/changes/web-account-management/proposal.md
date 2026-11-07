---
cairn: change
id: web-account-management
status: active
created: 2026-10-18
---

# Manage existing accounts in the local Web UI

## Requested outcome

The user requests editing and deleting registered accounts from the existing Web UI, and fewer repeated clicks. The present UI lists names but only appends new accounts. Its protected writer is append-only; safe editing/removal requires an explicit replacement workflow rather than truncating the live config or rebuilding all accounts from the limited form schema.

## Proposed intent for human review

Select an existing account in the sidebar to edit its supported connection/authentication settings using the same normal form. Keep existing credentials by default through an opaque server-side reference: never return stored raw secrets or arbitrary credential commands to the browser, never resolve credentials just to display an account, and never overwrite shared Keychain items. Explicit secret replacement creates a separate new item only when saving. Preserve unrelated/global/advanced fields, literal values, comments and other accounts. Unsupported or inherited settings must not silently disappear during an edit.

Provide a distinct account-removal action with target-specific confirmation. Removal means deleting only the Himalaya account configuration, not email messages, remote accounts, local mail files or Keychain items. If the default account is removed and others remain, require an explicit replacement default. Keep a private local recovery backup before an edit/removal; never log its contents or expose arbitrary paths in browser requests.

Bind edits/removals to the original target and server-held snapshot, with revision-bound one-use authorization. Reuse localhost capability/Host/Origin/CSP and lifecycle protections. Add a guarded durable replacement writer with private permissions, ACL/regular-file/symlink/inode/content validation and coordinating locks; detect concurrent external edits and fail safely instead of overwriting them. Report uncertain commit/cleanup states without enabling automatic retries. Browser tests use disposable configs and fake credentials, never the user's actual accounts.

The phrase about repeatedly pressing a control is ambiguous. Clarify whether it refers to the launcher, destination acknowledgement, or another button before changing launch/authentication/consent behavior. Do not install a persistent service, weaken session authentication, expose a fixed unauthenticated localhost endpoint, or remove safety confirmation based on a guess.

Implementation is pending human review of this intent. No production config modification/deletion, credential enumeration, startup-service installation, commit, push or PR is authorized by this proposal. Custom work remains only in SORALNV/himalaya.
