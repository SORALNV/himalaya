# Wave-2 upstream classification (r2)

Range: `7bfaaa09..upstream/master` (111 commits, upstream order below = `git rev-list --reverse`).
Base `7bfaaa09` = "feat(pimdir)!: follow storage-draft-09 (io-pimdir 5c18a02), read search dates in the local zone" (2026-11-06). The range is upstream's v3 overhaul: the wizard drop, the plain-text/records output rework, the pimalaya-cli shared core adoption, the pimdir storage drafts 10–15, the contract/audit batches, and the shared→unified split.

Fork-protected surfaces honored throughout: `src/wizard/web/**`, `src/wizard/{configure,secret}.rs`, `vendor/pimalaya-stream/src/legacy.rs` + the legacy-mprg validation block in `src/config.rs`, `src/shared/inbox.rs` + `src/shared/inbox/scan.rs` (OpenMausBot wrapper JSON/error-code contract), the account-groups feature (`groups` in `src/config.rs`, column+`--group` in `src/account/list.rs`, `--group` in `src/shared/inbox.rs`), and anything changing CLI JSON output/error wording the wrapper parses.

Evidence used beyond the rubric:

- **Skip proof**: the 21 importable cherry-pick sources on `origin/master` are all ancestors of the base (outside the range); `git patch-id --stable` over the range finds zero overlap with them, and no range subject matches any carried subject. One fork trailer (`ef89cbd3`, "fix smtp port (#94)") points at a commit upstream's re-sign removed — so "22 imported" is 21 importable + 1 dead reference. **skip-shas is empty.**
- **Conflict prediction**: every WAVE2 candidate was dry-run with `git merge-tree --write-tree --merge-base=<parent> <fork-tree> <commit>` in upstream order with abort-on-conflict semantics (exactly what task-3 will do), without touching the working tree. Predicted conflicts are stated in the reason/risk columns.
- **Fork reality that reshaped the calls** (not in the rubric): the fork *deleted whole surfaces upstream still reworks in-range* — `src/mcp.rs`, `src/repl.rs`, `src/session.rs`, `src/capability.rs`, `src/account/capability.rs`, `src/imap/outcome.rs`, `src/email/vacation.rs`, most of pimdir's later modules (`search.rs`, `index.rs`, `tag.rs`, `thread.rs`, `attachment/**`, `flag/**`, `count.rs`), `shared/vacation/**`, `shared/mailbox/{create,delete,rename}.rs`, `shared/receipt.rs`, and most integration tests (`tests/mcp.rs`, `tests/repl.rs`, `tests/docs.rs`, `tests/capability.rs`, `tests/mailbox.rs`, …); the fork pins the whole pimalaya family via `[patch.crates-io]` (vendored `pimalaya-stream`, SORALNV `pimalaya-config`, git-pinned `pimalaya-cli` 0.2.6-line, git-pinned `io-imap`/`io-pimdir`); and the fork pruned a large set of upstream cairn dirs (`v2-release`, `v3-release`, `cli-alignment-leftovers`, `machine-output-contract`, `mcp-server`, `one-trust-policy-per-account`, `repl-session`, `the-outputs-become-data`, …).

Conflict-risk legend: **HIGH** = touches a protected surface, the wrapper's JSON/error/exit-code contract, or a fork-deleted/fork-policy surface (pick is dead or dangerous); **MEDIUM** = touches heavily fork-diverged code without direct contract impact (mechanical pick likely aborts, hand-merge feasible); **LOW** = docs/new files/Cargo-or-CI-only, no fork code at risk.

| sha | subject | category | files (short) | recommendation | reason | conflict-risk |
|---|---|---|---|---|---|---|
| e94de7bd | feat(mcp): attach files given inline to the composers (#807) | feat | src/mcp.rs(+531), src/shared/message/{builder,compose,forward,reply}, error.rs, tests/mcp.rs | WAVE3_DEFER | Targets the MCP surface the fork removed entirely (src/mcp.rs + tests/mcp.rs deleted); pick dies modify/delete and the feature is moot until the wave-3 MCP-core decision (#7 replaces the whole surface anyway) | HIGH |
| 6284d2fb | feat(mcp): read an attachment's content without touching disk (#805) (#808) | feat | src/mcp.rs(+319), src/shared/attachment/download.rs, error.rs, tests/mcp.rs | WAVE3_DEFER | Same: rides the fork-deleted MCP surface and builds on e94de7bd's mcp groundwork | HIGH |
| 682edb40 | feat(wizard): pick the best option, review it, ask the password once | feat | src/wizard/** redesign (+3.5k: plan/signin/server/discover), wizard/configure.rs, wizard/secret.rs, Cargo.toml | WAVE3_DEFER | Full wizard redesign touching protected src/wizard/configure.rs + secret.rs (save-target hardening must survive byte-for-byte); introduces the pimalaya-cli pin 42134fd2 pairs with; 82207e0e drops the wizard later in the same wave anyway | HIGH |
| 42134fd2 | build: pin pimalaya-cli f976e40 | build | Cargo.toml/lock (patch-section pin) | WAVE3_DEFER | Pin only the deferred wizard rework (682edb40) needs; the fork pins pimalaya-cli 0.2.6→git e4ea7f40 in its own `[patch.crates-io]` block — hunk cannot land | HIGH |
| 82207e0e | feat!: drop the configuration wizard, print text output as plain columns and fields | feat! | deletes src/wizard/**, all text-output printers, Cargo, docs | WAVE3_DEFER | Breaking (rubric); deletes the tree the fork's protected web wizard and hardened configure/secret live beside, and reworks all text output | HIGH |
| 977c3599 | feat!: welcome a first run, take one config path per -c | feat! | src/cli.rs, src/config.rs, src/account/list.rs, src/main.rs, tests/first_run.rs | WAVE3_DEFER | Breaking (rubric); changes `-c` semantics the wrapper invokes, touches protected config.rs (groups, mprg validation) and account/list.rs (groups column) | HIGH |
| 8b118208 | feat(repl,mcp): run on pimalaya-cli's shared REPL and MCP core | feat | rewrites src/{mcp,repl,session}.rs, cli.rs, main.rs, tests | WAVE3_DEFER | The REPL/MCP core move (rubric); its targets src/mcp.rs, src/repl.rs, src/session.rs are all deleted in the fork | HIGH |
| aa425b48 | feat(config): reshape the sample per backend, test and ship it | feat | config.sample.toml rewrite, src/config.rs tests(+60), install.sh, default.nix | WAVE3_DEFER | Explicitly paired with the deferred first-run welcome (977c3599: "the first-run welcome copies it"); rewrites the fork-diverged sample and adds a sample-shape test (exact `[accounts.example]` header + section list) the fork's diverged sample (groups, mprg profiles) would fail | HIGH |
| ddcbaa49 | fix(repl,mcp): name the binary in session help, use mcp::text | fix | src/mcp.rs, cairn/spec/repl.md, tests/repl.rs | WAVE3_DEFER | Direct follow-up to the deferred core move (uses its mcp::text; fixes fork-deleted files) | HIGH |
| 1bec8bdc | build!: trim dependencies to the features they serve | build! | Cargo.toml/lock, deny.toml, src/{config,mcp,imap,jmap}, new src/shared/base64.rs | WAVE3_DEFER | Breaking build rework (rubric); the platform 25d3ae38/53c17c98 build on; rewrites the manifest against the fork's pin-and-vendor strategy | HIGH |
| a57fff00 | test: add live provider suites for Google, Microsoft and Fastmail | test | new tests/{common,google,microsoft,fastmail}.rs, CI, Cargo dev-deps | WAVE2_APPLY | New black-box live suites, `#[ignore]`d (never run in the battery) and feature-gated; simulated pick conflicts only in Cargo.toml/Cargo.lock (io-oauth dev-dep vs the fork pin block) + cairn/spec/packaging.md — hand-add the dev-dep from conflicted-shas.md | MEDIUM |
| 25d3ae38 | refactor: gate code by the features using it, every feature set clippy-clean | refactor | 30+ src files (config.rs +117, email/search reorg, mcp.rs) | WAVE3_DEFER | Runs over the post-#5/#10 tree: simulated conflict on 12 files incl. fork-deleted src/mcp.rs and config.rs near the protected groups block; its per-feature gating pairs with the deferred 1bec8bdc trim and 53c17c98 CI | HIGH |
| 59798cfa | docs(cairn): close stale tasks | docs | 15 cairn changes/*/tasks.md edits | WAVE3_DEFER | Edits cairn dirs the fork pruned (cli-alignment-leftovers, machine-output-contract, mcp-server, one-trust-policy-per-account, repl-session, the-outputs-become-data, v2/v3-release) — modify/delete aborts | MEDIUM |
| 53c17c98 | ci: run clippy over every feature set, test a build without SMTP | ci | .github/workflows/tests.yml | WAVE3_DEFER | CI exists to test 25d3ae38's feature gating (deferred); red-on-arrival without it | LOW |
| ba07a582 | docs: complete the v3 migration guide, net the changelog, drop table wording | docs | CHANGELOG/MIGRATION/README/CONTRIBUTING, cairn specs, src/main.rs doc lines | WAVE3_DEFER | Completes the migration guide for the deferred v3 state (presumes #5+ output/wizard); simulated conflict on fork-diverged root docs + main.rs | MEDIUM |
| 71461890 | docs: drop em dashes from the testing reports, wrap long doc comments | docs | cairn/spec/testing/*, doc-comment wraps in src/{mbox/*,shared/client.rs} | WAVE2_APPLY | Pure prose/doc-comment rework; simulated: everything auto-merges except one doc-comment hunk in the fork-rewritten src/shared/client.rs (pick aborts there; trivial hand-wrap from the record) | MEDIUM |
| 6226c839 | build: pin pimalaya-cli and pimalaya-config by git rev | build | Cargo.toml/lock (patch-section pins) | WAVE3_DEFER | Pins for the deferred core-move/config-split line; the fork pins pimalaya-config to its SORALNV fork — hunk cannot land | HIGH |
| c9eab51b | test(mcp): gate the account helper on smtp like its one caller | test | tests/mcp.rs (+1) | WAVE3_DEFER | tests/mcp.rs is deleted in the fork; the gate aligns the test with 25d3ae38's deferred gating | HIGH |
| 6ce2d12e | build: take every pimalaya crate from crates.io, dropping the git pins | build | Cargo.toml/lock | WAVE3_DEFER | Drops the git-pin strategy the fork depends on ([patch.crates-io] vendored stream + pinned family); serves the deferred v3 kit line | HIGH |
| d619b9e0 | feat(config)!: fix the flag glyphs, ignore the envelope.list.table block | feat! | src/config.rs(45), src/account/context.rs, shared/envelope/list.rs | WAVE3_DEFER | Breaking (rubric); config.rs protected blocks + envelope list output contract | HIGH |
| 16673d11 | docs(cairn): track the v3 screenshot in the release tasks | docs | cairn/changes/v3-release/tasks.md (+1) | WAVE3_DEFER | The fork pruned cairn/changes/v3-release — simulated modify/delete abort | LOW |
| 927950e2 | feat!: print wide lists as records, the way git log prints commits | feat! | 33 files: every list printer (gmail/jmap/msgraph/pimdir/shared) | WAVE3_DEFER | Breaking list-output rework (rubric) | HIGH |
| a8ccc8f8 | feat!: hang envelopes and threads as trees, drop relative dates | feat! | imap/envelope/thread.rs, pimdir/thread.rs, shared/envelope/list.rs | WAVE3_DEFER | Breaking envelope/thread output rework (rubric) | HIGH |
| ca792b96 | feat!: print messages one line each, with a header and a footer | feat! | shared/envelope/list.rs(+313), json_schema.rs | WAVE3_DEFER | Breaking output rework; json_schema changes the wrapper's schema surface | HIGH |
| b21c2d7f | docs: rename the shared API to the unified API | docs | 60 files: CHANGELOG/specs/testing reports + doc comments across src | WAVE3_DEFER | v3-terminology rename; simulated conflict on fork-pruned v3-release/proposal + fork-diverged root docs; the src comment renames presume deferred state | MEDIUM |
| c4164233 | feat!: print every list as envelope list does | feat! | every list command (gmail/jmap/imap/msgraph/shared) | WAVE3_DEFER | Breaking list-output unification (rubric) | HIGH |
| de1bd25f | docs: group the message subcommands and say what each does | docs | src/shared/message/{cli,compose,forward,reply} help text, README, cairn | WAVE2_APPLY | clap help/about regrouping (non-breaking, no JSON/error impact); simulated: only CHANGELOG.md + fork-diverged src/shared/message/cli.rs conflict — hand-merge the help texts from the record | MEDIUM |
| a9da91bb | fix!: fix what the live test campaign found across every backend | fix! | every backend (gmail/imap/jmap/msgraph/maildir/mbox), search parsers | WAVE3_DEFER | Breaking campaign-fix batch (rubric); rewrites the backends later range work (f33316ac, c1e85ebd…) anchors on | HIGH |
| 5e26640a | docs: fold the live test campaign into the spec | docs | CHANGELOG(+99), cairn spec/changes/log adds | WAVE3_DEFER | Spec prose describing the deferred a9da91bb campaign's outcomes; conflicts on fork-diverged CHANGELOG/MIGRATION | LOW |
| eaf90100 | feat!: drop the m2dir backend | feat! | deletes src/m2dir/**, config.rs(46), cli, CI | WAVE3_DEFER | Breaking (rubric); the fork kept m2dir (fork-diverged modules) — delete-vs-diverged aborts; config.rs protected blocks | HIGH |
| c0dd1a77 | fix!: fix what the local stores campaign found in Maildir and mbox | fix! | maildir/mbox rework, shared/client.rs, error.rs(+26) | WAVE3_DEFER | Breaking stores-campaign batch (rubric); error.rs changes contract wording; 49530ebd/a5623dca anchor on its is_root/client changes | HIGH |
| 4c746260 | docs: fold the local stores campaign into the spec | docs | CHANGELOG, cairn adds, spec/testing/maildir* | WAVE3_DEFER | Documents the deferred c0dd1a77 campaign's outcomes | LOW |
| 46e8b4ea | build: bump io-pimdir to 0.11 | build | Cargo.toml/lock | WAVE3_DEFER | io-pimdir pin paired with dda23f28's draft-10 follow (rubric); the fork pins io-pimdir at its own older git rev — a lone bump changes storage semantics under draft-09 code | HIGH |
| dda23f28 | feat(pimdir)!: follow pimdir draft-10 on io-pimdir 0.11 | feat! | src/pimdir/{backend,index,queue/*}(+663) | WAVE3_DEFER | Breaking storage-draft bump (rubric); the fork's pimdir is trimmed/diverged (index/search/tag deleted) | MEDIUM |
| 963a847b | feat!: a refused command line exits 2, partial outcomes from 3 | feat! | src/main.rs, error flows, tests/exit.rs | WAVE3_DEFER | Breaking exit-code contract the wrapper sees (rubric "!") | HIGH |
| cabcf26b | feat(pimdir)!: follow pimdir draft-11 on io-pimdir cd33f0e | feat! | src/pimdir/{backend,index,search} | WAVE3_DEFER | Breaking draft bump (rubric) | MEDIUM |
| 7518aee3 | build: take io-pimdir 0.12 from crates.io | build | Cargo + src/pimdir/backend.rs(17) | WAVE3_DEFER | The crates release of cabcf26b's draft-11 pin; its code hunk rides the deferred draft | MEDIUM |
| f33316ac | build: adopt the io-* releases of 2026-11-13 | build | Cargo + imap/mbox/jmap/msgraph backends, submission.rs | WAVE3_DEFER | Family release adoption with backend code changes anchored in a9da91bb's rewrites (deferred); the fork pins io-imap for the mprg transport | HIGH |
| 8a4b9b8d | build: update pimalaya-stream to 0.3.2 | build | Cargo.lock only | WAVE3_DEFER | Lock-only bump of the crate the fork vendors (0.3.x + legacy.rs): a no-op under `[patch.crates-io]`, and the lock hunk hits the vendor pin entry. NOTE for wave 3: 0.3.2 adopts io-proxy 0.2 (proxied-target host validation, bracketed IPv6) — the vendored copy still pins io-proxy 0.1; port that fix into vendor/pimalaya-stream | MEDIUM |
| 26d9aa2b | build: update pimalaya-stream to 0.4 and the io-* crates | build | Cargo + src/config.rs (TlsServer conversion) | WAVE3_DEFER | stream 0.4 breaks the fork's vendored 0.3.x: the `legacy-mprg` feature needs `pimalaya-stream/legacy-pinned`, which upstream 0.4 does not carry — the patch stops matching and the build fails; the config.rs hunk rides the TlsConfig conversion the fork's mprg profile depends on | HIGH |
| 202eb72d | fix: run a credential command shared by IMAP and SMTP once | fix | src/shared/client.rs(+92), src/imap/client.rs, src/smtp/client.rs | WAVE2_APPLY | Real #813 fix (a `message send` with `save-copy` runs the credential command twice); simulated conflict: the fork rewrote EmailClient (SmtpTransport design) + CHANGELOG + fork-pruned cli-alignment-leftovers docs — pick aborts; port the SecretResolver-sharing into the fork's client from the conflicted-shas record | HIGH |
| fa1ea661 | feat(tls): add tls.client, a client certificate for mutual TLS (#809) | feat | src/config.rs(+42), config.sample.toml, cairn/spec/config.md | WAVE3_DEFER | Proven dependency on deferred 26d9aa2b: the conversion maps tls.client onto pimalaya-stream 0.4's `TlsClient`, and its import/conversion hunks anchor on 26d9aa2b's `TlsServer` lines; the fork's stream is vendored 0.3.x — re-vendor in wave 3 first | HIGH |
| 8bfb1090 | refactor(config): move the conversions out of the config | refactor | src/config.rs(-494), new src/{proxy,sasl,server,tls}.rs | WAVE3_DEFER | Config refactor over the deferred 977c3599/aa425b48/d619b9e0 state; splits the file holding the protected groups + mprg validation blocks | HIGH |
| 159221ce | refactor(config): split the schema by block | refactor | src/config.rs(-936), new src/config/*.rs (16 files) | WAVE3_DEFER | Same; creates the split layout 39817e83/4d100c4b need | HIGH |
| 4d100c4b | docs: list the sample's values in brackets | docs | config.sample.toml(-251), src/config.rs tests(+234) | WAVE3_DEFER | Reworks aa425b48's sample test post-m2dir-drop (10 sections, not 11); Cargo hunk context = deferred kit lines (io-oauth dev-dep, pimalaya-config 0.4); rewrites the fork-diverged sample | HIGH |
| 08909c64 | docs(cairn): propose relative dates in search | docs | new cairn/changes/search-dates-can-be-relative/* | WAVE2_APPLY | Pure cairn adds (the proposal behind the deferred 6dd75523 feature); simulated CLEAN | LOW |
| e25ba319 | feat(search): add the message-id clause (#811) | feat | search parsers/evals, src/capability.rs, backends, tests/{capability,envelope_search}.rs | WAVE3_DEFER | Same-lines with the deferred search-language rewrites (6dd75523's relative dates, bf9f4e2a's pimdir query language) that consume this parser work; touches src/capability.rs and tests the fork deleted | HIGH |
| ac40305f | docs(cairn): propose a result carrying its errors | docs | new cairn/changes/a-result-carries-its-errors/* | WAVE2_APPLY | Pure cairn adds (the proposal behind the deferred e575f8de/bdeb5791 line); simulated CLEAN | LOW |
| e2452a87 | test: build the live suites' accounts from the new sample | test | tests/{common,google,microsoft,fastmail}.rs | WAVE3_DEFER | Rewires the live suites onto the deferred aa425b48/4d100c4b sample shape — meaningless without it | LOW |
| 4f7b560c | docs(cairn): narrow recipient-refused and the Graph 400 mapping | docs | edits ac40305f's new cairn files | WAVE2_APPLY | Docs-only refinement of ac40305f's proposal; simulated CLEAN after ac40305f (apply in order) | LOW |
| e575f8de | feat: list the items a command could not serve in its errors | feat | src/error.rs(+60), new email/outcome.rs, every backend, shared/*, src/mcp.rs | WAVE3_DEFER | The error-carrying rework: changes error wording/structure the wrapper parses; rides a9da91bb's backend rewrites; touches fork-deleted src/mcp.rs | HIGH |
| bdeb5791 | feat: report the items the backends could not read or write | feat | backends + jmap/error.rs + live suites | WAVE3_DEFER | The backends' side of e575f8de's error-carrying line (audit-line head, cf. rubric's contract batches) | HIGH |
| c1e85ebd | fix(jmap): destroy the draft of a refused send | fix | src/jmap/backend.rs(+234) | WAVE3_DEFER | Proven: anchors on the send_email `imported()` structure only the deferred a9da91bb campaign introduced (absent at base 7bfaaa09) | MEDIUM |
| d70fb8d1 | feat: build as a library for himalaya-tui, report the mailbox total | feat | new src/lib.rs, src/main.rs(-179), src/session.rs, backends | WAVE3_DEFER | Library extraction touching fork-deleted src/{session,mcp}.rs; depends on deferred backend state; head of the tui line (b70d52ae/39817e83 "!") | HIGH |
| b70d52ae | feat!: print lists, item errors and capabilities through pimalaya-cli | feat! | every list printer, pimalaya-cli adoption | WAVE3_DEFER | Breaking kit adoption (rubric); output contract | HIGH |
| 39817e83 | feat(config)!: refuse unknown keys at every level, himalaya-tui keys under tui | feat! | src/config.rs(137), src/config/{envelope,message}.rs | WAVE3_DEFER | Breaking config-schema rework (rubric); operates on 159221ce's post-split files and the protected config.rs | HIGH |
| 81f40f9e | build: pin pimalaya-cli to git 9952b7f | build | Cargo.toml/lock | WAVE3_DEFER | Kit pin serving the deferred 6f7c7a1f/be5beee5 refusal line; fork pins pimalaya-cli 0.2.6→e4ea7f40 | HIGH |
| c650a1a2 | build: pin pimalaya-cli to git 3bada99 | build | Cargo.toml/lock | WAVE3_DEFER | Same line as 81f40f9e | HIGH |
| 6f7c7a1f | feat!: code the account refusals and a backend left out of the build | feat! | src/backend.rs, cli.rs, error.rs(+34), config.rs, mcp.rs | WAVE3_DEFER | Breaking refusal coding (rubric); error.rs contract; mcp.rs fork-deleted | HIGH |
| bf7b6987 | build: pin pimalaya-cli to git 8190dc5 | build | Cargo.toml/lock | WAVE3_DEFER | Same line as 81f40f9e | HIGH |
| 72175d1d | build: move to the released libraries | build | Cargo.toml(-25 pins)/lock | WAVE3_DEFER | Drops the git-pin block; the fork's whole `[patch.crates-io]` strategy (vendored stream, SORALNV config, pinned cli/io-imap) is the opposite | HIGH |
| e830c02e | build: move to io-imap 0.11.1, sort the io-* requirements | build | Cargo.toml/lock | WAVE3_DEFER | io-imap bump vs the fork's io-imap git pin (the mprg probe rides this transport); the requirement sort hits the diverged manifest | HIGH |
| a196cc70 | docs(cairn): archive the landed envelope-relative-date change | docs | cairn rename changes/ → changes/archive/ | WAVE2_APPLY | Pure cairn rename of a dir the fork still tracks at base content; simulated CLEAN | LOW |
| 49530ebd | fix(imap): say how the trash deletes when a delete finds no id | fix | src/imap/backend.rs(+30), src/shared/message/delete.rs(+23) | WAVE3_DEFER | Proven: the new display arms and test use e575f8de's `errors` field + `missing_message` helper and the deferred "message-not-found" naming — does not compile without them | HIGH |
| 60e48de8 | build: move to io-pimdir 0.13 | build | Cargo.toml/lock | WAVE3_DEFER | io-pimdir draft-line pin (fork pins its own rev; pimdir code is fork-diverged) | HIGH |
| 5b17916b | fix: name no next page after the last one, code a Graph quota refusal first | fix | src/email/envelope.rs, every backend, shared/{client,envelope/*} | WAVE3_DEFER | Page naming rides the deferred list/page rework (927950e2/c4164233/d70fb8d1); the Graph quota refusal is the deferred error-coding line; touches envelope output | HIGH |
| 1a8bb275 | feat!: code whole-command refusals, name message ids alike, read with id and etag | feat! | 72 files: error naming, message-id args, cairn archive moves | WAVE3_DEFER | Breaking refusal/id naming (rubric); message ids and refusal wording are wrapper contract | HIGH |
| be5beee5 | feat!: adopt pimalaya-cli 0.5.1, share the account outputs, count Graph messages | feat! | src/account/{check,list}.rs, kit 0.5.1 | WAVE3_DEFER | Breaking kit adoption (rubric); account/list.rs is the protected groups surface | HIGH |
| c7016650 | feat!: page pimdir search by token, list a header-less message as an error | feat! | pimdir, jmap queries, email/envelope.rs, error.rs | WAVE3_DEFER | Breaking (rubric); error.rs item-error coding; pimdir search is fork-deleted territory | HIGH |
| 8e8bdb37 | fix!: parse an option after a query, move onto itself as a no-op, close the live findings | fix! | 81 files: search parser, capability.rs, config/* | WAVE3_DEFER | Breaking live-findings close-out (rubric) over the deferred v3 state; capability.rs fork-deleted | MEDIUM |
| 780551a2 | test(graph): accept a null envelope size | test | tests/common.rs(+2) | WAVE3_DEFER | Asserts the null-size behavior 8e8bdb37/a04ff103 (deferred) introduce; context = bdeb5791's common.rs | LOW |
| 85a12d9a | fix!: scope a JMAP read to its mailbox, name what a get did not find, refuse a listed pimdir mailbox | fix! | jmap/gmail/pimdir backends, error naming | WAVE3_DEFER | Breaking read-scoping + not-found naming (rubric); error contract | HIGH |
| 098986c7 | fix!: scope a Gmail or Graph read to its mailbox, name what every JMAP get did not find | fix! | gmail/jmap/msgraph backends, error.rs(+11) | WAVE3_DEFER | Breaking (rubric); error.rs contract | HIGH |
| 30b3872c | chore: archive the landed changes, verify the cairn, bump pimalaya-config to 0.5.2 | chore | cairn archive renames + Cargo (pimalaya-config 0.5.2) | WAVE3_DEFER | Renames hit fork-pruned dirs (a-body-clause…, a-refused-line…, a-refused-send…); the config bump fights the fork's SORALNV pimalaya-config patch pin | HIGH |
| a726769e | feat: name the JMAP escape hatch jmap request, keep query hidden | feat | src/jmap/{cli,query→request}.rs, src/cli.rs, json_schema.rs | WAVE3_DEFER | Renames the jmap escape-hatch command; anchors on the deferred a9da91bb/5b17916b jmap rewrites; json_schema touch | MEDIUM |
| c7c43045 | feat!: refuse a mailbox created twice, page Graph listings by their next link, share the queue row | feat! | mailbox create/graph/queue, error.rs | WAVE3_DEFER | Breaking refusal coding (rubric); error contract | HIGH |
| 7a9d76dc | ci: run the live suites weekly and by hand, never on push | ci | .github/workflows/tests.yml, CONTRIBUTING.md | WAVE2_APPLY | CI trigger change only; simulated conflict solely on the tests.yml lines a57fff00 adds — take with a57fff00 (hand-add both from the record, or defer both together) | MEDIUM |
| a96c63a6 | feat!: print protocol listings raw, refuse query flags beside a Graph page token, align the schema review shapes | feat! | 67 files: raw printing, schema shapes | WAVE3_DEFER | Breaking raw/schema rework (rubric); output contract | HIGH |
| 3a077814 | feat!: print the remaining protocol commands raw, add msgraph and gmail request, one duplicate sentence, -i long-only for inline | feat! | 56 files: raw commands, cli | WAVE3_DEFER | Breaking CLI-surface change (rubric) | HIGH |
| 0154fd1f | feat!: follow pimdir draft-13 on io-pimdir 0.14, keep raw requests on the API origin, bump the family crates | feat! | pimdir, family crates, error.rs | WAVE3_DEFER | Breaking draft-13 + family bump (rubric) | HIGH |
| 10675b0e | feat!: rename maildir and mbox message save to add, save a hidden alias | feat! | maildir/mbox save→add, cli, json_schema | WAVE3_DEFER | Breaking CLI verb rename (rubric) | MEDIUM |
| d98528b5 | build(deps): bump io-pimdir to 0.14.1 | build | Cargo.toml/lock | WAVE3_DEFER | io-pimdir pin paired with draft-13 (rubric) | HIGH |
| b9712f7f | build(deps): bump jiff to 1, io-pimdir to 0.15.1 (rusqlite 0.43) and io-msgraph to 0.9.3 | build | Cargo.toml/lock | WAVE3_DEFER | Family bump serving the deferred draft-14/15 line and the msgraph rework; fork pins io-pimdir itself | HIGH |
| edf58f38 | chore(deny): ignore the unmaintained chrono advisory | chore | deny.toml only | WAVE2_APPLY | Standalone advisory ignore (rubric: advisory pin); simulated CLEAN — the fork's diverged deny.toml still merged the hunk | LOW |
| a04ff103 | fix: the family analysis fix-now batch (HIM-1 to HIM-5, HIM-10, HIM-14, HIM-18, HIM-19, TRI-1) | fix | 57 files: search parser, gmail/imap/mbox/pimdir, shared/envelope | WAVE3_DEFER | Family analysis fix-now batch (rubric); fixes audit findings over the deferred v3 state | HIGH |
| e440b4e4 | feat!: the v3 contract batch (COH-5, HIM-11, HIM-17, HIM-21, HIM-24, TRI-4, TRI-5, TRI-7, TRI-8, schema C3, C6, C12) | feat! | 117 files: cli, error.rs, backends, tests (incl. new tests/pimdir_refusals.rs) | WAVE3_DEFER | v3 contract batch (rubric); schema/error contract | HIGH |
| 6dd75523 | feat!: the pre-v3 batch (TRI-9, PIM-1, PKG-1, COH-4, HIM-6 to HIM-23 remainder, relative search dates, account tls) | feat! | 120 files: config.rs + config/tls.rs, search parsers, packaging | WAVE3_DEFER | Pre-v3 batch (rubric); includes the relative search dates that supersede e25ba319 | HIGH |
| dd9c6b12 | docs(cairn): the v3 live provider reports of 2026-11-18 | docs | cairn/spec/testing/* (11 reports) | WAVE3_DEFER | Re-runs the provider reports on the v3 CLI (describes deferred behavior); its hunks stack on 71461890/b21c2d7f/5e26640a's edits to the same files, which cannot land yet | MEDIUM |
| 8be0221c | fix: code missing mailboxes on every backend, take the jmap request refusal path, name tls cert paths | fix | error.rs(+17), search filter parser, gmail/imap clients | WAVE3_DEFER | 2026-11-18 audit-line fix batch: rides a726769e's jmap-request path and e440b4e4/6dd75523's contract state; error.rs contract | HIGH |
| 59eb42f5 | chore: point the hash references at the re-signed commits | chore | cairn changes/archive + log docs | WAVE3_DEFER | Re-points hash refs after upstream's re-sign; edits cairn files only a04ff103–8be0221c (deferred) created — modify/delete aborts | LOW |
| 1b37b2d9 | feat(pimdir)!: follow io-pimdir 0.17.0 (draft-14), name queue rows by handle, word store-version once | feat! | pimdir backend/queue, error.rs | WAVE3_DEFER | Breaking draft-14 (rubric) | HIGH |
| 0cb2ff4d | docs: record what pimalaya-cli 0.6.1 and pimalaya-config 0.6.0 change for users | docs | CHANGELOG + new cairn log | WAVE3_DEFER | Documents the deferred kit 0.6.x user-facing changes — the fork's kit is the 0.2.x line, patched; CHANGELOG conflicts | LOW |
| aa8b71be | fix: close the pre-v3 pass of the family analysis | fix | 50 files: email/{address,envelope,flag,mailbox,outcome}, imap, jmap | WAVE3_DEFER | Family analysis fix-batch close-out (rubric); operates on e575f8de's outcome types | HIGH |
| b6eed3e8 | feat!: print queue rows with their handle, add pimdir status | feat! | new src/pimdir/status.rs, pimdir backend/cli, json_schema | WAVE3_DEFER | Breaking (rubric); introduces pimdir status — a4a5f7d8/38a0e2dc/ae9d5f2c all fix this new surface; the fork's pimdir is trimmed | MEDIUM |
| a4a5f7d8 | fix(pimdir): name the mailbox mailboxId in pimdir status | fix | src/pimdir/status.rs, backend | WAVE3_DEFER | Fixes b6eed3e8's new status.rs — exists only after the deferred parent | MEDIUM |
| bffef6aa | feat!: move to the releases of 2026-11-19 | feat! | 29 files: cli, gmail/msgraph request, imap/raw, pimdir | WAVE3_DEFER | Breaking family adoption closing the deferred v3 line (rubric-adjacent releases) | HIGH |
| 15ffe166 | fix(imap): print an undecoded raw response as base64 in text | fix | src/imap/raw.rs(+40) | WAVE3_DEFER | Proven: raw.rs's events/Undecoded/exit-3 form only exists after the deferred 963a847b/3a077814 — the base raw.rs has none of it | MEDIUM |
| 066accfd | test(pimdir): hold a test store while it is filled | test | tests/pimdir_refusals.rs(+64) | WAVE3_DEFER | Proven: that test file was created by e440b4e4's v3 contract batch (deferred) — modify on a file the fork never gets | MEDIUM |
| a5623dca | test: hold the page end and inbox tests on 32-bit and case-insensitive filesystems | test | src/maildir/client.rs, src/pimdir/backend.rs (test blocks) | WAVE3_DEFER | Proven: the maildir assertion needs `is_root`, introduced by the deferred c0dd1a77; the pimdir `page_end` hunk rides deferred paging | MEDIUM |
| 134b2a1a | build: move to io-pimdir 0.18.1 | build | Cargo.toml/lock | WAVE3_DEFER | io-pimdir pin of the draft-15 line (rubric) | HIGH |
| 38a0e2dc | fix(pimdir): count unread mail through the queue as the listing does | fix | pimdir status counting | WAVE3_DEFER | Fixes b6eed3e8's status unread count (deferred parent) | MEDIUM |
| ae9d5f2c | fix(pimdir): describe the status schema rows without doc links | fix | pimdir status schema docs | WAVE3_DEFER | Fixes b6eed3e8/bffef6aa's status schema rows (deferred parents) | LOW |
| 296dcef2 | feat!: align on the 2026-11-20 contract audit rulings | feat! | 78 files: account/check, backend, cli, config/*, error.rs | WAVE3_DEFER | The 2026-11-20 contract audit batch (rubric); schema/error contract | HIGH |
| 041f3fd3 | feat!: follow io-pimdir 0.19.0 (pimdir draft-15) | feat! | src/pimdir/backend.rs, Cargo | WAVE3_DEFER | Breaking draft-15 (rubric) | HIGH |
| b3e15438 | refactor: split the shared module into the unified API and the root | refactor | moves src/shared/* → src/{unified,root} (60 files) | WAVE3_DEFER | Post-v3 structural split over fork-diverged files; operates in the fork's protected inbox neighborhood (src/shared/mod.rs beside fork-only inbox.rs) | HIGH |
| 4c026b28 | refactor: remove every inline comment | refactor | whole tree: src/**, build.rs | WAVE3_DEFER | Post-v3 comment strip; hunks land on fork-diverged files everywhere (mechanical noise, no behavior) | MEDIUM |
| aa32c77f | docs: hold every doc comment to the guideline ceilings | docs | src doc comments: config/*, base64.rs, error.rs, gmail/* | WAVE3_DEFER | Edits doc comments in files only the deferred 159221ce split and b3e15438 move created (src/config/*.rs, src/base64.rs) | MEDIUM |
| ce5ad83c | refactor: drop the kebab-case envelope aliases | refactor | src/unified/email/envelope.rs only | WAVE3_DEFER | Post-b3e15438 file (absent in the fork); removing envelope JSON aliases touches the wrapper's envelope contract | HIGH |
| af054a5c | build: move to the io-* releases of 2026-11-21 | build | Cargo.toml/lock | WAVE3_DEFER | Family bump closing the deferred v3 wave; fights the fork pin strategy | HIGH |
| e6891901 | docs(readme): cut to the CLI budgets, a tested JSON block in place of the screenshot | docs | README.md(-439), tests/docs.rs(+172) | WAVE3_DEFER | README rewritten for the deferred v3 CLI; tests/docs.rs (deleted in the fork) gains a test that runs the README's `envelope search --json` block against the binary — the fork's current output would fail it | MEDIUM |
| bf9f4e2a | feat(search)!: envelope search reads the pimdir query language | feat! | search backends/parsers, unified/* | WAVE3_DEFER | The search query-language change (rubric); supersedes the deferred e25ba319 parser work | HIGH |

## wave2-shas

```
a57fff0094d1bef8230ba217e0a9df029dbf9d1f
7146189073e4db6a4d3171cb176b7ab5fef2be3c
de1bd25f55e2cfe1f6c7abf5b10542e8510fa9de
202eb72daae970ddfb31617c2cecb60de6a6ec5a
08909c6415641963a258cfb44cab341e593bb883
ac40305ff9d83ab29278dae2c13fbcdf8bcd3c41
4f7b560c76c951aeddcb11ed8b5918a707339797
a196cc704fb32bf72724c16731e5e33423a33c19
7a9d76dcc99131d71909959a73a7d088074c4504
edf58f38a1f08220ecfb5a476e2b4ea7d2b0500c
```

## wave3-shas

```
e94de7bd68bfb756429daf089ec2bb813d6c0987
6284d2fb6fab264215b48d58c13e91c46aee40ec
682edb40dab03cbab4417a9346a3debfef886adf
42134fd20ca6d37aa86e02d6777487231cfe7032
82207e0eca0630ce8d888602e7cb78ffe8bf036b
977c35992136d407fb4050874d9c578d6591e3d0
8b118208b4d136dcda804474a87fc0e255e08800
aa425b484685153558a06e84f698134d567e36d3
ddcbaa4993a98881e3c4120b298e3fbfd32fdea0
1bec8bdcf040bbd89c9a0b26206c553465e7b05d
25d3ae383935af26f74e1ba1b069814a42d9dbc1
59798cfa0688a301f9c0542c083c60258fc69227
53c17c9809a3c5fd8da0c864fcf06b1f0205b212
ba07a582abac487543cf4a1607bce36bb93f15c6
6226c8397d9ac9e33ff85ebd58f0ddef4c025f3c
c9eab51b4388a3f2e18fdb2db6af071e71fc1edd
6ce2d12ed5127e5eb622e9e55314fc1a0e1acc2c
d619b9e05f3e32627a4e0811a2d7d9d3e316eed7
16673d111b46e73ed851e89d19aba8e4ac837ea6
927950e26183c0e53a29f7642cc692d477fca870
a8ccc8f8b13ad9a561fc65840d3062ae06d54490
ca792b9662978665fc034b808daef8afebfab7e6
b21c2d7f18aef8caeb5b68a7dd0fefee5673a4bc
c416423385f42500b3c691420ce95093a31b194d
a9da91bbec61650cc343d8ad9be68a930e4e2db8
5e26640a8166c4c466df749adb03d01359c323ca
eaf90100ec03fb6a0265b0aaf0676a5268a8dc7b
c0dd1a774d15372c589f0800d763e00ba717bf4a
4c7462604aaa49093d99d6ea7c44757ad9ede5be
46e8b4ea05e892f4d7b00caf8472533e43ce869f
dda23f28c87fc50a6a59de41dd7a5294e8238a18
963a847bcde6f2ae695930f1083318b68e91ee42
cabcf26beba9cbd3317028839dd8002f4aacc15d
7518aee32807de7600587fc0ab0e9c2b70c7d2ae
f33316acb45fbfd04ec9a00238cc1855ac73a227
8a4b9b8d01c6ed1594d4f323e020597a01fb87a8
26d9aa2b631eabfe4f8225d7bd0c8a2f60187fc6
fa1ea66156126363cb9760ddc204905cea167a06
8bfb1090db3ab84348b6fac2f1fad6ec09cad6f9
159221ce1e89dbf881f77dcd03ab109fcb284d88
4d100c4be3221ba9b998cab8a18cdc0ab3aafd98
e25ba319b88501b1d8a622876b68ffe31eb41c1c
e2452a8717c4666e0c3cacad6741183cabedbf40
e575f8de689e3633dca0599c35eefe261f6304f3
bdeb579104c882aaa169d89158297d6fa3d06f71
c1e85ebd3c3f00d75053281f4b6f2b8f93378bcf
d70fb8d11cf9d7d66b1cda8f2e8bb5270acd0da2
b70d52aedb4f0ae6dca0dc6a1bd6a44425efd875
39817e83a46bf2affbc34d317cc93ade362f46dd
81f40f9ef3be85cbeaaff30e502ac28c947ab91b
c650a1a2be974c3dac6be86ccb4e088efa15330f
6f7c7a1fa2d2b93cdb745baae37a729cb864ff0a
bf7b698799e79cd25d7776ceedff3e4d64958bb3
72175d1dfeeacae3988d463e547ac6ac2e87e358
e830c02e6c82a36b456faa0f65ef29aac9510304
49530ebdf3bac8d300fd24f537f5c13835eb8bf5
60e48de83889da89acdcd60282329c9c8778831a
5b17916bb8a451eaf5034ffdb0a3f6b2c0c93ced
1a8bb2753af590f22a5bad6ee38cfc98e1cb9d32
be5beee5b64eec07ec201ad17e90bf3558015570
c70166503072ac614e76a82d286eacfa5cf01a2b
8e8bdb37ddeb917ed2f3751431868cf391c55c46
780551a200206615ae8f6f20840f9a49bea5fc6d
85a12d9a7171771953250f7c0cd078142503c0fc
098986c74542ecca5c054848dee448ff64a7cfa4
30b3872cefe851d8eae7f8a72ebfb4451822938f
a726769e4c94a9e690f8eda24b0808f673d81069
c7c4304510076061cf9e17acd516030371cf0eed
a96c63a6bacec1256e647ef8c0d75c6038040f63
3a077814a2e14e55c743f8e3225ca1fd630630a2
0154fd1f0a19b7f7f93a8955842111ca70d1498a
10675b0ee6ed9d664265cdfd58efb1837da7b2ce
d98528b54eff33dd546c39caf11c7d7b319acbf4
b9712f7f712e3263bd34a1204002d5f2e60dc8d5
a04ff10368ed9a7a08633d591f6b0127138ec6b8
e440b4e440a8a38649068eb8511f885d7f578175
6dd7552348c156feaec957e52578dfb4b2b059c2
dd9c6b125686326fbbf62895a90024fcec2fefba
8be0221cb3cb6525a9b1c79d8127803c8796b22f
59eb42f5be273e07457c6f1b70b35a6732bea014
1b37b2d9559f0ebb7856006d66b7e34d4efafef3
0cb2ff4d41da4f0d94786b0facab592365f14932
aa8b71bed77e70f866730fbdbcdf977643513f48
b6eed3e85b3adea5ee70d6d356f5210b4d05c40c
a4a5f7d83a9be4be823902660434996b7c954d12
bffef6aa21691530ba8b08faf5e45979d9f770ea
15ffe166dcb6b82474e397a1958fc8b97713e14a
066accfd50a2d7bd9c44deaa930670438b1d3bf6
a5623dcac6150f1a8d8ee2d381c4569d7dc815b8
134b2a1ad9963f300d5e4335f5283fa33a477d29
38a0e2dc5c3048e7bcdab4d5a46d80c6161a3405
ae9d5f2c4b35496df4917f04cdb7f7e56f279b58
296dcef299e19b97dff33482ebca08e62bdf01f7
041f3fd37dd4ec9066539844ebf0a614561acf26
b3e154380baf1b3b153e76205c375a4c29388644
4c026b28d41b3fc2c1b686d6b309ec2897e99d11
aa32c77fffd7ca5ef29b63cf77da7d7e182b1c11
ce5ad83cf8cd3c706b47af2cf2829d7468291b46
af054a5ce85c07909d04eeb1e1e4bb6f740be4e5
e6891901b468433045d3321052d27d5151b0eb77
bf9f4e2a25484b2ee12bafdc06c891b73ceca3e8
```

## skip-shas

```
```

(None: the 21 importable upstream sources behind the fork's "cherry picked from" trailers are all ancestors of the base, outside the range; patch-id comparison over the range finds no equivalent, and no range subject matches a carried subject.)
