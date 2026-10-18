"""Windows Actions only: official release inventory and bounded CSRI probe.
No output from this host is automatically admitted as a reference capture.
"""
import ctypes as C
import hashlib
import json
import locale
import os
from pathlib import Path
import platform
import struct
import subprocess
import sys
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(os.environ["RUNNER_TEMP"]) / "r002-xy"
OUT.mkdir(exist_ok=True)
URL = "https://github.com/pinterf/xy-VSFilter/releases/download/3.2.0.810/xyVSFilterSubFilter-pfmod-3.2.0.810.7z"
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
def write(name, obj):
    (OUT / name).write_text(json.dumps(obj,indent=2)+"\n")
def pe(path):
    data = path.read_bytes()
    assert data[:2] == b"MZ"
    offset = struct.unpack_from("<I",data,0x3c)[0]
    assert data[offset:offset+4] == b"PE\0\0"
    return hex(struct.unpack_from("<H",data,offset+4)[0])

def child(dll, fixture):
    # Public CSRI ABI facts, independently declared; no renderer implementation.
    class Format(C.Structure):
        _fields_ = [("pixel_format",C.c_int),("width",C.c_uint),("height",C.c_uint)]
    class Frame(C.Structure):
        _fields_ = [("pixel_format",C.c_int),("planes",C.c_void_p*4),("strides",C.c_ssize_t*4)]
    C.windll.gdi32.AddFontResourceExW.argtypes=[C.c_wchar_p,C.c_uint,C.c_void_p]
    assert C.windll.gdi32.AddFontResourceExW(str(OUT / "SourceSans3-Regular.otf"),0x10,None) > 0
    dll_search = os.add_dll_directory(str(dll.parent))
    C.windll.ole32.CoInitializeEx(None,2)
    lib = C.CDLL(str(dll))
    for name, args, result in [
        ("csri_renderer_default",[],C.c_void_p),
        ("csri_open_file",[C.c_void_p,C.c_char_p,C.c_void_p],C.c_void_p),
        ("csri_request_fmt",[C.c_void_p,C.POINTER(Format)],C.c_int),
        ("csri_render",[C.c_void_p,C.POINTER(Frame),C.c_double],None),
        ("csri_close",[C.c_void_p],None)
    ]:
        fn = getattr(lib,name)
        fn.argtypes, fn.restype = args,result
    for repeat in [1,2]:
        inst = lib.csri_open_file(lib.csri_renderer_default(),str(fixture.resolve()).encode("utf-8"),None)
        assert inst, "CSRI refused the script"
        try:
            # Only this RGB32 selector renders in the supplied source; BGR24
            # falsely negotiates then traps. Do not probe unsafe formats.
            fmt = Format(0x102,64,64)
            assert lib.csri_request_fmt(inst,C.byref(fmt)) == 0
            for fourth in [0,255]:
                for background in [0,255]:
                    initial = bytes([background,background,background,fourth])*(64*64)
                    buffer = C.create_string_buffer(initial,len(initial))
                    frame = Frame(0x102,(C.c_void_p*4)(C.addressof(buffer),0,0,0),(C.c_ssize_t*4)(256,0,0,0))
                    lib.csri_render(inst,C.byref(frame),0.5)
                    name = f"{fixture.stem}-b{background}-x{fourth}-r{repeat}.rgb32"
                    (OUT / name).write_bytes(buffer.raw)
        finally:
            lib.csri_close(inst)

if sys.argv[1:] and sys.argv[1] == "--child":
    child(Path(sys.argv[2]),Path(sys.argv[3]))
    sys.exit(0)

assert os.name == "nt" and C.sizeof(C.c_void_p) == 8
archive = OUT / "official.7z"
if not archive.exists():
    with urllib.request.urlopen(URL,timeout=60) as response, archive.open("wb") as f:
        total=0
        while block := response.read(1024*1024):
            total += len(block)
            assert total <= 64*1024*1024, "archive acquisition budget exceeded"
            f.write(block)
archive_hash = sha(archive)
# Discovery only; never accept an arbitrary newly downloaded binary as a golden.
seven = Path(os.environ["ProgramFiles"]) / "7-Zip/7z.exe"
listing = subprocess.check_output([str(seven),"l","-slt",str(archive)],text=True)
(OUT / "archive-list.txt").write_text(listing)
for line in listing.splitlines():
    if line.startswith("Path = ") and line[7:] != str(archive):
        p = Path(line[7:])
        assert not p.is_absolute() and ".." not in p.parts
extracted = OUT / "release"
subprocess.run([str(seven),"x","-y",str(archive),"-o"+str(extracted)],check=True)
inventory=[]
for path in sorted(extracted.rglob("*")):
    if path.is_file():
        row=dict(path=path.relative_to(extracted).as_posix(),size=path.stat().st_size,sha256=sha(path))
        if path.suffix.lower() == ".dll":
            row["pe_machine"] = pe(path)
        inventory.append(row)
write("inventory.json",dict(url=URL,archive_sha256=archive_hash,files=inventory,
                            source_commit_equivalence="unverified",acceptance="pending-human-review"))
print("R002_XY_INVENTORY="+json.dumps(dict(archive_sha256=archive_hash,files=inventory)),flush=True)
# Vector drawings only: no glyph outlines/fallback are exercised or certified.
# Controlled font installed privately for ancillary line metrics, with exact bytes.
font_record = next(r for r in json.loads((ROOT/"build-support/sources.lock.json").read_text())["sources"] if r["id"] == "font")
font = OUT / "SourceSans3-Regular.otf"
with urllib.request.urlopen(font_record["url"],timeout=60) as response:
    data=response.read(4*1024*1024+1)
assert len(data) <= 4*1024*1024 and hashlib.sha256(data).hexdigest() == font_record["sha256"]
font.write_bytes(data)
C.windll.gdi32.AddFontResourceExW.argtypes=[C.c_wchar_p,C.c_uint,C.c_void_p]
registered = C.windll.gdi32.AddFontResourceExW(str(font),0x10,None)
assert registered > 0, "controlled private font registration failed"
# A separate process does not inherit FR_PRIVATE registration. Each child must
# register the same verified file before opening CSRI (see the child entry point).
environment=dict(os=platform.platform(),version=sys.getwindowsversion()[:],architecture=platform.machine(),
                 image=os.getenv("ImageVersion"),locale=locale.getlocale(),
                 acp=C.windll.kernel32.GetACP(),oemcp=C.windll.kernel32.GetOEMCP(),
                 font=font_record,font_registration="FR_PRIVATE; child registration",
                 glyph_fallback="not exercised: polygon drawings only; text face selection unverified",
                 script_resolution=[64,64],output=[64,64],timestamp_seconds=0.5,
                 pipeline="CSRI 0x102 -> MSP_RGBA; top-first host buffer; stride=256; raw four bytes preserved",
                 conversion="none; no YUV, color matrix/range or PNG conversion",
                 alpha="uncalibrated fourth-byte probes at 0/255; no A8 extraction authorized",
                 capture_admission="blocked pending ABI/pipeline calibration, font mapping and maintainer review")
write("environment.json",environment)
dlls=[extracted/r["path"] for r in inventory if Path(r["path"]).name.lower()=="vsfilter.dll" and r.get("pe_machine")=="0x8664"]
assert len(dlls)==1, f"expected one x64 VSFilter (not XySubFilter): {dlls}"
results=[]
for fixture in sorted((ROOT/"validation/proposals/R002").glob("*.ass")):
    result=subprocess.run([sys.executable,__file__,"--child",str(dlls[0]),str(fixture)],timeout=90,capture_output=True,text=True)
    results.append(dict(script=fixture.name,input_sha256=sha(fixture),exit=result.returncode,stdout=result.stdout,stderr=result.stderr))
    if result.returncode:
        break  # bounded feasibility probe, no repeated infrastructure hunting
outputs=[dict(file=p.name,sha256=sha(p),size=p.stat().st_size) for p in sorted(OUT.glob("*.rgb32"))]
for p in OUT.glob("*-r1.rgb32"):
    assert p.read_bytes() == p.with_name(p.name.replace("-r1.","-r2.")).read_bytes(), "CSRI repeat differs"
write("probe.json",dict(binary_sha256=sha(dlls[0]),binary_path=str(dlls[0].relative_to(extracted)),
                       results=results,outputs=outputs,capture_verified=False,approved=False))
print("R002_XY_PROBE="+json.dumps(dict(results=results,outputs=outputs,capture_verified=False)),flush=True)
if any(r["exit"] for r in results):
    sys.exit(1)
