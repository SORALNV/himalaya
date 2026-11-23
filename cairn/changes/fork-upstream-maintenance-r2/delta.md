---
cairn: delta
change: fork-upstream-maintenance-r2
---

## ADDED Requirements

### Packaging: Live provider suites
The binary carries one ignored integration suite per provider under tests/
(google.rs, microsoft.rs, fastmail.rs), sharing tests/common.rs, running only
with `--ignored`: an `anonymous` module per provider that touches no account,
and optional credentialed modules minting their own token from `HIMALAYA_`
variables. Landed with the a57fff00 pick (fork-adapted: only the io-oauth
dev-dependency, the fork's crate pins kept).

## MODIFIED Requirements

### Config: A credential command is spawned once per run
The requirement now carries the send-saving-copy scenario: an account whose
IMAP and SMTP blocks name one password command runs it once, the SMTP
transport connecting on the send with the value the IMAP session resolved.
Landed with the 202eb72d pick (fork-adapted onto the fork's lazy SmtpTransport
and fork-only scan path resolving their own).

## REMOVED Requirements

None. The manual Posteo provider report under cairn/spec/testing/ was
superseded by the live Fastmail suite, following upstream's own move.
