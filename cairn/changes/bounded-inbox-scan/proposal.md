---
cairn: change
id: bounded-inbox-scan
status: landed
created: 2026-10-18
---

# Bounded resumable receipt scans

The user approved fixing the fork and read-only wrapper, Astra review and real-machine read-only retrieval. Add a single-account IMAP inbox scan with server-side received-date candidate search, exact INTERNALDATE filtering, a fixed UID/as-of snapshot, bounded fetch/output batches, opaque resumable cursors and completed UID checkpoints. Never stop at the first old date or replace receipt time with sender Date. Existing cross-backend inbox list remains available.

The wrapper will isolate accounts, classify safe failures, retain valid partial results and expose continuation rather than conflate timeouts/output limits/authentication. A separate acknowledged monitoring path may retain only private metadata checkpoints: pending delivery must replay until acknowledged, never silently advance and lose notifications. Ordinary inbox queries remain stateless. Default limits stay below the process/output boundary; no sends/deletes/Seen changes, secret output, live bot/conversation changes or upstream PR changes. New protocol logic remains a thin adapter over existing io-imap APIs.
