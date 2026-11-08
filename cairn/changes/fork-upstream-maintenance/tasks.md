---
cairn: tasks
change: fork-upstream-maintenance
---

- [x] Compare current fork and upstream, identify existing fixes and excluded wizard simplification.
- [x] Human review the integration scope before changing code: the upstream maintenance was selected on 2026-11-08.
- [x] Import selected upstream changes and preserve stronger wizard and credential-helper protections.
- [x] Adapt receipt-aware inbox calls to the pinned libraries: none needed, the imported IMAP changes keep `EXAMINE`, `UID SEARCH` and `UID FETCH` untouched.
- [x] Verify offline tests, feature builds, security/inbox smoke checks, formatting and Clippy.
- [x] Update current specification and validation notes, append a dated log, and mark landed.
- [x] Commit the scoped integration locally and report publication status.
