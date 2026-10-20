"""CI-only additive measurements; never modifies corpus or comparator results."""
import hashlib
import json
import os
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "validation"))
import images
from fontTools.ttLib import TTFont

CORPUS = Path(os.environ["TAIYOU_CORPUS"])
OUT = Path(os.environ["TAIYOU_BE_AUDIT_OUTPUT"])
OUT.mkdir(exist_ok=True, parents=True)

def sha(data):
    return hashlib.sha256(data).hexdigest()

def stats(image):
    w, h, pixels = image
    points = [(i % w, i // w) for i, a in enumerate(pixels[3::4]) if a]
    bands, active = [], None
    rows = {y for _, y in points}
    for y in range(h + 1):
        if y in rows and active is None:
            active = y
        if y not in rows and active is not None:
            bands.append([active, y])
            active = None
    return {"nontransparent": len(points), "bbox": None if not points else
            [min(x for x,y in points), min(y for x,y in points), max(x for x,y in points)+1, max(y for x,y in points)+1],
            "row_bands": bands, "alpha_max": max(pixels[3::4], default=0)}

def measured(expected, actual):
    e, a = expected[2][3::4], actual[2][3::4]
    return {"expected": stats(expected), "actual": stats(actual),
            "metrics": images.compare(expected, actual),
            "only_expected": sum(x != 0 and y == 0 for x,y in zip(e,a)),
            "only_actual": sum(y != 0 and x == 0 for x,y in zip(e,a)),
            "both_nonzero": sum(y != 0 and x != 0 for x,y in zip(e,a))}

font_path = CORPUS / "regression/.fonts/Aileron-Regular.otf"
font = TTFont(font_path)
os2, head, hhea = font["OS/2"], font["head"], font["hhea"]
factor = head.unitsPerEm / (os2.usWinAscent + os2.usWinDescent)
font_info = {"sha256": sha(font_path.read_bytes()), "upem": head.unitsPerEm,
             "win_ascent": os2.usWinAscent, "win_descent": os2.usWinDescent,
             "hhea_ascent": hhea.ascent, "hhea_descent": hhea.descent,
             "cell_to_em_factor": factor,
             "glyphs": {c: {"name": font.getBestCmap()[ord(c)], "advance": font["hmtx"][font.getBestCmap()[ord(c)]][0]} for c in ".+¶ "}}
(OUT / "font.json").write_text(json.dumps(font_info, indent=2))
report = json.loads((Path(os.environ["RUNNER_TEMP"]) / "taiyou-comparison/report.json").read_text())
summary = {"candidate": os.environ["CANDIDATE_SHA"], "font": font_info, "frames": []}
for record in report["records"]:
    frame = record["case"]
    ms = int(Path(frame).stem.split("-")[-1])
    expected = images.read_png(CORPUS / frame)
    actual = images.read_png(Path(os.environ["RUNNER_TEMP"]) / "taiyou-comparison" / record["artifacts"]["actual"])
    row = {"case": frame, "raw_sha256": record["raw_sha256"], "baseline": measured(expected, actual), "controls": {}}
    # Real renderer path and native-stage reconstruction must have the same raw hash.
    assert sha((OUT / f"{ms}-actual.rgba").read_bytes()) == record["raw_sha256"]
    one = images.raw_rgba(OUT / f"{ms}-single-fill.rgba",800,600)
    row["controls"]["single-fill-same-mask"] = measured(expected,one)
    units = images.raw_rgba(OUT / f"{ms}-units-only-control.rgba",800,600)
    row["controls"]["units-only-not-xy-raster"] = measured(expected,units)
    for label, image in [("expected",expected),("actual",actual)]:
        alpha = bytearray()
        for a in image[2][3::4]:
            a = (a + 128)//257
            alpha.extend((a,a,a,255))
        images.write_png(OUT / f"{ms}-{label}-alpha.png",800,600,alpha)
    summary["frames"].append(row)
(OUT / "summary.json").write_text(json.dumps(summary,indent=2))
print(json.dumps(summary,indent=2))
