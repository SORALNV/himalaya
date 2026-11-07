---
cairn: delta
change: web-wizard-parity
---

## ADDED Requirements

### Browser and terminal configuration share supported workflows

The browser setup SHALL expose the existing terminal wizard's supported discovery, backend and credential-source choices through conventional Web forms, as explicitly selected by the user. It SHALL NOT embed a terminal or restrict the account to Gmail/iCloud presets. The browser SHALL clearly describe and confirm risks inherited from custom credential commands and raw-secret configuration.

## MODIFIED Requirements

### Local browser setup is not provider-specific

Replace the fixed Gmail/iCloud-only requirement with shared-wizard capability. Retain authenticated loopback transport, explicit user actions, bounded resource use, protected save targets and fork-only scope. Adjust secret-handling claims to the actual selected workflow rather than claiming all terminal credential choices use Keychain.

## REMOVED Requirements

Remove the browser-only restriction to two providers, PLAIN app passwords and fixed preset endpoints. This SHALL NOT remove TLS certificate verification, bypass organization policy or grant arbitrary unauthenticated access.
