---
cairn: tasks
change: fork-upstream-maintenance-r2
---

- [x] Classify all 111 commits in `7bfaaa09..upstream/master` into WAVE2_APPLY / WAVE3_DEFER / SKIP with conflict risk (upstream-analyst).
- [x] Run the baseline battery on the branch point `28b9c0d2` (wave2-baseline).
- [x] Lead reviews the classification table and fixes the wave-2 list.
- [x] Apply wave-2 picks in upstream order (`git cherry-pick -x`, abort-on-conflict for the mechanical pass; lead resolves conflicts personally).
- [x] Full battery on the result: cargo test/fmt/clippy, 17+21+23 smokes, TERM/HUP, 32 wrapper tests, browser fixture, MPRG legacy probe.
- [x] Finalize delta.md from the landed set, fold spec, write the dated log, set `status: landed`.
- [x] Push `integration/upstream-2026-11-23` and the fast-forwarded fork `master` to origin (no upstream PRs), and leave the wave-3 scope documented.
