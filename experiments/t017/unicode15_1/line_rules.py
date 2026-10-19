"""Independent UAX14 revision51 default rule probe; no ASS shaping/tailoring.
Uses only the version-pinned tables generated alongside this experiment.
"""
def line_breaks(cps, db, number_test_tailoring=False):
    raw = [db.get("LB",c,"XX") for c in cps]
    gc = [db.get("GC",c,"Cn") for c in cps]
    p = []
    for c,g in zip(raw,gc):
        p.append("AL" if c in {"AI","SG","XX"} else ("CM" if g in {"Mn","Mc"} else "AL") if c == "SA" else "NS" if c == "CJ" else c)
    # Fold inherited combining marks into logical base units for contextual rules.
    # Preserve raw offsets and the immediately preceding ZWJ for LB8a.
    units = []
    inherited = set()
    for i,c in enumerate(p):
        if c in {"CM","ZWJ"}:
            if units and units[-1][1] not in {"BK","CR","LF","NL","SP","ZW"}:
                inherited.add(i)
                continue
            c = "AL"
        units.append((i,c))
    result = {len(cps)}
    hangul = {"JL","JV","JT","H2","H3"}
    alphabetic = {"AL","HL"}
    closing = {"CL","CP","EX","IS","SY"}
    hard = {"BK","CR","LF","NL"}
    def cp(j):
        return cps[units[j][0]]
    def category(j):
        return gc[units[j][0]]
    def at(j):
        return units[j][1] if 0 <= j < len(units) else "sot" if j < 0 else "eot"
    def aksara(j):
        return at(j) in {"AK","AS"} or (0 <= j < len(units) and cp(j) == 0x25cc)
    for j in range(1,len(units)):
        i,b = units[j]
        a = at(j-1)
        if a == "BK":
            result.add(i); continue
        if a == "CR" and b == "LF":
            continue
        if a in {"CR","LF","NL"}:
            result.add(i); continue
        if b in hard or b in {"SP","ZW"}:
            continue
        k = j-1
        while k >= 0 and at(k) == "SP":
            k -= 1
        previous = at(k)
        if previous == "ZW":
            result.add(i); continue
        if raw[i-1] == "ZWJ":
            continue
        if a == "WJ" or b == "WJ" or a == "GL":
            continue
        if b == "GL" and a not in {"SP","BA","HY"}:
            continue
        if (b in closing and not (number_test_tailoring and a == "NU" and b != "EX")) or previous == "OP":
            continue
        if previous == "QU" and k >= 0 and category(k) == "Pi" and at(k-1) in hard|{"sot","OP","QU","GL","SP","ZW"}:
            continue
        if b == "QU" and category(j) == "Pf" and at(j+1) in hard|{"eot","SP","GL","WJ","CL","QU","CP","EX","IS","SY","ZW"}:
            continue
        if previous in {"CL","CP"} and b == "NS":
            continue
        if previous == b == "B2":
            continue
        if a == "SP":
            result.add(i); continue
        if a == "QU" or b == "QU":
            continue
        if a == "CB" or b == "CB":
            result.add(i); continue
        if b in {"BA","HY","NS"} or a == "BB":
            continue
        if a in {"HY","BA"} and at(j-2) == "HL":
            continue
        if a == "SY" and b == "HL" or b == "IN":
            continue
        if a in alphabetic and b == "NU" or a == "NU" and b in alphabetic:
            continue
        if a == "PR" and b in {"ID","EB","EM"} or a in {"ID","EB","EM"} and b == "PO":
            continue
        if a in {"PR","PO"} and b in alphabetic or a in alphabetic and b in {"PR","PO"}:
            continue
        if number_test_tailoring:
            # The *unmodified upstream corpus* explicitly selects UAX14 §8.2
            # Example7 numeric tailoring. Keep it distinct from the default
            # pair approximation and from all ASS/cluster filters.
            k = j-1
            if at(k) in {"CL","CP"}:
                k -= 1
            numeric_suffix = False
            while k >= 0 and at(k) in {"NU","SY","IS"}:
                numeric_suffix |= at(k) == "NU"
                k -= 1
            if (a in {"PO","PR"} and (b == "NU" or b in {"OP","HY"} and at(j+1) == "NU")
                or a in {"OP","HY"} and b == "NU"
                or a == "NU" and b in {"NU","SY","IS"}
                or a in {"NU","SY","IS"} and numeric_suffix and b in {"NU","SY","IS","CL","CP"}
                or numeric_suffix and b in {"PO","PR"}):
                continue
        elif (a in {"CL","CP","NU"} and b in {"PO","PR"}
              or a in {"PO","PR"} and b in {"OP","NU"}
              or a in {"HY","IS","NU","SY"} and b == "NU"):
            continue
        if a == "JL" and b in {"JL","JV","H2","H3"} or a in {"JV","H2"} and b in {"JV","JT"} or a in {"JT","H3"} and b == "JT":
            continue
        if a in hangul and b == "PO" or a == "PR" and b in hangul:
            continue
        if a in alphabetic and b in alphabetic:
            continue
        if a == "AP" and aksara(j) or aksara(j-1) and b in {"VF","VI"}:
            continue
        if a == "VI" and aksara(j-2) and (b == "AK" or cp(j) == 0x25cc):
            continue
        if aksara(j-1) and aksara(j) and at(j+1) == "VF":
            continue
        if a == "IS" and b in alphabetic:
            continue
        if a in alphabetic|{"NU"} and b == "OP" and db.get("EAW",cp(j),"N") not in {"F","W","H"}:
            continue
        if a == "CP" and db.get("EAW",cp(j-1),"N") not in {"F","W","H"} and b in alphabetic|{"NU"}:
            continue
        if a == b == "RI":
            k = j-1
            while k >= 0 and at(k) == "RI":
                k -= 1
            if (j-1-k)%2 == 1:
                continue
        if b == "EM" and (a == "EB" or db.get("EP",cp(j-1),"") and category(j-1) == "Cn"):
            continue
        result.add(i)
    return result
