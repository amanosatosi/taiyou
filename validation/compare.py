"""T015 external-corpus adapter. Run only on CI; never regenerates references."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile

from inventory import InventoryError, canonical, digest, load, safe_path
from policy import results, snapshots
import images

ROOT = Path(__file__).resolve().parent.parent
EXE = ".exe" if os.name == "nt" else ""


def timestamp(case, frame):
    prefix = case["source"][:-4] + "-"
    expected = frame["expected"]
    if not expected.startswith(prefix) or not re.fullmatch(r"[0-9]+\.png", expected[len(prefix):]):
        raise InventoryError("invalid reference timestamp/name")
    value = int(expected[len(prefix):-4])
    if value != frame["timestamp_ms"] or not 0 <= value <= (2 ** 63 - 1) // 1000:
        raise InventoryError("invalid reference timestamp")
    return value * 1000


def select(inventory, suite, manifest, cases):
    indexed = {c["id"]: c for c in inventory["cases"]}
    if len(indexed) != len(inventory["cases"]):
        raise InventoryError("duplicate inventory case")
    if suite == "full":
        if manifest or cases:
            raise InventoryError("full run cannot hide cases with selectors")
        return list(indexed.values())
    ids = load(manifest)["cases"] if manifest else cases
    if not ids or len(ids) != len(set(ids)) or any(i not in indexed for i in ids):
        raise InventoryError("empty, duplicate or unselected focused case")
    return [indexed[i] for i in ids]


def restricted():
    # Linux runner bounds, not a hostile-code sandbox or a hard RSS guarantee.
    import resource
    resource.setrlimit(resource.RLIMIT_CPU, (30, 30))
    resource.setrlimit(resource.RLIMIT_AS, (2 * 1024 ** 3, 2 * 1024 ** 3))
    resource.setrlimit(resource.RLIMIT_FSIZE, (128 * 1024 ** 2, 128 * 1024 ** 2))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def execute(command, directory, timeout=45):
    # No GitHub token, repository secret, ART_* overrides, or user config.
    environment = {k: v for k, v in os.environ.items() if k in ("PATH", "SystemRoot", "WINDIR", "LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH")}
    environment.update(HOME=str(directory), TMPDIR=str(directory), TEMP=str(directory), TMP=str(directory), LC_ALL="C.UTF-8")
    log = directory / "process.log"
    try:
        with log.open("wb") as output:
            process = subprocess.run([str(v) for v in command], cwd=directory, env=environment,
                                     stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT,
                                     timeout=timeout, preexec_fn=restricted if sys.platform == "linux" else None)
        with log.open("rb") as output:
            diagnostic = output.read(65536).decode("utf-8", errors="replace")
        if log.stat().st_size > 65536:
            diagnostic += "\n[diagnostic truncated at 64 KiB]"
        return process.returncode, diagnostic
    except subprocess.TimeoutExpired:
        return -999, "renderer timeout"


def classify(case, outcome, metrics, diagnostic):
    support = case["profile_records"]["CoreV1"]["support"]
    if outcome < 0:
        return "CRASH"
    if outcome != 0:
        if outcome == 20:
            return "UNSUPPORTED" if support == "UNSUPPORTED" else "REGRESSION"
        # A malformed source that the renderer safely rejects is still reported;
        # it is never a compatibility pass. Accepted supported sources must work.
        if outcome in (11, 12):
            return "UNSUPPORTED" if support == "UNSUPPORTED" else "REGRESSION"
        return "INFRA_ERROR"
    if metrics and metrics["exact"]:
        return "PASS_EXACT"
    return "UNSUPPORTED" if support == "UNSUPPORTED" else "REGRESSION"


def run_case(case, corpus, binary, probe, out, assets, artifacts):
    record = {"case": case["id"], "frame": case["frames"][0]["expected"] if case["frames"] else None,
              "profile": "CoreV1", "kind": case["kind"], "status": "INFRA_ERROR", "feature": "T015/unpromoted-external-case",
              "accepted_support": case.get("profile_records", {}).get("CoreV1", {}).get("support"),
              "accepted_reason": case.get("profile_records", {}).get("CoreV1", {}).get("reason"),
              "source": assets[case["source"]], "fonts": [assets[f] for f in case["fonts"]],
              "scale": case["scale"], "metrics": None, "stage": "asset-verification", "diagnostics": ""}
    if case["kind"] == "crash":
        record["safety_only"] = True
    try:
        source = safe_path(corpus, case["source"])
        fonts = [safe_path(corpus, f) for f in case["fonts"]]
        with tempfile.TemporaryDirectory(prefix="taiyou-compare-") as directory:
            directory = Path(directory)
            if case["kind"] == "crash":
                code, text = execute([probe, "compare-crash-probe", source], directory)
                record.update(status="CRASH" if code < 0 else "PASS_EXACT" if code == 0 else "INFRA_ERROR",
                              diagnostics=text, stage="parser-safety-probe", safety_only=True)
                return record
            frame = case["frames"][0]
            time_us = timestamp(case, frame)
            expected = images.read_png(safe_path(corpus, frame["expected"]))
            record["expected"] = assets[frame["expected"]]
            if list(expected[:2]) != frame["reference_size"]:
                raise ValueError("reference dimensions differ from inventory")
            if not fonts:
                raise ValueError("regression case has no explicit fonts")
            if case["scale"] != "1":
                record.update(status="UNSUPPORTED", feature="T015/oversampling-adapter", stage="scale",
                              diagnostics="CoreV1 has no independent supersampling viewport mapping; no silent resizing/downsampling")
                return record
            png, raw = directory / "actual.png", directory / "actual.rgba"
            command = [binary, "render", "--input", source, "--time-us", str(time_us), "--width", str(expected[0]),
                       "--height", str(expected[1]), "--profile", "core-v1", "--output", png, "--raw", raw]
            for font in fonts:
                command.extend(["--font", font])
            code, diagnostic = execute(command, directory)
            strict_unsupported = code == 20
            # Tolerant exploratory output cannot promote the accepted support state.
            if strict_unsupported:
                record["strict_diagnostics"] = diagnostic
                code, diagnostic = execute(command + ["--unsupported", "tolerant"], directory)
            metrics = None
            if code == 0:
                actual = images.raw_rgba(raw, expected[0], expected[1])
                # Validate the published PNG independently, without using it as
                # authoritative raw output or requiring a lossy premul roundtrip.
                decoded_png = images.read_png(png)
                if decoded_png[:2] != expected[:2]:
                    raise ValueError("renderer PNG dimensions mismatch")
                key = digest(case["id"].encode())[:16]
                artifact = out / key
                save = artifacts < 12
                if save:
                    artifact.mkdir()
                metrics = images.compare(expected, actual, artifact / "difference.png" if save else None)
                record["raw_sha256"] = digest(raw.read_bytes())
                record["png_sha256"] = digest(png.read_bytes())
                if save:
                    (artifact / "actual.png").write_bytes(png.read_bytes())
                    record["artifacts"] = {"actual": key + "/actual.png", "difference": key + "/difference.png",
                                           "expected": "external:" + frame["expected"]}
            status = classify(case, code, metrics, diagnostic)
            if strict_unsupported and status == "PASS_EXACT":
                status = "UNSUPPORTED" if case["profile_records"]["CoreV1"]["support"] == "UNSUPPORTED" else "REGRESSION"
            record.update(status=status, metrics=metrics, diagnostics=diagnostic, stage="pixel-comparison" if metrics else "render",
                          tolerant_attempt=strict_unsupported, exit_code=code)
    except images.ImageDecodeError as error:
        record.update(status="INFRA_ERROR", diagnostic_code="InvalidImage", diagnostics=str(error))
    except (OSError, ValueError, KeyError, InventoryError) as error:
        record.update(status="INFRA_ERROR", diagnostics=str(error))
    return record


def summarize(inventory, selected, policy, records):
    # Preserve the accepted T005 status/denominator contract. These additive
    # subtotals prevent aggregate PASS_EXACT from implying visual matches.
    summary = results({**inventory, "cases": selected}, {**policy, "profiles": ["CoreV1"]}, records)
    summary.update(inventory_total=len(inventory["cases"]), selected_including_disabled=len(selected),
                   unselected=[c["id"] for c in inventory["cases"] if c not in selected],
                   carried_results=[], disabled=[c["id"] for c in selected if c["kind"] == "disabled-upstream"],
                   counts_scope="all-active-cases-including-parser-safety")
    groups = {"visual": [], "safety": []}
    kinds = {c["id"]: c["kind"] for c in selected}
    for record in records:
        kind = kinds[record["case"]]
        if record["kind"] != kind:
            raise InventoryError("result kind differs from accepted inventory")
        if kind == "crash":
            if record.get("safety_only") is not True or record["metrics"] is not None or (
                    record["status"] == "PASS_EXACT" and record["stage"] != "parser-safety-probe"):
                raise InventoryError("invalid parser-only safety result")
            groups["safety"].append(record)
        elif kind == "regression":
            if record.get("safety_only") or (record["status"] == "PASS_EXACT" and not (record["metrics"] or {}).get("exact")):
                raise InventoryError("visual exact match requires pixel evidence")
            groups["visual"].append(record)
        else:
            raise InventoryError("unexpected result kind")
    for name, grouped in groups.items():
        counts = {status: sum(r["status"] == status for r in grouped) for status in policy["allowed_results"]}
        summary[name] = {"total": len(grouped), "counts": counts}
    summary["visual"]["exact_matches"] = summary["visual"]["counts"]["PASS_EXACT"]
    summary["safety"]["successful_parser_probes"] = summary["safety"]["counts"]["PASS_EXACT"]
    return summary


def accepted_snapshot():
    accepted = Path(os.environ.get("TAIYOU_ACCEPTED", ROOT)).resolve()
    inventory = load(accepted / "validation/inventory.json")
    policy = load(accepted / "validation/policy.json")
    # Baseline identity is recorded; the Actions workflow materializes it from
    # the PR base. No assertion that candidate CI is independently trusted.
    baseline = os.environ.get("ACCEPTED_BASELINE")
    if baseline:
        if not re.fullmatch(r"[0-9a-f]{40}", baseline):
            raise InventoryError("invalid accepted baseline SHA")
        original_inventory = json.loads(subprocess.check_output(["git", "show", baseline + ":validation/inventory.json"], cwd=ROOT))
        original_policy = json.loads(subprocess.check_output(["git", "show", baseline + ":validation/policy.json"], cwd=ROOT))
    else:
        original_inventory, original_policy = inventory, policy
    snapshots(inventory, policy, digest(canonical(original_inventory)), digest(canonical(original_policy)), os.environ)
    return accepted, inventory, policy


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--suite", choices=["full", "focused"], required=True)
    parser.add_argument("--profile", choices=["core-v1"], required=True)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--case", action="append", default=[])
    parser.add_argument("--corpus", type=Path, default=os.environ.get("TAIYOU_CORPUS"))
    parser.add_argument("--output", type=Path, default=Path(os.environ.get("RUNNER_TEMP", ".")) / "taiyou-comparison")
    parser.add_argument("--renderer", type=Path, default=ROOT / "target/debug" / ("taiyou-compare" + EXE))
    parser.add_argument("--probe", type=Path, default=ROOT / "target/debug" / ("xtask" + EXE))
    args = parser.parse_args()
    if args.manifest and args.case:
        parser.error("choose manifest or --case, not both")
    accepted, inventory, policy = accepted_snapshot()
    if not args.corpus:
        parser.error("set TAIYOU_CORPUS or --corpus to the acquired external corpus")
    corpus = args.corpus.resolve()
    if not args.renderer.is_file() or not args.probe.is_file():
        parser.error("build native taiyou-cli and xtask on the runner first")
    selected = select(inventory, args.suite, args.manifest, args.case)
    args.output.mkdir(parents=True, exist_ok=False)
    records = []
    try:
        subprocess.run([sys.executable, str(accepted / "validation/inventory.py"), "verify", "--corpus", str(corpus),
                        "--manifest", str(accepted / "validation/inventory.json")], check=True)
        assets = {a["path"]: a for a in inventory["assets"]}
        for case in selected:
            if case["kind"] == "disabled-upstream":
                continue
            record = run_case(case, corpus, args.renderer.resolve(), args.probe.resolve(), args.output, assets,
                              sum("artifacts" in r for r in records))
            records.append(record)
            print(json.dumps({"case": record["case"], "status": record["status"], "stage": record["stage"]}), flush=True)
        summary = summarize(inventory, selected, policy, records)
        report = {"schema": 1, "candidate_sha": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                  "accepted_baseline": os.environ.get("ACCEPTED_BASELINE", "candidate-local-unaccepted"),
                  "inventory_sha256": digest(canonical(inventory)), "policy_sha256": digest(canonical(policy)),
                  "corpus_revision": inventory["commit"], "profile": "CoreV1", "suite": args.suite,
                  "independently_trusted": False, "summary": summary, "records": records,
                  "oracle": "separate native-oracle.json; not inferred from Taiyou results"}
        (args.output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
        print("COMPARISON_SUMMARY=" + json.dumps(summary), flush=True)
        return int(any(r["status"] in ("REGRESSION", "CRASH", "INFRA_ERROR") for r in records))
    finally:
        subprocess.run([sys.executable, str(accepted / "validation/inventory.py"), "verify", "--corpus", str(corpus),
                        "--manifest", str(accepted / "validation/inventory.json")], check=True)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (InventoryError, OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print("INFRA_ERROR: " + str(error), file=sys.stderr)
        sys.exit(1)
