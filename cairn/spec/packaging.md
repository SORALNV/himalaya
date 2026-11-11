---
cairn: spec
capability: packaging
status: current
---

# Packaging

Himalaya is an application, the top layer of the Pimalaya stack. It writes no protocol or storage logic of its own: it is a thin shell driving the sans-I/O io-* libraries below it, consuming their blocking `*Std` clients and orchestrating and rendering the results. The CLI plumbing (clap args, printer, logger), TOML config loading and the blocking stream runtime come from pimalaya-cli, pimalaya-config and pimalaya-stream.

### Requirement: Binary only
Himalaya SHALL build as a binary with no public library API and no lib target. A user needing the protocol or storage logic reaches for the io-* library that owns it (io-imap, io-jmap, io-gmail, io-msgraph, io-smtp, io-managesieve, io-maildir, io-m2dir, io-mbox, io-pimdir).

### Requirement: Feature-gated backends
Every backend SHALL sit behind its own cargo feature (`imap`, `smtp`, `jmap`, `gmail`, `msgraph`, `maildir`, `m2dir`, `mbox`, `pimdir`), plus a `wizard` feature, so a build ships only the protocols it needs. A protocol command, its backend adapter, and its wizard branch compile only when its feature is on.

### Requirement: TLS provider features
The TLS providers SHALL be cargo features forwarded to pimalaya-stream and every network backend: `rustls-ring` (default), `rustls-aws`, `native-tls`.

### Requirement: Release profile
The binary manifest SHALL carry the shared release profile (`lto = "fat"`, `codegen-units = 1`, `strip = "symbols"`, `panic = "abort"`) and omit the library-only manifest fields (the docs.rs metadata block, the documentation field, the no-std category).

### Requirement: Licence
Himalaya SHALL be dual-licensed MIT OR Apache-2.0, with no per-file headers.

### Requirement: Live provider suites
The binary SHALL carry one ignored integration suite per provider under tests/ (google.rs, microsoft.rs, fastmail.rs), run with `--ignored` and sharing tests/common.rs. Each SHALL hold an `anonymous` module that runs every backend the provider's sample offers on a made-up token and touches no account, its accounts taken from the provider sections of config.sample.toml where one fits so a provider drifting from the sample fails it, and asserting the exit code of the output contract (2 with the `account check` report, 1 with the failure object) and a message naming the refusal in Himalaya's words. It MAY hold a `credentialed` module that mints its own token from `HIMALAYA_`-prefixed variables and only reads (`account check`, `mailbox list`, `envelope list`), failing naming the variables it misses, which CI runs in the upstream repository only and once its secrets exist. A token SHALL reach the binary through a token command reading its environment, never argv nor the configuration, and no assertion SHALL print a token or secret. CI SHALL run each suite in its own job at `RUST_LOG=debug` at most.

