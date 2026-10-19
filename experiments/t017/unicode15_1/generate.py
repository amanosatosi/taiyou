"""Offline Unicode15.1 data prerequisite; execution restricted to Actions.
No host Unicode database, runtime dependency or ASS tailoring is used here.
"""
from bisect import bisect_right
import hashlib
import json
import os
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parent

def sha(data):
    return hashlib.sha256(data).hexdigest()

def verify():
    manifest = json.loads((ROOT/"manifest.json").read_text())
    assert manifest["unicode"] == "15.1.0"
    for row in manifest["files"]:
        assert Path(row["file"]).name == row["file"]
        data = (ROOT/"data"/row["file"]).read_bytes()
        assert len(data) == row["bytes"] and sha(data) == row["sha256"],row["file"]
    return manifest

def properties(name):
    rows = []
    for line in (ROOT/"data"/name).read_text(encoding="utf-8").splitlines():
        fields = [s.strip() for s in line.split("#")[0].split(";")]
        if len(fields) < 2:
            continue
        ends = fields[0].split("..")
        value = fields[1] if len(fields) == 2 else "_".join(fields[1:])
        rows.append([int(ends[0],16),int(ends[-1],16),value])
    return rows

def merge(rows):
    result = []
    for lo,hi,value in sorted(rows):
        assert 0 <= lo <= hi <= 0x10ffff
        assert not result or result[-1][1] < lo,"overlapping property ranges"
        if result and result[-1][1]+1 == lo and result[-1][2] == value:
            result[-1][1] = hi
        else:
            result.append([lo,hi,value])
    return result

def tables():
    result = {}
    for key,name in [("LB","LineBreak.txt"),("GCB","GraphemeBreakProperty.txt"),("EAW","EastAsianWidth.txt")]:
        result[key] = merge(properties(name))
    result["EP"] = merge([r for r in properties("emoji-data.txt") if r[2] == "Extended_Pictographic"])
    result["InCB"] = merge([[a,b,v.removeprefix("InCB_")] for a,b,v in properties("DerivedCoreProperties.txt") if v.startswith("InCB_")])
    categories = []
    first = None
    for line in (ROOT/"data/UnicodeData.txt").read_text().splitlines():
        fields = line.split(";")
        cp,name,category = int(fields[0],16),fields[1],fields[2]
        if name.endswith(", First>"):
            assert first is None
            first = (cp,category)
        elif name.endswith(", Last>"):
            assert first is not None and first[1] == category
            categories.append([first[0],cp,category])
            first = None
        else:
            assert first is None
            categories.append([cp,cp,category])
    assert first is None
    result["GC"] = merge(categories)
    return result

class Lookup:
    def __init__(self, tables):
        self.tables = tables
        self.starts = {name:[r[0] for r in rows] for name,rows in tables.items()}
    def get(self, name, cp, default):
        i = bisect_right(self.starts[name],cp)-1
        rows = self.tables[name]
        return rows[i][2] if i >= 0 and cp <= rows[i][1] else default

def graphemes(cps, db):
    # Independent direct rule implementation of UAX29 revision43, default
    # extended grapheme clusters. Return code-point offsets, including endpoints.
    p = [db.get("GCB",c,"Other") for c in cps]
    incb = [db.get("InCB",c,"None") for c in cps]
    ep = [db.get("EP",c,"") != "" for c in cps]
    result = {0,len(cps)}
    controls = {"Control","CR","LF"}
    for i in range(1,len(cps)):
        a,b = p[i-1],p[i]
        if a == "CR" and b == "LF":
            continue
        if a in controls or b in controls:
            result.add(i)
            continue
        if a == "L" and b in {"L","V","LV","LVT"}:
            continue
        if a in {"LV","V"} and b in {"V","T"}:
            continue
        if a in {"LVT","T"} and b == "T":
            continue
        if b in {"Extend","ZWJ","SpacingMark"} or a == "Prepend":
            continue
        if incb[i] == "Consonant":
            j,saw_linker = i-1,False
            while j >= 0 and incb[j] in {"Extend","Linker"}:
                saw_linker |= incb[j] == "Linker"
                j -= 1
            if saw_linker and j >= 0 and incb[j] == "Consonant":
                continue
        if ep[i] and a == "ZWJ":
            j = i-2
            while j >= 0 and p[j] == "Extend":
                j -= 1
            if j >= 0 and ep[j]:
                continue
        if a == b == "Regional_Indicator":
            j = i-1
            while j >= 0 and p[j] == "Regional_Indicator":
                j -= 1
            if (i-1-j)%2 == 1:
                continue
        result.add(i)
    return result

def cases(name):
    count = 0
    for number,line in enumerate((ROOT/"data"/name).read_text(encoding="utf-8").splitlines(),1):
        body = line.split("#")[0].strip()
        if not body:
            continue
        tokens = body.split()
        assert len(tokens)%2 == 1
        cps = [int(c,16) for c in tokens[1::2]]
        markers = tokens[0::2]
        assert all(c in {"÷","×"} for c in markers)
        assert all(0 <= c <= 0x10ffff for c in cps)
        count += 1
        yield number,cps,{i for i,c in enumerate(markers) if c == "÷"}
    expected = 1187 if name == "GraphemeBreakTest.txt" else 10274
    assert count == expected,(name,count)

if __name__ == "__main__":
    assert os.environ.get("GITHUB_ACTIONS") == "true","generation/conformance executes on Actions only"
    manifest = verify()
    output = Path(sys.argv[1])
    output.mkdir(parents=True,exist_ok=True)
    data = tables()
    encoded = (json.dumps({"unicode":"15.1.0","defaults":{"LB":"XX","GCB":"Other","EAW":"N","EP":"","GC":"Cn","InCB":"None"},"tables":data},separators=(",",":"))+"\n").encode()
    assert encoded == (ROOT/"generated/properties.json").read_bytes(),"stored experimental tables differ from verified regeneration"
    (output/"properties.json").write_bytes(encoded)
    db = Lookup(data)
    failures = []
    count = 0
    for line,cps,expected in cases("GraphemeBreakTest.txt"):
        actual = graphemes(cps,db)
        count += 1
        if actual != expected:
            failures.append({"line":line,"actual":sorted(actual),"expected":sorted(expected)})
    from line_rules import line_breaks
    # Independently expose the numeric-profile distinction. These authored
    # expectations are separate from every unmodified upstream test record.
    assert line_breaks([0x25,0x28],db) == {2}
    assert line_breaks([0x25,0x28],db,number_test_tailoring=True) == {1,2}
    line_cases,line_failures = 0,[]
    for line,cps,expected in cases("LineBreakTest.txt"):
        actual = line_breaks(cps,db,number_test_tailoring=True)
        line_cases += 1
        if actual != expected:
            line_failures.append({"line":line,"actual":sorted(actual),"expected":sorted(expected)})
    summary = {"unicode":"15.1.0","manifest_sha256":sha((ROOT/"manifest.json").read_bytes()),"generator_sha256":sha(Path(__file__).read_bytes()),"line_rules_sha256":sha((ROOT/"line_rules.py").read_bytes()),"generated_bytes":len(encoded),"generated_sha256":sha(encoded),"ranges":{k:len(v) for k,v in data.items()},"grapheme_unmodified_cases":count,"grapheme_failures":failures,"line_unmodified_cases_imported":line_cases,"line_failures":line_failures,"line_test_numeric_profile":"UAX14 section8.2 Example7 as required by original test header","line_algorithm_conformance":"experimental rule probe; admission pending review","ass_tailoring":"not applied to Unicode conformance","production_automatic_wrapping":"disabled"}
    (output/"summary.json").write_text(json.dumps(summary,indent=2)+"\n")
    print(json.dumps(summary),flush=True)
    assert not failures,"unmodified grapheme conformance failed"
    assert not line_failures,"unmodified line-break conformance failed"
