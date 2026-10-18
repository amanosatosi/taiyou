"""Additive T016 diagnosis using the unchanged frozen comparator and inputs.
Run only in Actions. Report every original diagnostic and preserve the exit code.
"""
import json
import os
from pathlib import Path
import subprocess
import sys

out = Path(os.environ["RUNNER_TEMP"]) / "t016-limit-reproduction"
cases = [
    "regression/font_nonunicode/symbol-first-char-0xF000-1000.png",
    "regression/implicit-pos/bordercollision-2000.png",
    "regression/karaoke/357-k-and-kf-desynced-6798.png",
    "regression/karaoke/357-k-and-kf-desynced-7170.png",
    "regression/karaoke/357-k-and-kf-desynced-8170.png",
]
command = ["cargo","run","--locked","-p","xtask","--","compare","--suite","focused",
           "--profile","core-v1","--output",str(out)]
for case in cases:
    command += ["--case",case]
result = subprocess.run(command)
report = out / "report.json"
if report.is_file():
    evidence = json.loads(report.read_text())
    for case in evidence["records"]:
        print("T016_CASE="+json.dumps(case),flush=True)
    print("T016_REPRODUCTION_SUMMARY="+json.dumps(evidence["summary"]),flush=True)
sys.exit(result.returncode)
