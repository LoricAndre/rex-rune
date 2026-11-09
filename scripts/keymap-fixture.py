#!/usr/bin/env python3
"""Record client.keymap via rex-probe against an isolated rev-11 server.

Usage: python3 scripts/keymap-fixture.py [path/to/rex-probe]
The reply is intentionally raw, including the temporary config path and instance ID.
No default socket, saved sessions, or tailnet listener is used.
"""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests/fixtures/keymap"
PROBE = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else ROOT / "target/debug/rex-probe"

with tempfile.TemporaryDirectory(prefix="rex-keymap-") as directory:
    work = Path(directory)
    socket = work / "rex.sock"
    endpoint = f"unix://{socket}"
    env = dict(os.environ, REX_SERVER=endpoint, REX_KEYMAP_CAPTURE=str(FIXTURE),
               REX_EXPECT_REVISION="11")
    for key in ("HOME", "XDG_CONFIG_HOME", "XDG_STATE_HOME", "XDG_RUNTIME_DIR", "XDG_CACHE_HOME"):
        folder = work / key.lower()
        folder.mkdir(mode=0o700)
        env[key] = str(folder)
    config = work / "init.lua"
    config.write_bytes((FIXTURE / "init.lua").read_bytes())
    with (work / "server.log").open("w+") as log:
        server = subprocess.Popen(
            ["rex", "server", "run", "--listen", endpoint, "--restore=false",
             "--config", str(config), "--state-dir", str(work / "server-state")],
            env=env, stdout=log, stderr=log)
        try:
            for _ in range(100):
                if socket.exists():
                    break
                if server.poll() is not None:
                    raise RuntimeError("disposable Rex server exited")
                time.sleep(0.05)
            else:
                raise RuntimeError("disposable Rex socket did not appear")
            subprocess.run([str(PROBE), endpoint], env=env, check=True)
        except Exception:
            log.seek(0)
            print(log.read(), file=sys.stderr)
            raise
        finally:
            server.terminate()
            server.wait(timeout=10)
