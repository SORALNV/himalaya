#!/usr/bin/env python3
"""Exercise fork-only inbox routing with synthetic Maildir and loopback IMAP."""

import argparse
import datetime
import json
from pathlib import Path
import socket
import subprocess
import tempfile
import threading


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path(__file__).resolve().parents[1] / "target/debug/himalaya")
    binary = parser.parse_args().binary.resolve(strict=True)
    checks = {}
    with tempfile.TemporaryDirectory(prefix="himalaya-inbox-") as directory:
        root = Path(directory)
        for account in ["alpha", "beta"]:
            for folder in ["cur", "new", "tmp"]:
                (root / account / "inbox" / folder).mkdir(parents=True)
            (root / account / "inbox/new/same-id").write_bytes(b"From: fake@example.test\r\nSubject: fixture\x1b[2J\r\nDate: Sun, 11 Oct 2026 00:00:00 +0000\r\n\r\nFAKE\r\n")
        config = root / "local.toml"
        config.write_text("\n".join(f'[accounts.{a}]\nmaildir.root = "{root / a}"' for a in ["alpha", "beta"]))

        def run(arguments, path=config, json_output=True):
            result = subprocess.run([str(binary), "-c", str(path), *(["--json"] if json_output else []), *arguments], capture_output=True, timeout=20)
            if result.returncode:
                raise AssertionError(result.stdout.decode(errors="replace"))
            return json.loads(result.stdout) if json_output else result.stdout

        output = run(["inbox", "list"])
        checks["merge_preserves_colliding_ids"] = output["complete"] and len(output["entries"]) == 2 and {e["account"] for e in output["entries"]} == {"alpha", "beta"}
        checks["selected_account"] = len(run(["-a", "beta", "inbox", "list"])["entries"]) == 1
        output = run(["inbox", "list", "--since", "30m"])
        checks["missing_receipt_not_sender_fallback"] = not output["complete"] and not output["entries"] and all(a["missing_received_at"] == 1 for a in output["accounts"])
        output = run(["inbox", "list", "--page-size", "1", "--max-pages", "1"])
        checks["bound_is_explicit"] = not output["complete"] and all(not a["exhausted"] for a in output["accounts"])
        checks["plain_sanitizes_controls"] = b"\x1b" not in run(["inbox", "list"], json_output=False)
        checks["does_not_mark_seen"] = all((root / a / "inbox/new/same-id").exists() for a in ["alpha", "beta"])
        with config.open("a") as file:
            file.write('\n[inbox]\nsince = "30m"\n')
        checks["configured_default"] = not run(["inbox", "list"])["entries"]
        checks["all_time_overrides_default"] = len(run(["inbox", "list", "--all-time"])["entries"]) == 2
        invalid_default = root / "invalid-default.toml"
        invalid_default.write_text(config.read_text().replace('since = "30m"', 'since = "invalid"'))
        checks["cli_overrides_invalid_default"] = run(["inbox", "list", "--since", "30m"], invalid_default)["since"] is not None
        checks["all_time_bypasses_invalid_default"] = len(run(["inbox", "list", "--all-time"], invalid_default)["entries"]) == 2
        unknown = subprocess.run([str(binary), "-c", str(config), "inbox", "list", "-a", "absent"], capture_output=True, timeout=5)
        checks["unknown_account_is_error"] = unknown.returncode > 0
        with config.open("a") as file:
            file.write(f'\n[accounts.broken]\nmaildir.root = "{root / "absent"}"\n')
        output = run(["inbox", "list", "--all-time"])
        checks["failure_keeps_successful_accounts"] = not output["complete"] and len(output["entries"]) == 2 and any(a["error"] for a in output["accounts"])
        for value in ["0m", "-1h", "9999999999999999999d", "１分"]:
            result = subprocess.run([str(binary), "inbox", "list", "--since", value], capture_output=True, timeout=5)
            checks[f"invalid_duration_{value}"] = result.returncode > 0 and b"panicked" not in result.stderr

        # NOTE: no credentials or public server; PREAUTH is synthetic loopback.
        now = datetime.datetime.now(datetime.timezone.utc).replace(microsecond=0)
        received = [now - datetime.timedelta(minutes=10), now - datetime.timedelta(hours=1), now - datetime.timedelta(minutes=20)]
        listener = socket.socket()
        listener.bind(("127.0.0.1", 0))
        listener.listen()
        listener.settimeout(15)
        commands = []
        errors = []
        port = listener.getsockname()[1]

        def server():
            try:
                # Four independently opened command sessions.
                for _ in range(4):
                    connection, _ = listener.accept()
                    with connection:
                        connection.settimeout(15)
                        stream = connection.makefile("rb")
                        connection.sendall(b"* PREAUTH [CAPABILITY IMAP4rev1] fixture\r\n")
                        while True:
                            line = stream.readline()
                            if not line:
                                break
                            commands.append(line)
                            parts = line.decode().strip().split()
                            tag, command = parts[:2]
                            command = command.upper()
                            if command == "CAPABILITY":
                                connection.sendall(b"* CAPABILITY IMAP4rev1\r\n")
                            elif command == "LIST":
                                connection.sendall(b'* LIST (\\Inbox) "/" "INBOX"\r\n')
                            elif command in ["EXAMINE", "SELECT"]:
                                connection.sendall(b"* 3 EXISTS\r\n* FLAGS (\\Seen)\r\n* OK [UIDVALIDITY 1] valid\r\n")
                            elif command == "FETCH":
                                numbers = parts[2].split(":")
                                first, last = int(numbers[0]), int(numbers[-1])
                                for number in range(first, last + 1):
                                    date = received[number - 1].strftime("%d-%b-%Y %H:%M:%S %z")
                                    # Deliberately stale sender Date; sequence order is not receipt order.
                                    row = f'* {number} FETCH (UID {number} FLAGS () INTERNALDATE "{date}" RFC822.SIZE 100 ENVELOPE ("Mon, 01 Jan 2001 00:00:00 +0000" "fixture" NIL NIL NIL NIL NIL NIL NIL "<fixture-{number}@example.test>"))\r\n'
                                    connection.sendall(row.encode())
                            elif command == "LOGOUT":
                                connection.sendall(b"* BYE fixture\r\n")
                            else:
                                errors.append(f"Unexpected command {command}")
                            connection.sendall(f"{tag} OK fixture\r\n".encode())
                            if command == "LOGOUT":
                                break
            except Exception as error:
                errors.append(str(error))
            finally:
                listener.close()

        worker = threading.Thread(target=server, daemon=True)
        worker.start()
        network = root / "network.toml"
        network.write_text(f'[accounts.fixture]\nimap.server = "imap://127.0.0.1:{port}"\nimap.starttls = false\n')
        output = run(["inbox", "list", "--since", "30m", "--page-size", "1"], network)
        checks["receipt_window_scans_past_old_sequence"] = output["complete"] and [e["envelope"]["id"] for e in output["entries"]] == ["1", "3"]
        output = run(["-a", "fixture", "inbox", "list", "--since", "2h"], network)
        checks["selected_receipt_window"] = output["complete"] and len(output["entries"]) == 3
        output = run(["inbox", "list", "--since", "1s"], network)
        checks["empty_recent_window"] = output["complete"] and not output["entries"]
        output = run(["inbox", "list", "--since", "30m", "--max-pages", "1", "--page-size", "1"], network)
        checks["network_truncation_explicit"] = not output["complete"]
        worker.join(5)
        checks["summary_read_only"] = not errors and not any(b"STORE" in c or b"BODY[" in c or b"SELECT " in c for c in commands) and any(b"INTERNALDATE" in c for c in commands)

    print(json.dumps(checks, indent=2))
    if not all(checks.values()):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
