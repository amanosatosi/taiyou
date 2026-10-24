"""Parallel xy-VSFilter/Taiyou render stages plus final comparison."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import sys

import oracle

ROOT = Path(__file__).resolve().parents[2]



def compare_raw_bytes(expected_path: Path, actual_path: Path, width: int, height: int, diff_path: Path) -> dict:
    expected = expected_path.read_bytes()
    actual = actual_path.read_bytes()
    size = width * height * 4
    if len(expected) != size or len(actual) != size:
        raise ValueError("raw comparison buffer length mismatch")

    sums = [0, 0, 0, 0]
    maximum = 0
    changed = 0
    alpha_mismatches = 0
    bounds = None
    diff = bytearray(size)

    for i in range(0, size, 4):
        er, eg, eb, ea = expected[i:i + 4]
        ar, ag, ab, aa = actual[i:i + 4]
        if er > ea or eg > ea or eb > ea:
            raise ValueError("xy raw frame is not premultiplied")
        if ar > aa or ag > aa or ab > aa:
            raise ValueError("Taiyou raw frame is not premultiplied")

        dr = abs(er - ar)
        dg = abs(eg - ag)
        db = abs(eb - ab)
        da = abs(ea - aa)
        sums[0] += dr
        sums[1] += dg
        sums[2] += db
        sums[3] += da
        local_max = max(dr, dg, db, da)
        if local_max > maximum:
            maximum = local_max
        if local_max:
            changed += 1
            pixel = i // 4
            x = pixel % width
            y = pixel // width
            if bounds is None:
                bounds = [x, y, x + 1, y + 1]
            else:
                if x < bounds[0]:
                    bounds[0] = x
                if y < bounds[1]:
                    bounds[1] = y
                if x + 1 > bounds[2]:
                    bounds[2] = x + 1
                if y + 1 > bounds[3]:
                    bounds[3] = y + 1
        if da:
            alpha_mismatches += 1
        diff[i:i + 4] = bytes((max(dr, da), dg, db, 255))

    oracle.images.write_png(diff_path, width, height, diff)
    pixels = width * height
    return {
        "expected_dimensions": [width, height],
        "actual_dimensions": [width, height],
        "dimensions_match": True,
        "max_channel_delta": maximum * 257,
        "channel_mae": [value * 257 / pixels for value in sums],
        "changed_pixels": changed,
        "changed_bbox": bounds,
        "alpha_mismatches": alpha_mismatches,
        "exact": changed == 0,
        "units": "premultiplied-encoded-RGBA16",
    }


def compare_reference_fast(reference: Path, raw_path: Path, width: int, height: int, diff_path: Path) -> dict:
    expected = oracle.images.read_png(reference)
    if expected[:2] != (width, height):
        raise ValueError(
            f"reference dimensions {expected[:2]} do not match manifest {(width, height)}"
        )
    raw = raw_path.read_bytes()
    size = width * height * 4
    if len(raw) != size:
        raise ValueError("xy raw frame length mismatch")

    ep = expected[2]
    sums = [0, 0, 0, 0]
    maximum = 0
    changed = 0
    alpha_mismatches = 0
    bounds = None
    diff = bytearray(size)

    for i in range(0, size, 4):
        ar, ag, ab, aa = raw[i:i + 4]
        if ar > aa or ag > aa or ab > aa:
            raise ValueError("xy raw frame is not premultiplied")
        actual16 = (ar * 257, ag * 257, ab * 257, aa * 257)
        d0 = abs(ep[i] - actual16[0])
        d1 = abs(ep[i + 1] - actual16[1])
        d2 = abs(ep[i + 2] - actual16[2])
        d3 = abs(ep[i + 3] - actual16[3])
        sums[0] += d0
        sums[1] += d1
        sums[2] += d2
        sums[3] += d3
        local_max = max(d0, d1, d2, d3)
        if local_max > maximum:
            maximum = local_max
        if local_max:
            changed += 1
            pixel = i // 4
            x = pixel % width
            y = pixel // width
            if bounds is None:
                bounds = [x, y, x + 1, y + 1]
            else:
                if x < bounds[0]:
                    bounds[0] = x
                if y < bounds[1]:
                    bounds[1] = y
                if x + 1 > bounds[2]:
                    bounds[2] = x + 1
                if y + 1 > bounds[3]:
                    bounds[3] = y + 1
        if d3:
            alpha_mismatches += 1
        diff[i:i + 4] = bytes((max(d0, d3) // 257, d1 // 257, d2 // 257, 255))

    oracle.images.write_png(diff_path, width, height, diff)
    pixels = width * height
    return {
        "expected_dimensions": [width, height],
        "actual_dimensions": [width, height],
        "dimensions_match": True,
        "max_channel_delta": maximum,
        "channel_mae": [value / pixels for value in sums],
        "changed_pixels": changed,
        "changed_bbox": bounds,
        "alpha_mismatches": alpha_mismatches,
        "exact": changed == 0,
        "units": "premultiplied-encoded-RGBA16",
    }


def base_record(case: dict) -> dict:
    return {
        "id": case["id"],
        "source": case["source"],
        "reference": case.get("reference"),
        "timestamp_ms": case["timestamp_ms"],
        "dimensions": [case["width"], case["height"]],
        "scale_metadata": case.get("scale", "1"),
        "status": "ERROR",
    }


def cases():
    corpus_commit = "a5073319af8117271d8ac13ef5c088197ff2d59b"
    visual, nonvisual = oracle.inventory_cases(corpus_commit)
    visual.append(oracle.mua_office_case())
    return corpus_commit, visual, nonvisual


def xy_case(case: dict, corpus: Path, dll: Path, release_root: Path, output: Path) -> dict:
    record = base_record(case)
    case_dir = output / "cases" / oracle.artifact_key(case["id"])
    case_dir.mkdir(parents=True, exist_ok=False)
    try:
        source, reference, fonts, font_records = oracle.validate_case_assets(case, corpus)
        record.update(
            source_sha256=oracle.sha256_file(source),
            reference_sha256=oracle.sha256_file(reference) if reference else None,
            fonts=font_records,
            embedded_fonts=bool(case.get("embedded_fonts")),
        )
        captures = [
            ("black", (0, 0, 0)),
            ("black-repeat", (0, 0, 0)),
            ("white", (255, 255, 255)),
            ("validation", oracle.VALIDATION_RGB),
        ]
        paths = {}
        logs = {}
        for name, rgb in captures:
            path = case_dir / f"{name}.bgrx"
            code, log = oracle.capture(dll, release_root, source, fonts, case, rgb, path)
            logs[name] = {"exit_code": code, "log": log}
            if code != 0:
                raise RuntimeError(f"{name} xy capture failed ({code}): {log[-2000:]}")
            if path.stat().st_size != case["width"] * case["height"] * 4:
                raise ValueError(f"{name} xy capture size mismatch")
            paths[name] = path

        black = paths["black"].read_bytes()
        repeat = paths["black-repeat"].read_bytes()
        deterministic = black == repeat
        record["determinism"] = {
            "exact": deterministic,
            "black_sha256": oracle.sha256_bytes(black),
            "black_repeat_sha256": oracle.sha256_bytes(repeat),
        }
        if not deterministic:
            raise ValueError("repeated fresh-process xy captures differ")

        raw_rgba, reconstruction = oracle.reconstruct_rgba(
            black,
            paths["white"].read_bytes(),
            paths["validation"].read_bytes(),
            oracle.VALIDATION_RGB,
        )
        record["reconstruction"] = reconstruction
        record["worker_logs"] = logs
        if raw_rgba is None:
            raise ValueError("xy alpha reconstruction failed validation")

        raw_path = case_dir / "xy.rgba"
        png_path = case_dir / "xy.png"
        raw_path.write_bytes(raw_rgba)
        oracle.preview_png(png_path, case["width"], case["height"], raw_rgba)
        record["xy_raw_sha256"] = oracle.sha256_file(raw_path)
        record["xy_png_sha256"] = oracle.sha256_file(png_path)

        if reference is not None:
            libass_diff = case_dir / "xy-vs-libass-diff.png"
            record["libass_reference_comparison"] = compare_reference_fast(
                reference, raw_path, case["width"], case["height"], libass_diff
            )

        for path in paths.values():
            path.unlink(missing_ok=True)
        record["status"] = "OK"
    except Exception as error:
        record["error"] = f"{type(error).__name__}: {error}"
    (case_dir / "xy.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    return record


def taiyou_case(case: dict, corpus: Path, binary: Path, output: Path) -> dict:
    record = base_record(case)
    case_dir = output / "cases" / oracle.artifact_key(case["id"])
    case_dir.mkdir(parents=True, exist_ok=False)
    try:
        source, _reference, fonts, font_records = oracle.validate_case_assets(case, corpus)
        record.update(
            source_sha256=oracle.sha256_file(source),
            fonts=font_records,
            embedded_fonts=bool(case.get("embedded_fonts")),
        )
        png = case_dir / "taiyou.png"
        raw = case_dir / "taiyou.rgba"
        code, log = oracle.render_taiyou(binary, source, fonts, case, png, raw)
        record["render"] = {"exit_code": code, "log": log}
        if code != 0:
            raise RuntimeError(f"Taiyou render failed ({code}): {log[-2000:]}")
        if raw.stat().st_size != case["width"] * case["height"] * 4:
            raise ValueError("Taiyou raw frame size mismatch")
        decoded = oracle.images.read_png(png)
        if decoded[:2] != (case["width"], case["height"]):
            raise ValueError("Taiyou PNG dimensions mismatch")
        record["taiyou_raw_sha256"] = oracle.sha256_file(raw)
        record["taiyou_png_sha256"] = oracle.sha256_file(png)
        record["status"] = "OK"
    except Exception as error:
        record["error"] = f"{type(error).__name__}: {error}"
    (case_dir / "taiyou.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    return record


def render_stage(mode: str, args) -> int:
    if os.name != "nt":
        raise RuntimeError("render stages require Windows")
    corpus = Path(args.corpus).resolve()
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)
    corpus_commit, visual, nonvisual = cases()

    report = {
        "schema": 1,
        "stage": mode,
        "candidate_sha": os.environ.get("GITHUB_SHA"),
        "corpus_commit": corpus_commit,
        "nonvisual_inventory_cases": nonvisual,
        "cases": [],
    }

    if mode == "xy":
        release_root = Path(args.release_root).resolve()
        dll, probe, attempts = oracle.find_csri_dll(release_root, output)
        report["dll"] = probe
        report["dll_probe_attempts"] = attempts
        render_one = lambda case: xy_case(case, corpus, dll, release_root, output)
    else:
        binary = Path(args.taiyou).resolve()
        if not binary.is_file():
            raise FileNotFoundError(f"Taiyou renderer not found: {binary}")
        render_one = lambda case: taiyou_case(case, corpus, binary, output)

    for case in visual:
        record = render_one(case)
        report["cases"].append(record)
        print(json.dumps({
            "stage": mode,
            "case": record["id"],
            "status": record["status"],
        }), flush=True)

    report["summary"] = {
        "total": len(visual),
        "ok": sum(r["status"] == "OK" for r in report["cases"]),
        "errors": sum(r["status"] != "OK" for r in report["cases"]),
    }
    (output / f"{mode}-report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"{mode.upper()}_RENDER_SUMMARY=" + json.dumps(report["summary"]), flush=True)
    return 0


def load_report(path: Path, name: str) -> dict:
    report = json.loads((path / f"{name}-report.json").read_text(encoding="utf-8-sig"))
    return report


def compare_stage(args) -> int:
    xy_root = Path(args.xy_input).resolve()
    taiyou_root = Path(args.taiyou_input).resolve()
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)

    xy_report = load_report(xy_root, "xy")
    taiyou_report = load_report(taiyou_root, "taiyou")
    xy_records = {r["id"]: r for r in xy_report["cases"]}
    taiyou_records = {r["id"]: r for r in taiyou_report["cases"]}
    ids = list(dict.fromkeys([*(r["id"] for r in xy_report["cases"]), *(r["id"] for r in taiyou_report["cases"])]))

    report = {
        "schema": 1,
        "purpose": "direct Taiyou comparison with actual xy-VSFilter 3.2.0.810 output",
        "classification_policy": {
            "same": "changed_pixels == 0",
            "good": f"0 < changed_pixels <= {oracle.GOOD_MAX_CHANGED_PIXELS}",
            "bad": "changed_pixels > good threshold",
            "note": "good is diagnostic only; only same is exact compatibility",
        },
        "cases": [],
        "nonvisual_inventory_cases": xy_report.get("nonvisual_inventory_cases", []),
    }

    for case_id in ids:
        xr = xy_records.get(case_id)
        tr = taiyou_records.get(case_id)
        record = {"id": case_id, "status": "ERROR"}
        if not xr or not tr:
            record["error"] = "missing render-stage record"
            report["cases"].append(record)
            continue
        record.update(
            dimensions=xr.get("dimensions"),
            xy_status=xr.get("status"),
            taiyou_status=tr.get("status"),
            libass_reference_comparison=xr.get("libass_reference_comparison"),
        )
        if xr.get("status") != "OK" or tr.get("status") != "OK":
            record["error"] = {
                "xy": xr.get("error"),
                "taiyou": tr.get("error"),
            }
            report["cases"].append(record)
            continue

        key = oracle.artifact_key(case_id)
        xdir = xy_root / "cases" / key
        tdir = taiyou_root / "cases" / key
        outdir = output / "cases" / key
        outdir.mkdir(parents=True, exist_ok=False)
        xy_raw, taiyou_raw = xdir / "xy.rgba", tdir / "taiyou.rgba"
        xy_png, taiyou_png = xdir / "xy.png", tdir / "taiyou.png"
        diff = outdir / "diff.png"
        width, height = xr["dimensions"]
        metrics = compare_raw_bytes(xy_raw, taiyou_raw, width, height, diff)
        classification, limit = oracle.classify_xy(metrics, width, height)

        shutil.copy2(xy_png, outdir / "xy.png")
        shutil.copy2(taiyou_png, outdir / "taiyou.png")
        libass_diff = xdir / "xy-vs-libass-diff.png"
        if libass_diff.is_file():
            shutil.copy2(libass_diff, outdir / "xy-vs-libass-diff.png")

        record.update(
            status="OK",
            taiyou_vs_xy_comparison=metrics,
            xy_classification=classification,
            good_changed_pixel_limit=limit,
            xy_raw_sha256=xr.get("xy_raw_sha256"),
            taiyou_raw_sha256=tr.get("taiyou_raw_sha256"),
            artifacts={
                "xy": f"cases/{key}/xy.png",
                "taiyou": f"cases/{key}/taiyou.png",
                "diff": f"cases/{key}/diff.png",
            },
        )
        if libass_diff.is_file():
            record["artifacts"]["xy_vs_libass_diff"] = f"cases/{key}/xy-vs-libass-diff.png"
        (outdir / "case.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
        report["cases"].append(record)
        print(json.dumps({
            "case": case_id,
            "status": record["status"],
            "classification": classification,
            "changed_pixels": metrics["changed_pixels"],
        }), flush=True)

    classes = {
        name: sum(r.get("xy_classification") == name for r in report["cases"])
        for name in ("same", "good", "bad")
    }
    classification_lists = {
        name: [r["id"] for r in report["cases"] if r.get("xy_classification") == name]
        for name in ("same", "good", "bad")
    }
    report["classification_lists"] = classification_lists
    report["summary"] = {
        "total_visual_cases": len(report["cases"]),
        "same": classes["same"],
        "good": classes["good"],
        "bad": classes["bad"],
        "errors": sum(r.get("status") != "OK" for r in report["cases"]),
        "taiyou_vs_xy_exact": classes["same"],
        "libass_vs_xy_exact": sum(
            (r.get("libass_reference_comparison") or {}).get("exact") is True
            for r in report["cases"]
        ),
        "nonvisual_inventory_cases": len(report["nonvisual_inventory_cases"]),
    }
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print("XY_COMPARE_SUMMARY=" + json.dumps(report["summary"]), flush=True)
    print("XY_COMPARE_SAME=" + json.dumps(classification_lists["same"]), flush=True)
    print("XY_COMPARE_GOOD=" + json.dumps(classification_lists["good"]), flush=True)
    print("XY_COMPARE_BAD=" + json.dumps(classification_lists["bad"]), flush=True)
    return 0


def parser():
    p = argparse.ArgumentParser()
    p.add_argument("--mode", choices=("xy", "taiyou", "compare"), required=True)
    p.add_argument("--corpus")
    p.add_argument("--release-root")
    p.add_argument("--taiyou")
    p.add_argument("--xy-input")
    p.add_argument("--taiyou-input")
    p.add_argument("--output", required=True)
    return p


if __name__ == "__main__":
    args = parser().parse_args()
    try:
        if args.mode == "xy":
            if not args.corpus or not args.release_root:
                raise ValueError("xy mode requires --corpus and --release-root")
            sys.exit(render_stage("xy", args))
        if args.mode == "taiyou":
            if not args.corpus or not args.taiyou:
                raise ValueError("taiyou mode requires --corpus and --taiyou")
            sys.exit(render_stage("taiyou", args))
        if not args.xy_input or not args.taiyou_input:
            raise ValueError("compare mode requires --xy-input and --taiyou-input")
        sys.exit(compare_stage(args))
    except Exception as error:
        print(f"PARALLEL_COMPARE_ERROR: {type(error).__name__}: {error}", file=sys.stderr)
        sys.exit(2)
