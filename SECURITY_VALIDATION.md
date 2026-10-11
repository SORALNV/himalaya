# Fork security validation

The validation branch combines three Himalaya fixes with the credential-helper fix in [SORALNV/config](https://github.com/SORALNV/config/tree/codex/secret-error-redaction). Cargo.toml and Cargo.lock pin that dependency to commit `12ed680559182516d7df56f5bc349f820e00c808`. This dependency override is for the fork's validation branch and is not part of the individual upstream Himalaya fixes.

## Build and reproduce

```sh
cargo build --locked
python3 scripts/security_smoke.py
```

The POSIX smoke check uses only fake messages, dummy secrets, a temporary Maildir and private pseudo-terminals. It contacts no email server, reads no real account configuration and removes its temporary fixtures after running. All applicable scenarios must report PASS; macOS runs seventeen, including explicit/inherited ACL rejection and configuration/FIFO substitution during wizard prompts. A terminated or timed-out process is not accepted as a successful rejection.

## Verified behavior

Attachment downloads reserve new files exclusively. Existing files and leaf symlinks are never replaced, exhaustion fails, and concurrent writers reserve distinct names. The download directory itself must be trusted.

Human-readable message views replace control and bidi characters in headers, MIME metadata and decoded bodies. Body tabs and newlines remain readable. JSON strings and explicit raw output preserve their original content, so raw bytes should be redirected to a file when inspecting untrusted mail.

On Unix, the wizard creates configuration files with mode 0600 and new directories with mode 0700. On macOS, ACL-bearing write targets and new directories are rejected before generated credentials are written, rather than silently deleting user ACLs. Failed creation may leave empty files/directories. The named `configure` command rejects nonregular files, retains the original descriptor through prompts and checks contents and Unix device/inode identity before appending. Appending tightens file permissions before adding the account. Existing parent directory permissions and configuration bytes remain intact. Parent directories must be trusted; hostile same-user concurrent mutation, Windows ACL hardening, and the general configuration loader used by bare invocation and other commands are outside this fix.

Failed credential helpers report their exit status without echoing stdout or stderr. Successful helpers retain first-line secret resolution.

## Validation results

On macOS arm64, the updated combined fork's all-features suite passed 254 unit tests and 6 integration tests, with one existing test ignored. All-features clippy with warnings denied and formatting passed. The dependency's all-features suite previously passed 25 unit tests and one documentation test, and its clippy and formatting passed. The isolated wizard PR passed 247 unit tests and 6 integration tests, including all sixteen wizard configuration tests.

Reduced IMAP/SMTP and JMAP builds passed. The ManageSieve-only suite passed 64 tests, and the wizard/Maildir-only configuration test suite passed. Reduced builds emit existing unused-code warnings outside the changed files; they do not fail to build. The dependency builds with no features, TOML alone and secrets alone. Its secrets-only test target requires the existing TOML serialization tests' dependency, so tests were run with all features instead of changing unrelated tests.

Cargo audit found no known vulnerabilities or advisory warnings among 328 locked packages using the RustSec database updated on 2026-10-09. Cargo deny passed advisories, licenses, bans and sources; it emitted dependency-duplication warnings. These results cover the checked source and macOS build, not every operating system or external OAuth helper.

## Gmail validation and upstream submission

Use a separate private configuration and a Keychain-backed app password. Run the connection check, list a small inbox page, read a known test message and download a known harmless attachment into a fresh directory. An optional send test should send only a user-approved message to a user-selected recipient.

The account holder registered an app password locally in Keychain. IMAP and SMTP authentication, a small envelope listing, and a user-authorized self-addressed dummy message send/receive succeeded. The dummy message was displayed without marking it seen. Attachment download and repeat download succeeded, selecting a suffixed name without replacing the first file; both SHA-256 hashes matched. No unrelated mail bodies were read and no account identifiers or credentials are included here.

The fixes were submitted as [attachment PR #782](https://github.com/pimalaya/himalaya/pull/782), [message-view PR #783](https://github.com/pimalaya/himalaya/pull/783), [wizard PR #784](https://github.com/pimalaya/himalaya/pull/784), and [credential-helper PR #1](https://github.com/pimalaya/config/pull/1). PR #784 was updated in place with ACL and save-target validation; no history was force-rewritten. The fork-only dependency override remains excluded from those Himalaya PRs.

## Independent review

An independent gpt-6-astra subagent reviewed the wizard follow-up. It found an inode-reuse gap in the first iteration; retaining the original file handle and adding unlink/recreation tests corrected it. Its final source review found no remaining blocking issue within the named-wizard scope. It did not independently rerun the test suites. GitHub CI had not reported checks when inspected; local test success is not an upstream CI result.
