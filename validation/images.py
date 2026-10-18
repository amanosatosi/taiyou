"""Bounded RGBA PNG I/O for comparison, with no gamma/color-space conversion.

Inputs use libass compare/image.c's exact fixed-reciprocal premultiplication;
8-bit results expand by 257. 16-bit results retain their precision. Alpha is ordinary
coverage (not libass's internal inverted alpha); absolute deltas are equivalent.
Only the upstream documented noninterlaced RGBA8/16 targets are accepted.
"""
from array import array
from pathlib import Path
import struct
import zlib

MAX_PIXELS = 16 * 1024 * 1024
MAX_FILE = 128 * 1024 * 1024
SIGNATURE = b"\x89PNG\r\n\x1a\n"


class ImageDecodeError(ValueError):
    """Malformed compressed PNG payload, reportable as a per-case failure."""


def dimensions(width, height):
    if not (0 < width <= 16384 and 0 < height <= 16384 and width * height <= MAX_PIXELS):
        raise ValueError("image dimensions exceed comparison budget")


def chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))


def write_png(path, width, height, rgba):
    dimensions(width, height)
    if len(rgba) != width * height * 4:
        raise ValueError("RGBA length mismatch")
    rows = b"".join(b"\0" + rgba[y * width * 4:(y + 1) * width * 4] for y in range(height))
    Path(path).write_bytes(SIGNATURE + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
                           + chunk(b"IDAT", zlib.compress(rows)) + chunk(b"IEND", b""))


def paeth(a, b, c):
    p = a + b - c
    da, db, dc = abs(p - a), abs(p - b), abs(p - c)
    return a if da <= db and da <= dc else b if db <= dc else c


def read_png(path):
    path = Path(path)
    if path.stat().st_size > MAX_FILE:
        raise ValueError("PNG file budget")
    data = path.read_bytes()
    if not data.startswith(SIGNATURE):
        raise ValueError("invalid PNG signature")
    pos, header, compressed, ended, idat_ended = 8, None, bytearray(), False, False
    while pos < len(data):
        if pos + 12 > len(data):
            raise ValueError("truncated PNG chunk")
        size = struct.unpack_from(">I", data, pos)[0]
        kind = data[pos + 4:pos + 8]
        end = pos + 12 + size
        if end > len(data):
            raise ValueError("truncated PNG data")
        body = data[pos + 8:pos + 8 + size]
        if zlib.crc32(kind + body) != struct.unpack_from(">I", data, end - 4)[0]:
            raise ValueError("PNG CRC mismatch")
        if header is None and kind != b"IHDR":
            raise ValueError("PNG IHDR must be first")
        if kind == b"IHDR":
            if header is not None or size != 13:
                raise ValueError("duplicate/invalid PNG IHDR")
            header = struct.unpack(">IIBBBBB", body)
            width, height, depth, color, compression, filtering, interlace = header
            dimensions(width, height)
            if depth not in (8, 16) or color != 6 or (compression, filtering, interlace) != (0, 0, 0):
                raise ValueError("expected noninterlaced RGBA8/16 PNG; no implicit conversion")
        elif kind == b"IDAT":
            if idat_ended:
                raise ValueError("nonconsecutive PNG IDAT")
            compressed.extend(body)
        elif kind == b"IEND":
            if size or not compressed or end != len(data):
                raise ValueError("invalid PNG end")
            ended = True
            break
        else:
            if compressed:
                idat_ended = True
            if not kind[0] & 32:
                raise ValueError("unknown critical PNG chunk")
        pos = end
    if not ended:
        raise ValueError("missing PNG end")
    bpp = 4 * depth // 8
    stride = width * bpp
    expected = (stride + 1) * height
    try:
        decoder = zlib.decompressobj()
        raw = decoder.decompress(compressed, expected + 1)
    except zlib.error as error:
        raise ImageDecodeError("invalid PNG zlib stream: " + str(error)) from error
    if len(raw) != expected or not decoder.eof or decoder.unused_data or decoder.unconsumed_tail:
        raise ValueError("invalid/over-budget PNG decompression")
    pixels = array('H')
    previous = bytearray(stride)
    for y in range(height):
        offset = y * (stride + 1)
        method, row = raw[offset], bytearray(raw[offset + 1:offset + stride + 1])
        if method > 4:
            raise ValueError("invalid PNG filter")
        for x in range(stride):
            a, b, c = row[x - bpp] if x >= bpp else 0, previous[x], previous[x - bpp] if x >= bpp else 0
            predict = (0, a, b, (a + b) // 2, paeth(a, b, c))[method]
            row[x] = (row[x] + predict) & 255
        if depth == 8:
            for x in range(0, stride, 4):
                r, g, b, a = row[x:x + 4]
                ra, ga, ba = r * a, g * a, b * a
                pixels.extend((((ra + (ra >> 8) + 128) >> 8) * 257,
                               ((ga + (ga >> 8) + 128) >> 8) * 257,
                               ((ba + (ba >> 8) + 128) >> 8) * 257, a * 257))
        else:
            for r, g, b, a in struct.iter_unpack(">HHHH", row):
                ra, ga, ba = r * a, g * a, b * a
                pixels.extend(((ra + (ra >> 16) + (1 << 15)) >> 16,
                               (ga + (ga >> 16) + (1 << 15)) >> 16,
                               (ba + (ba >> 16) + (1 << 15)) >> 16, a))
        previous = row
    return width, height, pixels


def raw_rgba(path, width, height):
    dimensions(width, height)
    if Path(path).stat().st_size != width * height * 4:
        raise ValueError("raw renderer dimensions/length mismatch")
    raw = Path(path).read_bytes()
    if any(raw[i + c] > raw[i + 3] for i in range(0, len(raw), 4) for c in range(3)):
        raise ValueError("raw frame is not premultiplied")
    return width, height, array('H', (v * 257 for v in raw))


def compare(expected, actual, diff_path=None):
    ew, eh, ep = expected
    aw, ah, ap = actual
    if len(ep) != ew * eh * 4 or len(ap) != aw * ah * 4:
        raise ValueError("comparison pixel length mismatch")
    metrics = {"expected_dimensions": [ew, eh], "actual_dimensions": [aw, ah],
               "dimensions_match": (ew, eh) == (aw, ah), "max_channel_delta": None,
               "channel_mae": None, "changed_pixels": None, "changed_bbox": None,
               "alpha_mismatches": None, "exact": False, "units": "premultiplied-encoded-RGBA16"}
    if not metrics["dimensions_match"]:
        return metrics
    sums, maximum, changed, alpha, bounds = [0] * 4, 0, 0, 0, None
    diff = bytearray(len(ep)) if diff_path else None
    for i in range(0, len(ep), 4):
        delta = [abs(ep[i + c] - ap[i + c]) for c in range(4)]
        for c in range(4):
            sums[c] += delta[c]
        maximum = max(maximum, *delta)
        if any(delta):
            changed += 1
            x, y = (i // 4) % ew, (i // 4) // ew
            bounds = [x, y, x + 1, y + 1] if bounds is None else [min(bounds[0], x), min(bounds[1], y), max(bounds[2], x + 1), max(bounds[3], y + 1)]
        alpha += delta[3] != 0
        if diff is not None:
            # Visible opaque heatmap includes alpha-only differences in red.
            diff[i:i + 4] = bytes([max(delta[0], delta[3]) // 257, delta[1] // 257, delta[2] // 257, 255])
    metrics.update(max_channel_delta=maximum, channel_mae=[s / (ew * eh) for s in sums],
                   changed_pixels=changed, changed_bbox=bounds, alpha_mismatches=alpha, exact=changed == 0)
    if diff_path:
        write_png(diff_path, ew, eh, diff)
    return metrics
