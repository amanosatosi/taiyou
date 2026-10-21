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
PlayRes scaling or SBAS. The standalone renderer treats output as original video
and has no admitted LayoutRes/resampling host: target=(1,1). Blur is in device
pixels even when SBAS is enabled. Host target scaling is explicit in BlurSpec.

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
