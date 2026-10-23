"""Acquire the pinned unmodified MUA Office Regular font for the xy oracle."""
import hashlib
import json
import os
from pathlib import Path
import urllib.request

ROOT = Path(__file__).resolve().parent
identity = json.loads((ROOT / "mua-office.json").read_text(encoding="utf-8"))
record = identity["acquisition"]
destination = Path(os.environ["RUNNER_TEMP"]) / "mua-office" / "MUAOffice-Regular.ttf"
destination.parent.mkdir(parents=True, exist_ok=True)
request = urllib.request.Request(record["url"], headers={"User-Agent": "Taiyou-xy-oracle"})
with urllib.request.urlopen(request, timeout=120) as response:
    data = response.read(record["size"] + 1)
if len(data) != record["size"]:
    raise RuntimeError(f"MUA Office size mismatch: expected {record['size']}, got {len(data)}")
blob = hashlib.sha1(b"blob " + str(len(data)).encode("ascii") + b"\0" + data).hexdigest()
if blob != record["git_blob_sha1"]:
    raise RuntimeError(f"MUA Office Git blob mismatch: expected {record['git_blob_sha1']}, got {blob}")
sha256 = hashlib.sha256(data).hexdigest()
expected = record.get("sha256")
if expected and sha256 != expected:
    raise RuntimeError(f"MUA Office SHA-256 mismatch: expected {expected}, got {sha256}")
destination.write_bytes(data)

out = Path(os.environ["RUNNER_TEMP"]) / "xy-oracle"
out.mkdir(parents=True, exist_ok=True)
result = {
    "schema": 1,
    "family": identity["family"],
    "variant": identity["variant"],
    "creator": identity["creator"],
    "license": identity["license"],
    "source_page": identity["source_page"],
    "source_page_date": identity["source_page_date"],
    "repository": record["repository"],
    "commit": record["commit"],
    "path": record["path"],
    "git_blob_sha1": blob,
    "sha256": sha256,
    "size": len(data),
}
(out / "mua-office-acquisition.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as env:
    env.write(f"MUA_OFFICE_FONT={destination}\n")
print("MUA_OFFICE=" + json.dumps(result, sort_keys=True), flush=True)
