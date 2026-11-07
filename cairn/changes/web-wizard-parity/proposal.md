---
cairn: change
id: web-wizard-parity
status: landed
created: 2026-10-17
---

# Restore terminal wizard capability in browser setup

## Why

The user rejected the Gmail/iCloud-only browser setup. The requested product is a browser interface to the existing account configuration workflow, not a second wizard with fewer services or authentication options. The initial preset restriction was an implementation choice, not a user requirement.

## Approved intent

On 2026-10-17 the user selected conventional Web forms, explicitly rejecting terminal embedding. Make `configure --web` expose the existing wizard through typed form questions, retaining discovery, backend selection and authentication/credential-source choices supported by the installed build. Do not maintain a separate provider allowlist. Reuse the actual discovery/authentication workflow with a prompt adapter, not a second backend-specific wizard.

Retain ephemeral loopback binding, per-launch authentication, exact Host/Origin checks, bounded resource use, locally bundled assets and the fork's guarded configuration writer. Browser inputs must never select the executable or launch a general-purpose shell directly. Explicitly disclose and confirm that the original wizard's custom secret commands execute with the user's local permissions and raw-secret storage is not recommended. A bounded fixed self subprocess exchanges typed questions over private standard I/O and returns only the newly configured account to the parent, which renders a credential-redacted preview and performs an explicitly confirmed guarded save. Shutdown terminates and reaps the owned worker process group; polling does not refresh idle expiry. No extra service access or live-account checks occur without user action.

This change remains exclusive to SORALNV/himalaya. Existing account data, unrelated work and upstream security PRs remain untouched. Astra security review remains part of the approved Web UI workflow after the design is selected.

## Local delivery

Ordinary forms, shared discovery/authentication questions, typed bounded credential checks and guarded generated-TOML saving are implemented and installed locally. Tests passed: 307 unit plus 6 integration tests, strict clippy, formatting/diff checks, 17 security and 21 inbox smoke checks, isolated TERM/HUP teardown, and real-browser actual-local/synthetic-credential flows. Astra's final source review found no actionable blockers within the stated model. The previous executable is backed up, and the replacement UI was launched against the existing integration config without modifying that file. No commit, push, upstream PR, real-provider authentication or user credential operation is claimed.
