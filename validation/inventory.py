"""Candidate corpus inventory tooling. Trusted deployment is a separate gate."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import struct
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent


class InventoryError(Exception):
    pass


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=True).encode()


def digest(value):
    return hashlib.sha256(value).hexdigest()


def load(path):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise InventoryError("duplicate JSON key: " + key)
            result[key] = value
        return result
    if path.stat().st_size > 8 * 1024 * 1024:
        raise InventoryError("JSON resource limit")
    return json.loads(path.read_text(encoding="utf-8"), object_pairs_hook=unique)


def safe_path(root, relative):
    name = PurePosixPath(relative)
    if name.is_absolute() or ".." in name.parts or "\\" in relative or not name.parts:
        raise InventoryError("unsafe corpus path")
    path = root.joinpath(*name.parts)
    try:
        path.resolve(strict=True).relative_to(root.resolve(strict=True))
    except (OSError, ValueError) as error:
        raise InventoryError("missing or escaping asset: " + relative) from error
    return path


def pin():
    return next(item for item in load(ROOT / "docs/references.lock.json")["references"] if item["id"] == "libass-tests")


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args])


def checked_bytes(root, relative, mode, blob):
    path = safe_path(root, relative)
    if path.is_symlink() != (mode == "120000"):
        raise InventoryError("symlink mode lost: " + relative)
    if path.stat().st_size > 64 * 1024 * 1024:
        raise InventoryError("asset exceeds resource limit")
    raw = os.readlink(path).encode() if mode == "120000" else path.read_bytes()
    actual = hashlib.sha1(b"blob " + str(len(raw)).encode() + b"\0" + raw).hexdigest()
    if actual != blob:
        raise InventoryError("working asset differs from Git blob: " + relative)
    return path, raw


def build(root):
    expected = pin()
    if git(root, "rev-parse", "HEAD").decode().strip() != expected["commit"]:
        raise InventoryError("unexpected corpus revision")
    assets = []
    for entry in filter(None, git(root, "ls-tree", "-r", "-z", "HEAD").split(b"\0")):
        metadata, name = entry.split(b"\t", 1)
        mode, kind, blob = metadata.decode().split()
        relative = name.decode("utf-8")
        if kind != "blob" or mode not in {"100644", "100755", "120000"}:
            raise InventoryError("unsupported Git entry")
        path, raw = checked_bytes(root, relative, mode, blob)
        resolved = path.resolve()
        content = resolved.read_bytes()
        suffix = path.suffix.lower()
        role = "font" if suffix in {".ttf", ".otf", ".ttc"} else "image" if suffix == ".png" else "source" if relative.endswith((".ass", ".ass.disabled")) else "metadata"
        asset = {"path": relative, "mode": mode, "git_blob": blob, "sha256": digest(raw), "bytes": len(raw), "role": role, "license": "unknown-upstream-no-license-file"}
        if mode == "120000":
            asset.update({"target": os.readlink(path), "resolved": resolved.relative_to(root.resolve()).as_posix(), "resolved_sha256": digest(content)})
        if role == "image":
            if len(content) < 24 or content[:8] != b"\x89PNG\r\n\x1a\n" or content[12:16] != b"IHDR":
                raise InventoryError("invalid PNG header")
            width, height = struct.unpack(">II", content[16:24])
            if not 0 < width <= 100000 or not 0 < height <= 100000:
                raise InventoryError("invalid PNG dimensions")
            asset["dimensions"] = [width, height]
        assets.append(asset)
    paths = {asset["path"]: asset for asset in assets}
    present = set()
    for directory, directories, files in os.walk(root, followlinks=False):
        if Path(directory) == root:
            directories[:] = [name for name in directories if name != ".git"]
        for name in files:
            present.add((Path(directory) / name).relative_to(root).as_posix())
        for name in directories:
            if (Path(directory) / name).is_symlink():
                present.add((Path(directory) / name).relative_to(root).as_posix())
    if present != set(paths):
        raise InventoryError("INFRA_ERROR: missing or untracked corpus files")
    cases = []
    used_images = set()
    for asset in assets:
        source = asset["path"]
        if asset["role"] != "source":
            continue
        path = PurePosixPath(source)
        kind = "disabled-upstream" if source.endswith(".disabled") else "crash" if path.parts[0] == "crash" else "regression"
        frames = []
        if kind == "regression":
            prefix = source[:-4] + "-"
            for image, record in sorted(paths.items()):
                if image.startswith(prefix) and re.fullmatch(r"[0-9]+\.png", image[len(prefix):]):
                    time = int(image[len(prefix):-4])
                    frames.append({"expected": image, "timestamp_ms": time, "reference_size": record["dimensions"]})
                    used_images.add(image)
            if not frames:
                raise InventoryError("INFRA_ERROR: regression source has no expected PNG: " + source)
        fonts = sorted(name for name, item in paths.items() if PurePosixPath(name).parent == path.parent and item["role"] == "font")
        scale_path = path.parent / "scale"
        raw_scale = safe_path(root, str(scale_path)).read_text().strip() if str(scale_path) in paths else "1"
        if not re.fullmatch(r"[1-8](x[1-8])?", raw_scale):
            raise InventoryError("unsupported upstream scale metadata")
        common = {"parent_id": source, "kind": kind, "source": source, "fonts": fonts, "scale": raw_scale, "features": [path.parent.name], "profile_records": {profile: {"support": "UNSUPPORTED", "reason": "T014-renderer-not-implemented"} for profile in ["CoreV1", "LegacyXyV1"]}, "oracle": {"commit": next(item["commit"] for item in load(ROOT / "docs/references.lock.json")["references"] if item["id"] == "libass"), "build": None, "config": "pending-T015"}, "semantics_review": "timestamp filename and scale metadata recorded; pinned oracle adapter pending T015"}
        for frame in frames or [None]:
            cases.append(dict(common, id=frame["expected"] if frame else source, frames=[frame] if frame else []))
    return {"schema": 1, "repository": expected["repository"], "commit": expected["commit"], "assets": assets, "cases": cases, "auxiliary_images": sorted(name for name, item in paths.items() if item["role"] == "image" and name not in used_images), "license_status": "unknown: pin has no LICENSE/COPYING; no corpus redistribution authorized", "protection_status": "candidate-only; maintainer configuration and independent review pending"}


def validate(inventory):
    if inventory.get("schema") != 1 or inventory.get("commit") != pin()["commit"]:
        raise InventoryError("inventory schema/revision")
    assets = inventory.get("assets", [])
    cases = inventory.get("cases", [])
    if not assets or not cases:
        raise InventoryError("empty inventory")
    paths = {item["path"] for item in assets}
    if len(paths) != len(assets) or len({case["id"] for case in cases}) != len(cases):
        raise InventoryError("duplicate asset/case")
    for item in assets:
        if not re.fullmatch("[0-9a-f]{64}", item["sha256"]):
            raise InventoryError("invalid asset SHA-256")
    for case in cases:
        if case["source"] not in paths or any(font not in paths for font in case["fonts"]):
            raise InventoryError("missing source/font")
        if case["kind"] == "regression" and not case["frames"]:
            raise InventoryError("missing expected PNG")
        for frame in case["frames"]:
            if frame["expected"] not in paths:
                raise InventoryError("missing expected PNG")


def verify(root, inventory):
    validate(inventory)
    if build(root) != inventory:
        raise InventoryError("INFRA_ERROR: corpus/inventory differs")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["generate", "verify"])
    parser.add_argument("--corpus", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    args = parser.parse_args()
    if args.action == "generate":
        inventory = build(args.corpus)
        validate(inventory)
        args.manifest.write_text(json.dumps(inventory, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        print("INVENTORY_JSON=" + json.dumps(inventory, sort_keys=True), flush=True)
    else:
        inventory = load(args.manifest)
        verify(args.corpus, inventory)
        print("INVENTORY_VERIFIED=" + json.dumps({"commit": inventory["commit"], "assets": len(inventory["assets"]), "cases": len(inventory["cases"]), "frames": sum(len(case["frames"]) for case in inventory["cases"]), "digest": digest(canonical(inventory))}), flush=True)


if __name__ == "__main__":
    try:
        main()
    except (InventoryError, OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print("INFRA_ERROR: " + str(error), file=sys.stderr)
        sys.exit(1)
