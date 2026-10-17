"""Runner-only proposed T009 input acquisition; never promotes expected outputs."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import unittest
import urllib.parse
import urllib.request

ROOT = Path(__file__).resolve().parent.parent
MAX_BYTES = 24 * 1024 * 1024


def verify(data, record, discover=False):
    if len(data) > MAX_BYTES or ("size" in record and len(data) != record["size"]):
        raise ValueError("fixture size differs or exceeds budget")
    blob = hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest()
    if blob != record["git_blob"]:
        raise ValueError("fixture git blob differs")
    digest = hashlib.sha256(data).hexdigest()
    if "sha256" in record:
        if digest != record["sha256"]:
            raise ValueError("fixture SHA-256 differs")
    elif not discover:
        raise ValueError("fixture has no locked SHA-256")
    return digest


def acquire(record, repository, commit, discover):
    if not re.fullmatch(r"[0-9a-f]{40}", commit):
        raise ValueError("fixture must use an exact commit")
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError("invalid official repository")
    path = record["path"]
    if any(part in {"", ".", ".."} for part in path.split("/")) or "\\" in path:
        raise ValueError("invalid repository-relative path")
    url = f"https://raw.githubusercontent.com/{repository}/{commit}/" + urllib.parse.quote(path)
    request = urllib.request.Request(url, headers={"User-Agent": "Taiyou-T009-fixture-audit"})
    with urllib.request.urlopen(request, timeout=120) as response:
        data = response.read(MAX_BYTES + 1)
    digest = verify(data, record, discover)
    return data, digest


def main(discover):
    # No local font/cache directory and no installation side effects.
    destination = Path(os.environ["RUNNER_TEMP"]) / "taiyou-shaping-fonts"
    destination.mkdir(exist_ok=True)
    manifest = json.loads((ROOT / "build-support/shaping-fonts.proposed.json").read_text(encoding="utf-8"))
    from fontTools.ttLib import TTFont
    for record in manifest["fonts"]:
        data, digest = acquire(record, record["repository"], record["commit"], discover)
        license_data, license_digest = acquire(record["license"], record["repository"], record["commit"], discover)
        if "SIL OPEN FONT LICENSE Version 1.1" not in license_data.decode("utf-8"):
            raise ValueError("expected OFL-1.1 license text")
        path = destination / (record["id"] + Path(record["path"]).suffix)
        path.write_bytes(data)
        (destination / (record["id"] + "-LICENSE.txt")).write_bytes(license_data)
        with TTFont(path, lazy=True) as font:
            axes = {axis.axisTag: axis.defaultValue for axis in font["fvar"].axes} if "fvar" in font else {}
            cmap = font.getBestCmap()
            missing = sorted({ord(char) for char in record["sample"] if ord(char) not in cmap})
            if missing:
                raise ValueError(f"fixture sample lacks nominal coverage: {missing}")
            metadata = {"version": font["name"].getDebugName(5), "family": font["name"].getDebugName(1),
                        "upem": font["head"].unitsPerEm, "glyphs": font["maxp"].numGlyphs, "default_axes": axes}
        print("SHAPING_INPUT=" + json.dumps({"id": record["id"], "sha256": digest, "size": len(data),
              "license_sha256": license_digest, "license_git_blob": record["license"]["git_blob"],
              "license_size": len(license_data), **metadata}, sort_keys=True), flush=True)
    print("SHAPING_INPUTS_VERIFIED; no shaping/cluster/advance oracle or acceptance claimed", flush=True)


class IntegrityTests(unittest.TestCase):
    def test_exact_bytes_and_digests(self):
        data = b"fixture"
        record = {"size": len(data), "git_blob": hashlib.sha1(b"blob 7\0" + data).hexdigest(),
                  "sha256": hashlib.sha256(data).hexdigest()}
        self.assertEqual(verify(data, record), record["sha256"])
        for mutation in [{"size": 9}, {"git_blob": "0" * 40}, {"sha256": "0" * 64}]:
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                verify(data, {**record, **mutation})
        with self.assertRaises(ValueError):
            verify(b"fixturf", record)

    def test_missing_sha_is_discovery_only(self):
        data = b"fixture"
        record = {"git_blob": hashlib.sha1(b"blob 7\0" + data).hexdigest()}
        with self.assertRaises(ValueError):
            verify(data, record)
        self.assertEqual(verify(data, record, True), hashlib.sha256(data).hexdigest())

    def test_acquisition_rejects_unpinned_and_traversal_before_network(self):
        for repository, commit, path in [("google/fonts", "main", "font.ttf"),
                                         ("../fonts", "a" * 40, "font.ttf"),
                                         ("google/fonts", "a" * 40, "../font.ttf")]:
            with self.subTest(path=path), self.assertRaises(ValueError):
                acquire({"path": path}, repository, commit, False)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--discover", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        unittest.main(argv=[__file__])
    else:
        main(args.discover)
