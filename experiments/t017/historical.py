"""Strictly pinned historical samples; changed candidates are always measured.

Owner authorization: T017 review follow-up, 2026-10-20. Not comparison policy.
"""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
from reuse_completed import ROOT, validate

TRUSTED = "f41261ddebc61b1ebb77e6175aba619f0d45a41e"
INDEX = "experiments/t017/results/completed.json"

def digest(data):
    return hashlib.sha256(data).hexdigest()

def checked(data, expected):
    assert digest(data) == expected, "historical evidence hash mismatch"

def historical(kind, destination):
    index_bytes = (ROOT / INDEX).read_bytes()
    # The reviewed commit fixes the manifest itself, not just self-authenticated
    # candidate-controlled hashes. Fetch that one small Git object if necessary.
    subprocess.run(["git", "fetch", "--no-tags", "origin", TRUSTED], cwd=ROOT, check=True, capture_output=True)
    original = subprocess.check_output(["git", "show", TRUSTED+":"+INDEX], cwd=ROOT)
    assert index_bytes.replace(b"\r\n",b"\n") == original.replace(b"\r\n",b"\n"), "reviewed evidence index changed"
    entry = json.loads(original)["sets"][kind]
    saved = (ROOT / entry["directory"]).resolve()
    assert saved.is_relative_to((ROOT / "experiments/t017/results").resolve())
    target = destination / "historical"
    target.mkdir(parents=True,exist_ok=False)
    for name, expected in entry["files"].items():
        assert Path(name).name == name
        data = (saved/name).read_bytes()
        checked(data,expected)
        (target/name).write_bytes(data)
    env = json.loads((target/"environment.json").read_text())
    rows = json.loads((target/"samples.json").read_text()) if kind == "debug" else [json.loads(s) for s in (target/"samples.jsonl").read_text().splitlines()]
    validate(kind,rows,env,entry["candidate"])
    # Historical measurements cannot authenticate changed dependency/tool policy.
    for path in ["Cargo.lock","Cargo.toml","build-support/sources.lock.json","build-support/shaping-fonts.proposed.json"]:
        expected = subprocess.check_output(["git","show",TRUSTED+":"+path],cwd=ROOT)
        assert (ROOT/path).read_bytes().replace(b"\r\n",b"\n") == expected.replace(b"\r\n",b"\n"), path+" changed; historical reuse needs review"
    labels = {"before"} if kind == "debug" else {"earlier","reviewed","earlier-4Msegments","reviewed-4Msegments"}
    selected = [dict(r,evidence_origin="reused-historical",source_job=entry["source_job"]) for r in rows if r["implementation"] in labels]
    expected_labels = ["before"] if kind == "debug" else ["earlier","reviewed"]
    for label in expected_labels:
        instrument = json.loads((target/(label+"-instrumentation.json")).read_text())
        sha = instrument["sha"] if kind == "debug" else instrument["commit"]
        subprocess.run(["git","fetch","--no-tags","origin",sha],cwd=ROOT,check=True,capture_output=True)
        files = {"crates/taiyou-render/src/renderer.rs": {"original":instrument["source_sha256"]}} if kind == "debug" else instrument["files"]
        for path, facts in files.items():
            if "original" not in facts:
                continue # Generated measurement driver: authenticated above.
            source = subprocess.check_output(["git","show",sha+":"+path],cwd=ROOT).decode().replace("\r\n","\n").encode()
            checked(source,facts["original"])
    provenance = {"status":"historical-samples-reused-candidate-must-execute", "reviewed_manifest_commit":TRUSTED,"entry":entry,"recorded_environment":env,"warning":"Historical CPU/OS/samples retained exactly; new samples are not same-run paired baseline measurements"}
    (destination/"historical-reuse.json").write_text(json.dumps(provenance,indent=2)+"\n")
    return selected

def assert_workload(rows, name, data):
    selected = [r for r in rows if r["case"] == name]
    assert selected and {r["script_sha256"] for r in selected} == {digest(data)}, "historical workload mismatch"

def negative_tests():
    # Fail closed on corrupt bytes, mismatched workload, and incomplete evidence.
    for data, expected in [(b"corrupt",digest(b"original")),(b"",digest(b"missing"))]:
        try:
            checked(data,expected)
        except AssertionError:
            pass
        else:
            raise AssertionError("bad history accepted")
    for rows in [[],[{"case":"x","script_sha256":digest(b"other")}]]:
        try:
            assert_workload(rows,"x",b"expected")
        except AssertionError:
            pass
        else:
            raise AssertionError("missing/mismatched workload accepted")
    print("T017_HISTORY_NEGATIVE: corrupt, missing and mismatched evidence rejected",flush=True)
