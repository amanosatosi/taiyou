"""Reuse completed supplemental samples only when measured inputs are unchanged.

This never controls correctness tests, the external comparator or CI selection.
Reuse retains the original candidate/environment; it is not a new measurement.
"""
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

if os.environ.get("GITHUB_ACTIONS") != "true" or platform.system() != "Linux":
    raise SystemExit("measurement is restricted to Linux GitHub Actions")
root = Path(__file__).resolve().parents[2]
measured = "f473e8190c34cc74e6718187b8772efba69c7ba2"
head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
subprocess.run(["git", "fetch", "--no-tags", "origin", measured], cwd=root, check=True)
changed = subprocess.check_output(["git", "diff", "--name-only", measured, "--"], cwd=root, text=True).splitlines()
allowed_files = {
    "crates/taiyou-cli/tests/collision_review.rs",
    "crates/taiyou-cli/tests/wrapping_workload_review.rs",
    "experiments/t017/reuse_or_measure.py",
    "experiments/t017/README.md",
}
unchanged_inputs = all(
    p.startswith(("docs/", "experiments/t017/results/")) or p in allowed_files
    for p in changed
)
if not unchanged_inputs:
    subprocess.run([sys.executable, root / "experiments/t017/measure.py", *sys.argv[1:]], check=True)
    raise SystemExit(0)

saved = root / "experiments/t017/results/f473e81"
samples = json.loads((saved / "samples.json").read_text())
environment = json.loads((saved / "environment.json").read_text())
assert environment["candidate"] == measured
assert environment["base"] == "9eee96fe16989089d8fd3f65b1bb136455ffca7e"
cases = {r["case"] for r in samples}
assert len(cases) == 8 and len(samples) == 48
for case in cases:
    rows = [r for r in samples if r["case"] == case]
    assert {(r["implementation"], r["sample"]) for r in rows} == {
        (implementation, sample) for implementation in ["before", "after"] for sample in range(3)
    }
    assert all(r["exit_code"] == 0 for r in rows)
    assert len({r["raw_sha256"] for r in rows}) == 1
    assert len({r["script_sha256"] for r in rows}) == 1
destination = Path(sys.argv[1]) / "T017-review-measurements-reused"
destination.mkdir(parents=True, exist_ok=False)
for source in saved.iterdir():
    shutil.copy2(source, destination / source.name)
provenance = {
    "status": "reused-original-measurements-not-reexecuted",
    "measured_candidate": measured,
    "current_candidate": head,
    "source_job": "https://github.com/amanosatosi/taiyou/actions/runs/39184889909/job/117845262915",
    "artifact_id": 12157799614,
    "artifact_zip_sha256": "2855e4129608e390f776ff6795273207b09ac520ef995faedd1fba768b8f636b",
    "differences": changed,
    "condition": "No changed production/build/measurement/input bytes; only named new review fixtures, docs and original sample evidence may differ",
}
(destination / "reuse-provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
print("T017_MEASUREMENT_REUSE=" + json.dumps(provenance), flush=True)
