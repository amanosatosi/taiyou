"""Strictly pinned historical samples; changed candidates are always measured.

Owner authorization: T017 review follow-up, 2026-10-20. Not comparison policy.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import copy
import tomllib
from reuse_completed import ROOT, validate

TRUSTED = "f41261ddebc61b1ebb77e6175aba619f0d45a41e"
INDEX = "experiments/t017/results/completed.json"

def digest(data):
    return hashlib.sha256(data).hexdigest()

def checked(data, expected):
    assert digest(data) == expected, "historical evidence hash mismatch"


def checked_dbcs_lock(original, current):
    """Allow only the repair's pinned codec addition, never rewrite history."""
    before = tomllib.loads(original.decode())
    after = tomllib.loads(current.decode())
    expected = copy.deepcopy(before)
    codec = {"name":"encoding_rs", "version":"0.8.35",
             "source":"registry+https://github.com/rust-lang/crates.io-index",
             "checksum":"75030f3c4f45dafd7586dd6780965a8c7e8e285a5ecb86713e63a79c5b2766f3",
             "dependencies":["cfg-if"]}
    font = [p for p in expected["package"] if p["name"] == "taiyou-font"]
    assert len(font) == 1 and "encoding_rs" not in font[0]["dependencies"]
    font[0]["dependencies"] = sorted(font[0]["dependencies"] + ["encoding_rs"])
    expected["package"].append(codec)
    key = lambda p: (p["name"],p["version"])
    expected["package"].sort(key=key)
    after["package"].sort(key=key)
    assert expected == after, "dependency change exceeds pinned optional DBCS codec intake"

def historical(kind, destination):
    # The reviewed commit fixes the manifest itself, not just self-authenticated
    # candidate-controlled hashes. Fetch that one small Git object if necessary.
    subprocess.run(["git", "fetch", "--no-tags", "origin", TRUSTED], cwd=ROOT, check=True, capture_output=True)
    original = subprocess.check_output(["git", "show", TRUSTED+":"+INDEX], cwd=ROOT)
    # Read the index itself from the reviewed Git object. The mutable completed
    # index may subsequently advance to newly verified candidate measurements;
    # it is deliberately not an authority for these historical rows.
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
    # Candidate measurements always execute separately. Authenticate the old
    # dependencies as historical evidence; admit only the documented codec delta
    # and record both identities, never claim an unchanged measurement environment.
    dependencies = []
    for path in ["Cargo.lock","Cargo.toml","build-support/sources.lock.json","build-support/shaping-fonts.proposed.json"]:
        expected = subprocess.check_output(["git","show",TRUSTED+":"+path],cwd=ROOT)
        current = (ROOT/path).read_bytes()
        changed = current.replace(b"\r\n",b"\n") != expected.replace(b"\r\n",b"\n")
        if path == "Cargo.lock" and changed:
            checked_dbcs_lock(expected,current)
        else:
            assert not changed, path+" changed; historical reuse needs review"
        dependencies.append({"path":path,"historical_sha256":digest(expected),
                             "candidate_sha256":digest(current),"changed":changed})
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
    provenance = {"status":"historical-samples-reused-candidate-must-execute", "reviewed_manifest_commit":TRUSTED,"entry":entry,"recorded_environment":env,"dependency_policy":dependencies,"warning":"Historical CPU/OS/dependencies/samples retained exactly; candidate codec intake is separate and new samples are not same-run paired baseline measurements"}
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
    original = b'version=4\n[[package]]\nname="taiyou-font"\nversion="0.1.0"\ndependencies=["cfg-if"]\n[[package]]\nname="cfg-if"\nversion="1.0.5"\n'
    valid = original.replace(b'dependencies=["cfg-if"]',b'dependencies=["cfg-if","encoding_rs"]') + b'[[package]]\nname="encoding_rs"\nversion="0.8.35"\nsource="registry+https://github.com/rust-lang/crates.io-index"\nchecksum="75030f3c4f45dafd7586dd6780965a8c7e8e285a5ecb86713e63a79c5b2766f3"\ndependencies=["cfg-if"]\n'
    checked_dbcs_lock(original, valid)
    for changed in [valid.replace(b'0.8.35',b'0.8.36'),valid.replace(b'1.0.5',b'1.0.6'),valid.replace(b'75030f',b'000000'),original]:
        try:
            checked_dbcs_lock(original,changed)
        except AssertionError:
            pass
        else:
            raise AssertionError("unrelated dependency policy accepted")
    print("T017_HISTORY_NEGATIVE: corrupt/missing/workload and unrelated dependency changes rejected",flush=True)
