"""Pinned upstream native compare, separate from Taiyou's byte metrics.

No -o, ART_REG_GEN, asset modification, or upstream target generation. The
original harness classifies SAME/GOOD/BAD/FAIL; these are not Taiyou passes.
"""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys

from inventory import load

ROOT = Path(__file__).resolve().parent.parent


def harness_summary(text, expected_count):
    match = re.search(r"(?:All (\d+)|Only (\d+) of (\d+)) images have passed test", text)
    if not match:
        raise ValueError("native harness missing complete frame summary")
    passed = int(match[1] or match[2])
    total = int(match[1] or match[3])
    if total != expected_count or passed > total:
        raise ValueError("native harness denominator differs from accepted inventory")
    return {"passed": passed, "total": total}


def run(command, **kwargs):
    print("ORACLE_COMMAND=" + json.dumps([str(v) for v in command]), flush=True)
    return subprocess.run([str(v) for v in command], check=True, **kwargs)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if any(k.startswith("ART_REG_") for k in os.environ):
        raise ValueError("forbidden ART_REG override")
    accepted = Path(os.environ["TAIYOU_ACCEPTED"])
    corpus = Path(os.environ["TAIYOU_CORPUS"])
    inventory = load(accepted / "validation/inventory.json")
    lock = load(accepted / "docs/references.lock.json")
    pin = next(p for p in lock["references"] if p["id"] == "libass")
    args.output.mkdir(parents=True, exist_ok=True)
    report = {"schema": 1, "oracle_commit": pin["commit"], "corpus_commit": inventory["commit"],
              "candidate_sha": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
              "certification": "ENVIRONMENT_PENDING", "results": [], "environment": {}, "taiyou_results": "separate"}
    verifier = [sys.executable, accepted / "validation/inventory.py", "verify", "--corpus", corpus,
                "--manifest", accepted / "validation/inventory.json"]
    try:
        run(verifier)
        directory = Path(os.environ["RUNNER_TEMP"]) / "taiyou-pinned-libass"
        if not directory.exists():
            directory.mkdir()
            run(["git", "init", directory])
            run(["git", "-C", directory, "fetch", "--depth=1", pin["repository"], pin["commit"]])
            run(["git", "-C", directory, "checkout", "--detach", "FETCH_HEAD"])
        actual = subprocess.check_output(["git", "-C", directory, "rev-parse", "HEAD"], text=True).strip()
        if actual != pin["commit"]:
            raise ValueError("native oracle pin mismatch")
        run(["git", "-C", directory, "fsck", "--full"])
        # Record exact distro dependency versions; no unpinned system libass.
        report["environment"]["packages"] = subprocess.check_output(
            ["dpkg-query", "-W", "libfreetype-dev", "libharfbuzz-dev", "libfribidi-dev", "libpng-dev", "nasm"], text=True)
        report["environment"]["compiler"] = subprocess.check_output(["cc", "--version"], text=True)
        flags = ["--enable-compare", "--disable-fontconfig", "--disable-shared", "--disable-require-system-font-provider"]
        report["environment"]["configure"] = flags
        run(["sh", "autogen.sh"], cwd=directory)
        run(["./configure", *flags], cwd=directory)
        run(["make", "-j2"], cwd=directory)
        binary = directory / "compare/compare"
        directories = sorted({str(Path(c["source"]).parent) for c in inventory["cases"] if c["kind"] == "regression"})
        for index, name in enumerate(directories):
            scale = next(c["scale"] for c in inventory["cases"] if str(Path(c["source"]).parent) == name)
            # Exact default zero policy. Upstream's original run-single.sh uses
            # the same call with -p2 by default; record both separately.
            for level in (0, 2):
                command = [str(binary), str(corpus / name), "-s", scale, "-p", str(level)]
                completed = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120, text=True)
                log = f"directory-{index}-p{level}.txt"
                (args.output / log).write_text(completed.stdout)
                expected_count = sum(len(c["frames"]) for c in inventory["cases"] if c["kind"] == "regression" and str(Path(c["source"]).parent) == name)
                summary = harness_summary(completed.stdout, expected_count)
                report["results"].append({"directory": name, "scale": scale, "pass_level": level,
                                          "exit_code": completed.returncode, "log": log, **summary})
        # All original scripts are retained in provenance, not executed with
        # unsafe ART_REG_* knobs. No failure is suppressed or renamed a pass.
        report["certification"] = "REPRODUCED" if all(r["exit_code"] == 0 for r in report["results"] if r["pass_level"] == 2) else "REFERENCE_DIFFERENCES"
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        report.update(certification="ENVIRONMENT_FAILURE", error=str(error))
    finally:
        try:
            run(verifier)
        except subprocess.SubprocessError as error:
            report.update(certification="ENVIRONMENT_FAILURE", error="post-run inventory verification failed: " + str(error))
        (args.output / "native-oracle.json").write_text(json.dumps(report, indent=2) + "\n")
        print("NATIVE_ORACLE=" + json.dumps(report), flush=True)
    return int(report["certification"] == "ENVIRONMENT_FAILURE" or any(r["exit_code"] < 0 or r["exit_code"] > 3 for r in report["results"]))


if __name__ == "__main__":
    sys.exit(main())
