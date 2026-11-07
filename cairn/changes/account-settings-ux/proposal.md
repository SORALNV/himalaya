---
cairn: change
id: account-settings-ux
status: landed
created: 2026-10-17
---

# Account settings as an editable Web form

## Problem

The user reports that the delivered ordinary-form wizard is difficult to use. Inspection confirms an explanation-heavy welcome screen, excessive whitespace, one field per sequential worker question, technical credential labels and only restart-based correction. This preserves terminal mechanics rather than providing a usable account-settings workflow. Directly opening the source HTML also produces an unusable entry point; launch clarity must be part of the design.

## Approved intent

Replace the one-question interaction with an account-settings screen: existing accounts listed alongside an add-account form, email input visible immediately, discovered settings filled into editable grouped connection and authentication sections, and explicit connection-check/save actions. Keep specialist fields under an expandable details section without removing provider/backend/authentication capabilities. Show errors next to the relevant fields and preserve non-secret drafts for correction rather than forcing a complete restart. Never silently replay authentication or custom command execution when a previous answer changes.

The intended primary password path accepts a password in the Web form and creates a dedicated Keychain item only on explicitly confirmed saving, reusing the existing bounded create-only/rollback safeguards rather than requiring a separate terminal operation. Existing credential references and external token-broker choices remain available. Native OAuth provisioning is not implied by displaying an OAuth option. General server fields represent a larger capability than the terminal wizard's discovery-only interaction and require explicit destination validation/confirmation before credentials are transmitted.

Provide an unambiguous launch entry and a clear non-functional-file-preview notice instead of presenting a source HTML preview as a working application. Avoid arbitrary launcher-selected config paths, browser-persistent credentials, additional permissions, public deployment, upstream PRs or changes to existing accounts.

The user approved this intent on 2026-10-17 with “それで進めて。” Implementation may proceed. Existing accounts, unrelated work and upstream PRs remain unchanged.
