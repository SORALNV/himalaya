---
cairn: delta
change: account-groups
---

## ADDED Requirements

### Requirement: Accounts carry one optional group
An account SHALL declare at most one group, a plain label (`account.group = "work"`), defaulting to none. The group SHALL have no behaviour beyond selection and display: no defaults, no inheritance between accounts, no nesting.

### Requirement: Account listing groups and filters
`account list` SHALL show each account's group in its own column, blank when unset, and SHALL accept `--group NAME` to list only the accounts carrying that group. Without the flag every account SHALL be listed as before.

### Requirement: Unified inbox selects accounts by group
`inbox list` and `inbox scan` SHALL accept `--group NAME` and expand it to every configured account carrying that group before scanning, each account keeping its own failure isolation, receipt window and coverage reporting. Naming both `--group` and `--account` SHALL be an error.
