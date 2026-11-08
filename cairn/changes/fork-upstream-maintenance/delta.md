---
cairn: delta
change: fork-upstream-maintenance
---

## ADDED Requirements

### Requirement: A composed Message-ID sits on the sender's domain
`message compose`, `reply` and `forward` SHALL set the `Message-ID` on the domain of the `From` address rather than the hostname of the machine, the local part coming from the generator. With no `From`, or a domain that is not a plain dot-atom (a domain literal, an internationalized domain), the header SHALL be left to the generator, which falls back to the hostname.

### Requirement: Preserve fork protections during upstream maintenance
The fork SHALL keep the stronger named-wizard save-target checks and the pinned credential-helper error redaction when importing upstream maintenance. Its unified inbox SHALL retain account identity, backend receipt timestamps and explicit partial coverage across protocol-library updates.

## MODIFIED Requirements

### Requirement: IMAP move without the MOVE extension
The IMAP adapter SHALL use source UID search to count successfully copied messages when COPYUID is absent or unreadable, and SHALL flag and UID-expunge only those source messages. Native UID MOVE SHALL first resolve the present source UIDs and SHALL report no successful move for absent messages. A plain EXPUNGE SHALL NOT remove unrelated messages.

### Requirement: Raw message input is shared
Raw RFC 5322 message input SHALL be read whole as bytes from files and piped stdin. CRLF normalization SHALL preserve non-UTF-8 content, and read failures SHALL NOT become silent successful truncation.
