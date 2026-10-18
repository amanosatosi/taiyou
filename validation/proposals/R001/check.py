"""Check this source-only proposal; never import captures or promote support.

Run on Actions. These checks certify internal consistency, not historical pixels
or an archive's upstream commit identity. No external/native dependency is used.
"""

import copy
import hashlib
import json
from pathlib import Path
import re
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def load():
    def read(path):
        return json.loads((ROOT / path).read_text(encoding="utf-8"))
    return (
        read("validation/legacy-rules.json"),
        read("validation/proposals/R001/fixtures.json"),
        read("validation/proposals/R001/sources.json"),
    )


def validate(inventory, proposals, sources):
    require(inventory["schema"] == proposals["schema"] == sources["schema"] == 1, "schema")
    require(inventory["profile"] == proposals["profile"] == "LegacyXyV1", "profile")
    require(inventory["state"] == "source-research-checkpoint", "checkpoint state")
    require(inventory["maintainer_accepted"] is False, "fabricated acceptance")
    require(inventory["enabled_capabilities"] == [], "capability admission forbidden")
    require(inventory["integration_base"] == "4204f3d2a8e9734a6bad81b4716e9b7f74b388fe", "base")
    require(inventory["source_manifest"] == "validation/proposals/R001/sources.json", "source manifest")
    require(inventory["fixture_manifest"] == "validation/proposals/R001/fixtures.json", "fixture manifest")
    environment = proposals["environment"]
    require(environment["id"] == "xy-environment-pending" and environment["status"] == "unavailable", "environment")
    require(all(value is None for key, value in environment.items() if key not in {"id", "status"}), "invented environment")
    source_ids = {source["id"] for source in sources["sources"]}
    require(len(source_ids) == len(sources["sources"]), "duplicate source identity")
    for source in sources["sources"]:
        require(source["verified_commit"] is None and source["archive_container_sha256"] is None, "unverified source identity claim")
        require(source["git_metadata_present"] is False, "invented Git identity")
    primary = next(s for s in sources["sources"] if s["id"] == "pinterf-local")
    require(primary["repository"] == "https://github.com/pinterf/xy-VSFilter", "wrong xy authority")
    require(primary["pin_equivalence"] == "unverified", "archive pin promotion")
    file_map = {}
    for source in sources["files"]:
        key = (source["source_id"], source["file"])
        require(source["source_id"] in source_ids and key not in file_map, "duplicate/unknown source file")
        require(re.fullmatch(r"[0-9a-f]{64}", source["sha256"]) and source["line_count"] > 0, "source fingerprint")
        file_map[key] = source
    local = json.loads((ROOT / "docs/local-sources.json").read_text(encoding="utf-8"))
    aliases = {"xy-vsfilter": "pinterf-local", "original-vsfilter": "guliverkli2-local", "libass": "libass-local"}
    for source in local["sources"]:
        for entry in source["files"]:
            key = (aliases.get(source["id"]), entry["path"])
            if key in file_map:
                require(file_map[key]["sha256"] == entry["sha256"], "known local fingerprint changed")
    rule_ids = set()
    all_fixture_ids = set()
    counts = {"rules": len(inventory["rules"]), "source-established": 0, "capture-verified": 0, "provisional": 0, "blocked": 0, "approved": 0}
    rules = {}
    for rule in inventory["rules"]:
        require(rule["id"] not in rule_ids, "duplicate rule")
        rule_ids.add(rule["id"])
        rules[rule["id"]] = rule
        require(rule["approved"] is False and rule["enabled"] is False, "unverified rule promoted")
        evidence = rule["evidence_type"]
        require(evidence in {"source-established", "provisional", "blocked"}, "unsupported capture evidence")
        require(rule["decision"] == ("blocked" if evidence == "blocked" else "provisional"), "decision disagrees")
        counts[evidence] += 1
        require(rule["core_v1"]["executed"] is False and rule["core_v1"]["assessment"], "fabricated Core execution")
        require(rule["libass"]["execution"] == "not-run-on-this-proposal" and rule["libass"]["result"] is None, "fabricated libass result")
        require(rule["libass"]["pinned_commit"] == "f61db567e6593df3470e91594bcd4ad2d0473aff", "libass pin")
        capture = rule["capture"]
        require(capture == {"status": "pending", "environment_id": environment["id"], "output": None, "sha256": None}, "invalid capture claim")
        require(all(rule[field] for field in ("title", "family", "alternatives", "expected_xy", "confidence", "follow_up", "dependent_tasks", "source", "fixtures")), "incomplete rule")
        for evidence_source in rule["source"]:
            key = (evidence_source["source_id"], evidence_source["file"])
            require(key in file_map and key[0] == "pinterf-local", "missing/non-primary rule source")
            source = file_map[key]
            require(evidence_source["sha256"] == source["sha256"], "source hash mismatch")
            a, b = evidence_source["lines"]
            require(1 <= a <= b <= source["line_count"] and evidence_source["function"], "invalid source location")
        for fixture in rule["fixtures"]:
            require(fixture not in all_fixture_ids, "fixture reused/duplicate")
            all_fixture_ids.add(fixture)
    require(counts == inventory["counts"], "counts disagree")
    require(counts == {"rules": 30, "source-established": 25, "capture-verified": 0, "provisional": 2, "blocked": 3, "approved": 0}, "checkpoint totals")
    seen = set()
    scripts = set()
    for fixture in proposals["fixtures"]:
        require(fixture["id"] not in seen and fixture["id"] in all_fixture_ids, "duplicate/orphan fixture")
        seen.add(fixture["id"])
        rule = rules[fixture["rule"]]
        require(fixture["id"] in rule["fixtures"] and fixture["family"] == rule["family"], "fixture mapping")
        require(fixture["expected_observable"] == rule["expected_xy"] and fixture["source"] == rule["source"], "fixture evidence disagrees")
        require(fixture["approval"] == "pending" and fixture["capture_status"] == "not-obtained" and fixture["expected_image"] is None, "fixture promotion")
        require(fixture["environment_id"] == environment["id"], "fixture environment")
        require(fixture["timestamps_us"] and all(type(t) is int and t >= 0 for t in fixture["timestamps_us"]), "timestamps")
        require(len(fixture["viewport"]) == 2 and all(type(d) is int and d > 0 for d in fixture["viewport"]), "dimensions")
        path = fixture["script"]
        require(path.startswith("validation/proposals/R001/scripts/") and ".." not in Path(path).parts, "script escapes proposal")
        require(path not in scripts and Path(path).suffix in {".ass", ".ssa"}, "duplicate/invalid script")
        scripts.add(path)
        data = (ROOT / path).read_bytes()
        require(hashlib.sha256(data).hexdigest() == fixture["sha256"], "script bytes changed")
        require(data.startswith(b"\xef\xbb\xbf") and b"\r" not in data and "[Events]" in data.decode("utf-8-sig"), "script encoding/structure")
    require(seen == all_fixture_ids and len(seen) == 34, "missing fixture")
    actual = {str(p.relative_to(ROOT)).replace("\\", "/") for p in (ROOT / "validation/proposals/R001/scripts").iterdir()}
    require(actual == scripts, "unlisted script")
    doc = (ROOT / "docs/research/R001-legacy.md").read_text(encoding="utf-8")
    rows = re.findall(r"^\| \[(R001-[A-Z][0-9]{2}):", doc, re.M)
    require(len(rows) == len(rule_ids) and set(rows) == rule_ids, "document rule table disagrees")
    require("25 source-established, 2 provisional interpretations, 3 blocked" in doc, "document counts")
    return counts


class RejectedEvidence(unittest.TestCase):
    def setUp(self):
        self.inventory, self.fixtures, self.sources = copy.deepcopy(load())

    def reject(self):
        with self.assertRaises(ValueError):
            validate(self.inventory, self.fixtures, self.sources)

    def test_no_approval_without_capture(self):
        self.inventory["rules"][0]["approved"] = True
        self.reject()

    def test_no_silent_enable(self):
        self.inventory["enabled_capabilities"] = ["R001-P01"]
        self.reject()

    def test_no_fabricated_capture(self):
        self.inventory["rules"][0]["evidence_type"] = "capture-verified"
        self.reject()

    def test_no_unknown_source_revision(self):
        self.sources["sources"][0]["verified_commit"] = "25cf3e8433b3a83a5489e47a2c335516492bb815"
        self.reject()

    def test_no_missing_source(self):
        self.inventory["rules"][0]["source"] = []
        self.reject()

    def test_no_invalid_source_lines(self):
        self.inventory["rules"][0]["source"][0]["lines"] = [0, 999999]
        self.reject()

    def test_no_duplicate_rules(self):
        self.inventory["rules"].append(copy.deepcopy(self.inventory["rules"][0]))
        self.reject()

    def test_no_dropped_fixture(self):
        self.fixtures["fixtures"].pop()
        self.reject()

    def test_no_changed_script_hash(self):
        self.fixtures["fixtures"][0]["sha256"] = "0" * 64
        self.reject()

    def test_no_invented_font_environment(self):
        self.fixtures["environment"]["font_files"] = [{"name": "Arial", "sha256": "0" * 64}]
        self.reject()

    def test_no_fabricated_libass_result(self):
        self.inventory["rules"][0]["libass"]["result"] = "PASS_EXACT"
        self.reject()


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        unittest.main(argv=[sys.argv[0]])
    elif sys.argv[1:]:
        raise SystemExit("usage: check.py [--self-test]")
    else:
        print("R001 proposal consistency (not capture verification):", validate(*load()))
