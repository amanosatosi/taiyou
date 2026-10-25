"""Supplemental live libass renders of authored border controls; no corpus edits."""
import ctypes as C
import ctypes.util
import json
import os
from pathlib import Path
import subprocess

from PIL import Image

assert os.environ.get("GITHUB_ACTIONS") == "true"
root = Path(os.environ["TAIYOU_TRANSFORMED_BORDER_OUTPUT"])
ass = C.CDLL(ctypes.util.find_library("ass"))


class AssImage(C.Structure):
    pass


AssImage._fields_ = [("w", C.c_int), ("h", C.c_int), ("stride", C.c_int),
                    ("bitmap", C.POINTER(C.c_ubyte)), ("color", C.c_uint32),
                    ("x", C.c_int), ("y", C.c_int), ("next", C.POINTER(AssImage))]


def api(name, result, *args):
    fn = getattr(ass, name)
    fn.restype, fn.argtypes = result, args
    return fn


ptr, integer = C.c_void_p, C.c_int
library = api("ass_library_init", ptr)()
renderer = api("ass_renderer_init", ptr, ptr)(library)
assert library and renderer
font = Path(os.environ["TAIYOU_TEST_FONT"]).read_bytes()
api("ass_add_font", None, ptr, C.c_char_p, C.c_char_p, integer)(library, b"explicit.otf", font, len(font))
api("ass_set_fonts", None, ptr, C.c_char_p, C.c_char_p, integer, C.c_char_p, integer)(renderer, None, b"Source Sans 3", 0, None, 1)
api("ass_set_frame_size", None, ptr, integer, integer)(renderer, 320, 240)
api("ass_set_storage_size", None, ptr, integer, integer)(renderer, 320, 240)
read = api("ass_read_memory", ptr, ptr, C.c_char_p, C.c_size_t, C.c_char_p)
render = api("ass_render_frame", C.POINTER(AssImage), ptr, ptr, C.c_longlong, C.POINTER(integer))
free = api("ass_free_track", None, ptr)
report = {"libass_version": api("ass_library_version", integer)(),
          "package": subprocess.check_output(["dpkg-query", "-W", "libass9"], text=True),
          "font_source": "explicit supplied font bytes; provider=NONE", "cases": []}
for path in sorted(root.glob("*.ass")):
    data = path.read_bytes()
    track = read(library, data, len(data), None)
    assert track
    image = Image.new("RGBA", (320, 240))
    node = render(renderer, track, 500, None)
    while node:
        row = node.contents
        if row.w and row.h:
            opacity = 255 - (row.color & 255)
            bitmap = C.string_at(row.bitmap, row.stride * row.h)
            alpha = bytes(bitmap[y * row.stride + x] * opacity // 255
                          for y in range(row.h) for x in range(row.w))
            layer = Image.new("RGBA", (row.w, row.h),
                              (row.color >> 24, row.color >> 16 & 255, row.color >> 8 & 255))
            layer.putalpha(Image.frombytes("L", (row.w, row.h), alpha))
            image.alpha_composite(layer, (row.x, row.y))
        node = row.next
    free(track)
    image.save(root / (path.stem + "-libass.png"))
    taiyou = Image.open(path.with_suffix(".png")).convert("RGBA")
    a, b = image.getchannel("A"), taiyou.getchannel("A")
    # Keep literal pixel differences visible; no new tolerance or pass bucket.
    differences = sum(x != y for x, y in zip(image.getdata(), taiyou.getdata()))
    report["cases"].append({"id": path.stem, "libass_bbox": a.getbbox(),
                            "taiyou_bbox": b.getbbox(), "changed_pixels": differences})
for kind in ("drawing", "text"):
    for axis, coordinate in (("x", 3), ("y", 2)):
        boxes = [next(c for c in report["cases"] if c["id"] == f"{kind}-{name}")["libass_bbox"]
                 for name in ("zero", axis + "45", axis + "80", axis + "89", axis + "90")]
        widths = [0 if b is None else b[coordinate] - b[coordinate - 2] for b in boxes]
        assert all(b < a for a, b in zip(widths, widths[1:])), (kind, axis, widths)
api("ass_renderer_done", None, ptr)(renderer)
api("ass_library_done", None, ptr)(library)
(root / "libass-report.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report))
