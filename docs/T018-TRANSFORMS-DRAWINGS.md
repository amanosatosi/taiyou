# T018 ordered transforms and drawing words

Implementation checkpoint, not LegacyXyV1 admission or authenticated pixel proof.
The owner's continuation instruction explicitly includes these features in PR32.
The earlier `508e676` compatibility fixes are preserved in its existing branch.

The local pinterf pfmod 3.2.0.810 RTS.cpp SHA-256 is
`dd4ae1950606979b255a742279f4b64a4cd90ce8a0647135980fe08e02618ffb`.
Its exact equivalence to locked upstream commit
`25cf3e8433b3a83a5489e47a2c335516492bb815` remains unverified.
Reference sources stay read-only; no GPL source is vendored or linked.

Source-observed transform paths: ParseSSATag 2050–2211 parameter collection,
embedded commands and suffix handling; 2234–2319 colors/alpha/blur/BE/border;
2414–2515 font selection/size/scale/spacing/italic; 2587–2683 positioning,
polygon controls, q/reset/shadow and the four t parameter forms;
CalcAnimation 2856–2871; local millisecond clock 3157–3158.

Supported forms are t(tags), t(accel,tags), t(t1,t2,tags),
t(t1,t2,accel,tags). Empty comma fields are skipped. Three-parameter times
use real-prefix conversion then truncation; four-parameter times use decimal
integer prefixes. Zero end means duration. Finite zero/negative acceleration
is retained; a nonfinite evaluated result fails. The first closing parenthesis
terminates the parameter substring, matching the historical reader even for
partially nested commands. Excess parameter counts execute no children.

| Existing operation | Inside transform |
|---|---|
| Gaussian blur, BE | Interpolate current to raw target; Gaussian clamps afterward |
| Size | Interpolate, including relative tenth-size target; nonpositive result restores original |
| X/Y scale, border, shadow | Interpolate then nonnegative clamp |
| Spacing | Interpolate signed value |
| RGB | Interpolate native shifted channels, truncate and mask |
| Transparency | Interpolate transparency, truncate to byte, then invert opacity |
| Font family, bold, italic | Immediate assignment, no invented numeric interpolation |
| Alignment, position, reset, q | Immediate existing ASS control operation |
| p, pbo | Immediate drawing state, not interpolated |
| Taiyou warpmode | Immediate event selector; Unicode engine remains gated |
| Unsupported embedded tags | Existing strict error / tolerant diagnostic; no invented animation |

Flat transform header/child/tail operations preserve original nonoverlapping
byte spans. Publication validates nesting. Evaluation uses the current state at
each operation, not a frame-history animation layer. Changes are marked temporal.

Drawing source paths: RTS.cpp CPolygon::Get6BitFixedPoint / ParseStr 765–916,
CreatePath 919–938, ParsePolygon 2024–2047, pbo/p 2587–2611;
Rasterizer.cpp ScanConvert 3254–3335 and its uniform B-spline evaluation.
Drawing words retain their literal decoded source span, an owned font-independent
outline and local metrics. No fake font face or duplicated effects renderer is
introduced. m starts closed figures; n is the historical non-closing move;
l adds edges; b consumes cubic triples; s/p/c form/extend/close uniform cubic
B-splines. Coordinates truncate to Q6 before drawing-scale conversion.
The divisor is 2^(p-1); p0 resumes text. pbo changes baseline metrics.
Native shift/coordinate/count/allocation overflows return typed limits.

The local libass compare/README.md and compare.c 174–220, 327–362 specify the
8x1 host: render at scaled dimensions and average premultiplied bytes into
RGBA16 with its fixed reciprocal. The pinned libass-tests run-single.sh passes
the unchanged scale file directly. The adapter uses that rule, never a resized
reference or a tolerance. The owner explicitly approved a 512-radius caller
budget for the oversampled frame; ordinary callers retain 256. All other
allocation/dimension limits remain active. Original oversized raw output is
preserved; normalized PNGs are display previews, not comparison authority.

Focused Actions and exact-case results are recorded after execution. Until then,
implemented paths are verification-pending. Static BE's remaining GDI-outline
and 0–64 span-raster differences remain independent of these features.
