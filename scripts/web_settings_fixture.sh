#!/bin/sh
# Synthetic typed worker for the isolated Rust browser fixture only.
# No network, credentials, configuration reads, or provider authentication.
set -eu
exec python3 -c '
import json, sys, time
request = json.loads(sys.stdin.readline())
if request["action"] == "discover":
    time.sleep(0.4)
    data = {"choices": [
        {"backend": "imap", "label": "IMAP / SMTP · mail.example.test", "receive": {"host": "imap.example.test", "port": 993, "encryption": "tls", "login": "dummy@example.test"}, "smtp": {"host": "smtp.example.test", "port": 587, "encryption": "starttls", "login": "dummy@example.test"}},
        {"backend": "jmap", "label": "JMAP", "server": "https://mail.example.test/jmap"},
        {"backend": "gmail", "label": "Gmail API"},
        {"backend": "msgraph", "label": "Microsoft Graph"}
    ]}
else:
    time.sleep(0.4)
    data = {"ok": True}
print(json.dumps({"type": "data", "data": data}))
'
