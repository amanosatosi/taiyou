# T018 — ASS scalar blur candidate

Historical static-filter implementation record. The preserved 508e676 production metric/layout/shadow corrections are recorded in [BE audit](T018-BE-AUDIT.md); the current owner-requested transform/drawing extension is recorded in [implementation notes](T018-TRANSFORMS-DRAWINGS.md). Statements below about transforms being unsupported describe the earlier candidate.

Integration base: `5c6672660b50a69a98518a2fc024abd27559807d` (merged PR31).
This implements static `\blur` and real-valued `\be`, not animated transforms,
LegacyXyV1 admission or an authenticated xy pixel oracle. No dependency added.

## Source authority

Read-only local pinterf pfmod 3.2.0.810 archive from LOCAL_REFERENCES, not another
fork. Exact equivalence to locked commit 25cf3e8433b3a83a5489e47a2c335516492bb815
is unverified. No refetch, reference build or fabricated capture.

| File | SHA-256 | Relevant functions/lines |
|---|---|---|
| RTS.cpp | dd4ae1950606979b255a742279f4b64a4cd90ce8a0647135980fe08e02618ffb | PaintFromNoneBluredOverlay 213–235; command suffix extraction 2148–2182; CMD_blur 2291–2298; CMD_be 2312–2319; CMD_r 2620–2625; target mapping 1856–1874 and 3365–3369 |
| Rasterizer.cpp | 29b113f2c927793005afd25d8d0c861a430b55462f6e8e057e1bbaceccf08334 | GaussianCoefficients::init 96–164; be_blur 568–671; Rasterize 788–835; IsItReallyBlur 884–894; Blur 1044–1084; GaussianBlur 1086–1165; BeBlur 1167–1249 |
| xy_filter.cpp | f4ef0161a7f11cc93ea86987c4c632f0852342e4690bbb3bc6f2d3ee6e22808c | Gaussian float filter/conversion 39–132, 780–940; XyRounding 947–1036; fractional filter 1078–1116; xy_calculate_filter 1248–1292; xy_be_blur 1340–1387 |

The Rust scalar implementation is independently expressed from these filter
equations and coverage observations. No GPL source or SIMD implementation is
copied/translated, linked or vendored. Existing source-license policy remains.

## Implemented behavior

Suffix numeric prefix parsing, omitted-value original-style restoration,
raw finite signed Gaussian targets, source ordering and style resets. Static
evaluation clamps negative Gaussian to zero after reading the target, leaving
the typed operation intact for future animation. Style.blur remains nonnegative.
BE retains its signed
real state; nonpositive BE does not enable an edge filter. Nonfinite strengths
are rejected by the existing parser recovery policy. `\t` remains unsupported;
tolerant rendering may show the existing exploratory static state only.

The effective gate is BE>0 or Gaussian>double(0.333333f). If BE enables the stage,
any positive Gaussian executes first. Gaussian width is ceil(3*sigma)|1 and radius
is width/2. Symmetric exponential samples and normalized float coefficients,
float intermediates between axes, scalar +0.5 byte conversion. Gaussian filters
expanded E when an outline exists, otherwise F. F stays unfiltered with outline;
subtract F from filtered E afterward. Shadow uses its translated selected E/F
mask and the same filtering. Existing event-level shadow/border/fill paint order,
fractional shadow raster phase and transparent-shadow fast path stay intact.

BE padding is round(target_axis*sqrt(be/4)); integer passes are truncation of
the [1,2,1] outer product /16, with the outer allocation ring unchanged. Scaled
strength is float(be)*0.5*(target_x+target_y). The residual uses coefficient
floor(256*f/(1+3*f)), center 256-2*coefficient, horizontal then vertical passes,
and the actual scalar XyRounding expression, not an assumed IEEE tie rule.
Owner instruction explicitly selects xy's smaller padding and resulting edge
truncation over a mathematically complete N-pixel BE halo. No extra guard ring.

xy target scale is output/original-video (or explicit LayoutRes), not font scale,
PlayRes scaling or SBAS. A positive declared LayoutRes pair takes precedence
over the host original-video dimensions; otherwise the host original size (or
content size when omitted) supplies the layout size. Blur target scale is output
content divided by that layout size, independently of SBAS. BlurSpec carries it.

Full Gaussian/selected BE allocations occur before filtering and final viewport
clip; source masks stay immutable. Existing radius/pass/dimension/pixel/output
and transient limits preflight combined support, scratch and fallible allocations.
Errors publish no frame; typed memory pressure remains eligible for existing
optional-staging fallback, fixed radius/pass limits do not retry.

## Evidence boundary and checks

Taiyou uses FreeType A8 rather than xy's native overlay coverage. Reference SIMD
float accumulation/conversion and fractional-pass vector/tail rounding may differ
from this explicit scalar arithmetic. Source-derived kernels and actual rendering
do not establish end-to-end pixel identity. No LegacyXyV1 capability is promoted.

Additive tests cover real mask impulse values, threshold, integer/fractional BE,
combined ordering, immutable source, checked limits, original-style resets,
rendered glyphs/outlines/shadows, frame edges and fresh/repeated/backward seeks.
Existing test edits are owner-authorized recognition of typed BE in the
anisotropic unsupported-tag test (all four anisotropic assertions remain), and
replacement of obsolete static-blur failure triggers by unsupported animated
blur in three CLI tests. All their error/output/filesystem assertions remain.

Actions evidence, frozen blur-case deltas and final complete comparison are
reported in the PR. No performance claim, changed reference or tolerance.

Focused implementation run: [39431029173](https://github.com/amanosatosi/taiyou/actions/runs/39431029173),
implementation `c88535f31e8706730f575cb851586785013d6a91`. Parser/evaluation,
five scalar mask tests and three actual ASS render tests passed, including
outline/body selection, shadows, frame edges, failure recovery and seeks.
All 14 frozen blur cases were compared: no exact matches, all unsupported,
no infrastructure failures, regressions or crashes. These are supplemental
results; the PR records the final complete corpus run separately.

The subsequent [four-frame BE audit](T018-BE-AUDIT.md) distinguishes rendered
pixel mismatches from the frozen inventory label. It records the actual masks,
independent integer-equation checks, measured upstream metric/layout/shadow
contributions and the unresolved xy coverage/overlay prerequisite. The four
static BE frames are not pixel-compatible and remain open before acceptance.

## Stale T017 expectations corrected after positive cell sizing

The focused [metric review](https://github.com/amanosatosi/taiyou/actions/runs/39593907687/job/119201385792)
changes no production renderer code or external reference. The pinned CJK witness
retains all 202 glyph identities and offsets; its advances change from em20 to
13.8125 after native Q6 cell conversion (UPEM 1000, Win cell 1448, em Q6 884).
The experiment verifies font bytes, hmtx-derived advances, exact old/current
witness hashes, repeated output and corrupt/missing/mismatched evidence. Historical
performance samples remain immutable. Their pixels are compared within the old
metric policy; current repeated/staging pixels are compared within the corrected
policy. Cross-policy equality would require restoring the disproven em semantics.

The 500-event segment test has these measured boundaries (event, preparations):

| Border | Old em geometry control | Positive cell geometry |
|---|---|---|
| 0.5 | 441, 444 | 423, 426 |
| 3 | 235, 238 | 225, 228 |

Changing only size back to the old em restores both old boundaries. The current
1M budget expires at the next input geometry, before its stroke. Increasing only
the caller segment budget to 1,010,000 moves exhaustion to events 427/227, where
native stroke diagnostics report 2063/2071 points against 271/1138 remaining.
All original one-attempt, original-error/event/span, repeatability and 4M-success
pixel/diagnostic equivalence assertions remain. These counts describe geometry,
not an ASS requirement. The default limit and resource classification are unchanged.

The actual embedded-font `efont-1000` font has UPEM 1000 and Win ascent/descent
863/144 (SHA-256 `fde6c58b1eafe11c24de4f1860a6e6539df4d009b792f4929e8759f27b76472a`).
At requested 48 its corrected scale is 0.047671875; restoring only old em sizing
uses 0.048 and reduces alpha MAE from 45.48030 to 42.95213. Restoring only the old
coincident shadow yields 44.75749. These controls isolate the intentional sizing
and zero-shadow changes; they do not reconstruct all old baseline-rounding code.
The earlier 43.35202 and current 45.48030 observations both retain 384 changed
pixels and bounds [296,411,345,461]. The source remains a pixel disagreement,
not an exact match; no fixture-specific correction is justified. Its legacy
YCbCr metadata blocks strict rendering, so these MAEs are explicitly **tolerant
diagnostic** measurements and never compatibility passes. The focused test also
checks that the normal embedded-font path reproduces the current diagnostic bytes.
## Ordinary placement, rotation and fade batch

The source-established follow-up implements move/org/frz/fr/fad/fade through the
existing evaluator, GeometryMap and paint/composition paths. Reference is the
local pinterf pfmod 3.2.0.810 RTS.cpp (SHA-256
`dd4ae1950606979b255a742279f4b64a4cd90ce8a0647135980fe08e02618ffb`;
archive/upstream-commit equivalence remains unverified).
Parser/effect locations: 2090-2125, 2372-2399, 2446-2453, 2552-2605;
render timing/slot application: 3157-3229; origin collision exclusion: 3257;
paint alpha MulDiv: 1258-1264, 1287-1294, 1306-1313.
C3 describes the exact boundaries, shared/separate first-wins slots, native
coordinate truncation and fade precedence. The subsequent owner-directed batch
below extends x/y rotation, shear, clipping, karaoke and effect axes. Frozen references and
comparison arithmetic remain unchanged. No historical pixel-parity claim follows
from source inspection or the additive tests.

## Ordinary ASS coverage batch

The owner requested working clip/iclip, frx/fry/fax/fay, xbord/ybord/xshad/yshad,
u/s, SSA a and k/K/kf/ko/kt before pixel-mismatch tuning. All use the existing
evaluation, geometry, effects and composition stages; fe remains unsupported.

Local pinterf pfmod 3.2.0.810 source locations (RTS.cpp hash recorded above):

| Behaviour | Reference |
|---|---|
| Raw signed hexadecimal alpha, narrowing after interpolation | ParseSSATag 2181–2191; CreateSubFromSSATag 2251–2278 |
| SSA alignment, first alignment slot | CreateSubFromSSATag 2287–2294 |
| Rectangular clip assignment/animation; first vector clip slot | CreateSubFromSSATag 2330–2364; RenderEx 3520–3564 |
| X/Y rotation and shear | CreateSubFromSSATag 2400–2444; CWord::Transform 420–556 |
| Karaoke clock and word ownership | CreateSubFromSSATag 2512–2550; ParseString 1966–2018 |
| Karaoke paint sweep, ko outline suppression | CLine::PaintAll 1243–1248, 1317–1344 |
| Decoration switches and independent effect axes | CreateSubFromSSATag 2640–2645, 2679–2725 |

Rectangles animate their device-space integer edges; vector clips are immediate
and do not rotate/move with text. Both slots intersect during composition, after
effects. Numeric rotations, shear and effect axes use ordered CalcAnimation;
decoration, alignment and karaoke commands are immediate inside t. Raw alpha
targets remain signed native LONG values until the evaluated transparency is
narrowed to a byte. The three alpha-clipping frames reached strict pixel
comparison in focused Actions run 39641044733: zero execution failures and three
pixel mismatches. No visual match is claimed.

The existing FreeType round-stroker baseline is retained for equal radii.
Positive unequal radii normalize each axis before round stroking and restore it
before rasterization. A zero axis instead unions nonzero-winding eighth-pixel
spans along the other axis, before accumulating coverage; it uses neither an
A8 maximum filter nor an invented small nonzero radius. Relevant source is
Rasterizer.cpp ScanLineData 3112–3358 and CreateWidenedRegion 3369–3390
(SHA-256 `29b113f2c927793005afd25d8d0c861a430b55462f6e8e057e1bbaceccf08334`),
and xy_widen_region.cpp gen_left_arc 452 onward
(SHA-256 `af0034118448829065a6bdd6748bb8943aac4a62fbf852a59cd869d63486dd7c`).
This remains an A8 composition backend; source-derived property semantics do
not establish GDI/xy outline, scan-span, compositing or frozen-pixel identity.
No authenticated renderer capture or archive/upstream-commit equivalence is
claimed. Existing blur kernels and all frozen corpus assets remain unchanged.

Coverage-sweep verification: focused Actions run
[39679050047](https://github.com/amanosatosi/taiyou/actions/runs/39679050047)
passed 15 parser tests, five evaluator tests, ten native coverage tests, the
zero-axis widening test, 14 renderer safety tests and native Clippy. This includes
CFF/TrueType underline and strikeout coverage monotonicity, vector-clip frame
coordinates, karaoke word boundaries across empty override blocks and seeking.
The three alpha-clipping frames rendered strictly and remained pixel mismatches
(zero execution failures); their comparator exit remained nonzero. The formatter
patch was generated in that Actions run and applied to the candidate. The
decoration regression locates the existing pinned shaping fonts through
RUNNER_TEMP so the focused and full native runners use identical font bytes.

## Layout, unknown commands, vertical fonts and opaque boxes

LayoutRes is storage/layout space, never a replacement for PlayRes coordinates.
STS.cpp 2066–2074 reads it; RTS.cpp RenderEx 3365–3404 selects it only when both
axes are positive. GetSubtitle 2912–2961 constructs the font at
`fontsize * layoutY / PlayResY`, spaces at `spacing * layoutX / PlayResX`, and
SBAS effect axes at their respective layout ratios. Final target scaling is
output/layout. Shaping returns normalized script-space metrics/advances;
geometry rotates/projects in layout pixels before applying target scale. Script
positions, margins, movement, origin and drawings retain PlayRes coordinates.
Rectangular clip edges animate/round in layout pixels before target scaling;
vector clips remain independent frame geometry. Partial/nonpositive LayoutRes
pairs leave host storage size active. Encodings 0 (ANSI/default Unicode font
request) and 1 use the supplied Unicode face; no legacy cmap/codepage emulation,
style Encoding >=2, fe, YCbCr mangling or SSA V4 is admitted by this batch.

ParseSSATag 2133–2220 dispatches the longest known prefix and consumes CMD_COUNT;
its evaluation default does nothing. Thus `board1` dispatches b with `oard1`,
whose wcstol value is zero (RTS.cpp 2320–2328). Unknown commands are consumed,
not claimed as missing renderer features. Known unimplemented fe/fsc and
unresolved named resets remain explicit unsupported operations. Original text
spans retain override-block gaps, including ignored/empty blocks, so karaoke
word ownership still changes at those boundaries.

Vertical fonts are delegated unchanged to LOGFONT by STS.cpp 3719–3730 and
RTS.cpp CMyFont 115–141, then TextOut/GetPath by CText::CreatePath 646–693.
The narrow Windows probe uses that same path, existing pinned fonts, and records
real glyph IDs, advances, metrics and contour coordinates; it is not a desktop
screenshot or an authenticated xy binary capture. Runs
[39682552397](https://github.com/amanosatosi/taiyou/actions/runs/39682552397) and
[39682853492](https://github.com/amanosatosi/taiyou/actions/runs/39682853492)
distinguish TrueType IPAexGothic from CFF Noto Sans CJK JP. The former retains
horizontal Latin but rotates CJK/fullwidth contours and substitutes vertical
punctuation; the latter rotates Latin too and uses its rotated cell bounds.
The existing validated face is resolved after removing @ from lookup only;
the semantic flag remains. Native vert/vkna substitutions, vertical advances and
an owned `(advance + typoDescender - y, x + typoDescender)` outline map reproduce
that horizontal TextOut run representation. This is not modern TTB paragraph
layout: baseline/run progression remain xy's X progression until ordinary ASS
rotation transforms the word. All normal geometry, painting, clipping, karaoke,
font resets and drawing transitions remain shared. No universal GDI version,
fallback or pixel-parity claim follows from the two controlled font witnesses.

Standard BorderStyle=3 follows RTS.cpp CreateOpaqueBox 556–573 and
xy_overlay_paint_machine.cpp PaintOutline/PaintShadow 116–134. The background
is the outline color/alpha, made from each compacted compatible word segment's
advance and cell height on its own line. Outline X/Y padding rounds in native
eighth units; the copied polygon style reapplies font scale, shear, rotation and
projection, including the reference's observable double scale of word dimensions.
Body and box blur independently; shadow uses the box polygon. The background
is not a glyph-outline subtraction ring. Zero outline disables the background
pass, but a visible nonzero shadow can still use the unpadded cell box. Source
alpha/fades, clipping, margins and placement use ordinary event machinery.
Whitespace compaction and blank-line metrics stay unchanged. Collision includes
visible box geometry and shadow support before final raster quantization, with
caller limits enforced during the prepass. BorderStyle=1 keeps its existing path.

Phase 1 verification at `8f2f6045e1cae5599da21b8d6886395e91bebc24`:
[focused Actions 39685908253](https://github.com/amanosatosi/taiyou/actions/runs/39685908253)
passed parser tests (15), the seven layout/vertical/opaque-box integration tests,
14 renderer safety tests and native Clippy. The 20-witness external comparison
was truthfully red: 14 strict pixel mismatches, six blocked by the independently
deferred non-None YCbCr conversion, no execution failures, infrastructure errors
or crashes. All eight scaling_playres witnesses (including both opaque boxes),
all five karaoke-and-runsplits witnesses and the vertical karaoke witness reached
pixel comparison. The embedded-font, three 357-k-and-kf-desynced and two zero-area
witnesses still stop at YCbCr; LayoutRes no longer blocks them. The desynced
fixture also reports an event effect in its tolerant diagnostic attempt. These
are remaining feature limitations, not successful visual comparisons. The frozen
187 assets and 104-entry inventory verified unchanged before and after this run.
The final per-line translucent-box overlap regression also passed in
[39686721095](https://github.com/amanosatosi/taiyou/actions/runs/39686721095)
at `7c5f1af73febf2f1c09b06f73f43a8245027bbdc`, with the same 20-case outcomes.

## Source-mask correction

[Identical-path Actions 39688011319](https://github.com/amanosatosi/taiyou/actions/runs/39688011319)
at `8c8a08eccde68df9b2be3f595bf772af1bbbb40e` separates geometry from coverage:
the very same one-pixel rectangle with a 1/16-pixel X edge produces FreeType A8
`[240,16]`, versus xy's eighth-crossing accumulation `[56,8]` in native 0..64
units (`[223,32]` only after conversion). A diagonal triangle independently
exercises the half-column inclusion rule (36 covered eighth cells, not analytic
half-area). Both results precede fonts or blur. Native allocation includes the
extra eighth-unit extent and right/bottom pixel; integer BE preserves its actual
small padding and untouched edges. These differences cannot be fixed by changing
Gaussian sigma or BE coefficients.

The renderer now independently scan-converts supplied contours, preserving raw
0..64 coverage until filtering and paint-alpha conversion. This is not an A8
quantization shim. Shared fill, shadow and opaque-box paths use it; existing
FreeType stroke contours are also exported as owned paths and passed through it.
The FreeType outline/stroker geometry is not claimed equivalent to GDI outlines
or xy's span-region widening. Independent saved GDI observations (run 39454311798,
artifact 12279509017, exact digest checked by `source-paths.py`) measure that
remaining geometry difference separately on period/plus/paragraph, at multiple
phases and BE strengths. No authenticated xy-render capture is claimed.

The owner explicitly authorized replacing only the first-render C oracle's
disproven FreeType A8 assumption. Its font metrics, outline geometry, ten
alignment/offscreen placements and C6 blending checks remain independent and
unchanged. The C oracle implements row sampling/nonzero winding independently
of the production edge/span implementation and does not call it. Separate
FreeType-specific raster stage tests remain intact. Rasterizer.cpp source:
AlignLeftTop 3058–3094; ScanLineData 3112–3358; Rasterize 789–885;
FillAlphaMashBody_c 2320–2334 (file hash above). Frozen corpus images, metrics,
tolerances, classifications and the libass oracle are unchanged.

## Verified source-mask controls and Gaussian limitation (2026-10-22)

The authorized independent C first-render oracle now compiles with
`-Wall -Wextra -Werror`. The unused sampling variable was removed; the supplied
`FT_Outline` control uses the pinned header's `FT_Byte` tags and `FT_UShort`
contours. No compiler warnings were suppressed. Actions
[39822252375 / 119976275455](https://github.com/amanosatosi/taiyou/actions/runs/39822252375/job/119976275455)
at `4c21a74074656fde5bc4e647c8969c360a9fdba1` passes all 14 renderer_slice
checks, six source-mask tests, the actual supplied-GDI invocation, native
Clippy, and the focused ordinary ASS/effect/transform/drawing regressions.
The job is red because the real 15-case comparator returns failure for pixel
mismatches. Its earlier missing test-font environment was workflow wiring,
corrected without changing production or test assertions. The first Fast run
for the new diagnostic test required formatting; the Actions-generated format
patch was applied to the source.

The additional `gaussian_source` control reconstructs the real frozen blur+t
path at every selected time with the same evaluated state, geometry and paint:

- Its FreeType-A8 variant exactly reproduces the `5bbde64d` comparator metrics.
- The native-64 variant equals production RGBA byte for byte.
- Cropping only zero source padding equals production byte for byte, ruling out
  that padding as the demonstrated cause of this Gaussian change.
- Converting the same eighth-grid source to A8 **before** Gaussian reduces much
  of the additional alpha error, but is a diagnostic counterfactual, not a fix:
  pinterf `Rasterizer::GaussianBlur` (1086–1165) passes raw overlay bytes;
  `xy_byte_2_float_c/sse` (xy_filter.cpp 636–716) converts bytes directly to float;
  `xy_gaussian_blur` (874–940) retains float values between axes and narrows back
  to byte before paint. There is no x255/64 normalization there.

For example, at 1000 ms, alpha MAE in comparison RGBA16 units is 100.34833 for
FreeType A8, 109.92961 for native xy coverage, and 103.28027 for the diagnostic
pre-Gaussian A8 conversion. At 1900 ms those values are 109.21911, 123.39427 and
112.24261. Thus the native-unit filtering/quantization boundary accounts for
most of the newly increased Gaussian error; identical-mask zero padding does
not. Geometry/raster differences account for the remaining counterfactual
error; these controls do not establish GDI equivalence.

Every frozen blur+t frame contains all 256 eight-bit alpha levels, while the
current native-unit reconstruction contains 65. This is a measured output
precision discrepancy, not evidence identifying the binary that created those
references. Overlapping paint may generate additional levels in other scenes.
The pinned independent libass oracle reproduces the references; that is not an
xy rendering capture. Source-only scalar and SIMD Gaussian conversion also
have different rounding/accumulation paths (xy_filter.cpp 780–871). Exact
binary/output provenance and reduced identical-overlay captures would be
needed to decide a conflicting historical-build interpretation. Do not move
normalization before Gaussian, change sigma, alter BE, or change references
merely because the counterfactual scores better.

The measured residuals remain real pixel mismatches. Compared with the exact
`5bbde64df55015f1e94d86236551ca5e1c8ed08d` baseline:

| Frozen frame | Changed pixels before → after | Alpha MAE before → after (RGBA16) |
|---|---:|---:|
| be-0500 | 8750 → 9119 | 37.02674 → 31.06648 |
| be-1500 | 13959 → 12556 | 69.18494 → 45.27965 |
| be-2500 | 15595 → 13397 | 74.63869 → 29.68564 |
| be-3500 | 15985 → 13688 | 105.11835 → 31.08683 |
| blur+t-1000 | 55516 → 56851 | 100.34833 → 109.92961 |
| blur+t-1100 | 58566 → 59778 | 100.41525 → 110.03187 |
| blur+t-1200 | 61754 → 63060 | 108.51129 → 117.90089 |
| blur+t-1300 | 63921 → 65535 | 105.24257 → 116.44349 |
| blur+t-1400 | 65977 → 67621 | 103.80551 → 115.89094 |
| blur+t-1500 | 67957 → 69677 | 107.56574 → 119.00813 |
| blur+t-1600 | 69992 → 71850 | 105.74319 → 118.48824 |
| blur+t-1700 | 72314 → 73984 | 109.29353 → 122.78175 |
| blur+t-1800 | 73625 → 75571 | 108.19807 → 121.47051 |
| blur+t-1900 | 73992 → 76852 | 109.21911 → 123.39427 |
| anamorphic blur-0000 | 81905 → 82081 | 117.97685 → 124.93585 |

All 15 execute in strict mode: zero exact, 15 mismatches, zero unsupported,
execution failures, infrastructure errors or crashes. Static BE alpha error
improves strongly; its first frame's changed-pixel count worsens. Gaussian and
anamorphic errors worsen and are not described as passes.

Raw frame variants, group bounds/strengths, alpha-level sets and metrics are in
artifact `12444768351`, SHA-256
`cabc81079d48c665af8fb9aa919d758b33488f4096a9afbe9b82ad38461a352b`.
The saved GDI-versus-FreeType outline differences remain at most one Q6 unit for
the measured glyphs. Shared-raster plus output is identical; period and paragraph
have small residuals, generally reduced by BE. This does not explain away the
much larger identical-geometry FreeType-A8/eighth-grid difference. Remaining
source-mask gaps include exact curve/phase behavior beyond these controls and
FreeType round-stroker versus xy span-region border widening. No GDI outline
replacement or filter retuning is claimed.

### Dense-scene reservation correction found by final platform validation

Candidate `e32f217` completed the full comparison with 1 exact, 83 pixel
mismatches, 17 genuinely blocked visual frames, and no execution, infrastructure
or crash failures. All 104 inventory entries and 187 assets were preserved.
The independent libass job passed. This is recorded evidence, not a permanent
count requirement.

That candidate's Linux native workspace test exposed a real resource regression
in the unchanged `adaptive-256-multiglyph` experiment: both 32 MiB variants
failed native shaping at EventId(96), source 10461..10566, while 128 MiB full
and minimal variants completed with equal raw hash
`362e23a149d753bd00da31fe2b9902a84c4b4cd032808cd93c809b39cb69c8cc`.
The Windows/macOS adaptive-staging assertion also failed because no successful
partial-retention result remained in the tested budget table. Those failures
were not suppressed or reclassified.

The larger source-defined raster extents exposed unnecessary retention of
finished masks wholly outside the frame. The renderer now releases their
unique mask reservations after all border/shadow/filter work, before retaining
the draw list. Collision still receives the entire active set, and all
preparation, geometry, native work and segment-limit checks still execute.
An offscreen body is retained if its shadow or filtered halo reaches the frame.
Shared coincident-shadow/box masks release their reservation only once. No
limit or protected test assertion changed.

The additive `source_mask_budget` integration tests exercise the exact reduced
256-event workload, 32/128 MiB output/diagnostic equality, repeated random time
requests, and offscreen-body witnesses with visible shadow and Gaussian halo.
Final platform and comparison results must be read at the final candidate;
`e32f217` remains the preserved before-correction evidence.

Across the 64 visual cases that already rendered at `5bbde64d`, this eighth-grid
candidate changes 61 raw RGBA hashes. Changed-pixel counts improve in 33, are
equal in nine, and worsen in 22; alpha MAE improves in 11, is equal in three,
and worsens in 50. These renderer-wide changes are reported explicitly: the
identical-geometry controls establish the source coverage correction, not a
universal improvement against the independent frozen libass references.
