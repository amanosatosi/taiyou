# T005: frozen inventory and solo-maintainer policy

The external libass-tests corpus stays outside Taiyou at commit
a5073319af8117271d8ac13ef5c088197ff2d59b. The unchanged manifest records **187 tracked
assets**, **101 regression-frame records**, **two crash inputs**, **one upstream-
disabled source** and **one separately retained auxiliary image**. Each source,
font and image has its original Git blob identity and SHA-256; font symlinks retain
their raw target, safe in-corpus resolution and resolved identity. No corpus bytes
are committed, regenerated or downloaded to the user's PC.

The inventory was generated explicitly on Actions at candidate
6531526eadb340476b1e040775155a163359c40c
([original job](https://github.com/amanosatosi/taiyou/actions/runs/39041240365/job/117421005537)).
Its raw SHA-256 remains
4482337de14f60c7750d98a5f7f43a8878ac824d9304143ac94d15e5a204680c;
canonical JSON digest is
251afc7a8ae01cdbb9daf159f925ac91256e07c18391c3608e63839619725e4e.
Both profiles, historical T014-not-implemented reasons and historical protection
metadata remain frozen. T014 now exists, but does not certify these external
cases. Governance now follows [ADR-011](../docs/decisions/011-solo-maintainer-validation.md),
not those historical metadata strings.

## Accepted baseline and routine checks

[The corpus workflow](../.github/workflows/inventory.yml) runs on every PR; its
required context is **Corpus policy gate**, without workflow-level path filtering.
It loads the guard, acquisition/verifier tools, manifest and policy from the exact
PR base revision. Bootstrap PR #11 uses owner-reviewed immutable snapshot
39f6ce2fa64b231fd705862d853c8df239b54a22 and cannot change its inventory, reference
lock or regression-bearing policy fields. The owner's instruction authorizes only
the ADR-011 governance/infrastructure amendment; final candidate acceptance is
still required.

The base-revision guard compares protected paths, including validation files,
reference locks, workflows, selectors, normative contracts and Taiyou expected
fixtures. Unacknowledged changes fail with an exact head/base/change-digest comment
for the human owner to review and post. [Setup/promotion instructions](MAINTAINER-SETUP.md)
explain acknowledgments, reruns and final acceptance. An acknowledged proposal
does not replace the base snapshot used for that run.

The external source cache is an optimization only. Every job checks the exact
commit, Git objects, complete file set, every blob/SHA-256 and safe symlink mapping.
The accepted verifier runs before checks and again even after failures. Corpus
files are marked read-only before candidate tests. Normal CI never generates a
replacement manifest. Candidate tests are separately exercised against the frozen
accepted inventory; tests never mutate upstream references.

Runner-only commands (the workflow exports TAIYOU_ACCEPTED and TAIYOU_CORPUS):

```text
python "$TAIYOU_ACCEPTED/validation/acquire.py"
python "$TAIYOU_ACCEPTED/validation/inventory.py" verify --corpus "$TAIYOU_CORPUS" --manifest "$TAIYOU_ACCEPTED/validation/inventory.json"
cargo +1.85.1 fetch --locked
cargo run --locked --offline -p xtask -- inventory verify --corpus "$TAIYOU_CORPUS" --manifest "$TAIYOU_ACCEPTED/validation/inventory.json"
cargo run --locked --offline -p xtask -- policy self-test
python validation/baseline.py self-test
python "$TAIYOU_ACCEPTED/validation/inventory.py" verify --corpus "$TAIYOU_CORPUS" --manifest "$TAIYOU_ACCEPTED/validation/inventory.json"
```

Synthetic tests retain hash/asset-byte/threshold/support/selector tampering,
missing/duplicate/unselected results, false snapshots, ART_REG_* overrides,
duplicate JSON keys, missing source/font/PNG files and safe/escaping symlinks.
Additional regressions cover symlink loops, duplicate selections/profiles and
result-level demotion of an accepted supported case. Baseline tests exercise
owner identity, stale head/base/digest acknowledgments and bootstrap restrictions.
No full comparator or native libass oracle is implemented; those remain T015.

## Manifest semantics and licenses

Each frame has a stable ID and parent source, fonts, timestamp, reference size,
original scale metadata, feature tags, profile support records and pending oracle
provenance. Crash/disabled records remain enumerated; unsupported active cases stay
in the result denominator. Missing assets are INFRA_ERROR, never unsupported.

The pinned corpus README/run-single.sh and locally inspected libass compare source
explain filename millisecond timestamps and 1–8[x1–8] scale metadata. The local
archive is not authenticated as the pinned oracle revision; exact native oracle,
viewport and comparison semantics remain T015. No legacy compatibility is inferred.

No LICENSE/COPYING appears at this corpus pin; per-asset license unknowns remain
explicit. Acquisition is not redistribution authorization. Do not bundle fonts or
substitute different files while claiming identical provenance.

## States, evidence and limitations

[Protection evidence](protection-evidence.json) preserves earlier observations
that main was unprotected. New observations and governance states are separate;
no file or test alone proves that GitHub settings are active.

- **Inventory verified:** the exact external corpus/manifest matched in named CI.
- **Candidate CI passed:** ordinary Actions passed on the reported candidate SHA.
- **Solo-maintainer accepted:** the human explicitly accepted that exact candidate.
- **Independently trusted:** not claimed by this solo-maintainer configuration.

The [setup guide](MAINTAINER-SETUP.md) recommends ordinary main protection with CI
and zero required second-person approvals. No extra reviewer, dedicated App or
separate validator is mandatory. Actual configured controls and outstanding manual
actions must be reported; do not claim deployment before checking settings.

The admin can deliberately change baselines or controls. Agents using the owner's
account cannot be cryptographically distinguished from the human and must not
self-acknowledge or merge. Candidate-controlled CI can manipulate its own checks;
read-only modes, base snapshots and synthetic negatives are accidental-change
safeguards, not proof of malicious-code containment or independent trust.
The stronger separately controlled validator/issuer remains an optional upgrade.
