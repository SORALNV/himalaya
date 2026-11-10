---
cairn: log
change: account-groups
date: 2026-11-10
---

# Accounts group into named categories

The user asked for folder-like grouping of mail accounts, to handle the work accounts as one set. One optional plain label per account (`account.group = "work"`, one group, no nesting) answers it.

`account list` shows the label in a new GROUP column, blank when unset, and takes `--group NAME` to list only the accounts carrying it, erroring on a group no account carries. `inbox list` takes the same flag and expands it to the matching accounts before the per-account loop, so one command sweeps the work set; naming both `--group` and `-a` is refused. The per-account failure isolation, receipt windows and coverage reporting are untouched. `inbox scan` stays single-account on purpose: its cursor binds account, mailbox and window, and the OpenMausBot wrapper is what expands a set of accounts into per-account scans with acknowledgement.

The Web settings sidebar shows the group as a small badge read-only, next to each account name; editing it from the browser is out of scope for this change. The status payload gained a `groups` map of the non-null labels, extracted beside the existing account names without resolving secrets.

Verified on the fork: 333 unit and 6 integration tests, formatting, clippy with warnings denied, the 17 security, 21 inbox and 23 inbox-scan smoke checks, and a real-Chrome check of the sidebar badge and of both filters against a four-account fixture (two `work`, one `private`, one ungrouped), JSON shapes included.

Capabilities moved: **config** (Multi-account schema: the group label, the `account list` column and filter added), **unified-inbox** (Account identity and routing: `--group` sweep added). Fork-only; no upstream PR.
