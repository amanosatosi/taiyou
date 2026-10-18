"""Solo-maintainer accidental-change guard; not an independent trust authority."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import unittest
import urllib.request

BOOTSTRAP = "39f6ce2fa64b231fd705862d853c8df239b54a22"
REPOSITORY = "amanosatosi/taiyou"
OWNER = "amanosatosi"
PREFIXES = ("validation/", ".github/", "tests/taiyou/", "docs/decisions/")
FILES = {"AGENTS.md", "docs/references.lock.json", "docs/TESTING.md",
         "docs/CONTRACTS.md", "docs/COMPATIBILITY.md", "docs/CACHING.md",
         "docs/DEVELOPMENT.md", "docs/tasks/T005.md", "docs/tasks/T015.md"}


def protected(path):
    return path in FILES or path.startswith(PREFIXES) or (
        path.startswith("scripts/") and Path(path).name.startswith(("check-", "select-")))


def git(*args):
    return subprocess.check_output(["git", *args])


def blob(revision, path):
    entries = git("ls-tree", "-z", revision, "--", path)
    if not entries:
        return None
    return git("show", f"{revision}:{path}")


def changes(base, head):
    paths = git("diff", "--name-only", "--no-renames", "-z", base, head, "--").decode().split("\0")
    records = []
    for path in sorted(p for p in paths if p and protected(p)):
        record = {"path": path}
        for field, revision in [("before", base), ("after", head)]:
            data = blob(revision, path)
            record[field] = None if data is None else hashlib.sha256(data).hexdigest()
            record[field + "_tree"] = git("ls-tree", "-z", revision, "--", path).hex()
        records.append(record)
    digest = hashlib.sha256(json.dumps(records, sort_keys=True, separators=(",", ":")).encode()).hexdigest()
    return records, digest


def acknowledgment(head, base, digest):
    return f"TAIYOU_BASELINE_ACK {head} {base} {digest}"


def acknowledged(comments, expected):
    return any(c.get("user", {}).get("login") == OWNER
               and c.get("user", {}).get("type") == "User"
               and c.get("author_association") == "OWNER"
               and c.get("body", "").strip() == expected for c in comments)


def comments(number):
    headers = {"Accept": "application/vnd.github+json", "User-Agent": "Taiyou-baseline-guard"}
    token = os.environ.get("GH_TOKEN")
    if token:
        headers["Authorization"] = "Bearer " + token
    for page in range(1, 11):
        url = f"https://api.github.com/repos/{REPOSITORY}/issues/{number}/comments?per_page=100&page={page}"
        with urllib.request.urlopen(urllib.request.Request(url, headers=headers), timeout=30) as response:
            raw = response.read(8 * 1024 * 1024 + 1)
        if len(raw) > 8 * 1024 * 1024:
            raise ValueError("comment response exceeds budget")
        batch = json.loads(raw)
        if not isinstance(batch, list):
            raise ValueError("invalid comments response")
        yield from batch
        if len(batch) < 100:
            return
    raise ValueError("comment pagination exceeds budget")


def bootstrap(base, head, event):
    if base != BOOTSTRAP or event.get("number") != 11 or event.get("repository", {}).get("full_name") != REPOSITORY:
        raise ValueError("bootstrap is restricted to reviewed T005 PR #11")
    for path in ["validation/inventory.json", "docs/references.lock.json"]:
        if blob(base, path) != blob(head, path):
            raise ValueError("bootstrap cannot change frozen assets/reference pin")
    old = json.loads(blob(base, "validation/policy.json"))
    new = json.loads(blob(head, "validation/policy.json"))
    # Owner-authorized governance amendment only; regression fields stay exact.
    for key in ["state", "governance"]:
        old.pop(key, None)
        new.pop(key, None)
    if json.dumps(old, sort_keys=True) != json.dumps(new, sort_keys=True):
        raise ValueError("bootstrap cannot change regression policy")


class BaselineTests(unittest.TestCase):
    def test_real_git_change_accounting_and_digest(self):
        from unittest.mock import patch
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def command(*args):
                return subprocess.check_output(["git", *args], cwd=root, stderr=subprocess.STDOUT)
            command("init")
            command("config", "user.name", "Synthetic fixture")
            command("config", "user.email", "synthetic@example.invalid")
            (root / "validation").mkdir()
            (root / "validation/inventory.json").write_bytes(b"original synthetic inventory")
            (root / "source.rs").write_bytes(b"ordinary implementation")
            command("add", ".")
            command("commit", "-m", "synthetic baseline")
            base = command("rev-parse", "HEAD").decode().strip()
            (root / "validation/inventory.json").unlink()
            (root / "validation/policy.json").write_bytes(b"proposed policy")
            (root / "source.rs").write_bytes(b"changed ordinary implementation")
            command("add", "-A")
            command("commit", "-m", "synthetic proposal")
            head = command("rev-parse", "HEAD").decode().strip()
            with patch(__name__ + ".git", side_effect=command):
                records, digest = changes(base, head)
                self.assertEqual([r["path"] for r in records], ["validation/inventory.json", "validation/policy.json"])
                self.assertIsNone(records[0]["after"])
                self.assertIsNone(records[1]["before"])
                self.assertEqual(changes(base, head), (records, digest))
                self.assertNotEqual(changes(head, base)[1], digest)

    def test_required_workflow_is_unfiltered_and_uses_accepted_verifier(self):
        workflow = Path(".github/workflows/inventory.yml").read_text()
        self.assertNotRegex(workflow, r"(?m)^\s+paths(?:-ignore)?:")
        self.assertIn("name: Corpus policy gate", workflow)
        self.assertIn('git show "$accepted:validation/baseline.py"', workflow)
        self.assertGreaterEqual(workflow.count('python "$TAIYOU_ACCEPTED/validation/inventory.py" verify'), 2)
        self.assertIn("if: always()", workflow)

    def test_surface_selection(self):
        for path in ["validation/inventory.json", "validation/new-policy.json", ".github/workflows/inventory.yml",
                     "docs/references.lock.json", "tests/taiyou/new.expected.png", "scripts/select-source.py"]:
            self.assertTrue(protected(path))
        self.assertFalse(protected("crates/taiyou-render/src/renderer.rs"))

    def test_owner_exact_commit_acknowledgment(self):
        expected = acknowledgment("a" * 40, "b" * 40, "c" * 64)
        good = {"user": {"login": OWNER, "type": "User"}, "author_association": "OWNER", "body": expected}
        self.assertTrue(acknowledged([good], expected))
        for altered in [{**good, "body": acknowledgment("d" * 40, "b" * 40, "c" * 64)},
                        {**good, "body": acknowledgment("a" * 40, "d" * 40, "c" * 64)},
                        {**good, "body": acknowledgment("a" * 40, "b" * 40, "d" * 64)},
                        {**good, "user": {"login": "not-the-owner", "type": "User"}},
                        {**good, "user": {"login": OWNER, "type": "Bot"}},
                        {**good, "author_association": "CONTRIBUTOR"}]:
            self.assertFalse(acknowledged([altered], expected))
        self.assertFalse(acknowledged([], expected))

    def test_bootstrap_rejects_wrong_pr_and_regression_changes(self):
        from unittest.mock import patch
        event = {"number": 11, "repository": {"full_name": REPOSITORY}}
        original = {"comparison": {"max_channel_delta": 0}, "selector": "all", "state": "old"}
        def snapshots(revision, path):
            if path == "validation/policy.json":
                value = original if revision == BOOTSTRAP else {**original, "state": "solo", "governance": {"model": "solo-maintainer"}}
                return json.dumps(value).encode()
            return b"frozen"
        with patch(__name__ + ".blob", side_effect=snapshots):
            bootstrap(BOOTSTRAP, "a" * 40, event)
            with self.assertRaises(ValueError):
                bootstrap(BOOTSTRAP, "a" * 40, {**event, "number": 12})
        for path in ["validation/inventory.json", "docs/references.lock.json", "validation/policy.json"]:
            def changed(revision, name):
                if revision != BOOTSTRAP and name == path:
                    return b'{}'
                return snapshots(revision, name)
            with patch(__name__ + ".blob", side_effect=changed), self.assertRaises(ValueError):
                bootstrap(BOOTSTRAP, "a" * 40, event)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("action", choices=["check", "self-test"])
    parser.add_argument("--base")
    parser.add_argument("--head")
    parser.add_argument("--bootstrap", action="store_true")
    args = parser.parse_args()
    if args.action == "self-test":
        if not unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(BaselineTests)).wasSuccessful():
            raise ValueError("baseline self-tests failed")
        return
    if not all(re.fullmatch("[0-9a-f]{40}", s or "") for s in [args.base, args.head]):
        raise ValueError("full base/head commit SHAs required")
    if any(k.startswith("ART_REG_") for k in os.environ):
        raise ValueError("forbidden upstream override")
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
    records, digest = changes(args.base, args.head)
    expected = acknowledgment(args.head, args.base, digest)
    print(json.dumps({"accepted_baseline": args.base, "candidate": args.head, "changes": records,
                      "acknowledgment": expected, "independently_trusted": False,
                      "solo_maintainer_accepted": False}), flush=True)
    if args.bootstrap:
        bootstrap(args.base, args.head, event)
        print("BOOTSTRAP: owner-authorized ADR-011 scope; final candidate acceptance pending")
    elif records:
        number = event.get("number")
        if not isinstance(number, int) or not acknowledged(comments(number), expected):
            raise ValueError("protected-by-policy change needs exact owner acknowledgment: " + expected)
        print("ACKNOWLEDGED_PROPOSAL: accepted baseline remains the comparison source")
    else:
        print("BASELINE_UNCHANGED: no acknowledgment needed")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print("BASELINE_ERROR: " + str(error), file=sys.stderr)
        sys.exit(1)
