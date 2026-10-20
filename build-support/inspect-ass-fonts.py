"""Runner-only, frozen corpus font metadata evidence; no reference rewriting."""
import hashlib
import json
import os
from pathlib import Path
from fontTools.ttLib import TTFont

root = Path(__file__).resolve().parent.parent
corpus = Path(os.environ["TAIYOU_CORPUS"])
inventory = json.loads((root / "validation/inventory.json").read_text())
report = []
for case in inventory["cases"]:
    if case["source"] not in ["regression/font_nonunicode/shiftjis.ass", "regression/faux-italics/cff.ass", "regression/embedded-font/efont.ass"]:
        continue
    item = {"case": case["id"], "source": case["source"], "script": (corpus / case["source"]).read_text(encoding="utf-8-sig"), "fonts": []}
    # Keep attachment output bounded: source hashes and header/style lines only.
    item["script"] = "\n".join(line for line in item["script"].splitlines() if line.startswith(("Style:", "ScriptType:", "fontname:", "[")))
    for relative in case["fonts"]:
        path = corpus / relative
        font = TTFont(path)
        names = [{"id": n.nameID, "platform": n.platformID, "encoding": n.platEncID, "language": n.langID, "value": n.toUnicode()}
                 for n in font["name"].names if n.nameID in [1, 4, 6, 16, 21]]
        item["fonts"].append({"path": relative, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "names": names})
        font.close()
    report.append(item)
out = Path(os.environ["RUNNER_TEMP"]) / "ass-font-metadata.json"
out.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
print(out.read_text(encoding="utf-8"))
