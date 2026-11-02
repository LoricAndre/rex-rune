#!/usr/bin/env python3
"""Isolated real-server contract checks; never connects to the user's Rex server.

Requires Python websockets. Pass a revision-7 binary with --server to check the
compatibility handshake. Optional --probe runs Rex Linux's controller probe.
"""
import argparse
import base64
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time

from websockets.sync.client import unix_connect

PROTOCOL = "rex.data.terminal.v1"
SELECTOR = {"creator_name": "com.superlogical.terminal", "data_ref_id": "terminal", "subprotocol": PROTOCOL}
SHELL = {"block": {"flavor": "com.superlogical.terminal.shell"}}


class Control:
    def __init__(self, sock, token=None):
        self.ws = unix_connect(str(sock), uri="ws://localhost/api/control", subprotocols=["rex.control.v1"])
        hello = {"type": "hello", "api_revision": 8, "api_min_revision": 7,
                 "client": {"kind": "app", "name": "revision8-check"}}
        if token:
            hello["reconnect_token"] = token
        self.ws.send(json.dumps(hello))
        self.welcome = json.loads(self.ws.recv(timeout=8))
        assert self.welcome["type"] == "welcome"
        self.seq = 0
        self.events = []

    def call(self, method, **payload):
        self.seq += 1
        self.ws.send(json.dumps({"type": "request", "id": str(self.seq), "method": method, "payload": payload}))
        while True:
            message = json.loads(self.ws.recv(timeout=8))
            if message["type"] == "event":
                self.events.append(message)
            elif message["type"] == "response":
                assert message["id"] == str(self.seq)
                return message

    def ok(self, method, **payload):
        reply = self.call(method, **payload)
        assert "error" not in reply, reply
        return reply["payload"]

    def close(self):
        self.ws.close()


def checks(sock, revision):
    controls = []

    def control(token=None):
        c = Control(sock, token)
        controls.append(c)
        assert c.welcome["api_revision"] == revision
        return c

    def bootstrap(**payload):
        c = control()
        return c, c.ok("session.bootstrap", event_scope="session", data_tickets=[SELECTOR], **payload)

    def handshake(ticket, protocol=PROTOCOL):
        with unix_connect(str(sock), uri="ws://localhost/api/data", subprotocols=[protocol]) as ws:
            ws.send(b"\x01" + base64.urlsafe_b64decode(ticket + "=="))
            return ws.recv(timeout=8)

    try:
        admin = control()
        if revision == 7:
            admin.ok("session.attach_all", event_scope="session")
            print("PASS revision-7 server negotiates 7 from range 7–8", flush=True)
            return
        _, empty = bootstrap(attach_session_ids=[])
        assert not empty["attach_all"]["sessions"] and not empty.get("data_tickets")
        sid = admin.ok("session.create", label="revision8-fixture", initial_windows=[
            {"window_label": "one", "layout": SHELL}, {"window_label": "two", "layout": SHELL}])["session_id"]
        admin.ok("session.attach", session_id=sid)
        view = admin.ok("session.view", session_id=sid)
        first, second = view["windows"]
        admin.ok("session.focus_window", session_id=sid, window_id=first["window_id"])
        admin.ok("session.new_split", session_id=sid, anchor_id=first["focused_block_id"],
                 direction="horizontal", side="after", layout=SHELL, focus=True)
        view = admin.ok("session.view", session_id=sid)
        blocks = {b["block_id"] for w in view["windows"] for l in w["layers"] for b in l["blocks"]}
        missing = "session:00000000-0000-0000-0000-000000000000"
        c, boot = bootstrap(attach_session_ids=[sid, sid, missing])
        tickets = boot["data_tickets"]
        assert boot["selected_session"] is None and len(boot["attached_sessions"]) == 1
        covered = {t["block_id"] for t in tickets}
        print(json.dumps({"revision": revision, "total_blocks": len(blocks), "tickets": len(tickets),
                          "inactive_window_ticket": second["focused_block_id"] in covered,
                          "expires_in_ms": [t["expires_in_ms"] for t in tickets]}), flush=True)
        assert covered <= blocks and covered
        token = tickets[0]["ticket"]
        welcome = handshake(token)
        assert welcome[0] == 0x81
        rejected = handshake(token)
        assert rejected[0] == 0x82
        print(f"PASS bundled ticket opens data stream; reused ticket rejected (reason {rejected[1]})", flush=True)
        fresh = c.ok("data.connect", session_id=sid, block_id=tickets[0]["block_id"],
                     data_ref_id="terminal", subprotocol=PROTOCOL)
        assert handshake(fresh["ticket"])[0] == 0x81
        print("PASS data.connect fallback opens a fresh stream", flush=True)
        wrong = c.ok("data.connect", session_id=sid, block_id=tickets[0]["block_id"],
                     data_ref_id="terminal", subprotocol=PROTOCOL)
        mismatch = handshake(wrong["ticket"], "rex.data.wrong.v1")
        assert mismatch[:2] == b"\x82\x07", mismatch
        print("PASS tickets are bound to the requested subprotocol", flush=True)
        _, default = bootstrap(preferred_session_id=missing)
        assert default["selected_session"]["session"]["session_id"] == sid
        _, none = bootstrap(attach_session_ids=[])
        assert none["selected_session"] is None and not none.get("attached_sessions") and not none.get("data_tickets")
        admin.ok("session.zoom_block", session_id=sid, block_id=first["focused_block_id"], zoomed=True)
        _, zoom = bootstrap(attach_session_ids=[sid])
        print(json.dumps({"zoomed_tickets": len(zoom.get("data_tickets") or []),
                          "zoomed_block_covered": first["focused_block_id"] in {t["block_id"] for t in zoom.get("data_tickets") or []}}), flush=True)
        failed = control()
        old_identity = failed.welcome["client"]
        invalid = failed.call("session.bootstrap", event_scope="session", data_tickets=[SELECTOR] * 9)
        assert "error" in invalid
        failed.close()
        retry = control(old_identity["reconnect_token"])
        assert not retry.welcome["client"].get("resumed", False)
        assert retry.welcome["client"]["id"] != old_identity["id"]
        retry.ok("session.bootstrap", event_scope="session", attach_session_ids=[])
        print("PASS failed-bootstrap graceful close forgets identity; new connection bootstraps", flush=True)
        admin.ok("session.destroy", session_id=sid)
    finally:
        for c in controls:
            c.close()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server", default="rex")
    parser.add_argument("--revision", type=int, default=8, choices=[7, 8])
    parser.add_argument("--probe", action="append", default=[])
    parser.add_argument("--ui-script", action="append", default=[])
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="rex-r8-") as directory:
        work = Path(directory)
        sock = work / "rex.sock"
        endpoint = f"unix://{sock}"
        with open(work / "server.log", "w") as log:
            server = subprocess.Popen([args.server, "server", "run", "--listen", endpoint, "--restore=false",
                                       "--config", "/dev/null", "--state-dir", str(work / "state")], stdout=log, stderr=log)
            try:
                deadline = time.monotonic() + 10
                while not sock.exists():
                    assert time.monotonic() < deadline and server.poll() is None, "private server failed"
                    time.sleep(0.05)
                checks(sock, args.revision)
                for probe in args.probe:
                    subprocess.run([probe, endpoint], check=True, timeout=120,
                                   env=dict(os.environ, REX_EXPECT_REVISION=str(args.revision)))
                for script in args.ui_script:
                    subprocess.run([sys.executable, script], check=True, timeout=180,
                                   env=dict(os.environ, REX_SERVER=endpoint))
            finally:
                server.terminate()
                server.wait(timeout=10)


if __name__ == "__main__":
    main()
