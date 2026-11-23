---
cairn: change
id: fork-upstream-maintenance-r2
status: landed
created: 2026-11-23
---

# Wave 2 of the upstream maintenance import

## Why

Upstream moved 111 commits past the fork's last import point (`7bfaaa09`, imported 2026-11-08 as 22 cherry-picks). The fork should keep taking the safe part now: dependency and advisory bumps, fixes, docs, tests and small features, so the next security or behavior fix lands close to upstream. The 24 breaking commits in the range (the v3/pre-v3 contract batches, pimdir storage draft-13..15, the search query language, the REPL/MCP core move) are one coherent jump and stay deferred to a wave 3 proposal, applied after this one is verified.

## What

- Branch `integration/upstream-2026-11-23` from `origin/master` (`28b9c0d2`).
- A teammate classifies every commit in `7bfaaa09..upstream/master` into WAVE2_APPLY, WAVE3_DEFER or SKIP (already-covered), in upstream order, with conflict risk against the fork-only surfaces named below. The lead reviews the table before anything is applied.
- A teammate applies the approved WAVE2 list mechanically with `git cherry-pick -x`, aborting on any conflict; the lead resolves the conflicted picks personally, keeping the fork-only code byte-for-byte.
- The full battery runs before (baseline) and after: `cargo test --release`, `cargo fmt --check`, `cargo clippy --release --all-targets -- -D warnings`, the 17/21/23 smoke scripts, the TERM/HUP wizard smoke, the 32 OpenMausBot wrapper tests, the browser fixture (CSS/JS load, MPRG preset, group badges, action bar in flow) and the legacy MPRG account-check probe (transport reaches authentication).
- Fork-only surfaces that must survive untouched: the web wizard security checks (`src/wizard/web/**`), the wizard save-target hardening (`src/wizard/configure.rs`, `src/wizard/secret.rs`), the legacy-mprg profile (`vendor/pimalaya-stream/src/legacy.rs`, its `src/config.rs` validation), the wrapper contract (`src/shared/inbox.rs`, `src/shared/inbox/scan.rs`, its JSON shape and error codes) and the account groups feature.
- Push: `integration/upstream-2026-11-23` and the fast-forwarded fork `master` to origin, per the standing instruction. No upstream PRs.
