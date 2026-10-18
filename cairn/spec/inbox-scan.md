---
cairn: spec
capability: inbox-scan
status: current
---

# Fork-only bounded IMAP receipt scans

### Requirement: Scope and receipt timestamps

`inbox scan` SHALL require one explicit account and use the IMAP storage selected by the requested backend. Automatic selection SHALL reject an account whose compiled higher-priority storage is not IMAP. Mailbox aliases SHALL resolve before scope binding. Cursor scope SHALL bind account, resolved mailbox, normalized endpoint and non-secret authentication identities with SHA-256; passwords and secret references SHALL NOT enter cursor metadata. Unsupported backends SHALL return a fixed safe error, not claim complete coverage.

A new round SHALL capture UIDVALIDITY and UIDNEXT through EXAMINE, derive a finite upper UID and then choose its default as_of. Explicit `--as-of` SHALL be supported for fixed receipt-time queries. Receipt candidates SHALL use server-side internal-date SINCE, widened by one date to accommodate timezone loss in IMAP date predicates. Exact inclusive filtering SHALL use INTERNALDATE between since and as_of, never sender Date. The scan SHALL NOT stop at an old receipt. No upper date predicate SHALL hide future-dated UIDs.

### Requirement: Read-only bounded batches

Scanning SHALL issue EXAMINE, UID SEARCH and UID FETCH of UID, INTERNALDATE, flags, envelope and size only. It SHALL NOT fetch bodies, change Seen, send, copy, delete or otherwise mutate messages. Each call SHALL allow 1–100 messages per fetch, 1–4 fetch batches and 8192–131072 serialized response bytes; defaults SHALL be 25, 4 and 131072. The final pretty-JSON representation plus newline SHALL fit the selected byte bound, including empty/error responses. CLI text output SHALL contain coverage metadata only.

FETCH bounds SHALL NOT be described as a hard bound on candidate-discovery memory: the underlying IMAP search API materializes matching UIDs before fetch batching. Wrapper process deadlines SHALL bound the wall-clock attempt, not assert a hard RAM limit. Explicit all-time scans MAY discover a large UID set. Clients SHALL use bounded receipt windows for normal monitoring.

### Requirement: Stable continuation and honest coverage

Opaque versioned metadata cursors SHALL retain UIDVALIDITY, UID upper bound, since, as_of, receipt-window policy, scope digest, processed UID position and any deferred UID floor. They SHALL reject malformed, oversized, wrong-kind, changed-scope, changed-window, future-as_of, impossible range and UIDVALIDITY changes without silently resetting. A resumed cursor SHALL preserve its original time/UID snapshot. Concurrently expunged messages and mutation before observation SHALL NOT be promised to form an atomic mailbox snapshot.

Output SHALL include account, mailbox, since, as_of, complete, scanned, matched, uid_validity, snapshot_uid, deferred, cursor, next_cursor, checkpoint, reason and entries. `cursor` SHALL identify the start of this emitted batch for replay. Incomplete batches SHALL expose next_cursor and no checkpoint. Output/page bounds, missing fetched rows, absent INTERNALDATE, invalid UID responses and protocol failures SHALL preserve the successfully processed prefix and leave unprocessed positions available for retry. An entry too large to fit SHALL NOT be skipped. Complete SHALL mean all candidates for this fixed receipt interval were checked, not that a truncated attempt found no mail. Safe static errors SHALL NOT embed provider diagnostics or secret-helper output.

### Requirement: Acknowledged incremental rounds and deferred receipts

Only a completed scan SHALL produce a checkpoint. `--after` SHALL start a new UID round and extend the receipt cutoff to at least the previous as_of when the configured lookback would otherwise leave a downtime gap. Empty/new-UID-free rounds SHALL NOT issue reversed IMAP ranges. UIDVALIDITY changes and regressed UID state SHALL fail closed.

Any INTERNALDATE newer than as_of SHALL be excluded from current entries but retained as a constant-size deferred UID floor across batches. Completion SHALL set deferred=true and restrict the next checkpoint watermark to the UID immediately before the first deferred UID, so a later round can revisit it. This MAY replay later UIDs under another batch identifier; consumers requiring deduplication SHALL use account, mailbox, UIDVALIDITY and UID. The original snapshot upper UID SHALL remain visible independently of the checkpoint watermark. No unbounded deferred UID list SHALL be stored.

The paired read-only wrapper SHALL isolate accounts, cap calls and aggregate output, preserve continuation and expose fixed failure codes. Its optional monitoring path SHALL store only private metadata and advance a pending cursor/checkpoint only after explicit matching acknowledgement. Ordinary inbox queries SHALL NOT consume that state. Pending replay changes SHALL stop rather than acknowledge an altered batch. No live bot, conversation or scheduled task SHALL be modified merely to verify this capability.
