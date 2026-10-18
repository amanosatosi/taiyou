"""CI-only explicit T005 acquisition; never obtains local reference archives."""
import json
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parent.parent
pin = next(item for item in json.loads((root / "docs/references.lock.json").read_text())["references"] if item["id"] == "libass-tests")
destination = Path(os.environ["RUNNER_TEMP"]) / "taiyou-libass-tests"
if not destination.exists():
    destination.mkdir()
    subprocess.run(["git", "init", str(destination)], check=True)
    subprocess.run(["git", "-C", str(destination), "fetch", "--depth=1", pin["repository"], pin["commit"]], check=True)
    subprocess.run(["git", "-C", str(destination), "checkout", "--detach", "FETCH_HEAD"], check=True)
actual = subprocess.check_output(["git", "-C", str(destination), "rev-parse", "HEAD"], text=True).strip()
if actual != pin["commit"]:
    raise RuntimeError("corpus commit mismatch")
subprocess.run(["git", "-C", str(destination), "fsck", "--full"], check=True)
print("CORPUS_COMMIT=" + actual, flush=True)
with open(os.environ["GITHUB_ENV"], "a", encoding="utf-8") as output:
    output.write("TAIYOU_CORPUS=" + str(destination) + "\n")
