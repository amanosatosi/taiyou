"""Supplemental strict direct-xy smoke: one near-match family and exact guards.

This never changes the accepted inventory, captures or full comparison policy.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess

import oracle
import parallel


CASES = {
    "regression/faux-italics/truetype-0000.png",
    "regression/faux-italics/cff-0000.png",
    "regression/embedded-font/efont-1000.png",
    "regression/implicit-pos/shift-dir-9500.png",
    "regression/scaling_playres/playresx_sbas-yes-4000.png",
}


def main():
    assert os.environ.get("GITHUB_ACTIONS") == "true" and os.name == "nt"
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=True)
    corpus = Path(os.environ["TAIYOU_CORPUS"])
    release = Path(os.environ["RUNNER_TEMP"]) / "xy-vsfilter-release"
    binary = oracle.ROOT / "target/debug/taiyou.exe"
    dll, probe, attempts = oracle.find_csri_dll(release, out)
    visual, nonvisual = oracle.inventory_cases("a5073319af8117271d8ac13ef5c088197ff2d59b")
    selected = [case for case in visual if case["id"] in CASES]
    assert len(selected) == len(CASES) and {case["id"] for case in selected} == CASES
    report = {"candidate_sha": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
              "purpose": "supplemental strict xy smoke, never a replacement for the full run",
              "dll": probe, "dll_probe_attempts": attempts,
              "selected": sorted(CASES), "unselected": [c["id"] for c in visual if c["id"] not in CASES],
              "nonvisual": nonvisual, "cases": []}
    failed = False
    for case in selected:
        assert case["inventory"] is True  # oracle.render_taiyou enforces explicit mode
        xy = parallel.xy_case(case, corpus, dll, release, out / "xy")
        taiyou = parallel.taiyou_case(case, corpus, binary, out / "taiyou")
        row = {"id": case["id"], "xy_status": xy["status"], "taiyou_status": taiyou["status"],
               "libass_reference_comparison": xy.get("libass_reference_comparison"),
               "xy_error": xy.get("error"), "taiyou_error": taiyou.get("error")}
        if xy["status"] == taiyou["status"] == "OK":
            key = oracle.artifact_key(case["id"])
            xr, tr = out / "xy/cases" / key / "xy.rgba", out / "taiyou/cases" / key / "taiyou.rgba"
            row["metrics"] = parallel.compare_raw_bytes(xr, tr, case["width"], case["height"], out / (key + "-diff.png"))
            row["xy_raw_sha256"], row["taiyou_raw_sha256"] = oracle.sha256_file(xr), oracle.sha256_file(tr)
            failed |= not row["metrics"]["exact"]
            xr.unlink()
            tr.unlink()
        else:
            failed = True
        report["cases"].append(row)
        print("NEAR_MATCH=" + json.dumps(row), flush=True)
    (out / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
