"""Owner-authorized reuse of original completed performance evidence only.

No correctness test, external comparison, selector or required gate is controlled
here. A changed measured input executes the experiment again. Corrupt/incomplete
saved evidence fails explicitly. Reuse never claims current-head measurements.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
REVIEWED = "027e58a52efdd870b99f49ab5f3bae6e952f8f59"
EARLIER = "9eee96fe16989089d8fd3f65b1bb136455ffca7e"
FONT = "08df266400933d3178d081a45f94a08814c3e55b4b7dd2e0ff69cb1329f13ab6"
DEBUG_CASES = {"normal-one", "normal-two", "normal-eight", "positioned-5000-one-glyph", "mixed-2000-one-glyph", "dense-100", "dense-500", "dense-1000"}
RELEASE_CASES = {"normal-one", "normal-eight", "dense-100", "dense-500", "dense-1000", "mixed-2000", "positioned-5000", "adaptive-256-multiglyph", "typesetting-hd-100"} | {f"typesetting-{n}-border-{b}" for n in [100, 500] for b in [0.5, 3]}

def validate(kind, rows, environment, measured):
    assert re.fullmatch(r"[0-9a-f]{40}", measured)
    cell_policy = environment.get("pixel_identity_policy")
    assert cell_policy in (None, "positive-cell-height-v2")
    if cell_policy and kind == "debug":
        assert environment["metric_font"] == {"sha256":FONT, "upem":1000, "win_ascent":1000, "win_descent":326}
    elif cell_policy:
        assert kind == "optimized"
        assert environment["cell_identity"]["historical_sha256"] == "9171c7aba2e228f4ea715a86eb8d2108a13d62bc01fbe17cf5a40bbd5511d032"
        assert environment["cell_identity"]["current_sha256"] == "e99750fabb0ffd5ce301ee971c47100f97c546430f18809fa949feb178538b20"
        assert environment["cell_identity"]["native_em_q6"] == 884
    expected_cases = DEBUG_CASES if kind == "debug" else RELEASE_CASES
    assert {r["case"] for r in rows} == expected_cases
    assert environment["font"]["sha256"] == FONT
    if kind == "debug":
        assert environment["candidate"] == measured and environment["base"] == EARLIER
    else:
        assert environment["versions"] == {"earlier": EARLIER, "reviewed": REVIEWED, "candidate": measured}
    for case in expected_cases:
        selected = [r for r in rows if r["case"] == case]
        implementations = {"before", "after"} if kind == "debug" else {"earlier", "reviewed", "candidate"}
        if kind != "debug" and case == "adaptive-256-multiglyph":
            implementations |= {"candidate-adaptive-32MiB", "candidate-minimal-32MiB", "candidate-minimal-128MiB"}
        if kind != "debug" and case.startswith("typesetting-500-"):
            implementations |= {"earlier-4Msegments", "reviewed-4Msegments", "candidate-4Msegments"}
        slots = {(label, sample) for label in implementations for sample in range(3)}
        assert len(selected) == len(slots)
        assert {(r["implementation"], r["sample"]) for r in selected} == slots
        assert all(r["elapsed_ns"] > 0 and r["peak_rss_kib"] > 0 for r in selected)
        for row in selected:
            if row["exit_code"] != 0:
                assert kind != "debug" and row["status"] == "renderer-error-no-frame"
                assert row["raw_sha256"] is None and "LimitExceeded" in row["error"]
            else:
                assert re.fullmatch(r"[0-9a-f]{64}", row["raw_sha256"])
        if kind != "debug":
            assert all(r["commit"] == environment["versions"][r["implementation"].split("-")[0]] for r in selected)
        assert all(re.fullmatch(r"[0-9a-f]{64}", r["script_sha256"]) for r in selected)
        assert len({r["script_sha256"] for r in selected}) == 1
        if kind != "debug":
            for reviewed in [r for r in selected if r["implementation"].startswith("reviewed") and r["exit_code"] == 0]:
                assert all(r["exit_code"] == 0 for r in selected if r["implementation"].startswith("candidate") and r["path_segments_per_frame"] == reviewed["path_segments_per_frame"])
        comparable = selected if kind == "debug" else [r for r in selected if not r["implementation"].startswith("earlier") and r["exit_code"] == 0]
        cohorts = [[r for r in comparable if r["implementation"].startswith(prefix)]
                   for prefix in (("before", "after") if kind == "debug" else ("reviewed", "candidate"))] if cell_policy else [comparable]
        for cohort in cohorts:
            assert len({r["raw_sha256"] for r in cohort}) <= 1

def unchanged(measured, kind):
    subprocess.run(["git", "fetch", "--no-tags", "origin", measured, REVIEWED], cwd=ROOT, check=True, capture_output=True)
    for sha in [measured, REVIEWED]:
        subprocess.run(["git", "cat-file", "-e", sha + "^{commit}"], cwd=ROOT, check=True)
    entries = subprocess.check_output(["git", "diff", "--name-status", "--no-renames", measured, "--"], cwd=ROOT, text=True).splitlines()
    changed = []
    for entry in entries:
        status, path = entry.split("\t")
        changed.append(path)
        # The sanitizer executor runs after performance sampling, in a separate
        # target/prefix/environment. Its discovery fix cannot change these builds.
        if path == "build-support/font-sanitizer.py":
            continue
        # This private module is included only under cfg(all(test, native)).
        # The benchmark's cargo build --bin never compiles unit-test fixtures;
        # changing its production inclusion in renderer.rs still rejects reuse.
        if path == "crates/taiyou-render/src/renderer/staging_tests.rs":
            continue
        # Separate prerequisite job/data never enters these measured binaries,
        # workload builders, native prefix or benchmark process environment.
        # Any corresponding production include/build change still rejects reuse.
        if path == ".github/workflows/t017-prerequisites.yml" or path.startswith("experiments/t017/unicode15_1/"):
            continue
        if kind == "debug" and path == "experiments/t017/optimized.py":
            continue
        if kind == "optimized" and path == "experiments/t017/measure.py":
            continue
        if path.startswith(("docs/", "experiments/t017/results/")) or path in {"experiments/t017/reuse_completed.py", "experiments/t017/reuse_or_measure.py", "experiments/t017/historical.py", "experiments/t017/reuse_native_work.py", "experiments/t017/README.md"}:
            continue
        # Only independently added tests, absent from the reviewed candidate.
        if path.startswith("crates/") and "/tests/" in path and path.endswith(".rs") and status != "D":
            existed = subprocess.run(["git", "cat-file", "-e", REVIEWED + ":" + path], cwd=ROOT, capture_output=True)
            if existed.returncode != 0 and (status == "A" or path == "crates/taiyou-cli/tests/adaptive_optimized_performance.rs"):
                continue
            if existed.returncode != 0 and status == "M":
                # Owner-authorized additive experiment tests: every original
                # byte/function/assertion must remain the unchanged file prefix.
                original = subprocess.check_output(["git", "show", measured + ":" + path], cwd=ROOT)
                current = (ROOT / path).read_bytes()
                if path == "crates/taiyou-cli/tests/adaptive_wrapping_width_experiment.rs":
                    # Only the explicit reuse guard plus appended executor
                    # wrapper may differ. All original measurement/assertion
                    # bytes must still survive in their original order.
                    current = current.split(b"// T017 native-work evidence reuse wrapper")[0]
                    current = current.replace(b"    if reuse_completed_native_work(&folder) {\n        return;\n    }\n",b"")
                if current.startswith(original):
                    continue
        return None
    return changed

def try_reuse(kind, folder):
    assert os.environ.get("GITHUB_ACTIONS") == "true"
    index_path = ROOT / "experiments/t017/results/completed.json"
    if not index_path.exists():
        return False
    index = json.loads(index_path.read_text())
    assert index["version"] == 1
    if kind not in index["sets"]:
        return False
    entry = index["sets"][kind]
    measured = entry["candidate"]
    changed = unchanged(measured, kind)
    if changed is None:
        return False
    saved = (ROOT / entry["directory"]).resolve()
    assert saved.is_relative_to((ROOT / "experiments/t017/results").resolve())
    assert entry["files"] and re.fullmatch(r"[0-9a-f]{64}", entry["artifact_zip_sha256"])
    for name, digest in entry["files"].items():
        assert Path(name).name == name and re.fullmatch(r"[0-9a-f]{64}", digest)
        assert (saved / name).stat().st_size <= 4 * 1024 * 1024
        assert hashlib.sha256((saved / name).read_bytes()).hexdigest() == digest, name
    samples_name = "samples.json" if kind == "debug" else "samples.jsonl"
    assert samples_name in entry["files"] and "environment.json" in entry["files"]
    rows = json.loads((saved / samples_name).read_text()) if kind == "debug" else [json.loads(line) for line in (saved / samples_name).read_text().splitlines()]
    validate(kind, rows, json.loads((saved / "environment.json").read_text()), measured)
    target = Path(folder) / ("T017-review-measurements-reused" if kind == "debug" else "T017-optimized-reused")
    target.mkdir(parents=True, exist_ok=False)
    for name in entry["files"]:
        shutil.copyfile(saved / name, target / name)
    provenance = {"status": "original-completed-evidence-reused-not-reexecuted", "kind": kind, "measured_candidate": measured, "current_candidate": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(), "source_job": entry["source_job"], "artifact_id": entry["artifact_id"], "artifact_zip_sha256": entry["artifact_zip_sha256"], "allowed_differences": changed, "scope": "performance observations only; all current correctness gates and full comparator still run"}
    (target / "reuse-provenance.json").write_text(json.dumps(provenance, indent=2) + "\n")
    print("T017_COMPLETED_REUSE=" + json.dumps(provenance), flush=True)
    return True

def self_test():
    saved = ROOT / "experiments/t017/results/f473e81"
    rows = json.loads((saved / "samples.json").read_text())
    environment = json.loads((saved / "environment.json").read_text())
    validate("debug", rows, environment, environment["candidate"])
    broken = [rows[:-1], rows + [rows[0]], [dict(r, exit_code=32) if i == 0 else r for i, r in enumerate(rows)], [dict(r, raw_sha256="0" * 64) if i == 0 else r for i, r in enumerate(rows)]]
    for candidate in broken:
        try:
            validate("debug", candidate, environment, environment["candidate"])
        except AssertionError:
            continue
        raise AssertionError("invalid performance evidence was accepted")
    cell_debug = dict(environment, pixel_identity_policy="positive-cell-height-v2",
                      metric_font={"sha256":FONT, "upem":1000, "win_ascent":1000, "win_descent":326})
    validate("debug", rows, cell_debug, environment["candidate"])
    for candidate in broken:
        try:
            validate("debug", candidate, cell_debug, environment["candidate"])
        except AssertionError:
            continue
        raise AssertionError("invalid cell-height debug evidence accepted")
    try:
        validate("debug", rows, dict(cell_debug, metric_font={}), environment["candidate"])
    except AssertionError:
        pass
    else:
        raise AssertionError("unverified metric font accepted")
    # Old evidence keeps its original checks. The metric revision only permits
    # cross-policy differences; corruption within the current policy must fail.
    saved = ROOT / "experiments/t017/results/dc83cab/T017-optimized"
    release = [json.loads(line) for line in (saved / "samples.jsonl").read_text().splitlines()]
    env = json.loads((saved / "environment.json").read_text())
    validate("optimized", release, env, env["versions"]["candidate"])
    env.update(pixel_identity_policy="positive-cell-height-v2", cell_identity={
        "historical_sha256":"9171c7aba2e228f4ea715a86eb8d2108a13d62bc01fbe17cf5a40bbd5511d032",
        "current_sha256":"e99750fabb0ffd5ce301ee971c47100f97c546430f18809fa949feb178538b20",
        "native_em_q6":884})
    validate("optimized", release, env, env["versions"]["candidate"])
    invalid = [release[:-1], release + [release[0]],
               [dict(r, raw_sha256="0"*64) if r["implementation"] == "candidate" and r["sample"] == 0 else r for r in release],
               [dict(r, exit_code=32) if r["implementation"] == "candidate" and r["sample"] == 0 else r for r in release]]
    for bad in invalid:
        try:
            validate("optimized", bad, env, env["versions"]["candidate"])
        except AssertionError:
            continue
        raise AssertionError("invalid cell-metric evidence was accepted")
    for bad_identity in [{}, dict(env["cell_identity"], current_sha256="0"*64), dict(env["cell_identity"], native_em_q6=1280)]:
        try:
            validate("optimized", release, dict(env, cell_identity=bad_identity), env["versions"]["candidate"])
        except (AssertionError, KeyError):
            continue
        raise AssertionError("invalid cell identity was accepted")
    print("T017_REUSE_SELF_TEST: missing, duplicate, failed and mismatched raw samples rejected for both metric policies", flush=True)

if __name__ == "__main__":
    assert os.environ.get("GITHUB_ACTIONS") == "true"
    self_test()
    kind, folder = sys.argv[1:]
    assert kind in {"debug", "optimized"}
    if not try_reuse(kind, folder):
        subprocess.run([sys.executable, ROOT / "experiments/t017" / ("measure.py" if kind == "debug" else "optimized.py"), folder], check=True)
