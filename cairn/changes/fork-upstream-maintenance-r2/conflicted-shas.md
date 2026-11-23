# Wave-2 conflicted / aborted SHAs
# Branch: integration/upstream-2026-11-23 (base 28b9c0d2)
# Mechanical pass: `git cherry-pick -x <sha>`; on ANY conflict, the unmerged
# file list was captured with `git diff --name-only --diff-filter=U` before
# `git cherry-pick --abort`. One sha per line, in pick order.

a57fff0094d1bef8230ba217e0a9df029dbf9d1f
- reason: cherry-pick conflict (content)
- conflict files (git diff --name-only --diff-filter=U):
  - Cargo.lock
  - Cargo.toml
  - cairn/spec/packaging.md

7146189073e4db6a4d3171cb176b7ab5fef2be3c
- reason: cherry-pick conflict (content)
- conflict files (git diff --name-only --diff-filter=U):
  - src/shared/client.rs

de1bd25f55e2cfe1f6c7abf5b10542e8510fa9de
- reason: cherry-pick conflict (content)
- conflict files (git diff --name-only --diff-filter=U):
  - CHANGELOG.md
  - src/shared/message/cli.rs

202eb72daae970ddfb31617c2cecb60de6a6ec5a
- reason: cherry-pick conflict (content, plus modify/delete on the
  cli-alignment-leftovers change files deleted in HEAD)
- conflict files (git diff --name-only --diff-filter=U):
  - CHANGELOG.md
  - cairn/changes/cli-alignment-leftovers/proposal.md
  - cairn/changes/cli-alignment-leftovers/tasks.md
  - src/imap/client.rs
  - src/shared/client.rs
  - src/smtp/client.rs

7a9d76dcc99131d71909959a73a7d088074c4504
- reason: cherry-pick conflict (content)
- conflict files (git diff --name-only --diff-filter=U):
  - .github/workflows/tests.yml

## Summary (pass of 2026-11-23/24, mechanical, abort-on-conflict)

- Applied cleanly (5): 08909c6415641963a258cfb44cab341e593bb883 -> e308bc4c,
  ac40305ff9d83ab29278dae2c13fbcdf8bcd3c41 -> 837af634,
  4f7b560c76c951aeddcb11ed8b5918a707339797 -> fcd9b03c,
  a196cc704fb32bf72724c16731e5e33423a33c19 -> 888bb400,
  edf58f38a1f08220ecfb5a476e2b4ea7d2b0500c -> bb4d7298
  (all with `git cherry-pick -x`, in the approved order).
- Conflicted and aborted (5): listed above, in pick order.
- No commit was resolved by hand, amended, reordered, or skipped silently;
  no cargo/test run, no push, no branch switch. The only write outside the
  picks is this file.

