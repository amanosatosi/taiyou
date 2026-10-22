"""Diagnostic alternatives, never reference regeneration or acceptance criteria.

All images use the existing comparison math. The pre-Gaussian conversion is
intentionally a counterfactual: pinterf passes raw overlay bytes to float.
"""
import hashlib
import json
import os
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "validation"))
from images import compare, raw_rgba, read_png

root = Path(os.environ["TAIYOU_BE_AUDIT_OUTPUT"])
corpus = Path(os.environ["TAIYOU_BE_AUDIT_CORPUS"])
records = []
for ms in range(1000, 2000, 100):
    expected_path = corpus / f"regression/blurs/blur+t-{ms}.png"
    expected = read_png(expected_path)
    w, h, pixels = expected
    alpha_levels = sorted(set(pixels[3::4]))
    record = {"timestamp_ms": ms, "reference_sha256": hashlib.sha256(expected_path.read_bytes()).hexdigest(),
              "reference_alpha_levels": alpha_levels, "variants": {}}
    for label in ["ft-a8", "xy64", "xy-converted-before-gaussian", "xy-tight"]:
        path = root / f"gaussian-{ms}-{label}.rgba"
        data = path.read_bytes()
        actual = raw_rgba(path, w, h)
        metrics = compare(expected, actual)
        record["variants"][label] = {"raw_sha256": hashlib.sha256(data).hexdigest(),
                                     "alpha_levels": sorted(set(actual[2][3::4])), "metrics": metrics}
    records.append(record)
    print(ms, [(k, v["metrics"]["changed_pixels"], round(v["metrics"]["channel_mae"][3], 5))
               for k, v in record["variants"].items()])
(root / "gaussian-unit-controls.json").write_text(json.dumps({"candidate": os.environ.get("GITHUB_SHA"),
    "meaning": "diagnostic alternatives; raw native overlay remains production; no reference or threshold changed",
    "records": records}, indent=2) + "\n")
