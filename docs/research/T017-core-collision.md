# T017 — Core collision implementation checkpoint

Integration base: `d5a545ac49aabbfc23e3895231d407da576e276f`.
Scope: implement existing C4 CoreV1 collision, preserving T010 WrapStyle 2 and
the merged T016 border/shadow pipeline. This is not completed Legacy-compatible
T017. Candidate SHA and exact Actions jobs are recorded in the implementation PR.

## Implemented policy

The current [adaptive staging follow-up](T017-adaptive-staging.md) classifies
placement from parsed operations, preserving evaluator semantics without fully
evaluating text for classification. Zero/single-eligible scenes bypass the native
prepass and collision solver. Other scenes prepare eligible events and publish
compact descriptors, retaining a resource-bounded prefix of immutable local
layouts for paint. Unretained locals are dropped and reconstructed for paint;
retained locals are shaped once. Explicit events are prepared only for ordinary
rendering and never enter descriptors, collision sorting, obstacle searches or
intervals. Retention is confined to this frame, with no history/per-event cache.
If optional storage causes a limit failure, discard the unpublished attempt and
retry once with minimal staging. Measurements report retained/transient peaks,
calls and replay rather than claiming the memory/performance tradeoff is free.

The resolver uses the owner-approved [ADR-012](../decisions/012-core-collision-footprint.md)
Core geometric effect footprint, distinct from immutable logical/ink/local
boxes. It includes visible transformed ink, device-space Q6 stroke radius and
visible Q6-offset shadow source support. Transparent effects add no padding;
the base layout reservation remains. No arbitrary antialias/bitmap padding or
historical rule is inferred. Geometric edge contact can share an antialiased
boundary pixel; a future fixed explicit rotation origin requires a new decision.

Each layer is independent. Eligible events are placed in `(start, EventId)`
order; bottom alignment moves up and middle/top moves down. Explicit positions
are neither moved nor obstacles. Zero-width/height boxes occupy no region;
blank lines inside a nonempty block still contribute the existing style metrics.
Each event gathers horizontally relevant same-layer obstacle intervals once,
sorts them and merges strictly overlapping open intervals. Equal endpoints are
not merged, preserving legal touching anchors. The component containing the
natural anchor gives its nearest legal up/down endpoint directly. Complete-set
complexity is O(N² log N), with O(N) workspace, replacing the prior O(N³) repeated
scan. No viewport clamp, history, arbitrary displacement/iteration cap or epsilon.
Checked numeric/allocation/reservation failures remain explicit errors.

Results are sorted by stable EventId. Geometry uses the resolved anchor, then
the unchanged T016 coverage grouping and event-wide shadow/border/fill passes.
Composition remains layer/source order, independently of start-ordered placement.
Output clipping happens during composition, including stacks outside the frame.
All active-set work is rebuilt for every requested timestamp.

## Admission boundaries and unresolved policy

| Portion | Status | Required follow-up |
|---|---|---|
| Core C4 stateless collision using existing local boxes | Implemented; Actions verification recorded on PR | Technical review and maintainer acceptance |
| Core hard/soft breaks in WrapStyle 2, empty-line metrics, NBSP, nine alignments | Existing behavior preserved | Existing T010/stage tests plus additive integration tests |
| Automatic WrapStyle 0/1/3 | Unsupported; W2 Core semantics approved in review | Final [ADR-013](../decisions/013-core-unicode-wrapping.md)/C4 confirmation of recursive rounding, Unicode data/conformance/storage/licensing, native-work preflight and C7 limits; no production wrapping yet |
| Legacy collision/placement L04–L07 | Unsupported | Authenticated xy captures, metrics, exact fixture/policy and maintainer admission |
| Historical persistent allocation L08 | Blocked | Resolve conflict with C1/C4 deterministic seeking through separate contract review; never enable implicitly |
| Core geometric effect footprint | F1 owner-approved, implemented for review | ADR-012/C4/CACHING amendment, numeric and fixed-font verification, final technical review and exact-head protected-change acknowledgment |

R001/R002 remain research checkpoints: no Legacy rule, capture or golden image
is promoted. Different border widths now affect Core collision through ADR-012;
this checkpoint makes no historical claim about collision displacement.
No blur/BE, cache framework, new font metrics or parser behavior is implemented.

## Additive validation and existing-test authorization

`crates/taiyou-render/tests/wrapping_collision.rs` exercises numeric rectangles:
all directions, minimum contact, layers, explicit exclusion, start/ID ordering,
rechecking, empty boxes, fractional/negative/offscreen positions, long stacks,
duplicate IDs, extreme arithmetic, configured reservation limits and Legacy gating.

`crates/taiyou-cli/tests/wrapping_collision.rs` uses the existing Actions fixed
Source Sans 3 font and the real ASS → evaluation → shaping → layout → geometry →
coverage → RGBA pipeline. It compares collision results with independently
specified explicit anchors derived from the existing local metrics, including
all nine alignments, margins, unequal output/PlayRes scales, hard/soft/blank
lines, NBSP, visible borders/shadows, start-versus-paint ordering, explicit
exclusion, empty events, offscreen stacks and gated automatic modes.
Fresh frames are compared byte-for-byte with forward, backward and shuffled
timestamp requests, including exact event starts/ends and the A0–1/B0.5–2 case.

The human explicitly authorized adding identical `\pos(10,10)` tags to the
two events in the existing T016 paint-order test
`prior_layer_order_keeps_visible_default_shadows_and_fill_coverage`.
Its exact assertions and expected bytes are unchanged. This preserves its
intended overlapping-pass coverage while excluding relocation; new tests cover
the same effects when collision does move events. No external comparison fixture,
reference, selection, threshold, classification, error mapping or workflow changes.

All compilation, rendering and test execution occurs in GitHub Actions.
The unchanged full T015 comparator must retain its true exit status and artifacts;
the pinned native libass oracle is independent of Taiyou results and future xy
captures. Raw paired renderer measurements are preserved under
`experiments/t017/results/f473e81/`; their measured source/environment remain
explicit, including the original larger-workload timeout. Pure solver scaling
and candidate-width workload records are under `results/7900332/` with their
source job. At 100/500/1000/5000 events the new solver's median times were
0.709/15.358/60.495/1506.555 ms, with 4950/124750/499500/12497500 obstacle checks.
Old 1000 took 14928.342 ms; old 5000 was not executed (20.846 billion derived checks).
New 5000 isolated process median RSS was 3692 KiB and vector capacities 400000 bytes;
the conservative workspace reservation was 1200000 bytes versus old 960000 bytes.
Pure solver workspace increases; the large renderer memory reduction comes from
dropping retained full layouts, not pretending intervals have no memory cost.
No production multi-worker or universal speedup claim is made. Full T017 completion
remains blocked on final W2 contract/admission gates and technical acceptance.
