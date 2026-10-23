# Compatibility findings

Descriptive working notes, updated when evidence changes; no behavioral approval
procedure is implied. Investigations use both libass and pinterf/xy-VSFilter;
shared behavior is preferred, with historical xy behavior taking default priority
when established ASS behavior conflicts. Libass is equally valuable for bug
discovery and modern Unicode/shaping behavior. Local source
identities are recorded in [the reference map](LOCAL_REFERENCES.md). Source
inspection establishes semantics, not pixel equivalence. The frozen libass-tests
PNGs are **libass references**, not authenticated xy captures; no authenticated
xy output is available for the findings below. Internal passing tests alone do
not establish historical compatibility.

**Unresolved — pair kerning follow-up.** Hash-authenticated Windows GDI probes
of the frozen DejaVu Sans and Arimo faces report additive A/V and T/o widths at
positive cell height 6400. Pinterf's ASS TextOut/GetTextExtent path differs from
its separate libssf kerning path. Libass parses the script's Kerning header and
configures its shaper accordingly (`ass.c`, `ass_shaper.c`). Default/header
semantics and end-to-end output still need comparison. A local shaping experiment
is isolated outside PR #34 and has not passed CI; production shaping is unchanged
by this observation.

Measurements: [full 101-frame run at 6f667c6](https://github.com/amanosatosi/taiyou/actions/runs/39949464745)
and [focused collision run](https://github.com/amanosatosi/taiyou/actions/runs/39948311194).
Strict mismatches remain failures; the Windows GDI observation step succeeded
but its separate whole-corpus symlink verifier failed on Windows path encoding.
Full results are 1 exact, 83 mismatches, 17 unsupported, 0 crashes/infrastructure
errors. Compared with exporter candidate 7a40a72, 48 frames have fewer differing
pixels and none have more. Arabic simplified's alpha MAE rises slightly from
27.46738 to 27.46988 despite unchanged 448 differing pixels.

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

Strict libass differing pixels at production commit `6f667c6`:

| Frozen fixture | Before | After |
|---|---:|---:|
| `font_nonunicode/big5-hkscs-1000` | 99,146 | 20,963 |
| `font_nonunicode/shiftjis-1000` | 2,446 | 2,263 |
| `faux-italics/cff-0000` | 1,746 | 400 |
| `faux-italics/truetype-0000` | 2,979 | 450 |

Big5 changed from zero visible pixels to 87,815 (reference 99,146), with missing
glyph diagnostics removed. Arabic/symbol output was unchanged by the cmap bridge;
contour/collision corrections subsequently improve symbol cases 1,668→829 and
1,012→714. These remain mismatches.

**Partial — synthetic italics.** Pinterf's LOGFONT italic request reaches GDI
TextOut/GetPath. The focused Windows GDI probe authenticates the two frozen font
bytes and verifies actual selected family. At positive cell height 6400, CFF slant
is approximately 0.1763 and TrueType approximately 0.33984, with unchanged advance.
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
produces identical output for spacing 0, -0.6 and 1.25. The original 784-square
`draw-images/drawImg-104396` remains 1,048 differing pixels (1,044 alpha), unchanged
by the contour fix. Fractional edge coverage/compositing equivalence remains
unresolved; no alternating-column artifact was deliberately introduced.

## Layout, positioning and collision

**Partial — transparent borders.** Pinterf RTS.cpp GetNextLine (1453–1502) collects
line borders without checking alpha; MakeLines (1536–1565) retains first/top and
last/bottom borders; AllocRect (1606–1651) inflates reservation before allocation.
Inspected RTS.cpp SHA-256:
`dd4ae1950606979b255a742279f4b64a4cd90ce8a0647135980fe08e02618ffb`.
Taiyou now supplements its geometric footprint with that logical vertical border
reservation, using nearest eighth-pixel border widths and existing SBAS mapping.
Whitespace-only lines retain borders; genuinely empty hard lines reset them.
On nonempty lines, Compact recomputes borders from retained words, excluding
trimmed edge spaces and the terminating break's style (RTS.cpp:1171–1220).
Mixed-line/hidden-border regressions and the independent geometric oracle cover
this change. The old invisible-border exclusion in ADR-012 is retired. Existing
horizontal support, ordering, explicit-position/origin exclusions and stateless
solver remain; complete pinterf allocator parity is not claimed. Strict
`implicit-pos/bordercollision-2000` differs in 500 pixels instead of 950; channel
MAE drops 326.96→78.73 at `228f759`. It remains a mismatch. The sampled `shift-dir`
frame is unchanged.

## Scaling, borders and shadows

**Partial.** Existing implementation separates PlayRes/layout geometry mapping
from layout-to-target effects scaling and scales border axes under SBAS. Local
pinterf RTS.cpp:2956–2961 separates font/spacing and outline/shadow scaling. The
focused `scaling_playres/playresx_sbas-yes-0000` and
`scaled-border-default/ffmpeg-8000` remain mismatches. This session changes
collision reservation, not device mapping or border kernels. The broader scaling
differences are unresolved; the shared contour correction improves the latter
fixture 15,839→13,980 differing pixels without a scaling-specific patch.

## Alpha and compositing

**Source-verified — RGB overlays and matrix metadata.** Libass ass_types.h
documents that its image API preserves ASS RGB and delegates video-dependent
YCbCr conversion to the caller. Pinterf RTS.cpp RenderEx selects ARGB for RGB
outputs; xy_bitmap.cpp TransColor returns unchanged ARGB with its default flags.
Its optional host correction depends on consumer video matrix/range settings
(xy_sub_filter.cpp), which Taiyou's RGB-only API does not receive. Taiyou now
preserves the header in ScriptInfo without rejecting or changing RGB overlays.
This corrects an over-rejection, not an implementation of video-space mangling.
Colored fill/border/shadow tests cover metadata retention and strict/tolerant RGB
identity across matrix headers. FFmpeg n7.1 vf_subtitles.c initializes host
drawing with the video link's format/colorspace/range, then blends libass's RGB
colors/masks. That file does not itself consume the ASS matrix header; it is an
example of the host boundary, not proof of complete legacy color mangling.
Focused comparison at 29b33bb renders embedded-font (192 differing pixels) and
zero-area at 2000/8000 ms (633/303). Three `357-k-and-kf-desynced` frames advance
past the matrix rejection but remain unsupported at event-effect evaluation.
No exact match is inferred from removing the header rejection. Taiyou has no
video-compositor integration to exercise; no FFmpeg dependency was added.

**Source-verified semantics; pixel mismatch.** Pinterf RTS.cpp:2260–2274 animates
the raw hexadecimal alpha target and narrows to BYTE afterward. Taiyou retains
511 from `\1a1FF`, interpolates, then narrows. Existing native tests verify
opacities 47, 250, 5 at 2040, 2560, 4960 ms. The three frozen `vsf-quirks-var/alpha-clipping`
frames remain 691, 673, 572 differing pixels. Their raw output is unchanged by this
session's contour fix; no parser or opacity compensation was introduced.

## Blur and effects

**Partial / mismatch.** Prior source-backed Gaussian/BE behavior is described in
[the T018 evidence](T018-CANDIDATE.md). This session preserves those kernels and
native 0..64 coverage units. Focused BE, Gaussian-transform and anamorphic frames
still differ in alpha. Source geometry/coverage remains separate from filtering;
the current evidence does not justify tuning blur constants to libass PNGs.

## Karaoke

**Partial / mismatch.** Pinterf RTS.cpp:2512 onward resets both clocks for `\kt`
without changing the effect kind; `\kf` advances from the current end. Existing
clock/run-boundary/seek tests exercise that behavior. `v4++/kt-01500` improves
1,734→1,728; each of the five run-split frames improves 11,477→10,900. These remain
mismatches after shared font/contour repairs. Timing, segmentation and
shared font/raster differences are not yet independently isolated for the full
series; timestamps are not treated as separate bugs.

## Parsing and other ASS compatibility details

**Mismatch with correct style selection.** `token-parsing/trimming-0000` selects
the normal Example style and renders Green; the unusual `***Example` style is
irrelevant to this event. The contour fix reduces differing pixels 1,225→1,163;
remaining metrics/coverage differences are unresolved. No token-parser rewrite
was made. Legacy video color conversion remains host-owned; deferred recognized
event effects remain separate from visual matches.

**Verified semantics / pixel mismatch — unknown event effects.** Libass
`ass_parse.c:ass_apply_transition_effects` and pinterf `RTS.cpp:ParseEffect`
leave unknown labels inert. Taiyou preserves the Effect string and ignores
unknown names, following pinterf's case-insensitive, semicolon-delimited known
names. Ordinary and karaoke/fade pixel equivalence witnesses pass in
[focused CI at 290b151](https://github.com/amanosatosi/taiyou/actions/runs/39978189222).
The three `357-k-and-kf-desynced` frames now render without diagnostics;
6798/7170/8170 ms differ from frozen libass by 8,474/8,474/8,483 pixels.
These are not exact matches or authenticated xy captures. Banner/Scroll motion
remains a distinct implementation task.

**Partial — SSA V4 styles.** Pinterf STS.cpp:2011–2026 and libass ass.c
`process_style`/`set_style_alpha` agree: signed AlphaLevel is clamped to 0..255
and replaces primary/secondary/outline transparency; shadow transparency is 128;
BackColour supplies outline RGB, and legacy alignment is mapped into numpad
alignment. Taiyou now normalizes these into the existing style/render pipeline,
including hexadecimal signed alpha and decimal style colors. Authored parser
and SSA-to-explicit-ASS rendering witnesses pass in the existing
[focused CI at 1e19957](https://github.com/amanosatosi/taiyou/actions/runs/39966806938).
All five frozen `ssa/alpha` frames now render instead of returning unsupported;
their differing-pixel counts at 0/1000/2000/3000/4000 ms are
602/550/552/552/602. These remain strict mismatches, not exact compatibility.
The preceding 23 focused cases retain their prior metrics. No reference or
inventory changes.
