"""Supplemental performance audit; never changes the independent oracle result."""
import json
from pathlib import Path
import sys

before = json.loads(Path(sys.argv[1]).read_text())
after = json.loads(Path(sys.argv[2]).read_text())
assert before["corpus_revision"] == after["corpus_revision"], "different corpus"
assert before["accepted_baseline"] == after["accepted_baseline"], "different base"
old = {r["case"]: r["raw_sha256"] for r in before["frames"]}
visual = [r for r in after["records"] if r.get("frame")]
assert len(visual) == len({r["case"] for r in visual}), "duplicate frame IDs"
new = {r["case"]: r.get("raw_sha256") for r in visual}
changed = sorted(k for k in old.keys() | new.keys() if old.get(k) != new.get(k))
print(json.dumps({
    "before": before["candidate_sha"],
    "after": after["candidate_sha"],
    "frames_before": len(old), "frames_after": len(new),
    "missing": sorted(old.keys() - new.keys()),
    "added": sorted(new.keys() - old.keys()),
    "changed": changed,
    "independent_oracle_summary": after["summary"],
    "scope": "Same Taiyou output bytes only; does not accept oracle mismatches",
}, indent=2))
sys.exit(bool(changed) or old.keys() != new.keys() or not old)
