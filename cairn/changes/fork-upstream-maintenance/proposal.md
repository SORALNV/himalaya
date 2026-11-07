---
cairn: change
id: fork-upstream-maintenance
status: draft
created: 2026-10-14
---

# Integrate useful upstream fixes without weakening fork security

## Why

The user requests useful changes from the current upstream and PR follow-up while explicitly preserving the fork's stronger wizard save-target checks. The fork already contains attachment-exclusive creation and message-view sanitization, so replaying those implementations or the simplified upstream wizard would add no value.

## What

Import the upstream Message-ID sender-domain fix, whole-byte raw message input, and corrected IMAP copy/move counts and fallback. Align the protocol libraries with upstream's pinned revisions, including its IMAP and HTTP response-buffer limits, and adapt the fork's receipt-aware inbox calls without changing their public contract.

Preserve the wizard implementation and its descriptor, snapshot, inode, nonregular-file, FIFO and macOS ACL defenses byte-for-byte. Preserve the fork's pinned credential-helper redaction dependency, unified inbox, account identity, server receipt timestamps and explicit incomplete coverage.

Do not import the simplified PR #784 implementation. Defer the unrelated new pimdir attachment list/search feature and do not change upstream PR branches, send mail or access real credentials. Record the upstream/fork distinction accurately in SECURITY_VALIDATION.md and Cairn. Validate with offline unit/integration tests, reduced feature builds, inbox and security smoke checks, formatting and Clippy before committing locally.

## Sources

Reviewed upstream master is `27d2ea709ad352560cccf285eeac11764219e495`. Intended source commits are `1c36a30`, `f81afdc`, `d230f3a`, `9287601`, `ef77718`, `252ec66` and `27d2ea7`. Their imported changes will retain provenance in the fork's history. Publishing to the fork can follow verification; no upstream feature PR is planned.
