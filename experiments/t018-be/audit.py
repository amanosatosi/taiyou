"""CI-only additive measurements; never modifies corpus or comparator results."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
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

def style_control(source, factor, hide_shadow, half_blank, no_be=False):
    rows = []
    for line in source.splitlines():
        if line.startswith("Style:"):
            fields = line[6:].strip().split(",")
            fields[2] = format(float(fields[2]) * factor, ".12g")
            if hide_shadow:
                fields[6] = "&HFF000000"
            line = "Style: " + ",".join(fields)
        if line.startswith("Dialogue:") and half_blank:
            line = line.replace(r"\N\N", r"\N{\fs" + format(50*factor,".12g") + r"}\N{\fs" + format(100*factor,".12g") + "}")
        if line.startswith("Dialogue:") and no_be:
            import re
            line = re.sub(r"\\be[0-9]+", r"\\be0", line)
        rows.append(line)
    return "\n".join(rows) + "\n"

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
source = (CORPUS / "regression/blurs/be.ass").read_text(encoding="utf-8-sig")
summary = {"candidate": os.environ["CANDIDATE_SHA"], "font": font_info, "frames": []}
binary = ROOT / "target/debug/taiyou-cli"
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
    for name, scale, hide, half, no_be in [
        ("shadow-off",1.0,True,False,False),
        ("shadow-off-cell-height",factor,True,False,False),
        ("shadow-off-cell-height-half-blank",factor,True,True,False),
        ("shadow-off-cell-height-half-blank-no-be",factor,True,True,True),
    ]:
        text = style_control(source,scale,hide,half,no_be)
        ass, raw, png = OUT / f"{ms}-{name}.ass", OUT / f"{ms}-{name}.rgba", OUT / f"{ms}-{name}.png"
        ass.write_text(text,encoding="utf-8")
        command = [str(binary),"render","--input",str(ass),"--time-us",str(ms*1000),"--width","800","--height","600","--profile","core-v1","--output",str(png),"--raw",str(raw),"--font",str(font_path)]
        run = subprocess.run(command,capture_output=True,text=True,timeout=45)
        assert run.returncode == 0, run.stderr
        row["controls"][name] = measured(expected,images.raw_rgba(raw,800,600))
        row["controls"][name]["raw_sha256"] = sha(raw.read_bytes())
        if name == "shadow-off":
            assert raw.read_bytes() == (OUT / f"{ms}-single-fill.rgba").read_bytes()
    for label, image in [("expected",expected),("actual",actual)]:
        alpha = bytearray()
        for a in image[2][3::4]:
            a = (a + 128)//257
            alpha.extend((a,a,a,255))
        images.write_png(OUT / f"{ms}-{label}-alpha.png",800,600,alpha)
    summary["frames"].append(row)
(OUT / "summary.json").write_text(json.dumps(summary,indent=2))
print(json.dumps(summary,indent=2))
