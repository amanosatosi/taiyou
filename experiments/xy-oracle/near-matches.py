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
    "regression/draw-images/drawImg-104396.png",
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
    # Read the frozen CFF program rather than infer its design coordinates from
    # raster edges. This is diagnostic source evidence, not a generated oracle.
    from fontTools.ttLib import TTFont
    from fontTools.pens.recordingPen import RecordingPen
    cff = next(case for case in selected if case["id"] == "regression/faux-italics/cff-0000.png")
    report["cff_outline_evidence"] = []
    for spec in cff["fonts"]:
        path = corpus / spec["path"]
        with TTFont(path, recalcTimestamp=False) as font:
            if "CFF " not in font:
                continue
            glyph = font.getBestCmap()[ord("A")]
            pen = RecordingPen()
            font.getGlyphSet()[glyph].draw(pen)
            report["cff_outline_evidence"].append({"path": spec["path"], "sha256": oracle.sha256_file(path),
                "upem": font["head"].unitsPerEm, "glyph": glyph, "commands": pen.value,
                "advance": font["hmtx"].metrics[glyph][0],
                "win_cell": [font["OS/2"].usWinAscent, font["OS/2"].usWinDescent],
                "font_matrix": font["CFF "].cff.topDictIndex[0].FontMatrix})
    paint_source = out / "packed-paint.ass"
    paint_source.write_text(
        "[Script Info]\nPlayResX: 64\nPlayResY: 32\nWrapStyle: 2\n"
        "[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,SecondaryColour,OutlineColour,BackColour,Bold,Italic,Underline,StrikeOut,ScaleX,ScaleY,Spacing,Angle,BorderStyle,Outline,Shadow,Alignment,MarginL,MarginR,MarginV,Encoding\n"
        "Style: Default,FansubBlock-CFF,20,&H00FFFFFF,&H00FFFFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,0,0,7,0,0,0,1\n"
        "[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\n"
        r"Dialogue: 0,0:00:00.00,0:00:02.00,Default,,0,0,0,,{\pos(0,0)\p1\1c&H11113E&\1a&H9D&}m 0 0 l 24 0 24 16 0 16" "\n"
        r"Dialogue: 1,0:00:00.00,0:00:02.00,Default,,0,0,0,,{\pos(32,0)\p1\1c&HFFFFFF&\1a&H7F&}m 0 0 l 16 0 16 16 0 16" "\n"
        r"Dialogue: 2,0:00:00.00,0:00:02.00,Default,,0,0,0,,{\pos(40,0)\p1\1c&HFFFFFF&\1a&HFE&}m 0 0 l 16 0 16 16 0 16" "\n",
        encoding="utf-8")
    # The existing adapter's inventory flag selects explicit font admission.
    # This authored control is separately identified, never an accepted case.
    paint_case = {"id": "witness/packed-paint", "source": "authored/packed-paint.ass", "source_external": str(paint_source),
                  "reference": None, "timestamp_ms": 1000, "width": 64, "height": 32,
                  "fonts": cff["fonts"], "inventory": True, "scale": "1"}
    report["supplemental_authored_cases"] = [paint_case["id"]]
    failed = False
    report_path = out / "report.json"
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    for case in selected + [paint_case]:
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
        report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print("NEAR_MATCH=" + json.dumps(row), flush=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return int(failed)


if __name__ == "__main__":
    raise SystemExit(main())
