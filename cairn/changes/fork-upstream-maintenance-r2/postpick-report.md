cairn: report
id: fork-upstream-maintenance-r2-postpick
created: 2026-11-23

# Wave-2 post-pick battery — result at 9d7c99d5

Same ten-item battery as the baseline, run on the wave-2 result: commit `9d7c99d55d1c9b9e307359f76455126b92f4fd73`, branch `integration/upstream-2026-11-23`, ten picks on top of the baseline point `28b9c0d2`. Tree verified clean before starting (`git status --short` showed only the untracked `cairn/changes/fork-upstream-maintenance-r2/` dir). Nothing was fixed, committed or pushed; the only write is this report.

Environment (unchanged from baseline): macOS 27.2 (26B123), cargo 1.99.1, rustc 1.99.1, Python 3.15.2, node v26.13.0. Baseline for comparison: `baseline-report.md` in this directory (all ten items PASS at `28b9c0d2`).

**Headline: the wave-2 result regresses the battery.** The plain release binary builds and every binary-level smoke still passes, but the crate no longer compiles in test mode: two `#[cfg(test)]`-only call sites were not updated for the signature changes made by the credential-once pick (`b0d499a1`). Items 2, 3, 4 and 8 fail; items 1, 5, 6, 7, 9, 10 pass, identical to baseline.

## Results at a glance

| # | Item | Result | Baseline | Diff from baseline |
|---|------|--------|----------|--------------------|
| 1 | `cargo build --release` | PASS — 1m 58s, exit 0 | PASS — 2m 01s | none (timing only) |
| 2 | `cargo test --release` | **FAIL** — E0061 ×2, test target does not compile, 0 tests ran, exit 101 | PASS — 408 passed, 0 failed, 5 ignored | **REGRESSION** |
| 3 | `cargo fmt --check` | **FAIL** — 1 diff (`src/account/check.rs:593`), exit 1 | PASS — clean | **REGRESSION** |
| 4 | `cargo clippy --release --all-targets -- -D warnings` | **FAIL** — same E0061 ×2, exit 101 | PASS — clean | **REGRESSION** |
| 5 | `scripts/security_smoke.py` | PASS — 17 PASS entries | PASS — 17 | none |
| 6 | `scripts/inbox_smoke.py` | PASS — 21 `: true` | PASS — 21 | none |
| 7 | `scripts/inbox_scan_smoke.py` | PASS — 23 `: true` | PASS — 23 | none |
| 8 | `scripts/web_wizard_signal_smoke.py` | **FAIL** — prerequisite build fails; no wave-2 test binary exists | PASS — SIGTERM/SIGHUP PASS | **REGRESSION (blocked by item 2)** |
| 9 | OpenMausBot readonly wrapper suite | PASS — 32 pass, 0 fail | PASS — 32 pass | none |
| 10 | Legacy MPRG transport probe | PASS — reached auth: `NO Authentication failed.` | PASS — same | none |

## 1. cargo build --release

PASS. Exit 0.

```
   Compiling himalaya v2.2.1 (/Users/sora/Code/himalaya/himalaya)
    Finished `release` profile [optimized] target(s) in 1m 58s
```

Diff from baseline: none (baseline 2m 01s).

## 2. cargo test --release

**FAIL.** Exit 101. The `himalaya` bin test target does not compile, so no suite ran at all — there are zero `^test result` lines and no `Running …` lines in the output. Compile errors, verbatim:

```
error[E0061]: this function takes 2 arguments but 1 argument was supplied
   --> src/account/check.rs:597:17
    |
597 |         assert!(crate::smtp::client::SmtpClient::new(smtp).is_err());
    |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^------ argument #2 of type `&mut pimalaya_config::secret::SecretResolver` is missing
    |
note: associated function defined here
   --> src/smtp/client.rs:41:12
 41 |     pub fn new(config: SmtpConfig, resolver: &mut SecretResolver) -> Result<Self> {
    |            ^^^                     -----------------------------
help: provide the argument
    |
597 |         assert!(crate::smtp::client::SmtpClient::new(smtp, /* &mut SecretResolver */).is_err());
    |                                                          +++++++++++++++++++++++++++

error[E0061]: this method takes 3 arguments but 4 arguments were supplied
   --> src/shared/client.rs:724:24
    |
724 |         assert!(client.send_message(None, &[], raw, false).is_err());
    |                        ^^^^^^^^^^^^       --- unexpected argument #2 of type `&[_; 0]`
    |
note: method defined here
   --> src/shared/client.rs:479:12
479 |     pub fn send_message(
    |            ^^^^^^^^^^^^
help: remove the extra argument
    |
724 -         assert!(client.send_message(None, &[], raw, false).is_err());
724 +         assert!(client.send_message(None, raw, false).is_err());
    |

For more information about this error, use `rustc --explain E0061`.
error: could not compile `himalaya` (bin "himalaya" test) due to 2 previous errors
warning: build failed, waiting for other jobs to finish...
```

Diff from baseline: **regression** — baseline compiled and passed 408 tests (394 unit + 5 ignored; 6 + 6 + 2 integration), 0 failed. Post-pick, no test executes.

Note on the expected ignored-count rise: the wave-2 picks added `#[ignore]`d live provider suites (`tests/google.rs`, `tests/microsoft.rs`, `tests/fastmail.rs`), which should raise the unit suite's ignored count above the baseline 5 — but this cannot be observed because the test target never compiles. No pass/fail/ignored numbers exist to record.

## 3. cargo fmt --check

**FAIL.** Exit 1. Exactly one formatting diff, verbatim:

```
Diff in /Users/sora/Code/himalaya/himalaya/src/account/check.rs:593:
         assert!(connect_imap(&imap, &mut resolver).is_err());
         assert!(connect_smtp(&smtp, &mut resolver).is_err());
         assert!(!marker.exists());
-        assert!(crate::imap::client::ImapClient::new(imap, &mut pimalaya_config::secret::SecretResolver::new()).is_err());
+        assert!(
+            crate::imap::client::ImapClient::new(
+                imap,
+                &mut pimalaya_config::secret::SecretResolver::new()
+            )
+            .is_err()
+        );
         assert!(crate::smtp::client::SmtpClient::new(smtp).is_err());
         assert!(!marker.exists());
     }
```

Diff from baseline: **regression** — baseline was clean.

## 4. cargo clippy --release --all-targets -- -D warnings

**FAIL.** Exit 101. `--all-targets` compiles the test profile, so clippy stops on the same two E0061 compile errors as item 2 (no lint findings are reached):

```
error[E0061]: this function takes 2 arguments but 1 argument was supplied
error[E0061]: this method takes 3 arguments but 4 arguments were supplied
error: could not compile `himalaya` (bin "himalaya" test) due to 2 previous errors
warning: build failed, waiting for other jobs to finish...
```

Diff from baseline: **regression** — baseline was clean (Finished in 8.54s, exit 0).

## 5. python3 scripts/security_smoke.py --binary target/release/himalaya

PASS. Exit 0. Exactly **17 PASS entries**, e.g. tail:

```
  "wizard_rejects_replaced_config": "PASS",
  "wizard_rejects_fifo_substituted_during_prompt": "PASS",
  "wizard_rejects_explicit_acl": "PASS",
  "wizard_rejects_inherited_acl": "PASS"
}
```

Diff from baseline: none (17 = 17).

## 6. python3 scripts/inbox_smoke.py --binary target/release/himalaya

PASS. Exit 0. Exactly **21 `": true"` entries**, e.g. tail:

```
  "network_truncation_explicit": true,
  "summary_read_only": true
}
```

Diff from baseline: none (21 = 21).

## 7. python3 scripts/inbox_scan_smoke.py --binary target/release/himalaya

PASS. Exit 0. Exactly **23 `": true"` entries**, e.g. tail:

```
  "server_internal_date_search": true,
  "read_only_protocol": true
}
```

Diff from baseline: none (23 = 23).

## 8. python3 scripts/web_wizard_signal_smoke.py --test-binary <main test binary>

**FAIL.** The prerequisite `cargo test --release --no-run` exits 101 with the same two E0061 errors and prints no `Executable …` lines — no wave-2 test binary was produced:

```
error[E0061]: this function takes 2 arguments but 1 argument was supplied
error[E0061]: this method takes 3 arguments but 4 arguments were supplied
error: could not compile `himalaya` (bin "himalaya" test) due to 2 previous errors
EXIT: 101
```

The smoke itself was therefore not run: there is no wave-2 unittests binary to point it at. Running it against the stale baseline test binary (`target/release/deps/himalaya-67e53013c95d6d1e`, built from `28b9c0d2`) would exercise baseline code, not wave-2, so it would measure nothing about this HEAD. (The binary-selection rule from the baseline report still applies: use cargo's `Executable unittests src/main.rs` line, not the largest-file heuristic — here there is no line to use.)

Diff from baseline: **regression, blocked by item 2** — baseline produced `SIGTERM: PASS` / `SIGHUP: PASS`.

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
ℹ duration_ms 6769.237666
```

Diff from baseline: none (32 pass, 0 fail both rounds).

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

Exactly the expected PASS condition, identical to baseline: STARTTLS transport to `mail.mprg.cs.chubu.ac.jp:143` and the `legacy-mprg` sha256 pin accepted (no TLS error, no pin error); the server rejected the deliberately nonexistent user `zz-probe-nonexistent` with the dummy secret.

Diff from baseline: none.

## Diagnosis (observation only — nothing was fixed)

The four failures share one root: the credential-once pick (`b0d499a1`, "fix: run a credential command shared by IMAP and SMTP once") changed two production signatures — `SmtpClient::new(config, resolver)` (`src/smtp/client.rs:41`) and `send_message` down to 3 parameters (`src/shared/client.rs:479`) — but two call sites that live only in test-gated modules were not updated:

- `src/account/check.rs:597`, inside `#[cfg(all(test, feature = "imap", feature = "smtp", feature = "wizard"))] mod legacy_tests` — `SmtpClient::new(smtp)` missing the new `&mut SecretResolver` argument (its neighbour on line 596, `ImapClient::new(imap, …)`, *was* updated — as was line 594's `connect_smtp(&smtp, &mut resolver)` — which suggests the stale line was simply missed).
- `src/shared/client.rs:724`, inside the `#[cfg(all(test, feature = "imap", feature = "smtp"))]` test module — `send_message(None, &[], raw, false)` still passes the removed envelope argument `&[]`.

Because both sites are `#[cfg(...test...)]`-gated, the plain release build (item 1) and every binary-level smoke (items 5–7, 9, 10) are unaffected; only the test-compiling commands fail (items 2, 4, 8). The item-3 fmt diff is a long unformatted `ImapClient::new` line at `src/account/check.rs:593` in the same test module, landing with the same pick. Fixing requires updating those two test call sites (plus one rustfmt pass over the same module) — deliberately not done here.
