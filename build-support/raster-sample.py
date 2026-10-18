"""CI-only provenance for a genuine A8 stage sample, never an approved golden."""
import hashlib
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
folder = Path(os.environ["TAIYOU_RASTER_SAMPLE_DIR"])
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
print("T012_A8_SAMPLE=" + json.dumps(report), flush=True)
