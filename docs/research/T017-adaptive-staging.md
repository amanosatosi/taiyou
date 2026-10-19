# T017 adaptive staging and W2 performance follow-up

Performance/verification follow-up to reviewed candidate
`027e58a52efdd870b99f49ab5f3bae6e952f8f59`. Existing measurements, including the
5,000-event four-glyph 180-second timeout, remain evidence, not acceptance limits.
W2 Core semantics are owner-approved; its final recursive-rounding contract,
data/conformance/native-work admission and C7 limits remain pending. No automatic
wrapping mode is enabled.

## Deterministic staging

Eligibility examines parsed operations only: the evaluator starts with a default
anchor and only SetPosition writes that anchor. Style resets and unsupported or
recovered literal operations do not reset it. Actual evaluation still enforces
strict/tolerant policy. Renderer event references avoid repeating the public
linear EventId lookup.

For two or more eligible events, retain a stable prefix of prepared immutable
layouts up to one eighth of the initial remaining transient budget. Account
the slot array, transitive local ownership and saved diagnostics separately.
Unretained events retain only collision descriptors and are reconstructed during
painting. Explicit events never enter this preparation or collision indexing.
Zero/one eligible events have no collision prepass. Slots are sorted by EventId,
looked up in logarithmic time and taken without moving other slots.

Optional retention cannot change a successful minimal frame into LimitExceeded:
if an attempt using optional storage hits a limit or allocation failure, drop
the entire unpublished attempt and retry once with minimal staging. No output
lease, diagnostics, masks, native context or retained layout survives the failed
attempt. Genuine minimal failures remain failures. This is bounded frame-local
staging, not a cache, eviction framework or memory-observation heuristic.

The initial one-quarter trial is preserved as partial evidence at candidate
`2574e17e1c60ef106e3785947f327c6c0f75313b`: a 32 MiB, 256-event/32-glyph
effects scene replayed after admission, producing 640 shaping calls versus
512 for minimal staging. The smaller admission fraction leaves more native
scratch headroom. A new 32 MiB test requires successful partial reuse with fewer
preparations than minimal staging, preventing a replay from masquerading as
successful partial retention. Whole-frame retry still protects arbitrary tiny
caller budgets.

## Font authentication and experimental widths

The immutable validated catalog already authenticates its font bytes. New
catalog-backed outline/shaping paths reuse that proof; public untrusted blob
outline extraction still authenticates each supplied blob. All native dimension,
face, glyph, scratch and resource checks remain. No mutable native face is cached.

The W2 advances experiment shares actual itemization, HarfBuzz shaping, fallback,
cluster/source ownership and metrics with full shaping. It skips native glyph ink
extraction and deliberately publishes Unknown ink, preventing its use as ordinary
renderable layout. Candidate widths still require real boundary-specific shaping;
no approximate character widths or monotonicity assumption is introduced.

## Verification

Measured production candidate:
`ae467c953b9cf937469fb118e48f3f8c909fcaac`; integration base
`d5a545ac49aabbfc23e3895231d407da576e276f`. The follow-up sanitizer/experiment/test
changes do not change these production sources. Original samples, scripts,
environment, instrumentation/build identities and artifact provenance are under
[results/ae467c9](../../experiments/t017/results/ae467c9/provenance.json).
The source job completed the measurements before its sanitizer discovery failed.
That job is not reported as a passing native gate. Follow-up candidate
`77afbd20be79b5a5c4863fe55eff79d3fefa9e30` passed Linux/macOS/Windows, the
Platform gate, Fast/MSRV and Documentation. Its completed release rerun,
including expanded-budget cases, is preserved in the
[144-attempt report](../../experiments/t017/results/77afbd2/summary.md).

### Release measurements

Three sequential cold CLI processes per cell, default release opt-level3;
Linux ubuntu24.04 runner/image and CPU details are in the saved environment.
Same pinned Source Sans3 Regular3.052 (SHA256
`08df266400933d3178d081a45f94a08814c3e55b4b7dd2e0ff69cb1329f13ab6`),
FreeType/HarfBuzz recipe, input bytes and dimensions for all three snapshots.
Numbers include font intake, render and output encoding; instrumentation is
inclusive/overlapping. They are observations, not guaranteed playback FPS.

| Workload | Earlier9eee ms | Reviewed027 ms | Adaptive ae467c9 ms | Evaluation / shaping calls reviewed → adaptive |
|---|---:|---:|---:|---|
| One ordinary event | 10.225 | 10.213 | 6.407 | 2/1 → 1/1 |
| Eight ordinary events | 45.711 | 48.567 | 14.652 | 24/16 → 8/8 |
| 100 automatic events | 61.207 | 88.336 | 16.182 | 300/200 → 100/100 |
| 500 automatic events | 479.825 | 418.977 | 58.955 | 1500/1000 → 500/500 |
| 1,000 automatic events | 2173.354 | 833.694 | 115.035 | 3000/2000 → 1000/1000 |
| 2,000 positioned +2 dialogue | 1090.298 | 1120.214 | 244.126 | 4006/2004 → 2002/2002 |
| 5,000 positioned one-glyph events | 2723.744 | 2805.683 | 605.416 | 10000/5000 → 5000/5000 |
| 256 automatic,32 glyphs,effects | 2349.073 | 2407.450 | 498.086 | 768/512 → 256/256 |
| 100 typesetting,bord0.5,hidden shadow | 555.851 | 564.437 | 156.913 | 202/102 → 100/100 |
| 100 typesetting,bord3,visible shadow | 621.311 | 626.630 | 222.039 | 202/102 → 100/100 |
| 100 typesetting,1920×1080 | 784.850 | 799.003 | 387.847 | 202/102 → 100/100 |

The small scenes use PlayRes/output160×100, fs1 dense glyphs or fs5 typesetting;
they deliberately isolate event staging. The HD scene uses PlayRes/output
1920×1080, 98 explicitly positioned "Fansub typesetting" events at fs32,bord2,
visible shad2 plus two automatic "ordinary dialogue" events. All inputs use
WrapStyle2 and unscaled borders/shadows; actual ASS bytes/hashes are preserved.

The default 500-event multiglyph thin/hidden-shadow and thick/visible-shadow
workloads both return genuine LimitExceeded in all snapshots, exit2 at the CLI,
with no published raw frame/hash. In the thin case event441 requests 1,964
stroker points with only834 frame segments remaining. This is a configured
segment limit, not a visual match or timing of a completed frame. The explicit
4M-segment caller experiment is separate; production defaults/comparator stay
unchanged. Full error details and all three attempts remain in original JSONL.

### Memory and partial retention

Median process RSS (KiB); requested logical transient and aggregate local-stage
peaks (bytes) are conservative reservations, not heap/RSS measurements.

| Scene | RSS earlier / reviewed / adaptive | Transient earlier / reviewed / adaptive | Local earlier / reviewed / adaptive |
|---|---|---|---|
| Eight ordinary | 5772 /5880 /5844 | 15106776 /15048399 /15107736 | 132024 /16503 /132024 |
| 1,000 automatic | 7952 /6528 /7988 | 19458352 /15229513 /19581352 | 4267000 /4267 /4267000 |
| 2,000 positioned +2 dialogue | 10300 /6968 /7020 | 25536384 /15286233 /15291516 | 10032408 /9204 /23415 |
| 5,000 positioned | 17812 /9136 /9192 | 41474352 /15744327 /15744327 | 25035000 /5007 /5007 |
| 100 HD typesetting | 25008 /24744 /24636 | 30923896 /30867226 /30900616 | 2092378 /21014 /54020 |

Auto-only scenes intentionally spend the caller's bounded optional allowance to
avoid repeated shaping; they use more memory than reviewed minimal staging.
Position-dominated scenes keep the prior large memory savings. This is a measured
tradeoff, not a claim of universal lower RSS. Partial retention reduces shaping
work here, with modest observed time changes and no eviction mechanism:

| 256×32-glyph scene strategy | Shaping calls | Median ms | RSS KiB | Transient B | Aggregate local B |
|---|---:|---:|---:|---:|---:|
| Full128MiB | 256 | 498.086 | 7944 | 23235683 | 8096768 |
| Partial32MiB | 439 | 520.267 | 6816 | 17447759 | 2340472 |
| Minimal32MiB | 512 | 527.698 | 6768 | 16417346 | 31628 |
| Minimal128MiB | 512 | 528.232 | 6824 | 16417346 | 31628 |

All three samples have the same call counts and raw frame identity. Partial
retention reuses73 events, with no repeated full-frame preparation in this
measured configuration. The superseded one-quarter attempt's640 calls remain
recorded, not replaced. Arbitrary tiny budgets retain the bounded minimal retry.

Accounting: frame output/diagnostic/context reservations precede native work;
eligibility flags, descriptor/placement vectors and prepared slots are charged
before allocation. Each admitted local owns one transitive stage reservation and
its report has a separate charge. Moving to paint removes the saved total,
merges diagnostics into the pre-reserved frame report, drops the saved Vec and
holds only the local's charge. Drop the local charge after geometry/raster output
publication. No extra charge is attached to every Arc edge. Failed optional
attempts release all these objects before minimal retry; no partial frame lease
escapes. Tests compare successes and code/event/span failures at1/18/20/22/24/32/
128MiB, including real partial reuse rather than a replay counted as reuse.

### Stage costs and exactness

In the positioned5,000 sample1, evaluation/shaping/layout/outlines/geometry/raster/
composition costs were5.930/287.525/6.030/239.747/247.729/4.481/0.115ms.
There were zero footprint constructions or collision calls. Repeated per-glyph
font-byte authentication was the main avoidable previous cost; remaining work
includes5,000 native shapes and5,000 native outline extractions. Avoiding those
would require a different reuse design and is outside this correction.

The twelve-range CJK release profile held synthetic ASS reconstruction constant:
reviewed parsing0.132ms,evaluation0.022ms,full shaping126.137ms,of which repeated
authentication124.527ms. Adaptive parsing0.101ms,evaluation0.014ms,full shaping
1.211ms. Glyph IDs/advances/offsets match. Removing repeated parsing alone could
not explain this difference. The width-only experiment's complete multilingual
results and missing production-budget evidence are in [W2](T017-wrapping-policy-v2.md).

Every completed reviewed/current/staging-variant frame hash matches. All three
generations also match where Core behavior is unchanged. The sole successful
workload with a pre-F1 difference is the256-event effects stack: old hash
`266aef8ede868b0b3d5dfbbe52dd62ed0e890b242ec2372afdd5bd1de4a79ad8`,
approved-F1/current hash
`8d72e9f87c466e695feca177435455a667a8a499965186e17bb4ad8c176b6878`.
The additive `adaptive_f1_equivalence` test independently unions actual glyph
ink translated by local positions, radius0.5 and shadow1.25 under unit mapping,
derives successive touching anchors, and compares the real frame with explicit
F1 anchors and with the different pre-F1 local-box anchors. No expected image
is changed. This separates authorized placement behavior from staging exactness.

The48 debug samples also completed atae467c9: earlier9eee→adaptive median seconds
were1.350→0.052(eight),1.879→0.058(100),11.058→0.183(500),33.366→0.368(1000),
36.735→0.620(mixed2000),92.346→1.513(positioned5000). Both snapshots used two
case workers; these samples must not be combined with the earlier sequential
debug run to claim a precise speedup. All original samples/CPU/RSS remain saved.

### Verification status and remaining decisions

At ae467c9 Windows/macOS native,Fast and Documentation passed. Linux ordinary
tests and experiments passed before `font-sanitizer.py` rejected its expected
three-target set after addition of authenticated_outline. The narrow owner-
authorized fix validates exact Cargo target names/kinds,manifest and target-dir
ownership for the four unit/integration executables, logs the actual list and
runs all four with unchanged ASan/UBSan flags and pinned-native verification.
[Linux job118183558546](https://github.com/amanosatosi/taiyou/actions/runs/39288634496/job/118183558546)
confirmed the actual four Cargo artifacts and ran all30 tests successfully:
9font-unit,9catalog,11shaping,1authenticated-outline. The
[original sanitizer log excerpt](../../experiments/t017/results/77afbd2/sanitizer-log-excerpt.txt)
preserves executable identities and test results. ASan/UBSan instrumentation,
native prefix checks, leak detection and halt-on-error remain unchanged.

The final expanded4M-segment experiment completed all18 previously limited
configuration/sample pairs: thin-border median earlier/reviewed/current was
2790.392/2808.607/780.277ms; thick-border was3120.886/3141.708/1113.481ms.
Each workload's RGBA hashes match all three generations. Current RSS was
6760/7376KiB; logical transient peaks15973040/17255522B,local39422B.
The18 default-limit errors remain separate, with no raw output. The candidate's
bounded optional fallback retries those non-memory failures once; the resulting
failed-attempt counts are not completed-frame performance. No production limit
or protected comparator behavior is changed.

All126 successful frames in the144-attempt rerun retain exact reviewed/current
identity, including three samples of439-call partial staging. The complete48
debug samples were verified and reused with their originalae467c9 provenance.
Final exact-head correctness/reuse evidence and the human-owner acknowledgment
request are reported on PR#29; an earlier SHA's acknowledgment is never reused.

The complete T015 comparison onae467c9 remains exit1:101 visual frames comprise
1exact,92unsupported,8infrastructure errors,0regressions,0crashes; two parser
probes pass and one source is disabled, for104 inventory entries and187 verified
frozen assets. The independent native libass oracle remains separate. No newly
failing case or reference/threshold/classification/harness/workflow change is
authorized. Final candidate results must be checked individually again.

F1 is approved Core policy. The owner approved W2 semantics and approved its
Unicode data plan in principle. [Draft ADR-013](../decisions/013-core-unicode-wrapping.md)
submits the narrow recursive-rounding choice and proposed C4 text for final
confirmation. Data/conformance/native-work/C7 admission and implementation remain
gated; no automatic mode0/1/3 or Legacy capability is enabled. PR#29 must
remain unmerged for technical review and the final exact-head ADR-011 human
acknowledgment. An agent must not post that acknowledgment or mark T017 complete.
