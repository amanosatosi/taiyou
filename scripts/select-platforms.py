"""Conservative platform selection: only known prose-only changes skip builds."""
import os
from pathlib import PurePosixPath
import re
import subprocess
import sys
import unittest


def needs_platforms(paths):
    for path in paths:
        if path in {"docs/DEPENDENCIES.md", "THIRD_PARTY_NOTICES.md"}:
            return True
        if path in {"README.md", "AGENTS.md"}:
            continue
        if path.startswith("docs/") and PurePosixPath(path).suffix == ".md":
            continue
        return True
    return False


class PlatformSelectionTests(unittest.TestCase):
    def test_font_ffi_native_and_manifests_select_all_platforms(self):
        for path in ["crates/taiyou-font/src/native.rs", "crates/taiyou-font/Cargo.toml", "crates/taiyou-capi/src/lib.rs", "crates/taiyou-types/src/scene.rs", "crates/taiyou-render/src/raster.rs", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "build-support/build-native.py", "build-support/sources.lock.json", "native/future.cc", "future/ffi.h", "future/new_adapter.rs"]:
            with self.subTest(path=path):
                self.assertTrue(needs_platforms([path]))

    def test_policy_and_unknown_changes_select_all_platforms(self):
        for path in [".github/workflows/native-deps.yml", "scripts/select-platforms.py", "docs/references.lock.json", "unknown/path", "odd\nname.rs", "docs/DEPENDENCIES.md", "THIRD_PARTY_NOTICES.md"]:
            with self.subTest(path=path):
                self.assertTrue(needs_platforms([path]))

    def test_only_known_prose_can_skip(self):
        self.assertFalse(needs_platforms(["README.md", "docs/tasks/T008.md"]))
        self.assertTrue(needs_platforms(["docs/tasks/T008.md", "crates/taiyou-font/src/adapter.rs"]))

    def test_workflow_has_no_trigger_path_filter(self):
        from pathlib import Path
        workflow = Path(__file__).resolve().parent.parent / ".github/workflows/native-deps.yml"
        self.assertNotRegex(workflow.read_text(), r"(?m)^\s+paths(?:-ignore)?:")


def main():
    if len(sys.argv) != 3:
        raise SystemExit("usage: select-platforms.py BASE_SHA HEAD_SHA | --self-test")
    base, head = sys.argv[1:]
    if not base or base == "0" * 40:
        selected = True
    else:
        if not all(re.fullmatch("[0-9a-f]{40}", sha) for sha in [base, head]):
            raise SystemExit("expected full Git commit SHAs")
        paths = subprocess.check_output(["git", "diff", "--name-only", "--no-renames", "-z", base, head, "--"])
        selected = needs_platforms([path for path in paths.decode("utf-8", errors="surrogateescape").split("\0") if path])
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write("native=" + str(selected).lower() + "\n")
    print("native=" + str(selected).lower())


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        unittest.main(argv=[sys.argv[0]])
    else:
        main()
