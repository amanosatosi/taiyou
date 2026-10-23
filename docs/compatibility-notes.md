# Compatibility findings

Descriptive working notes, updated when evidence changes; no behavioral approval
procedure is implied. The primary target is pinterf/xy-VSFilter. Local source
identities are recorded in [the reference map](LOCAL_REFERENCES.md). Source
inspection establishes semantics, not pixel equivalence. The frozen libass-tests
PNGs are **libass references**, not authenticated xy captures; no authenticated
xy output is available for the findings below. Internal passing tests alone do
not establish historical compatibility.

Measurements: [font/contour/italic run](https://github.com/amanosatosi/taiyou/actions/runs/39946036720)
and [production collision run](https://github.com/amanosatosi/taiyou/actions/runs/39948311194).
Strict mismatches remain failures; the Windows GDI observation step succeeded
but its separate whole-corpus symlink verifier failed on Windows path encoding.

## Font selection, legacy cmaps, metrics and shaping

**Partial — legacy DBCS fonts.** Pinterf uses GDI Unicode TextOut on the selected
face. The corpus Big5 face has a Microsoft Big5 format-2 cmap and no real Unicode
cmap; the related Shift-JIS face also lacks Unicode. Taiyou previously passed
Unicode directly to HarfBuzz OT mapping, producing missing glyphs. It now converts
Unicode through the cmap's Microsoft DBCS encoding and resolves actual glyph IDs,
inheriting OT shaping/metrics and preserving Unicode byte clusters. A FreeType
Unicode cmap synthesized from CFF glyph names is not an OT cmap. Real Unicode
tables remain preferred. Authored Big5/Shift-JIS/mixed-map fixtures verify glyph
identity, advances, outlines, unmappable input and budget failure; no per-glyph
Chinese mapping is built into production. This does not implement ASS input
decoding or `\fe` semantics.

Strict libass differing pixels measured at development commit `2165d91`:

| Frozen fixture | Before | After |
|---|---:|---:|
| `font_nonunicode/big5-hkscs-1000` | 99,146 | 20,963 |
| `font_nonunicode/shiftjis-1000` | 2,446 | 2,263 |
| `faux-italics/cff-0000` | 1,746 | 400 |
| `faux-italics/truetype-0000` | 2,979 | 450 |

Big5 changed from zero visible pixels to 87,815 (reference 99,146), with missing
glyph diagnostics removed. Arabic/symbol output was unchanged by the cmap bridge;
some later contour corrections change those masks. These remain mismatches.

**Partial — synthetic italics.** Pinterf's LOGFONT italic request reaches GDI
TextOut/GetPath. The focused Windows GDI probe authenticates the two frozen font
bytes and verifies actual selected family. At positive cell height6400, CFF slant
is approximately0.1763 and TrueType approximately0.33984, with unchanged advance.
Taiyou applies format-specific shear before vertical orientation/ASS transforms,
updates ink support and avoids double-slanting actual italic faces. Catalog,
shaping and rendered/seek regressions pass. Remaining edge coverage differs from
libass; this probe is GDI evidence, not an authenticated xy rendering capture.

## Glyph outlines and rasterization

**Verified defect; partial pixel compatibility — curve continuity.** Pinterf's
Rasterizer `_EvaluateLine` reconnects successive path points. Taiyou's converted
quadratic polynomial could truncate its final endpoint one Q6 tick short, while
the next segment began at the exact point. This left nonzero scanline winding
and horizontal connectors between glyphs. The final segment now uses the supplied
endpoint. A closed-contour winding regression passes and fails with the old code
restored as a negative control. Big5, Arabic traditional, symbols and trimming
show improvements; not every individual alpha metric improves. Production still
uses the existing xy span rasterizer, without an A8 compensation factor.

## Drawing geometry and coverage

**Partial / libass mismatch.** Pinterf CPolygon::ParseStr derives polygon advance
from bounds; CText's `fontSpacing` is absent from that path. Taiyou preserves this
distinction. A four-square reduced native test at integer/fractional positions
produces identical output for spacing0, -0.6 and1.25. The original784-square
`draw-images/drawImg-104396` remains1,048 differing pixels (1,044 alpha), unchanged
by the contour fix. Fractional edge coverage/compositing equivalence remains
unresolved; no alternating-column artifact was deliberately introduced.

## Layout, positioning and collision

**Partial — transparent borders.** Pinterf RTS.cpp GetNextLine1453–1502 collects
line borders without checking alpha; MakeLines1536–1565 retains first/top and
last/bottom borders; AllocRect1606–1651 inflates reservation before allocation.
Inspected RTS.cpp SHA-256:
`dd4ae1950606979b255a742279f4b64a4cd90ce8a0647135980fe08e02618ffb`.
Taiyou now supplements its geometric footprint with that logical vertical border
reservation, using nearest eighth-pixel border widths and existing SBAS mapping.
Compacted whitespace retains borders; genuinely empty hard lines reset them.
Mixed-line/hidden-border regressions and the independent geometric oracle cover
this change. The old invisible-border exclusion in ADR-012 is retired. Existing
horizontal support, ordering, explicit-position/origin exclusions and stateless
solver remain; complete pinterf allocator parity is not claimed. Strict
`implicit-pos/bordercollision-2000` differs in500 pixels instead of950; channel
MAE drops326.96→78.73 at `228f759`. It remains a mismatch. The sampled `shift-dir`
frame is unchanged.

## Scaling, borders and shadows

**Partial.** Existing implementation separates PlayRes/layout geometry mapping
from layout-to-target effects scaling and scales border axes under SBAS. Local
pinterf RTS.cpp2956–2961 separates font/spacing and outline/shadow scaling. The
focused `scaling_playres/playresx_sbas-yes-0000` and
`scaled-border-default/ffmpeg-8000` remain mismatches. This session changes
collision reservation, not device mapping or border kernels. The broader scaling
differences are unresolved.

## Alpha and compositing

**Source-verified semantics; pixel mismatch.** Pinterf RTS.cpp2260–2274 animates
the raw hexadecimal alpha target and narrows to BYTE afterward. Taiyou retains
511 from `\1a1FF`, interpolates, then narrows. Existing native tests verify
opacities47,250,5 at2040,2560,4960ms. The three frozen `vsf-quirks-var/alpha-clipping`
frames remain691,673,572 differing pixels. Their raw output is unchanged by this
session's contour fix; no parser or opacity compensation was introduced.

## Blur and effects

**Partial / mismatch.** Prior source-backed Gaussian/BE behavior is described in
[the T018 evidence](T018-CANDIDATE.md). This session preserves those kernels and
native0..64 coverage units. Focused BE, Gaussian-transform and anamorphic frames
still differ in alpha. Source geometry/coverage remains separate from filtering;
the current evidence does not justify tuning blur constants to libass PNGs.

## Karaoke

**Partial / mismatch.** Pinterf RTS.cpp2512 onward resets both clocks for `\kt`
without changing the effect kind; `\kf` advances from the current end. Existing
clock/run-boundary/seek tests exercise that behavior. Focused `v4++/kt-01500` and
`karaoke/karaoke-and-runsplits-2040` remain mismatches. Timing, segmentation and
shared font/raster differences are not yet independently isolated for the full
series; timestamps are not treated as separate bugs.

## Parsing and other ASS compatibility details

**Mismatch with correct style selection.** `token-parsing/trimming-0000` selects
the normal Example style and renders Green; the unusual `***Example` style is
irrelevant to this event. The contour fix reduces differing pixels1,225→1,163;
remaining metrics/coverage differences are unresolved. No token-parser rewrite
was made. Legacy YCbCr emulation, SSA V4 styles and deferred event effects/wrapping
remain unsupported and separate from visual matches.
