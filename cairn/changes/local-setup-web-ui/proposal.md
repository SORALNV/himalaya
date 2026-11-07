---
cairn: change
id: local-setup-web-ui
status: landed
created: 2026-10-17
---

# Fork-only local account setup UI

## Why

The user registered a university Google Workspace mailbox through several terminal prompts, a separate Keychain operation, failed discovery and manual TOML editing. A local browser wizard can make provider selection, credential storage, connection checks and explicit saving understandable without relaxing the fork's save-target protections.

The user approved this proposal on 2026-10-17. Implementation, isolated tests, native disposable-Keychain checks, browser verification and Astra source review are complete. This is not a security certification or live-provider verification. The feature belongs only to SORALNV/himalaya; existing upstream PRs and unrelated fork maintenance remain unchanged.

## Initial scope

Add `himalaya configure --web`, explicitly launched by the user, on macOS builds with the required features. Other platforms fail clearly without starting a server. The existing terminal wizard remains available and its discovery-only contract is unchanged.

The Japanese UI adds Gmail/Google Workspace or iCloud accounts using fixed provider presets, including custom Google Workspace domains without automatic discovery. The flow is provider, account identity, credential, connection check, safe configuration preview, explicit save, then finish. Provider endpoints and transport security are visible but not editable. Connection checks authenticate only; they do not send messages or fetch mailbox contents.

The initial scope includes creating a fresh dedicated Keychain item, or using an explicitly named existing item for the new draft. It excludes arbitrary IMAP hosts, account replacement/removal, changing defaults of existing accounts, editing raw TOML, custom secret commands, mailbox browsing, mail mutations, mobile/LAN access, cloud services and native OAuth grants. OAuth is a separate follow-up requiring its own consent, scopes, client provisioning, state and PKCE design; the UI must not imply password authentication is OAuth or bypass university policy.

## Security boundary

### Local process and browser session

- Bind only an OS-assigned port on `127.0.0.1`, never a wildcard or LAN interface. Accept only the exact bound IP/port in Host; reject alternate names, ports, duplicate Host fields and malformed requests. Do not rely on browser local-network protections.
- Generate a cryptographically random 256-bit capability per launch. Open the browser with the capability in a URL fragment, never a query or path. Remove the fragment with history replacement after bootstrap. Do not print the capability or complete bootstrap URL to logs; browser-open failures fail closed without publishing an unauthenticated fallback.
- Keep the capability in page memory only and require it in a custom header on every non-public API request. Compare it in constant time. Public static assets reveal neither credentials nor session capabilities. An unauthenticated request cannot discover the capability.
- Require the exact same Origin for state-changing requests, validate Origin when supplied on other API requests, and reject null/foreign origins, cross-origin preflights and unsupported content types. CORS is not authorization. No GET route changes state, performs provider checks or reads a Keychain secret. Require the authenticated token even for account metadata.
- Bundle all assets locally. Apply restrictive CSP, including no remote connections/assets, no inline scripts, no framing, and no injected HTML; add no-store and nosniff headers. Treat labels, errors and configuration-derived strings as untrusted text. No analytics, external fonts, service worker or browser persistent storage.
- Bound HTTP header/body sizes, read times, concurrent work and provider timeouts; reject unsupported methods and ambiguous framing. Use a maintained HTTP implementation rather than a handwritten parser. Stop after ten authenticated-idle minutes or thirty total minutes, with authenticated explicit finish/cancel and Ctrl-C support. Unauthenticated traffic cannot extend the session. After stopping, the old token and requests cannot save.

### Credentials and network operations

- Accept an app password only for the selected preset. Validate and normalize its expected input shape locally; never transmit it to a guessed, user-provided or redirect-selected endpoint. Use normal TLS verification, with no UI switch to disable it, downgrade transport or apply arbitrary proxies.
- Prefer native macOS Keychain APIs. Passwords never become command-line arguments, environment variables, URLs, config values, logs, HTTP response bodies or telemetry. Do not claim browser memory can be reliably zeroized; clear fields and transient server values promptly and use secret-aware Rust types where practical.
- An existing Keychain item is referenced only by the explicit user-supplied name, not discovered by dumping Keychain contents. No API returns a saved password. Existing items are neither overwritten nor deleted. Never execute arbitrary credential commands parsed from an existing configuration during UI listing or checks.
- Test an in-memory, typed draft through existing backend authentication clients, using fixed verified Gmail/iCloud endpoints. Return bounded, classified diagnostics rather than raw provider errors or commands. Recheck/require the successful test to match the exact draft being saved.
- New Keychain items get unpredictable per-account identifiers and are created only on explicit save. Native operations run in ten-second deadline-bounded helpers with private standard-I/O IPC; persistent item references retain exact rollback identity across processes. On a pre-write config failure, roll back only the exact item created by this operation. If a creation timeout, cleanup or a partial config write is uncertain, report a recoverable partial outcome; do not silently claim atomicity or delete existing items.

### Configuration persistence

- Fix the save target from CLI `-c`/normal resolution before the listener starts; the browser cannot select paths. Reject multiple merged targets for this first UI rather than write to an ambiguous configuration.
- Share the fork's existing validated descriptor/snapshot and append implementation with the terminal wizard; do not introduce a second weaker writer. Keep private modes, macOS ACL refusal, regular-file checks, nonblocking opens, original descriptor/device/inode/content verification, unchanged-symlink compatibility and trusted-parent assumptions.
- Show only the typed draft and its redacted/reference-only preview, never the raw existing document. Preserve existing bytes, account defaults and unrelated settings. Reject duplicate account keys, arbitrary helper commands and conflicting concurrent saves. Serialize draft test/save actions, share an advisory descriptor lock across cooperating CLI/web writers, and authorize immediately before mutation. Coordinate explicit revocation with the commit boundary so a previously committed save has a known outcome before shutdown is acknowledged.
- Existing CLI security and unified-inbox behavior must remain unchanged. A UI failure does not justify chmod/ACL workarounds, deleting a user's config, or falling back to plaintext secrets.

## Verification and release gate

Before delivery, adversarial tests must demonstrate that rejected requests cannot invoke Keychain, connect to a provider or change a config. Cover unauthenticated/cross-origin/rebinding requests, malicious labels, HTTP limits, timeouts, token replay, session expiry, duplicate submits, credential redaction, provider allowlisting and save-target races.

Run native Keychain integration only against unique disposable test items with explicit cleanup; never probe or overwrite user credentials. Use isolated fixtures for HTTP and persistence failures, plus the fork's security and inbox smoke suites. Perform browser end-to-end verification with dummy credentials/fake provider adapters; real account checks require a separate explicit user action and must not fetch mail or send messages. Check Git diff/log output for credentials before publishing, update current specs only once implementation lands, and document residual risk.

Loopback HTTP is not a sandbox against compromised same-user software, malicious browser extensions, root access, an untrusted configuration parent directory or hostile same-user concurrent writes. This feature does not itself grant OpenMausBot access or make its model/data path private.
