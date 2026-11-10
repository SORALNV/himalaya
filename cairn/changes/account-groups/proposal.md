---
cairn: change
id: account-groups
status: landed
created: 2026-11-10
---

# Group accounts into named categories

## Why

The user keeps several mail accounts and wants them grouped, folder-like, so the work accounts can be handled together: listed as one set in `account list`, and swept by one command in the fork's unified inbox instead of naming each account.

## What

Add one optional `group` string per account (`account.group = "work"`). A group is a plain label with no behaviour of its own: no default, no inheritance, no nesting.

- `account list` gains a `group` column (blank when unset) and a `--group NAME` filter; ungrouped accounts stay visible without the filter and are the only ones shown under `--group` matching... they are shown only when their group matches, `--group` selecting among the values present.
- The fork's `inbox list` and `inbox scan` gain the same `--group NAME`, expanding to every account carrying that group before the per-account loop, so one flag sweeps the work set. Account failure isolation, receipt windows and coverage reporting are unchanged.
- The Web settings UI shows the group as a read-only hint in the account list for now; editing it is out of scope for this change.

No upstream PR: fork-only custom feature per the fork policy.
