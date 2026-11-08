---
cairn: change
id: fork-upstream-maintenance
status: landed
created: 2026-10-14
---

# Integrate useful upstream fixes without weakening fork security

## Why

The user requests useful changes from the current upstream while explicitly preserving the fork's stronger wizard save-target checks and credential-helper redaction. The fork's base (`341daeb`, 2026-10-09) is 52 commits behind upstream master (`7bfaaa09`, 2026-11-06). The fork already contains the attachment-exclusive creation, message-view sanitization and private-config wizard fixes, so replaying those implementations adds no value.

## What

Import the upstream fixes and small features that keep the fork's architecture: the Message-ID sender-domain fix, whole-byte raw message input, the corrected IMAP copy/move counts and fallback, the UIDVALIDITY 0 `COPYUID`, the count of message ids found, the one `hasAttachment` rule across the MIME backends, Maildir mailbox names relative to the root, the platform config-path separator, the `secret-tool` attribute fix, the RustSec dependency bumps, the nix bump, the README and provider-report documentation, and the three message features (`message reply --all`, `message forward` carrying its source's attachments, `answered`/`forwarded` flags on a sent reply or forward) with `msgraph messages send-draft`.

Preserve the wizard implementation and its descriptor, snapshot, inode, nonregular-file, FIFO and macOS ACL defenses byte-for-byte. Preserve the fork's pinned credential-helper redaction dependency (`SORALNV/config`), unified inbox, account identity, server receipt timestamps and explicit incomplete coverage.

Do not import the changes that move the fork away from its mail-parser architecture or its v2 command surface: the mime-rs reader, the capability-union shared API and its verbs, the machine output contract and camelCase JSON keys, the rc-file layering, the REPL and MCP server, the new search clauses, the pimdir library re-pins and their attachment list/search feature, and the upstream simplification of the wizard save target. Send no mail and access no real credentials. Validate with offline unit/integration tests, reduced feature builds, inbox and security smoke checks, formatting and Clippy before committing locally.

## Sources

Reviewed upstream master is `7bfaaa09d37d428709d6562d63b4d9a8a4dab15e`. Imported commits retain provenance in the fork's history through `git cherry-pick -x`. Publishing to the fork can follow verification; no upstream feature PR is planned.
