"""Independent tiny image fixtures; never read or alter external reference PNGs."""
from array import array
import copy
from pathlib import Path
import struct
import tempfile
import unittest
import zlib

import compare
import images
from inventory import InventoryError, load
from policy import results


class ImageTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def image(self, name, values, width=2, height=1):
        path = self.root / name
        images.write_png(path, width, height, bytes(values))
        return images.read_png(path)

    def test_exact_and_transparent_rgb(self):
        a = self.image("a.png", [100, 20, 70, 0, 255, 10, 0, 255])
        b = self.image("b.png", [0, 0, 0, 0, 255, 10, 0, 255])
        self.assertTrue(images.compare(a, b)["exact"])

    def test_shifted_pixel_metrics_and_diff(self):
        a = self.image("a.png", [255, 0, 0, 255, 0, 0, 0, 0])
        b = self.image("b.png", [0, 0, 0, 0, 255, 0, 0, 255])
        m = images.compare(a, b, self.root / "diff.png")
        self.assertEqual(m["max_channel_delta"], 65535)
        self.assertEqual(m["changed_pixels"], 2)
        self.assertEqual(m["alpha_mismatches"], 2)
        self.assertEqual(m["changed_bbox"], [0, 0, 2, 1])
        self.assertEqual(m["channel_mae"], [65535, 0, 0, 65535])
        self.assertEqual(images.read_png(self.root / "diff.png")[:2], (2, 1))

    def test_partial_alpha_independent_arithmetic(self):
        a = self.image("a.png", [200, 100, 50, 128], 1)
        self.assertEqual(list(a[2]), [100 * 257, 50 * 257, 25 * 257, 128 * 257])
        raw = self.root / "a.rgba"
        raw.write_bytes(bytes([100, 50, 25, 128]))
        self.assertTrue(images.compare(a, images.raw_rgba(raw, 1, 1))["exact"])

    def test_alpha_only_difference(self):
        a = self.image("a.png", [0, 0, 0, 10], 1)
        b = self.image("b.png", [0, 0, 0, 11], 1)
        m = images.compare(a, b)
        self.assertEqual(m["channel_mae"], [0, 0, 0, 257])
        self.assertEqual(m["alpha_mismatches"], 1)

    def test_16bit_precision_preserved(self):
        path = self.root / "sixteen.png"
        path.write_bytes(images.SIGNATURE + images.chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 16, 6, 0, 0, 0))
                         + images.chunk(b"IDAT", zlib.compress(b"\0" + struct.pack(">HHHH", 65535, 32768, 1, 65535)))
                         + images.chunk(b"IEND", b""))
        self.assertEqual(list(images.read_png(path)[2]), [65535, 32768, 1, 65535])

    def test_all_png_filters(self):
        # Hand-encoded two-row RGBA image for each PNG prediction filter.
        rows = [bytes([10, 20, 30, 255, 40, 50, 60, 255]), bytes([30, 40, 50, 255, 70, 80, 90, 255])]
        for method in range(5):
            encoded = bytearray()
            for y, row in enumerate(rows):
                encoded.append(method)
                prev = rows[y - 1] if y else bytes(8)
                for x, value in enumerate(row):
                    a, b, c = row[x - 4] if x >= 4 else 0, prev[x], prev[x - 4] if x >= 4 else 0
                    pred = (0, a, b, (a + b) // 2, images.paeth(a, b, c))[method]
                    encoded.append((value - pred) & 255)
            path = self.root / f"filter{method}.png"
            path.write_bytes(images.SIGNATURE + images.chunk(b"IHDR", struct.pack(">IIBBBBB", 2, 2, 8, 6, 0, 0, 0))
                             + images.chunk(b"IDAT", zlib.compress(encoded)) + images.chunk(b"IEND", b""))
            self.assertEqual(list(images.read_png(path)[2]), [v * 257 for row in rows for v in row])

    def test_corrupt_missing_truncated_and_decompression_bomb(self):
        path = self.root / "bad.png"
        for content in [b"not PNG", images.SIGNATURE, images.SIGNATURE + b"\xff" * 12]:
            path.write_bytes(content)
            with self.assertRaises(ValueError): images.read_png(path)
        with self.assertRaises(OSError): images.read_png(self.root / "missing.png")
        self.image("valid.png", [0] * 8)
        corrupt = bytearray((self.root / "valid.png").read_bytes())
        corrupt[-1] ^= 1
        path.write_bytes(corrupt)
        with self.assertRaises(ValueError): images.read_png(path)
        path.write_bytes(images.SIGNATURE + images.chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0))
                         + images.chunk(b"IDAT", zlib.compress(bytes(1000))) + images.chunk(b"IEND", b""))
        with self.assertRaises(ValueError): images.read_png(path)

    def test_wrong_dimensions_and_raw_size(self):
        a = self.image("a.png", [0] * 8)
        b = self.image("b.png", [0] * 4, 1)
        self.assertFalse(images.compare(a, b)["dimensions_match"])
        path = self.root / "a.rgba"
        path.write_bytes(b"\0")
        with self.assertRaises(ValueError): images.raw_rgba(path, 2, 1)
        path.write_bytes(bytes([255, 0, 0, 0]))
        with self.assertRaises(ValueError): images.raw_rgba(path, 1, 1)
        with self.assertRaises(ValueError): images.dimensions(65536, 65536)


class AccountingTests(unittest.TestCase):
    def setUp(self):
        self.inventory = load(Path(__file__).with_name("inventory.json"))
        self.policy = load(Path(__file__).with_name("policy.json"))

    def test_denominator_and_disabled_preserved(self):
        cases = compare.select(self.inventory, "full", None, [])
        records = [{"case": c["id"], "frame": c["frames"][0]["expected"] if c["frames"] else None,
                    "profile": "CoreV1", "status": "UNSUPPORTED", "feature": "synthetic"}
                   for c in cases if c["kind"] != "disabled-upstream"]
        summary = results(self.inventory, {**self.policy, "profiles": ["CoreV1"]}, records)
        self.assertEqual(summary["total"] + summary["disabled_upstream"], len(cases))
        for invalid in [records[:-1], records + records[:1]]:
            with self.assertRaises(InventoryError): results(self.inventory, {**self.policy, "profiles": ["CoreV1"]}, invalid)

    def test_selectors(self):
        case = self.inventory["cases"][0]["id"]
        self.assertEqual(len(compare.select(self.inventory, "focused", None, [case])), 1)
        for suite, ids in [("full", [case]), ("focused", []), ("focused", [case, case]), ("focused", ["absent"])]:
            with self.assertRaises(InventoryError): compare.select(self.inventory, suite, None, ids)

    def test_invalid_timestamps(self):
        case = {"source": "test.ass"}
        self.assertEqual(compare.timestamp(case, {"expected": "test-0500.png", "timestamp_ms": 500}), 500000)
        for name, value in [("test-x.png", 0), ("test-1.png", 2), ("other-1.png", 1), ("test-9223372036854776.png", 9223372036854776)]:
            with self.assertRaises(InventoryError): compare.timestamp(case, {"expected": name, "timestamp_ms": value})

    def test_no_false_pass_for_unsupported_or_missing_fonts(self):
        case = next(c for c in self.inventory["cases"] if c["kind"] == "regression")
        self.assertEqual(compare.classify(case, 2, None, "FontUnavailable"), "INFRA_ERROR")
        self.assertEqual(compare.classify(case, -9, None, "killed"), "CRASH")
        self.assertEqual(compare.classify(case, -999, None, "timeout"), "CRASH")
        self.assertEqual(compare.classify(case, 0, {"exact": False}, ""), "UNSUPPORTED")
        supported = copy.deepcopy(case)
        supported["profile_records"]["CoreV1"]["support"] = "SUPPORTED"
        self.assertEqual(compare.classify(supported, 0, {"exact": False}, ""), "REGRESSION")
        self.assertEqual(compare.classify(supported, 2, None, "UnsupportedFeature"), "REGRESSION")


if __name__ == "__main__":
    unittest.main(verbosity=2)
