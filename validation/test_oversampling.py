"""Additive adapter tests. Execute only in GitHub Actions."""
from array import array
from pathlib import Path
import tempfile
import unittest
import oversampling


class OversamplingTests(unittest.TestCase):
    def test_scale_grammar_and_fixed_reciprocal(self):
        self.assertEqual(oversampling.factors("8x1"), (8, 1))
        self.assertEqual(oversampling.factors("2"), (2, 2))
        for value in ["0", "9x1", "1x0", "8x", "8x1x1", "01"]:
            with self.assertRaises(ValueError):
                oversampling.factors(value)
        with tempfile.TemporaryDirectory() as folder:
            raw = Path(folder) / "input.rgba"
            raw.write_bytes(bytes([1, 2, 3, 255, 4, 5, 6, 255]))
            self.assertEqual(oversampling.normalize(raw, 1, 1, 2, 1),
                             (1, 1, array("H", [642, 899, 1156, 65535])))
            raw.write_bytes(bytes([0, 0, 0, 0, 255, 255, 255, 255]) * 4)
            self.assertEqual(oversampling.normalize(raw, 1, 1, 8, 1)[2], array("H", [32767] * 4))
            raw.write_bytes(bytes([255, 0, 0, 0]) * 8)
            with self.assertRaisesRegex(ValueError, "premultiplied"):
                oversampling.normalize(raw, 1, 1, 8, 1)
            raw.write_bytes(b"truncated")
            with self.assertRaisesRegex(ValueError, "length"):
                oversampling.normalize(raw, 1, 1, 8, 1)


if __name__ == "__main__":
    unittest.main()
