#!/usr/bin/env python3
"""Capture only the native X11 window of an explicitly supplied fixture PID.

Requires python-xlib and Pillow. The caller owns the fixture and its sandbox;
this helper never captures the desktop or controls any window.
"""

import argparse
import time
from pathlib import Path

from PIL import Image
from Xlib import X, display, error


def capture(pid, output):
    connection = display.Display()
    try:
        root = connection.screen().root
        pid_atom = connection.intern_atom("_NET_WM_PID")
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            pending = list(root.query_tree().children)
            while pending:
                window = pending.pop()
                try:
                    owner = window.get_full_property(pid_atom, X.AnyPropertyType)
                    if owner is not None and list(owner.value) == [pid]:
                        geometry = window.get_geometry()
                        if window.get_attributes().map_state != X.IsViewable:
                            continue
                        if geometry.depth not in (24, 32):
                            raise RuntimeError(f"unsupported X11 depth: {geometry.depth}")
                        pixels = window.get_image(
                            0, 0, geometry.width, geometry.height, X.ZPixmap, 0xFFFFFFFF
                        )
                        image = Image.frombytes(
                            "RGB", (geometry.width, geometry.height), pixels.data,
                            "raw", "BGRX", 0, 1,
                        )
                        image.save(output)
                        print(f"Captured fixture PID {pid}: {output}")
                        return
                    pending.extend(window.query_tree().children)
                except error.XError:
                    continue
            time.sleep(0.05)
        raise RuntimeError(f"no visible X11 window for fixture PID {pid}")
    finally:
        connection.close()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pid", type=int)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.pid <= 0 or not args.output.is_absolute() or not args.output.parent.is_dir():
        parser.error("provide a positive fixture PID and an absolute path in an existing directory")
    capture(args.pid, args.output)
