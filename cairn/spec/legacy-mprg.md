---
cairn: spec
capability: legacy-mprg
status: current
---

# Explicit pinned legacy mail transport

### Requirement: Narrow, opt-in endpoint policy

With the legacy-mprg build feature on macOS, IMAP 143 and SMTP 587 STARTTLS at mail.mprg.cs.chubu.ac.jp MAY select an explicit 64-hex-digit SHA-256 DER leaf pin through their per-protocol TLS provider. Missing feature/platform, invalid endpoint/pin, disabled STARTTLS, implicit TLS, userinfo, query/fragment, non-root path or conflicting TLS options SHALL fail closed. ManageSieve SHALL reject this profile before resolving credentials. Ordinary provider selection and all unrelated accounts SHALL remain unchanged. Unsupported implicit-TLS consumers SHALL never return a legacy transport.

### Requirement: Pin before authentication and bounded lifetime

The compatibility branch SHALL negotiate TLS 1.0 using macOS native TLS and compare the same connection's actual leaf DER hash before returning a stream to authentication code. Missing or changed certificates SHALL fail before application/authentication bytes. No plaintext fallback, silent pin renewal, OS trust import, global TLS policy change, relay or persistent service SHALL be introduced. A ten-second total handshake deadline SHALL survive trickling input; its owned socket monitor SHALL be cancelled/joined before returning success or failure, and successful streams SHALL regain prior socket timeouts.

### Requirement: Explicit risks and isolated credential handling

The UI SHALL disclose obsolete cryptography, replacement of ordinary CA/hostname/validity checks and unauthenticated initial pin-bootstrap risk. Runtime legacy selection SHALL remain off by default. A preset MAY populate known endpoints/observed pins but SHALL NOT grant risk approval, run authentication or save. Receive/send pins SHALL be independent. Transport/pin edits SHALL revoke risk approval; every settings edit SHALL retain existing destination/ticket invalidation. Changing away from IMAP SHALL clear the legacy selection. Errors and previews SHALL not reveal provider/helper secrets. Local credential input, create-only Keychain saving and guarded append protections SHALL remain unchanged.

### Requirement: Honest read-only verification

Transport-only success SHALL NOT be reported as authentication or receipt success. Real-provider authentication SHALL require local user input and SHALL avoid send/delete/Seen changes, message bodies, subjects and attachments. A successful account check and IMAP STATUS count retrieval MAY establish authentication and read-only mailbox access. Long-term stability and arrival of a new test message SHALL remain separately reported evidence, not inferred from cached mail or a TCP connection.
