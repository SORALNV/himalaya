#!/usr/bin/env python3
"""Exercise TERM/HUP only against an explicitly isolated test executable."""
import argparse
import os
import selectors
import signal
import subprocess
import time

parser = argparse.ArgumentParser()
parser.add_argument("--test-binary", required=True)
args = parser.parse_args()
for termination in (signal.SIGTERM, signal.SIGHUP):
    process = subprocess.Popen([args.test_binary, "wizard::web::tests::termination_fixture", "--ignored", "--exact", "--nocapture"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    worker = None
    try:
        selector = selectors.DefaultSelector()
        selector.register(process.stdout, selectors.EVENT_READ)
        deadline = time.monotonic() + 8
        ready = False
        pending = b""
        while time.monotonic() < deadline:
            if not selector.select(0.2):
                continue
            chunk = os.read(process.stdout.fileno(), 4096)
            pending += chunk
            while b"\n" in pending:
                line, pending = pending.split(b"\n", 1)
                if line.startswith(b"TERMINATION_FIXTURE_READY "):
                    worker = int(line.split()[-1])
                    ready = True
            if ready:
                break
            if not chunk and process.poll() is not None:
                break
        assert ready, "isolated signal fixture did not become ready"
        process.send_signal(termination)
        output, errors = process.communicate(timeout=5)
        assert process.returncode == 0, (output, errors)
        assert b"TERMINATION_FIXTURE_SETTLED" in output
        try:
            os.kill(worker, 0)
        except ProcessLookupError:
            pass
        else:
            raise AssertionError("owned worker survived graceful termination")
        print(f"{termination.name}: PASS")
    finally:
        if process.poll() is None:
            if worker is not None:
                try:
                    os.killpg(worker, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            process.kill()
            process.wait(timeout=5)
