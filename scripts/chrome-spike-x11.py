#!/usr/bin/env python3
"""R1 spike on real X11 events: Xvfb + openbox, xdotool input, EWMH state read back with xprop.

Checks the vendored Rune window additions on a borderless window: minimize, maximize and
restore buttons, drag-region double-click, and edge resize. X11 has no window-menu request in
winit, so that one is Wayland-only."""
import os, re, subprocess, sys, tempfile, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target/debug/examples/chrome_spike"
DISP = ":" + os.environ.get("SPIKE_DISPLAY", "91")
env = dict(os.environ, DISPLAY=DISP, WAYLAND_DISPLAY="")

def sh(*args):
    return subprocess.run(args, env=env, capture_output=True, text=True).stdout.strip()

procs = []
def start(*args, **kw):
    p = subprocess.Popen(args, env=env, **kw)
    procs.append(p)
    return p

results = []
def check(name, ok, detail=""):
    results.append(ok)
    print(("PASS " if ok else "FAIL ") + name + (f" — {detail}" if detail else ""))

try:
    start("Xvfb", DISP, "-screen", "0", "1600x1000x24", stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1)
    start("openbox", stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1)
    log = open(os.path.join(tempfile.gettempdir(), "chrome-spike-x11.log"), "w+")
    app = start(str(BIN), stdout=log, stderr=log)
    win = ""
    for _ in range(100):
        win = sh("xdotool", "search", "--name", "Rex chrome spike").split("\n")[0]
        if win:
            break
        time.sleep(0.2)
    time.sleep(1.5)
    def rects():
        log.flush(); text = open(log.name).read()
        out = {}
        for k, x, y, w, h in re.findall(r"spike: rect (\w+) ([\d.]+) ([\d.]+) ([\d.]+) ([\d.]+)", text):
            out[k] = tuple(map(float, (x, y, w, h)))
        return out
    def geom():
        g = dict(l.split("=") for l in sh("xdotool", "getwindowgeometry", "--shell", win).split("\n") if "=" in l)
        return int(g["X"]), int(g["Y"]), int(g["WIDTH"]), int(g["HEIGHT"])
    def state():
        return sh("xprop", "-id", win, "_NET_WM_STATE", "WM_STATE")
    def click_rect(key, double=False):
        x, y, w, h = rects()[key]
        gx, gy, _, _ = geom()
        sh("xdotool", "mousemove", str(int(gx + x + w / 2)), str(int(gy + y + h / 2)))
        sh("xdotool", "click", *(["--repeat", "2", "--delay", "80"] if double else []), "1")
        time.sleep(1.2)
    start_geom = geom()
    print("start", start_geom, "decorations:", sh("xprop", "-id", win, "_MOTIF_WM_HINTS"))
    click_rect("max")
    s = state()
    check("maximize button sets _NET_WM_STATE_MAXIMIZED", "MAXIMIZED_VERT" in s and "MAXIMIZED_HORZ" in s, s.split("\n")[0])
    time.sleep(0.5)
    check("view reads Cx::window_maximized", "maximized true" in open(log.name).read())
    click_rect("max")
    s = state()
    check("restore button clears maximized", "MAXIMIZED" not in s.split("\n")[0], s.split("\n")[0])
    # Drag region double-click maximizes.
    x, y, w, h = rects()["min"]
    gx, gy, gw, gh = geom()
    sh("xdotool", "mousemove", str(gx + 120), str(gy + 22)); sh("xdotool", "click", "--repeat", "2", "--delay", "80", "1"); time.sleep(1.2)
    s = state()
    check("drag-region double-click maximizes", "MAXIMIZED_VERT" in s, s.split("\n")[0])
    sh("xdotool", "mousemove", str(geom()[0] + 120), str(geom()[1] + 22)); sh("xdotool", "click", "--repeat", "2", "--delay", "80", "1"); time.sleep(1.2)
    # Drag region moves the window.
    gx, gy, gw, gh = geom()
    sh("xdotool", "mousemove", str(gx + 200), str(gy + 22), "mousedown", "1")
    for i in range(1, 11):
        sh("xdotool", "mousemove", str(gx + 200 + i * 10), str(gy + 22 + i * 6)); time.sleep(0.05)
    sh("xdotool", "mouseup", "1"); time.sleep(0.8)
    nx, ny, _, _ = geom()
    check("drag region moves the window", (nx, ny) != (gx, gy), f"{(gx, gy)} → {(nx, ny)}")
    # South-east corner resize.
    gx, gy, gw, gh = geom()
    sh("xdotool", "mousemove", str(gx + gw - 2), str(gy + gh - 2), "mousedown", "1")
    for i in range(1, 11):
        sh("xdotool", "mousemove", str(gx + gw - 2 + i * 8), str(gy + gh - 2 + i * 5)); time.sleep(0.05)
    sh("xdotool", "mouseup", "1"); time.sleep(0.8)
    _, _, nw, nh = geom()
    check("south-east corner resizes", nw > gw and nh > gh, f"{gw}x{gh} → {nw}x{nh}")
    gx, gy, gw, gh = geom()
    sh("xdotool", "mousemove", str(gx + 2), str(gy + gh // 2), "mousedown", "1")
    for i in range(1, 11):
        sh("xdotool", "mousemove", str(gx + 2 - i * 6), str(gy + gh // 2)); time.sleep(0.05)
    sh("xdotool", "mouseup", "1"); time.sleep(0.8)
    nx, _, nw, _ = geom()
    check("west edge resizes", nw > gw and nx < gx, f"{gw} at {gx} → {nw} at {nx}")
    click_rect("min")
    s = state()
    check("minimize button iconifies", "HIDDEN" in s or "Iconic" in s, " ".join(s.split()))
    print("log tail:", "".join(open(log.name).readlines()[-3:]).strip())
finally:
    for p in reversed(procs):
        p.terminate()
sys.exit(0 if all(results) else 1)
