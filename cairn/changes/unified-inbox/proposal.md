---
cairn: change
id: unified-inbox
status: landed
created: 2026-10-11
---

# Unified inbox with configurable recent windows

The user requests a unified inbox across configured accounts and a configurable recent window such as thirty minutes. This is a new fork feature, kept separate from the existing upstream security PRs.

Approved interface: `inbox list` for every configured account, global `-a NAME` for one account, `--since 30m`, `--since 2h` or `--since 1d` for recent mail, and a persistent default window in configuration. Results carry account, mailbox and original message ID, are ordered deterministically, and never deduplicate different accounts merely because their IDs match. Summary retrieval does not mark mail seen. A failed account is reported explicitly rather than making partial data look complete. The user approved implementation and requires this and future custom features to remain fork-only, with no upstream feature PR.

Recent means server receipt, never the sender-controlled Date header. The user clarified that both all-account and selected-account windows are required. Receipt filtering must never silently fall back to sender Date. Define pagination bounds and explicit incomplete results rather than silently skipping unseen pages. No credentials or private mail will enter repository fixtures.
