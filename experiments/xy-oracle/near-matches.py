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
    "regression/implicit-pos/bordercollision-2000.png",
    *{f"regression/implicit-pos/shift-dir-{time:04d}.png" for time in range(500, 9500, 1000)},
    "regression/scaling_playres/playresx_sbas-yes-3000.png",
    "regression/scaling_playres/playresx_sbas-yes-4000.png",
    "regression/blurs/be-0500.png",
    "regression/blurs/be-3500.png",
    "regression/blurs/blur+t-1000.png",
    "regression/blurs/blur+t-1900.png",
    "regression/blurs-anamorphic/blur-0000.png",
    "regression/ssa/alpha-2000.png",
    *{f"regression/ssa/alpha-{time:04d}.png" for time in range(0, 5000, 1000)},
    *{f"regression/vsf-quirks-var/alpha-clipping-{time:04d}.png" for time in (2040, 2560, 4960)},
    "regression/ssa/effect-delay-1600.png",
    *{f"regression/ssa/effect-delay-{time:04d}.png" for time in (600, 1600, 2600, 3960, 4960, 5960)},
    "regression/v4++/kt-01000.png",
    "regression/v4++/kt-04800.png",
    "regression/v4++/kt-06700.png",
    "regression/v4++/kt-11000.png",
    "regression/karaoke/karaoke-and-runsplits-2800.png",
    "regression/karaoke/357-k-and-kf-desynced-7170.png",
}

FAMILIES = ("regression/v4++/", "regression/karaoke/")


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
    selected = [case for case in visual if case["id"] in CASES or case["id"].startswith(FAMILIES)]
    selected_ids = {case["id"] for case in selected}
    assert CASES <= selected_ids
    report = {"candidate_sha": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
              "purpose": "supplemental strict xy smoke, never a replacement for the full run",
              "dll": probe, "dll_probe_attempts": attempts,
              "selected": sorted(selected_ids), "unselected": [c["id"] for c in visual if c["id"] not in selected_ids],
              "nonvisual": nonvisual, "cases": []}
    # Read the frozen CFF program rather than infer its design coordinates from
    # raster edges. This is diagnostic source evidence, not a generated oracle.
    from fontTools.ttLib import TTFont
    from fontTools.pens.recordingPen import RecordingPen
    cff = next(case for case in selected if case["id"] == "regression/faux-italics/cff-0000.png")
    ssa = next(case for case in selected if case["id"] == "regression/ssa/effect-delay-1600.png")
    diagnostic = subprocess.run(
        [str(oracle.ROOT / "target/debug/examples/mua_metrics.exe"),
         str(corpus / ssa["source"]), str(corpus / ssa["fonts"][0]["path"]), "--explicit-layout"],
        capture_output=True, text=True, encoding="utf-8")
    (out / "ssa-layout.txt").write_text(diagnostic.stdout + diagnostic.stderr, encoding="utf-8")
    assert diagnostic.returncode == 0
    motion_source = out / "drawing-motion.ass"
    motion_lines = []
    for line in (corpus / ssa["source"]).read_text(encoding="utf-8-sig").splitlines():
        if line.startswith("Dialogue:"):
            fields = line.split(",", 9)
            if fields[3] != "Probe":
                continue
            fields[9] = r"{\p1}m 0 0 l 2 0 2 2 0 2"
            line = ",".join(fields)
        motion_lines.append(line)
    motion_source.write_text("\n".join(motion_lines) + "\n", encoding="utf-8")
    motion_cases = [{**ssa, "id": f"witness/drawing-motion-{time}",
                     "source_external": str(motion_source), "reference": None,
                     "timestamp_ms": time} for time in (600, 1600, 2600, 3960, 4960, 5960)]
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
    blur_source = out / "drawing-blurs.ass"
    blur_lines = []
    for row, tag in enumerate([r"\blur0", r"\blur0.5", r"\blur1", r"\blur2", r"\blur8", r"\blur16", r"\be1", r"\be4", r"\be0.5"]):
        # Native paths with both straight and cubic edges isolate filtering
        # from installed fonts, shaping, and native outline conversion.
        for col, drawing in enumerate(["m 0 0 l 23 0 23 13 0 13", "m 0 0 b 32 -8 32 24 0 16 b -8 16 -8 0 0 0"]):
            blur_lines.append(f"Dialogue: 0,0:00:00.00,0:00:02.00,Default,,0,0,0,,{{\\pos({50+col*100},{35+row*50})\\p1{tag}}}{drawing}\n")
    blur_source.write_text(
        paint_source.read_text(encoding="utf-8").split("[Events]")[0].replace("PlayResX: 64", "PlayResX: 256").replace("PlayResY: 32", "PlayResY: 512")
        + "[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\n" + "".join(blur_lines), encoding="utf-8")
    blur_case = {"id": "witness/drawing-blurs", "source": "authored/drawing-blurs.ass", "source_external": str(blur_source),
                 "reference": None, "timestamp_ms": 1000, "width": 256, "height": 512,
                 "fonts": cff["fonts"], "inventory": True, "scale": "1"}
    border_source = out / "drawing-borders.ass"
    border_lines = []
    for row, (rx, ry) in enumerate([(0.125, 0.125), (0.25, 0.25), (0.5, 0.5), (1, 1), (2, 2), (3, 3), (1, 3), (3, 1)]):
        for col, drawing in enumerate(["m 0 0 l 23 0 23 13 0 13", "m 0 0 b 32 -8 32 24 0 16 b -8 16 -8 0 0 0"]):
            border_lines.append(f"Dialogue: 0,0:00:00.00,0:00:02.00,Default,,0,0,0,,{{\\pos({50+col*100},{35+row*50})\\p1\\1a&HFF&\\3c&HFFFFFF&\\xbord{rx}\\ybord{ry}}}{drawing}\n")
    border_source.write_text(
        blur_source.read_text(encoding="utf-8").split("[Events]")[0]
        + "[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\n" + "".join(border_lines), encoding="utf-8")
    border_case = {"id": "witness/drawing-borders", "source": "authored/drawing-borders.ass", "source_external": str(border_source),
                   "reference": None, "timestamp_ms": 1000, "width": 256, "height": 512,
                   "fonts": cff["fonts"], "inventory": True, "scale": "1"}
    kernel_source = out / "ellipse-phases.ass"
    kernel_lines = []
    for row, (rx, ry) in enumerate([(0.125, 0.125), (0.25, 0.25), (0.5, 0.5), (1, 1), (2, 2), (3, 3), (1, 3), (3, 1)]):
        # Eight translations of a single eighth-grid cell expose the discrete
        # ellipse's row sums without font contours or a downsampled-kernel guess.
        for phase in range(8):
            kernel_lines.append(f"Dialogue: 0,0:00:00.00,0:00:02.00,Default,,0,0,0,,{{\\pos({15+30*phase},{25+60*row+phase/8})\\p1\\1a&HFF&\\3c&HFFFFFF&\\xbord{rx}\\ybord{ry}}}m 0 0 l 0.125 0 0.125 0.125 0 0.125\n")
    kernel_source.write_text(
        blur_source.read_text(encoding="utf-8").split("[Events]")[0]
        + "[Events]\nFormat: Layer,Start,End,Style,Name,MarginL,MarginR,MarginV,Effect,Text\n" + "".join(kernel_lines), encoding="utf-8")
    kernel_case = {"id": "witness/ellipse-phases", "source": "authored/ellipse-phases.ass", "source_external": str(kernel_source),
                   "reference": None, "timestamp_ms": 1000, "width": 256, "height": 512,
                   "fonts": cff["fonts"], "inventory": True, "scale": "1"}
    report["supplemental_authored_cases"] = [paint_case["id"], blur_case["id"], border_case["id"], kernel_case["id"],
                                               *[case["id"] for case in motion_cases]]
    failed = False
    report_path = out / "report.json"
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    for case in selected + [paint_case, blur_case, border_case, kernel_case] + motion_cases:
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
