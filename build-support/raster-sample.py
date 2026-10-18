"""CI-only provenance for a genuine A8 stage sample, never an approved golden."""
import hashlib
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
folder = Path(os.environ["TAIYOU_RASTER_SAMPLE_DIR"])
if os.environ.get("TAIYOU_RASTER_REFERENCE"):
    subprocess.run([os.environ["TAIYOU_RASTER_REFERENCE"], str(folder / "commands.q6"), str(folder / "expected.a8"), str(folder / "expected-info.txt")], check=True)
info = dict(line.split("=", 1) for line in (folder / "sample-info.txt").read_text().splitlines())
width, height, stride = (int(info[k]) for k in ["width", "height", "stride"])
data = (folder / "sample.a8").read_bytes()
existing = folder / "report.json"
if existing.exists():
    previous = json.loads(existing.read_text())
    if previous["mask_sha256"] != hashlib.sha256(data).hexdigest():
        raise RuntimeError("same-candidate sample changed between validation passes")
if not 0 < width <= stride or height <= 0 or len(data) != stride * height:
    raise RuntimeError("invalid sample extent")
if any(data[y * stride + width:(y + 1) * stride].strip(b"\0") for y in range(height)):
    raise RuntimeError("uninitialized sample row padding")
fonts = json.loads((root / "build-support/shaping-fonts.proposed.json").read_text())["fonts"]
font = next(f for f in fonts if f["id"] == "latin")
if info["font_sha256"] != font["sha256"] or int(info["face_index"]) != 0:
    raise RuntimeError("sample did not use the pinned Latin face")
candidate = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
if os.environ.get("CANDIDATE_SHA", candidate) != candidate:
    raise RuntimeError("sample candidate identity mismatch")
report = {
    "schema": 1, "task": "T012", "profile": "CoreV1", "candidate_sha": candidate,
    "status": "genuine stage sample, not an approved reference or VSFilter parity claim",
    "backend": info["backend"], "raster_flags": 7, "policy_revision": 1,
    "format": "A8, top-down linear geometric coverage 0 transparent / 255 full",
    "width": width, "height": height, "stride": stride,
    "bounds": [int(info["x0"]), int(info["y0"]), int(info["x0"]) + width, int(info["y0"]) + height],
    "mask_sha256": hashlib.sha256(data).hexdigest(),
    "source_sha256": hashlib.sha256((folder / "input.ass").read_bytes()).hexdigest(),
    "font": {k: font[k] for k in ["repository", "commit", "path", "sha256", "license", "metadata"]},
    "font_face_index": 0,
}
(folder / "report.json").write_text(json.dumps(report, indent=2) + "\n")
# PGM is a lossless diagnostic coverage preview, not a Taiyou PNG/frame pipeline.
pixels = b"".join(data[y * stride:y * stride + width] for y in range(height))
(folder / "coverage.pgm").write_bytes(f"P5\n{width} {height}\n255\n".encode("ascii") + pixels)
if (folder / "expected.a8").exists():
    expected_info = [int(n) for n in (folder / "expected-info.txt").read_text().split()]
    if expected_info != [width, height, int(info["x0"]), int(info["y0"])]:
        raise RuntimeError("independent bitmap API bounds mismatch")
    expected = (folder / "expected.a8").read_bytes()
    if len(expected) != len(pixels):
        raise RuntimeError("independent bitmap API length mismatch")
    difference = bytes(abs(a - b) for a, b in zip(expected, pixels))
    changed = [i for i, v in enumerate(difference) if v]
    metrics = {"reference": "independent C FT_Outline_Get_Bitmap over the same checked Q6 stage geometry", "reference_source_sha256": hashlib.sha256((root / "build-support/raster-reference.c").read_bytes()).hexdigest(), "max_delta": max(difference), "mae": sum(difference) / len(difference), "mismatch_count": len(changed), "changed_pixel_bounds": None if not changed else [min(i % width for i in changed), min(i // width for i in changed), max(i % width for i in changed) + 1, max(i // width for i in changed) + 1]}
    (folder / "comparison.json").write_text(json.dumps(metrics, indent=2) + "\n")
    (folder / "expected.pgm").write_bytes(f"P5\n{width} {height}\n255\n".encode("ascii") + expected)
    (folder / "difference.pgm").write_bytes(f"P5\n{width} {height}\n255\n".encode("ascii") + difference)
    print("T012_INDEPENDENT_API_COMPARISON=" + json.dumps(metrics), flush=True)
    if changed:
        raise RuntimeError("independent bitmap API pixels differ: zero tolerance")
print("T012_A8_SAMPLE=" + json.dumps(report), flush=True)
