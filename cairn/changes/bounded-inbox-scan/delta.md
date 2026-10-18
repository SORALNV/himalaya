---
cairn: delta
change: bounded-inbox-scan
---

## ADDED Requirements

Receipt scans SHALL search by internal received date, fix account/mailbox/UIDVALIDITY/UID upper bound and time boundaries across continuations, enforce entry/byte limits, and report incomplete coverage honestly. Completed checkpoint polls SHALL fetch only later UIDs, preserve gaps on failure and detect changed UID validity. Read-only execution SHALL use EXAMINE and metadata FETCH, never bodies or mutation commands.

## MODIFIED Requirements

Monitoring SHALL isolate per-account failures and classify process limits without forwarding raw diagnostics. Durable checkpoints SHALL advance only after explicit receipt acknowledgement; state SHALL contain metadata only and be private, locked and atomically replaced. Ordinary listing SHALL not consume monitoring checkpoints.

## REMOVED Requirements

None. Existing read-only restrictions, ordinary inbox listing and upstream/fork separation remain intact.
