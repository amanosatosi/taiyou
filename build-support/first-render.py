"""Actions only. Actual Taiyou CLI produces PNG/raw; stdlib only verifies PNG."""
import binascii
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import zlib

root = Path(__file__).resolve().parent.parent
folder = Path(os.environ["TAIYOU_RENDER_SAMPLE_DIR"])
folder.mkdir(exist_ok=True)
source = root / "tests/taiyou/core-slice/first.ass"
shutil.copyfile(source, folder / "input.ass")
font = Path(os.environ["RUNNER_TEMP"]) / "taiyou-shaping-fonts/latin.otf"
command = ["cargo", "run", "--locked", "-p", "taiyou-cli", "--features", "native", "--bin", "taiyou", "--", "render",
           "--input", str(folder / "input.ass"), "--font", str(font), "--time-us", "500000", "--width", "320", "--height", "180",
           "--profile", "core-v1", "--output", str(folder / "first.png"), "--raw", str(folder / "first.rgba")]
print("T014_CLI_COMMAND=" + json.dumps(command), flush=True)
result = subprocess.run(command, cwd=root, capture_output=True)
(folder / "cli-stdout.txt").write_bytes(result.stdout)
(folder / "cli-stderr.txt").write_bytes(result.stderr)
print(result.stderr.decode("utf-8", errors="replace"), flush=True)
result.check_returncode()
raw = (folder / "first.rgba").read_bytes()
png = (folder / "first.png").read_bytes()
if len(raw) != 320 * 180 * 4 or png[:8] != b"\x89PNG\r\n\x1a\n" or len(png) > 2 * 1024 * 1024:
    raise RuntimeError("invalid CLI output dimensions/signature")
offset = 8
compressed = bytearray()
seen = []
while offset < len(png):
    length = struct.unpack_from(">I", png, offset)[0]
    kind = png[offset+4:offset+8]
    payload = png[offset+8:offset+8+length]
    if offset + 12 + length > len(png) or binascii.crc32(kind + payload) & 0xffffffff != struct.unpack_from(">I", png, offset+8+length)[0]:
        raise RuntimeError("invalid PNG chunk extent/CRC")
    seen.append(kind.decode("ascii"))
    if kind == b"IHDR" and struct.unpack(">IIBBBBB", payload) != (320,180,8,6,0,0,0):
        raise RuntimeError("PNG must be noninterlaced straight RGBA8")
    if kind == b"IDAT": compressed.extend(payload)
    offset += 12 + length
inflater = zlib.decompressobj()
rows = inflater.decompress(bytes(compressed), 180 * (320 * 4 + 1) + 1)
if len(rows) != 180 * (320 * 4 + 1) or not inflater.eof:
    raise RuntimeError("invalid PNG decompressed extent")
decoded = bytearray()
previous = bytearray(320*4)
for y in range(180):
    start = y * (320 * 4 + 1)
    filt = rows[start]
    row = bytearray(rows[start+1:start+1+320*4])
    for x in range(len(row)):
        left = row[x-4] if x>=4 else 0
        up = previous[x]
        ul = previous[x-4] if x>=4 else 0
        if filt == 0: predictor = 0
        elif filt == 1: predictor = left
        elif filt == 2: predictor = up
        elif filt == 3: predictor = (left+up)//2
        elif filt == 4:
            p = left+up-ul
            distances = [abs(p-left),abs(p-up),abs(p-ul)]
            predictor = [left,up,ul][distances.index(min(distances))]
        else: raise RuntimeError("invalid PNG filter")
        row[x] = (row[x]+predictor)&255
    decoded.extend(row)
    previous=row
expected = bytearray()
for i in range(0,len(raw),4):
    r,g,b,a=raw[i:i+4]
    if max(r,g,b)>a: raise RuntimeError("raw output is not premultiplied")
    expected.extend([0 if a==0 else min(255,(c*255+a//2)//a) for c in [r,g,b]]+[a])
delta = [abs(a-b) for a,b in zip(expected,decoded)]
if any(delta): raise RuntimeError("PNG decoding differs from exact C6 conversion")
candidate=subprocess.check_output(["git","rev-parse","HEAD"],cwd=root,text=True).strip()
if os.environ.get("CANDIDATE_SHA",candidate)!=candidate: raise RuntimeError("candidate mismatch")
fixtures=json.loads((root/"build-support/shaping-fonts.proposed.json").read_text(encoding="utf-8"))
font_record=next(f for f in fixtures["fonts"] if f["id"]=="latin")
digest=lambda b: hashlib.sha256(b).hexdigest()
if digest(font.read_bytes())!=font_record["sha256"]: raise RuntimeError("font pin differs")
report={"task":"T014","candidate":candidate,"profile":"CoreV1","workers":1,"cache":"disabled","time_us":500000,
        "width":320,"height":180,"stride":1280,"png_producer":"actual Taiyou CLI --bin taiyou",
        "raw_format":"premultiplied encoded-domain RGBA8 top-down", "png_format":"straight encoded-domain RGBA8; no color conversion",
        "source_sha256":digest(source.read_bytes()),"raw_sha256":digest(raw),"png_sha256":digest(png),"font":font_record,
        "native_sources":json.loads((root/"build-support/sources.lock.json").read_text(encoding="utf-8"))["sources"],
        "png_chunks":seen,"png_conversion_max_delta":max(delta),"png_conversion_mismatches":sum(v!=0 for v in delta),
        "diagnostics":result.stderr.decode("utf-8",errors="replace"),"status":"candidate evidence, not approved reference or T005 validation"}
path=folder/"report.json"
if path.exists():
    old=json.loads(path.read_text(encoding="utf-8"))
    if old["raw_sha256"]!=report["raw_sha256"] or old["png_sha256"]!=report["png_sha256"]: raise RuntimeError("CLI output changed between same-candidate passes")
path.write_text(json.dumps(report,indent=2)+"\n",encoding="utf-8")
print("T014_FIRST_PNG="+json.dumps(report),flush=True)
comparisons=[]
for actual_path in sorted(folder.glob("align*.actual.rgba")):
    stem=actual_path.name.split(".")[0]
    actual=actual_path.read_bytes()
    expected=(folder/(stem+".expected.rgba")).read_bytes()
    if len(actual)!=160*100*4 or len(expected)!=len(actual): raise RuntimeError("invalid fixed-font evidence extent")
    diff=bytes(abs(a-b) for a,b in zip(actual,expected))
    changed=[i//4 for i in range(0,len(diff),4) if any(diff[i:i+4])]
    item={"case":stem,"max_delta":max(diff),"per_channel_mae":[sum(diff[c::4])/(160*100) for c in range(4)],"mismatch_count":len(changed),
          "alpha_mismatch_count":sum(v!=0 for v in diff[3::4]),"changed_pixel_bounds":None if not changed else [min(i%160 for i in changed),min(i//160 for i in changed),max(i%160 for i in changed)+1,max(i//160 for i in changed)+1]}
    comparisons.append(item)
    header=b"P5\n160 100\n255\n"
    (folder/(stem+".actual-alpha.pgm")).write_bytes(header+actual[3::4])
    (folder/(stem+".expected-alpha.pgm")).write_bytes(header+expected[3::4])
    (folder/(stem+".difference.pgm")).write_bytes(header+bytes(max(diff[i:i+4]) for i in range(0,len(diff),4)))
(folder/"alignment-comparison.json").write_text(json.dumps({"candidate":candidate,"reference":"independent C FreeType metrics/outline; xy eighth-grid coverage; C6 integer paint expectations", "reference_source_sha256":digest((root/"build-support/renderer-reference.c").read_bytes()),"cases":comparisons,"status":"not run on this platform" if not comparisons else "exact candidate-side checks, pending numeric review"},indent=2)+"\n",encoding="utf-8")
if any(item["mismatch_count"] for item in comparisons): raise RuntimeError("fixed-font reference pixels differ: zero tolerance")
print("T014_ALIGNMENT_COMPARISONS="+json.dumps(comparisons),flush=True)
