"""CI-only cold native recipes and binding audit. No system library fallback."""
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

root = Path(__file__).resolve().parent.parent
sources = Path(os.environ["RUNNER_TEMP"]) / "taiyou-native-sources"
prefix = Path(os.environ["RUNNER_TEMP"]) / "taiyou-native-install"
builds = sources / "b"
builds.mkdir(exist_ok=True)
if platform.system() == "Darwin":
    os.environ["MACOSX_DEPLOYMENT_TARGET"] = "15.0"

def run(*args, **kwargs):
    print("RUN=" + json.dumps([str(arg) for arg in args]), flush=True)
    return subprocess.run([str(arg) for arg in args], cwd=root, check=True, **kwargs)

run(sys.executable, "build-support/shaping-fixtures.py", "--self-test")
run(sys.executable, "build-support/shaping-fixtures.py")

if platform.system() == "Darwin" and platform.machine() != "arm64":
    raise RuntimeError("macOS acceptance requires ARM64")
if os.name == "nt":
    vswhere = Path(os.environ["ProgramFiles(x86)"]) / "Microsoft Visual Studio/Installer/vswhere.exe"
    installation = subprocess.check_output([str(vswhere), "-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"], text=True).strip()
    command = f'call "{installation}\\Common7\\Tools\\VsDevCmd.bat" -no_logo -arch=x64 >nul && set'
    # Windows list2cmdline escapes inner quotes for C argv, not cmd builtins.
    # Use the shell's own /c quoting for this trusted vswhere installation path.
    environment = subprocess.check_output(command, shell=True, executable=os.environ["COMSPEC"], encoding="utf-8", errors="replace")
    for line in environment.splitlines():
        key, sep, value = line.partition("=")
        if sep and key.upper() in {"PATH", "INCLUDE", "LIB", "LIBPATH", "VCTOOLSINSTALLDIR", "WINDOWSSDKDIR", "WINDOWSSDKVERSION", "UNIVERSALCRTSDKDIR"}:
            os.environ[key] = value
    os.environ["CC"] = "cl"
    os.environ["CXX"] = "cl"

print("ENVIRONMENT=" + json.dumps({"os":platform.platform(),"arch":platform.machine(),"image":os.environ.get("ImageVersion"),"python":platform.python_version()}), flush=True)
run("meson", "--version")
run("ninja", "--version")
pkgconf = shutil.which("pkgconf")
if not pkgconf:
    raise RuntimeError("pinned pkgconf executable unavailable")
run(pkgconf, "--version")
flags = {
    "freetype": ["-Dbrotli=disabled", "-Dbzip2=disabled", "-Dharfbuzz=disabled", "-Dmmap=disabled", "-Dpng=disabled", "-Dtests=disabled", "-Dzlib=disabled"],
    "harfbuzz": ["-D" + name + "=disabled" for name in ["glib", "gobject", "cairo", "chafa", "png", "zlib", "icu", "graphite", "graphite2", "freetype", "fontations", "gdi", "directwrite", "coretext", "harfrust", "kbts", "wasm", "raster", "vector", "gpu", "gpu_demo", "subset", "tests", "introspection", "docs", "utilities", "benchmark"]],
}
os.environ["PKG_CONFIG"] = pkgconf
os.environ["PKG_CONFIG_LIBDIR"] = str(prefix / "lib/pkgconfig")
os.environ["PKG_CONFIG_PATH"] = str(prefix / "lib/pkgconfig")
os.environ["PKG_CONFIG_ALL_STATIC"] = "1"
os.environ["PKG_CONFIG_ALLOW_SYSTEM_LIBS"] = "1"
# freetype-sys unconditionally depends on libz-sys even with FreeType's zlib
# support disabled. Force its checksummed vendored source, never host zlib.
os.environ["LIBZ_SYS_STATIC"] = "1"
os.environ.pop("PKG_CONFIG_SYSROOT_DIR", None)
for name in ["freetype", "harfbuzz"]:
    source = next(path for path in (sources / name).iterdir() if path.is_dir())
    # HarfBuzz's generated source-list command can exceed Windows CreateProcess
    # limits with a commit-length archive directory repeated for every source.
    # Keep unchanged verified sources at short paths next to the build directory.
    short = sources / {"freetype": "f", "harfbuzz": "h"}[name]
    source.resolve().relative_to(sources.resolve())
    short.resolve().relative_to(sources.resolve())
    source.rename(short)
    source = short
    directory = builds / name
    common = ["--prefix", str(prefix), "--libdir", "lib", "--default-library", "static", "--buildtype", "release", "--wrap-mode", "nodownload", "-Dauto_features=disabled"]
    if os.name == "nt": common.append("-Db_vscrt=md")
    run("meson", "setup", directory, source, *common, *flags[name])
    run("meson", "compile", "-C", directory, "-j", "2")
    run("meson", "install", "-C", directory)
    options = json.loads(subprocess.check_output(["meson", "introspect", str(directory), "--buildoptions"], text=True))
    print("BUILD_OPTIONS=" + json.dumps({"library":name,"options":[v for v in options if v["type"] in {"feature", "boolean", "combo"}]}), flush=True)
for name in ["freetype2", "harfbuzz"]:
    run(pkgconf, "--modversion", name)
    actual = subprocess.check_output([pkgconf,"--variable=prefix",name], text=True).strip()
    if Path(actual).resolve() != prefix.resolve(): raise RuntimeError("pkgconf selected a host library")

os.environ["RUSTUP_TOOLCHAIN"] = "1.99.1"
os.environ["TAIYOU_TEST_FONT"] = str(sources / "font.otf")
run("python", root / "build-support/font-fixtures.py")
os.environ["TAIYOU_TEST_BOLD"] = str(sources / "bold.otf")
os.environ["TAIYOU_TEST_COLLECTION"] = str(sources / "collection.otc")
native_features = ["--features", "taiyou-cli/native"]
reference_args = ["--discover"] if os.environ.get("TAIYOU_DISCOVER_SHAPING") == "1" else []
run(sys.executable, root / "build-support/shaping-reference.py", *reference_args)
os.environ["TAIYOU_SHAPING_REFERENCES"] = str(Path(os.environ["RUNNER_TEMP"]) / "taiyou-shaping-reference.tsv")
os.environ["TAIYOU_RENDER_SAMPLE_DIR"] = str(Path(os.environ["RUNNER_TEMP"]) / "taiyou-render-sample")
if platform.system() == "Linux":
    render_reference = Path(os.environ["RUNNER_TEMP"]) / "taiyou-renderer-reference"
    run("cc", "-std=c11", "-Werror", "-fsyntax-only", "-I" + str(prefix / "include/freetype2"), "-I" + str(prefix / "include/harfbuzz"), root / "build-support/font-abi.c")
    render_expectations = Path(os.environ["RUNNER_TEMP"]) / "taiyou-renderer-expectations"
    render_expectations.mkdir(exist_ok=True)
    run("cc", "-std=c11", "-Wall", "-Wextra", "-Werror", "-I" + str(prefix / "include/freetype2"), root / "build-support/renderer-reference.c", "-L" + str(prefix / "lib"), "-lfreetype", "-lm", "-o", render_reference)
    run(render_reference, Path(os.environ["RUNNER_TEMP"]) / "taiyou-shaping-fonts/latin.otf", render_expectations)
    os.environ["TAIYOU_RENDER_REFERENCE_DIR"] = str(render_expectations)
run(sys.executable, root / "build-support/first-render.py")
renderer_tests = subprocess.run(["cargo", "test", "--locked", "-p", "taiyou-cli", "--features", "native", "--test", "renderer_slice"], cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, encoding="utf-8")
(Path(os.environ["TAIYOU_RENDER_SAMPLE_DIR"]) / "renderer-tests.txt").write_text(renderer_tests.stdout, encoding="utf-8")
print(renderer_tests.stdout, flush=True)
run(sys.executable, root / "build-support/first-render.py")
renderer_tests.check_returncode()
if os.environ.get("TAIYOU_FOCUSED_RENDERER") == "1":
    run("cargo", "clippy", "--locked", "--workspace", "--all-targets", *native_features, "--", "-D", "warnings")
    print("FOCUSED_T014_COMPLETE; full existing matrix remains required", flush=True)
    sys.exit(0)
os.environ["TAIYOU_RASTER_SAMPLE_DIR"] = str(Path(os.environ["RUNNER_TEMP"]) / "taiyou-raster-sample")
if platform.system() == "Linux":
    run("cc", "-std=c11", "-Werror", "-fsyntax-only", "-I" + str(prefix / "include/freetype2"), "-I" + str(prefix / "include/harfbuzz"), root / "build-support/font-abi.c")
    reference = Path(os.environ["RUNNER_TEMP"]) / "taiyou-raster-reference"
    run("cc", "-std=c11", "-Wall", "-Wextra", "-Werror", "-I" + str(prefix / "include/freetype2"), root / "build-support/raster-reference.c", "-L" + str(prefix / "lib"), "-lfreetype", "-lm", "-o", reference)
    os.environ["TAIYOU_RASTER_REFERENCE"] = str(reference)
    run(reference, "--rects")
run("cargo", "test", "--locked", "-p", "taiyou-render", "--features", "native", "--test", "raster")
run("cargo", "test", "--locked", "-p", "taiyou-font", "--features", "native", "raster_native")
try:
    run("cargo", "test", "--locked", "-p", "taiyou-cli", "--features", "native", "--test", "raster")
finally:
    if (Path(os.environ["TAIYOU_RASTER_SAMPLE_DIR"]) / "sample-info.txt").exists():
        run(sys.executable, root / "build-support/raster-sample.py")
if os.environ.get("TAIYOU_FOCUSED_RASTER") == "1":
    run("cargo", "clippy", "--locked", "--workspace", "--all-targets", *native_features, "--", "-D", "warnings")
    print("FOCUSED_T012_COMPLETE; full existing matrix remains required", flush=True)
    sys.exit(0)
run("cargo", "test", "--locked", "-p", "taiyou-render", "--features", "native", "--test", "geometry_numeric")
run("cargo", "test", "--locked", "-p", "taiyou-font", "--features", "native", "geometry_native")
run("cargo", "test", "--locked", "-p", "taiyou-cli", "--features", "native", "--test", "geometry")
if os.environ.get("TAIYOU_FOCUSED_GEOMETRY") == "1":
    run("cargo", "clippy", "--locked", "--workspace", "--all-targets", *native_features, "--", "-D", "warnings")
    print("FOCUSED_T011_COMPLETE; full existing matrix remains required", flush=True)
    sys.exit(0)
run("cargo", "test", "--locked", "-p", "taiyou-render", "--features", "native", "--test", "layout_numeric", "--test", "layout_fixed_font")
run("cargo", "test", "--locked", "-p", "taiyou-cli", "--features", "native", "--test", "layout_pipeline")
if os.environ.get("TAIYOU_FOCUSED_LAYOUT") == "1":
    run("cargo", "clippy", "--locked", "--workspace", "--all-targets", *native_features, "--", "-D", "warnings")
    print("FOCUSED_T010_COMPLETE; full existing matrix remains required", flush=True)
    sys.exit(0)
run("cargo", "test", "--locked", "-p", "taiyou-font", "--features", "native", "--test", "shaping")
run("cargo", "test", "--locked", "-p", "taiyou-cli", "--features", "native", "--test", "shaping")
if os.environ.get("TAIYOU_FOCUSED_SHAPING") == "1":
    run("cargo", "clippy", "--locked", "--workspace", "--all-targets", *native_features, "--", "-D", "warnings")
    print("FOCUSED_T009_COMPLETE; full matrix/workspace verification remains required", flush=True)
    sys.exit(0)
run("rustc", "--version", "--verbose")
for manifest in [root / "Cargo.toml", root / "build-support/dependency-smoke/Cargo.toml"]:
    lock = manifest.parent / "Cargo.lock"
    if not lock.exists():
        run("cargo", "generate-lockfile", "--manifest-path", manifest)
    print("LOCK_BEGIN=" + str(manifest.relative_to(root)), flush=True)
    print(lock.read_text(encoding="utf-8"), flush=True)
    print("LOCK_END", flush=True)
run("cargo", "build", "--locked", "--workspace", *native_features)
run("cargo", "fmt", "--all", "--", "--check")
run("cargo", "clippy", "--locked", "--workspace", "--all-targets", *native_features, "--", "-D", "warnings")
run("cargo", "test", "--locked", "-p", "taiyou-font", "--features", "native", "catalog_")
run("cargo", "test", "--locked", "--workspace", *native_features)
run("pwsh", "-NoProfile", "-File", "scripts/check-workspace.ps1", "-SelfTest")
manifest = root / "build-support/dependency-smoke/Cargo.toml"
run("cargo", "fmt", "--manifest-path", manifest, "--", "--check")
run("cargo", "tree", "--locked", "--manifest-path", manifest)
run("cargo", "clippy", "--locked", "--manifest-path", manifest, "--all-targets", "--", "-D", "warnings")
run("cargo", "run", "--locked", "--manifest-path", manifest, "--", sources / "font.otf")
metadata = json.loads(subprocess.check_output(["cargo","metadata","--locked","--manifest-path",str(manifest),"--format-version","1"], text=True))
print("DEPENDENCY_LICENSES=" + json.dumps([{k:p.get(k) for k in ["name","version","license","repository","rust_version"]} for p in metadata["packages"]]), flush=True)
zlib = next(package for package in metadata["packages"] if package["name"] == "libz-sys")
for header in Path(zlib["manifest_path"]).parent.rglob("zlib.h"):
    versions = [line.strip() for line in header.read_text(encoding="utf-8").splitlines() if line.startswith("#define ZLIB_VERSION")]
    print("BUNDLED_ZLIB=" + json.dumps({"crate": zlib["version"], "header": str(header), "versions": versions}), flush=True)
if platform.system() == "Linux":
    run("python", root / "build-support/audit.py")
    run("rustup", "toolchain", "install", "1.85.0", "--profile", "minimal")
    minimum = dict(os.environ, RUSTUP_TOOLCHAIN="1.85.0")
    run("cargo", "test", "--locked", "--workspace", *native_features, env=minimum)
    run("cargo", "run", "--locked", "--manifest-path", manifest, "--", sources / "font.otf", env=minimum)
    run("python", root / "build-support/font-sanitizer.py")
run(sys.executable, root / "build-support/raster-sample.py")
run(sys.executable, root / "build-support/first-render.py")
print("NATIVE_RECIPE_COMPLETE", flush=True)
