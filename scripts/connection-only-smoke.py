#!/usr/bin/env python3
"""A missing/refused default socket must never cause the client to execute Rex.

Build rex-probe first. Isolated runtime and a sentinel rex on PATH; no real server
is started, stopped, or reconfigured by this regression check.
"""
import os
from pathlib import Path
import socket
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PROBE = Path(os.environ.get("REX_PROBE", ROOT / "target/debug/rex-probe")).resolve()


def main():
    with tempfile.TemporaryDirectory(prefix="rex-client-only-") as directory:
        work = Path(directory)
        runtime = work / "runtime"
        runtime.mkdir(mode=0o700)
        sock = runtime / "rex/server.sock"
        sock.parent.mkdir()
        marker = work / "rex-was-executed"
        fake = work / "rex"
        fake.write_text('#!/bin/sh\nprintf invoked > "$REX_TEST_MARKER"\nexit 90\n')
        fake.chmod(0o755)
        env = dict(os.environ, XDG_RUNTIME_DIR=str(runtime), PATH=str(work), REX_TEST_MARKER=str(marker))
        env.pop("REX_SERVER", None)
        for stale in [False, True]:
            if stale:
                with socket.socket(socket.AF_UNIX) as listener:
                    listener.bind(str(sock))
            result = subprocess.run([str(PROBE)], env=env, capture_output=True, text=True, timeout=15)
            assert result.returncode != 0, "Unexpected server connection"
            expected = "Connection refused" if stale else "No such file or directory"
            assert expected in result.stderr, result.stderr
            assert not marker.exists(), "The client executed the Rex CLI"
            assert sock.exists() == stale, "The client changed the server socket"
            print(f"PASS {'stale' if stale else 'missing'} default socket reports connection failure without executing Rex")


if __name__ == "__main__":
    main()
