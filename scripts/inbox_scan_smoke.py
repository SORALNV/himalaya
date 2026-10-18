#!/usr/bin/env python3
"""Check bounded UID scans against synthetic, read-only loopback IMAP."""

import argparse
import datetime as dt
import json
from pathlib import Path
import re
import socketserver
import subprocess
import tempfile
import threading
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path(__file__).resolve().parents[1] / "target/debug/himalaya")
    binary = parser.parse_args().binary.resolve(strict=True)
    now = dt.datetime.now(dt.timezone.utc).replace(microsecond=0)
    state = {
        "validity": 17,
        "rows": {1: now - dt.timedelta(minutes=10), 5: now - dt.timedelta(hours=2), 9: now - dt.timedelta(minutes=20), 20: now - dt.timedelta(minutes=40), 21: now - dt.timedelta(minutes=5)},
        "missing": None,
        "omit_date": None,
        "subject": "fixture",
        "commands": [],
        "errors": [],
    }

    class Handler(socketserver.StreamRequestHandler):
        def handle(self):
            self.connection.settimeout(5)
            self.wfile.write(b"* PREAUTH [CAPABILITY IMAP4rev1] fixture\r\n")
            while line := self.rfile.readline():
                parts = line.decode().strip().split()
                state["commands"].append(parts[1:])
                tag, command = parts[:2]
                command = command.upper()
                if command == "CAPABILITY":
                    self.wfile.write(b"* CAPABILITY IMAP4rev1\r\n")
                elif command == "EXAMINE":
                    self.wfile.write(f'* {len(state["rows"])} EXISTS\r\n* FLAGS (\\Seen)\r\n* OK [UIDVALIDITY {state["validity"]}] valid\r\n* OK [UIDNEXT {max(state["rows"], default=0) + 1}] next\r\n'.encode())
                elif command == "UID" and parts[2].upper() == "SEARCH":
                    match = re.search(r"\bUID (\d+)(?::(\d+))?", line.decode())
                    assert match, "search must contain a bounded UID range"
                    low = int(match[1])
                    high = int(match[2] or match[1])
                    # Deliberately wider than the real date search to exercise exact filtering.
                    ids = " ".join(str(uid) for uid in sorted(state["rows"]) if low <= uid <= high)
                    self.wfile.write(f"* SEARCH {ids}\r\n".encode())
                elif command == "UID" and parts[2].upper() == "FETCH":
                    assert "BODY" not in line.decode().upper(), "no body fetch"
                    requested = []
                    for part in parts[3].split(","):
                        bounds = list(map(int, part.split(":")))
                        requested.extend(range(bounds[0], bounds[-1] + 1))
                    for uid in requested:
                        if uid == state["missing"] or uid not in state["rows"]:
                            continue
                        date = state["rows"][uid].strftime("%d-%b-%Y %H:%M:%S %z")
                        internal_date = "" if uid == state["omit_date"] else f'INTERNALDATE "{date}" '
                        sequence = sorted(state["rows"]).index(uid) + 1
                        self.wfile.write(f'* {sequence} FETCH (UID {uid} FLAGS () {internal_date}RFC822.SIZE 100 ENVELOPE ("Mon, 01 Jan 2001 00:00:00 +0000" "{state["subject"]}" NIL NIL NIL NIL NIL NIL NIL "<fixture-{uid}@example.test>"))\r\n'.encode())
                elif command == "LOGOUT":
                    self.wfile.write(b"* BYE fixture\r\n")
                else:
                    state["errors"].append(command)
                self.wfile.write(f"{tag} OK fixture\r\n".encode())
                self.wfile.flush()
                if command == "LOGOUT":
                    return

    class Server(socketserver.ThreadingTCPServer):
        daemon_threads = True

    checks = {}
    with Server(("127.0.0.1", 0), Handler) as server, tempfile.TemporaryDirectory(prefix="himalaya-scan-") as directory:
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        config = Path(directory) / "fixture.toml"
        config.write_text("\n".join(f'[accounts.{name}]\nimap.server = "imap://127.0.0.1:{server.server_address[1]}"\nimap.starttls = false\n' for name in ["alpha", "beta"]))

        def run(*args, account="alpha"):
            result = subprocess.run([str(binary), "--json", "--log-level", "off", "-c", str(config), "-a", account, "inbox", "scan", *args], capture_output=True, timeout=10)
            assert result.returncode == 0, result.stderr.decode(errors="replace")
            value = json.loads(result.stdout)
            assert len(result.stdout) <= 131072
            return value

        first = run("--since", "30m", "--page-size", "2", "--max-pages", "1")
        checks["bounded_prefix"] = not first["complete"] and first["scanned"] == 2 and [e["envelope"]["id"] for e in first["entries"]] == ["1"]
        checks["fixed_replay"] = run("--cursor", first["cursor"], "--page-size", "2", "--max-pages", "1") == first
        state["rows"][50] = now
        second = run("--cursor", first["next_cursor"], "--page-size", "2", "--max-pages", "1")
        third = run("--cursor", second["next_cursor"], "--page-size", "2", "--max-pages", "1")
        checks["resume_past_old_receipts"] = second["scanned"] == 2 and [e["envelope"]["id"] for e in second["entries"]] == ["9"] and third["complete"] and [e["envelope"]["id"] for e in third["entries"]] == ["21"]
        checks["snapshot_excludes_late_uid"] = third["snapshot_uid"] == 21 and third["checkpoint"] is not None
        assert third["complete"], third
        delta = run("--after", third["checkpoint"])
        checks["incremental_new_uid_only"] = delta["complete"] and [e["envelope"]["id"] for e in delta["entries"]] == ["50"]
        checks["no_new_uid_avoids_reversed_range"] = run("--after", delta["checkpoint"])["scanned"] == 0
        historical = run("--since", "30m", "--as-of", (now - dt.timedelta(days=2)).isoformat())
        state["rows"][60] = now - dt.timedelta(days=1)
        recovered = run("--after", historical["checkpoint"])
        checks["downtime_extends_receipt_window"] = recovered["complete"] and "60" in [e["envelope"]["id"] for e in recovered["entries"]] and recovered["since"] == historical["as_of"]
        checks["historical_checkpoint_preserves_later_receipts"] = historical["deferred"] and "1" in [e["envelope"]["id"] for e in recovered["entries"]]
        checks["scope_is_account_specific"] = run("--cursor", first["cursor"], account="beta")["error"]["code"] == "CURSOR_SCOPE_CHANGED"
        checks["window_cannot_change"] = run("--cursor", first["cursor"], "--since", "6h")["error"]["code"] == "INVALID_CURSOR"
        checks["checkpoint_kind_required"] = run("--after", first["cursor"])["error"]["code"] == "INVALID_CURSOR"
        checks["invalid_cursor_safe"] = run("--cursor", "fake-secret-marker!")["error"]["code"] == "INVALID_CURSOR"
        state["validity"] += 1
        checks["uidvalidity_fail_closed"] = run("--after", delta["checkpoint"])["error"]["code"] == "UIDVALIDITY_CHANGED"
        state["validity"] -= 1
        state["missing"] = 9
        incomplete = run("--cursor", first["next_cursor"])
        checks["missing_row_no_checkpoint"] = not incomplete["complete"] and incomplete["checkpoint"] is None and incomplete["reason"] == "MISSING_FETCH_ROW" and not incomplete["entries"]
        state["missing"] = None
        state["omit_date"] = 9
        checks["missing_internaldate_no_fallback"] = run("--cursor", first["next_cursor"])["reason"] == "MISSING_RECEIVED_AT"
        state["omit_date"] = None
        state["subject"] = "x" * 10000
        oversized = run("--since", "30m", "--max-bytes", "8192")
        checks["oversized_entry_does_not_advance"] = oversized["reason"] == "ENTRY_TOO_LARGE" and oversized["scanned"] == 0 and oversized["cursor"] == oversized["next_cursor"] and not oversized["entries"]
        state["subject"] = "x" * 2000
        bounded = run("--since", "6h", "--max-bytes", "8192")
        checks["output_budget_retains_prefix"] = bounded["reason"] == "OUTPUT_BUDGET" and 0 < bounded["scanned"] < len(state["rows"]) and len(json.dumps(bounded, indent=2).encode()) <= 8192
        state["subject"] = "fixture"
        state["rows"][80] = dt.datetime.now(dt.timezone.utc) + dt.timedelta(seconds=1)
        future = run("--since", "30m", "--page-size", "1", "--max-pages", "1")
        while not future["complete"]:
            future = run("--cursor", future["next_cursor"], "--page-size", "1", "--max-pages", "1")
        checks["future_floor_survives_continuations"] = future["deferred"]
        time.sleep(1.2)
        eligible = run("--after", future["checkpoint"])
        checks["future_receipt_revisited_after_wait"] = eligible["complete"] and "80" in [e["envelope"]["id"] for e in eligible["entries"]]
        state["rows"][90] = now + dt.timedelta(days=10)
        distant = run("--after", eligible["checkpoint"])
        checks["distant_future_not_hidden_by_upper_date_search"] = distant["complete"] and distant["deferred"] and distant["scanned"] == 1 and not distant["entries"]
        error = run("--since", "30m", "--max-bytes", "8192", account="x" * 10000)
        checks["error_response_byte_bound"] = error["error"]["code"] == "CONFIG_ERROR" and len(json.dumps(error, indent=2).encode()) <= 8192
        checks["server_internal_date_search"] = all("SINCE" in c and "BEFORE" not in c and "SENTSINCE" not in c for c in state["commands"] if c[:2] == ["UID", "SEARCH"])
        checks["read_only_protocol"] = not state["errors"] and not any(c[0] in ["SELECT", "STORE", "APPEND", "EXPUNGE", "COPY", "MOVE"] for c in state["commands"])
        server.shutdown()
        worker.join(2)
    print(json.dumps(checks, indent=2))
    if not all(checks.values()):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
