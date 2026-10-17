"""Conservative source-job selection; unknown paths require the source suite."""

import os
from pathlib import PurePosixPath
import re
import subprocess
import sys
import unittest


def needs_source(paths):
    for path in paths:
        if path in {"README.md", "AGENTS.md"}:
            continue
        if path.startswith("docs/") and PurePosixPath(path).suffix == ".md":
            continue
        return True
    return False


class SelectionTests(unittest.TestCase):
    def test_documentation_only(self):
        self.assertFalse(needs_source(["README.md", "docs/tasks/T001.md"]))
        self.assertFalse(needs_source(["AGENTS.md", "docs/decisions/new.md"]))

    def test_mixed_change_cannot_hide_source(self):
        self.assertTrue(needs_source(["README.md", "crates/taiyou-types/src/lib.rs"]))

    def test_policy_and_unknown_paths_are_conservative(self):
        for path in [
            ".github/workflows/fast.yml",
            "scripts/select-source.py",
            "scripts/check-workspace.ps1",
            "Cargo.toml",
            "Cargo.lock",
            "rust-toolchain.toml",
            "docs/references.lock.json",
            "docs/generated.rs",
            "new-directory/unknown",
            "docs/not-markdown",
            "crates/README.md",
            "odd\nname.rs",
        ]:
            with self.subTest(path=path):
                self.assertTrue(needs_source([path]))


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: select-source.py BASE_SHA HEAD_SHA | --self-test")
    base, head = sys.argv[1:]
    # A new branch push has no before SHA. Validate broadly in that case.
    if base == "0" * 40 or not base:
        selected = True
    else:
        if not all(re.fullmatch(r"[0-9a-f]{40}", sha) for sha in [base, head]):
            raise SystemExit("expected full Git commit SHAs")
        output = subprocess.check_output(
            ["git", "diff", "--name-only", "--no-renames", "-z", base, head, "--"]
        )
        # NUL separation includes deleted paths and both sides of renames safely.
        paths = output.decode("utf-8", errors="surrogateescape").split("\0")
        selected = needs_source([path for path in paths if path])
    value = str(selected).lower()
    print(f"source={value}")
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"source={value}\n")


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        unittest.main(argv=[sys.argv[0]])
    else:
        main()
