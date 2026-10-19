#!/usr/bin/env python3
"""Generate the Snake app icon in every format the release needs.

Outputs:
  packaging/snake.png   512x512 PNG (Linux desktop icon)
  packaging/snake.ico   multi-resolution Windows icon
  packaging/snake.icns  macOS icon bundle
  src/icon.rs           raw RGBA frames used for the window icon

Run from the repository root:  python3 packaging/make_icons.py
Requires Pillow:              pip install pillow
"""

from __future__ import annotations

import struct
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parent.parent
PACKAGING = ROOT / "packaging"
SRC = ROOT / "src"

SIZE = 1024
SS = 2  # supersampling factor for smooth edges

BG_TOP = (13, 23, 35, 255)
BG_BOTTOM = (27, 45, 62, 255)
BORDER = (94, 234, 154, 60)
BODY_HEAD = (64, 235, 130, 255)
BODY_TAIL = (16, 158, 88, 255)
FOOD = (255, 71, 87, 255)
LEAF = (68, 189, 74, 255)
STEM = (120, 88, 42, 255)


def lerp(a: int, b: int, t: float) -> int:
    return round(a + (b - a) * t)


def mix(c1: tuple[int, ...], c2: tuple[int, ...], t: float) -> tuple[int, int, int, int]:
    return tuple(lerp(a, b, t) for a, b in zip(c1, c2))  # type: ignore[return-value]


def rounded_mask(size: int, radius: int) -> Image.Image:
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, size - 1, size - 1), radius=radius, fill=255)
    return mask


def build_master() -> Image.Image:
    s = SIZE * SS
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))

    # Drop shadow.
    shadow = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    ImageDraw.Draw(shadow).rounded_rectangle(
        (78 * SS, 96 * SS, 946 * SS, 964 * SS), radius=200 * SS, fill=(0, 0, 0, 110)
    )
    shadow = shadow.filter(ImageFilter.GaussianBlur(26 * SS))
    img.alpha_composite(shadow)

    # Gradient tile, clipped to a rounded square.
    tile = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    grad = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    gd = ImageDraw.Draw(grad)
    for y in range(s):
        gd.line([(0, y), (s, y)], fill=mix(BG_TOP, BG_BOTTOM, y / s))
    tile.paste(grad, (0, 0), rounded_mask(s, 200 * SS))
    img.alpha_composite(tile)

    # Inner highlight ring.
    ring = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    ImageDraw.Draw(ring).rounded_rectangle(
        (66 * SS, 66 * SS, 958 * SS, 958 * SS),
        radius=198 * SS,
        outline=BORDER,
        width=3 * SS,
    )
    img.alpha_composite(ring)

    # Snake body: a chunky S-shaped path, filled with one continuous
    # head-to-tail gradient so the joints stay smooth.
    path = [
        (330, 800),
        (330, 600),
        (520, 600),
        (520, 780),
        (720, 780),
        (720, 480),
        (470, 480),
        (470, 330),
        (680, 330),
    ]
    pts = [(x * SS, y * SS) for x, y in path]
    body_w = 108 * SS
    body_mask = Image.new("L", (s, s), 0)
    md = ImageDraw.Draw(body_mask)
    for p, q in zip(pts, pts[1:]):
        md.line([p, q], fill=255, width=body_w)
    for px, py in pts:
        md.ellipse(
            (px - body_w // 2, py - body_w // 2, px + body_w // 2, py + body_w // 2),
            fill=255,
        )

    body_grad = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    bgd = ImageDraw.Draw(body_grad)
    y0, y1 = 300 * SS, 830 * SS
    for y in range(s):
        t = min(1.0, max(0.0, (y - y0) / (y1 - y0)))
        bgd.line([(0, y), (s, y)], fill=mix(BODY_HEAD, BODY_TAIL, t))
    body = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    body.paste(body_grad, (0, 0), body_mask)
    img.alpha_composite(body)

    d = ImageDraw.Draw(img)

    # Head cap and face (head points right).
    hx, hy = pts[-1]
    head_r = 62 * SS
    d.ellipse((hx - head_r, hy - head_r, hx + head_r, hy + head_r), fill=BODY_HEAD)

    # Forked tongue.
    tongue = (255, 92, 110, 255)
    d.line([(hx + 52 * SS, hy), (hx + 96 * SS, hy)], fill=tongue, width=9 * SS)
    d.line(
        [(hx + 96 * SS, hy), (hx + 112 * SS, hy - 12 * SS)],
        fill=tongue,
        width=9 * SS,
    )
    d.line(
        [(hx + 96 * SS, hy), (hx + 112 * SS, hy + 12 * SS)],
        fill=tongue,
        width=9 * SS,
    )

    for dy in (-27, 27):
        ex, ey = hx + 20 * SS, hy + dy * SS
        eye_r = 20 * SS
        d.ellipse((ex - eye_r, ey - eye_r, ex + eye_r, ey + eye_r), fill=(255, 255, 255, 255))
        pr = 9 * SS
        px, py = ex + 7 * SS, ey
        d.ellipse((px - pr, py - pr, px + pr, py + pr), fill=(13, 23, 35, 255))

    # Apple in the empty top-left corner.
    ax, ay = 305 * SS, 310 * SS
    ar = 62 * SS
    d.ellipse((ax - ar, ay - ar, ax + ar, ay + ar), fill=FOOD)
    d.ellipse(
        (ax - ar * 1.25, ay - ar * 1.25, ax + ar * 1.25, ay + ar * 1.25),
        outline=(255, 71, 87, 40),
        width=10 * SS,
    )
    # Highlight.
    d.ellipse(
        (ax - ar * 0.55, ay - ar * 0.6, ax - ar * 0.05, ay - ar * 0.1),
        fill=(255, 179, 186, 120),
    )
    # Stem and leaf.
    d.line(
        [(ax, ay - ar * 0.9), (ax + 6 * SS, ay - ar * 1.5)],
        fill=STEM,
        width=10 * SS,
    )
    d.ellipse(
        (ax + 8 * SS, ay - ar * 1.65, ax + 66 * SS, ay - ar * 1.15),
        fill=LEAF,
    )

    return img.resize((SIZE, SIZE), Image.LANCZOS)


def icns_bytes(master: Image.Image, sizes: list[tuple[bytes, int]]) -> bytes:
    entries = bytearray()
    for kind, px in sizes:
        png = pack_png(master, px)
        entries += kind + struct.pack(">I", len(png) + 8) + png
    header = b"icns" + struct.pack(">I", len(entries) + 8)
    return header + bytes(entries)


def pack_png(master: Image.Image, px: int) -> bytes:
    import io

    buf = io.BytesIO()
    master.resize((px, px), Image.LANCZOS).save(buf, format="PNG")
    return buf.getvalue()


def rgb_module(name: str, master: Image.Image, px: int) -> str:
    rgba = master.resize((px, px), Image.LANCZOS).tobytes()
    body = []
    for i in range(0, len(rgba), 24):
        body.append("    " + ", ".join(str(b) for b in rgba[i : i + 24]) + ",")
    return (
        f"/// {px}x{px} RGBA pixels, row-major.\n"
        f"#[rustfmt::skip]\n"
        f"pub const {name}: [u8; {len(rgba)}] = [\n" + "\n".join(body) + "\n];\n"
    )


def main() -> None:
    PACKAGING.mkdir(exist_ok=True)
    SRC.mkdir(exist_ok=True)

    master = build_master()

    master.resize((512, 512), Image.LANCZOS).save(PACKAGING / "snake.png", optimize=True)

    master.save(
        PACKAGING / "snake.ico",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )

    icns_sizes: list[tuple[bytes, int]] = [
        (b"icp4", 16),
        (b"icp5", 32),
        (b"icp6", 64),
        (b"ic07", 128),
        (b"ic08", 256),
        (b"ic09", 512),
        (b"ic10", 1024),
        (b"ic11", 32),
        (b"ic12", 64),
        (b"ic13", 256),
        (b"ic14", 512),
    ]
    (PACKAGING / "snake.icns").write_bytes(icns_bytes(master, icns_sizes))

    module = (
        "// Generated by packaging/make_icons.py - do not edit by hand.\n"
        "// Used by the window configuration in main.rs.\n\n"
        + rgb_module("SMALL", master, 16)
        + "\n"
        + rgb_module("MEDIUM", master, 32)
        + "\n"
        + rgb_module("BIG", master, 64)
    )
    (SRC / "icon.rs").write_text(module)

    print("Wrote packaging/snake.png, packaging/snake.ico, packaging/snake.icns")
    print("Wrote src/icon.rs")


if __name__ == "__main__":
    main()
