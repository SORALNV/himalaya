cairn: report
id: fork-upstream-maintenance-r2-baseline
created: 2026-11-23

# Wave-2 baseline battery — branch point before cherry-picks

Baseline recorded on commit `28b9c0d2cf232ef662d82a7ec4d41690a7352477`, branch `integration/upstream-2026-11-23` (= fork master content at the branch point before any wave-2 cherry-picks). After the wave-2 picks, re-run this identical battery and compare against the numbers below.

Environment: macOS 27.2 (26B123), cargo 1.99.1 (5f94df478 2026-08-27, Homebrew), rustc 1.99.1 (58cf8f9c3 2026-10-13), Python 3.15.2, node v26.13.0 (`/opt/homebrew/bin/node`). Crate: himalaya v2.2.1. No code was modified, nothing committed or pushed; the only write is this report.

## Results at a glance

| # | Item | Result |
|---|------|--------|
| 1 | `cargo build --release` | PASS |
| 2 | `cargo test --release` | PASS — 0 failed (408 passed, 5 ignored) |
| 3 | `cargo fmt --check` | PASS — clean |
| 4 | `cargo clippy --release --all-targets -- -D warnings` | PASS — clean |
| 5 | `scripts/security_smoke.py` | PASS — 17 PASS entries |
| 6 | `scripts/inbox_smoke.py` | PASS — 21 `: true` |
| 7 | `scripts/inbox_scan_smoke.py` | PASS — 23 `: true` |
| 8 | `scripts/web_wizard_signal_smoke.py` | PASS — SIGTERM: PASS, SIGHUP: PASS |
| 9 | OpenMausBot readonly wrapper suite | PASS — 32 pass, 0 fail |
| 10 | Legacy MPRG transport probe | PASS — reached authentication (`NO Authentication failed.`) |

## 1. cargo build --release

PASS. Exit 0.

```
   Compiling himalaya v2.2.1 (/Users/sora/Code/himalaya/himalaya)
    Finished `release` profile [optimized] target(s) in 2m 01s
```

## 2. cargo test --release

PASS. Exit 0. Every `^test result` line (four suites):

```
     Running unittests src/main.rs (target/release/deps/himalaya-67e53013c95d6d1e)
running 399 tests
test result: ok. 394 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 4.92s
     Running tests/message_forward.rs (target/release/deps/message_forward-1c411543306217b0)
running 6 tests
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.45s
     Running tests/message_parse.rs (target/release/deps/message_parse-e7c55b9f569e69c5)
running 6 tests
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s
     Running tests/message_reply.rs (target/release/deps/message_reply-479a01dc5b8e0fd8)
running 2 tests
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

Totals: **408 passed, 0 failed, 5 ignored** (394 unit + 6 + 6 + 2 integration across `message_forward`, `message_parse`, `message_reply`).

Note vs expectation: the task said "roughly 333 passed + 6 integration passed". The actual baseline at `28b9c0d2` is 394 unit passed and 14 integration passed — higher than the rough guide, with 0 failed everywhere. Recorded verbatim; the post-pick run should be compared against these actual numbers, not the rough guide.

## 3. cargo fmt --check

PASS. Exit 0, no output (no formatting diffs).

## 4. cargo clippy --release --all-targets -- -D warnings

PASS. Exit 0, no warnings emitted.

```
   Compiling himalaya v2.2.1 (/Users/sora/Code/himalaya/himalaya)
    Finished `release` profile [optimized] target(s) in 8.54s
```

## 5. python3 scripts/security_smoke.py --binary target/release/himalaya

PASS. Exit 0. Exactly **17 PASS entries**, e.g. tail:

```
  "wizard_rejects_replaced_config": "PASS",
  "wizard_rejects_fifo_substituted_during_prompt": "PASS",
  "wizard_rejects_explicit_acl": "PASS",
  "wizard_rejects_inherited_acl": "PASS"
}
```

## 6. python3 scripts/inbox_smoke.py --binary target/release/himalaya

PASS. Exit 0. Exactly **21 `": true"` entries**, e.g. tail:

```
  "network_truncation_explicit": true,
  "summary_read_only": true
}
```

## 7. python3 scripts/inbox_scan_smoke.py --binary target/release/himalaya

PASS. Exit 0. Exactly **23 `": true"` entries**, e.g. tail:

```
  "server_internal_date_search": true,
  "read_only_protocol": true
}
```

## 8. python3 scripts/web_wizard_signal_smoke.py --test-binary <main test binary>

PASS. Exit 0, expected pair:

```
SIGTERM: PASS
SIGHUP: PASS
```

Preparation: `cargo test --release --no-run` first (exit 0):

```
    Finished `release` profile [optimized] target(s) in 0.28s
  Executable unittests src/main.rs (target/release/deps/himalaya-67e53013c95d6d1e)
  Executable tests/message_forward.rs (target/release/deps/message_forward-1c411543306217b0)
  Executable tests/message_parse.rs (target/release/deps/message_parse-e7c55b9f569e69c5)
  Executable tests/message_reply.rs (target/release/deps/message_reply-479a01dc5b8e0fd8)
```

Test binary used: `target/release/deps/himalaya-67e53013c95d6d1e` (12,072,176 bytes — the binary cargo itself reports as `Executable unittests src/main.rs`).

Caveat for the post-pick run: the "largest `himalaya-<hash>` file under `target/release/deps/`" heuristic does **not** pick the test binary in this tree. The largest such file is `himalaya-1610fd509c8282a7` (13,128,816 bytes), which is byte-identical to the plain `target/release/himalaya` release binary artifact that `cargo build --release` leaves in `deps/` — it has no test harness and would fail the smoke. Use the `himalaya-<hash>` path that `cargo test --release --no-run` prints on its `Executable unittests src/main.rs` line.

## 9. OpenMausBot wrapper suite

PASS. Exit 0. Run as:

```
cd /Users/sora/Code/OpenMausBot/OpenMausBotPersonal
HIMALAYA_BIN=/Users/sora/Code/himalaya/himalaya/target/release/himalaya /opt/homebrew/bin/node --test scripts/himalaya-readonly.node-test.mjs
```

node `--test` summary:

```
ℹ tests 32
ℹ suites 0
ℹ pass 32
ℹ fail 0
ℹ cancelled 0
ℹ skipped 0
ℹ todo 0
ℹ duration_ms 6754.368042
```

**32 pass, 0 fail** — matches expectation.

## 10. Legacy MPRG transport probe

PASS. Command:

```
/Users/sora/Code/himalaya/himalaya/target/release/himalaya account check -a mprg-probe -c /tmp/himalaya-mprg-test/probe.toml
```

Full output (exit 0):

```
Account: mprg-probe
  imap: FAIL (IMAP AUTHENTICATE PLAIN failed: NO Authentication failed.)
```

This is exactly the expected PASS condition: the probe reached IMAP `AUTHENTICATE PLAIN` against `mail.mprg.cs.chubu.ac.jp:143` over STARTTLS with the `legacy-mprg` sha256 pin accepted (no TLS error, no pin error), and the server rejected the deliberately nonexistent user `zz-probe-nonexistent` with the dummy secret. Transport and pinning at the branch point are OK.
