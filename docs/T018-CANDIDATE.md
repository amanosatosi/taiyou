# T018 — ASS scalar blur candidate

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
negative Gaussian clamp, source ordering and style resets. BE retains its signed
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
