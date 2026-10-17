"""Policy primitives; caller must supply independently trusted snapshot hashes."""
import argparse
import copy
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest

from inventory import InventoryError, canonical, checked_bytes, digest, load, safe_path, validate


def snapshots(inventory, policy, inventory_hash, policy_hash, environment):
    if any(name.startswith("ART_REG_") for name in environment):
        raise InventoryError("forbidden upstream override")
    if digest(canonical(inventory)) != inventory_hash or digest(canonical(policy)) != policy_hash:
        raise InventoryError("protected snapshot differs")
    validate(inventory)
    if policy["schema"] != 1 or policy["selector"] != "all-active-inventory-cases":
        raise InventoryError("unsupported policy schema/selector")
    if policy["comparison"] != {"max_channel_delta": 0, "max_changed_pixels": 0, "max_alpha_delta": 0}:
        raise InventoryError("tolerance requires separately approved policy implementation")


def expected_results(inventory, policy):
    keys = set()
    for case in inventory["cases"]:
        if case["kind"] == "disabled-upstream":
            continue
        frames = [frame["expected"] for frame in case["frames"]] or [None]
        for frame in frames:
            for profile in policy["profiles"]:
                keys.add((case["id"], frame, profile))
    return keys


def results(inventory, policy, records):
    remaining = expected_results(inventory, policy)
    total = len(remaining)
    counts = {status: 0 for status in policy["allowed_results"]}
    for record in records:
        key = (record["case"], record["frame"], record["profile"])
        if key not in remaining:
            raise InventoryError("duplicate/unselected result")
        remaining.remove(key)
        status = record["status"]
        if status not in counts:
            raise InventoryError("invalid result status")
        # No approved exceptions or renderer support promotions exist yet.
        if status in {"PASS_APPROVED_TOLERANCE", "EXPECTED_DIFFERENCE"}:
            raise InventoryError("unapproved result exception")
        if status == "UNSUPPORTED" and not record.get("feature"):
            raise InventoryError("unsupported requires feature ID")
        counts[status] += 1
    if remaining:
        raise InventoryError("missing cases: " + str(len(remaining)))
    return {"total": total, "counts": counts, "disabled_upstream": sum(case["kind"] == "disabled-upstream" for case in inventory["cases"])}


class PolicyTests(unittest.TestCase):
    def setUp(self):
        # Synthetic policy fixtures. External corpus bytes are never mutated.
        from inventory import pin
        self.inventory = {"schema": 1, "commit": pin()["commit"], "assets": [{"path": "test.ass", "sha256": "0" * 64}, {"path": "test-0000.png", "sha256": "1" * 64}], "cases": [{"id": "test.ass", "source": "test.ass", "kind": "regression", "fonts": [], "frames": [{"expected": "test-0000.png"}], "support": "UNSUPPORTED"}]}
        self.policy = load(Path(__file__).with_name("policy.json"))
        self.inv_hash = digest(canonical(self.inventory))
        self.policy_hash = digest(canonical(self.policy))

    def check(self, inventory=None, policy=None, environment=None):
        snapshots(self.inventory if inventory is None else inventory, self.policy if policy is None else policy, self.inv_hash, self.policy_hash, {} if environment is None else environment)

    def test_clean(self):
        self.check()

    def test_golden_tamper(self):
        altered = copy.deepcopy(self.inventory)
        altered["assets"][1]["sha256"] = "2" * 64
        with self.assertRaises(InventoryError): self.check(inventory=altered)

    def test_threshold_tamper(self):
        altered = copy.deepcopy(self.policy)
        altered["comparison"]["max_channel_delta"] = 255
        with self.assertRaises(InventoryError): self.check(policy=altered)

    def test_support_tamper(self):
        altered = copy.deepcopy(self.inventory)
        altered["cases"][0]["support"] = "SUPPORTED"
        with self.assertRaises(InventoryError): self.check(inventory=altered)

    def test_selector_tamper(self):
        altered = copy.deepcopy(self.policy)
        altered["selector"] = "none"
        with self.assertRaises(InventoryError): self.check(policy=altered)

    def test_missing_case(self):
        altered = copy.deepcopy(self.inventory)
        altered["cases"] = []
        with self.assertRaises(InventoryError): self.check(inventory=altered)

    def test_missing_expected(self):
        altered = copy.deepcopy(self.inventory)
        altered["assets"].pop()
        with self.assertRaises(InventoryError): validate(altered)

    def test_override_forbidden(self):
        for name in ["ART_REG_GEN", "ART_REG_SKIP", "ART_REG_TOLERANCE", "ART_REG_FUTURE"]:
            with self.assertRaises(InventoryError): self.check(environment={name: "1"})

    def test_denominator_and_duplicate_missing_results(self):
        records = [{"case": case, "frame": frame, "profile": profile, "status": "UNSUPPORTED", "feature": "T014"} for case, frame, profile in sorted(expected_results(self.inventory, self.policy))]
        report = results(self.inventory, self.policy, records)
        self.assertEqual(report["total"], 2)
        self.assertEqual(report["counts"]["UNSUPPORTED"], 2)
        for altered in [records[:-1], records + records[:1]]:
            with self.assertRaises(InventoryError): results(self.inventory, self.policy, altered)

    def test_paths_and_symlinks(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "font").write_bytes(b"synthetic font placeholder")
            self.assertEqual(safe_path(root, "font"), root / "font")
            for path in ["../escape", "/escape", "a\\b", "missing"]:
                with self.assertRaises(InventoryError): safe_path(root, path)
            try:
                (root / "escape").symlink_to(root.parent)
            except (OSError, NotImplementedError):
                self.fail("symlink safety test requires runner symlink support")
            with self.assertRaises(InventoryError): safe_path(root, "escape")
            (root / "local").symlink_to("font")
            self.assertEqual(safe_path(root, "local").resolve(), root / "font")

    def test_duplicate_json_keys(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "duplicate.json"
            path.write_text('{"schema":1,"schema":2}')
            with self.assertRaises(InventoryError): load(path)

    def test_asset_mutation_and_missing_file(self):
        import hashlib
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / "expected.png"
            original = b"independently authored synthetic expected bytes"
            path.write_bytes(original)
            blob = hashlib.sha1(b"blob " + str(len(original)).encode() + b"\0" + original).hexdigest()
            self.assertEqual(checked_bytes(root, "expected.png", "100644", blob)[1], original)
            path.write_bytes(original + b"tamper")
            with self.assertRaises(InventoryError): checked_bytes(root, "expected.png", "100644", blob)
            path.unlink()
            with self.assertRaises(InventoryError): checked_bytes(root, "expected.png", "100644", blob)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["self-test", "check"])
    parser.add_argument("--inventory", type=Path)
    parser.add_argument("--policy", type=Path)
    parser.add_argument("--trusted-inventory-sha256")
    parser.add_argument("--trusted-policy-sha256")
    args = parser.parse_args()
    if args.action == "self-test":
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(PolicyTests)
        if not unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful():
            sys.exit(1)
    else:
        if not all([args.inventory, args.policy, args.trusted_inventory_sha256, args.trusted_policy_sha256]):
            parser.error("check requires inventory, policy and externally trusted hashes")
        snapshots(load(args.inventory), load(args.policy), args.trusted_inventory_sha256, args.trusted_policy_sha256, os.environ)
        print("SNAPSHOTS_MATCH: hash provenance/trusted issuer must be checked separately")


if __name__ == "__main__":
    try:
        main()
    except (InventoryError, OSError, ValueError, KeyError) as error:
        print("INFRA_ERROR: " + str(error), file=sys.stderr)
        sys.exit(1)
