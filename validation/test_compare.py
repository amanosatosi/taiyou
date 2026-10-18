"""Independent tiny image fixtures; never read or alter external reference PNGs."""
from array import array
import copy
import contextlib
import io
from pathlib import Path
import re
import struct
import tempfile
import unittest
import zlib
from unittest.mock import patch

import compare
import images
from libass_adapter import harness_summary
from inventory import InventoryError, canonical, digest, load
from policy import results
from reference_pixels import rgba8_pairs, rgba16_samples, write_rgba16


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

    def test_rgba8_all_65536_pairs_match_pinned_fixed_reciprocal(self):
        # compare/image.c at f61db567: fixed reciprocal 257/65536, not 1/255.
        # This algebraic reference is independent of the decoder's shift/add
        # implementation. The native oracle also executes original image.c.
        path = self.root / "all-eight.png"
        images.write_png(path, 256, 256, rgba8_pairs())
        decoded = images.read_png(path)[2]
        differs_from_div255 = 0
        for alpha in range(256):
            for channel in range(256):
                product = channel * alpha
                expected = (product * 257 + 32768) // 65536
                index = (alpha * 256 + channel) * 4
                self.assertEqual(list(decoded[index:index + 4]), [expected * 257] * 3 + [alpha * 257])
                differs_from_div255 += expected != (product + 127) // 255
        self.assertEqual(differs_from_div255, 24)
        self.assertEqual(decoded[(152 * 256 + 229) * 4], 136 * 257)

    def test_rgba16_fixed_reciprocal_boundary_sweep(self):
        # Independently expand the pinned 16-bit reciprocal 65537 / 2**32.
        # Sweep every channel against six alpha edges, not merely opaque input.
        data = rgba16_samples()
        path = self.root / "sixteen-sweep.png"
        write_rgba16(path, 256, 1536, data)
        decoded = images.read_png(path)[2]
        for i, (r, g, b, a) in enumerate(struct.iter_unpack(">HHHH", data)):
            expected = [(c * a * 65537 + (1 << 31)) // (1 << 32) for c in (r, g, b)] + [a]
            self.assertEqual(list(decoded[i * 4:i * 4 + 4]), expected)

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

    def test_valid_png_crcs_with_invalid_zlib_are_typed(self):
        path = self.root / "bad-zlib.png"
        # Correct zlib header followed by reserved DEFLATE block type 3.
        path.write_bytes(images.SIGNATURE + images.chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0))
                         + images.chunk(b"IDAT", b"\x78\x9c\x07") + images.chunk(b"IEND", b""))
        with self.assertRaisesRegex(images.ImageDecodeError, "invalid PNG zlib stream"):
            images.read_png(path)


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

    def test_native_harness_denominator(self):
        self.assertEqual(harness_summary("All 10 images have passed test (SAME or better)", 10), {"passed": 10, "total": 10})
        self.assertEqual(harness_summary("Only 2 of 10 images have passed test (BAD or better)", 10), {"passed": 2, "total": 10})
        for text in ("No images found!", "All 9 images have passed test", "Only 11 of 10 images have passed test"):
            with self.assertRaises(ValueError): harness_summary(text, 10)

    def test_no_false_pass_for_unsupported_or_missing_fonts(self):
        case = next(c for c in self.inventory["cases"] if c["kind"] == "regression")
        self.assertEqual(compare.classify(case, 30, None, "font absent"), "INFRA_ERROR")
        self.assertEqual(compare.classify(case, -9, None, "killed"), "CRASH")
        self.assertEqual(compare.classify(case, -999, None, "timeout"), "CRASH")
        self.assertEqual(compare.classify(case, 0, {"exact": False}, ""), "UNSUPPORTED")
        supported = copy.deepcopy(case)
        supported["profile_records"]["CoreV1"]["support"] = "SUPPORTED"
        self.assertEqual(compare.classify(supported, 0, {"exact": False}, ""), "REGRESSION")
        self.assertEqual(compare.classify(supported, 20, None, "tag"), "REGRESSION")

    def test_renderer_errors_cannot_be_hidden_by_message_substrings(self):
        case = next(c for c in self.inventory["cases"] if c["kind"] == "regression")
        self.assertEqual(compare.classify(case, 80, None, "fake UnsupportedFeature"), "INFRA_ERROR")
        self.assertEqual(compare.classify(case, 34, None, "taiyou: UnsupportedFeature: fake"), "INFRA_ERROR")
        self.assertEqual(compare.classify(case, 20, None, "any diagnostic wording"), "UNSUPPORTED")

    def test_missing_explicit_fonts_are_infrastructure_error(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "test.ass").write_text("synthetic source")
            images.write_png(root / "test-0000.png", 1, 1, bytes([0, 0, 0, 0]))
            case = {"id": "test-0000.png", "kind": "regression", "source": "test.ass", "fonts": [], "scale": "1",
                    "frames": [{"expected": "test-0000.png", "timestamp_ms": 0, "reference_size": [1, 1]}]}
            assets = {p: {"path": p} for p in ("test.ass", "test-0000.png")}
            record = compare.run_case(case, root, root / "never-executed", root / "never-executed", root, assets, 0)
            self.assertEqual(record["status"], "INFRA_ERROR")
            self.assertIn("no explicit fonts", record["diagnostics"])

    def test_visual_and_parser_safety_subtotals_preserve_policy_counts(self):
        records = [{"case": c["id"], "frame": c["frames"][0]["expected"] if c["frames"] else None,
                    "profile": "CoreV1", "kind": c["kind"], "status": "UNSUPPORTED", "feature": "synthetic",
                    "stage": "render", "metrics": None}
                   for c in self.inventory["cases"] if c["kind"] != "disabled-upstream"]
        for record in records:
            if record["kind"] == "crash":
                record.update(status="PASS_EXACT", stage="parser-safety-probe", safety_only=True)
        visual = next(r for r in records if r["kind"] == "regression")
        visual.update(status="PASS_EXACT", stage="pixel-comparison", metrics={"exact": True})
        summary = compare.summarize(self.inventory, self.inventory["cases"], self.policy, records)
        self.assertEqual(summary["counts"]["PASS_EXACT"], 3)  # Unchanged T005 aggregate.
        self.assertEqual(summary["visual"]["exact_matches"], 1)
        self.assertEqual(summary["safety"]["successful_parser_probes"], 2)
        self.assertEqual(summary["visual"]["total"] + summary["safety"]["total"] + len(summary["disabled"]), len(self.inventory["cases"]))
        for mutation in ({"kind": "crash"}, {"metrics": None}, {"safety_only": True}):
            altered = copy.deepcopy(records)
            next(r for r in altered if r["kind"] == "regression").update(mutation)
            with self.assertRaises(InventoryError):
                compare.summarize(self.inventory, self.inventory["cases"], self.policy, altered)

    def test_bad_zlib_case_does_not_abort_full_report(self):
        # Entirely synthetic corpus. Neither external bytes nor the accepted
        # inventory are edited. Only acquisition/probe subprocesses are mocked;
        # the actual PNG decoder, case loop, accounting and report writer run.
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in ("test.ass", "font.ttf", "crash1.ass", "crash2.ass", "probe", "renderer"):
                (root / name).write_text("synthetic")
            (root / "test-0000.png").write_bytes(images.SIGNATURE
                + images.chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0))
                + images.chunk(b"IDAT", b"\x78\x9c\x07") + images.chunk(b"IEND", b""))
            images.write_png(root / "test-0001.png", 1, 1, bytes(4))
            cases = [{"id": f"test-{time:04}.png", "source": "test.ass", "kind": "regression", "fonts": ["font.ttf"],
                      "scale": "8x1", "frames": [{"expected": f"test-{time:04}.png", "timestamp_ms": time, "reference_size": [1, 1]}]}
                     for time in (0, 1)]
            cases += [{"id": name, "source": name, "kind": "crash", "fonts": [], "frames": [], "scale": "1"}
                      for name in ("crash1.ass", "crash2.ass")]
            cases += [{"id": "disabled.ass.disabled", "kind": "disabled-upstream", "frames": []}]
            for case in cases:
                case["profile_records"] = {"CoreV1": {"support": "UNSUPPORTED"}}
            paths = ["test.ass", "font.ttf", "test-0000.png", "test-0001.png", "crash1.ass", "crash2.ass"]
            inventory = {"commit": self.inventory["commit"], "cases": cases, "assets": [{"path": p} for p in paths]}
            output = root / "report"
            argv = ["compare", "--suite", "full", "--profile", "core-v1", "--corpus", str(root),
                    "--output", str(output), "--renderer", str(root / "renderer"), "--probe", str(root / "probe")]
            with patch("sys.argv", argv), patch.object(compare, "accepted_snapshot", return_value=(root, inventory, self.policy)), \
                 patch.object(compare.subprocess, "run") as verification, \
                 patch.object(compare.subprocess, "check_output", return_value="0" * 40), \
                 patch.object(compare, "execute", return_value=(0, "SAFETY_ONLY")), contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(compare.main(), 1)
            self.assertEqual(verification.call_count, 2)  # Before and after.
            report = load(output / "report.json")
            self.assertEqual(len(report["records"]), 4)
            self.assertEqual(report["records"][0]["status"], "INFRA_ERROR")
            self.assertEqual(report["records"][0]["diagnostic_code"], "InvalidImage")
            self.assertEqual(report["summary"]["selected_including_disabled"], 5)
            self.assertEqual(report["summary"]["unselected"], [])
            self.assertEqual(report["summary"]["visual"]["counts"]["INFRA_ERROR"], 1)
            self.assertEqual(report["summary"]["safety"]["successful_parser_probes"], 2)


class FullReportInvariantTests(unittest.TestCase):
    def setUp(self):
        # Independent tiny corpus; outcome counts are not the external corpus's
        # present observations. No accepted inventory or policy is modified.
        self.policy = load(Path(__file__).with_name("policy.json"))
        self.candidate = "0" * 40
        self.inventory = {"commit": "1" * 40, "cases": [
            {"id": "test-0000.png", "kind": "regression", "scale": "1",
             "frames": [{"expected": "test-0000.png", "reference_size": [1, 1]}]},
            {"id": "probe.ass", "kind": "crash", "frames": [], "scale": "1"},
            {"id": "disabled.ass.disabled", "kind": "disabled-upstream", "frames": []}]}
        for case in self.inventory["cases"]:
            case["profile_records"] = {"CoreV1": {"support": "UNSUPPORTED"}}
        self.records = [
            {"case": "test-0000.png", "frame": "test-0000.png", "profile": "CoreV1", "kind": "regression",
             "status": "INFRA_ERROR", "exit_code": 30, "stage": "render", "metrics": None, "feature": "synthetic"},
            {"case": "probe.ass", "frame": None, "profile": "CoreV1", "kind": "crash", "status": "PASS_EXACT",
             "exit_code": 0, "stage": "parser-safety-probe", "safety_only": True, "metrics": None}]

    def report(self, records=None):
        records = copy.deepcopy(self.records if records is None else records)
        return {"suite": "full", "profile": "CoreV1", "candidate_sha": self.candidate,
                "corpus_revision": self.inventory["commit"], "inventory_sha256": digest(canonical(self.inventory)),
                "policy_sha256": digest(canonical(self.policy)), "records": records,
                "summary": compare.summarize(self.inventory, self.inventory["cases"], self.policy, records)}

    def check(self, report):
        return validate_full_report(report, self.inventory, self.policy, self.candidate)

    def exact_records(self):
        # Authored opaque red pixel, independently identical on both sides.
        # Exercise real raw/PNG decoding and comparison, not a fabricated flag.
        with tempfile.TemporaryDirectory() as directory:
            raw, png = Path(directory) / "pixel.rgba", Path(directory) / "pixel.png"
            raw.write_bytes(bytes([255, 0, 0, 255]))
            images.write_png(png, 1, 1, bytes([255, 0, 0, 255]))
            records = copy.deepcopy(self.records)
            records[0].update(status="PASS_EXACT", exit_code=0, stage="pixel-comparison",
                              metrics=images.compare(images.read_png(png), images.raw_rgba(raw, 1, 1)),
                              raw_sha256=digest(raw.read_bytes()), png_sha256=digest(png.read_bytes()))
            return records

    def test_genuine_infrastructure_recovery_can_become_exact(self):
        before = self.check(self.report())
        after = self.check(self.report(self.exact_records()))
        self.assertEqual(before["visual"]["counts"]["INFRA_ERROR"], 1)
        self.assertEqual(after["visual"]["counts"]["INFRA_ERROR"], 0)
        self.assertEqual((before["visual"]["exact_matches"], after["visual"]["exact_matches"]), (0, 1))
        self.assertEqual(before["total"], after["total"])
        self.assertEqual(before["safety"], after["safety"])

    def test_missing_duplicate_unselected_stale_and_carried_rejected(self):
        for records in (self.records[:-1], self.records + self.records[:1],
                        [{**self.records[0], "case": "unselected"}, self.records[1]]):
            with self.subTest(records=records), self.assertRaises(InventoryError):
                self.check(self.report(records))
        for mutate in (lambda r: r.update(candidate_sha="2" * 40),
                       lambda r: r.update(inventory_sha256="2" * 64),
                       lambda r: r["summary"]["carried_results"].append("old"),
                       lambda r: r["summary"]["unselected"].append("test-0000.png"),
                       lambda r: r["records"][0].update(carried_from_sha="2" * 40),
                       lambda r: r["summary"]["visual"].update(exact_matches=99),
                       lambda r: r["summary"]["counts"].update(PASS_EXACT=99)):
            report = self.report()
            mutate(report)
            with self.subTest(report=report), self.assertRaises(AssertionError): self.check(report)

    def test_exact_and_safety_passes_require_consistent_evidence(self):
        for mutation in ({"metrics": {"exact": True}}, {"exit_code": 30}, {"raw_sha256": ""},
                         {"stage": "render"}, {"kind": "crash"}, {"safety_only": True}):
            records = self.exact_records()
            records[0].update(mutation)
            with self.subTest(mutation=mutation), self.assertRaises((AssertionError, InventoryError)):
                self.check(self.report(records))
        records = self.exact_records()
        records[0]["metrics"]["max_channel_delta"] = 1
        with self.assertRaises(AssertionError): self.check(self.report(records))
        for mutation in ({"exit_code": 1}, {"exit_code": None}, {"stage": "render"}, {"safety_only": False}):
            records = copy.deepcopy(self.records)
            records[1].update(mutation)
            with self.subTest(mutation=mutation), self.assertRaises((AssertionError, InventoryError)):
                self.check(self.report(records))

    def test_infrastructure_cannot_be_relabelled_unsupported(self):
        records = copy.deepcopy(self.records)
        records[0]["status"] = "UNSUPPORTED"
        with self.assertRaises(AssertionError): self.check(self.report(records))
        records[0].update(exit_code=None, diagnostic_code="InvalidImage", stage="asset-verification")
        with self.assertRaises(AssertionError): self.check(self.report(records))

    def test_supported_regression_remains_failure(self):
        self.inventory["cases"][0]["profile_records"]["CoreV1"]["support"] = "SUPPORTED"
        records = self.exact_records()
        expected = (1, 1, array("H", [65535, 0, 0, 65535]))
        actual = (1, 1, array("H", [0, 0, 0, 65535]))
        records[0].update(status="REGRESSION", metrics=images.compare(expected, actual))
        with self.assertRaises(AssertionError): self.check(self.report(records))
        records[0]["status"] = "UNSUPPORTED"
        with self.assertRaises(InventoryError): self.check(self.report(records))


def validate_full_report(report, inventory, policy, candidate_sha):
    """Validate current evidence/accounting, never freeze observed outcome counts."""
    if report["suite"] != "full" or report["profile"] != "CoreV1" or report["corpus_revision"] != inventory["commit"]:
        raise AssertionError("expected full CoreV1 run at accepted corpus revision")
    if report["candidate_sha"] != candidate_sha:
        raise AssertionError("report is not from the current candidate")
    if report["inventory_sha256"] != digest(canonical(inventory)) or report["policy_sha256"] != digest(canonical(policy)):
        raise AssertionError("report does not use accepted snapshots")
    # T005 rejects missing, duplicate, unselected and demoted supported cases;
    # summarize checks accepted kinds and reconstructs all subtotals/denominators.
    expected = compare.summarize(inventory, inventory["cases"], policy, report["records"])
    if report["summary"] != expected:
        raise AssertionError("report summary differs from complete case accounting")
    cases = {c["id"]: c for c in inventory["cases"]}
    for record in report["records"]:
        case = cases[record["case"]]
        status, code = record["status"], record.get("exit_code")
        if any(record.get(key) for key in ("carried", "carried_from", "carried_from_sha")):
            raise AssertionError("carried result cannot count as current")
        if record.get("diagnostic_code") == "InvalidImage" and status != "INFRA_ERROR":
            raise AssertionError("image infrastructure failure was reclassified")
        if code is None:
            # Only pre-execution infrastructure errors or the explicit scale
            # limitation can lack an execution result. Neither is a pass.
            if record["stage"] == "scale" and case["scale"] != "1":
                required = "UNSUPPORTED"
            else:
                required = "INFRA_ERROR"
        elif type(code) is not int:
            raise AssertionError("invalid subprocess exit evidence")
        elif case["kind"] == "crash":
            required = "CRASH" if code < 0 else "PASS_EXACT" if code == 0 else "INFRA_ERROR"
        else:
            if code == 0:
                metrics = record["metrics"]
                frame = next(f for f in case["frames"] if f["expected"] == record["frame"])
                if not metrics or record["stage"] != "pixel-comparison" or metrics.get("dimensions_match") is not True:
                    raise AssertionError("successful render lacks pixel evidence")
                if metrics["expected_dimensions"] != frame["reference_size"] or metrics["actual_dimensions"] != frame["reference_size"]:
                    raise AssertionError("pixel evidence has wrong dimensions")
                zero = (metrics["max_channel_delta"] == 0 and metrics["channel_mae"] == [0] * 4
                        and metrics["changed_pixels"] == 0 and metrics["changed_bbox"] is None
                        and metrics["alpha_mismatches"] == 0)
                if metrics["exact"] is not zero:
                    raise AssertionError("exact flag contradicts pixel metrics")
                if any(not re.fullmatch(r"[0-9a-f]{64}", record.get(key, "")) for key in ("raw_sha256", "png_sha256")):
                    raise AssertionError("pixel evidence lacks output identities")
            required = compare.classify(case, code, record["metrics"], record.get("diagnostics", ""))
            if record.get("tolerant_attempt") and required == "PASS_EXACT":
                support = case["profile_records"]["CoreV1"]["support"]
                required = "UNSUPPORTED" if support == "UNSUPPORTED" else "REGRESSION"
        if status != required:
            raise AssertionError("status contradicts execution evidence")
        if status in ("REGRESSION", "CRASH"):
            raise AssertionError("regression or crash remains a failed validation")
    # INFRA_ERROR counts are deliberately unrestricted: they remain visible in
    # this valid report and still fail the separate comparator command.
    return expected


def check_full_report(path):
    report = load(path)
    _, inventory, policy = compare.accepted_snapshot()
    candidate = compare.subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=compare.ROOT, text=True).strip()
    validate_full_report(report, inventory, policy, candidate)
    print("FULL_REPORT_ACCOUNTING_VERIFIED: complete current inventory, evidence and visual/safety subtotal invariants")


if __name__ == "__main__":
    import sys
    if len(sys.argv) == 3 and sys.argv[1] == "--report":
        check_full_report(Path(sys.argv[2]))
    else:
        unittest.main(verbosity=2)
