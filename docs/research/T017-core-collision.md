# T017 — Core collision implementation checkpoint

Integration base: `d5a545ac49aabbfc23e3895231d407da576e276f`.
Scope: implement existing C4 CoreV1 collision, preserving T010 WrapStyle 2 and
the merged T016 border/shadow pipeline. This is not completed Legacy-compatible
T017. Candidate SHA and exact Actions jobs are recorded in the implementation PR.

## Implemented policy

The renderer evaluates, shapes and locally lays out the complete active set
before resolving placement. Local layout remains immutable and independent of
absolute displacement; moving an event does not reshape its glyphs. Reservations
include all simultaneously retained local artifacts, collision descriptors,
ordering and placement outputs. Local reservations release after each event's
geometry and passes are finished. Native path and retained-mask accounting remain
in the existing geometry/raster stages.

The resolver uses script-space `LocalLayout::collision_bounds()`, distinct from
logical and ink bounds. This is the existing union of the logical box and any
ink box, with no added stroke/shadow/bitmap padding. It translates that local box
by the natural alignment anchor, including inherited margins and PlayRes.

Each layer is independent. Eligible events are placed in `(start, EventId)`
order; bottom alignment moves up and middle/top moves down. Explicit positions
are neither moved nor obstacles. Zero-width/height boxes occupy no region;
blank lines inside a nonempty block still contribute the existing style metrics.
Every jump clears the first intersecting placed obstacle in canonical order,
then rechecks all obstacles. Open forbidden intervals of anchor Y values give
half-open edge contact without epsilon or pixel rounding. Each jump permanently
leaves an obstacle's interval in the chosen direction, so N prior obstacles
require at most N jumps and a final clearance scan. There is no viewport clamp,
history cache, arbitrary displacement or iteration cap. Nonfinite/unrepresentable
arithmetic and allocation/reservation failures return explicit errors.

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
| Automatic WrapStyle 0/1/3 | Unsupported | Approved executable break examples, font metrics, tie rules and boundary reshaping evidence; R001 L01–L03 are proposals only |
| Legacy collision/placement L04–L07 | Unsupported | Authenticated xy captures, metrics, exact fixture/policy and maintainer admission |
| Historical persistent allocation L08 | Blocked | Resolve conflict with C1/C4 deterministic seeking through separate contract review; never enable implicitly |
| Border-inflated collision boxes | Unresolved task/contract conflict | T017 asks for inflation; C4 gates border inflation on R001. Decide whether to add a separately reviewed Core padding rule (including units, transforms, per-line/per-event extents and rounding), or retain existing Core boxes and admit historical inflation only with captures. No padding formula is chosen here. |

R001/R002 remain research checkpoints: no Legacy rule, capture or golden image
is promoted. Different border widths continue to affect T016 raster support;
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
captures. No performance or multi-worker claim is made. Full T017 completion
remains blocked on the wrapping and border policy acceptance gates above.
