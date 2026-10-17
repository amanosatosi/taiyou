# T011 owned outlines and Core 2D mapping candidate

Integration base: `284ac1d272d8e16af14e8021b3f9e359fe07f697` (merged T010).
This candidate remains unmerged for shared-interface and native/FFI technical review.
CI evidence belongs to the exact PR head; implementation alone is not acceptance.

## Ownership and extraction

`FontContext::outline` rechecks SHA-256, collection face index, glyph range and
units per em. It uses pinned FreeType 2.14.3 with `FT_LOAD_NO_SCALE |
FT_LOAD_NO_HINTING | FT_LOAD_NO_BITMAP | FT_LOAD_IGNORE_TRANSFORM` (2059).
It does not set NO_RECURSE: compound glyphs are expanded by FreeType. No bitmap
rendering, LCD mode, platform font lookup or second font parser is introduced.
The [FreeType glyph retrieval reference](https://freetype.org/freetype2/docs/reference/ft2-glyph_retrieval.html#ft_load_xxx)
defines those flags; the [outline API](https://freetype.org/freetype2/docs/reference/ft2-outline_processing.html#ft_outline_decompose)
defines decomposition into line, quadratic and cubic callbacks.

Native outline counts, pointers, contour endpoints and coordinates are checked;
the callback vector is reserved before entering C. Copy callbacks cannot grow
it or unwind on reservation failure. Face/library ownership remains per context,
not Send/Sync. Native pointers and callbacks stay in `taiyou-font/src/native.rs`.
Owned paths survive all native owners. Outline identity contains font digest,
face, glyph and extraction revision, without paint or placement. Variable-font
coordinates and synthetic decorations are not introduced in this task.
Even-odd native outlines and non-vector glyphs return explicit unsupported errors.
Instruction-dependent faces marked `FT_FACE_FLAG_TRICKY` also return unsupported:
FreeType documents that NO_SCALE can otherwise produce meaningless outlines.
Supporting those faces requires a separately reviewed unhinted extraction policy.

## Coordinate handoff

`LocalLayout` now retains its additional local x/y scale; the existing constructor
means unit additional scale. T010's positioned origins already include HarfBuzz
offsets, advances and local scales. T011 uses those origins unchanged and applies
font-size/UPEM and ASS/additional scale to the **contour** exactly once.
Font y-up converts to script y-down once, then z rotation acts about the event
origin, then content width/PlayResX and height/PlayResY map independently.
`GeometryPlacement` supplies the alignment anchor explicitly; it does not guess
viewport margins or perform collision placement. Default rotation origin is that
anchor. A host may supply an absolute script origin. The integrated evaluator's
`Placement::origin()` is still None: parsed org support belongs to a future input
extension, not an invented coordinate or a new tag in T011.

Q6 quantization uses C1 nearest, ties away from zero. Whole-pixel decomposition
uses Euclidean division: -1 tick is offset -1 and phase 63. Device commands retain
the residual fractional placement; consumers add the integer offset **only**,
not phase a second time. Bounds are absolute Q6 conservative control-point
bounds, not exact curve ink bounds or layout/collision rectangles. No clipping
occurs outside the viewport. T012 must consume actual curves with nonzero winding.

Integer translations preserve residual commands when quantization is equivalent,
as K2 requires. At a half-tick crossing zero, C1 ties-away rounding itself is not
integer-translation invariant; the candidate retains the correct absolute rounded
coordinates rather than claiming a reusable contour in that exceptional case.
No cache or alternate rounding rule is introduced.

## Verification scope

Numeric expectations use elementary signed matrix arithmetic and Q6 values.
Native tests use the existing pinned Source Sans 3 CFF, Indic TrueType and
regular/bold collection fixtures, including real compound detection via FreeType.
The parser/evaluator/shaper/layout integration checks decoded source ownership,
scale and cardinal-rotation order, whitespace and lifetime after context drop.
Fresh contexts must produce identical owned geometry. No Taiyou-derived golden
font output or protected reference is added or rewritten.

All compilation/tests run on Actions. The focused workflow precedes the existing
three-platform native matrix, workspace tests/Clippy, exact MSRV, C ABI checks and
Linux native ASan/UBSan/leak checks. The sanitizer executable inventory is unchanged:
new native outline tests are in the already instrumented font unit executable.
Candidate-side unsafe checks remain distinct from independently trusted validation
and GitHub protection. T005's outstanding configuration/review is not completed here.

No performance claim, rasterization, mask allocation, effects or PNG output.
T012 begins only after this T011 candidate is reviewed, accepted and integrated.
