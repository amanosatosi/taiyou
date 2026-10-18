# R002 — Coverage, stroke and blur research checkpoint

Status: **research checkpoint**, proposed and unaccepted. Integration base:
`77143fb259ac5423488975d2a64bafb9a6b93394`.
**0 approved Legacy rules, 0 enabled capabilities, 0 capture-verified rules.**
Source-observed behavior is distinguished from pixel evidence throughout.
R001's 30 proposals remain unchanged and unapproved. This checkpoint does not
satisfy R001's admission prerequisite for dependent Legacy implementation.

## Bounded investigation and evidence

Two focused sessions only:

1. Inspect actual T012/T015, R001, C5/C6, compatibility/testing/reference maps,
   ADR-002/011 and the supplied pinterf source. Record selected-file hashes and
   trace coverage, widening, mask selection, filtering and draw order. Prepare
   independent synthetic inputs and the bounded official-release feasibility
   experiment. No alternative scan converter is justified by a measured xy
   mismatch yet.
2. Execute/repeat the experiments in GitHub Actions, inspect original outputs and
   measurements, close the capture feasibility question, and record the checkpoint
   and minimum follow-ups. Mechanical corrections/reverification do not introduce
   additional algorithm investigations. Exact runs and observations are recorded
   in the execution record below.

Local execution was limited to reading, editing, Git and lightweight hashes.
All compiler/native/render/test/benchmark commands belong to Actions. No GPL
algorithm implementation was copied or translated. The mathematical descriptions
below are observations, not implementation recipes accepted for Legacy.

Evidence vocabulary:

- **CoreV1 contract-established:** C5/C6 and ADR-002; normative for Core even when
  a stage is not yet implemented.
- **xy source-observed:** identified code in the locally fingerprinted tree;
  archive-to-upstream-commit equivalence remains unverified.
- **xy capture-verified:** requires an authenticated binary, environment, input,
  output and calibrated pipeline; none admitted here.
- **Provisional interpretation:** source observation applied to an observable
  script result without an accepted capture.
- **Blocked:** required numerical/output/environment evidence is missing.
  A successful host invocation or parser state cannot remove this block.

The direct xy regression suite remains **PLANNED, not operational**. Existing
pinned libass-tests and future authenticated xy captures are independent protected
suites; neither can replace or weaken the other. No comparison harness, accepted
fixture, reference, threshold, selector, classification or failure interpretation
is modified by R002.

## Source authority and identity

Sole main xy source: supplied
`C:/taiyou-time/refrences/xy-VSFilter-3.2.0.810`. README 1–27 identifies pinterf
pfmod 3.2.0.810 (20240123), branch xy_sub_filter_rc5, and both VSFilter/XySubFilter
and their AviSynth signatures. The folder has no Git metadata. No compressed
source-container file was supplied: its hash is unknown, not invented. The locked
upstream commit `25cf3e8433b3a83a5489e47a2c335516492bb815` is **not verified**
as equivalent. Neither a directory name nor the official release authenticates
that equivalence.

[Source manifest](../../validation/proposals/R002/sources.json) gives exact
SHA-256, line counts and paths for all eleven inspected files. These one-based
citations refer to those bytes, not online master or the different locked commit.

| File | SHA-256 |
|---|---|
| Rasterizer.cpp | `29b113f2c927793005afd25d8d0c861a430b55462f6e8e057e1bbaceccf08334` |
| RTS.cpp | `dd4ae1950606979b255a742279f4b64a4cd90ce8a0647135980fe08e02618ffb` |
| xy_widen_region.cpp | `af0034118448829065a6bdd6748bb8943aac4a62fbf852a59cd869d63486dd7c` |
| xy_filter.cpp | `f4ef0161a7f11cc93ea86987c4c632f0852342e4690bbb3bc6f2d3ee6e22808c` |
| draw_item.cpp | `3e8085ce65377fe8be28968ee7e87d038a9c552c672183c5ee2ffbc5c474f0d6` |
| csriapi.cpp | `9d2a5a38813457610322859072f99a2674259aacfc24888bee503b605334a7eb` |

Local reference checker passed four folders / 24 entry-point fingerprints; this
does not certify a full tree or a commit. No Guliverkli2, XySubFilter, FreeType or
libass rendering result is treated as xy-VSFilter evidence.

## Family decision matrix

“Core authorized” refers to the existing contract, not a new supported capability.
All five Legacy families remain evidence-pending; none is approved.

| Family | Core policy | Observed xy behavior | Evidence | Backend choice | Performance cost | Gate |
|---|---|---|---|---|---|---|
| Fill coverage | T012 FreeType 2.14.3 grayscale, Q6, nonzero | 8×8 span accumulation and optional post-raster phase shifting | Rasterizer Rasterize 789–881; RTS PaintFromOverlay 197–211 | Keep existing FT for Core; Legacy undecided | Actions samples; no xy/backend speedup claim | Core authorized; Legacy pending |
| Stroke widening | Isotropic device-space round FT baseline after transforms | Widen already transformed/scan-converted spans, x/y radii; special zero-axis paths | RTS PaintFromPathData 250–325; Rasterizer CreateWidenedRegion 3369–3388 | FT isotropic Core baseline only; no replacement selected | Public-API probe only; full widening cost unknown | Core isotropic authorized; Legacy/anisotropic pending |
| Overlap/grouping | Combined adjacent compatible contours; saturating E-F; global pass order per event | Style/karaoke-dependent word concatenation; expanded/body masks selected at drawing | RTS Append 181–195, Compact 1171–1206; draw_item Draw 191–274 | Explicit Core grouping retained; no Legacy emulation | Retained A8 bytes measured; total allocation unknown | Core authorized; Legacy pending |
| Blur/BE | C5 integer kernels, exact support, Gaussian then BE | Float Gaussian coefficients, shorter support; selected mask; integer BE truncation and fractional residual path | Rasterizer GaussianCoefficients 96–164, Blur 1044–1084, BeBlur 1167–1249; xy_filter 874–940, 1340–1387 | C5 Core kernels for T018; Legacy kernel deferred | Not executed; unknown | Core policy authorized; Legacy blocked |
| Shadow | Translate geometry before raster; E if border else F; blur then clip | Separate shadow position/alpha/selection; uses word overlay machinery | RTS PaintAll 1215–1290; draw_item Draw 40–97 | Core mask/offset policy; Legacy deferred | Not executed; unknown | Core authorized; Legacy blocked |

## Geometry, coverage and stroke findings

Actual Taiyou: `crates/taiyou-font/src/native.rs` RasterContext uses
FT 2.14.3 smooth/ftgrays with AA|DIRECT|CLIP=7. T012
`crates/taiyou-render/src/raster.rs` accepts Core Fill only, combines each explicit
group's contours, preserves negative origins and conservative control-hull bounds,
and checks dimensions/path/transient budgets. There is **no production stroker,
shadow or blur adapter** to compare yet. A planned contract or available native
FT_Stroker API does not imply implementation. Legacy group creation rejects.

Core uses owned, unhinted, non-LCD/non-bitmap outlines. Font-to-script y flip,
layout scale, rotation about origin and content mapping precede Q6 quantization.
Integer translation factors out with Euclidean residual phase 0..63; no separate
rounding of offset. T012 stores full source support, never viewport-clipped
coverage. Nonempty length=stride×height with zero padding; canonical empty mask
has zero dimensions/stride/bytes. Full group contour rasterization gives
nonzero winding, including opposite-winding holes, rather than source-over of
separately rasterized glyphs. These are established Core rules, not xy parity.

xy ScanConvert (Rasterizer 3254–3355) closes open contours, sorts directed edges
and emits intervals while winding is nonzero. Cubics are subdivided using an
acceleration-dependent step (EvaluateBezier 3112–3185). A quadratic FT input
therefore has no demonstrated identical native xy contour path: an ASS cubic
witness and a separately labelled FT quadratic experiment are provided.
Rasterize 789–881 masks x/y phase to 0..7, sums span lengths into 8×8 cells, and
stores coverage on a 0..64 scale before later alpha conversion. Its allocation
has an extra terminal pixel and 16-byte-aligned pitch. This differs structurally
from T012's A8 0..255 storage and full control-hull rectangle. It does **not**
prove a particular end-to-end pixel delta. RTS PaintFromOverlay 197–211 can
bilinearly shift a cached overlay according to subpixel-controller settings;
a universal “xy only supports eighth-pixel motion” conclusion would be wrong.
Capture must identify these settings and compare fractional phases.

RTS PaintFromPathData 250–325 transforms the path, aligns its bounds,
scan-converts, then widens. Border radii derive from style outline x/y widths
times target scale with positive +0.5 integer conversion; style widths have
already received SBAS script scaling in ParseSSATagString's caller path
(RTS 2955–2961). Text font scale/rotation are not applied to those width values
as outline geometry. Thus a post-transform device-axis widening interpretation
is source-supported, including rotation preserving device thickness; exact phase,
rounding and LayoutRes/non-square scaling stay capture/R004-gated.

CreateWidenedRegion clamps negative integer radii to zero. Positive y delegates
to elliptical span widening (xy_widen_region xy_overlap_region 158–243,
XyEllipse::init 572–596); zero x has a degenerate vertical arc
(gen_left_arc 452–473). Positive x / zero y uses a dedicated repeated overlap
path. Both zero skips widening in the RTS path. The evidence supports neither a
square maximum filter nor the claim that an FT round stroke has identical
acute/self-intersection pixels. Open-path cap equivalence is also blocked:
ordinary ASS drawing contours are closed by this scan path.

The independently authored FT probe covers rectangles and acute polygons at
Q6 radii 0,16,64,1024 (0,0.25,1,16 pixels), using public round-cap/round-join
stroker APIs and native y-up contours. Its outside-border outline is rasterized
as an experimental expanded silhouette. It is not a production E-F implementation
and has no authenticated xy mask reference. Anisotropic/zero-axis widths,
self-crossing strokes, glyph overlap and transformed/negative contours require
separate paired captures before Legacy backend selection. No alternative
scanline/widening algorithm is implemented: source differences alone do not
supply accepted target pixels or justify translating the GPL implementation.

## Grouping, subtraction and pass order

Core C5 combines a maximal adjacent compatible geometry/effect/paint run's
contours once: fill F, expanded silhouette E, outline max(E-F,0). Shadow source
is E when a border exists, otherwise F. Transparent paint never punches a
previous outline incorrectly. C6 source-over uses the separate specified
mul255 roundings, including alpha. All shadow groups precede all border groups,
then all fill groups within each event's scene order; do not interleave per-glyph
shadow/border/fill triplets. Different events retain layer/source order.

xy's boundary is a word/path, not proven identical to Core's group key:
CWord::Append 181–195 checks equal style, hard-break and karaoke conditions;
CText::Append 640–644 requires the same text type; CLine::Compact 1171–1206
joins eligible neighbors. CText::CreatePath 646 onward obtains a GDI path for
that word. Polygon concatenation is not proof of text grouping.
CompositeDrawItem::Draw (draw_item 191–274) schedules shadow, outline, body
loops over draw items; spatial grouping/additive composition complicates
overlap. Preserve this source observation without asserting universal event/global
alpha semantics.

Rasterizer FillAlphaMashBorder_c 2413–2428 uses saturating expanded-minus-body
before paint alpha multiplication on coverage units of 64, with +32 then >>6.
Overlay::FillAlphaMash 2770–2801 selects body, full expanded, or subtracted
outline according to fBody/fBorder. PaintAll 1255–1306 chooses flags partly from
body opacity/karaoke; an opaque-body fast path can use full expanded coverage,
where translucent paint needs the hole. Blur can change expanded/body coverage
before this selection/subtraction. Therefore per-glyph independently composited
effects are not an admissible substitute for explicit grouping.

The A8 experiment computes a **Core algebra witness** from actual T012 overlapping
rectangle masks: opaque alpha255 and translucent alpha128, combined versus two
separate source-over operations. Its difference is not a failure against xy.
These masks share output bounds; stored origin/size differences are reported
separately from per-pixel deltas. Text boundary, style change, karaoke and multiple
event fixtures remain follow-up work, not approved by a polygon-only capture.

## Blur, BE, shadow and allocation

Core C5 is complete enough for T018's later implementation: BE is n separable
[1,2,1]/4 passes, (sum+2)/4 after **each axis**, zero outside, exact n-pixel halo
per axis. Gaussian sigma is positive blur in device pixels (SBAS mapping as C5);
radius ceil(3×sigma), normalized symmetric integer coefficients sum65536,
nearest rounding and center correction, i64 accumulation, (sum+32768)>>16 at
each axis. Sigma0 bypasses. Gaussian precedes BE, selected pass masks are
filtered separately, then clipped. Expand by the sum of exact supports, preserve
origins, reject oversized sigma/radius/pass/allocation limits. Do not truncate
support or mutate cached masks. No production filter is added here.

xy's actual dispatched path is Rasterizer::Blur 1044–1084 via
RTS PaintFromNoneBluredOverlay 213–235, **not** OldFixedPointBlur 897–1041.
IsItReallyBlur 886–894 gates BE>0 or Gaussian>0.333333; when BE is active a small
positive Gaussian still reaches GaussianBlur. GaussianCoefficients::init 96–164
uses sigma=style blur×target-axis-scale, normalized float samples of
exp(-i²/(2 sigma²)), width=(ceil(3 sigma)|1), radius=width/2. Thus radius is
not Core's ceil(3 sigma). This is a traced mapping, not a guessed sigma.
It remains provisional for the official binary until source/build relationship
and pipeline are established.

GaussianBlur 1086–1165 expands by filter radius in eighth-pixel offset units,
allocates aligned pitch and filters border when widened, body otherwise,
preserving the unfiltered body when a border exists. xy_filter::xy_gaussian_blur
874–940 uses float intermediate horizontal/vertical buffers and zero-padding;
float-to-byte conversion at 780–869 includes SIMD conversion and scalar +0.5
paths. CPU/MXCSR/tail differences are not reduced to an invented universal integer
rounding rule here. Legacy coefficients/support are source-observed, exact
end-to-end rounding is **blocked** until targeted impulse/tail capture.

BE BeBlur 1167–1249 pads each axis by
round(target_scale_axis×sqrt(be_strength/4)) pixels (positive +0.5 truncation),
not one pixel per pass. It derives pass strength from the average target scale,
runs its integer part through be_blur, and sends the residual to xy_be_blur.
The integer 3×3 [1,2,1] outer-product /16 path truncates at >>4
(Rasterizer 568–671), unlike Core's per-axis rounding. The residual path
(xy_filter 1248–1387) calculates a three-tap filter and dispatches its named
ROUND_HALF_TO_EVEN template. Its exact tie/tail/platform behavior needs capture;
the template name alone is not proof. Padding is zero initialized; behavior when
support reaches allocation edges needs impulse/crop witnesses, not inference.

PaintAll 1215–1290 computes shadow offset before overlay acquisition, with
target-axis scale and +0.5 integer conversion, and selects body/expanded coverage
using outline/opacity/karaoke flags. Shadow alpha uses a different source rounding
path from body/outline. DrawItem::Draw 40–97 applies the clip with overlay alpha
generation at drawing time, after filtering. Fractional negative offsets,
borderless/translucent shadow origin, blur+clip interactions and offscreen bounds
are blocked for Legacy. Core follows its explicit C5 translate-before-raster
policy. No legacy video YUV matrix/range behavior is inferred from RGB output.

Cache keys must retain Core raster policy1/backend2.14.3/settings, complete
transformed geometry/phase, explicit ordered group membership, fill rule,
post-transform x/y border radii and stroke policy, full source masks, Gaussian
coefficient/support identity, BE count/order and fractional shadow phase.
See existing CACHING K2. An unaccepted Legacy proposal must never share a
Core mask/effect policy identity or silently become policy1. R002 creates no
runtime registry or cache policy revision.

## Minimal proposed witnesses and gates

All scripts are independently authored, 64×64 PlayRes/output, sampled at 0.5s.
Their exact byte hashes are in [fixtures.json](../../validation/proposals/R002/fixtures.json);
[rules.json](../../validation/proposals/R002/rules.json) records evidence,
line ranges, expected observation, environment and approval=false for each.
These are proposal inputs, **not accepted regression fixtures or expected pixels**.
Source references below resolve through the hashed manifest. Each decision is
provisional/evidence-pending; exact pixel expectation is null.

| Proposal ID / script stem | Distinguishing observation | Source function and one-based lines | Gate |
|---|---|---|---|
| R002-fill-straight | white 8×8 rectangle; xy 64-unit accumulation versus FT A8 | Rasterize, Rasterizer 789–881 | pending |
| R002-fill-phase | x=8+1/64; base raster phase versus configured bilinear shift | PaintFromOverlay, RTS 197–211 | pending |
| R002-fill-cubic | cubic cap; FT and xy edge approximation | EvaluateBezier, Rasterizer 3112–3185 | pending |
| R002-fill-winding | opposite inner contour creates hole | ScanConvert, Rasterizer 3254–3355 | pending |
| R002-stroke-thin | border0.25; integer/phase quantization | PaintFromPathData, RTS 250–325 | pending |
| R002-stroke-thick-acute | border16 around acute triangle | xy_overlap_region, xy_widen_region 158–243 | pending |
| R002-stroke-anisotropic | x4/y1; ellipse versus isotropic stroke | XyEllipse::init, xy_widen_region 572–596 | pending |
| R002-stroke-zero-y | x4/y0 dedicated horizontal path | CreateWidenedRegion, Rasterizer 3369–3388 | pending |
| R002-stroke-zero-x | x0/y4 degenerate vertical arc | gen_left_arc, xy_widen_region 452–473 | pending |
| R002-stroke-rotated | z30, scale200/50, border2; transform before widen | PaintFromPathData, RTS 250–325 | pending |
| R002-group-opaque | two overlapping contours at alpha255 | Append, RTS 181–195; Compact 1171–1206 | pending text boundaries |
| R002-group-translucent | same at alpha128; grouped versus separate alpha | CompositeDrawItem::Draw, draw_item 191–274 | pending |
| R002-outline-hole | invisible fill / translucent border; E-F versus E | FillAlphaMashBorder_c, Rasterizer 2413–2428 | pending |
| R002-be-one | borderless BE1; truncation versus per-axis rounding | be_blur, Rasterizer 568–671 | pending |
| R002-be-repeated | border2 BE3; selected plane and padding | BeBlur, Rasterizer 1167–1249 | pending |
| R002-gaussian | border2 blur1; odd width versus Core radius | GaussianCoefficients::init, Rasterizer 96–164 | pending |
| R002-gaussian-be | blur1+BE1 ordering/subtraction | Blur, Rasterizer 1044–1084 | pending |
| R002-shadow-fractional | x0.5/y-0.25 shadow2-border origin | PaintAll, RTS 1215–1290 | pending |
| R002-shadow-clip | shadow2+blur1+rect clip | DrawItem::Draw, draw_item 40–97 | pending |

Additional exact inputs to establish remaining gates: rotated self-crossing stroke,
two truly separate text words with equal/different paint at overlap, two same-layer
events reversed in source order, Gaussian impulse sigma0/0.333333/0.5/1/2 at widths
1/3/4/15/16/17 and near allocation edges, BE impulses n1/2/3 and fractional target
scales, borderless versus bordered shadow with fractional negative offsets,
hard crop through a blurred edge, and matching isolated/combined contour masks.
These require explicit input hashes and original paired outputs before promotion.
No invented output is attached to them.

## Experiment design, dependencies and capture boundary

[Experiment instructions](../../experiments/coverage/README.md) describe the two
independent process repeats, raw A8/PGM outputs, numerical grouping difference,
bounds, allocation limits and timing scope. Actual Taiyou APIs generate 15 fill
cases: straight, fractional phases (1/4,1/64,7/64), negative/offscreen, acute,
quadratic/cubic, self-cross, winding hole, rotated/scaled, combined/separate overlap.
The native FT public-API stroke probe generates eight more masks.
No FT-versus-xy max delta/MAE is available without authenticated, aligned xy
coverage. Null is not zero. Repeat equality has zero tolerance. Raw timings
are hosted-runner smoke measurements, not speedups or R003 controlled benchmarks.
Retained A8 size is measured; complete native allocations/RSS are unknown.

The Windows feasibility host downloads the user-specified official 3.2.0.810
archive, inventories every file/hash/PE architecture, chooses only the x64
**VSFilter.dll**, and probes its public CSRI ABI in isolated time-bounded Python
children. XySubFilter is separately inventoried and never executed as xy.
Local csriapi request_fmt 108–130 advertises formats that render 132–158 does
not actually implement: **BGR24/YUV must not be used**, since that path traps.
The host requests 0x102 RGB32 only and preserves all four bytes over black/white
and fourth-byte0/255 initial buffers, twice per input. No desktop screenshots,
lossy conversion, YUV or inferred A8 alpha extraction.

The public CSRI header's ABI declarations are BSD-style (csri.h 1–29);
the ctypes declarations were independently written from interface facts, not
GPL renderer implementation. Existing Python standard library supplies the host,
so no new production or Python package dependency. External process/reference
dependency proposal: official pinterf3.2.0.810 GPL binary, research only; Microsoft
Windows2022 x64 runner/GDI; preinstalled 7-Zip extraction (version recorded in
listing); existing pinned Source Sans3 3.052R/OFL1.1 file from the accepted source
lock. Alternative AviSynth+MaskSub would add a verified host dependency and is
deferred if CSRI calibration fails. No DLL/source is redistributed in Taiyou.

Binary/hash discovery is not human acceptance or a production pin. Before reuse
as a trusted oracle, review the discovered archive/DLL hashes and supported ABI,
approve the binary pin, calibrate orientation/channel/fourth-byte semantics and
record settings. Private font registration does not establish GDI's actual face
or fallback: polygon-only probes do not certify any glyph/font behavior.
Original raw RGB32, hashes and environment are retained as **unaccepted probe
evidence**, even if rendering succeeds. T015 capture_import remains explicitly
unavailable for authenticated capture admission; this host does not bypass it
or change its failure status.

## Execution record

Actions results and limitations are appended after the bounded sessions.
No unrun test or measurement is claimed as passed. The original full external
comparison remains independent, including its nonzero exit for missing fonts.

## Dependent implementation and remaining work

T016 may implement **existing Core isotropic** device-space round FT expansion,
explicit grouping, max(E-F,0), shadow source E/F, pre-raster fractional offsets,
negative support, budget checks and event pass order as C5/C6 specify. It must
add real Core stage fixtures, run existing unit/stage and full external checks,
and not advertise Legacy support. Anisotropic/zero-axis/self-intersecting strokes
remain gated; no square max filter. Collision-box border policy remains R001/R017
work, not a raster choice.

T018 may later implement the existing Core coefficients, two per-axis roundings,
zero-extension, support growth, Gaussian-before-BE, immutable mask and post-blur
clip policy. It still needs impulse/edge/limit/order fixtures and full comparison.
No Legacy blur sigma/support/rounding is approved by R002.

Minimum follow-up: review/pin the official binary discovery; establish a calibrated
CSRI or verified AviSynth MaskSub host with exact font-file/selected-face mapping
and subpixel/CPU environment; capture paired fill/stroke/overlap/impulse/shadow
witnesses and run exact bounds-aligned differences twice. Only then assess FT
sufficiency or propose an independently specified alternative with measurements.
Conflicts or absent evidence keep the corresponding family unsupported.
Stop at the unmerged PR for technical review and exact-SHA human-owner baseline
acknowledgment under ADR-011; agents must never post that acknowledgment.
