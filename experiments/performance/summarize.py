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
    if "cache_hits" in rows[0]:
        for field in ("cache_hits", "cache_misses", "cache_evictions"):
            record[field] = sum(int(row[field]) for row in rows)
        requests = record["cache_hits"] + record["cache_misses"]
        record["cache_hit_rate"] = record["cache_hits"] / requests if requests else None
        record["cache_peak_bytes"] = max(int(row["cache_bytes"]) for row in rows)
    render_ns = sum(int(row["total_ns"]) for row in rows)
    record["render_fps_equivalent"] = len(rows) * 1e9 / render_ns if render_ns else None
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
