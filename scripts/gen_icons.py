#!/usr/bin/env python3
"""Generate toast logo PNGs (assets/*.png) for the built-in presets.

Stdlib only. Run once after setup:
    python scripts/gen_icons.py

Produces assets/claude.png, assets/codex.png, assets/aider.png — 64x64 RGBA
rounded squares of the preset color with a letter (appLogoOverride-friendly).
"""

import os
import struct
import zlib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ASSETS = os.path.join(ROOT, "assets")

PRESETS = {
    "claude": ((128, 90, 180), "C"),
    "codex": ((30, 140, 170), "D"),
    "aider": ((90, 150, 110), "A"),
}

# Minimal 5x5 uppercase font (rows of 0/1).
FONT = {
    "A": [
        "01110",
        "10001",
        "10001",
        "11111",
        "10001",
    ],
    "C": [
        "01111",
        "10000",
        "10000",
        "10000",
        "01111",
    ],
    "D": [
        "11110",
        "10001",
        "10001",
        "10001",
        "11110",
    ],
}


def chunk(typ, data):
    return (
        struct.pack(">I", len(data))
        + typ
        + data
        + struct.pack(">I", zlib.crc32(typ + data) & 0xFFFFFFFF)
    )


def in_rounded_square(x, y, w, h, r):
    cx, cy = w / 2, h / 2
    dx, dy = abs(x - cx), abs(y - cy)
    if dx <= cx - r or dy <= cy - r:
        return True
    ax, ay = dx - (cx - r), dy - (cy - r)
    return ax * ax + ay * ay <= r * r


def make_grid(w, h, color, letter=None, scale=4):
    grid = []
    for y in range(h):
        row = []
        for x in range(w):
            row.append(tuple(color) + (255,) if in_rounded_square(x, y, w, h, 10) else (0, 0, 0, 0))
        grid.append(row)
    if letter:
        rows = FONT[letter]
        gh, gw = len(rows), max(len(r) for r in rows)
        ox = (w - gw * scale) // 2
        oy = (h - gh * scale) // 2
        for i, row in enumerate(rows):
            for j, ch in enumerate(row):
                if ch == "1":
                    for dy in range(scale):
                        for dx in range(scale):
                            gx, gy = ox + j * scale + dx, oy + i * scale + dy
                            if 0 <= gx < w and 0 <= gy < h:
                                grid[gy][gx] = (255, 255, 255, 255)
    return grid


def write_png(path, grid):
    w, h = len(grid[0]), len(grid)
    sig = b"\x89PNG\r\n\x1a\n"
    ihdr = struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)
    raw = b"".join(b"\x00" + b"".join(bytes(px) for px in row) for row in grid)
    idat = zlib.compress(raw, 9)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(sig + chunk(b"IHDR", ihdr) + chunk(b"IDAT", idat) + chunk(b"IEND", b""))
    print(f"wrote {os.path.relpath(path, ROOT)}")


def main():
    for name, (color, letter) in PRESETS.items():
        write_png(os.path.join(ASSETS, f"{name}.png"), make_grid(64, 64, color, letter))


if __name__ == "__main__":
    main()
