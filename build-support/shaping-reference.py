"""CI-only independent C/HarfBuzz observations. No Taiyou output promotion."""
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import sys

root = Path(__file__).resolve().parent.parent
temporary = Path(os.environ["RUNNER_TEMP"])
prefix = temporary / "taiyou-native-install"
source = root / "build-support/shaping-reference.c"
exe = temporary / ("taiyou-shaping-reference.exe" if os.name == "nt" else "taiyou-shaping-reference")
if os.name == "nt":
    args = ["cl", "/nologo", "/std:c11", "/O2", "/MD", "/D_CRT_SECURE_NO_WARNINGS", "/I" + str(prefix / "include/harfbuzz"), str(source), str(prefix / "lib/harfbuzz.lib"), "/Fe:" + str(exe), "/Fo:" + str(temporary / "shaping-reference.obj")]
else:
    args = ["cc", "-std=c11", "-O2", "-I" + str(prefix / "include/harfbuzz"), str(source), "-L" + str(prefix / "lib"), "-lharfbuzz", "-lc++" if platform.system() == "Darwin" else "-lstdc++", "-lm", "-o", str(exe)]
subprocess.run(args, check=True)
records = json.loads((root / "build-support/shaping-cases.proposed.json").read_text(encoding="utf-8"))["cases"]
fonts = json.loads((root / "build-support/shaping-fonts.proposed.json").read_text(encoding="utf-8"))["fonts"]
discover = sys.argv[1:] == ["--discover"]
lines = []
for case in records:
    record = next((f for f in fonts if f["id"] == case["font"]), None)
    if record is None and case["font"] != "collection":
        raise RuntimeError("unknown reference font: " + case["font"])
    font = temporary / "taiyou-shaping-fonts" / (record["id"] + Path(record["path"]).suffix) if record else Path(os.environ["TAIYOU_TEST_COLLECTION"])
    data = case["text"].encode("utf-8")
    text_path = temporary / "taiyou-reference-input.txt"
    text_path.write_bytes(data)
    start, end = case.get("range", [0, len(data)])
    output = subprocess.check_output([str(exe), str(font), str(text_path), str(case.get("face", 0)), case["direction"], case["script"], case["language"], str(start), str(end), str(int(case.get("ligatures", True)))], text=True)
    observed = json.loads(output)
    if not discover and observed != case.get("expected"):
        raise RuntimeError("provisional independent reference differs or is not frozen: " + case["id"])
    print("SHAPING_REFERENCE=" + json.dumps({"id": case["id"], "expected": observed}, sort_keys=True), flush=True)
    lines.append("case " + case["id"] + " " + " ".join(map(str, [observed["upem"], *observed["metrics"]])))
    lines.extend("glyph " + " ".join(map(str, glyph)) for glyph in observed["glyphs"])
    lines.append("end")
destination = temporary / "taiyou-shaping-reference.tsv"
destination.write_text("\n".join(lines) + "\n", encoding="utf-8")
print("SHAPING_REFERENCE_CLIENT_SHA256=" + hashlib.sha256(source.read_bytes()).hexdigest(), flush=True)
print("SHAPING_REFERENCES_PROVISIONAL=" + str(destination), flush=True)
