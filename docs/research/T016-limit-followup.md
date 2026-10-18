# T016 PR28 follow-up — exported stroke support and invisible shadow work

Status: implementation correction proposed for review; no Legacy promotion.
Integration base: `ae4cf96a554b1c719861e1f8db7f0b85a07aeeaf`.
Reviewed original candidate: `6a39ad64cd9e8d115231bac824893fde57f4a1b7`.
Final candidate and fresh Actions links are recorded on PR28.

## Exact reproduction and cause

[Diagnostic Actions job](https://github.com/amanosatosi/taiyou/actions/runs/39176405801/job/117819882211)
executed the unchanged T015 comparator with the five original frozen cases,
using candidate `b645832baaae87940e65bc4f9ecdb748006ef961`. That candidate added
bounded numerical error evidence only, without changing stroke behavior.
All five reproduced exit32/LimitExceeded at **export-point-bounds**.
The original red result and output artifacts were retained.

[Initial evidence](../../experiments/t016/initial-evidence.json) preserves source
SHA-256 identities and complete bounded error details. Each row below is an
observed failure, not a new expected result, classification or acceptance rule.
Q6 values are integer ticks, 64 ticks per device pixel. Export hulls use the
predicted output mask's local x-right/y-up coordinate system.

| Frozen case | Input ops / points / contours | Export points / contours | Radius Q6 | Predicted device bounds | Export hull Q6 | Failing extent |
|---|---|---|---|---|---|---|
| regression/font_nonunicode/symbol-first-char-0xF000-1000.png | 27 / 40 / 3 | 89 / 3 | 128 | [411,214,431,254] | [7,33,1286,2536] | x max1286 > width20×64=1280 |
| regression/implicit-pos/bordercollision-2000.png | 15 / 13 / 2 | 40 / 2 | 1920 | [153,224,231,303] | [-36,42,5028,5002] | x min-36; x max5028 > width78×64=4992 |
| regression/karaoke/357-k-and-kf-desynced-6798.png | 395 / 594 / 24 | 1518 / 24 | 384 | [431,71,1491,172] | [18,18,67842,6454] | x max67842 > width1060×64=67840 |
| regression/karaoke/357-k-and-kf-desynced-7170.png | 395 / 594 / 24 | 1518 / 24 | 384 | [431,71,1491,172] | [18,18,67842,6454] | same two-tick overshoot |
| regression/karaoke/357-k-and-kf-desynced-8170.png | 395 / 594 / 24 | 1518 / 24 | 384 | [431,71,1491,172] | [18,18,67842,6454] | same two-tick overshoot |

| Case family | Remaining frame segments | Available transient bytes | Base reservation | Export-inclusive reservation |
|---|---|---|---|---|
| symbol | 999685 | 117723556 | 2117628 | 2129661 |
| bordercollision | 999985 | 118948659 | 2131300 | 2136712 |
| each karaoke timestamp | 999605 | 110669258 | 2957696 | 3162770 |

None reaches the configured point, frame-segment, pixel, dimension or transient
reservation cap. The incorrectly predicted control box was treated as a resource
limit. The five errors are renderer defects; their previous infrastructure label
does not excuse them. The comparator/error mapping is unchanged.

## Correction and resource accounting

The source contour hull expanded by radius is an initial support prediction.
FreeType round joins can export Bezier controls outside that prediction.
The adapter now unions the prediction with the complete exported control hull,
in device coordinates with the correct y flip, and rebases the native outline
against that resulting mask origin. It does not clip or discard those controls.

Before allocating/rasterizing the mask it rechecks the resulting dimensions,
pixel/output caps and transient reservation. Additional reservation is exactly
three times the additional A8 byte count plus 64 times the additional width+height,
matching the existing adapter's conservative coverage/scratch estimate.
The exported point arrays and original native scratch reservation remain charged.
No configured limit, native ABI count cap or error classification is raised.
Input vectors are now reserved after their base reservation check, avoiding
allocation before budget admission.

The numerical diagnostic remains bounded by C7's detail cap and records check,
input/export counts, radius, bounds/hull, remaining segments, available transient
and allocation estimates. It includes no contour dump or font bytes. Normal
rendering emits no per-glyph diagnostic log. Genuine extent, segment and memory
failures still return LimitExceeded.

## Invisible passes, zero offsets and SBAS

A shadow with final evaluated paint opacity0 provably contributes no pixels.
The renderer skips offset quantization, stroking, rasterization, mask allocation
and shadow DrawOp creation for that pass. A visible zero-offset shadow remains:
it shares E when bordered, otherwise F, and composites before border/fill.
An invisible shadow no longer consumes a second stroke's segment budget.

The previous script_offset_q6 unconditionally multiplied by content/PlayRes.
Core's ScaledBorderAndShadow=false keeps effect widths/offsets in device pixels;
true scales x/y shadow offsets independently by their content/PlayRes ratios.
Border expansion remains post-transform and isotropic. Unequal device-axis ratios
with a scaled isotropic border remain unsupported; a borderless shadow can scale
independently. No normative contract or Legacy policy is changed.

## Prior test audit and additive coverage

The original T016 change made renderer_slice's style shadow transparent and
restored shadow transparency after combined alpha tags. Those scripts intended
to isolate fill coverage. A default opaque zero-offset shadow becomes observable
once T016 implements the C5 pass, especially on partially covered/transparent fill.
Explicit transparency is therefore necessary to keep a fill-only test isolated,
but cannot replace coverage of the original inputs.

The follow-up does not edit renderer_slice.rs or any existing assertion. New
renderer_effect_regressions tests preserve the pre-T016 style format without
BackColour, exercise all nine alignments and layer/source order with visible
default zero-offset shadows, and compare exact C6 shadow-then-fill composition.
They also retain exact fill-only A8 equality against the existing independent
C/FreeType H reference on Linux. Other platforms retain coverage/composition
checks using the isolated fill path; no reference is regenerated.

Additional tests cover visible zero-offset shadow under transparent fill,
invisible fractional/huge offsets under the same tight frame segment budget,
visible nonzero shadows, SBAS false/true at 2× scaling, fractional offsets and
independent x2/y3 scaling. New stroke tests cover acute round-join control growth,
repeat byte equality, retained negative support and genuine post-export dimension,
segment and transient limits.

The additive exact-five diagnosis is supplemental. Existing full comparison,
focused smoke, pinned libass oracle, all104 inventory entries, all187 frozen
assets and the four required checks remain unchanged. New CI infrastructure needs
the exact-head human-owner acknowledgment under ADR-011 after final review; the
agent must never post it or merge the PR.

## Verification record

All compilation, native execution and tests run in GitHub Actions only.
The first behavior-preserving diagnostic reproduced all five failures above.
The correction is verified with new focused regressions, the same five cases,
the full original comparison and all three platform jobs. Exact final results,
remaining limitations and artifact links are recorded on PR28 after completion.
No unrun result is claimed as passed by this document.
