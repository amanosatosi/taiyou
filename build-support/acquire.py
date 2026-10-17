"""CI-only source acquisition. Discovery is not a verified build result."""
import hashlib
import json
import os
from pathlib import Path
import sys
import tarfile
import urllib.request

root = Path(__file__).resolve().parent.parent
discover = sys.argv[1:] == ["--discover"]
manifest = root / "build-support" / ("source-proposals.json" if discover else "sources.lock.json")
records = json.loads(manifest.read_text(encoding="utf-8"))["sources"]
destination = Path(os.environ["RUNNER_TEMP"]) / "taiyou-native-sources"
destination.mkdir(parents=True, exist_ok=True)
for record in records:
    request = urllib.request.Request(record["url"], headers={"User-Agent": "Taiyou-T002-source-audit"})
    path = destination / (record["id"] + (".otf" if record["id"] == "font" else ".tar.gz"))
    digest = hashlib.sha256()
    size = 0
    with urllib.request.urlopen(request, timeout=120) as response, path.open("wb") as output:
        while block := response.read(1024 * 1024):
            size += len(block)
            if size > 64 * 1024 * 1024:
                raise RuntimeError("source archive exceeds acquisition budget")
            digest.update(block)
            output.write(block)
    record["sha256"] = digest.hexdigest()
    if "git_blob" in record:
        blob = path.read_bytes()
        actual = hashlib.sha1(b"blob " + str(len(blob)).encode() + b"\0" + blob).hexdigest()
        if actual != record["git_blob"]:
            raise RuntimeError("pinned font git blob differs")
    if not discover:
        expected = next(r for r in json.loads(manifest.read_text(encoding="utf-8"))["sources"] if r["id"] == record["id"])["sha256"]
        if digest.hexdigest() != expected:
            raise RuntimeError("pinned source SHA-256 differs")
        if record["id"] != "font":
            target = destination / record["id"]
            if not target.exists():
                with tarfile.open(path) as archive:
                    archive.extractall(target, filter="data")
    print("SOURCE_AUDIT=" + json.dumps(record, sort_keys=True), flush=True)
print("SOURCE_ROOT=" + str(destination), flush=True)
