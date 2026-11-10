---
cairn: tasks
change: account-groups
---

- [x] Human reviews the scope (CLI-first, one group per account).
- [x] Add `account.group` to the config model with serde round-trip.
- [x] `account list`: group column + `--group` filter.
- [x] `inbox list`: `--group` expands to the matching accounts. `inbox scan` stays single-account: its cursor binds the account, and the wrapper expands sets per account.
- [x] Web UI: show the group in the account list (read-only).
- [x] Tests: config round-trip, list filter, inbox expansion, failure isolation unchanged.
- [x] Fold spec, dated log, mark landed.
