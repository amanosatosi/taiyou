"""Reuse only complete exact-source native-work observations, never correctness.
Negative/fallback/width equivalence tests execute before this optional research
reuse guard. Production/build/input changes return 2 to execute all measurements.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
from reuse_completed import ROOT, FONT, unchanged

CASES = {"latin-long","cjk-long","mixed-bidi","pathological-combining"}

def validate(rows):
    assert len(rows) == 4 and {r["case"] for r in rows} == CASES,"incomplete native-work evidence"
    for row in rows:
        assert row["status"] == "complete-experiment-not-production"
        assert all(isinstance(row[k],int) and row[k] > 0 for k in ["candidates","native_calls","context_bytes","range_bytes","raw_glyphs","elapsed_ns","candidate_storage_bytes"])
        assert row["context_bytes"] >= row["range_bytes"]

def self_test():
    rows = [json.loads(s) for s in (ROOT/"experiments/t017/results/dc83cab/T017-raw-native-work.jsonl").read_text().splitlines()]
    validate(rows)
    for bad in [rows[:-1],rows+[rows[0]],[dict(r,status="partial") if i == 0 else r for i,r in enumerate(rows)],[dict(r,context_bytes=0) if i == 0 else r for i,r in enumerate(rows)]]:
        try:
            validate(bad)
        except AssertionError:
            continue
        raise AssertionError("invalid native-work evidence accepted")
    print("T017_NATIVE_REUSE_NEGATIVE: incomplete, duplicate, partial and invalid work rejected",flush=True)

if __name__ == "__main__":
    assert os.environ.get("GITHUB_ACTIONS") == "true"
    self_test()
    if sys.argv[1] == "--self-test":
        raise SystemExit(0)
    entry = json.loads((ROOT/"experiments/t017/results/native-work-completed.json").read_text())
    measured = entry["candidate"]
    assert re.fullmatch(r"[0-9a-f]{40}",measured)
    differences = unchanged(measured,"native-work")
    if differences is None:
        print("T017_NATIVE_WORK: measured source/build changed; execute fresh candidates",flush=True)
        raise SystemExit(2)
    saved = (ROOT/entry["directory"]).resolve()
    assert saved.is_relative_to((ROOT/"experiments/t017/results").resolve())
    target = Path(sys.argv[1])/"T017-native-work-reused"
    target.mkdir(parents=True,exist_ok=False)
    for name,expected in entry["files"].items():
        assert not Path(name).is_absolute() and ".." not in Path(name).parts
        data = (saved/name).read_bytes()
        assert hashlib.sha256(data).hexdigest() == expected,"native-work payload hash mismatch: "+name
        destination = target/name
        destination.parent.mkdir(parents=True,exist_ok=True)
        destination.write_bytes(data)
    rows = [json.loads(s) for s in (target/"T017-raw-native-work.jsonl").read_text().splitlines()]
    validate(rows)
    environment = json.loads((target/"T017-review-measurements/environment.json").read_text())
    assert environment["candidate"] == measured and environment["font"]["sha256"] == FONT
    provenance = {"status":"completed-research-observations-reused-not-reexecuted","measured_candidate":measured,"current_candidate":subprocess.check_output(["git","rev-parse","HEAD"],cwd=ROOT,text=True).strip(),"evidence":entry,"allowed_differences":differences,"scope":"No production/build/native-work loop change; current correctness/negative tests still executed. Different source must execute fresh measurements."}
    (target/"reuse-provenance.json").write_text(json.dumps(provenance,indent=2)+"\n")
    print("T017_NATIVE_WORK_REUSE="+json.dumps(provenance),flush=True)
