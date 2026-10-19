"""Linux CI only: instrument pinned FreeType/HarfBuzz and run actual font tests.

The C library is ASan/UBSan instrumented; this is not a Rust ASan build or fuzzing.
"""
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
temporary = Path(os.environ["RUNNER_TEMP"])
source = temporary / "taiyou-native-sources/f"
build = temporary / "taiyou-font-sanitizer-build"
prefix = temporary / "taiyou-font-sanitizer-install"

def run(*args, **kwargs):
    print("SANITIZER_RUN=" + json.dumps([str(arg) for arg in args]), flush=True)
    return subprocess.run([str(arg) for arg in args], cwd=root, check=True, **kwargs)

run("meson", "setup", build, source, "--prefix", prefix, "--libdir", "lib",
    "--default-library", "static", "--buildtype", "debugoptimized", "--wrap-mode", "nodownload",
    "-Dauto_features=disabled", "-Db_sanitize=address,undefined", "-Db_lundef=false",
    *["-D" + name + "=disabled" for name in ["brotli", "bzip2", "harfbuzz", "mmap", "png", "tests", "zlib"]])
run("meson", "compile", "-C", build, "-j", "2")
run("meson", "install", "-C", build)
hb_build = temporary / "taiyou-harfbuzz-sanitizer-build"
hb_flags = ["-D" + name + "=disabled" for name in ["glib", "gobject", "cairo", "chafa", "png", "zlib", "icu", "graphite", "graphite2", "freetype", "fontations", "gdi", "directwrite", "coretext", "harfrust", "kbts", "wasm", "raster", "vector", "gpu", "gpu_demo", "subset", "tests", "introspection", "docs", "utilities", "benchmark"]]
run("meson", "setup", hb_build, temporary / "taiyou-native-sources/h", "--prefix", prefix,
    "--libdir", "lib", "--default-library", "static", "--buildtype", "debugoptimized",
    "--wrap-mode", "nodownload", "-Dauto_features=disabled", "-Db_sanitize=address,undefined", "-Db_lundef=false", *hb_flags)
run("meson", "compile", "-C", hb_build, "-j", "2")
run("meson", "install", "-C", hb_build)
environment = dict(os.environ)
environment.update(PKG_CONFIG_LIBDIR=str(prefix / "lib/pkgconfig"),
                   PKG_CONFIG_PATH=str(prefix / "lib/pkgconfig"),
                   CARGO_TARGET_DIR=str(temporary / "taiyou-font-sanitizer-target"),
                   RUSTFLAGS="-C link-arg=-lasan -C link-arg=-lubsan")
actual = subprocess.check_output([environment["PKG_CONFIG"], "--variable=prefix", "freetype2"], env=environment, text=True).strip()
if Path(actual).resolve() != prefix.resolve():
    raise RuntimeError("sanitizer selected a different FreeType")
actual_hb = subprocess.check_output([environment["PKG_CONFIG"], "--variable=prefix", "harfbuzz"], env=environment, text=True).strip()
if Path(actual_hb).resolve() != prefix.resolve():
    raise RuntimeError("sanitizer selected a different HarfBuzz")
run("cc", "-std=c11", "-Werror", "-fsyntax-only", "-I" + str(prefix / "include/freetype2"), "-I" + str(prefix / "include/harfbuzz"), root / "build-support/font-abi.c")
result = run("cargo", "test", "--locked", "-p", "taiyou-font", "--features", "native",
             "--no-run", "--message-format=json", env=environment, stdout=subprocess.PIPE, text=True)
executables = []
artifacts = []
expected_targets = {("taiyou_font", "lib"), ("catalog", "test"),
                    ("shaping", "test"), ("authenticated_outline", "test")}
for line in result.stdout.splitlines():
    item = json.loads(line)
    if item.get("reason") == "compiler-artifact" and item.get("profile", {}).get("test") and item.get("executable"):
        target = item["target"]
        artifacts.append({"name": target["name"], "kind": target["kind"],
                          "manifest_path": item["manifest_path"], "executable": item["executable"]})
        executables.append(item["executable"])
print("FONT_SANITIZER_ARTIFACTS=" + json.dumps(artifacts), flush=True)
actual_targets = {(item["name"], item["kind"][0]) for item in artifacts if len(item["kind"]) == 1}
if (len(artifacts) != len(expected_targets) or actual_targets != expected_targets
        or any(Path(item["manifest_path"]).resolve() != (root / "crates/taiyou-font/Cargo.toml").resolve()
               or not Path(item["executable"]).resolve().is_relative_to(Path(environment["CARGO_TARGET_DIR"]).resolve())
               for item in artifacts)):
    raise RuntimeError("expected exact font unit, catalog, shaping and authenticated_outline executable targets; got " + json.dumps(artifacts))
environment["LD_PRELOAD"] = ":".join(subprocess.check_output(["cc", "-print-file-name=" + name], text=True).strip() for name in ["libasan.so", "libubsan.so"])
environment["ASAN_OPTIONS"] = "detect_leaks=1:halt_on_error=1"
environment["UBSAN_OPTIONS"] = "halt_on_error=1:print_stacktrace=1"
for executable in sorted(executables):
    run(executable, "--test-threads=1", env=environment)
print("FONT_NATIVE_ASAN_UBSAN_COMPLETE", flush=True)
