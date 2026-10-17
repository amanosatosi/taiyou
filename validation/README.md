# T005 candidate inventory and policy checks

The frozen [inventory](inventory.json) contains metadata and hashes only. It was generated from the exact `a5073319af8117271d8ac13ef5c088197ff2d59b` libass-tests checkout in Actions at candidate `6531526eadb340476b1e040775155a163359c40c`, [run 39041240365, job 117421005537](https://github.com/amanosatosi/taiyou/actions/runs/39041240365/job/117421005537). That job verified 187 tracked assets, 104 case records (101 regression frames, two crash inputs, one upstream-disabled source), and eleven synthetic negative-policy/safety tests. The Fast job on that initial candidate failed formatting; subsequent candidate evidence is recorded in [PR 11](https://github.com/amanosatosi/taiyou/pull/11). This is no renderer or native-oracle result.

## Manifest schema 1

Each asset records its original relative path, Git mode/blob identity, raw SHA-256, byte length, role and license status. Symlinks additionally record the original target, resolved in-corpus path and resolved content SHA-256. PNG assets include IHDR dimensions. All 187 entries remain visible, including scripts, metadata, disabled inputs and the auxiliary `regression/karaoke/357_diff.png`; that diagnostic image does not match an ASS/timestamp frame name and is listed separately. No upstream file is modified or bundled.

Each frame has a stable case ID equal to its expected image path and a stable parent ID equal to its source path. Crash/disabled records use the source path as their ID. Records include source/font paths, filename timestamp in milliseconds, reference image size, original upstream scale text, group feature tag, both profile support records and oracle revision. The asset table supplies the corresponding content hashes. No Taiyou renderer exists, so both CoreV1/LegacyXyV1 records remain explicitly unsupported for T014; they are retained in active result denominators. Upstream-disabled input remains visible with its original status. Missing PNG/font/source is INFRA_ERROR, never unsupported.

Timestamp/scale provenance: the pinned corpus README and run-single.sh describe these fields; local libass compare.c lines 454 onward use `<subtitle_name>-<time_in_msec>.png`, and its parse_scale function accepts 1–8[x1–8]. The local archive is unverified against the immutable oracle pin. The manifest therefore records raw metadata without claiming a reproduced oracle configuration. Pinned oracle builds, viewport/scale interpretation and capture evidence remain T015 work. No legacy behavior, tolerance or feature is admitted by this inventory.

No LICENSE/COPYING file appears in this corpus revision. Per-asset licenses remain unknown pending provenance review, including fonts with possible embedded notices. CI acquisition is not redistribution authorization. No license or reviewer identity is guessed.

## Commands on Actions only

The [candidate workflow](../.github/workflows/inventory.yml) acquires the missing corpus explicitly in RUNNER_TEMP, outside the candidate checkout, and caches that source copy on the candidate branch. The cache is an optimization, not a trusted asset publisher: every run verifies the exact commit, Git objects, working-file Git blob identity, every SHA-256, safe symlink resolution and exact file set. It cannot be a trusted source for a privileged job without separate approved hash verification. No build artifacts or corpus bytes are downloaded onto the user's PC.

Runner commands:

```text
python validation/acquire.py
cargo run --offline -p xtask -- inventory verify --corpus <runner-corpus> --manifest validation/inventory.json
cargo run --offline -p xtask -- policy self-test
```

Inventory generation is an explicit proposal operation, never an automatic replacement of an existing manifest. The verifier compares the complete manifest against checked immutable inputs and rejects missing, changed or extra files. The policy self-test uses independently authored synthetic bytes and metadata; it never mutates upstream goldens. It covers golden/threshold/support/selector/missing-case tampering, missing expected assets, forbidden ART_REG_* variables, duplicate JSON keys, path/symlink escape, missing/duplicate results and denominator retention.

`policy check` requires independently supplied inventory/policy SHA-256 arguments. Those hashes must come from an approved protected base or validator. Candidate-provided hashes do not establish trust. No tolerance/expected-difference exception is currently admitted, and no upstream run.sh/regeneration path is executed.

## Protection gate remains open

[protection-evidence.json](protection-evidence.json) records the public branch/rules API responses: `protected: false`, active rules `[]` at main `5b23d4df6c19b0f956c3d8225bccf02a89e15ee0`. The candidate workflow has a read-only token and no secrets, but its own code and cache are candidate-controlled. It is not the independently issued required validation status described by V4, and this job does not create a sandbox for executing arbitrary renderer binaries against protected storage.

An actual maintainer must choose/configure the trusted workflow/app or protected validator, named independent reviewers, required issuer/rules, stale-review dismissal and author self-approval prevention. Read-only asset execution, pre/post execution hashing and trusted result collection must be verified there before renderer regression acceptance. No placeholder CODEOWNERS or self-issued policy approval substitutes for this evidence. T005 is not accepted and this protected PR must not be self-integrated.
