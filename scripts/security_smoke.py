#!/usr/bin/env python3
"""Verify security fixes against a built binary using only temporary fake mail."""

import argparse
import base64
import json
import os
from pathlib import Path
import pty
import select
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=Path(__file__).resolve().parents[1] / "target/debug/himalaya")
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    results = {}

    with tempfile.TemporaryDirectory(prefix="himalaya-security-smoke-") as temporary:
        root = Path(temporary)

        def run(arguments, data=None):
            return subprocess.run([str(binary), *map(str, arguments)], input=data, capture_output=True, timeout=15)

        def check(name, condition):
            results[name] = "PASS" if condition else "FAIL"

        for subdirectory in ["cur", "new", "tmp"]:
            (root / "mail/inbox" / subdirectory).mkdir(parents=True)
            (root / "wizard-mail" / subdirectory).mkdir(parents=True)
        config = root / "account.toml"
        config.write_text(f'[accounts.audit]\ndefault = true\nemail = "audit@example.test"\nmaildir.root = "{root}/mail"\n')

        def mail(name, raw):
            (root / "mail/inbox/cur" / f"{name}:2,S").write_bytes(raw)

        controls = b"\x1b[2J\x1b]52;c;YXVkaXQ=\x07"
        raw = b"From: sender@example.test\r\nSubject: audit\r\nContent-Type: text/plain\r\nContent-Transfer-Encoding: base64\r\n\r\n" + base64.b64encode(controls) + b"\r\n"
        eml = root / "message.eml"
        eml.write_bytes(raw)
        mail("controls", raw)
        for command in ["parse", "read"]:
            arguments = ["message", "parse", eml] if command == "parse" else ["-c", config, "message", "read", "controls"]
            output = run(arguments)
            check(f"safe_message_{command}", output.returncode == 0 and b"\x1b" not in output.stdout and b"\x07" not in output.stdout)
        output = run(["--json", "message", "parse", eml])
        check("json_preserves_decoded_body", output.returncode == 0 and json.loads(output.stdout)["text"].encode() == controls)
        output = run(["-c", config, "message", "read", "controls", "--raw"])
        check("explicit_raw_preserves_bytes", output.returncode == 0 and output.stdout == raw)

        attachment = b"From: sender@example.test\r\nMIME-Version: 1.0\r\nContent-Type: multipart/mixed; boundary=\"audit\"\r\n\r\n--audit\r\nContent-Type: text/plain\r\n\r\nbody\r\n--audit\r\nContent-Type: application/octet-stream\r\nContent-Disposition: attachment; filename=\"report.txt\"\r\n\r\nATTACHMENT_PAYLOAD\r\n--audit--\r\n"
        mail("attachment", attachment)
        downloads = root / "downloads"
        downloads.mkdir()
        outside = root / "outside.txt"
        (downloads / "report.txt").symlink_to(outside)
        output = run(["-c", config, "attachment", "download", "attachment", "--dir", downloads])
        check("dangling_symlink_not_followed", output.returncode == 0 and not outside.exists() and (downloads / "report (1).txt").read_bytes() == b"ATTACHMENT_PAYLOAD")
        full = root / "occupied"
        full.mkdir()
        (full / "report.txt").write_bytes(b"ORIGINAL")
        for number in range(1, 1024):
            (full / f"report ({number}).txt").write_bytes(b"ORIGINAL")
        output = run(["-c", config, "attachment", "download", "attachment", "--dir", full])
        check("exhausted_names_do_not_overwrite", output.returncode != 0 and all(path.read_bytes() == b"ORIGINAL" for path in full.iterdir()))

        helper = root / "helper.toml"
        helper.write_text('[accounts.audit]\ndefault = true\nimap.server = "imaps://127.0.0.1:1"\nimap.sasl.plain.username = "audit"\nimap.sasl.plain.password.command = ["/bin/sh", "-c", "printf SECRET_SMOKE_MARKER; printf STDERR_SMOKE_MARKER >&2; exit 7"]\n')
        for flags in [[], ["--json"], ["--log-level", "trace"]]:
            output = run(["-c", helper, *flags, "imap", "list"])
            emitted = output.stdout + output.stderr
            label = "plain" if not flags else flags[-1].lstrip("-")
            check(f"failed_helper_redacted_{label}", output.returncode != 0 and b"SECRET_SMOKE_MARKER" not in emitted and b"STDERR_SMOKE_MARKER" not in emitted)

        def terminal(arguments, wizard=False):
            master, slave = pty.openpty()
            process = subprocess.Popen([str(binary), *map(str, arguments)], stdin=slave, stdout=slave, stderr=slave, preexec_fn=lambda: os.umask(0o022))
            os.close(slave)
            captured = b""
            answered = saved = False
            deadline = time.monotonic() + 15
            try:
                while time.monotonic() < deadline:
                    if select.select([master], [], [], 0.1)[0]:
                        try:
                            block = os.read(master, 65536)
                        except OSError:
                            break
                        if not block:
                            break
                        captured += block
                        if b"\x1b[6n" in block:
                            os.write(master, b"\x1b[1;1R")
                        if wizard and not answered and b"Email:" in captured:
                            os.write(master, str(root / "wizard-mail").encode() + b"\r")
                            answered = True
                        if wizard and not saved and (b"Save this account" in captured or b"Append account" in captured):
                            os.write(master, b"\r")
                            saved = True
                    if process.poll() is not None:
                        break
                if process.poll() is None:
                    process.terminate()
                process.wait(timeout=5)
                return process.returncode, captured
            finally:
                if process.poll() is None:
                    process.kill()
                    process.wait(timeout=5)
                os.close(master)

        code, emitted = terminal(["message", "parse", eml])
        check("tty_does_not_emit_mail_control_sequences", code == 0 and controls not in emitted and b"\x1b]52;c;" not in emitted and b"\x1b[2J" not in emitted)
        generated = root / "private/config.toml"
        code, _ = terminal(["-c", generated, "configure"], wizard=True)
        check("wizard_creates_private_config", code == 0 and generated.exists() and generated.stat().st_mode & 0o777 == 0o600 and generated.parent.stat().st_mode & 0o777 == 0o700)
        if generated.exists():
            previous = generated.read_bytes()
            generated.chmod(0o644)
            code, _ = terminal(["-c", generated, "configure"], wizard=True)
            check("wizard_append_restricts_config", code == 0 and generated.stat().st_mode & 0o777 == 0o600 and generated.read_bytes().startswith(previous) and len(generated.read_bytes()) > len(previous))
        else:
            check("wizard_append_restricts_config", False)

    print(json.dumps(results, indent=2))
    return 0 if all(value == "PASS" for value in results.values()) else 1


if __name__ == "__main__":
    raise SystemExit(main())
