# T015 external reference adapter

Run on Actions only. The corpus stays external at T005's accepted pin. No
source, font, reference image, support state or tolerance is updated by this task.
The ordinary Corpus policy gate continues using the PR base's verifier and
requires the human's exact-head/base/digest acknowledgment for these proposed
infrastructure changes. This is candidate CI under ADR-011, not independent trust.

```text
cargo build --locked -p xtask -p taiyou-cli --features taiyou-cli/native
cargo run --locked -p xtask -- compare --suite full --profile core-v1
cargo run --locked -p xtask -- compare --suite focused --profile core-v1 --case regression/DIRECTORY/SCRIPT-0500.png
cargo run --locked -p xtask -- compare --suite focused --profile core-v1 --manifest selectors.json
python validation/test_compare.py
```

The focused manifest is `{"cases":["exact inventory ID", "another ID"]}`. Unknown,
duplicate and empty selections fail. Full mode cannot accept selectors. Focused
reports list every unselected ID, carry no stale results, and never claim full
coverage. Full reports include all inventory entries: active regression frames
and crash probes in results, disabled upstream sources in the explicit disabled
list. The auxiliary image remains in the verified asset inventory, not a frame.

`TAIYOU_CORPUS` points to the acquired corpus. `TAIYOU_ACCEPTED` points to the
PR-base snapshot materialized by Actions. `ACCEPTED_BASELINE` records its SHA.
Without that environment the command uses the working snapshot and labels it
unaccepted, never independently trusted. Reference snapshots and every source,
font, symlink and reference are verified by the accepted tool before and after
execution, including after renderer failures. Every cache is verified again.

## Upstream semantics and provenance

Inspected the local libass compare README, compare.c and image.c, then the missing
pinned corpus `run.sh`, `regression/run-all.sh`, `regression/run-single.sh` and
`crash/run-all.sh` directly at the locked revision. The local archive is not
claimed to authenticate the pinned libass commit; Actions obtains that exact Git
revision independently. Native compare runs without `-o`; it cannot regenerate
targets. Source links:

- [Native compare conventions](https://github.com/libass/libass/blob/f61db567e6593df3470e91594bcd4ad2d0473aff/compare/README.md)
- [Regression runner](https://github.com/libass/libass-tests/blob/a5073319af8117271d8ac13ef5c088197ff2d59b/regression/run-single.sh)

`SCRIPT-NNNN.png` uses decimal milliseconds, checked before conversion into signed
microseconds. Reference dimensions are the viewport. Fonts are precisely the
frozen directory font list in inventory order, with symlinks resolved inside the
corpus and their hashes retained. No system font lookup or font substitution.

The native oracle uses pinned libass built from source with compare enabled and
system font providers disabled. Its exact distro build dependencies, compiler,
configure flags, per-directory scale, exit codes and original logs are archived.
Both upstream default `-p2` and exact `-p0` are reported. Native SAME/GOOD/BAD/FAIL
classifications never become Taiyou statuses. Reproduction differences and an
environment failure are distinguished; neither is described as a successful
oracle certification.

Upstream scale means render at multiplied dimensions and downsample in 16-bit
precision. CoreV1 currently lacks a separate sampling-size/device-viewport input;
passing larger dimensions to T014 would silently change semantics. Non-unit
scale cases therefore remain explicitly unsupported, with no fabricated metrics.
The original scale metadata and all cases remain visible. Implementing a correct
oversampling adapter needs a later reviewed viewport seam; no resizing is done.

## Pixels and statuses

Only noninterlaced RGBA8/16 PNG targets are accepted, as documented upstream.
Other/corrupt/missing images are infrastructure failures, not unsupported.
No gamma, profile, color conversion, cropping or dimension substitution occurs.
Following pinned libass image.c, RGBA8 straight references use exactly
`(product + (product >> 8) + 128) >> 8`, then expand by 257. This fixed reciprocal
differs from rounded division by 255 for 24 channel/alpha pairs (229,152 yields
136). RGBA16 uses `(product + (product >> 16) + (1 << 15)) >> 16`, preserving its
native precision. Both paths are checked against the original pinned C decoder:
all 65,536 RGBA8 channel/alpha pairs, and all 65,536 16-bit channel values against
six alpha boundaries (393,216 pairs). These synthetic inputs never alter corpus
files. Internal T014 premultiplied
RGBA8 raw output expands by 257. Alpha stays ordinary coverage. Metrics use
0..65535 encoded premultiplied channel units; divide by 257 for byte units.

Each computed comparison includes exact equality, maximum channel delta, per-
channel MAE, changed pixels, half-open changed bounding rectangle, alpha mismatch
count and both dimensions. Dimension mismatch is explicit, with numeric pixel
metrics null. The first twelve rendered outputs have actual PNG and opaque
absolute-difference heatmaps. Reports retain source/font/reference identities and
raw/PNG hashes. Original expected PNGs and fonts are not redistributed because
T005 records unknown redistribution rights; expected artifact links identify
their original external paths and hashes. No goldens are generated or approved.

Strict rendering is attempted first. If a typed unsupported feature is found,
an exploratory tolerant render may provide pixel metrics, but cannot promote the
case or turn strict unsupported behavior into a pass. Accepted supported cases
cannot be demoted. Known unpromoted cases with differences stay unsupported;
their nonzero metrics remain visible. Unapproved tolerance/expected-difference
statuses are rejected. No thresholds or support registry changes are included.

The private `taiyou-compare` subprocess adapter calls T014's unchanged
`render::run` and maps typed C7 errors to exit codes: 10 invalid argument, 11
encoding, 12 script, 20 unsupported, 30 font unavailable, 31 native failure, 32
limit, 33 allocation, 34 internal, 80 adapter setup. Classification never matches
human error strings or accepts renderer-supplied PASS records. The normal `taiyou`
and `taiyou-cli` commands and their output hashes are unchanged.

The two crash inputs use a bounded subprocess invoking the existing parser:
parse success or safe rejection is a **safety-only** pass, never a visual pass.
No implicit fonts, synthetic viewport or arbitrary timestamp are assigned to
these inputs, which have no expected frame. This is a parser safety probe, not
full render fuzz coverage. Disabled sources are never silently enabled.

Machine-readable `summary.visual.exact_matches` counts only pixel comparisons;
`summary.safety.successful_parser_probes` counts parser-only successes. Each group
also has its own total and status counts. T005's aggregate `summary.counts` and
statuses remain unchanged, explicitly labeled by `counts_scope`; its PASS_EXACT
includes safety probes and must never be presented as a visual-match count.
The full-run check rebuilds complete accounting from accepted case kinds and
validates current-candidate, pixel and parser-exit evidence. Observed outcome
counts belong in PR evidence, not permanent assertions: a genuine infrastructure
recovery or visual improvement must remain valid. Infrastructure errors stay
visible and still fail the comparator command; supported regressions fail too.
Invalid compressed PNG payloads
raise typed `ImageDecodeError`, becoming per-case INFRA_ERROR/InvalidImage;
remaining cases are still processed and the complete report is emitted.

## Execution and limitations

Renderer calls have 45-second wall/30-second CPU limits, 2 GiB address-space and
128 MiB per-file bounds on Linux, no stdin, a temporary working directory, a
64 KiB retained diagnostic limit and no inherited GitHub credentials or user
configuration. Corpus files are read-only. This is bounded accidental-failure
containment, **not** a hostile-code sandbox: no network namespace or independent
issuer is claimed, and the same runner user can reverse file permissions.
Pre/post inventory checks detect mutations. Candidate JSON is never accepted as
pixel evidence: the comparator reads actual raw bytes and computes its metrics.

`capture-import` fails explicitly until authenticated xy capture/environment
metadata is available. LegacyXyV1 is not implemented by T014 and is not claimed
as tested by CoreV1 runs. Reports do not infer VSFilter parity. Rendering defects
are reported; this task does not alter T014 to make references match.
