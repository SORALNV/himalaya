---
cairn: change
id: mprg-legacy-transport
status: landed
created: 2026-10-18
---

# Explicit pinned legacy transport for MPRG

## Reviewed user intent and changed risk boundary

The user now explicitly prioritizes successful stable connectivity for the MPRG mailbox over current TLS security, has no administrator/SSH route, and requests implementation with Astra consultation. This supersedes the earlier no-legacy assumption in mprg-secure-connectivity only for this named service. The user has been warned that intercepted Slack login codes can be used by a third party; compatibility is not described as secure or harmless.

Implement a disabled-by-default, explicit per-connection compatibility profile for mail.mprg.cs.chubu.ac.jp. Preserve every existing modern TLS path and every unrelated account. Use the smallest validated in-process TLS backend capable of TLS 1.0/DHE-RSA-AES256-SHA; do not install a public/local plaintext relay, change OS/OpenSSL global policy, import CA certificates globally or attempt Microsoft private API/token reuse.

Retain exact SHA-256 leaf-certificate pin verification before returning the transport to any authentication code. Pin mismatch, absent pin, unsupported host/profile or failed upgrade must fail closed with no plaintext fallback or silent pin renewal. The initial observed pin is an explicitly accepted bootstrap exception, not independently authenticated server identity. Certificate rotation requires explicit reconfirmation. Account credentials remain in Keychain/opaque references; no secrets in chat, argv, environment, logs or source. Existing references and config comments/defaults remain intact.

First prove credential-free repeated protocol/TLS operation, including negative pin and unchanged modern rejection. Only then ask the user for local password entry and perform bounded read-only authentication/metadata checks. No message send/delete/Seen changes. Preserve guarded config appends and backup installation before replacing the installed fork. New legacy/profile code and dependencies remain in SORALNV/himalaya, not an upstream security PR.

The current request approves this narrow compatibility intent. Concrete backend choice is subject to capability probes and Astra review; no broader downgrade or unrelated account mutation is approved. Web credential entry, if used, retains localhost capability/origin/session safeguards and a conspicuous legacy-risk approval rather than weakening normal forms.
