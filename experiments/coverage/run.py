"""CI-only R002 harness. New outputs are experiments, never accepted references."""
import csv
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import statistics
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(os.environ["RUNNER_TEMP"]) / "r002-coverage"
OUT.mkdir(exist_ok=True)
PREFIX = Path(os.environ["RUNNER_TEMP"]) / "taiyou-native-install"
env = dict(os.environ, PKG_CONFIG_PATH=str(PREFIX / "lib/pkgconfig"),
           PKG_CONFIG_LIBDIR=str(PREFIX / "lib/pkgconfig"),
           PKG_CONFIG_ALL_STATIC="1", LIBZ_SYS_STATIC="1", RUSTUP_TOOLCHAIN="1.99.1")
driver = Path(os.environ["RUNNER_TEMP"]) / "r002-driver"
driver.mkdir(exist_ok=True)
(driver / "src").mkdir(exist_ok=True)
shutil.copyfile(ROOT / "experiments/coverage/baseline.rs", driver / "src/main.rs")
manifest = '[package]\nname="r002-coverage-experiment"\nversion="0.0.0"\nedition="2024"\n[workspace]\n[lints.rust]\nunsafe_code="forbid"\n[dependencies]\n'
for name in ["types", "font", "render"]:
    path = (ROOT / f"crates/taiyou-{name}").as_posix()
    features = ', features=["native"]' if name != "types" else ""
    manifest += f'taiyou-{name}={{path="{path}"{features}}}\n'
(driver / "Cargo.toml").write_text(manifest)
shutil.copyfile(ROOT / "Cargo.lock", driver / "Cargo.lock")
def run(*args):
    subprocess.run(list(map(str, args)), env=env, check=True, cwd=ROOT)
# Reconcile the new root package only; reject any dependency not already locked.
run("cargo", "generate-lockfile", "--offline", "--manifest-path", driver / "Cargo.toml")
accepted = tomllib.loads((ROOT / "Cargo.lock").read_text())["package"]
resolved = tomllib.loads((driver / "Cargo.lock").read_text())["package"]
for package in resolved:
    if package["name"] != "r002-coverage-experiment":
        assert package in accepted, f"experiment changed locked dependency: {package}"
shutil.copyfile(driver / "Cargo.lock", OUT / "experiment.Cargo.lock")
# Formatting is runner-only and applies only to the temporary experiment copy.
run("rustfmt", "--edition", "2024", driver / "src/main.rs")
run("cargo", "build", "--release", "--locked", "--offline", "--manifest-path", driver / "Cargo.toml")
binary = driver / "target/release/r002-coverage-experiment"
for repeat in [1, 2]:
    run(binary, OUT / f"repeat-{repeat}")
rows = list(csv.DictReader((OUT / "repeat-1/samples.tsv").read_text().splitlines(), delimiter="\t"))
results = []
masks = {}
for row in rows:
    case = row["case"]
    a = (OUT / f"repeat-1/{case}.a8").read_bytes()
    b = (OUT / f"repeat-2/{case}.a8").read_bytes()
    assert a == b, f"independent-process nondeterminism: {case}"
    for key in ["x", "y", "width", "height", "stride", "mask_bytes"]:
        row[key] = int(row[key])
    second = next(r for r in csv.DictReader((OUT / "repeat-2/samples.tsv").read_text().splitlines(), delimiter="\t") if r["case"] == case)
    assert all(str(row[k]) == second[k] for k in ["x", "y", "width", "height", "stride"])
    samples = list(map(int, row.pop("ns_samples").split(",")))
    row.update(sha256=hashlib.sha256(a).hexdigest(), deterministic=True,
               ns_samples=samples, median_ns=statistics.median(samples),
               max_delta_repeat=0, mae_repeat=0, changed_pixels_repeat=0,
               xy_comparison=None, allocation_measurement="A8 retained bytes only; native scratch/RSS not measured")
    (OUT / f"{case}.pgm").write_bytes(f'P5\n{row["width"]} {row["height"]}\n255\n'.encode() + a)
    masks[case] = (row, a)
    results.append(row)
def sample(case, x, y):
    r, a = masks[case]
    u,v = x-r["x"], y-r["y"]
    return a[v*r["stride"]+u] if 0 <= u < r["width"] and 0 <= v < r["height"] else 0
grouping = []
for opacity in [255, 128]:
    combined, separate, delta = bytearray(), bytearray(), []
    mul = lambda a,b: (a*b+127)//255
    for y in range(8):
        for x in range(12):
            c = mul(sample("overlap-combined",x,y), opacity)
            a = mul(sample("overlap-a",x,y),opacity)
            b = mul(sample("overlap-b",x,y),opacity)
            s = b+mul(a,255-b)
            combined.append(c); separate.append(s); delta.append(abs(c-s))
    (OUT / f"grouped-alpha-{opacity}.a8").write_bytes(combined)
    (OUT / f"separate-alpha-{opacity}.a8").write_bytes(separate)
    (OUT / f"grouping-diff-{opacity}.pgm").write_bytes(b"P5\n12 8\n255\n"+bytes(delta))
    grouping.append(dict(opacity=opacity, max_delta=max(delta), mae=sum(delta)/len(delta),
                         changed_pixels=sum(d != 0 for d in delta), bounds_delta=[0,0,0,0],
                         evidence="Core C6 integer source-over over actual T012 masks; not xy"))
report = dict(schema=1, candidate=subprocess.check_output(["git","rev-parse","HEAD"],text=True).strip(),
              environment=dict(os=platform.platform(), cpu=platform.processor(), image=os.getenv("ImageVersion"),
                               rust=subprocess.check_output(["rustc","--version","--verbose"],text=True,env=env)),
              backend=(OUT / "repeat-1/backend.txt").read_text(), cases=results, grouping=grouping,
              alternative_backend_tested=False, xy_reference=None)
stroke = OUT / "stroke"
stroke.mkdir(exist_ok=True)
executable = OUT / "stroke-probe"
run("cc", "-std=c11", "-O2", "-Wall", "-Wextra", "-Werror", "-I"+str(PREFIX/"include/freetype2"),
    ROOT/"experiments/coverage/stroke.c", "-L"+str(PREFIX/"lib"), "-lfreetype", "-lm", "-o", executable)
for repeat in [1,2]:
    directory = stroke / str(repeat)
    directory.mkdir(exist_ok=True)
    log = subprocess.check_output([str(executable), str(directory)],text=True,env=env)
    (directory/"samples.txt").write_text(log)
    print(log,flush=True)
for a in (stroke/"1").glob("*.a8"):
    assert a.read_bytes() == (stroke/"2"/a.name).read_bytes(), "stroker repeat differs"
report["stroke"] = dict(backend="FreeType public stroker API probe; no Taiyou stroke implementation exists",
                       deterministic=True, cases=8, xy_comparison=None,
                       timing_scope="raster only, not widening; raw clock samples; no peak allocation measurement")
report["stroke"]["outputs"] = [
    dict(file=p.name,sha256=hashlib.sha256(p.read_bytes()).hexdigest(),mask_bytes=p.stat().st_size)
    for p in sorted((stroke/"1").glob("*.a8"))]
print("R002_STROKE_SUMMARY="+json.dumps(report["stroke"]),flush=True)
(OUT / "report.json").write_text(json.dumps(report,indent=2)+"\n")
for row in results:
    print("R002_A8="+json.dumps(row), flush=True)
print("R002_GROUPING="+json.dumps(grouping),flush=True)
