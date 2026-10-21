"""Actions-only focused inspection; frozen inputs/references remain read-only."""
import hashlib
import json
import os
from pathlib import Path
import sys
assert os.environ.get("GITHUB_ACTIONS") == "true"
sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "validation"))
import images
root = Path(os.environ["TAIYOU_REVIEW_OUTPUT"])
expected = images.read_png(Path(os.environ["TAIYOU_CORPUS"]) / "regression/embedded-font/efont-1000.png")
rows = []
for label in ["current", "em-control", "shadow-control", "em-shadow-control"]:
    path = root / f"efont-{label}.rgba"
    rows.append({"case": label, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
                 "metrics": images.compare(expected, images.raw_rgba(path, 640, 480))})
assert rows[0]["sha256"] == "43a6dec427ef562180b88864d151fb8eefdefcad3ccedd5da07bb6a13c797ea1"
report = {"candidate": os.environ["CANDIDATE_SHA"], "font_sha256": hashlib.sha256((root / "efont.otf").read_bytes()).hexdigest(),
          "historical_8d_raw_sha256": "23e58edac8e2a559047873679e8610c01199632eb27027ac478319b0d43fd749", "rows": rows}
(root / "efont-comparison.json").write_text(json.dumps(report, indent=2) + "\n")
print(json.dumps(report))
