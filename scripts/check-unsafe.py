"""Candidate-side unsafe boundary checks. Not independent GitHub protection.

Run only on Actions. Compiler forbid/deny and warnings-denied Clippy still run.
"""
import copy
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys
import tomllib
import unittest

BOUNDARY = "crates/taiyou-font/src/native.rs"
MEMBERS = ["crates/taiyou-" + name for name in ["types", "ass", "font", "render", "capi", "cli"]] + ["xtask"]


def rust_tokens(source):
    """Skip strings/comments, including nested comments and raw strings.

    This is deliberately a conservative policy scanner, not a Rust parser.
    Rust compilation remains responsible for syntax and macro/type checking.
    """
    tokens = []
    i = 0
    while i < len(source):
        if source[i].isspace():
            i += 1
        elif source.startswith("//", i):
            end = source.find("\n", i)
            i = len(source) if end < 0 else end + 1
        elif source.startswith("/*", i):
            depth = 1
            i += 2
            while depth:
                if i >= len(source):
                    raise ValueError("unterminated Rust block comment")
                if source.startswith("/*", i):
                    depth += 1
                    i += 2
                elif source.startswith("*/", i):
                    depth -= 1
                    i += 2
                else:
                    i += 1
        elif raw := re.match(r'(?:br|cr|r)(#*)"', source[i:]):
            end_marker = '"' + raw[1]
            end = source.find(end_marker, i + raw.end())
            if end < 0:
                raise ValueError("unterminated Rust raw string")
            i = end + len(end_marker)
        elif source[i] == '"':
            i += 1
            while i < len(source) and source[i] != '"':
                i += 2 if source[i] == "\\" else 1
            if i >= len(source):
                raise ValueError("unterminated Rust string")
            i += 1
        elif char := re.match(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'", source[i:]):
            i += char.end()
        elif word := re.match(r"(?:r#)?[A-Za-z_][A-Za-z_0-9]*", source[i:]):
            tokens.append(word[0].removeprefix("r#"))
            i += word.end()
        else:
            tokens.append(source[i])
            i += 1
    return tokens


def check_sources(sources):
    for path, source in sources.items():
        tokens = rust_tokens(source)
        if path == BOUNDARY:
            exception = ["#", "!", "[", "allow", "(", "unsafe_code", ")", "]"]
            if tokens[:len(exception)] != exception or tokens.count("unsafe_code") != 1:
                raise ValueError("native boundary must have exactly its designated module exception")
        elif "unsafe_code" in tokens:
            raise ValueError(f"unauthorized unsafe lint exception/reference: {path}")
        production = path.startswith(("crates/", "xtask/")) or path == "build.rs"
        if production:
            if path != BOUNDARY and "unsafe" in tokens:
                raise ValueError(f"unsafe code outside native boundary: {path}")
            # Do not inherit the boundary's allow through included/generated code,
            # custom source paths, or another workspace build/proc-macro script.
            for index, token in enumerate(tokens[:-1]):
                if token == "include" and tokens[index + 1] == "!":
                    raise ValueError(f"code include requires boundary policy review: {path}")
                if token == "path" and tokens[index + 1] == "=":
                    raise ValueError(f"custom module path requires boundary policy review: {path}")
            if path.endswith("/build.rs") or path == "build.rs":
                raise ValueError("workspace build scripts require boundary policy review")


def check_manifests(manifests):
    workspace = manifests["Cargo.toml"]
    if workspace.get("workspace", {}).get("lints", {}).get("rust", {}).get("unsafe_code") != "forbid":
        raise ValueError("workspace unsafe_code must remain forbid")
    for member in MEMBERS:
        manifest = manifests[member + "/Cargo.toml"]
        expected = {"rust": {"unsafe_code": "deny"}} if member == "crates/taiyou-font" else {"workspace": True}
        if manifest.get("lints") != expected:
            raise ValueError(f"unauthorized lint manifest override: {member}")
        if manifest.get("package", {}).get("build", False) is not False:
            raise ValueError("workspace build scripts require boundary policy review")
        library = manifest.get("lib", {})
        if library.get("proc-macro", False) or "path" in library:
            raise ValueError("custom library/proc-macro entry requires boundary policy review")
        for target in ["bin", "test", "example", "bench"]:
            if any("path" in entry for entry in manifest.get(target, [])):
                raise ValueError("custom target path requires boundary policy review")


class BoundaryTests(unittest.TestCase):
    def test_only_exact_native_exception(self):
        check_sources({BOUNDARY: "//! owner\n#![allow(unsafe_code)]\nunsafe fn f() {}"})
        for path in ["crates/taiyou-font/src/lib.rs", "crates/taiyou-font/src/catalog.rs", "crates/taiyou-font/tests/escape.rs", "xtask/src/main.rs", "build-support/extra.rs"]:
            for attribute in ["#![allow(unsafe_code)]", "#[expect(unsafe_code)]", "#![cfg_attr(feature=\"x\", allow(unsafe_code))]", "#![allow(r#unsafe_code)]", "#![allow(/*nested /* x */ y */ unsafe_code)]"]:
                with self.subTest(path=path, attribute=attribute), self.assertRaises(ValueError):
                    check_sources({path: attribute})

    def test_inherited_or_generated_escape_rejected(self):
        for text in ["unsafe fn f() {}", "mod x { unsafe fn f() {} }", "include!(concat!(env!(\"OUT_DIR\"),\"/x.rs\"));", "#[path=\"../../outside.rs\"] mod x;", "macro_rules! x { () => { unsafe {} } }", "macro_rules! x { () => { #[allow(unsafe_code)] fn f() {} } }"]:
            with self.subTest(text=text), self.assertRaises(ValueError):
                check_sources({"crates/taiyou-font/src/catalog.rs": text})
        with self.assertRaises(ValueError):
            check_sources({BOUNDARY: "#![allow(unsafe_code)]\ninclude!(\"extra.rs\");"})
        with self.assertRaises(ValueError):
            check_sources({"crates/taiyou-font/build.rs": "fn main() {}"})

    def test_boundary_cannot_change_exception(self):
        for text in ["unsafe fn f() {}", "#![cfg_attr(any(),allow(unsafe_code))]", "#![allow(unsafe_code)]\n#[allow(unsafe_code)] fn f() {}"]:
            with self.subTest(text=text), self.assertRaises(ValueError):
                check_sources({BOUNDARY: text})

    def test_comments_strings_and_lifetimes_are_not_executable_code(self):
        text = "/* nested /* unsafe */ comment */ // unsafe\nlet x = r###\"unsafe #![allow(unsafe_code)]\"###; let y = \"unsafe\"; fn f<'a>(x: &'a str) { let c = '\\u{61}'; }"
        check_sources({"crates/taiyou-font/src/catalog.rs": text})

    def test_manifest_exceptions_rejected(self):
        manifests = {"Cargo.toml": {"workspace": {"lints": {"rust": {"unsafe_code": "forbid"}}}}}
        for member in MEMBERS:
            manifests[member + "/Cargo.toml"] = {"lints": {"rust": {"unsafe_code": "deny"}} if member == "crates/taiyou-font" else {"workspace": True}}
        check_manifests(manifests)
        for member in MEMBERS:
            for lints in [{}, {"rust": {"unsafe_code": "allow"}}, {"rust": {"unsafe_code": {"level": "deny", "priority": -1}}}]:
                changed = copy.deepcopy(manifests)
                changed[member + "/Cargo.toml"]["lints"] = lints
                with self.subTest(member=member, lints=lints), self.assertRaises(ValueError):
                    check_manifests(changed)
        changed = copy.deepcopy(manifests)
        changed["Cargo.toml"]["workspace"]["lints"]["rust"]["unsafe_code"] = "deny"
        with self.assertRaises(ValueError):
            check_manifests(changed)
        for section in [{"package": {"build": "../script.py"}}, {"lib": {"proc-macro": True}}, {"lib": {"path": "../outside.rs"}}, {"test": [{"path": "../outside.rs"}]}]:
            changed = copy.deepcopy(manifests)
            changed["crates/taiyou-font/Cargo.toml"].update(section)
            with self.subTest(section=section), self.assertRaises(ValueError):
                check_manifests(changed)


def main():
    root = Path(__file__).resolve().parent.parent
    paths = subprocess.check_output(["git", "ls-files", "-z"], cwd=root).decode("utf-8").split("\0")
    rust = {}
    for path in paths:
        file = root / path
        if PurePosixPath(path).suffix == ".rs":
            if file.is_symlink():
                raise ValueError("Rust source symlinks require policy review")
            rust[path] = file.read_text(encoding="utf-8")
    manifests = {path: tomllib.loads((root / path).read_text(encoding="utf-8")) for path in ["Cargo.toml"] + [member + "/Cargo.toml" for member in MEMBERS]}
    check_manifests(manifests)
    check_sources(rust)
    print("PASS: exact native unsafe exception, compiler lint inheritance, source/include boundaries")


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        unittest.main(argv=[sys.argv[0]])
    elif sys.argv[1:]:
        raise SystemExit("usage: check-unsafe.py [--self-test]")
    else:
        main()
