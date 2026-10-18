"""Synthetic arithmetic fixtures and comparison with original pinned image.c.

This helper compiles/runs only in the native-oracle Actions job. No corpus
reference files are created or changed; generated fixtures are temporary.
"""
from pathlib import Path
import shlex
import struct
import subprocess
import tempfile
import zlib

import images
from inventory import digest


def rgba8_pairs():
    return bytes(v for alpha in range(256) for channel in range(256)
                 for v in (channel, channel, channel, alpha))


def rgba16_samples():
    # Every 16-bit channel at transparent, low-alpha, midpoint and opaque edges.
    alphas = (0, 1, 32767, 32768, 65534, 65535)
    data = bytearray()
    for alpha in alphas:
        for channel in range(65536):
            data.extend(struct.pack(">HHHH", channel, 65535 - channel, 32768, alpha))
    return data


def write_rgba16(path, width, height, data):
    if len(data) != width * height * 8:
        raise ValueError("synthetic RGBA16 length")
    rows = b"".join(b"\0" + data[y * width * 8:(y + 1) * width * 8] for y in range(height))
    Path(path).write_bytes(images.SIGNATURE + images.chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 16, 6, 0, 0, 0))
                           + images.chunk(b"IDAT", zlib.compress(rows)) + images.chunk(b"IEND", b""))


def check_pinned_pixels(source):
    source = Path(source)
    subprocess.run(["git", "-C", str(source), "diff", "--exit-code", "HEAD", "--", "compare/image.c", "compare/image.h"], check=True)
    with tempfile.TemporaryDirectory(prefix="taiyou-png-reference-") as temporary:
        folder = Path(temporary)
        bridge = folder / "decode-reference"
        flags = shlex.split(subprocess.check_output(["pkg-config", "--cflags", "--libs", "libpng"], text=True))
        subprocess.run(["cc", "-std=c11", "-I" + str(source / "compare"), str(Path(__file__).with_name("reference-pixels.c")),
                        str(source / "compare/image.c"), "-o", str(bridge), *flags], check=True)
        images.write_png(folder / "eight.png", 256, 256, rgba8_pairs())
        write_rgba16(folder / "sixteen.png", 256, 1536, rgba16_samples())
        for name in ("eight.png", "sixteen.png"):
            expected = subprocess.check_output([str(bridge), str(folder / name)], timeout=30)
            decoded = images.read_png(folder / name)[2]
            actual = b"".join(struct.pack(">H", v) for v in decoded)
            if actual != expected:
                raise ValueError("normalization differs from pinned libass image.c: " + name)
    return {"rgba8_channel_alpha_pairs": 65536, "rgba16_boundary_pairs": 393216,
            "source_sha256": digest((source / "compare/image.c").read_bytes()), "result": "EXACT"}
