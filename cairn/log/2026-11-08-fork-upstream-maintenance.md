---
cairn: log
change: fork-upstream-maintenance
date: 2026-11-08
---

# Integrate the upstream maintenance into the fork

The fork's base `341daeb` (2026-10-09) was 52 commits behind upstream master `7bfaaa09` (2026-11-06). Twenty-two of them were imported with `git cherry-pick -x`, so each keeps its provenance, and the rest were left out because they move the fork away from its mail-parser architecture, its v2 command surface or its pinned libraries.

Imported: the Message-ID on the sender's domain, raw messages read whole as bytes, the IMAP copy/move counts by source UID search, the UIDVALIDITY 0 `COPYUID`, the count of message ids found, one `hasAttachment` rule across IMAP, Maildir, m2dir and mbox, Maildir mailbox names relative to the root, the `$PATH`-style config-path separator, the `secret-tool` attribute fix, the rustls and RustSec dependency bumps, the nixpkgs bump, the README and provider-report documentation, `message reply --all`, `message forward` carrying its source's attachments, the `answered`/`forwarded` flags of a sent reply or forward, and `msgraph messages send-draft`.

Excluded: the mime-rs reader, the capability-union shared API and its verbs, the machine output contract and camelCase keys, the rc-file layering, the REPL and MCP server, the new search clauses, the pimdir library re-pins and their attachment list/search feature, and the upstream simplification of the wizard save target. The fork already carried the attachment-exclusive creation, the message-view sanitization and the private-config wizard fixes, so those upstream commits added nothing and were skipped.

Adaptations the fork needed: `part::has_attachment` walks the mail-parser tree rather than the mime-rs one, and Maildir, mbox and m2dir call it from their own envelope readers; the local `attachment` search clause is not imported, the fork's query language having none; `io-imap` is pinned to `5d83c49` for the UIDVALIDITY 0 `COPYUID`, and `pimalaya-cli` to `e4ea7f4` for the `secret-tool` fix, while `pimalaya-config` keeps the fork's `SORALNV/config` pin and `pimalaya-stream` its vendored tree; `message reply` keeps `reply_all: false` where the fork has no `--all` on the imported commit's base.

Verified offline: 394 unit and 14 integration tests, formatting, clippy with warnings denied, the reduced IMAP+SMTP and Maildir-only builds, 17 security smoke checks, 21 inbox smoke checks, 23 inbox-scan smoke checks and the isolated TERM/HUP teardown. The browser fixture was not run, no Playwright install being available. The wizard's save-target defenses, the credential-helper redaction and the receipt-aware inbox calls are unchanged.

Capabilities moved: **commands** (A composed Message-ID sits on the sender's domain: added; Raw message input is shared: modified), **backends** (IMAP move without the MOVE extension: modified), plus the already folded Maildir relative names, the IMAP copy/move and flag counts, the `hasAttachment` rule, the `reply --all`, the forward attachments and the flags of a sent reply or forward. Custom work remains only in SORALNV/himalaya; no upstream PR is created.
