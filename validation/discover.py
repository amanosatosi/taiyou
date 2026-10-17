"""Small metadata report only; never emits font/image bytes."""
import json
import os
from pathlib import Path
import subprocess

root = Path(os.environ["TAIYOU_CORPUS"])
entries = subprocess.check_output(["git", "-C", str(root), "ls-tree", "-r", "-z", "HEAD"]).decode().split("\0")
files = []
for entry in filter(None, entries):
    metadata, path = entry.split("\t", 1)
    mode, kind, blob = metadata.split()
    files.append({"path": path, "mode": mode, "kind": kind, "blob": blob})
print("CORPUS_TREE=" + json.dumps(files, sort_keys=True), flush=True)
for path in sorted(root.iterdir()):
    if path.is_file() and path.name.lower().startswith(("readme", "license", "copying")):
        print("CORPUS_DOC=" + path.name, flush=True)
        print(path.read_text(encoding="utf-8")[:20000], flush=True)
