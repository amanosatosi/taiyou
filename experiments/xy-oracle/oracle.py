"""Windows-only xy-VSFilter 3.2.0.810 pixel oracle.

This harness loads the official prebuilt filter DLL through its exported CSRI C API.
It renders into caller-owned BGRX buffers on controlled backgrounds, proves
byte-for-byte determinism, reconstructs an effective premultiplied RGBA frame from
black/white captures, and verifies that reconstruction against an independent
colored background before comparing it with the frozen libass-tests PNG.

The raw BGRX captures are the primary xy evidence. Reconstructed RGBA is accepted
only when the colored-background round-trip is exact.
"""
from __future__ import annotations

import argparse
import ctypes
import hashlib
import json
import math
import os
import re
from pathlib import Path
import subprocess
import sys
from typing import Iterable

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "validation"))
import images  # noqa: E402

CSRI_F_BGR_ = 0x102
FR_PRIVATE = 0x10
REQUIRED_EXPORTS = (
    "csri_renderer_default",
    "csri_renderer_info",
    "csri_open_file",
    "csri_request_fmt",
    "csri_render",
    "csri_close",
)
VALIDATION_RGB = (17, 93, 201)
GOOD_FRACTION = 0.001  # 0.1% of frame pixels; exact matches are always separate.
GOOD_MIN_PIXELS = 100


class CsriFmt(ctypes.Structure):
    _fields_ = [
        ("pixfmt", ctypes.c_int),
        ("width", ctypes.c_uint),
        ("height", ctypes.c_uint),
    ]


class CsriFrame(ctypes.Structure):
    _fields_ = [
        ("pixfmt", ctypes.c_int),
        ("planes", ctypes.c_void_p * 4),
        ("strides", ctypes.c_ssize_t * 4),
    ]


class CsriInfo(ctypes.Structure):
    _fields_ = [
        ("name", ctypes.c_char_p),
        ("specific", ctypes.c_char_p),
        ("longname", ctypes.c_char_p),
        ("author", ctypes.c_char_p),
        ("copyright", ctypes.c_char_p),
    ]


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()


def rel(root: Path, path: Path) -> str:
    return path.resolve().relative_to(root.resolve()).as_posix()


def decode(value: bytes | None) -> str | None:
    return value.decode("utf-8", errors="replace") if value else None


def dll_candidates(root: Path) -> list[Path]:
    candidates = [p for p in root.rglob("*.dll") if p.is_file()]

    def score(path: Path) -> tuple[int, int, str]:
        text = path.as_posix().lower()
        name = path.name.lower()
        return (
            0 if "vsfilter" in name else 1,
            0 if ("x64" in text or "64" in name) else 1,
            text,
        )

    return sorted(candidates, key=score)


def add_dll_directories(root: Path):
    handles = []
    if not hasattr(os, "add_dll_directory"):
        return handles
    for directory in sorted({str(p.parent.resolve()) for p in root.rglob("*.dll")}):
        try:
            handles.append(os.add_dll_directory(directory))
        except OSError:
            pass
    return handles


def configure_csri(dll: ctypes.CDLL) -> None:
    dll.csri_renderer_default.argtypes = []
    dll.csri_renderer_default.restype = ctypes.c_void_p

    dll.csri_renderer_info.argtypes = [ctypes.c_void_p]
    dll.csri_renderer_info.restype = ctypes.POINTER(CsriInfo)

    dll.csri_open_file.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_void_p]
    dll.csri_open_file.restype = ctypes.c_void_p

    dll.csri_request_fmt.argtypes = [ctypes.c_void_p, ctypes.POINTER(CsriFmt)]
    dll.csri_request_fmt.restype = ctypes.c_int

    dll.csri_render.argtypes = [ctypes.c_void_p, ctypes.POINTER(CsriFrame), ctypes.c_double]
    dll.csri_render.restype = None

    dll.csri_close.argtypes = [ctypes.c_void_p]
    dll.csri_close.restype = None


def load_csri(dll_path: Path, release_root: Path) -> tuple[ctypes.CDLL, list]:
    handles = add_dll_directories(release_root)
    dll = ctypes.CDLL(str(dll_path))
    missing = [name for name in REQUIRED_EXPORTS if not hasattr(dll, name)]
    if missing:
        for handle in handles:
            handle.close()
        raise RuntimeError("missing CSRI exports: " + ", ".join(missing))
    configure_csri(dll)
    return dll, handles


def probe_dll(args) -> int:
    dll_path = Path(args.dll).resolve()
    release_root = Path(args.release_root).resolve()
    output = Path(args.output)
    try:
        dll, handles = load_csri(dll_path, release_root)
        renderer = dll.csri_renderer_default()
        if not renderer:
            raise RuntimeError("csri_renderer_default returned NULL")
        info_ptr = dll.csri_renderer_info(renderer)
        if not info_ptr:
            raise RuntimeError("csri_renderer_info returned NULL")
        info = info_ptr.contents
        record = {
            "path": rel(release_root, dll_path),
            "sha256": sha256_file(dll_path),
            "renderer": {
                "name": decode(info.name),
                "specific": decode(info.specific),
                "longname": decode(info.longname),
                "author": decode(info.author),
                "copyright": decode(info.copyright),
            },
        }
        output.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
        del dll
        for handle in handles:
            handle.close()
        return 0
    except Exception as error:
        output.write_text(
            json.dumps({"error": f"{type(error).__name__}: {error}"}, indent=2) + "\n",
            encoding="utf-8",
        )
        return 2


class PrivateFonts:
    def __init__(self, fonts: Iterable[Path]):
        self.fonts = list(fonts)
        self.loaded: list[Path] = []
        self.gdi = ctypes.WinDLL("gdi32", use_last_error=True)
        self.gdi.AddFontResourceExW.argtypes = [ctypes.c_wchar_p, ctypes.c_uint, ctypes.c_void_p]
        self.gdi.AddFontResourceExW.restype = ctypes.c_int
        self.gdi.RemoveFontResourceExW.argtypes = [ctypes.c_wchar_p, ctypes.c_uint, ctypes.c_void_p]
        self.gdi.RemoveFontResourceExW.restype = ctypes.c_bool

    def __enter__(self):
        for font in self.fonts:
            count = self.gdi.AddFontResourceExW(str(font), FR_PRIVATE, None)
            if count <= 0:
                raise OSError(ctypes.get_last_error(), f"AddFontResourceExW failed: {font}")
            self.loaded.append(font)
        return self

    def __exit__(self, exc_type, exc, tb):
        for font in reversed(self.loaded):
            self.gdi.RemoveFontResourceExW(str(font), FR_PRIVATE, None)
        self.loaded.clear()


def background_bgrx(width: int, height: int, rgb: tuple[int, int, int]) -> bytes:
    r, g, b = rgb
    return bytes((b, g, r, 0)) * (width * height)


def render_capture(
    dll_path: Path,
    release_root: Path,
    source: Path,
    fonts: list[Path],
    width: int,
    height: int,
    timestamp_ms: int,
    rgb: tuple[int, int, int],
) -> bytes:
    dll, handles = load_csri(dll_path, release_root)
    try:
        renderer = dll.csri_renderer_default()
        if not renderer:
            raise RuntimeError("csri_renderer_default returned NULL")
        with PrivateFonts(fonts):
            instance = dll.csri_open_file(renderer, str(source).encode("utf-8"), None)
            if not instance:
                raise RuntimeError("csri_open_file returned NULL")
            try:
                fmt = CsriFmt(CSRI_F_BGR_, width, height)
                result = dll.csri_request_fmt(instance, ctypes.byref(fmt))
                if result != 0:
                    raise RuntimeError(f"csri_request_fmt failed: {result}")

                size = width * height * 4
                storage = (ctypes.c_ubyte * (size + 31))()
                address = (ctypes.addressof(storage) + 15) & ~15
                seed = background_bgrx(width, height, rgb)
                ctypes.memmove(address, seed, size)

                planes = (ctypes.c_void_p * 4)()
                planes[0] = address
                strides = (ctypes.c_ssize_t * 4)()
                strides[0] = width * 4
                frame = CsriFrame(CSRI_F_BGR_, planes, strides)

                dll.csri_render(instance, ctypes.byref(frame), timestamp_ms / 1000.0)
                return ctypes.string_at(address, size)
            finally:
                dll.csri_close(instance)
    finally:
        del dll
        for handle in handles:
            handle.close()


def worker_capture(args) -> int:
    try:
        rgb = tuple(int(part) for part in args.background.split(","))
        if len(rgb) != 3 or any(not 0 <= value <= 255 for value in rgb):
            raise ValueError("background must be R,G,B bytes")
        data = render_capture(
            Path(args.dll).resolve(),
            Path(args.release_root).resolve(),
            Path(args.source).resolve(),
            [Path(path).resolve() for path in args.font],
            args.width,
            args.height,
            args.timestamp_ms,
            rgb,
        )
        Path(args.output).write_bytes(data)
        return 0
    except Exception as error:
        print(f"{type(error).__name__}: {error}", file=sys.stderr, flush=True)
        return 2


def run_worker(command: list[str], timeout: int = 45) -> tuple[int, str]:
    try:
        completed = subprocess.run(
            command,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=timeout,
        )
        return completed.returncode, completed.stdout[-65536:]
    except subprocess.TimeoutExpired as error:
        text = error.stdout or ""
        if isinstance(text, bytes):
            text = text.decode("utf-8", errors="replace")
        return -999, text[-65536:] + "\nworker timeout"


def find_csri_dll(release_root: Path, output: Path) -> tuple[Path, dict, list[dict]]:
    attempts = []
    probe_dir = output / "probe"
    probe_dir.mkdir(parents=True, exist_ok=True)
    for index, candidate in enumerate(dll_candidates(release_root)):
        probe_out = probe_dir / f"{index:02d}.json"
        code, log = run_worker(
            [
                sys.executable,
                str(Path(__file__).resolve()),
                "--probe-dll",
                "--dll",
                str(candidate),
                "--release-root",
                str(release_root),
                "--output",
                str(probe_out),
            ],
            timeout=20,
        )
        record = {
            "path": rel(release_root, candidate),
            "exit_code": code,
            "log": log,
        }
        if probe_out.is_file():
            try:
                record["probe"] = json.loads(probe_out.read_text(encoding="utf-8-sig"))
            except json.JSONDecodeError:
                record["probe"] = {"error": "invalid probe JSON"}
        attempts.append(record)
        if code == 0 and record.get("probe", {}).get("renderer", {}).get("name"):
            return candidate, record["probe"], attempts
    raise RuntimeError("no released DLL exposed the required CSRI API")


def validate_case_assets(
    case: dict, corpus: Path
) -> tuple[Path, Path | None, list[Path], list[dict]]:
    source = (
        Path(case["source_external"]).resolve()
        if case.get("source_external")
        else (corpus / case["source"]).resolve()
    )
    reference = (
        None
        if not case.get("reference")
        else (corpus / case["reference"]).resolve()
    )
    if not source.is_file():
        raise FileNotFoundError(f"missing source: {source}")
    if reference is not None and not reference.is_file():
        raise FileNotFoundError(f"missing reference: {reference}")
    fonts = []
    font_records = []
    for spec in case.get("fonts", []):
        font = (
            Path(spec["external_path"]).resolve()
            if spec.get("external_path")
            else (corpus / spec["path"]).resolve()
        )
        if not font.is_file():
            raise FileNotFoundError(f"missing font: {font}")
        actual = sha256_file(font)
        expected = spec.get("sha256")
        if expected and actual.lower() != expected.lower():
            raise ValueError(f"font SHA-256 mismatch: {font}")
        fonts.append(font)
        font_records.append(
            {
                "path": spec.get("path"),
                "external_path": spec.get("external_path"),
                "family": spec.get("family"),
                "sha256": actual,
                "expected_sha256": expected,
            }
        )
    return source, reference, fonts, font_records



def artifact_key(case_id: str) -> str:
    safe = re.sub(r"[^A-Za-z0-9._-]+", "-", case_id).strip("-")[:80] or "case"
    return safe + "-" + hashlib.sha256(case_id.encode("utf-8")).hexdigest()[:10]


def classify_xy(metrics: dict | None, width: int, height: int) -> tuple[str | None, int | None]:
    if metrics is None or metrics.get("changed_pixels") is None:
        return None, None
    changed = metrics["changed_pixels"]
    good_limit = max(GOOD_MIN_PIXELS, math.ceil(width * height * GOOD_FRACTION))
    if changed == 0:
        return "same", good_limit
    if changed <= good_limit:
        return "good", good_limit
    return "bad", good_limit


def inventory_cases(corpus_commit: str) -> tuple[list[dict], list[dict]]:
    inventory = json.loads((ROOT / "validation" / "inventory.json").read_text(encoding="utf-8"))
    if inventory.get("commit") != corpus_commit:
        raise ValueError("xy oracle inventory commit differs from acquired corpus")
    visual = []
    nonvisual = []
    for item in inventory["cases"]:
        if item["kind"] != "regression":
            nonvisual.append({
                "id": item["id"],
                "kind": item["kind"],
                "reason": "no reference image; retained as non-image inventory coverage",
            })
            continue
        if len(item.get("frames", [])) != 1:
            raise ValueError(f"unexpected frame count for {item['id']}")
        frame = item["frames"][0]
        width, height = frame["reference_size"]
        visual.append({
            "id": item["id"],
            "source": item["source"],
            "reference": frame["expected"],
            "timestamp_ms": frame["timestamp_ms"],
            "width": width,
            "height": height,
            "fonts": [{"path": path} for path in item.get("fonts", [])],
            "embedded_fonts": not item.get("fonts"),
            "scale": item.get("scale", "1"),
            "inventory": True,
        })
    return visual, nonvisual


def mua_office_case() -> dict:
    font = os.environ.get("MUA_OFFICE_FONT")
    if not font:
        raise ValueError("MUA_OFFICE_FONT is not set")
    return {
        "id": "witness/mua-office-myanmar-center",
        "source": "experiments/xy-oracle/fixtures/mua-office-myanmar.ass",
        "source_external": str(ROOT / "experiments" / "xy-oracle" / "fixtures" / "mua-office-myanmar.ass"),
        "reference": None,
        "timestamp_ms": 1000,
        "width": 1920,
        "height": 1080,
        "fonts": [{"external_path": font, "family": "MUA Office"}],
        "embedded_fonts": False,
        "scale": "1",
        "inventory": False,
        "provenance": "https://myanmarmiunicode.blogspot.com/2017/06/mua-office-font-adobe-supported.html",
    }


def capture(
    dll: Path,
    release_root: Path,
    source: Path,
    fonts: list[Path],
    case: dict,
    rgb: tuple[int, int, int],
    output: Path,
) -> tuple[int, str]:
    command = [
        sys.executable,
        str(Path(__file__).resolve()),
        "--worker-capture",
        "--dll",
        str(dll),
        "--release-root",
        str(release_root),
        "--source",
        str(source),
        "--width",
        str(case["width"]),
        "--height",
        str(case["height"]),
        "--timestamp-ms",
        str(case["timestamp_ms"]),
        "--background",
        ",".join(str(value) for value in rgb),
        "--output",
        str(output),
    ]
    for font in fonts:
        command.extend(["--font", str(font)])
    return run_worker(command)


def reconstruct_rgba(
    black: bytes,
    white: bytes,
    validation: bytes,
    validation_rgb: tuple[int, int, int],
) -> tuple[bytes | None, dict]:
    if len(black) != len(white) or len(black) != len(validation) or len(black) % 4:
        raise ValueError("capture buffer length mismatch")
    rgba = bytearray(len(black))
    channel_delta_mismatches = 0
    negative_deltas = 0
    premul_violations = 0
    validation_mismatches = 0
    x_byte_changes = 0
    max_validation_delta = 0
    vr, vg, vb = validation_rgb
    bg_bgr = (vb, vg, vr)

    for offset in range(0, len(black), 4):
        b0, g0, r0, x0 = black[offset : offset + 4]
        bw, gw, rw, xw = white[offset : offset + 4]
        bv, gv, rv, xv = validation[offset : offset + 4]
        deltas = (bw - b0, gw - g0, rw - r0)
        if any(delta < 0 for delta in deltas):
            negative_deltas += 1
            continue
        if deltas[0] != deltas[1] or deltas[0] != deltas[2]:
            channel_delta_mismatches += 1
            continue
        inverted_alpha = deltas[0]
        alpha = 255 - inverted_alpha
        if b0 > alpha or g0 > alpha or r0 > alpha:
            premul_violations += 1

        observed_bgr = (bv, gv, rv)
        source_bgr = (b0, g0, r0)
        for channel in range(3):
            expected = (
                (bg_bgr[channel] * (inverted_alpha + 1)) >> 8
            ) + source_bgr[channel]
            delta = abs(expected - observed_bgr[channel])
            if delta:
                validation_mismatches += 1
                max_validation_delta = max(max_validation_delta, delta)

        if x0 or xw or xv:
            x_byte_changes += 1
        rgba[offset : offset + 4] = bytes((r0, g0, b0, alpha))

    exact = (
        negative_deltas == 0
        and channel_delta_mismatches == 0
        and premul_violations == 0
        and validation_mismatches == 0
    )
    diagnostics = {
        "model": "xy AlphaBltPack: out=((background*(inverted_alpha+1))>>8)+premultiplied_source",
        "black_white_channel_delta_mismatches": channel_delta_mismatches,
        "negative_black_white_deltas": negative_deltas,
        "premultiplied_channel_gt_alpha": premul_violations,
        "validation_background_rgb": list(validation_rgb),
        "validation_channel_mismatches": validation_mismatches,
        "max_validation_channel_delta": max_validation_delta,
        "nonzero_unused_x_bytes": x_byte_changes,
        "exact": exact,
    }
    return (bytes(rgba) if exact else None), diagnostics


def preview_png(path: Path, width: int, height: int, premul_rgba: bytes) -> None:
    straight = bytearray(len(premul_rgba))
    for offset in range(0, len(premul_rgba), 4):
        r, g, b, a = premul_rgba[offset : offset + 4]
        if a:
            rr = min(255, (r * 255 + a // 2) // a)
            gg = min(255, (g * 255 + a // 2) // a)
            bb = min(255, (b * 255 + a // 2) // a)
        else:
            rr = gg = bb = 0
        straight[offset : offset + 4] = bytes((rr, gg, bb, a))
    images.write_png(path, width, height, bytes(straight))


def compare_reference(
    reference: Path, raw_rgba: Path, width: int, height: int, diff: Path
) -> dict:
    expected = images.read_png(reference)
    if expected[:2] != (width, height):
        raise ValueError(
            f"reference dimensions {expected[:2]} do not match manifest {(width, height)}"
        )
    actual = images.raw_rgba(raw_rgba, width, height)
    return images.compare(expected, actual, diff)


def render_taiyou(
    binary: Path,
    source: Path,
    fonts: list[Path],
    case: dict,
    png: Path,
    raw: Path,
) -> tuple[int, str]:
    command = [
        str(binary),
        "render",
        "--input",
        str(source),
        "--time-us",
        str(case["timestamp_ms"] * 1000),
        "--width",
        str(case["width"]),
        "--height",
        str(case["height"]),
        "--profile",
        "core-v1",
        "--output",
        str(png),
        "--raw",
        str(raw),
    ]
    for font in fonts:
        command.extend(["--font", str(font)])
    return run_worker(command, timeout=90)


def compare_raw(
    expected_raw: Path,
    actual_raw: Path,
    width: int,
    height: int,
    diff: Path,
) -> dict:
    expected = images.raw_rgba(expected_raw, width, height)
    actual = images.raw_rgba(actual_raw, width, height)
    return images.compare(expected, actual, diff)


def run_case(
    case: dict,
    corpus: Path,
    dll: Path,
    release_root: Path,
    taiyou: Path,
    output: Path,
) -> dict:
    record = {
        "id": case["id"],
        "source": case["source"],
        "reference": case["reference"],
        "timestamp_ms": case["timestamp_ms"],
        "dimensions": [case["width"], case["height"]],
        "scale_metadata": case.get("scale", "1"),
        "status": "ERROR",
    }
    case_dir = output / "cases" / artifact_key(case["id"])
    case_dir.mkdir(parents=True, exist_ok=False)
    if case.get("scale", "1") != "1":
        record["scale_note"] = (
            "Direct xy-vs-Taiyou oracle renders both engines at the final reference frame "
            "dimensions. The libass reference PNG for this case was generated with special "
            f"{case['scale']} comparison oversampling, so that separate libass-vs-xy metric "
            "has different sampling provenance."
        )
    try:
        source, reference, fonts, font_records = validate_case_assets(case, corpus)
        record["source_sha256"] = sha256_file(source)
        record["reference_sha256"] = sha256_file(reference) if reference else None
        record["fonts"] = font_records
        record["embedded_fonts"] = bool(case.get("embedded_fonts"))

        captures = [
            ("black", (0, 0, 0)),
            ("black-repeat", (0, 0, 0)),
            ("white", (255, 255, 255)),
            ("validation", VALIDATION_RGB),
        ]
        logs = {}
        paths = {}
        for name, rgb in captures:
            path = case_dir / f"{name}.bgrx"
            code, log = capture(dll, release_root, source, fonts, case, rgb, path)
            logs[name] = {"exit_code": code, "log": log}
            if code != 0:
                raise RuntimeError(
                    f"{name} capture failed with exit code {code}: {log[-2000:]}"
                )
            expected_size = case["width"] * case["height"] * 4
            if path.stat().st_size != expected_size:
                raise ValueError(f"{name} capture size mismatch")
            paths[name] = path

        black = paths["black"].read_bytes()
        black_repeat = paths["black-repeat"].read_bytes()
        deterministic = black == black_repeat
        record["determinism"] = {
            "exact": deterministic,
            "black_sha256": sha256_bytes(black),
            "black_repeat_sha256": sha256_bytes(black_repeat),
        }
        if not deterministic:
            raise ValueError("repeated fresh-process black captures differ")

        white = paths["white"].read_bytes()
        validation = paths["validation"].read_bytes()
        raw_rgba, reconstruction = reconstruct_rgba(
            black, white, validation, VALIDATION_RGB
        )
        record["reconstruction"] = reconstruction
        record["captures"] = {
            name: {
                "background_rgb": list(rgb),
                "sha256": sha256_file(paths[name]),
                "file": rel(output, paths[name]),
            }
            for name, rgb in captures
            if name != "black-repeat"
        }
        record["worker_logs"] = logs
        if raw_rgba is None:
            raise ValueError("black/white alpha reconstruction failed validation")

        raw_path = case_dir / "xy.rgba"
        png_path = case_dir / "xy.png"
        libass_diff_path = case_dir / "xy-vs-libass-diff.png"
        raw_path.write_bytes(raw_rgba)
        preview_png(png_path, case["width"], case["height"], raw_rgba)
        record["xy_raw_sha256"] = sha256_file(raw_path)
        record["xy_preview_sha256"] = sha256_file(png_path)
        if reference is not None:
            record["libass_reference_comparison"] = compare_reference(
                reference, raw_path, case["width"], case["height"], libass_diff_path
            )

        taiyou_png = case_dir / "taiyou.png"
        taiyou_raw = case_dir / "taiyou.rgba"
        taiyou_code, taiyou_log = render_taiyou(
            taiyou, source, fonts, case, taiyou_png, taiyou_raw
        )
        record["taiyou_render"] = {
            "exit_code": taiyou_code,
            "log": taiyou_log,
        }
        if taiyou_code != 0:
            raise RuntimeError(
                f"Taiyou render failed with exit code {taiyou_code}: {taiyou_log[-2000:]}"
            )
        expected_size = case["width"] * case["height"] * 4
        if taiyou_raw.stat().st_size != expected_size:
            raise ValueError("Taiyou raw frame size mismatch")
        decoded_taiyou = images.read_png(taiyou_png)
        if decoded_taiyou[:2] != (case["width"], case["height"]):
            raise ValueError("Taiyou PNG dimensions mismatch")

        taiyou_diff = case_dir / "diff.png"
        record["taiyou_vs_xy_comparison"] = compare_raw(
            raw_path,
            taiyou_raw,
            case["width"],
            case["height"],
            taiyou_diff,
        )
        record["taiyou_raw_sha256"] = sha256_file(taiyou_raw)
        record["taiyou_png_sha256"] = sha256_file(taiyou_png)
        classification, good_limit = classify_xy(
            record["taiyou_vs_xy_comparison"], case["width"], case["height"]
        )
        record["xy_classification"] = classification
        record["good_changed_pixel_limit"] = good_limit
        record["artifacts"] = {
            "xy_raw": rel(output, raw_path),
            "xy": rel(output, png_path),
            "taiyou_raw": rel(output, taiyou_raw),
            "taiyou": rel(output, taiyou_png),
            "diff": rel(output, taiyou_diff),
        }
        if reference is not None:
            record["artifacts"]["xy_vs_libass_diff"] = rel(output, libass_diff_path)
        record["status"] = "OK"
    except Exception as error:
        record["error"] = f"{type(error).__name__}: {error}"
    (case_dir / "case.json").write_text(
        json.dumps(record, indent=2) + "\n", encoding="utf-8"
    )
    return record


def main(args) -> int:
    if os.name != "nt":
        raise RuntimeError("xy-VSFilter oracle must run on Windows")
    release_root = Path(args.release_root).resolve()
    corpus = Path(args.corpus).resolve()
    taiyou = Path(args.taiyou).resolve()
    if not taiyou.is_file():
        raise FileNotFoundError(f"Taiyou renderer not found: {taiyou}")
    output = Path(args.output).resolve()
    output.mkdir(parents=True, exist_ok=True)

    corpus_commit = "a5073319af8117271d8ac13ef5c088197ff2d59b"
    visual_cases, nonvisual_cases = inventory_cases(corpus_commit)
    visual_cases.append(mua_office_case())

    report = {
        "schema": 1,
        "purpose": "supplemental authenticated runtime capture from official pinterf xy-VSFilter release",
        "candidate_sha": os.environ.get("CANDIDATE_SHA", os.environ.get("GITHUB_SHA")),
        "corpus_commit": corpus_commit,
        "classification_policy": {
            "same": "changed_pixels == 0",
            "good": f"0 < changed_pixels <= max({GOOD_MIN_PIXELS}, ceil(frame_pixels * {GOOD_FRACTION}))",
            "bad": "changed_pixels above the good threshold",
            "note": "good is diagnostic only and never counts as an exact compatibility match",
        },
        "inventory_visual_cases": len(visual_cases) - 1,
        "additional_witness_cases": 1,
        "nonvisual_inventory_cases": nonvisual_cases,
        "cases": [],
    }
    acquisition = output / "release-acquisition.json"
    if acquisition.is_file():
        report["release_acquisition"] = json.loads(
            acquisition.read_text(encoding="utf-8-sig")
        )

    try:
        dll, probe, attempts = find_csri_dll(release_root, output)
        report["dll"] = probe
        report["dll_probe_attempts"] = attempts
        for case in visual_cases:
            record = run_case(case, corpus, dll, release_root, taiyou, output)
            report["cases"].append(record)
            print(
                json.dumps(
                    {
                        "case": record["id"],
                        "status": record["status"],
                        "deterministic": record.get("determinism", {}).get("exact"),
                        "reconstruction": record.get("reconstruction", {}).get("exact"),
                        "taiyou_xy_exact": record.get(
                            "taiyou_vs_xy_comparison", {}
                        ).get("exact"),
                        "taiyou_xy_changed_pixels": record.get(
                            "taiyou_vs_xy_comparison", {}
                        ).get("changed_pixels"),
                        "classification": record.get("xy_classification"),
                        "libass_xy_exact": record.get(
                            "libass_reference_comparison", {}
                        ).get("exact"),
                        "libass_xy_changed_pixels": record.get(
                            "libass_reference_comparison", {}
                        ).get("changed_pixels"),
                    }
                ),
                flush=True,
            )
    except Exception as error:
        report["fatal_error"] = f"{type(error).__name__}: {error}"

    total = len(visual_cases)
    ok = sum(record.get("status") == "OK" for record in report["cases"])
    deterministic = sum(
        record.get("determinism", {}).get("exact") is True
        for record in report["cases"]
    )
    reconstructed = sum(
        record.get("reconstruction", {}).get("exact") is True
        for record in report["cases"]
    )
    taiyou_exact = sum(
        record.get("taiyou_vs_xy_comparison", {}).get("exact") is True
        for record in report["cases"]
    )
    libass_exact = sum(
        record.get("libass_reference_comparison", {}).get("exact") is True
        for record in report["cases"]
    )
    classes = {
        name: sum(record.get("xy_classification") == name for record in report["cases"])
        for name in ("same", "good", "bad")
    }
    unsupported = sum(record.get("status") == "UNSUPPORTED" for record in report["cases"])
    errors = sum(record.get("status") == "ERROR" for record in report["cases"])
    report["summary"] = {
        "total_visual_cases": total,
        "libass_inventory_visual_cases": len(visual_cases) - 1,
        "additional_witness_cases": 1,
        "captured_and_validated": ok,
        "deterministic": deterministic,
        "reconstruction_exact": reconstructed,
        "same": classes["same"],
        "good": classes["good"],
        "bad": classes["bad"],
        "unsupported": unsupported,
        "errors": errors,
        "taiyou_vs_xy_exact": taiyou_exact,
        "libass_vs_xy_exact": libass_exact,
        "nonvisual_inventory_cases": len(nonvisual_cases),
    }
    (output / "report.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    print("XY_ORACLE_SUMMARY=" + json.dumps(report["summary"]), flush=True)

    return 0 if "fatal_error" not in report and errors == 0 else 1


def parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser()
    modes = p.add_mutually_exclusive_group()
    modes.add_argument("--probe-dll", action="store_true")
    modes.add_argument("--worker-capture", action="store_true")
    p.add_argument("--release-root")
    p.add_argument("--corpus")
    p.add_argument("--taiyou")
    p.add_argument("--output", required=True)
    p.add_argument("--dll")
    p.add_argument("--source")
    p.add_argument("--font", action="append", default=[])
    p.add_argument("--width", type=int)
    p.add_argument("--height", type=int)
    p.add_argument("--timestamp-ms", type=int)
    p.add_argument("--background")
    return p


if __name__ == "__main__":
    args = parser().parse_args()
    try:
        if args.probe_dll:
            if not args.dll or not args.release_root:
                raise ValueError("--probe-dll requires --dll and --release-root")
            sys.exit(probe_dll(args))
        if args.worker_capture:
            required = (
                args.dll,
                args.release_root,
                args.source,
                args.width,
                args.height,
                args.timestamp_ms is not None,
                args.background,
            )
            if not all(required):
                raise ValueError("--worker-capture missing required arguments")
            sys.exit(worker_capture(args))
        if not args.release_root or not args.corpus or not args.taiyou:
            raise ValueError("main mode requires --release-root, --corpus and --taiyou")
        sys.exit(main(args))
    except Exception as error:
        print(
            f"XY_ORACLE_ERROR: {type(error).__name__}: {error}",
            file=sys.stderr,
        )
        sys.exit(2)
