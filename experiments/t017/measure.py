"""Actions-only paired renderer measurements; no protected input or oracle edits.

Build the reviewed implementation and candidate with identical measurement-only
atomic counters. Reuse the already verified native prefix; never rebuild FT/HB.
Record actual process peak RSS separately from conservative reservation bytes.
"""
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys
import tarfile
import threading
import time
from fontTools.ttLib import TTFont

def instrument_retained_releases(code, statement):
    """Owner-authorized recognition of exactly the two known source shapes."""
    release = "held = held.checked_sub(saved.charge).ok_or_else(limited)?;"
    original = "debug_assert_eq!(saved.id, event.id());"
    backoff = "if let Some((_, Some(saved))) = prepared.pop() {"
    assert code.count(original) == 1, "original retained-layout consumption changed"
    if backoff in code:
        assert code.count(backoff) == 1 and code.count("prepared.pop()") == 1
        expected = 2
    else:
        assert "prepared.pop()" not in code, "unknown prepass release shape"
        expected = 1
    assert code.count(release) == expected, "unknown retained-layout release count"
    return code.replace(release, release + "\n" + statement)


def check_retained_release_shapes():
    release = "held = held.checked_sub(saved.charge).ok_or_else(limited)?;"
    original = "debug_assert_eq!(saved.id, event.id());\n" + release
    backoff = "if let Some((_, Some(saved))) = prepared.pop() {\n" + release
    for code, count in [(original, 1), (backoff + "\n" + original, 2)]:
        assert instrument_retained_releases(code, "MEASURE_RELEASE;").count("MEASURE_RELEASE;") == count
    for code in [original.replace(release, ""), original + release,
                 backoff + original + release, backoff.replace(release, "") + original,
                 backoff + backoff + original]:
        try:
            instrument_retained_releases(code, "MEASURE_RELEASE;")
        except AssertionError:
            continue
        raise AssertionError("unexpected reservation-release source shape accepted")
    print("T017_RELEASE_SOURCE_SHAPES_VERIFIED: old/new and rejected malformed counts", flush=True)


check_retained_release_shapes()

if os.environ.get("GITHUB_ACTIONS") != "true" or platform.system() != "Linux":
    raise SystemExit("measurement is restricted to Linux GitHub Actions")
root = Path(__file__).resolve().parents[2]
out = Path(sys.argv[1]) / "T017-review-measurements"
out.mkdir(parents=True, exist_ok=True)
temp = Path(os.environ["RUNNER_TEMP"]) / "t017-review-build"
temp.mkdir(exist_ok=True)
base = "9eee96fe16989089d8fd3f65b1bb136455ffca7e"
head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
subprocess.run(["git", "fetch", "--no-tags", "origin", base], cwd=root, check=True)
font = Path(os.environ["RUNNER_TEMP"]) / "taiyou-shaping-fonts/latin.otf"
pin = next(f for f in json.loads((root / "build-support/shaping-fonts.proposed.json").read_text())["fonts"] if f["id"] == "latin")
assert hashlib.sha256(font.read_bytes()).hexdigest() == pin["sha256"]
# This pinned workload changed from em20 to positive-cell20 in T018.
# Verify the physical font metrics; do not refresh historical output hashes.
with TTFont(font, recalcTimestamp=False) as sfnt:
    metric_font = {"sha256":pin["sha256"], "upem":sfnt["head"].unitsPerEm,
                   "win_ascent":sfnt["OS/2"].usWinAscent, "win_descent":sfnt["OS/2"].usWinDescent}
assert metric_font == {"sha256":pin["sha256"], "upem":1000, "win_ascent":1000, "win_descent":326}

def snapshot(label, sha):
    destination = temp / label
    destination.mkdir(exist_ok=False)
    archive = temp / (label + ".tar")
    with archive.open("wb") as stream:
        subprocess.run(["git", "archive", sha], cwd=root, stdout=stream, check=True)
    with tarfile.open(archive) as source:
        source.extractall(destination, filter="data")
    path = destination / "crates/taiyou-render/src/renderer.rs"
    original = path.read_text()
    code = original
    counters = ["HELD", "LOCAL", "SHAPES", "RETAINED"]
    prefix = "".join(f'#[cfg(feature = "native")]\nstatic BENCH_{name}: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);\n' for name in counters)
    # Keep the module-level documentation before items.
    first = code.index('\n') + 1
    code = code[:first] + prefix + code[first:]
    marker = "fn available(limits: &Limits, held: u64) -> Result<Limits, RenderError> {"
    assert code.count(marker) == 1
    code = code.replace(marker, marker + '\n    BENCH_HELD.fetch_max(held, std::sync::atomic::Ordering::Relaxed);')
    marker = "let shaped = taiyou_font::shape_event("
    assert code.count(marker) == 1
    code = code.replace(marker, 'BENCH_SHAPES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);\n    ' + marker)
    if label == "before":
        marker = "        let input_charge = bytes::<CollisionInput>(count)?;"
        assert code.count(marker) == 1
        code = code.replace(marker, '        BENCH_LOCAL.fetch_max(prepared.iter().map(|s| s.charge).sum(), std::sync::atomic::Ordering::Relaxed);\n' + marker)
    else:
        # The new helper retains one local artifact, not the full active set.
        marker = "Ok((local, stage_held.checked_sub(held).ok_or_else(limited)?))"
        assert code.count(marker) == 1
        code = code.replace(marker, 'BENCH_LOCAL.fetch_max(BENCH_RETAINED.load(std::sync::atomic::Ordering::Relaxed) + stage_held.checked_sub(held).ok_or_else(limited)?, std::sync::atomic::Ordering::Relaxed);\n    ' + marker)
        if "retained_charge = next;" in code:
            code = code.replace("retained_charge = next;", "retained_charge = next;\nBENCH_RETAINED.store(next, std::sync::atomic::Ordering::Relaxed);")
            code = instrument_retained_releases(code, "BENCH_RETAINED.fetch_sub(saved.charge, std::sync::atomic::Ordering::Relaxed);")
    marker = "        .with_diagnostics(diagnostics)"
    assert code.count(marker) == 1
    code = code.replace(marker, marker + '.inspect(|_| { eprintln!("T017_BENCH={} {} {}", BENCH_HELD.load(std::sync::atomic::Ordering::Relaxed), BENCH_LOCAL.load(std::sync::atomic::Ordering::Relaxed), BENCH_SHAPES.load(std::sync::atomic::Ordering::Relaxed)); })')
    path.write_text(code)
    (out / (label + "-instrumentation.json")).write_text(json.dumps({"sha": sha, "source_sha256": hashlib.sha256(original.encode()).hexdigest(), "instrumented_sha256": hashlib.sha256(code.encode()).hexdigest(), "changes": "measurement-only atomics, including aggregate retained plus current local ownership; no algorithm changes"}, indent=2) + "\n")
    # Owner-authorized snapshot isolation: archive source mtimes can predate the
    # other checkout's fingerprints. Rebuild every workspace package, preserving
    # registry dependencies and the verified native prefix.
    subprocess.run(["cargo", "clean", "-p", "taiyou-types", "-p", "taiyou-ass", "-p", "taiyou-font", "-p", "taiyou-render", "-p", "taiyou-cli", "--target-dir", str(temp / "target")], cwd=destination, check=True, capture_output=True)
    with (out / (label + "-build.log")).open("w") as log:
        subprocess.run(["cargo", "build", "--locked", "-p", "taiyou-cli", "--features", "native", "--bin", "taiyou", "--target-dir", str(temp / "target")], cwd=destination, stdout=log, stderr=subprocess.STDOUT, check=True)
    assert "Compiling taiyou-font" in (out / (label + "-build.log")).read_text(), "font crate must compile from each snapshot"
    # Same compiler/native prefix/lockfiles permit registry artifacts to be
    # reused. Preserve each executable before Cargo builds the next snapshot.
    binary = temp / (label + "-taiyou")
    shutil.copy2(temp / "target/debug/taiyou", binary)
    return binary

from historical import historical, assert_workload, negative_tests
negative_tests()
history = historical("debug",out)
executables = {"after": snapshot("after",head)}
header = "[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,BackColour,Outline,Shadow,Alignment,MarginL,MarginR,MarginV\nStyle: Default,Source Sans 3,20,&H00FFFFFF,&HFF000000,0,0,7,13,21,9\n[Events]\nFormat: Layer,Start,End,Style,Text\n"
def row(text):
    return "Dialogue: 0,0:00:00.00,0:00:02.00,Default," + text + "\n"

cases = [
    ("normal-one", row("ordinary subtitle")),
    ("normal-two", row("ordinary subtitle") * 2),
    ("normal-eight", row("ordinary subtitle") * 8),
    ("positioned-5000-one-glyph", row(r"{\pos(-100,-100)\fs1}H") * 5000),
    ("mixed-2000-one-glyph", row(r"{\pos(-100,-100)\fs1}H") * 2000 + row("H") * 2),
    ("dense-100", row(r"{\fs1}H") * 100),
    ("dense-500", row(r"{\fs1}H") * 500),
    ("dense-1000", row(r"{\fs1}H") * 1000),
]
# Preserve the original larger inputs and the incomplete measurement instead of
# silently relabelling the smaller practical workloads as those measurements.
unavailable = [
    {"case": "positioned-5000-four-glyphs", "events": row(r"{\pos(-100,-100)\fs1}HHHH") * 5000, "status": "baseline-timeout-180s", "evidence": "https://github.com/amanosatosi/taiyou/actions/runs/39183812909/job/117841967333", "completed_frame": False},
    {"case": "mixed-2000-twenty-glyphs", "events": row(r"{\pos(-100,-100)\fs1}HHHHHHHHHHHHHHHHHHHH") * 2000 + row("H") * 2, "status": "not-executed-after-prior-timeout", "completed_frame": False},
]
for item in unavailable:
    script_bytes = (header + item.pop("events")).encode()
    (out / (item["case"] + ".ass")).write_bytes(script_bytes)
    item["script_sha256"] = hashlib.sha256(script_bytes).hexdigest()
(out / "larger-workloads-unavailable.json").write_text(json.dumps(unavailable, indent=2) + "\n")
sample_lock = threading.Lock()

def measure_case(case):
    name, events = case
    records = [r for r in history if r["case"] == name]
    script = out / (name + ".ass")
    script.write_text(header + events)
    assert_workload(history,name,script.read_bytes())
    for sample in range(3):
        for label, binary in executables.items():
            stem = f"{name}-{label}-{sample}"
            rss = out / (stem + ".resource.txt")
            command = ["/usr/bin/time", "-f", "%M %U %S", "-o", str(rss), str(binary), "render", "--input", str(script), "--font", str(font), "--time-us", "750000", "--width", "160", "--height", "100", "--profile", "core-v1", "--output", str(out / (stem + ".png")), "--raw", str(out / (stem + ".rgba"))]
            start = time.perf_counter_ns()
            result = subprocess.run(command, capture_output=True, timeout=180)
            elapsed = time.perf_counter_ns() - start
            (out / (stem + ".stderr.txt")).write_bytes(result.stderr)
            result.check_returncode()
            match = re.search(rb"T017_BENCH=(\d+) (\d+) (\d+)", result.stderr)
            assert match, "missing instrumented metrics"
            peak, local, shapes = map(int, match.groups())
            rss_kib, user, system = rss.read_text().split()
            raw_hash = hashlib.sha256((out / (stem + ".rgba")).read_bytes()).hexdigest()
            record = {"case": name, "implementation": label, "sample": sample, "elapsed_ns": elapsed, "peak_rss_kib": int(rss_kib), "user_seconds": float(user), "system_seconds": float(system), "peak_requested_reservation_bytes": peak, "peak_retained_local_reservation_bytes": local, "shape_calls": shapes, "raw_sha256": raw_hash, "script_sha256": hashlib.sha256(script.read_bytes()).hexdigest(), "exit_code": result.returncode, "evidence_origin": "newly-executed", "commit": head}
            records.append(record)
            with sample_lock:
                with (out / "samples.jsonl").open("a") as samples:
                    samples.write(json.dumps(record) + "\n")
            print("T017_MEMORY_SAMPLE=" + json.dumps(record), flush=True)
    # Cross-generation equality encoded obsolete em sizing. Keep all repeats,
    # inputs and saved hashes, requiring exact equality within each policy.
    for label in ["before", "after"]:
        hashes = {r["raw_sha256"] for r in records if r["case"] == name and r["implementation"] == label}
        assert len(hashes) == 1, f"repeated {label} pixels changed for {name}"
    return records

# Keep every original case and all three before/after pairs. Two bounded case
# workers avoid putting a complete repeated benchmark on the critical path of
# the 35-minute native matrix. Each case still runs its pairs in fixed order.
# Record concurrency explicitly: wall/CPU samples are not sequential-run claims.
with ThreadPoolExecutor(max_workers=2) as workers:
    records = [record for group in workers.map(measure_case, cases) for record in group]
(out / "samples.json").write_text(json.dumps(records, indent=2) + "\n")
environment = {"base": base, "candidate": head, "platform": platform.platform(), "runner_image": os.environ.get("ImageVersion"), "cpuinfo": Path("/proc/cpuinfo").read_text(), "rustc": subprocess.check_output(["rustc", "--version", "--verbose"], text=True), "font": pin, "profile": "debug, identical pinned native prefix, three atomic counters per process", "benchmark_case_workers": 2, "concurrency_semantics": "two cases concurrently, each case's three pairs sequential; other correctness tests may share the runner; do not compare wall times directly with the original sequential experiment", "native_prefix": os.environ.get("PKG_CONFIG_LIBDIR"), "memory_semantics": "RSS is process high-water KiB; reservation bytes are conservative logical accounting, not allocator/RSS claims", "limits": "unchanged CLI defaults"}
environment.update(pixel_identity_policy="positive-cell-height-v2", metric_font=metric_font)
(out / "environment.json").write_text(json.dumps(environment, indent=2) + "\n")
from reuse_completed import validate
validate("debug", records, environment, head)
