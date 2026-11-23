---
cairn: log
change: fork-upstream-maintenance-r2
date: 2026-11-23
---

# Wave 2 of the upstream import

Upstream moved 111 commits past the fork's last import point (`7bfaaa09`, imported 2026-11-08). A teammate classified every one against the fork's protected surfaces with merge dry-runs; the lead reviewed the table and wave 2 took the ten that stand alone: five docs/cairn/chore picks applied mechanically, five needed hand resolution onto the fork's diverged code.

What landed, in upstream order: the ignored live provider suites for Google, Microsoft and Fastmail (fork-adapted: only the io-oauth dev-dependency joined, the fork's crate pins kept, only the pick's own packaging requirement folded), the em-dash and doc-comment pass over the testing reports and client docs, the message-subcommand help regrouped by purpose (fork attribute style kept, upstream's capability-help system not present), the credential-command fix — one EmailClient-held resolver threaded into IMAP and SMTP so a `message send` with `save-copy` runs the password command once (#813), ported onto the fork's lazy SmtpTransport and fork-only scan path — three cairn proposal docs, the landed-change archive rename, the weekly live-suite CI triggers and the unmaintained-chrono advisory ignore. The CHANGELOG gained the fork-adapted entries and lost a duplicated ctutils line left by the first import.

The other 101 commits defer to a wave 3: the v3/pre-v3 contract batches, `print every output as data`, pimdir storage draft-13..15 with its io-pimdir pins, the search query language, the REPL/MCP core, `tls.client`, and everything touching the mcp/repl/session surfaces the fork deleted or riding them. Upstream also dropped the configuration wizard in-range; wave 3 must keep the fork's web wizard while taking the rest. pimalaya-stream 0.3.2's io-proxy host-validation fix is flagged for the vendored copy in wave 3.

Verified before and after: the ten-item battery ran on the branch point (394 unit + 14 integration passed) and on the result — 395 unit (+1, the fix's own test) + 14 integration, 0 failed, live suites ignored; fmt and clippy clean; security 17, inbox 21, inbox-scan 23 smokes; SIGTERM/SIGHUP teardown; the OpenMausBot wrapper suite 32/32; the legacy MPRG account-check probe reaching authentication (TLS 1.0 transport and pin unchanged); and a real-Chrome pass of the web wizard (preset fills, consent stays unchecked, pin absent from visible text, group badges render, action bar in flow, overlap 0). Two test-gated call sites the initial port missed were caught by the post-pick battery and folded into the pick.

Capabilities moved: **config** (the credential-command-once requirement carries the send-saving-copy scenario), **packaging** (the live provider suites requirement). No upstream PRs; fork-only merge branch pushed as `integration/upstream-2026-11-23` and fast-forwarded the fork's `master`.
