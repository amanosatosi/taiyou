"""Common-input Actions performance comparison; no visual-oracle policy changes."""
import csv
import hashlib
import io
import json
import math
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
CORPUS = "a5073319af8117271d8ac13ef5c088197ff2d59b"
SCENARIOS = ("cold", "same-time", "sequence", "seek")
ENGINES = ("taiyou-1", "taiyou-4", "libass")
REFERENCE = os.environ.get("TAIYOU_REFERENCE_BINARY")
if REFERENCE:
    ENGINES = ("taiyou-1", "taiyou-reference", "libass")


def sha(data):
    return hashlib.sha256(data).hexdigest()


def download_asset(path, destination, assets):
    record = assets[path]
    target = record.get("resolved", path)
    pinned = assets[target]
    request = urllib.request.Request(
        f"https://raw.githubusercontent.com/libass/libass-tests/{CORPUS}/{target}",
        headers={"User-Agent": "Taiyou-PR36-performance"})
    with urllib.request.urlopen(request, timeout=90) as response:
        data = response.read(16 * 1024 * 1024 + 1)
    assert len(data) == pinned["bytes"] and sha(data) == pinned["sha256"]
    assert hashlib.sha1(f"blob {len(data)}\0".encode() + data).hexdigest() == pinned["git_blob"]
    destination.write_bytes(data)
    return {"path": str(destination), "upstream_path": target, "sha256": sha(data),
            "bytes": len(data), "license": pinned["license"]}


def prepare(output):
    inputs = Path(os.environ["RUNNER_TEMP"]) / "taiyou-performance-inputs"
    inputs.mkdir(exist_ok=True)
    output.mkdir(exist_ok=True)
    assets = {a["path"]: a for a in json.loads((ROOT / "validation/inventory.json").read_text())["assets"]}
    font = Path(os.environ["TAIYOU_TEST_FONT"])
    copied = inputs / "SourceSans3-Regular.otf"
    copied.write_bytes(font.read_bytes())
    fonts = [{"path": str(copied), "sha256": sha(copied.read_bytes()),
              "bytes": copied.stat().st_size, "license": "OFL-1.1"}]
    for name in ("Arimo-Regular.ttf", "Arimo-Bold.ttf", "IPAexGothic.ttf"):
        fonts.append(download_asset("regression/karaoke/" + name, inputs / name, assets))
    dialogue = "Reusable work, deterministic pixels."
    vector = r"{\p1}m 0 0 b 40 -50 80 90 120 0 b 160 -60 200 80 240 0 l 240 70 0 70{\p0}"
    workloads = {
        "dialogue": (dialogue, 1),
        "thick-borders": (r"{\bord14\shad6}" + dialogue, 3),
        "large-blur": (r"{\bord6\blur12\be1.5}" + dialogue, 2),
        "transforms": (r"{\t(0,5000,\frz24\fscx140\blur4)}" + dialogue, 2),
        "combined-abuse": (r"{\bord10\shad7\blur6\be1.5\frz8\clip(0,0,1900,1050)\t(0,5000,\frz30\fscx130\1c&HFF0080&)}" + vector + dialogue, 8),
    }
    sources = {}
    for name, (text, count) in workloads.items():
        source = "[Script Info]\nScriptType: v4.00+\nPlayResX: 1920\nPlayResY: 1080\nWrapStyle: 2\nScaledBorderAndShadow: yes\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,SecondaryColour,OutlineColour,BackColour,Bold,Italic,Underline,StrikeOut,ScaleX,ScaleY,Spacing,Angle,BorderStyle,Outline,Shadow,Alignment,MarginL,MarginR,MarginV,Encoding\nStyle: Default,Source Sans 3,48,&H00FFFFFF,&H0000FFFF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,2,2,10,10,10,1\n[Events]\nFormat: Layer,Start,End,Text\n"
        source = source.replace("Format: Layer,Start,End,Text", "Format: Layer,Start,End,Style,Text")
        source += "".join(f"Dialogue: {n},0:00:00.00,0:00:10.00,Default,{text}\n" for n in range(count))
        path = inputs / (name + ".ass")
        path.write_bytes(source.encode())
        sources[name] = {"path": str(path), "sha256": sha(path.read_bytes()), "events": count,
                         "origin": "existing text/tags/font size/event count; explicit frozen Taiyou Core style defaults"}
    path = inputs / "upstream-karaoke.ass"
    sources["upstream-karaoke"] = download_asset(
        "regression/karaoke/357-k-and-kf-desynced.ass", path, assets)
    cases = []
    for name, width in [("dialogue", 1920), ("thick-borders", 1920), ("dialogue", 3840),
                        ("combined-abuse", 3840), ("large-blur", 3840),
                        ("transforms", 3840), ("upstream-karaoke", 1920)]:
        case = {"id": f"{name}-{width}", "workload": name, "width": width,
                "height": width * 9 // 16, "source": sources[name], "timestamps": {}}
        base = 6798 if name == "upstream-karaoke" else 1000
        seeks = [8170, 6798, 7170, 6798] if name == "upstream-karaoke" else [7500, 100, 4000, 1000]
        for scenario in SCENARIOS:
            values = [base + n * 16 if scenario == "sequence" else seeks[n % 4] if scenario == "seek" else base for n in range(31)]
            stamp = inputs / (case["id"] + "-" + scenario + ".timestamps")
            stamp.write_text("".join(f"{value}\n" for value in values))
            case["timestamps"][scenario] = {"path": str(stamp), "values_ms": values,
                                             "sha256": sha(stamp.read_bytes())}
        cases.append(case)
    manifest = {"schema": 1, "cases": cases, "fonts": fonts, "corpus": CORPUS,
                "samples_per_group": 31, "warmup_frames": 1, "mt_workers": 4,
                "taiyou_cache_limit": 128 * 1024 * 1024,
                "benchmark_transient_limit": 512 * 1024 * 1024,
                "benchmark_work_limit": 32_000_000,
                "libass_cache_policy": "normal library defaults; no cache limit overrides",
                "style_policy": "all authored style fields and event Style=Default explicit; Taiyou RGBA must equal the retained sparse-style run",
                "timestamp_policy": "integer milliseconds for both APIs; 16ms sequence step"}
    (output / "inputs.json").write_text(json.dumps(manifest, indent=2) + "\n")
    for path in inputs.iterdir():
        path.chmod(0o444)


def verify_inputs(manifest):
    for record in manifest["fonts"] + [c["source"] for c in manifest["cases"]]:
        assert sha(Path(record["path"]).read_bytes()) == record["sha256"]
    for case in manifest["cases"]:
        for record in case["timestamps"].values():
            assert sha(Path(record["path"]).read_bytes()) == record["sha256"]


def distribution(values):
    values = sorted(values)
    return {label: values[math.ceil(q * len(values)) - 1] / 1e6
            for label, q in (("median", .5), ("p95", .95), ("p99", .99))}


def run(output):
    manifest = json.loads((output / "inputs.json").read_text())
    verify_inputs(manifest)
    fonts = [f["path"] for f in manifest["fonts"]]
    rows, failures, processes, versions = [], [], [], set()
    resources = {}
    resource_differences = []
    for case_index, case in enumerate(manifest["cases"]):
        for scenario_index, scenario in enumerate(SCENARIOS):
            # Deterministic rotation avoids always putting one engine last.
            shift = (case_index + scenario_index) % 3
            order = ENGINES[shift:] + ENGINES[:shift]
            for engine in order:
                directory = output / case["id"] / scenario
                directory.mkdir(parents=True, exist_ok=True)
                binary = (REFERENCE if engine == "taiyou-reference" else
                          os.environ["TAIYOU_COMPARE_BINARY"] if engine != "libass" else os.environ["LIBASS_COMPARE_BINARY"])
                workers = "4" if engine == "taiyou-4" else "1"
                arguments = [binary, case["source"]["path"], str(case["width"]), str(case["height"]), workers,
                             case["timestamps"][scenario]["path"], scenario, *fonts]
                command = ["/usr/bin/time", "-v", "-o", str(directory / (engine + "-resources.txt")), *arguments]
                start = time.perf_counter_ns()
                try:
                    process = subprocess.run(command, capture_output=True, timeout=240)
                    elapsed = time.perf_counter_ns() - start
                    (directory / (engine + ".stderr.txt")).write_bytes(process.stderr)
                    (directory / (engine + ".csv")).write_bytes(process.stdout)
                    processes.append({"case": case["id"], "scenario": scenario, "engine": engine,
                                      "exit_code": process.returncode, "process_elapsed_ns": elapsed,
                                      "command": command})
                    assert process.returncode == 0, f"process exit {process.returncode}"
                    if REFERENCE and engine != "libass":
                        observed = re.findall(r"^POST_SAMPLE_DEFAULT_RESOURCE_(.+)$", process.stderr.decode(), re.M)
                        assert len(observed) == 2, "missing normal-limit cache OFF/ON observations"
                        resources[(case["id"], scenario, engine)] = observed
                    measured = list(csv.DictReader(io.StringIO(process.stdout.decode())))
                    assert len(measured) == 31
                    assert [int(r["sample"]) for r in measured] == list(range(31))
                    assert [int(r["timestamp_ms"]) for r in measured] == case["timestamps"][scenario]["values_ms"]
                    if engine == "libass":
                        version = re.findall(r"LIBASS_LIBRARY_VERSION=(0x[0-9a-f]+)", process.stderr.decode())
                        assert len(version) == 1
                        versions.add(version[0])
                    for row in measured:
                        row.update(case=case["id"], scenario=scenario, engine=engine)
                    rows.extend(measured)
                    print(f"COMPARED {case['id']} {scenario} {engine}: 31 samples", flush=True)
                except (AssertionError, OSError, subprocess.SubprocessError) as error:
                    failures.append({"case": case["id"], "scenario": scenario, "engine": engine, "error": str(error)})
                    print(f"FAILED {case['id']} {scenario} {engine}: {error}", flush=True)
    verify_inputs(manifest)
    assert len(versions) == 1, "missing/inconsistent compiled libass version"
    summaries = []
    digest_differences = []
    for case in manifest["cases"]:
        for scenario in SCENARIOS:
            for engine in ENGINES:
                selected = [r for r in rows if (r["case"], r["scenario"], r["engine"]) == (case["id"], scenario, engine)]
                if len(selected) != 31:
                    continue
                totals = [int(r["total_ns"]) for r in selected]
                cold = [int(r["total_ns"]) + int(r["renderer_setup_ns"]) for r in selected]
                record = {"case": case["id"], "scenario": scenario, "engine": engine, "samples": 31,
                          "total_ms": distribution(totals), "cold_session_ms": distribution(cold),
                          "fps_equivalent": len(totals) * 1e9 / sum(totals),
                          "nonzero_alpha_range": [min(int(r["nonzero_alpha"]) for r in selected), max(int(r["nonzero_alpha"]) for r in selected)],
                          "font_setup_ms": int(selected[0]["font_setup_ns"]) / 1e6,
                          "source_setup_ms": int(selected[0]["source_setup_ns"]) / 1e6,
                          "renderer_setup_ms": distribution([int(r["renderer_setup_ns"]) for r in selected])}
                if engine == "libass":
                    record.update(library_render_ms=distribution([int(r["library_render_ns"]) for r in selected]),
                                  client_compose_ms=distribution([int(r["client_compose_ns"]) for r in selected]),
                                  warnings=max(int(r["warning_count"]) for r in selected))
                else:
                    record["peak_cache_bytes"] = max(int(r["cache_bytes"]) for r in selected)
                summaries.append(record)
            one = [r for r in rows if (r["case"], r["scenario"], r["engine"]) == (case["id"], scenario, "taiyou-1")]
            peer = "taiyou-reference" if REFERENCE else "taiyou-4"
            many = [r for r in rows if (r["case"], r["scenario"], r["engine"]) == (case["id"], scenario, peer)]
            if len(one) == len(many) == 31:
                if REFERENCE:
                    if resources[(case["id"], scenario, "taiyou-1")] != resources[(case["id"], scenario, peer)]:
                        resource_differences.append({"case": case["id"], "scenario": scenario,
                                                     "candidate": resources[(case["id"], scenario, "taiyou-1")],
                                                     "reference": resources[(case["id"], scenario, peer)]})
                for a, b in zip(one, many):
                    assert len(a["rgba_digest"]) == len(a["diagnostic_digest"]) == 64
                    if (a["rgba_digest"], a["diagnostic_digest"]) != (b["rgba_digest"], b["diagnostic_digest"]):
                        digest_differences.append([case["id"], scenario, a["sample"]])
    prior_file = ROOT / "experiments/performance/results/libass-comparison/samples.csv"
    with prior_file.open() as stream:
        prior = {(r["case"], r["scenario"], r["engine"], r["sample"]): r["rgba_digest"]
                 for r in csv.DictReader(stream) if r["engine"] in ENGINES and r["engine"].startswith("taiyou-")}
    current = {(r["case"], r["scenario"], r["engine"], r["sample"]): r["rgba_digest"]
               for r in rows if r["engine"] in ("taiyou-1", "taiyou-4")}
    prior_changes = [key for key in prior.keys() | current.keys() if prior.get(key) != current.get(key)]
    report = {"schema": 1, "libass_api_version": sorted(versions), "groups": summaries,
              "engines": ENGINES, "same_runner_1t_reference": os.environ.get("SERIAL_REFERENCE") if REFERENCE else None,
              "normal_resource_differences": resource_differences,
              "failures": failures, "taiyou_digest_differences": digest_differences,
              "sparse_style_taiyou_rgba_differences": prior_changes,
              "visual_policy": "No cross-engine visual equality requirement; differences are valid performance samples",
              "primary_taiyou_timing": "render_parsed call only; renderer/font/source setup, stage timers and frame/diagnostic SHA-256 outside timing",
              "primary_libass_timing": "direct ass_render_frame call only (library_render_ns); renderer setup and host composition outside primary timing",
              "secondary_libass_timing": "client_compose_ns retains scalar CPU RGBA allocation/clear/composition; total_ns retains ass_render_frame plus that host work",
              "libass_digest": "FNV anti-elision only; not a cryptographic visual oracle",
              "processes": processes}
    (output / "comparison.json").write_text(json.dumps(report, indent=2) + "\n")
    fields = sorted({key for row in rows for key in row})
    with (output / "samples.csv").open("w", newline="") as stream:
        writer = csv.DictWriter(stream, fields)
        writer.writeheader()
        writer.writerows(rows)
    peer_title = "reference1T" if REFERENCE else "Taiyou4T"
    peer_engine = "taiyou-reference" if REFERENCE else "taiyou-4"
    ratio_title = "1T/libass" if REFERENCE else "MT/libass"
    table = [f"| Workload / state | Taiyou1T ms | {peer_title} ms | libass ass_render_frame ms | {ratio_title} | libass host RGBA ms | libass total ms |",
             "|---|---:|---:|---:|---:|---:|---:|"]
    for case in manifest["cases"]:
        for scenario in ("same-time", "sequence", "cold"):
            group = {r["engine"]: r for r in summaries if r["case"] == case["id"] and r["scenario"] == scenario}
            if len(group) == 3:
                a = group["taiyou-1"]["total_ms"]["median"]
                b = group[peer_engine]["total_ms"]["median"]
                c = group["libass"]["library_render_ms"]["median"]
                host = group["libass"]["client_compose_ms"]["median"]
                total = group["libass"]["total_ms"]["median"]
                ratio = a / c if REFERENCE else b / c
                table.append(f"| {case['id']} / {scenario} | {a:.3f} | {b:.3f} | {c:.3f} | {ratio:.2f}x | {host:.3f} | {total:.3f} |")
    (output / "table.md").write_text("\n".join(table) + "\n")
    serial = ["| Workload/state | Taiyou1T ms | libass ass_render_frame ms | 1T/libass | libass host RGBA ms | libass total ms | Painted pixels T/L | libass images | Taiyou bottleneck (prior profile) |",
              "|---|---:|---:|---:|---:|---:|---|---:|---|"]
    for case in manifest["cases"]:
        for scenario in SCENARIOS:
            group = {r["engine"]: r for r in summaries if r["case"] == case["id"] and r["scenario"] == scenario}
            if "taiyou-1" not in group or "libass" not in group:
                continue
            a, b = group["taiyou-1"], group["libass"]
            ta = a["total_ms"]["median"]
            lb = b["library_render_ms"]["median"]
            host = b["client_compose_ms"]["median"]
            total = b["total_ms"]["median"]
            area_a, area_b = a["nonzero_alpha_range"], b["nonzero_alpha_range"]
            images = [int(r["image_count"]) for r in rows if (r["case"], r["scenario"], r["engine"]) == (case["id"], scenario, "libass")]
            stage = ("span union/filter/masks; composition" if scenario in ("cold", "sequence", "seek") and case["workload"] != "dialogue"
                     else "composition/publication; base geometry")
            serial.append(f"| {case['id']}/{scenario} | {ta:.3f} | {lb:.3f} | {ta/lb:.2f}x | {host:.3f} | {total:.3f} | {area_a[0]}-{area_a[1]} / {area_b[0]}-{area_b[1]} | {min(images)}-{max(images)} | {stage} |")
    (output / "1t-table.md").write_text("\n".join(serial) + "\n")
    assert len(rows) == len(manifest["cases"]) * len(SCENARIOS) * 31 * len(ENGINES), "incomplete comparison inventory"
    assert not failures and not digest_differences, "process/sample/determinism failures remain failures"
    assert not prior_changes, "explicit style fields changed Taiyou output"
    assert not resource_differences, "normal resource result or exact output changed"


if __name__ == "__main__":
    assert os.environ.get("GITHUB_ACTIONS") == "true", "CI-only execution"
    action, folder = sys.argv[1:]
    (prepare if action == "prepare" else run)(Path(folder))
