"""Additive Actions-only release experiment; original debug evidence is untouched."""
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
import time

assert os.environ.get("GITHUB_ACTIONS") == "true" and platform.system() == "Linux"
root = Path(__file__).resolve().parents[2]
out = Path(sys.argv[1]) / "T017-optimized"
out.mkdir(parents=True, exist_ok=True)
temp = Path(os.environ["RUNNER_TEMP"]) / "t017-optimized-build"
temp.mkdir(exist_ok=True)
head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
versions = {"earlier": "9eee96fe16989089d8fd3f65b1bb136455ffca7e", "reviewed": "027e58a52efdd870b99f49ab5f3bae6e952f8f59", "candidate": head}
stages = ["evaluation", "shaping", "layout", "footprint", "collision", "outlines", "geometry", "raster", "stroke", "shadow", "composition", "authentication", "font_resolution", "native_shaping"]

def inject(code, name, index, statement=None):
    pattern = r"((?:pub(?:\(crate\))? )?fn " + name + r"\([^{}]*?\)\s*(?:->[^{}]*?)?\{)"
    code, count = re.subn(pattern, lambda m: m[0] + "\n" + (statement or f"let _measurement = taiyou_types::t017_metrics::guard({index});"), code, count=1, flags=re.S)
    assert count == 1, name
    return code

def build(label, sha):
    subprocess.run(["git", "fetch", "--no-tags", "origin", sha], cwd=root, check=True, capture_output=True)
    destination = temp / label
    destination.mkdir()
    archive = temp / (label + ".tar")
    with archive.open("wb") as stream:
        subprocess.run(["git", "archive", sha], cwd=root, stdout=stream, check=True)
    with tarfile.open(archive) as source:
        source.extractall(destination, filter="data")
    metrics = '''use std::{sync::atomic::{AtomicU64,Ordering},time::Instant};
static CALLS:[AtomicU64;14]=[const {AtomicU64::new(0)};14];
static NS:[AtomicU64;14]=[const {AtomicU64::new(0)};14];
static PEAK:[AtomicU64;2]=[const {AtomicU64::new(0)};2];
static RETAINED:AtomicU64=AtomicU64::new(0);
pub struct Guard(usize,Instant);
pub fn guard(i:usize)->Guard {CALLS[i].fetch_add(1,Ordering::Relaxed);Guard(i,Instant::now())}
impl Drop for Guard {fn drop(&mut self){NS[self.0].fetch_add(self.1.elapsed().as_nanos() as u64,Ordering::Relaxed);}}
pub fn maximum(i:usize,n:u64){PEAK[i].fetch_max(n,Ordering::Relaxed);}
pub fn retain(n:u64){RETAINED.store(n,Ordering::Relaxed);maximum(1,n);}
pub fn release(n:u64){RETAINED.fetch_sub(n,Ordering::Relaxed);}
pub fn local(n:u64){maximum(1,n+RETAINED.load(Ordering::Relaxed));}
pub fn report(){eprintln!("T017_PROFILE={:?}|{:?}|{:?}",CALLS.each_ref().map(|n|n.load(Ordering::Relaxed)),NS.each_ref().map(|n|n.load(Ordering::Relaxed)),PEAK.each_ref().map(|n|n.load(Ordering::Relaxed)));}
pub struct Report;
impl Drop for Report {fn drop(&mut self){report();}}
'''
    (destination / "crates/taiyou-types/src/t017_metrics.rs").write_text(metrics)
    types = destination / "crates/taiyou-types/src/lib.rs"
    types.write_text(types.read_text() + "\npub mod t017_metrics;\n")
    modifications = {}
    mapping = {
        "crates/taiyou-render/src/evaluate.rs": [("one", 0)],
        "crates/taiyou-font/src/shape.rs": [("shape_event_impl" if label == "candidate" else "shape_event", 1), ("priorities", 12)],
        "crates/taiyou-render/src/layout.rs": [("layout", 2)],
        "crates/taiyou-render/src/collision.rs": [("resolve_collisions", 4)],
        "crates/taiyou-render/src/geometry.rs": [("geometry", 6)],
        "crates/taiyou-render/src/compose.rs": [("compose", 10)],
        "crates/taiyou-font/src/catalog.rs": [("check_blob_identity", 11)],
        "crates/taiyou-font/src/native.rs": [("outline_authenticated" if label == "candidate" else "outline", 5), ("rasterize_path", 7), ("stroke_rasterize_path", 8), ("shape_authenticated_with_ink" if label == "candidate" else "shape_raw", 13)],
    }
    footprint = destination / "crates/taiyou-render/src/collision_footprint.rs"
    if footprint.exists():
        mapping[str(footprint.relative_to(destination))] = [("collision_footprint", 3)]
    for relative, functions in mapping.items():
        path = destination / relative
        original = path.read_text()
        code = original
        for function, index in functions:
            code = inject(code, function, index)
        path.write_text(code)
        modifications[relative] = {"original": hashlib.sha256(original.encode()).hexdigest(), "instrumented": hashlib.sha256(code.encode()).hexdigest()}
    path = destination / "crates/taiyou-render/src/renderer.rs"
    original = path.read_text()
    marker = "fn available(limits: &Limits, held: u64) -> Result<Limits, RenderError> {"
    code = original.replace(marker, marker + "\ntaiyou_types::t017_metrics::maximum(0,held);")
    if label == "earlier":
        marker = "let input_charge = bytes::<CollisionInput>(count)?;"
        code = code.replace(marker, "taiyou_types::t017_metrics::maximum(1,prepared.iter().map(|s|s.charge).sum());\n" + marker)
    else:
        marker = "Ok((local, stage_held.checked_sub(held).ok_or_else(limited)?))"
        code = code.replace(marker, "taiyou_types::t017_metrics::local(stage_held-held);\n" + marker)
        if label == "candidate":
            marker = "retained_charge = next;"
            code = code.replace(marker, marker + "\ntaiyou_types::t017_metrics::retain(next);")
            marker = "held = held.checked_sub(saved.charge).ok_or_else(limited)?;"
            assert code.count(marker) == 1
            code = code.replace(marker, marker + "\ntaiyou_types::t017_metrics::release(saved.charge);")
    marker = ".with_diagnostics(diagnostics)"
    assert code.count(marker) == 1
    code = inject(code, "render", 0, "let _measurement_report = taiyou_types::t017_metrics::Report;")
    code, shadow_scopes = re.subn(r"(let shadow = if state\.paint\(\)\.shadow\(\)\.a\(\) == 0(?:\s*\|\|\s*\(state\.effects\(\)\.shadow_x\(\)\.get\(\) == 0\.0\s*&& state\.effects\(\)\.shadow_y\(\)\.get\(\) == 0\.0\))?\s*\{\s*None\s*\} else \{)", r"\1\nlet _shadow_measurement = taiyou_types::t017_metrics::guard(9);", code)
    assert shadow_scopes == 1
    if label == "candidate":
        marker = "        let limits = &self.config.limits;"
        # First occurrence in render_staged; the public wrapper has no limits.
        assert code.count(marker) == 1
        code = code.replace(marker, '        taiyou_types::t017_metrics::retain(0); // A retry has dropped all previous ownership; preserve only the recorded peak.\n        let retain = retain && std::env::var_os("TAIYOU_EXPERIMENT_MINIMAL").is_none();\n' + marker)
    path.write_text(code)
    modifications[str(path.relative_to(destination))] = {"original": hashlib.sha256(original.encode()).hexdigest(), "instrumented": hashlib.sha256(code.encode()).hexdigest()}
    cli_path = destination / "crates/taiyou-cli/src/render.rs"
    cli_original = cli_path.read_text()
    markers = ("let config = RendererConfig::default();", "let mut config = RendererConfig::default();")
    assert sum(cli_original.count(value) for value in markers) == 1
    marker = next(value for value in markers if value in cli_original)
    cli_code = cli_original.replace(marker, 'let mut config = RendererConfig::default(); if let Ok(bytes) = std::env::var("TAIYOU_EXPERIMENT_BYTES") {config.limits = config.limits.with_transient_bytes(bytes.parse().unwrap()).unwrap();} if let Ok(segments) = std::env::var("TAIYOU_EXPERIMENT_SEGMENTS") {config.limits = config.limits.with_path_segments_per_frame(segments.parse().unwrap()).unwrap();}')
    cli_path.write_text(cli_code)
    modifications[str(cli_path.relative_to(destination))] = {"original": hashlib.sha256(cli_original.encode()).hexdigest(), "instrumented": hashlib.sha256(cli_code.encode()).hexdigest(), "scope": "experiment-only explicit caller budget override, same implementation for every snapshot"}
    driver = r'''use std::{path::PathBuf,time::Instant};
use taiyou_render::{renderer::*,shaping::shape_event};
fn main(){
let limits=Limits::default().with_transient_bytes(512*1024*1024).unwrap();
let mut ctx=FontContext::with_limits(limits.clone()).unwrap();
let mut builder=FontCatalogBuilder::new(limits.clone());
let path=PathBuf::from(std::env::var_os("RUNNER_TEMP").unwrap()).join("taiyou-shaping-fonts/cjk.otf");
builder.register(&mut ctx,&std::fs::read(path).unwrap(),0).unwrap();
let fonts=builder.snapshot(&mut CatalogIngestor::new()).unwrap();
let mut parse_ns=0_u128; let mut eval_ns=0_u128; let mut shape_ns=0_u128;
let mut identity=String::new();
for n in [1,3,6,9,12,15,18,21,24,27,30,36]{
let text="日本語字幕表示確認".repeat(4).chars().take(n).collect::<String>();
let source=format!("[Script Info]\nPlayResX: 384\nPlayResY: 288\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow\nStyle: Default,Noto Sans CJK JP,20,0,0\n[Events]\nFormat: Layer,Start,End,Style,Text\nDialogue: 0,0:00:00.00,0:00:02.00,Default,{text}");
let t=Instant::now();
let parsed=taiyou_ass::parse_ass(source.as_bytes(),&ParseOptions::new(InputEncoding::Utf8,Profile::CoreV1,limits.clone(),SyntaxPolicy::Strict),&mut TrackIngestor::new()).unwrap(); parse_ns+=t.elapsed().as_nanos();
let t=Instant::now(); let event=taiyou_render::evaluate(parsed.track(),EventId::new(0),TimeUs::from_micros(750000),Profile::CoreV1).unwrap(); eval_ns+=t.elapsed().as_nanos();
let t=Instant::now(); let shaped=shape_event(&mut ctx,&fonts,&event,&Default::default(),&limits).unwrap(); shape_ns+=t.elapsed().as_nanos();
for run in shaped.runs(){for glyph in run.glyphs(){identity.push_str(&format!("{}:{:?}:{:?};",glyph.glyph(),glyph.advance_x(),glyph.offset_x()));}}
}
eprintln!("T017_WRAP_PROFILE={{\"parse_ns\":{parse_ns},\"evaluation_ns\":{eval_ns},\"shape_ns\":{shape_ns}}}");
println!("{identity}"); taiyou_render::renderer::t017_metrics::report();
}
'''
    driver_path = destination / "crates/taiyou-cli/src/bin/t017-width-profile.rs"
    driver_path.write_text(driver)
    modifications[str(driver_path.relative_to(destination))] = {"generated_driver_sha256": hashlib.sha256(driver.encode()).hexdigest(), "purpose": "12 fixed CJK boundary candidates, original parse/evaluate/full-shape harness stage profile"}
    (out / (label + "-instrumentation.json")).write_text(json.dumps({"commit": sha, "files": modifications, "scope": "measurement-only timers/counters, inclusive nested stage times"}, indent=2))
    # Separate checkout mtimes can predate another checkout's successful build.
    # Remove workspace artifacts explicitly; keep verified native/registry deps.
    subprocess.run(["cargo", "clean", "--release", "-p", "taiyou-types", "-p", "taiyou-ass", "-p", "taiyou-font", "-p", "taiyou-render", "-p", "taiyou-cli", "--target-dir", str(temp / "target")], cwd=destination, check=True, capture_output=True)
    with (out / (label + "-build.log")).open("w") as log:
        subprocess.run(["cargo", "build", "--release", "--locked", "-p", "taiyou-cli", "--features", "native", "--bin", "taiyou", "--bin", "t017-width-profile", "--target-dir", str(temp / "target")], cwd=destination, stdout=log, stderr=subprocess.STDOUT, check=True)
    assert "Compiling taiyou-font" in (out / (label + "-build.log")).read_text(), "font crate must compile from each measured snapshot"
    profile = subprocess.run([str(temp / "target/release/t017-width-profile")], capture_output=True, timeout=30, check=True)
    (out / (label + "-cjk-profile.stderr.txt")).write_bytes(profile.stderr)
    (out / (label + "-cjk-shape-identity.txt")).write_bytes(profile.stdout)
    print("T017_CJK_PROFILE=" + label + " " + profile.stderr.decode(), flush=True)
    binary = temp / (label + "-taiyou")
    shutil.copy2(temp / "target/release/taiyou", binary)
    return binary

from historical import historical, assert_workload, negative_tests
negative_tests()
history = historical("optimized",out)
binaries = {"candidate": build("candidate",head)}
profiles = [out / (("historical/" + label if label != "candidate" else label) + "-cjk-shape-identity.txt") for label in versions]
assert len({hashlib.sha256(path.read_bytes()).hexdigest() for path in profiles}) == 1, "CJK glyph/advance/offset identity changed"
font = Path(os.environ["RUNNER_TEMP"]) / "taiyou-shaping-fonts/latin.otf"
pin = next(f for f in json.loads((root / "build-support/shaping-fonts.proposed.json").read_text())["fonts"] if f["id"] == "latin")
assert hashlib.sha256(font.read_bytes()).hexdigest() == pin["sha256"]
header = "[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,BackColour,Outline,Shadow,Alignment,MarginL,MarginR,MarginV\nStyle: Default,Source Sans 3,20,&H00FFFFFF,&HFF000000,0,0,7,13,21,9\n[Events]\nFormat: Layer,Start,End,Style,Text\n"
def row(text):
    return "Dialogue: 0,0:00:00.00,0:00:02.00,Default," + text + "\n"
cases = [("normal-one", row("ordinary subtitle")), ("normal-eight", row("ordinary subtitle") * 8)]
cases += [(f"dense-{n}", row(r"{\fs1}H") * n) for n in [100, 500, 1000]]
cases += [("mixed-2000", row(r"{\pos(-100,-100)\fs1}H") * 2000 + row("dialogue") * 2), ("positioned-5000", row(r"{\pos(-100,-100)\fs1}H") * 5000)]
cases += [("adaptive-256-multiglyph", row(r"{\fs5\bord0.5\shad1.25\4a&H00&}" + "H" * 32) * 256)]
for n in [100, 500]:
    for border, shadow in [(0.5, "&HFF&"), (3, "&H00&")]:
        typesetting = "".join(row(fr"{{\pos({(i % 10) * 16},{(i // 10 % 10) * 10})\fs5\bord{border}\shad1.25\4a{shadow}}}Fansub typesetting") for i in range(n-2))
        cases.append((f"typesetting-{n}-border-{border}", typesetting + row("dialogue") * 2))
cases.append(("typesetting-hd-100", "".join(row(fr"{{\pos({(i % 10) * 192},{(i // 10) * 100})\fs32\bord2\shad2\4a&H00&}}Fansub typesetting") for i in range(98)) + row("ordinary dialogue") * 2))
records = []
budget = time.monotonic() + 12 * 60
for name, events in cases:
    script = out / (name + ".ass")
    hd = name == "typesetting-hd-100"
    script.write_text((header.replace("PlayResX: 160", "PlayResX: 1920").replace("PlayResY: 100", "PlayResY: 1080") if hd else header) + events)
    assert_workload(history,name,script.read_bytes())
    records.extend(r for r in history if r["case"] == name)
    for sample in range(3):
        variants = [(label, binary, versions[label], dict(os.environ)) for label, binary in binaries.items()]
        if name.startswith("typesetting-500-"):
            # Preserve every default-limit failure, then independently measure
            # the same workload/all versions with an explicit caller budget.
            # Production defaults and the external comparison remain untouched.
            variants += [(label + "-4Msegments", binary, versions[label], dict(os.environ, TAIYOU_EXPERIMENT_SEGMENTS="4000000")) for label, binary in binaries.items()]
        if name == "adaptive-256-multiglyph":
            for mib, minimal in [(32, False), (32, True), (128, True)]:
                env = dict(os.environ, TAIYOU_EXPERIMENT_BYTES=str(mib*1024*1024))
                if minimal:
                    env["TAIYOU_EXPERIMENT_MINIMAL"] = "1"
                variants.append((f"candidate-{'minimal' if minimal else 'adaptive'}-{mib}MiB", binaries["candidate"], head, env))
        for label, binary, commit, env in variants:
            if time.monotonic() > budget:
                raise RuntimeError("bounded experiment exceeded 12 minutes; partial JSONL remains evidence")
            stem = f"{name}-{label}-{sample}"
            resource = out / (stem + ".resource.txt")
            raw = out / (stem + ".rgba")
            command = ["/usr/bin/time", "-f", "%M %U %S", "-o", str(resource), str(binary), "render", "--input", str(script), "--font", str(font), "--time-us", "750000", "--width", "1920" if hd else "160", "--height", "1080" if hd else "100", "--profile", "core-v1", "--output", str(out / (stem + ".png")), "--raw", str(raw)]
            start = time.perf_counter_ns()
            result = subprocess.run(command, capture_output=True, timeout=40, env=env)
            elapsed = time.perf_counter_ns() - start
            (out / (stem + ".stderr.txt")).write_bytes(result.stderr)
            match = re.search(rb"T017_PROFILE=(\[.*?\])\|(\[.*?\])\|(\[.*?\])", result.stderr)
            assert match, result.stderr
            calls, ns, peaks = map(json.loads, match.groups())
            rss, user, system = resource.read_text().splitlines()[-1].split()
            record = {"case": name, "implementation": label, "commit": commit, "transient_limit_bytes": int(env.get("TAIYOU_EXPERIMENT_BYTES",128*1024*1024)), "path_segments_per_frame": int(env.get("TAIYOU_EXPERIMENT_SEGMENTS",1000000)), "sample": sample, "elapsed_ns": elapsed, "peak_rss_kib": int(rss), "user_seconds": float(user), "system_seconds": float(system), "stage_calls": dict(zip(stages, calls)), "inclusive_stage_ns": dict(zip(stages, ns)), "peak_reservation_bytes": peaks[0], "peak_local_reservation_bytes": peaks[1], "raw_sha256": hashlib.sha256(raw.read_bytes()).hexdigest() if result.returncode == 0 else None, "script_sha256": hashlib.sha256(script.read_bytes()).hexdigest(), "exit_code": result.returncode, "status": "completed-frame" if result.returncode == 0 else "renderer-error-no-frame", "error": result.stderr.decode() if result.returncode != 0 else None, "evidence_origin": "newly-executed"}
            records.append(record)
            with (out / "samples.jsonl").open("a") as stream:
                stream.write(json.dumps(record) + "\n")
            print("T017_OPTIMIZED=" + json.dumps(record), flush=True)
            if result.returncode != 0:
                assert not raw.exists(), "a failed frame must not publish raw output"
                assert b"LimitExceeded" in result.stderr, "unexpected non-limit rendering failure"
    selected = [r for r in records if r["case"] == name]
    for reviewed in [r for r in selected if r["implementation"].startswith("reviewed") and r["exit_code"] == 0]:
        assert all(r["exit_code"] == 0 for r in selected if r["implementation"].startswith("candidate") and r["path_segments_per_frame"] == reviewed["path_segments_per_frame"]), "new candidate failure: " + name
    successful = [r for r in selected if r["exit_code"] == 0 and (r["implementation"].startswith("reviewed") or r["implementation"].startswith("candidate"))]
    assert len({r["raw_sha256"] for r in successful}) <= 1, name
(out / "samples.jsonl").write_text("".join(json.dumps(r)+"\n" for r in records))
(out / "environment.json").write_text(json.dumps({"versions": versions, "font": pin, "profile": "release default opt-level=3, sequential paired three-sample processes; inclusive timers", "platform": platform.platform(), "runner_image": os.environ.get("ImageVersion"), "cpuinfo": Path("/proc/cpuinfo").read_text(), "rustc": subprocess.check_output(["rustc", "--version", "--verbose"], text=True), "native_prefix": os.environ.get("PKG_CONFIG_LIBDIR"), "limitations": "timers overlap/nest, other correctness tests may share the runner; not isolated playback throughput; no retry of original four-glyph 180-second timeout"}, indent=2))
from reuse_completed import validate
validate("optimized", records, json.loads((out / "environment.json").read_text()), head)
