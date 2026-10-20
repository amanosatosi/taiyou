"""libass compare.c -s adapter, independently expressed; no pixel tolerance.

Render at (sx*w, sy*h) against the original video size, then average premultiplied
encoded RGBA8 into RGBA16 using the reference fixed-reciprocal arithmetic.
Source: local libass compare/compare.c compare(), lines 174-220; regression's
pinned run-single.sh passes the literal scale file to that interface.
"""
from array import array
from pathlib import Path
import images


def factors(value):
    fields = value.split("x")
    if len(fields) not in (1, 2) or any(len(v) != 1 or v not in "12345678" for v in fields):
        raise ValueError("invalid upstream comparison scale")
    x = int(fields[0])
    return x, int(fields[-1])


def normalize(path, width, height, sx, sy):
    images.dimensions(width * sx, height * sy)
    if Path(path).stat().st_size != width * height * sx * sy * 4:
        raise ValueError("oversampled raw length mismatch")
    raw = Path(path).read_bytes()
    if any(raw[i + c] > raw[i + 3] for i in range(0, len(raw), 4) for c in range(3)):
        raise ValueError("oversampled raw frame is not premultiplied")
    area = sx * sy
    reciprocal = (257 << 20) // area
    offset = (1 << 19) - 1
    result = array("H")
    stride = width * sx * 4
    for y in range(height):
        for x in range(width):
            start = y * sy * stride + x * sx * 4
            for channel in range(4):
                total = sum(raw[start + i * stride + j * 4 + channel]
                            for i in range(sy) for j in range(sx))
                result.append((total * reciprocal + offset) >> 20)
    return width, height, result


def preview(path, image):
    """Display-only RGBA8 export; normalized RGBA16 remains pixel authority."""
    rgba = bytearray()
    pixels = image[2]
    for i in range(0, len(pixels), 4):
        alpha = pixels[i + 3]
        for channel in range(3):
            rgba.append(0 if not alpha else min(255, (pixels[i + channel] * 255 + alpha // 2) // alpha))
        rgba.append((alpha + 128) // 257)
    images.write_png(path, image[0], image[1], rgba)
