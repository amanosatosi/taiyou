"""Supplemental live libass renders of authored border controls; no corpus edits."""
import ctypes as C
import ctypes.util
import json
import os
from pathlib import Path
import subprocess
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "validation"))
import images

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


def bbox(raw):
    points = [i // 4 for i in range(3, len(raw), 4) if raw[i]]
    if not points:
        return None
    return [min(p % 320 for p in points), min(p // 320 for p in points),
            max(p % 320 for p in points) + 1, max(p // 320 for p in points) + 1]


for path in sorted(root.glob("*.ass")):
    data = path.read_bytes()
    track = read(library, data, len(data), None)
    assert track
    image = bytearray(320 * 240 * 4)
    node = render(renderer, track, 500, None)
    while node:
        row = node.contents
        if row.w and row.h:
            opacity = 255 - (row.color & 255)
            color = (row.color >> 24, row.color >> 16 & 255, row.color >> 8 & 255)
            address = C.cast(row.bitmap, ptr).value
            for y in range(row.h):
                # libass guarantees w valid bytes, not padding after the last row.
                bitmap = C.string_at(address + y * row.stride, row.w)
                for x, coverage in enumerate(bitmap):
                    xx, yy = row.x + x, row.y + y
                    if not (0 <= xx < 320 and 0 <= yy < 240):
                        continue
                    alpha = coverage * opacity // 255
                    at = (yy * 320 + xx) * 4
                    for channel in range(3):
                        image[at + channel] = (color[channel] * alpha + image[at + channel] * (255 - alpha) + 127) // 255
                    image[at + 3] = alpha + (image[at + 3] * (255 - alpha) + 127) // 255
        node = row.next
    free(track)
    straight = bytearray(image)
    for at in range(0, len(image), 4):
        alpha = image[at + 3]
        if alpha:
            for channel in range(3):
                straight[at + channel] = min(255, (image[at + channel] * 255 + alpha // 2) // alpha)
    images.write_png(root / (path.stem + "-libass.png"), 320, 240, bytes(straight))
    taiyou = path.with_suffix(".rgba").read_bytes()
    # Keep literal pixel differences visible; no new tolerance or pass bucket.
    differences = sum(image[i:i + 4] != taiyou[i:i + 4] for i in range(0, len(image), 4))
    report["cases"].append({"id": path.stem, "libass_bbox": bbox(image),
                            "taiyou_bbox": bbox(taiyou), "changed_pixels": differences})
(root / "libass-report.json").write_text(json.dumps(report, indent=2) + "\n")
for kind in ("drawing", "text"):
    for axis, coordinate in (("x", 3), ("y", 2)):
        boxes = [next(c for c in report["cases"] if c["id"] == f"{kind}-{name}")["libass_bbox"]
                 for name in ("zero", axis + "45", axis + "80", axis + "89", axis + "90")]
        widths = [0 if b is None else b[coordinate] - b[coordinate - 2] for b in boxes]
        assert all(b < a for a, b in zip(widths, widths[1:])), (kind, axis, widths)
api("ass_renderer_done", None, ptr)(renderer)
api("ass_library_done", None, ptr)(library)
print(json.dumps(report))
