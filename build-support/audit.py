"""Runner-only advisory snapshot; unavailable advisory service is an error."""
import datetime
import json
import os
from pathlib import Path
import subprocess
import urllib.request

root = Path(__file__).resolve().parent.parent
manifest = root / "build-support/dependency-smoke/Cargo.toml"
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--manifest-path", str(manifest), "--format-version", "1"], cwd=root, text=True))
production = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--features", "taiyou-font/native", "--format-version", "1"], cwd=root, text=True))
unique = {(package["name"], package["version"]): package for package in metadata["packages"] + production["packages"] if package["source"] is not None}
packages = [{key: package.get(key) for key in ["name", "version", "license", "repository", "rust_version"]} for _, package in sorted(unique.items())]
queries = [{"package": {"name": package["name"], "ecosystem": "crates.io"}, "version": package["version"]} for package in packages]
for name, version in [("meson", "1.12.1"), ("ninja", "1.13.2"), ("pkgconf", "3.0.7.post0"), ("colorama", "0.4.6"), ("fonttools", "4.60.1")]:
    queries.append({"package": {"name": name, "ecosystem": "PyPI"}, "version": version})
request = urllib.request.Request("https://api.osv.dev/v1/querybatch", data=json.dumps({"queries": queries}).encode(), headers={"Content-Type": "application/json", "User-Agent": "Taiyou-T002-advisory-audit"})
with urllib.request.urlopen(request, timeout=60) as response:
    results = json.load(response)["results"]
if len(results) != len(queries):
    raise RuntimeError("incomplete OSV response")
report = {"candidate": os.environ["CANDIDATE_SHA"], "queried_at": datetime.datetime.now(datetime.timezone.utc).isoformat(), "service": "https://api.osv.dev/v1/querybatch", "packages": packages, "advisories": [{"query": query, "ids": sorted(item["id"] for item in result.get("vulns", []))} for query, result in zip(queries, results)], "native_scope": "Native upstream release/security notices require separate review; OSV Rust/PyPI queries do not certify C/C++ dependencies."}
print("DEPENDENCY_AUDIT=" + json.dumps(report, sort_keys=True), flush=True)
if any(item["ids"] for item in report["advisories"]):
    raise RuntimeError("dependency advisories require review")
