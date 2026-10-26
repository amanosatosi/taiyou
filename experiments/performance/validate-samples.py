"""Supplemental exact digest gate for identical benchmark timestamp sequences."""
import csv
import json
from pathlib import Path
import sys

rows = list(csv.DictReader(Path(sys.argv[1]).open()))
key_fields = ("workload", "width", "height", "scenario", "sample")
modes = (("off", "1"), ("on", "1"), ("on", "2"), ("on", "4"), ("on", "8"))
by_mode = {mode: {} for mode in modes}
for row in rows:
    mode = (row["cache"], row["workers"])
    key = tuple(row[k] for k in key_fields)
    assert key not in by_mode[mode], f"duplicate sample: {mode}/{key}"
    assert len(row["rgba_sha256"]) == len(row["diagnostic_sha256"]) == 64
    by_mode[mode][key] = (row["rgba_sha256"], row["diagnostic_sha256"])
reference = by_mode[("off", "1")]
assert reference, "no samples"
for mode, observed in by_mode.items():
    assert observed.keys() == reference.keys(), f"missing/extra samples: {mode}"
    different = [key for key in reference if observed[key] != reference[key]]
    assert not different, f"output differences in {mode}: {different}"
print(json.dumps({"samples_per_mode": len(reference), "modes": modes,
                  "different_rgba_or_diagnostic_digests": 0,
                  "note": "SHA-256 frame audit; supplemental tests compare bytes directly"}))
