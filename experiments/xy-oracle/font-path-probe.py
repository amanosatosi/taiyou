"""Actions-only observations of the native font path before ASS transforms.

No captures, comparison outcomes or renderer expectations are generated here.
"""
import ctypes as C
from ctypes import wintypes as W
import json
import os
from pathlib import Path
import struct

from fontTools.ttLib import TTFont
from fontTools.pens.basePen import BasePen


class Commands(BasePen):
    def __init__(self, glyphs):
        super().__init__(glyphs)
        self.ops = []

    def _moveTo(self, p):
        self.ops.append(["M", p])

    def _lineTo(self, p):
        self.ops.append(["L", p])

    def _curveToOne(self, a, b, c):
        self.ops.append(["C", a, b, c])

    def _qCurveToOne(self, a, b):
        self.ops.append(["Q", a, b])

    def _closePath(self):
        self.ops.append(["Z"])


def main():
    assert os.environ.get("GITHUB_ACTIONS") == "true" and os.name == "nt"
    gdi = C.WinDLL("gdi32", use_last_error=True)
    def api(name, result, *args):
        f = getattr(gdi, name)
        f.restype, f.argtypes = result, args
        return f
    class Point(C.Structure):
        _fields_ = [("x", W.LONG), ("y", W.LONG)]
    class Size(C.Structure):
        _fields_ = [("x", W.LONG), ("y", W.LONG)]
    class GlyphMetrics(C.Structure):
        _fields_ = [("width", W.UINT), ("height", W.UINT), ("origin", Point),
                    ("advance_x", C.c_short), ("advance_y", C.c_short)]
    class Transform(C.Structure):
        _fields_ = [(name, C.c_float) for name in ("xx", "xy", "yx", "yy", "x", "y")]
    add = api("AddFontMemResourceEx", W.HANDLE, W.LPVOID, W.DWORD, W.LPVOID, C.POINTER(W.DWORD))
    remove = api("RemoveFontMemResourceEx", W.BOOL, W.HANDLE)
    dc_new = api("CreateCompatibleDC", W.HDC, W.HDC)
    dc_delete = api("DeleteDC", W.BOOL, W.HDC)
    create = api("CreateFontW", W.HFONT, *([C.c_int] * 5), *([W.DWORD] * 8), W.LPCWSTR)
    select = api("SelectObject", W.HANDLE, W.HDC, W.HANDLE)
    delete = api("DeleteObject", W.BOOL, W.HANDLE)
    align = api("SetTextAlign", W.UINT, W.HDC, W.UINT)
    background = api("SetBkMode", C.c_int, W.HDC, C.c_int)
    begin = api("BeginPath", W.BOOL, W.HDC)
    end = api("EndPath", W.BOOL, W.HDC)
    text = api("TextOutW", W.BOOL, W.HDC, C.c_int, C.c_int, W.LPCWSTR, C.c_int)
    path = api("GetPath", C.c_int, W.HDC, C.POINTER(Point), C.POINTER(W.BYTE), C.c_int)
    extent = api("GetTextExtentPoint32W", W.BOOL, W.HDC, W.LPCWSTR, C.c_int, C.POINTER(Size))
    face = api("GetTextFaceW", C.c_int, W.HDC, C.c_int, W.LPWSTR)
    glyph_outline = api("GetGlyphOutlineW", W.DWORD, W.HDC, W.UINT, W.UINT,
                        C.POINTER(GlyphMetrics), W.DWORD, W.LPVOID, W.LPVOID)
    save = api("SaveDC", C.c_int, W.HDC)
    restore = api("RestoreDC", W.BOOL, W.HDC, C.c_int)
    mode = api("SetGraphicsMode", C.c_int, W.HDC, C.c_int)
    world = api("SetWorldTransform", W.BOOL, W.HDC, C.POINTER(Transform))

    def native_outline(character, format):
        matrix = (C.c_int32 * 4)(65536, 0, 0, 65536)
        gm = GlyphMetrics()
        n = glyph_outline(dc, ord(character), format, C.byref(gm), 0, None, matrix)
        if n == 0xFFFFFFFF:
            return {"format": format, "status": "GDI_ERROR"}
        assert 0 < n <= 2 * 1024 * 1024
        buffer = C.create_string_buffer(n)
        assert glyph_outline(dc, ord(character), format, C.byref(gm), n, buffer, matrix) == n
        raw, at, contours = buffer.raw, 0, []
        point = lambda at: [v / 65536 for v in struct.unpack_from("<ii", raw, at)]
        while at < n:
            length, kind = struct.unpack_from("<II", raw, at)
            assert kind == 24 and 16 <= length <= n - at
            end = at + length
            contour = {"start": point(at + 8), "curves": []}
            at += 16
            while at < end:
                kind, count = struct.unpack_from("<HH", raw, at)
                assert kind in (1, 2, 3) and count > 0 and at + 4 + count * 8 <= end
                contour["curves"].append({"kind": kind, "points": [point(at + 4 + i * 8) for i in range(count)]})
                at += 4 + count * 8
            contours.append(contour)
        return {"format": format, "status": "OK", "contours": contours}
    corpus = Path(os.environ["TAIYOU_CORPUS"])
    rows = []
    for relative, sizes, chars in [
        ("regression/blurs/Aileron-Regular.otf", [100], "+·¶"),
        ("regression/v4++/DejaVuSans.ttf", [30, 35, 42, 60], "KaraokeW"),
    ]:
        filename = corpus / relative
        raw = C.create_string_buffer(filename.read_bytes())
        count = W.DWORD()
        resource = add(raw, len(raw) - 1, None, C.byref(count))
        assert resource and count.value == 1
        font = TTFont(filename)
        glyphs = font.getGlyphSet()
        family = font["name"].getDebugName(1)
        dc = dc_new(None)
        assert dc and background(dc, 1) and align(dc, 24) != 0xFFFFFFFF
        try:
            for size in sizes:
                hfont = create(size * 64, 0, 0, 0, 400, 0, 0, 0, 1, 4, 0, 4, 0, family)
                assert hfont
                previous = select(dc, hfont)
                try:
                    actual = C.create_unicode_buffer(256)
                    assert face(dc, 256, actual) and actual.value == family
                    for char in dict.fromkeys(chars):
                        pen = Commands(glyphs)
                        glyphs[font.getBestCmap()[ord(char)]].draw(pen)
                        assert begin(dc) and text(dc, 0, 0, char, 1) and end(dc)
                        n = path(dc, None, None, 0)
                        assert 0 < n < 10000
                        points, types = (Point * n)(), (W.BYTE * n)()
                        assert path(dc, points, types, n) == n
                        saved = save(dc)
                        assert saved
                        try:
                            assert mode(dc, 2)
                            transform = Transform(1 / 64, 0, 0, 1 / 64, 0, 0)
                            assert world(dc, C.byref(transform))
                            fine, fine_types = (Point * n)(), (W.BYTE * n)()
                            assert path(dc, fine, fine_types, n) == n
                            stored = [[p.x / 64, -p.y / 64, int(t)] for p, t in zip(fine, fine_types)]
                        finally:
                            assert restore(dc, saved)
                        width = Size()
                        assert extent(dc, char, 1, C.byref(width))
                        rows.append({"font": relative, "family": family, "size": size, "char": char,
                                     "upem": font["head"].unitsPerEm,
                                     "win_cell": [font["OS/2"].usWinAscent, font["OS/2"].usWinDescent],
                                     "cff": "CFF " in font, "design": pen.ops, "advance_q6": width.x,
                                     "stored_device": stored,
                                     "native_outlines": [native_outline(char, f) for f in (2, 3, 258, 259)],
                                     "gdi": [[p.x, -p.y, int(t)] for p, t in zip(points, types)]})
                finally:
                    select(dc, previous)
                    delete(hfont)
        finally:
            dc_delete(dc)
            font.close()
            remove(resource)
    output = Path(os.environ["RUNNER_TEMP"]) / "font-path-probe.json"
    output.write_text(json.dumps(rows, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
