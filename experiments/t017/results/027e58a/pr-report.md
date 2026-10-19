Historical PR #29 report for candidate027e58a only. Superseded by the adaptive
follow-up; its acknowledgment is not valid for a changed candidate.

CoreV1 collision now excludes explicit positions structurally, finds minimum displacement with open intervals, and retains one full local layout at a time. Owner-approved F1 adds a separate geometric effects footprint. **This remains an unmerged T017 Core checkpoint: W2 automatic wrapping is unapproved and unimplemented. Do not mark T017 complete or merge yet.**

Integration base: `d5a545ac49aabbfc23e3895231d407da576e276f`
Candidate: `027e58a52efdd870b99f49ab5f3bae6e952f8f59`
Branch: `task/T017-wrapping-collision`
Reviewed implementation used for before measurements: `9eee96fe16989089d8fd3f65b1bb136455ffca7e`

### Core corrections

- Explicit `pos` events never enter collision descriptors, ordering, obstacle searches or intervals. They retain ordinary shaping, layout, effects and rendering at their specified coordinates. Zero/single-eligible scenes bypass the native collision prepass and solver. New tests render 5,001 positioned events around two automatic events, check prefix/suffix mixtures and an all-positioned frame, and independently prove unchanged eligible placement/work counters with 5,000 unrelated positioned inputs.
- Eligible events use canonical layer/start/EventId placement order. Each event gathers horizontally relevant same-layer open forbidden-anchor intervals once, sorts them, unions strictly overlapping intervals and chooses the nearest legal endpoint in the required direction. Equal endpoints stay separate/legal. Bottom moves up; middle/top down. Half-open rectangles, layer isolation, offscreen placement, finite arithmetic and deterministic seeking remain intact. Complete-set worst case changes from O(N³) to O(N² log N), with O(N) workspace and no arbitrary displacement/iteration cap.
- The exact preserved old solver blob `033ddf4cdee97d34886b1ccac6f68e2e354cb064` is a test-only independent oracle for 1,000 randomized fractional/mixed-layer/start/alignment/order inputs. Independent numeric rectangles and real ASS→evaluation→shaping→layout→collision→RGBA tests cover placement and fresh/forward/backward/shuffled seeks.
- [ADR-012](https://github.com/amanosatosi/taiyou/blob/027e58a52efdd870b99f49ab5f3bae6e952f8f59/docs/decisions/012-core-collision-footprint.md) separately records owner-approved F1 and amends C4/CACHING. Base logical/ink reservation plus transformed fill/outline/visible-shadow geometric hulls use actual Q6 effect parameters and existing SBAS rules. Invisible effects add no padding. Rotation precedes device-space expansion/offset; unequal SBAS=true isotropic border mapping remains Unsupported. Geometric contact can share antialiased boundary pixels. A future fixed explicit rotation origin requires a new placement policy. No LegacyXyV1 rule is admitted.
- Final source review corrected footprint rotation to match C5's modulo 360 reduction and exact cardinal values. New pinned-font tests require 90°,450°,−270° and a large equivalent rotation to have identical footprints and RGBA.
- Classification retains one flag per active event. A bounded eligible-event prepass publishes compact descriptors and drops each full local artifact immediately. Paint prepares one event at a time in the existing layer/source/pass order. Multi-eligible events intentionally shape twice; positioned events once. Source ownership, diagnostics, T016 masks/pass ordering and resource limits remain. No retained layout/history/cache or parallel production renderer was introduced.

The new bounded-staging fixture includes the existing fixed diagnostic/context reservation, then 8 MiB live headroom and a stricter 64-glyph caller cap for its maximum 20-glyph events. Earlier setup errors were in this new experiment's caller budget; production defaults were not raised. Multilingual intake uses the existing shaping/layout fixtures' explicit 512 MiB configuration because the CJK font legitimately exceeds the default 128 MiB intake reservation.

Changed implementation: `crates/taiyou-render/src/{collision,collision_footprint,renderer,lib}.rs`. Additive review tests: `collision_footprint`, `collision_interval_review`, `previous_collision` in taiyou-render and `collision_review`, `wrapping_workload_review` in taiyou-cli. Docs include C4/K2/K3, separate ADR-012, Core checkpoint and policy proposals; experiments/evidence are under `experiments/t017/`.

The only alteration to an existing test remains the human-authorized identical `pos(10,10)` tags in the overlapping-paint fixture; every assertion is preserved. No other reviewed-candidate test changed. No workflow, comparator, expected image, frozen corpus/reference/inventory, tolerance, selection, classification or error mapping changed.

### Current candidate: actual paired renderer measurements

[Linux job118051667251](https://github.com/amanosatosi/taiyou/actions/runs/39250129120/job/118051667251), [original native evidence artifact12186344421](https://github.com/amanosatosi/taiyou/actions/runs/39250129120/artifacts/12186344421).

All 48 samples completed: eight cases × three before/after pairs. Every case's six raw RGBA hashes match, also matching the earlier sequential measurements. Actual script/raw hashes, instrumented source hashes, logs, counters and environment are in the artifact.

Environment: Ubuntu 24.04 image20261012.334.1, Linux 6.17.0-1022-azure x86_64/glibc2.39, AMD EPYC 9V74, Rust 1.99.1 (`58cf8f9c35b355199727627d07ae34a4e2846d67`), debug binaries, identical verified native prefix. Source Sans 3 Regular 3.052 pin`ed1808970eb3c7301c9a523bee26473ba0bb62fa`, font SHA-256`08df266400933d3178d081a45f94a08814c3e55b4b7dd2e0ff69cb1329f13ab6`. Two bounded case workers; pairs within each case stay sequential, and other correctness tests may share the runner. **Do not compare these wall times directly with the original sequential run as speedups.** No universal speedup is claimed.

Medians: RSS is measured process high-water KiB; retained bytes are conservative local-stage reservations, not actual heap/RSS.

| Scene | Wall seconds before → after | Peak RSS KiB before → after | Retained local bytes before → after | Shape calls before → after |
|---|---:|---:|---:|---:|
| normal-one | 0.266 → 0.230 | 7,900 → 7,864 | 16,503 → 16,503 | 1 → 1 |
| normal-two | 0.504 → 0.550 | 7,824 → 7,864 | 33,006 → 16,503 | 2 → 4 |
| normal-eight | 1.787 → 1.980 | 7,952 → 7,940 | 132,024 → 16,503 | 8 → 16 |
| positioned-5000-one-glyph | 112.689 → 104.520 | 19,712 → 11,248 | 25,035,000 → 5,007 | 5,000 → 5,000 |
| mixed-2000-one-glyph | 49.938 → 49.139 | 12,132 → 8,924 | 10,021,054 → 5,007 | 2,002 → 2,004 |
| dense-100 | 2.569 → 3.769 | 8,124 → 7,940 | 426,700 → 4,267 | 100 → 200 |
| dense-500 | 12.770 → 15.641 | 8,868 → 8,236 | 2,133,500 → 4,267 | 500 → 1,000 |
| dense-1000 | 35.129 → 31.081 | 9,848 → 8,488 | 4,267,000 → 4,267 | 1,000 → 2,000 |

Peak requested transient reservations fall41,474,352→15,744,327 bytes for positioned5,000 and25,525,030→15,231,711 for mixed2,000. Ordinary2/8 and dense100/500 became slower in this run; extra shaping is an explicit cost of bounded retention. Small-scene RSS is effectively flat. Large-scene RSS and retained layouts decrease.

ZIP SHA-256:`bda45944683cb651eebc86af8f531d86c8192330a1e60f8adc69e5ea7ee0ceba`.

### Preserved original timeout and earlier completed evidence

The original 5,000-event/four-glyph baseline **timed out at 180 s**, with no completed frame or RSS result: [job117841967333](https://github.com/amanosatosi/taiyou/actions/runs/39183812909/job/117841967333). Its input hash and the unexecuted 2,000×20-glyph workload remain in `larger-workloads-unavailable.json`. The smaller one-glyph workloads have distinct identities. No larger result is fabricated.

All 48 original sequential smaller samples at`f473e8190c34cc74e6718187b8772efba69c7ba2` are preserved byte-for-byte as artifact JSON under `experiments/t017/results/f473e81/`: [original job](https://github.com/amanosatosi/taiyou/actions/runs/39184889909/job/117845262915), [original artifact](https://github.com/amanosatosi/taiyou/actions/runs/39184889909/artifacts/12157799614). Verified ZIP SHA-256:`2855e4129608e390f776ff6795273207b09ac520ef995faedd1fba768b8f636b`.

Candidate`7900332ff318a4fb029add11cbc9f063d1eafdcd` legitimately reused those unchanged renderer inputs while completing the corrected multilingual/scaling/MSRV checks. Its original scaling/workload records are committed under `results/7900332/`. The final rotation/measurement-source change invalidated reuse, so the current candidate reexecuted all 48 samples. No correctness test or comparator is skipped by experiment evidence reuse.

### Current pure solver scaling

Same Linux job; new solver three isolated child-process samples per size, old solver one bounded sample per executable size. Peak RSS below is the new child process, not an invented old measurement.

| Eligible events | Old ms (one sample) | New ms (median3) | New obstacle checks | New RSS median KiB |
|---|---:|---:|---:|---:|
| 100 | 22.364 | 0.655 | 4,950 | 3,132 |
| 500 | 1540.016 | 12.552 | 124,750 | 3,108 |
| 1,000 | 12328.476 | 52.645 | 499,500 | 3,216 |
| 5,000 | not executed | 1346.216 | 12,497,500 | 3,732 |

Old 5,000 would derive 20,845,830,000 checks and exceeds the separate 200M experimental work budget; old runtime/RSS are unavailable. New 5,000 has 400,000 bytes vector backing capacities and 1,200,000 conservative workspace bytes versus old 960,000. **Pure solver workspace increases 25%**; the renderer memory win comes from dropping full retained layouts, not free interval storage.

### W2 remains blocked on approval and cost evidence

[Revised exact proposal](https://github.com/amanosatosi/taiyou/blob/027e58a52efdd870b99f49ab5f3bae6e952f8f59/docs/research/T017-wrapping-policy-v2.md) handles attached leading/trailing spaces and single overwide atoms; pinned Unicode 15.1 UAX14/UAX29 opportunities intersected with paragraph shaping clusters; NBSP/glue, boundary reshaping, source provenance and deterministic mode0/1/3 selection/ties. No Unicode library/data choice or production automatic wrapper is admitted. Existing WrapStyle 2 remains unchanged.

Current experiment, real pinned Latin/Arabic/Devanagari/Japanese fonts, independent word/ideograph ranges; not UAX conformance or completed wrapping:

| Workload | Evaluated / full candidates | Shaped glyphs | Seconds | Actual stop |
|---|---:|---:|---:|---|
| ordinary-latin | 78/78 | 2,616 | 0.826 | complete |
| ordinary-arabic | 136/136 | 6,176 | 1.319 | complete |
| ordinary-indic | 78/78 | 2,106 | 1.644 | complete |
| ordinary-cjk | 114/666 | 1,936 | 60.471 | experiment-time-budget |
| long-latin-typesetting | 1,377/20,910 | 1,000,847 | 18.056 | glyph-work-limit |
| long-cjk-typesetting | 114/25,425 | 6,555 | 60.472 | experiment-time-budget |

Proposed 10,000/1,000,000 defaults are **not validated**. CJK searches remain incomplete at the separate 60 s experimental stop. The trial glyph counter checked after a whole candidate and overshot 847; a production bound requires preflight/remaining-work enforcement or explicit whole-event rejection. No partial/greedy fallback is accepted.

Concrete next proposal: independently measure a logical-width candidate prepass without collecting ink/outline bounds for every rejected candidate; prove advance/source-range equivalence to full boundary shaping on these workloads plus mixed styles, joining scripts, bidi, clusters and fractional/negative advances. Then remeasure and propose exact caller limits. This needs owner approval and is not an implemented font-metrics shortcut/cache. W2 and modes0/1/3 stay unimplemented pending decisions; Legacy L01–L08 remain separately evidence-gated.

### Exact-candidate Actions and protected comparison

- [Fast gate / workspace Rust 1.85.1](https://github.com/amanosatosi/taiyou/actions/runs/39250129159/job/118051839207): success.
- [Documentation structure](https://github.com/amanosatosi/taiyou/actions/runs/39250129122/job/118051607348): success.
- Native [Linux](https://github.com/amanosatosi/taiyou/actions/runs/39250129120/job/118051667251), [Windows](https://github.com/amanosatosi/taiyou/actions/runs/39250129120/job/118051667261), [macOS](https://github.com/amanosatosi/taiyou/actions/runs/39250129120/job/118051667189): success. Full native/stage/CLI checks, fresh/sequential/backward collision tests, equivalent-angle regression, strict resource fixture, MSRV 1.85.0 and existing sanitizer checks completed.
- [Platform gate](https://github.com/amanosatosi/taiyou/actions/runs/39250129120/job/118064606823): success.
- [Full external comparator](https://github.com/amanosatosi/taiyou/actions/runs/39250129206/job/118051682016): **exit 1/failure**, honestly retained. 101 visual frames: **1 exact, 92 unsupported, 8 infrastructure errors, 0 regressions, 0 crashes**. Two successful parser-only probes, one upstream-disabled source, all 104 entries and 187 assets verified. No active-case status changed. Focused supplemental smoke/accounting/corpus reverification/artifact upload succeeded. [Full report and differences](https://github.com/amanosatosi/taiyou/actions/runs/39250129206/artifacts/12185342211); ZIP SHA-256`5cb40fe9892359c37793cb469285b6d4b6f4f092cded290574644e4f8602755f`.
- [Independent native libass oracle](https://github.com/amanosatosi/taiyou/actions/runs/39250129206/job/118051681870): reproduced 101/101 at both p0/p2. Libass pin`f61db567e6593df3470e91594bcd4ad2d0473aff`; corpus`a5073319af8117271d8ac13ef5c088197ff2d59b`. This is separate from Taiyou and any future xy suite.
- [Corpus policy gate](https://github.com/amanosatosi/taiyou/actions/runs/39250129115/job/118051604459): **blocked/failure pending exact human-owner acknowledgment** for C4/CACHING/ADR changes. Nothing bypassed; no acknowledgment posted by an agent. Four required gates and branch protection remain intact.

Remaining infrastructure errors, read from the actual uploaded report:

| Exact case ID | Recorded cause |
|---|---|
| regression/embedded-font/efont-1000.png | asset-verification: no explicit fonts |
| regression/faux-italics/cff-0000.png | requested family absent from validated catalog; FontUnavailable, exit 30 |
| regression/font_nonunicode/shiftjis-1000.png | same FontUnavailable, exit 30 |
| regression/ssa/alpha-0000.png | same FontUnavailable, exit 30 |
| regression/ssa/alpha-1000.png | same FontUnavailable, exit 30 |
| regression/ssa/alpha-2000.png | same FontUnavailable, exit 30 |
| regression/ssa/alpha-3000.png | same FontUnavailable, exit 30 |
| regression/ssa/alpha-4000.png | same FontUnavailable, exit 30 |

### Remaining acceptance gates

1. Human technical review of the Core interval/footprint/bounded-staging correction and measured tradeoffs.
2. W2 semantics, Unicode data/dependency and a viable measured search-budget plan require owner approval before automatic wrapping implementation.
3. New exact-head/base/change-digest acknowledgment under ADR-011 **after final technical review**, posted only by the human owner. An acknowledgment permits protected proposal validation; it does not complete T017 or authorize merge.
4. Keep this PR unmerged. No Legacy promotion, blur, renderer rewrite, corpus change or accepted reference regeneration.

