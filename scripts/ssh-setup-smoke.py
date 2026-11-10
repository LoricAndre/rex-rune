#!/usr/bin/env python3
"""Opt-in SSH setup through the real UI, on a private localhost sshd.

Requires sshd (runnable as this user), ssh-keygen, Xvfb, openbox, xdotool and
ImageMagick. Set REX_CLI to a preview.14+ binary and REX_BIN to a built client.
No root, containers, package installs or changes to ~/.ssh are needed. The
private sshd uses a generated key and forced HOME/XDG/PATH under
~/.cache/agent-ws. A test-only ssh shim applies its private config to both the
native Rex start and the client's existing ssh -L path. All processes are
stopped by PID, including the personal server; artifacts remain for inspection.
Never run against a normal SSH host: the endpoint is fixed to this fixture.
"""
import importlib.util
import json
import os
from pathlib import Path
import shlex
import shutil
import signal
import socket
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
REX = str(Path(os.environ["REX_CLI"]).resolve())
BINARY = str(Path(os.environ["REX_BIN"]).resolve())
for tool in ("sshd", "ssh-keygen", "ssh", "Xvfb", "openbox", "xdotool", "import"):
    assert shutil.which(tool), f"{tool} required"
scratch = Path.home() / ".cache/agent-ws"
scratch.mkdir(parents=True, exist_ok=True)
work = Path(tempfile.mkdtemp(prefix="rex-ssh-", dir=scratch))
for directory in ("home", "config", "state", "runtime", "cache", "bin", "remote-bin", "captures", "local-home"):
    (work / directory).mkdir(mode=0o700)
assert len(str(work / "runtime/rex/login-server.sock")) < 104, "scratch path too long for Unix sockets"
for name in ("hostkey", "clientkey"):
    subprocess.run(["ssh-keygen", "-t", "ed25519", "-N", "", "-f", str(work / name)], check=True, stdout=subprocess.DEVNULL)
with socket.socket() as reserved:
    reserved.bind(("127.0.0.1", 0))
    port = reserved.getsockname()[1]


def executable(path, body):
    path.write_text("#!/bin/sh\n" + body + "\n")
    path.chmod(0o700)


# No login shell reads the real user's profile, and no Rex command sees their
# state/runtime/config directories. Log commands to verify attachment stays read-only.
executable(work / "force", f"""export HOME={shlex.quote(str(work / 'home'))}
export XDG_CONFIG_HOME={work}/config XDG_STATE_HOME={work}/state XDG_RUNTIME_DIR={work}/runtime XDG_CACHE_HOME={work}/cache
export PATH={work}/remote-bin:/usr/bin:/bin
unset REX_SERVER REX_CONFIG DBUS_SESSION_BUS_ADDRESS
printf '%s\\n' "$SSH_ORIGINAL_COMMAND" >> {work}/remote-commands.log
exec /bin/sh -c "$SSH_ORIGINAL_COMMAND"
""")
(work / "bin/rex").symlink_to(REX)
(work / "remote-bin/rex").symlink_to(REX)
(work / "sshd.conf").write_text(f"""Port {port}
ListenAddress 127.0.0.1
HostKey {work}/hostkey
PidFile {work}/sshd.pid
AuthorizedKeysFile {work}/clientkey.pub
StrictModes no
PasswordAuthentication no
KbdInteractiveAuthentication no
UsePAM no
AllowUsers {os.environ['USER']}
ForceCommand {work}/force
""")
(work / "client.conf").write_text(f"""Host rex-setup-private
 HostName 127.0.0.1
 Port {port}
 User {os.environ['USER']}
 IdentityFile {work}/clientkey
 IdentitiesOnly yes
 UserKnownHostsFile {work}/known_hosts
 StrictHostKeyChecking accept-new
 BatchMode yes
""")
executable(work / "bin/ssh", f"exec {shlex.quote(shutil.which('ssh'))} -F {work}/client.conf \"$@\"")
# Avoid any real tailnet access, even if a test accidentally opens Add host.
executable(work / "bin/tailscale", "exit 1")
endpoint = "ssh://rex-setup-private"
unix = "unix://" + str(work / "runtime/rex/server.sock")
for key in list(os.environ):
    if key.startswith("REX_") or key in ("DBUS_SESSION_BUS_ADDRESS", "DISPLAY", "WAYLAND_DISPLAY"):
        del os.environ[key]
os.environ.update(HOME=str(work / "local-home"), XDG_CONFIG_HOME=str(work / "config-local"),
                  XDG_STATE_HOME=str(work / "state-local"), XDG_RUNTIME_DIR=str(work / "runtime"),
                  XDG_CACHE_HOME=str(work / "cache"), PATH=f"{work}/bin:" + os.environ["PATH"],
                  REX_SERVER=endpoint, REX_BIN=BINARY, REX_CAPTURE_OUT=str(work / "captures"))


def wait(predicate, message, timeout=40):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        time.sleep(.1)
    raise AssertionError(message)


def status():
    return json.loads(subprocess.check_output([REX, "--autostart=false", "-S", unix,
                                              "server", "status", "--json"], timeout=10))


sshd = subprocess.Popen([shutil.which("sshd"), "-D", "-e", "-f", str(work / "sshd.conf")],
                        stdout=(work / "sshd.log").open("w"), stderr=subprocess.STDOUT)
desk = None
server_pid = None
try:
    time.sleep(.4)
    assert sshd.poll() is None, (work / "sshd.log").read_text()
    spec = importlib.util.spec_from_file_location("accept", ROOT / "scripts/rune-acceptance-x11.py")
    accept = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(accept)
    desk = accept.Desktop()
    (work / "workspace.json").write_text(json.dumps({"hosts":[{"label":"SSH lab", "endpoint":endpoint}]}))
    app = subprocess.Popen([BINARY, "--endpoint", endpoint, "--config", str(work / "workspace.json"),
                            "--window-size", "1200x800"], env=desk.env,
                           stdout=(work / "client.log").open("w"), stderr=subprocess.STDOUT)
    desk.apps.append(app)
    window = desk.x("search", "--sync", "--pid", app.pid)
    desk.x("windowfocus", window)
    wait(lambda: (work / "remote-commands.log").exists(), "SSH lookup never ran")
    time.sleep(3)
    assert not (work / "runtime/rex/server.sock").exists(), "ordinary attachment started a server"
    desk.capture(window, "ssh-missing")

    def preview():
        desk.key("ctrl+shift+p")
        desk.type("Start personal server")
        desk.key("Return")
        time.sleep(.4)

    preview()
    desk.capture(window, "ssh-start-preview")
    desk.x("windowsize", window, 640, 480)
    time.sleep(.4)
    desk.capture(window, "ssh-start-preview-small")
    desk.x("windowsize", window, 1200, 800)
    time.sleep(.4)
    desk.key("Return")  # The safe default is Not now.
    time.sleep(.4)
    assert not (work / "runtime/rex/server.sock").exists(), "default Return started a server"
    # A real SSH preflight must reject an old remote CLI without invoking auto-start.
    remote_rex = work / "remote-bin/rex"
    remote_rex.unlink()
    executable(remote_rex, 'if [ "$1" = version ]; then echo \'{"api_revision":11}\'; else exit 1; fi')
    preview()
    desk.x("mousemove", "--window", window, 755, 540)
    desk.x("click", 1)
    wait(lambda: "rex version --json" in (work / "remote-commands.log").read_text(), "remote version check never ran")
    time.sleep(1)
    assert not (work / "runtime/rex/server.sock").exists(), "old remote Rex was started"
    assert "rex server start" not in (work / "remote-commands.log").read_text()
    desk.x("windowsize", window, 640, 480)
    time.sleep(.4)
    desk.capture(window, "ssh-old-remote-error-small")
    desk.x("windowsize", window, 1200, 800)
    desk.key("Escape")
    remote_rex.unlink()
    remote_rex.symlink_to(REX)
    preview()
    # Explicitly press the named Start button in the fixed 1200x800 fixture.
    # Default Return above must never authorize setup.
    desk.x("mousemove", "--window", window, 755, 540)
    desk.x("click", 1)
    wait(lambda: (work / "runtime/rex/server.sock").exists(), "confirmed start did not create the private socket")
    reply = status()
    server_pid = reply["pid"]
    assert reply["api_revision"] >= 12 and reply["session_count"] == 0
    time.sleep(3)
    desk.capture(window, "ssh-started")
    # Native command used only for setup; the subsequent attachment uses ssh -L.
    commands = (work / "remote-commands.log").read_text()
    assert "cat" in commands.splitlines(), "client did not attach via its socket forward"
    assert reply["session_count"] == 0, "setup created a session"
    print(f"PASS opt-in start, Not now, SSH alias/config, zero sessions, ssh -L attachment: {work}")
finally:
    if desk:
        desk.close()
    # Read only our explicit socket, never a CLI default or the user's server.
    if server_pid is None and (work / "runtime/rex/server.sock").exists():
        server_pid = status()["pid"]
    if server_pid is not None:
        try:
            os.kill(server_pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
    sshd.terminate()
    sshd.wait(timeout=10)
