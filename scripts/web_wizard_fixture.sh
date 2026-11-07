#!/bin/sh
# Synthetic UI-only worker: no network, Keychain, config reads or actual secrets.
set -eu
question() {
  printf '%s\n' "$1"
  IFS= read -r ignored
}
question '{"type":"question","question":{"id":1,"kind":"text","label":"Email:","default":null,"choices":[]}}'
question '{"type":"question","question":{"id":2,"kind":"select","label":"Choose a configuration:","default":"0","choices":["IMAP + SMTP mail.example.test","JMAP https://mail.example.test/jmap","Gmail API","Microsoft Graph API"]}}'
question '{"type":"question","question":{"id":3,"kind":"text","label":"Gmail user id:","default":"me","choices":[]}}'
question '{"type":"question","question":{"id":4,"kind":"select","label":"Gmail access token strategy:","default":null,"choices":["security (macOS Keychain)","pass (password store), not found on PATH","ortie (Pimalaya OAuth 2.0 token broker), not found on PATH","pizauth (OAuth 2.0 token daemon), not found on PATH","oama (OAuth Anywhere Mail Agent), not found on PATH","Custom shell command","Store raw in the configuration (plaintext, NOT recommended)"]}}'
question '{"type":"question","question":{"id":5,"kind":"confirm","label":"The secret will be saved in plaintext in the configuration file. Keychain or a token broker is recommended. Continue?","default":"false","choices":[]}}'
question '{"type":"question","question":{"id":6,"kind":"password","label":"Gmail access token:","default":null,"choices":[]}}'
question '{"type":"question","question":{"id":7,"kind":"password","label":"Confirm Gmail access token:","default":null,"choices":[]}}'
printf '%s\n' '{"type":"complete","name":"dummy","document":"[accounts.dummy]\ngmail.user-id = \"me\"\ngmail.auth.token.raw = \"BROWSER_DUMMY_SECRET_MARKER\"\n"}'
