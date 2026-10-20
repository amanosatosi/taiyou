"""Runner-only T008 licensed fixture provisioning; no upstream expected edits."""
import hashlib
import json
import os
from pathlib import Path
import urllib.request
from fontTools.ttLib import TTFont, TTCollection

directory = Path(os.environ["RUNNER_TEMP"]) / "taiyou-native-sources"
url = "https://raw.githubusercontent.com/adobe-fonts/source-sans/ed1808970eb3c7301c9a523bee26473ba0bb62fa/OTF/SourceSans3-Bold.otf"
with urllib.request.urlopen(url, timeout=60) as response:
    bold = response.read(1024 * 1024)
if len(bold) != 343596 or hashlib.sha1(b"blob " + str(len(bold)).encode() + b"\0" + bold).hexdigest() != "d2129396eab406f5d268806c7598b78eb7bfdafb":
    raise RuntimeError("pinned bold font blob differs")
(directory / "bold.otf").write_bytes(bold)
collection = TTCollection()
collection.fonts = [TTFont(directory / name, recalcTimestamp=False) for name in ["font.otf", "bold.otf"]]
collection.save(directory / "collection.otc", shareTables=True)
for font in collection.fonts:
    font.close()
print("FONT_FIXTURES=" + json.dumps({"source_commit":"ed1808970eb3c7301c9a523bee26473ba0bb62fa", "license":"OFL-1.1", "generator":"fonttools 4.60.2 TTCollection; unchanged input fonts; recalcTimestamp=False; shareTables=True", "sha256":{name:hashlib.sha256((directory/name).read_bytes()).hexdigest() for name in ["font.otf","bold.otf","collection.otc"]}}), flush=True)

# Additive, independently authored name-table fixtures. Original font fixtures
# above remain byte-identical. These are font metadata tests, never goldens.
for source, output in [("font.otf", "alias-regular.otf"), ("bold.otf", "alias-bold.otf")]:
    font = TTFont(directory / source, recalcTimestamp=False)
    for name_id, text, language in [(1, "Taiyou Alias Family", 0x409),
                                    (1, "たいよう 別名", 0x411),
                                    (16, "Taiyou Typographic Family", 0x409)]:
        font["name"].setName(text, name_id, 3, 1, language)
    font["name"].setName("TaiyouPostScriptAlias", 6, 3, 1, 0x409)
    font["name"].setName("Taiyou Legacy Encoding Alias".encode("utf-16-be"), 1, 3, 2, 0x411)
    font.save(directory / output)
    font.close()
print("ALIAS_FIXTURES=" + json.dumps({name: hashlib.sha256((directory/name).read_bytes()).hexdigest()
                                      for name in ["alias-regular.otf", "alias-bold.otf"]}), flush=True)
