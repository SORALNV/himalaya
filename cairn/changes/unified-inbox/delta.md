---
cairn: delta
change: unified-inbox
---

## ADDED Requirements

### Requirement: Unified inbox keeps account identity
The unified listing SHALL include the source account, mailbox and original ID for every result. It SHALL keep account failures and incomplete coverage explicit, and SHALL NOT mark mail seen or conflate IDs across accounts.

### Requirement: Recent window is configurable
Users SHALL be able to choose a positive duration through CLI arguments and persist a default in configuration. The selected time basis SHALL be explicit, with no silent substitution of sender dates for unavailable received timestamps.

### Requirement: Fork-only customization policy
Custom features SHALL remain only in SORALNV/himalaya unless the user changes that policy. Existing upstream security PRs SHALL remain separate.

## MODIFIED Requirements

### Requirement: Shared summary retrieval
IMAP summary listings SHALL use EXAMINE and request INTERNALDATE. JMAP summary requests SHALL include receivedAt. Shared envelopes SHALL carry receipt metadata internally without changing legacy envelope JSON. Graph summary pagination SHALL follow trusted server cursors rather than calculated offsets.
