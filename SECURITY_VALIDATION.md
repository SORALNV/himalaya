# Fork security validation

The validation branch combines three Himalaya fixes with the credential-helper fix in [SORALNV/config](https://github.com/SORALNV/config/tree/codex/secret-error-redaction). Cargo.toml and Cargo.lock pin that dependency to commit `12ed680559182516d7df56f5bc349f820e00c808`. This dependency override is for the fork's validation branch and is not part of the individual upstream Himalaya fixes.

## Build and reproduce

```sh
cargo build --locked
python3 scripts/security_smoke.py
```

The smoke check uses only fake messages, dummy secrets, a temporary Maildir and private pseudo-terminals. It contacts no email server, reads no real account configuration and removes its temporary fixtures after running. All twelve scenarios must report PASS.

## Verified behavior

Attachment downloads reserve new files exclusively. Existing files and leaf symlinks are never replaced, exhaustion fails, and concurrent writers reserve distinct names. The download directory itself must be trusted.

Human-readable message views replace control and bidi characters in headers, MIME metadata and decoded bodies. Body tabs and newlines remain readable. JSON strings and explicit raw output preserve their original content, so raw bytes should be redirected to a file when inspecting untrusted mail.

On Unix, the wizard creates configuration files with mode 0600 and new directories with mode 0700. Appending tightens file permissions before adding the account. Existing parent directory permissions and configuration bytes remain intact.

Failed credential helpers report their exit status without echoing stdout or stderr. Successful helpers retain first-line secret resolution.

## Validation results

On macOS arm64, Himalaya's all-features suite passed 247 unit tests and 6 integration tests, with one existing test ignored. All-features clippy with warnings denied and formatting passed. The dependency's all-features suite passed 25 unit tests and one documentation test, and its clippy and formatting passed.

Reduced IMAP/SMTP and JMAP builds passed. The ManageSieve-only suite passed 64 tests. Reduced builds emit existing unused-code warnings outside the changed files; they do not fail to build. The dependency builds with no features, TOML alone and secrets alone. Its secrets-only test target requires the existing TOML serialization tests' dependency, so tests were run with all features instead of changing unrelated tests.

Cargo audit found no known vulnerabilities or advisory warnings among 328 locked packages using the RustSec database updated on 2026-10-09. Cargo deny passed advisories, licenses, bans and sources; it emitted dependency-duplication warnings. These results cover the checked source and macOS build, not every operating system or external OAuth helper.

## Gmail check before upstream submission

Use a separate private configuration and a Keychain-backed app password. Run the connection check, list a small inbox page, read a known test message and download a known harmless attachment into a fresh directory. An optional send test should send only a user-approved message to a user-selected recipient.

The real Gmail check requires the account holder's authentication setup and remains pending. Upstream pull requests follow that check, with one intent per PR and the dependency fix submitted to pimalaya/config. Each PR should disclose OpenAI Codex assistance and include the relevant regression tests and validation results.
