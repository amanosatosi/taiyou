# Compatibility findings

Descriptive working notes, updated when evidence changes; no behavioral approval
procedure is implied. Investigations use both libass and pinterf/xy-VSFilter;
shared behavior is preferred, with historical xy behavior taking default priority
when established ASS behavior conflicts. Libass is equally valuable for bug
discovery and modern Unicode/shaping behavior. Local source
identities are recorded in [the reference map](LOCAL_REFERENCES.md). Source
inspection establishes semantics, not pixel equivalence. The frozen libass-tests
PNGs are **libass references**, not authenticated xy captures. PR #35 now also
captures the official xy-VSFilter 3.2.0.810 DLL through CSRI; direct captures take
priority where available. Older source-only findings below retain their original
evidence limits. Internal passing tests alone do not establish compatibility.

**Host font fallback (in progress).** The direct MUA Office witness at `0915b31`
shows angle brackets in xy and missing-glyph boxes in Taiyou. Pinterf
`RTS.cpp:CMyFont/CText::CreatePath` selects a LOGFONT and sends text to GDI;
Taiyou previously exposed only supplied fonts. The host admission path now follows
libass's platform-provider approach (`ass_directwrite.c`, `ass_fontconfig.c`,
`ass_coretext.c`), while retaining Taiyou's whole-cluster HarfBuzz retries and
immutable catalogs. Only OS-selected faces are admitted. Explicit corpus mode
keeps fixture fonts, identities and order; it never discovers installed fonts.
Provider-selected substitutions are catalog metadata, preserved when adding
embedded fonts. This does not claim GDI and platform providers choose identical
faces, scale factors or metrics.

The first direct checkpoint at `2177a94` was **3 same / 10 good / 85 bad /
4 errors**, versus **3 / 10 / 86 / 3** at `0915b31`. All three exact cases were
preserved; there were no new exact cases. The fourth error was the Myanmar
witness exceeding the 128 MiB native font admission reservation, not a visual
improvement. On 2026-10-24 the owner explicitly approved a 256 MiB transient
allowance for system-font playback only. Explicit corpus mode and library
defaults remain at 128 MiB, with the same 64 MiB aggregate font limit and native
reservation checks. At `cf9335d`, the actual full direct comparison is again
**3 same / 10 good / 86 bad / 3 errors**. The MUA witness renders without
missing glyphs but differs at 3,907 pixels. The three errors are xy CSRI open
failures for `scaled-border-default/ffmpeg_legacy2+sbas-8000`,
`ffmpeg_legacy2-8000` and `ffmpeg_legacy3-8000`; Taiyou renders those frames.
See [checkpoint report](https://github.com/amanosatosi/taiyou/actions/runs/40037379434).

**Windows fallback reductions.** The [controlled direct-xy run at
`ccaf700`](https://github.com/amanosatosi/taiyou/actions/runs/40102156323)
uses the same MUA Office Regular font, positive size 60, 1920x1080 and centred
placement for Burmese-only, brackets-only and combined text. Underline-on/off
pairs separate glyph and decoration differences:

| Text | Underlined differing pixels | Without underline |
|---|---:|---:|
| `စမ်းမယ်` | 259 | 259 |
| `〈〉` | 1,004 | 734 |
| `〈စမ်းမယ်〉` | 3,907 | 2,968 |

The Burmese-only bounds and underline already match. Its remaining differences
are on glyph edges; these captures do not demonstrate a different shaping rule.
DirectWrite supplies Yu Gothic UI for the missing brackets. GDI TextOut on the
privately admitted MUA face produces a different bracket outline and keeps MUA's
line metrics and underline across the complete string. Pinterf
`RTS.cpp:120-137,645-691,696-721` uses precisely this positive-cell-height,
GetTextExtentPoint32/TextOut/GetPath path. Libass's font provider remains useful
for bounded discovery, but `ass_font_get_asc_desc`/`ass_render.c:measure_text`
aggregate actual fallback-face metrics; that does not establish GDI parity.

The [bounded GDI-era probe](https://github.com/amanosatosi/taiyou/actions/runs/40103245536)
selects MS PGothic through IMLangFontLink2's code-page mapping. It inspects only
that selected file (SHA-256
`4bde3e6392b96910fb59094c6c1a4dbfae18fee78d0bf13dc30616837c4f95db`).
MS UI Gothic in the same collection has identical bracket outlines; MS Gothic
has a different advance. This observation does not uniquely identify which of
the two equivalent bracket faces GDI internally links. A family name alone also
does not reproduce linked scaling. Microsoft's [font-link documentation](https://learn.microsoft.com/en-us/globalization/fonts-layout/fonts)
distinguishes GDI linking/scaling from fallback; [MapFont](https://learn.microsoft.com/en-us/previous-versions/windows/internet-explorer/ie-developer/platform-apis/aa741044%28v%3Dvs.85%29)
requires the selected source font and code-page mask.

At [`a9ca1db`](https://github.com/amanosatosi/taiyou/actions/runs/40104175335),
Windows playback admits the OS-selected face and immutable cluster/size metrics
at the CLI boundary. HarfBuzz still shapes complete clusters with paragraph
context. The reductions become **259 / 152 / 646** differing pixels, respectively,
with identical underline-on/off differences. The combined underlined bounds,
baseline and continuous primary-font underline now agree. These remain
mismatches, not new exact cases. Rounded advances do not uniquely determine the
native EM or baseline. The adapter now accepts a realization only when a bounded
candidate search reproduces every GDI path point and type exactly, including a
constant baseline translation. The brackets' EM is 3393 native Q6 ticks, rather
than 3394 inferred from their rounded advance. Their baseline differs by one
native tick from MUA's ascent.

**Verified checkpoint — word extent and placement.** Pinterf
`CText::GetTextInfo` stores `(extent.cx + 4) >> 3` per text word; fallback/script
item boundaries within a word are not independent rounding boundaries. Taiyou
now retains native glyph positions inside each word and uses its rounded extent
for subsequent words and alignment, in the physical LayoutRes metric grid.
The [full direct-xy run at
`a541a97`](https://github.com/amanosatosi/taiyou/actions/runs/40107881363)
reports **4 same / 10 good / 85 bad / 3 errors**, preserving all three preceding
exact cases and adding `regression/faux-italics/truetype-0000.png` (99 differing
pixels to zero). CFF improves from 135 to 42 differing pixels, all on edges; it
remains a mismatch. No shear coefficient or raster tolerance was tuned.
The [controlled MUA captures](https://github.com/amanosatosi/taiyou/actions/runs/40107877550)
are now **214 / 0 / 188** differing pixels for Burmese/brackets/combined,
respectively, for both underline-on and underline-off. Brackets-only is exact;
the other two retain glyph-edge differences. Identical underline-on/off counts
and the primary line metrics isolate those residuals from underline integration.
These captures do not prove complete FreeType/GDI shaping or outline parity.
The strict supplemental five-case smoke deliberately remains red for CFF's 42
pixels. Its three original exact guards and TrueType all pass. Corpus output
continues to use explicit fixture catalogs; MUA alone opts into system fonts.

The native-centering correction follows `CLine::PaintAll`'s integer
`m_width / 2`, avoiding a spurious sixteenth-pixel phase for odd eighth-pixel
extents. New numeric witnesses cover split shaped runs, following words and a
non-unit physical metric grid. The [full direct run at
`c914602`](https://github.com/amanosatosi/taiyou/actions/runs/40110046313)
is **4 same / 11 good / 84 bad / 3 errors**, with the same four exact cases and
no regressed exact cases. The six MUA reductions retain **214 / 0 / 188**.
The [separate full frozen-libass comparison](https://github.com/amanosatosi/taiyou/actions/runs/40110046334)
remains red: **1 exact / 100 pixel mismatches / 0 unsupported / 0 execution or
infrastructure errors / 0 crashes**; two successful parser-only probes and one
upstream-disabled source are separate from those 101 visual frames. All 187
frozen assets are reverified. Direct-xy success is never inferred from that
different oracle's PNGs.

The [native contour dump](https://github.com/amanosatosi/taiyou/actions/runs/40110042300)
retains seven HarfBuzz glyphs for Burmese-only: IDs 9, 29, 276, 274, 29, 30, 276,
with baseline 2713 Q6 ticks and scale 2.584 native ticks per design unit. GDI's
572 path points each have a nearest point of the same command kind in the
quadratic-to-cubic converted outline within one native Q6 tick (including
explicit closing endpoints); this is not a proof of path-order equality. For example,
GDI has cubic control `(198,1692)` where the continuous converted outline rounds
to `(198,1691)`. This is native outline conversion/rounding evidence, not a
proof of the precise scaler rounding rule. Production fill already uses the xy
span rasterizer; substituting a FreeType grayscale rasterizer cannot explain or
repair that upstream path discrepancy. No family-specific coordinate adjustment
or speculative shaping rewrite is made.

The owner separately authorized correcting the independent single-H C
reference's word extent/centering and the W2 experiment's layout-equivalence
expectation. Its original raw width measurements, historical records, workloads,
native-work/budget assertions, every alignment and zero-tolerance comparison
remain intact. The T017 shadow timer now enters only the executed shadow branch,
independently of its skip predicate. Its CLI instrumentation preserves the actual
font-source configuration; current measured cohorts explicitly select fixture
fonts and the original 128 MiB budget. The [full native matrix at
`b5902b3`](https://github.com/amanosatosi/taiyou/actions/runs/40126071476) passes
Windows 2022, Ubuntu 24.04 and macOS 15. At `d46ac86`, the repaired optimized
experiment retains 54 newly executed samples: 48 completed frames and six typed
default-segment-limit failures across the two 500-event workloads. Expanded
caller-budget variants and full/partial/minimal identity checks pass. Historical
records, workloads, safety budgets and no-retry invariants remain intact; the
failure event ordinal may change with corrected geometry. Hosted smoke is not
controlled playback throughput.

The Windows adapter uses platform GDI32/COM/MLang supplied by the runner's Windows
installation, with DirectWrite retaining file admission and the general fallback
path. No new Cargo package, font-directory scan, font name table or redistributed
native library is added. Private GDI registration is request-scoped and serialized;
no native handle enters a published catalog. Observations are keyed by primary
face, selected face, complete font request, cluster text and physical cell size;
catalog rebuilding preserves them. Explicit catalogs have no host observations
and retain 128 MiB. New public accessors/native calls still require engineering
review; the supplemental captures do not constitute maintainer acceptance.

**Repair sprint — ASS pair kerning.** Hash-authenticated Windows GDI probes
of the frozen DejaVu Sans and Arimo faces report additive A/V and T/o widths at
positive cell height 6400. Pinterf's ASS TextOut/GetTextExtent path differs from
its separate libssf kerning path. Libass parses the script's Kerning header and
configures its shaper accordingly (`ass.c:parse_bool`, `ass_shaper_set_kerning`);
its missing-header default is false. The ASS coordinator now requests additive
advances by default, with the retained libass `Kerning` header as an explicit
opt-in (yes/positive decimal prefixes, final duplicate wins). Direct shaping
retains its kerning-on default; a per-request option controls the native kern
feature without disabling GSUB ligatures, mark positioning or context. This
corrects ASS behavior, not public stage types or the generic shaping default.
Reduced additive-width/header/reset/karaoke tests pass on macOS at `c105dc0`.
The fixed segment-budget workload explicitly retains `Kerning: yes` so its
previously measured geometry, counts, limits and no-retry assertions stay intact;
the new regressions exercise the corrected default separately. No prior
unverified experiment is imported. Current exact-head Actions and pixel results
are recorded in [repair PR #35](https://github.com/amanosatosi/taiyou/pull/35).

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

**Verified direct xy — synthetic italic reductions.** Pinterf's LOGFONT italic request reaches GDI
TextOut/GetPath. The focused Windows GDI probe authenticates the two frozen font
bytes and verifies actual selected family. At positive cell height 6400, CFF slant
is approximately 0.1763 and TrueType approximately 0.33984, with unchanged advance.
Taiyou applies format-specific shear before vertical orientation/ASS transforms,
updates ink support and avoids double-slanting actual italic faces. Catalog,
shaping and rendered/seek regressions pass. The later [authenticated direct run
at `889440d`](https://github.com/amanosatosi/taiyou/actions/runs/40127465376)
makes both frozen faux-italic frames exact against xy. They still differ from
the separate libass PNGs.

The CFF program has rectangle corners `(3,863)`, `(3,-144)`, `(997,-144)`,
`(997,863)`, UPEM1000, cell863+144 and advance977. GDI measures the rounded EM6356
at cell6400, yielding advance6210 Q6 ticks. Its CFF contours instead retain the
fractional scale6400/1007: sheared corners round to `(986,0)`, `(-142,6400)`,
`(6175,6400)`, `(7304,0)`, matching the authenticated GDI path. Using the rounded
metric scale for contours made the lower-right x coordinate6176 and left 42
different edge pixels. Taiyou now separates horizontal CFF contour/ink scaling
from rounded metric/advance scaling, using portable catalog data and arithmetic.
TrueType, vertical mapping and admitted host realizations retain their existing
scales. No font name, glyph ID or coordinate offset selects this behavior.

## Glyph outlines and rasterization

**Opaque outline underlay correction.** Pinterf `RTS.cpp:CLine::Paint` passes
both body/border flags for the outline only when primary and secondary colors
are opaque and the event fade is zero. `Overlay::FillAlphaMash` then paints
the full expanded mask. Libass's `FILTER_FILL_IN_BORDER` has the same condition.
Taiyou previously subtracted fill coverage unconditionally, leaving alpha gaps
at antialiased body edges even inside a solid outline. It now retains the
expanded mask under opaque evaluated colors, while translucent colors retain
subtraction. The reduced horizontal-border regression compares complete buffers
against independently drawn rectangles for both opaque and translucent colors.

**Transparent-fill shadow correction.** The same pinterf path enables the
shadow body only for nontransparent original primary paint or sweep/outline
karaoke. Libass `FILTER_FILL_IN_SHADOW` agrees. A fully transparent ordinary
fill casts only its outline; without an outline it casts no shadow. Preserve
these two coverage decisions before applying event fade: a nonzero original
opacity that rounds to zero during fading still casts a filled shadow. The
immutable evaluated run retains those decisions, and coverage grouping compares
them. Shadow subtraction uses the unfiltered body at the translated shadow
phase after filtering its expanded outline. Full-buffer rectangle controls
exercise both offset signs, instant/sweep/outline karaoke, fade rounding and
the unbordered case. The former zero-offset shadow regression now uses a
nontransparent original fill faded to zero, preserving its coincident-shadow
assertion while correcting its prior fully-transparent premise.

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

**Drawing-word grouping correction.** Adjacent drawing words must retain
independent coverage even when their styles match. Pinterf
`RTS.cpp:CPolygon::Append` always returns false; libass
`ass_render.c:split_style_runs` breaks on either side of a drawing. Taiyou's
former contour union erased fractional-position seams and changed translucent
overlaps in reduced polygon fixtures. Coverage grouping now preserves these
boundaries, including drawing/text boundaries. Multiple contours within one
drawing still use nonzero winding. The reduced native regression compares the
complete buffer with separately positioned polygons at eighth/half-pixel
phases and opaque/translucent paint. These native tests pass in
[focused CI at 84c5d1e](https://github.com/amanosatosi/taiyou/actions/runs/40004530517/job/120589048112).
The frozen `drawImg` raw output remains unchanged, with 1,048 differing pixels.
Its move samples x=953.1 before quantization: pinterf's EF_MOVE truncates to
953.0 in eighth-pixel coordinates, while libass's `quantize_transform` rounds
the raster phase to 953.125. The resulting fractional rectangle seams explain
the regular bands; copying that libass phase would violate the xy motion rule.
This external frame remains a mismatch, not a completed pixel-compatibility fix.

Pinterf CPolygon::ParseStr derives polygon advance from bounds; CText's
`fontSpacing` is absent from that path. The existing four-square reduced native
test preserves identical geometry for spacing 0, -0.6 and 1.25.

## Layout, positioning and collision

**Repair sprint — zero-advance accents.** HarfBuzz 14.5.0
(`863d3f7787c6df18d20e4535c5906bf3eb803bd5`, `hb-ot-shape.cc`,
`hb_insert_dotted_circle`) inserts a generic base when BOT is set on a leading
mark. Taiyou set BOT; libass `ass_shaper.c:shape_harfbuzz` leaves it unset.
The adapter now leaves BOT unset while retaining paragraph context and
script-specific shaping. Zero-width, unbordered text also retains a zero-width
collision reservation despite ink overhang, matching pinterf `AllocRect`'s empty
intersection rule and libass `ass_render.c:fix_collisions`' zero-area exclusion.
`visual_compatibility` tests literal accent glyphs, visible ink, horizontal
outline reservation, both event orders and repeated/backward seeks. At
`0d166500fd6d3060bbcaa4537786253c41beb75f`, the
[full comparison](https://github.com/amanosatosi/taiyou/actions/runs/39998330806/job/120568011562)
reduces differing pixels 633→141 at 2000 ms and 303→141 at 8000 ms; alpha MAE
falls 253.09360/67.89148→2.24020. Both orders produce identical PNGs. All other
99 visual raw hashes remain unchanged. Counts remain 1 exact, 100 mismatches,
0 unsupported, 0 crashes/infrastructure errors. Windows/macOS native suites
pass; the Linux job was superseded by the kerning checkpoint. These are libass comparisons,
not authenticated xy captures; the remaining 141 pixels are still mismatches.

**Scroll / legacy FFmpeg distinction.** The inspected `effect-delay` triplet
has equal glyph size and four-line height, but different word breaks. Pinterf
`RTS.cpp:GetWrapWidth/GetNextLine` selects each line's target from remaining
advance width; libass `ass_render.c:wrap_lines_rebalance` subsequently moves
words between adjacent lines. Standard Taiyou wrapping follows pinterf, per B2;
copying libass's breaks would change that policy. Likewise libass `ass.c`
`custom_format_line_compatibility` enables SBAS for the legacy2/3 custom formats,
and `detect_legacy_conv_subs` recognizes legacy4's FFmpeg signature. Pinterf
`STS.cpp` initializes SBAS false without those exceptions. Neither policy is
silently changed to fit the frozen libass PNGs. These cases remain mismatches.

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

**Partial — Banner/Scroll motion.** Pinterf `RTS.cpp:ParseEffect` and
`EF_BANNER`/`EF_SCROLL`, and libass `ass_apply_transition_effects`, scale delay
into layout pixels, truncate it and clamp it to at least one. Taiyou now uses
the existing placement/layout/clip paths for both Banner directions and Scroll
up/down, including Banner's initial nowrap mode, collision exclusion and scroll
band clipping. Pinterf samples motion in integer eighth-pixels; libass's shift
is continuous. Taiyou follows the former. Nonzero fadeaway gradients remain
explicitly unsupported. Authored positioned/clipped drawing and LayoutRes delay
witnesses pass in [focused CI at 24462fb](https://github.com/amanosatosi/taiyou/actions/runs/39979500404).
The preceding 33 rendered comparison frames retain identical pixels and metrics;
the six frozen `ssa/effect-delay` frames are measured by the full comparison.
Source agreement and internal witnesses do not establish exact xy output parity.

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

**Verified direct xy — packed RGB and inverse alpha.** Pinterf
`Rasterizer.cpp:pixmix_c/packed_pix_mix_sse2` combines the complete numerator
`dst*(256-a)+color*(a+1)` before truncating `/256`, while its bitmap stores inverse
alpha. At drawImg `(949,684)`, xy has premultiplied `(23,6,6,98)`; Taiyou's old
separately rounded `/255` products gave `(24,7,7,98)`. The portable native-coverage
paint path now follows the measured packed rule and converts inverse alpha to
public opacity. Generic A8 composition remains separately typed. Byte-range,
premultiplication, zero-coverage and overlap regressions remain exact.

The full direct scoreboard progresses **4/11/84/3 → 5/13/81/3 → 6/12/81/3**
(same/good/bad/errors): [packed paint](https://github.com/amanosatosi/taiyou/actions/runs/40125520184)
adds exact `regression/draw-images/drawImg-104396.png` (236 pixels to zero);
[fractional CFF contours](https://github.com/amanosatosi/taiyou/actions/runs/40127465376)
adds exact `regression/faux-italics/cff-0000.png` (42 pixels to zero). Both
`scaling_playres/playresx_sbas-{no,yes}-1000.png` move bad to good. All prior exact
frames remain exact; there are no good-to-bad changes against the 4/11 checkpoint.
The three errors remain xy CSRI open failures, with Taiyou renders retained.
MUA stays **214/0/188** with or without underline; it is not promoted to exact.

The [supplemental direct smoke](https://github.com/amanosatosi/taiyou/actions/runs/40127461718)
passes all six selected corpus frames and an authored packed-paint/overlap
control. The earlier authored control first lacked a metadata field and then
used abbreviated ASS fields that pinterf interprets positionally; its failed
attempts remain in their original artifacts. The corrected control uses standard
ASS style/event fields and is separate from the unchanged 102-frame scoreboard.
Correcting that diagnostic input is not counted as a renderer compatibility win.

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
past the matrix rejection but were still unsupported at event-effect evaluation
at that checkpoint; the unknown-effect correction below removes that blocker.
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
These are not exact matches or authenticated xy captures. Recognized motion
effects use their own implementation; unknown labels do not activate it.

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
