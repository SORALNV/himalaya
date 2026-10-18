---
cairn: log
change: bounded-inbox-scan
date: 2026-10-18
---

# Prepare fork-only bounded scan publication

Prepared the user-authorized publication of bounded read-only IMAP receipt scans, fixed-snapshot continuation and incremental checkpoints. This publication includes only the scanner and its dependency/dispatch/adapter changes, synthetic tests and source documentation, leaving unrelated local feature work intact. Private operational reports and runtime-specific identifiers are not published.

The paired local wrapper isolates accounts, classifies process limits and stores private metadata-only progress with explicit acknowledgement. Astra review fixes covered future-dated receipt loss, oversized error output, SIGKILL escalation after early direct-child closure and assertion of final delta acknowledgement. Main-agent real-provider verification completed four existing accounts with replay, process restart, continuation and acknowledged incremental rounds, without send/delete/Seen changes or secret/content output. Notification delivery and continuous unattended operation were not asserted. Candidate UID SEARCH memory remains explicitly outside the FETCH/output cap.

Capabilities folded: inbox-scan, unified-inbox and inbox-scan-publication-testing. Public synthetic/verification scope is recorded in `cairn/spec/testing/inbox-scan-publication.md`. No upstream feature PR is created by this change.
