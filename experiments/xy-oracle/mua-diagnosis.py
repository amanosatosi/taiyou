"""Owner-requested supplemental direct-xy reductions. No accepted baseline changes."""
import argparse
import ctypes as C
from ctypes import wintypes as W
import json
import os
from pathlib import Path
import re
import subprocess

import oracle
import parallel


def gdi_metrics(font, texts, families):
    """Reproduce pinterf's positive 64x cell, extent and TextOut path calls."""
    gdi = C.WinDLL("gdi32", use_last_error=True)

    def api(name, result, *args):
        fn = getattr(gdi, name)
        fn.restype, fn.argtypes = result, args
        return fn

    class Metrics(C.Structure):
        _fields_ = [(name, W.LONG) for name in (
            "height", "ascent", "descent", "internal_leading", "external_leading",
            "average_width", "maximum_width", "weight", "overhang", "aspect_x", "aspect_y")]
        _fields_ += [(name, W.WCHAR) for name in ("first", "last", "default", "break_char")]
        _fields_ += [(name, W.BYTE) for name in ("italic", "underlined", "struck", "pitch_family", "charset")]

    class Size(C.Structure):
        _fields_ = [("x", W.LONG), ("y", W.LONG)]

    class Point(C.Structure):
        _fields_ = [("x", W.LONG), ("y", W.LONG)]

    add = api("AddFontMemResourceEx", W.HANDLE, W.LPVOID, W.DWORD, W.LPVOID, C.POINTER(W.DWORD))
    remove = api("RemoveFontMemResourceEx", W.BOOL, W.HANDLE)
    create_dc = api("CreateCompatibleDC", W.HDC, W.HDC)
    delete_dc = api("DeleteDC", W.BOOL, W.HDC)
    create = api("CreateFontW", W.HFONT, *([C.c_int] * 5), *([W.DWORD] * 8), W.LPCWSTR)
    select = api("SelectObject", W.HANDLE, W.HDC, W.HANDLE)
    delete = api("DeleteObject", W.BOOL, W.HANDLE)
    metrics = api("GetTextMetricsW", W.BOOL, W.HDC, C.POINTER(Metrics))
    face = api("GetTextFaceW", C.c_int, W.HDC, C.c_int, W.LPWSTR)
    extent = api("GetTextExtentPoint32W", W.BOOL, W.HDC, W.LPCWSTR, C.c_int, C.POINTER(Size))
    indices = api("GetGlyphIndicesW", W.DWORD, W.HDC, W.LPCWSTR, C.c_int, C.POINTER(W.WORD), W.DWORD)
    begin = api("BeginPath", W.BOOL, W.HDC)
    end = api("EndPath", W.BOOL, W.HDC)
    text_out = api("TextOutW", W.BOOL, W.HDC, C.c_int, C.c_int, W.LPCWSTR, C.c_int)
    get_path = api("GetPath", C.c_int, W.HDC, C.POINTER(Point), C.POINTER(W.BYTE), C.c_int)
    background = api("SetBkMode", C.c_int, W.HDC, C.c_int)
    data = C.create_string_buffer(font.read_bytes())
    count = W.DWORD()
    resource = add(data, len(data) - 1, None, C.byref(count))
    assert resource and count.value == 1, "MUA private GDI registration failed"
    dc = create_dc(None)
    assert dc
    rows = []
    try:
        assert background(dc, 1)
        for family in sorted(families):
            for underline in (0, 1):
                hfont = create(60 * 64, 0, 0, 0, 400, 0, underline, 0, 1, 4, 0, 4, 0, family)
                assert hfont
                previous = select(dc, hfont)
                assert previous
                try:
                    actual = C.create_unicode_buffer(256)
                    assert face(dc, 256, actual)
                    tm = Metrics()
                    assert metrics(dc, C.byref(tm))
                    for text in texts:
                        n = len(text.encode("utf-16-le")) // 2
                        size = Size()
                        assert extent(dc, text, n, C.byref(size))
                        glyphs = (W.WORD * n)()
                        assert indices(dc, text, n, glyphs, 1) != 0xFFFFFFFF
                        assert begin(dc) and text_out(dc, 0, 0, text, n) and end(dc)
                        length = get_path(dc, None, None, 0)
                        assert 0 < length < 20000, f"GDI path length {length}"
                        points, types = (Point * length)(), (W.BYTE * length)()
                        assert get_path(dc, points, types, length) == length
                        rows.append({"requested_family": family, "selected_primary_family": actual.value,
                                     "text": text, "underline": underline, "font_height_q6": 3840,
                                     "metrics": {name: getattr(tm, name) for name, _ in tm._fields_},
                                     "extent_q6": [size.x, size.y], "primary_glyph_indices": list(glyphs),
                                     "points_q6": [[p.x, p.y, t] for p, t in zip(points, types)]})
                finally:
                    select(dc, previous)
                    delete(hfont)
    finally:
        delete_dc(dc)
        remove(resource)
    return rows


def visible(raw, width, height):
    points = [(i % width, i // width) for i, a in enumerate(raw[3::4]) if a]
    return [min(x for x, y in points), min(y for x, y in points),
            max(x for x, y in points) + 1, max(y for x, y in points) + 1] if points else None


def main():
    assert os.environ.get("GITHUB_ACTIONS") == "true" and os.name == "nt"
    p = argparse.ArgumentParser()
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    corpus = Path(os.environ["TAIYOU_CORPUS"])
    release = Path(os.environ["RUNNER_TEMP"]) / "xy-vsfilter-release"
    binary = oracle.ROOT / "target/debug/taiyou.exe"
    example = oracle.ROOT / "target/debug/examples/mua_metrics.exe"
    base = oracle.mua_office_case()
    original = Path(base["source_external"]).read_text(encoding="utf-8")
    dll, probe, attempts = oracle.find_csri_dll(release, out)
    report = {"candidate_sha": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
              "purpose": "supplemental reductions; excluded from full-corpus acceptance counts",
              "dll": probe, "dll_probe_attempts": attempts, "cases": []}
    families = {"MUA Office"}
    for name, text in [("A-burmese", "စမ်းမယ်"), ("B-brackets", "〈〉"), ("C-combined", "〈စမ်းမယ်〉")]:
        for underline in (True, False):
            case = dict(base, id=f"mua-diagnosis/{name}/underline-{int(underline)}")
            source = out / (name + f"-u{int(underline)}.ass")
            source.write_text(original.replace("〈စမ်းမယ်〉", text).replace(r"\u1", r"\u1" if underline else r"\u0"), encoding="utf-8")
            case["source_external"] = str(source)
            xy = parallel.xy_case(case, corpus, dll, release, out / "xy")
            taiyou = parallel.taiyou_case(case, corpus, binary, out / "taiyou")
            assert xy["status"] == taiyou["status"] == "OK", (xy, taiyou)
            key = oracle.artifact_key(case["id"])
            xd, td = out / "xy/cases" / key, out / "taiyou/cases" / key
            diff = out / (key + "-diff.png")
            metrics = parallel.compare_raw_bytes(xd / "xy.rgba", td / "taiyou.rgba", 1920, 1080, diff)
            families.update(re.findall(r'font-catalog family="([^"]+)"', taiyou["render"]["log"]))
            shaped = subprocess.run([example, source, os.environ["MUA_OFFICE_FONT"]], capture_output=True, text=True, encoding="utf-8")
            (out / (key + "-shaping.txt")).write_text(shaped.stdout + shaped.stderr, encoding="utf-8")
            shaped.check_returncode()
            bounds = {}
            for label, path in [("xy", xd / "xy.rgba"), ("taiyou", td / "taiyou.rgba")]:
                raw = path.read_bytes()
                bounds[label] = visible(raw, 1920, 1080)
                # Small unscaled analytic views; the complete originals remain.
                box = [820, 480, 1100, 590]
                crop = b"".join(raw[(y * 1920 + box[0]) * 4:(y * 1920 + box[2]) * 4] for y in range(box[1], box[3]))
                oracle.preview_png(out / (key + f"-{label}-detail.png"), 280, 110, crop)
                path.unlink()  # retain bounded PNG/metrics rather than full raw buffers
            row = {"id": case["id"], "comparison": metrics, "visible_bbox": bounds,
                   "taiyou_log": taiyou["render"]["log"]}
            report["cases"].append(row)
            print("MUA_DIAGNOSIS=" + json.dumps(row, ensure_ascii=False), flush=True)
    gdi = gdi_metrics(Path(os.environ["MUA_OFFICE_FONT"]), ["စမ်းမယ်", "〈〉", "〈စမ်းမယ်〉"], families)
    (out / "gdi-metrics.json").write_text(json.dumps(gdi, ensure_ascii=False, indent=2), encoding="utf-8")
    (out / "report.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
