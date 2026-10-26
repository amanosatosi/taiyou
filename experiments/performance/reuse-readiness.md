# Existing boundaries and future feature load

This is an implementation review, not a new tag contract. No Mangetsu tag is
implemented by this phase. Current caches retain paint-independent coverage and
filtered coverage, not completed RGBA frames. Lookup uses full structural input
equality, not document-local IDs or unverified hashes. Native handles never enter
retained values.

| Future operation | Existing reusable work and remaining concern |
|---|---|
| Gradients, positional gradients, color cycling, polka | Paint and paint-coordinate mapping follow coverage; changing paint need not rerasterize equal geometry. Evaluation can still change coverage grouping or collision visibility. |
| Multiple borders and per-border properties | Fill coverage is separate from border widths and filters. Equal border requests reuse masks. Base outlines are currently extracted again on misses; measured outline cost is small, but a future many-border benchmark should revisit that boundary. |
| Furigana | Resolution/shaping/local layout precede coverage. Additional text/layout changes cannot be mistaken for an equal Q6 path. There is no whole shaped-run cache yet; today's shaping measurements do not rule out future metric or outline reuse. |
| Distort/warp, curved text | Newly transformed Q6 paths invalidate masks; shape and local layout remain separate inputs upstream. Continuous deformation can have a low raster hit rate, so miss cost remains important. |
| Static image resources | No decoder or image tag yet. Resource identity/decoding must remain distinct from placement/paint, with its own bounded ownership. The mask cache does not pretend to cache images. |
| Blend/composition | DrawOps consume immutable masks and preserve ordered composition. A composition-only change has no place in raster keys. New blend modes need their own arithmetic tests. |
| Native scrolling/chat layouts | Layout and placement precede geometry. Integer motion reuses local coverage where mapping/phase agree; fractional motion still changes raster phase. Native layout decisions remain serial and deterministic. |
| Animated clips | Clips are applied during composition, after full effects. A changing clip does not modify retained source masks. Clip-mask retention is a separate opportunity if profiling justifies it. |
| Deterministic random geometry | Resolve explicit seeded geometry before mask lookup. Equal complete paths can reuse; schedule or map iteration must never choose geometry. |

The present expensive changing-frame path is discrete span expansion/union,
not shaping. Its duplicate-row reduction is a measured algorithmic improvement
within that boundary. Warm filtered frames instead concentrate work in ordered
composition. Those findings guide parallel work; they do not justify a new
general scheduler, a finished-frame cache, or speculative future tag semantics.
