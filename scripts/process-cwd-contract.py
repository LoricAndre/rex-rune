#!/usr/bin/env python3
"""Check source-shell CWD inheritance against an isolated Rex server.

Adapted from kitten-lily/rex-rune@c27bb62. Requires rex. Set REX_BIN to a built
client to also test actual keyboard tab/split creation and pane-menu targeting
(requires Xvfb, openbox, xdotool and ImageMagick). Without REX_BIN this checks
only the server contract, not the client. Logs/captures stay in the printed
~/.cache/agent-ws directory. Uses its own HOME, socket and process PIDs only.
"""
import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
REX = shutil.which("rex")
assert REX, "rex required"
BINARY = str(Path(os.environ["REX_BIN"]).resolve()) if os.environ.get("REX_BIN") else None
base = Path.home() / ".cache/agent-ws"
base.mkdir(parents=True, exist_ok=True)
work = Path(tempfile.mkdtemp(prefix="rex-cwd-contract-", dir=base))
for name in ("home", "config", "state", "runtime", "cache", "captures", "source project", "split project"):
    (work / name).mkdir(mode=0o700)
for name in list(os.environ):
    if name.startswith(("REX_", "HERDR_")) or name in ("DBUS_SESSION_BUS_ADDRESS", "DISPLAY", "WAYLAND_DISPLAY"):
        del os.environ[name]
SERVER = "unix://" + str(work / "rex.sock")
os.environ.update(HOME=str(work / "home"), XDG_CONFIG_HOME=str(work / "config"),
                  XDG_STATE_HOME=str(work / "state"), XDG_RUNTIME_DIR=str(work / "runtime"),
                  XDG_CACHE_HOME=str(work / "cache"), REX_SERVER=SERVER,
                  REX_CAPTURE_OUT=str(work / "captures"))
REX_ARGS = [REX, "--autostart=false", "--server", SERVER]
TERMINAL = "com.superlogical.terminal"
# No rc files or OSC 7: inheritance must come from the server's process inspection.
SHELL = {"flavor": TERMINAL + ".shell", "options": {"shell": "/bin/sh"}}


def api(method, **payload):
    return json.loads(subprocess.check_output(REX_ARGS + ["api", "call", method, json.dumps(payload)],
                                             text=True, timeout=12))


def wait(predicate, description, timeout=10):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if value := predicate():
            return value
        time.sleep(.1)
    raise AssertionError(description)


def shell_cwd(session, block):
    reply = api(TERMINAL + ".process", session_id=session, block_id=block, args={})
    return (reply.get("child") or {}).get("cwd")


def create(label):
    reply = api("session.create", label=label, initial_windows=[{"layout": {"block": SHELL}}])
    return reply["session_id"], reply["initial_windows"][0]["block_ids"][0]


def split(session, anchor, cwd):
    layout = {"block": {**SHELL, "options": {**SHELL["options"], "cwd": cwd}}}
    reply = api("session.new_split", session_id=session, anchor_id=anchor, direction="horizontal",
                side="after", layout=layout, focus=False)
    return reply["block_ids"][0]


server = subprocess.Popen([REX, "server", "run", "--listen", SERVER, "--restore=false",
                           "--config", "/dev/null", "--state-dir", str(work / "server-state")],
                          stdout=(work / "server.log").open("w"), stderr=subprocess.STDOUT)
desk = None
try:
    print("Private CWD artifacts:", work, flush=True)
    wait(lambda: (work / "rex.sock").exists(), "private server did not start")
    status = api("server.status")
    (work / "server-status.json").write_text(json.dumps(status, indent=2))
    print("Server API revision:", status["api_revision"], flush=True)
    session, first = create("cwd-contract")
    home = str((work / "home").resolve())
    source = str(work / "source project")
    other = str(work / "split project")
    wait(lambda: shell_cwd(session, first) == home, "the first shell starts at home")
    subprocess.check_call(REX_ARGS + ["send", "-s", session, "-b", first,
                                    "cd " + shlex.quote(source) + "\n"])
    wait(lambda: shell_cwd(session, first) == source, "process reports cd without OSC 7")
    assert not api(TERMINAL + ".pwd", session_id=session, block_id=first, args={}).get("pwd"), \
        "the shell unexpectedly reported OSC 7"
    inherited = split(session, first, source)
    wait(lambda: shell_cwd(session, inherited) == source, "a split starts in its cwd")
    missing = split(session, first, str(work / "gone"))
    wait(lambda: shell_cwd(session, missing) == home, "missing cwd falls back to home")
    print("PASS server process/cwd contract, paths with spaces and missing-directory fallback", flush=True)

    if BINARY:
        for tool in ("Xvfb", "openbox", "xdotool", "import"):
            assert shutil.which(tool), f"{tool} required"
        os.environ["REX_BIN"] = BINARY
        spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
        accept = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(accept)
        accept.api = api
        session, first = create("cwd-client")
        config = work / "workspace.json"
        config.write_text(json.dumps({"hosts": [{"label": "private", "endpoint": SERVER,
                                                "shell": "/bin/sh"}],
                                      "notifications": {"system_notifications": False}}))
        desk = accept.Desktop()
        app, window = desk.launch(session, config, size="1200x800", log=(work / "client.log").open("w"))

        def view():
            return api("session.view", session_id=session)

        def focused():
            v = view()
            return next(w["focused_block_id"] for w in v["windows"]
                        if w["window_id"] == v["active_window_id"])

        def changed_from(previous):
            current = focused()
            return current if current != previous else None

        desk.type("cd " + shlex.quote(source))
        desk.key("Return")
        wait(lambda: shell_cwd(session, first) == source, "client shell changed directory")
        desk.key("ctrl+shift+d")
        second = wait(lambda: changed_from(first), "client split was not created")
        wait(lambda: shell_cwd(session, second) == source, "client split did not inherit source cwd")
        desk.type("cd " + shlex.quote(other))
        desk.key("Return")
        wait(lambda: shell_cwd(session, second) == other, "split changed directory")
        # Focus is on the right pane. Right-click the left pane; its menu must keep that target.
        desk.x("mousemove", "--window", window, 200, 300)
        desk.x("click", 3)
        time.sleep(.3)
        desk.capture(window, "source-pane-menu")
        desk.key("Return")  # First pane-menu row: Split right.
        third = wait(lambda: (current := focused()) not in (first, second) and current,
                     "pane-menu split was not created")
        wait(lambda: shell_cwd(session, third) == source, "pane menu inherited the wrong pane's cwd")
        desk.type("cd " + shlex.quote(other))
        desk.key("Return")
        wait(lambda: shell_cwd(session, third) == other, "menu split changed directory")
        desk.key("ctrl+shift+t")
        fourth = wait(lambda: changed_from(third), "client tab was not created")
        wait(lambda: shell_cwd(session, fourth) == other, "new tab did not inherit focused shell cwd")
        assert len(view()["windows"]) == 2, "creation duplicated a tab"
        assert shell_cwd(session, first) == source, "creation changed the source shell"
        desk.capture(window, "inherited-tab")
        print("PASS client keyboard split, captured pane-menu split and new-tab CWD inheritance", flush=True)
    else:
        print("SKIP client acceptance: set REX_BIN to a built client", flush=True)
except Exception:
    if desk and "window" in locals():
        desk.capture(window, "failure")
    raise
finally:
    if desk:
        desk.close()
        for process in [*desk.apps, desk.wm, desk.server]:
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait(timeout=5)
    server.terminate()
    try:
        server.wait(timeout=10)
    except subprocess.TimeoutExpired:
        server.kill()
        server.wait(timeout=5)
