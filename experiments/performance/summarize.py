"""Small raw-sample summary; timings are not a correctness/performance gate."""
import csv
import json
import math
from collections import defaultdict
from pathlib import Path
import sys

groups = defaultdict(list)
with Path(sys.argv[1]).open() as stream:
    for row in csv.DictReader(stream):
        groups[tuple(row[k] for k in ("workload", "width", "scenario", "workers", "cache"))].append(row)
out = []
for key, rows in groups.items():
    record = dict(zip(("workload", "width", "scenario", "workers", "cache"), key))
    record["samples"] = len(rows)
    for field in rows[0]:
        if not field.endswith("_ns"):
            continue
        values = sorted(int(row[field]) for row in rows)
        record[field.removesuffix("_ns") + "_ms"] = {
            label: values[max(0, math.ceil(p * len(values)) - 1)] / 1e6
            for label, p in (("median", .5), ("p95", .95), ("p99", .99))
        }
    out.append(record)
Path(sys.argv[2]).write_text(json.dumps(out, indent=2) + "\n")
for row in out:
    total = row["total_ms"]
    print(row["workload"], row["width"], row["scenario"], total)
