"""Supplemental exporter tests. Existing comparison tests remain unchanged."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import compare
import images
from inventory import load
from visual_review import Review, family, stem, verify


class VisualReviewTests(unittest.TestCase):
    def test_all_inventory_frames_have_unique_shallow_behavior_names(self):
        inventory = load(Path(__file__).with_name("inventory.json"))
        paths = []
        for case in inventory["cases"]:
            if case["kind"] == "regression":
                paths.append((family(case)[0], stem(case)))
        self.assertEqual(len(paths), len(set(paths)))
        self.assertEqual({p[0] for p in paths}, {"blur", "karaoke", "positioning", "scaling", "fonts", "drawings", "parsing", "alpha"})
        self.assertTrue(all("/" not in name and "\\" not in name for _, name in paths))
        with self.assertRaises(KeyError):
            family({"source": "regression/unknown/new.ass"})

    def exercise(self, strict_code, pixel, scale="1", malformed=False):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "regression/v4++/kt.ass"
            source.parent.mkdir(parents=True)
            source.write_text("synthetic source")
            (root / "font.ttf").write_bytes(b"synthetic")
            reference = source.with_name("kt-01000.png")
            images.write_png(reference, 1, 1, bytes([255, 0, 0, 255]))
            original = reference.read_bytes()
            case = {"id": "regression/v4++/kt-01000.png", "source": "regression/v4++/kt.ass",
                "kind": "regression", "fonts": ["font.ttf"], "scale": scale,
                "frames": [{"expected": "regression/v4++/kt-01000.png", "timestamp_ms": 1000, "reference_size": [1, 1]}]}
            assets = {p: {"path": p} for p in (case["source"], "font.ttf", case["frames"][0]["expected"])}
            review = Review(root / "visual-review")
            def execute(command, directory):
                if "--unsupported" not in command and strict_code != 0:
                    return strict_code, "strict result"
                png, raw = Path(command[command.index("--output") + 1]), Path(command[command.index("--raw") + 1])
                width = int(command[command.index("--width") + 1])
                images.write_png(png, width, 1, bytes(pixel) * width)
                raw.write_bytes(b"invalid" if malformed else bytes(pixel) * width)
                return 0, "rendered"
            with patch.object(compare, "execute", side_effect=execute):
                plain = compare.run_case(case, root, root / "renderer", root / "probe", root, assets, 12)
                exported = compare.run_case(case, root, root / "renderer", root / "probe", root, assets, 12, review)
            # Additive retention may not change a single authoritative field.
            self.assertEqual(plain, exported)
            report = {"candidate_sha": "a" * 40, "accepted_baseline": "b" * 40,
                "corpus_revision": "c" * 40, "suite": "focused", "summary": {}, "records": [exported]}
            review.finish(report, [case])
            self.assertEqual(reference.read_bytes(), original)
            if malformed:
                with self.assertRaises(ValueError):
                    verify(review.root)
            else:
                verify(review.root)
                if strict_code in (0, 20):
                    saved = review.frames[case["id"]]
                    self.assertEqual(saved["diagnostic"], strict_code == 20)
                    self.assertEqual(len(saved["images"]), 3)
                    self.assertEqual((review.root / saved["images"][0]).read_bytes(), original)
                    self.assertEqual(images.read_png(review.root / saved["images"][1])[:2], (1, 1))
                    diff = images.read_png(review.root / saved["images"][2])[2]
                    self.assertEqual(list(diff), [0, 0, 0, 65535] if pixel[0] == 255 else [65535, 0, 0, 65535])
                else:
                    self.assertEqual(review.frames, {})
            return exported

    def test_exact_and_mismatch_after_legacy_artifact_cap(self):
        self.assertEqual(self.exercise(0, [255, 0, 0, 255])["status"], "PASS_EXACT")
        self.assertEqual(self.exercise(0, [0, 0, 0, 255])["observed_result"], "PIXEL_MISMATCH")

    def test_existing_tolerant_output_remains_unsupported(self):
        record = self.exercise(20, [255, 0, 0, 255])
        self.assertEqual(record["status"], "UNSUPPORTED")
        self.assertIsNone(record["metrics"])
        self.assertEqual(record["exit_code"], 20)

    def test_oversampled_preview_has_comparison_dimensions(self):
        self.exercise(0, [255, 0, 0, 255], scale="2x1")

    def test_invalid_tolerant_pixels_do_not_change_strict_outcome(self):
        self.assertEqual(self.exercise(20, [255, 0, 0, 255], malformed=True)["status"], "UNSUPPORTED")

    def test_crashes_and_missing_fonts_do_not_gain_images(self):
        self.assertEqual(self.exercise(-9, [0, 0, 0, 0])["status"], "CRASH")
        self.assertEqual(self.exercise(30, [0, 0, 0, 0])["status"], "INFRA_ERROR")


if __name__ == "__main__":
    unittest.main(verbosity=2)
