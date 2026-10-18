# T014 CoreV1 candidate fixtures

first.ass is independently authored for Taiyou. It uses the unchanged, licensed
Source Sans 3 face from build-support/shaping-fonts.proposed.json. The real Taiyou
CLI produces first.png and first.rgba at 500000 microseconds, 320 by 180 pixels.
Raw RGBA is authoritative premultiplied encoded-domain output. PNG is straight
RGBA using C6, without color-space conversion. These outputs are candidate stage
evidence, not approved goldens, external compatibility references or T005 trust.

renderer_slice constructs additional ASS inputs to test timing, nine alignments,
pos, alpha, ordering, fallback, clipping, lifetimes, diagnostics and reservations.
On Linux, renderer-reference.c independently loads H from the pinned font, uses
design-unit metrics, calculates anchors/baselines and quantizes its contours,
then calls FT_Outline_Get_Bitmap. These A8 expectations do not call Taiyou code.
Exact expected RGBA is calculated from these independently obtained coverages
with C6 integer arithmetic. This is a candidate-side native API reference whose
numeric/geometry expectations still require technical review.

The slice supports CoreV1, hard breaks, UTF-8, static placement, scale, solid fill
and explicit fonts. It has no wrapping or collision adjustment (T017), effects,
drawings, movement, karaoke, advanced transforms, persistent cache or parallelism.
Strict requests fail on unsupported operations/effects. Tolerant evaluation
follows C3, retaining prior state for unsupported operations and publishing
explicit diagnostics; unsupported style effects/decorations are omitted with an
explicit fill-only diagnostic. It does not claim an effected subtitle is supported.
