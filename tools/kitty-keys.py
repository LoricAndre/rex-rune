#!/usr/bin/env python3
"""Record the raw bytes a terminal sends for keys, with Kitty keyboard reporting on.

Usage: kitty-keys.py READY RESULT SECONDS

Run it in a terminal pane. It puts the tty in raw mode, enables the Kitty keyboard
protocol with flags 3 (`CSI > 3 u`: disambiguate escape codes and report press,
repeat and release), touches READY, records everything typed for SECONDS, then
pops the mode, restores the tty and writes the bytes to RESULT. Read the result
with `od -c RESULT`. `scripts/x11-smoke.py` drives it under Xvfb.
"""
import os
from pathlib import Path
import select
import sys
import termios
import time
import tty


def main():
    ready, result, seconds = Path(sys.argv[1]), Path(sys.argv[2]), float(sys.argv[3])
    old = termios.tcgetattr(0)
    data = bytearray()
    try:
        tty.setraw(0)
        time.sleep(.25)
        termios.tcflush(0, termios.TCIFLUSH)
        os.write(1, b"\x1b[>3u")
        ready.touch()
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            if select.select([0], [], [], .05)[0]:
                data.extend(os.read(0, 4096))
    finally:
        os.write(1, b"\x1b[<u")
        termios.tcsetattr(0, termios.TCSADRAIN, old)
        result.write_bytes(data)


if __name__ == "__main__":
    main()
