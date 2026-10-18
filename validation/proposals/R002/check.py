"""Additive checkpoint checks. Run on Actions; never changes accepted policy."""
import copy
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HERE = Path(__file__).resolve().parent

def validate(inventory, sources, fixtures):
    assert inventory["status"] == "research-checkpoint"
    assert not inventory["maintainer_accepted"] and inventory["enabled_capabilities"] == []
    assert inventory["counts"] == dict(rules=19,approved=0,enabled=0,capture_verified=0)
    assert len(inventory["rules"]) == inventory["counts"]["rules"]
    source_files = {f["file"]:f for f in sources["files"]}
    assert sources["upstream_commit"] is None
    assert sources["locked_commit_equivalence"] == "unverified"
    assert len(source_files) == len(sources["files"])
    ids=set()
    for rule in inventory["rules"]:
        assert rule["id"] not in ids
        ids.add(rule["id"])
        assert rule["gate"] == "pending" and rule["interpretation"] == "provisional"
        assert not any(rule[k] for k in ["approved","enabled","capture_verified"])
        assert rule["xy_pixel_expectation"] is None
        ref=rule["source"]; source=source_files[ref["file"]]
        assert 1 <= ref["lines"][0] <= ref["lines"][1] <= source["lines"]
        assert len(source["sha256"]) == 64 and int(source["sha256"],16) >= 0
        path=HERE/rule["fixture"]
        assert path.parent == HERE and path.suffix == ".ass"
        assert hashlib.sha256(path.read_bytes()).hexdigest() == fixtures[path.name]
    assert {r["fixture"] for r in inventory["rules"]} == set(fixtures)
    assert set(fixtures) == {p.name for p in HERE.glob("*.ass")}
    document=(ROOT/"docs/research/R002-coverage.md").read_text(encoding="utf-8")
    assert all(r["id"] in document for r in inventory["rules"])
    for phrase in ["research checkpoint", "0 approved", "source-observed", "PLANNED"]:
        assert phrase in document
    previous=json.loads((ROOT/"validation/legacy-rules.json").read_text())
    assert previous["counts"]["rules"] == 30 and previous["counts"]["approved"] == 0
    assert previous["enabled_capabilities"] == []

inventory=json.loads((HERE/"rules.json").read_text())
sources=json.loads((HERE/"sources.json").read_text())
fixtures=json.loads((HERE/"fixtures.json").read_text())
validate(inventory,sources,fixtures)
# Reject invalid evidence rather than silently accepting a new capability.
for mutation in ["approved","enabled","capture_verified","source","fixture","commit"]:
    bad=copy.deepcopy(inventory); bad_sources=copy.deepcopy(sources); bad_fixtures=copy.deepcopy(fixtures)
    if mutation in ["approved","enabled","capture_verified"]:
        bad["rules"][0][mutation]=True
    elif mutation == "source":
        bad["rules"][0]["source"]["lines"]=[0,99999]
    elif mutation == "fixture":
        bad_fixtures[next(iter(bad_fixtures))]="0"*64
    else:
        bad_sources["upstream_commit"]="25cf3e8433b3a83a5489e47a2c335516492bb815"
    try:
        validate(bad,bad_sources,bad_fixtures)
    except AssertionError:
        pass
    else:
        raise AssertionError("invalid evidence accepted: "+mutation)
print("R002_CHECKPOINT: 19 proposals, 0 approved/enabled/capture-verified; 6 invalid evidence mutations rejected")
