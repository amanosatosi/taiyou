# Contracts

Normative for interfaces, units, ownership, resource limits and safety invariants. Rust below is interface notation, not compilable code or existing APIs. Behavioral descriptions record the implementation and are corrected when historical evidence disproves them; they are not vetoes against ordinary compatibility repairs. See descriptive [compatibility findings](compatibility-notes.md). Types live in `taiyou-types` unless noted. Private fields and checked constructors enforce invariants. `Arc` means shared immutable ownership; borrowed inputs last only for the call. Every stage is fallible where input or allocation can fail.

## C1. Units, arithmetic and identity

```rust
struct TimeUs(i64);                       // signed microseconds
struct EventId(u32);                      // source ordinal, not layer
struct TrackRevision(u64);                // renderer-assigned snapshot identity
struct CatalogRevision(u64);
struct StyleId(u32);
struct FontId { sha256: [u8; 32], face_index: u32 }
struct SourceSpan { start: u32, end: u32 } // half-open UTF-8 source byte range
struct DecodedSpan { source_map: Arc<SourceMappedText>, start: u32, end: u32 }
struct ScriptPoint { x: FiniteF64, y: FiniteF64 }
struct DevicePoint { x: Q6, y: Q6 }        // i32 ticks, 64 per output pixel
struct PixelRect { x0: i32, y0: i32, x1: i32, y1: i32 }
struct Size { width: u32, height: u32 }
enum Profile { CoreV1, LegacyXyV1 }
```

`CoreV1` is Taiyou's current implementation profile. Ordinary ASS/SSA behavior proven incompatible with pinterf/xy-VSFilter is corrected at the implementation and directly affected tests; an intentional difference belongs in an explicit extension or separate mode. `LegacyXyV1` remains representable for compatibility bookkeeping, not as a research admission gate. Profile and semantic revision are part of every affected key.

Input timestamps are converted with checked integer arithmetic. ASS `h:mm:ss.cc` uses centiseconds (10,000 µs); tag animation/move/fade times use milliseconds (1,000 µs); karaoke durations use centiseconds. Active interval is `start <= now < end`; zero/negative duration events are inactive with diagnostics. Request time is absolute and seeking has no history dependence. Fractions are evaluated from `now-start`, never accumulated frame deltas.

Script coordinates: top-left origin, x right, y down. Glyph contours start in font design units, y up; convert once at the font-to-script boundary. Device coordinates are physical output pixels, y down. Rectangle bounds and buffer ranges are half-open. A pixel center is `(x+0.5,y+0.5)`; coverage integrates over its unit square. Q6 conversion is nearest, ties away from zero; floor division and remainder produce integer offset and phase 0..63, including negative positions. Example: -1 tick becomes offset -1, phase 63. Use i64 intermediates and checked narrowing.

`FiniteF64` rejects NaN and infinities; canonicalize -0 to +0 for keys. Do not round parameters just to improve hit rate. Transform math uses f64, no fast-math or FMA-only shortcuts. Device path coordinates quantize once to Q6; blur kernels have separate quantization. Equality of key hashes alone is insufficient: compare full canonical keys. IDs are session-scoped, not persisted or pointer-derived. IDs and output order never depend on hash iteration.

## C2. Input and immutable IR

```rust
fn parse_ass(bytes: &[u8], options: &ParseOptions, ingestion: &mut TrackIngestor)
    -> Result<ParsedTrack, ParseError>;
struct ParsedTrack { track: Arc<Track>, diagnostics: Vec<Diagnostic> }
struct Track {
    source: Arc<str>, script: ScriptInfo,
    styles: Arc<[Style]>, events: Arc<[Event]>, revision: TrackRevision,
}
struct Event {
    id: EventId, layer: i32, start: TimeUs, end: TimeUs,
    style: StyleId, margins: Margins, actor: Arc<str>, effect: Arc<str>,
    program: Arc<[TextOp]>, source: SourceSpan,
    text_source: Option<SourceSpan>, operation_sources: Option<Arc<[Option<SourceSpan>]>>,
}
enum TextOp {
    Text { text: Arc<str>, source: SourceSpan }, HardBreak, SoftBreak, NoBreakSpace,
    Override(OverrideOp), Drawing(DrawingProgram),
    Unsupported { name: Arc<str>, source: SourceSpan },
}
```

The explicit ingestion argument is accepted by the repository owner's architectural decision in [ADR-008](decisions/008-track-ingestion-identity.md). A renderer/session coordinator owns one non-cloneable TrackIngestor for all admitted tracks; it allocates revisions at immutable publication, never recycles them and fails on exhaustion. ParseOptions remains value configuration. No parser may invent a global or pointer-derived identity policy.

Under owner-accepted [ADR-010](decisions/010-shaped-cluster-provenance.md), the optional operation-span sidecar preserves TextOp construction compatibility. Parser spans are checked against the actual event Text field, source order and exact escape bytes. Replacing a program clears its old sidecar. A host-created spanless unit remains valid IR but cannot claim exact shaping provenance without explicit valid information. SourceMappedText lives in taiyou-types with the existing font-module re-export retained; it owns original source and ordered decoded-to-original intervals. DecodedSpan is constructed only against its retained immutable map and validates UTF-8 boundaries. APIs combining spans/maps reject foreign-map spans, even if byte extents coincide. Ownership checks are not persisted IDs or cache keys.

V1 input accepts UTF-8 and UTF-8 BOM, CRLF/LF/CR. Invalid UTF-8 is an error, never guessed using OS locale. T026 adds explicit host-selected legacy decoding and records encoding provenance. Section/field names are ASCII case-insensitive; preserve text and font names. Parse `Format:` dynamically with the text field last for the Core subset; reject unsupported text-not-last records with diagnostic rather than splitting text commas incorrectly. Trim structural whitespace only. Unknown sections are ignored with bounded diagnostics. The owner-directed T008 compatibility follow-up admits ASS `[Fonts]` / `fontname:` attachments: offset-33 six-bit encoding, four characters per three bytes, two/three-character final groups yielding one/two bytes. Empty, one-character-tail or invalid-character payloads are malformed records (Strict rejects; Recover discards the attachment with a diagnostic); resource failures remain fatal. Blank lines, the next fontname, a standard ASS section or EOF finalize a payload. Names are metadata only; no attachment filesystem writes. ParsedTrack retains bounded decoded bytes and original spans; render_parsed registers them after host faces through the normal validated catalog, with aggregate font-byte limits. Native-invalid fonts cannot bypass validation. SSA V4 styles normalize signed AlphaLevel into fill/secondary/outline opacity, use BackColour for outline RGB and fixed shadow transparency 128, and convert legacy alignment into the existing style model. Legacy subtitle byte decoding remains separate T026 work. Other binary attachments remain unsupported. No implicit filesystem font paths.

Core style defaults: family `Arial` requested through the catalog, size 20 script units, bold/italic/underline/strikeout false, scale x/y 100%, spacing 0, angle 0, BorderStyle 1, border 2, shadow 2, alignment 2, margins 10, Encoding 1; opaque white primary, opaque yellow secondary, opaque black outline/shadow. Missing `Default` synthesizes this style with a diagnostic; unknown event style resolves to Default with diagnostic. Duplicate names: last definition wins, resolved after all sections. Preserve all definitions for diagnostics. Core default PlayRes is 384×288 only if both absent; if exactly one absent return `UnsupportedFeature` until the pinterf behavior is implemented and tested. Nonpositive specified resolution is invalid. Core default WrapStyle 0; Core default ScaledBorderAndShadow false. Historical defaults follow pinterf when implemented; they are not inferred from unrelated Core choices.

`Style` contains named fields matching the defaults plus name, font request, four straight RGBA colors, alignment 1..9, margins and encoding. Source ASS `&HAABBGGRR` maps to `r=RR,g=GG,b=BB,opacity=255-AA`. Header fields retain PlayRes, LayoutRes if present, WrapStyle, ScaledBorderAndShadow, YCbCr Matrix, collisions and unknown metadata; unsupported fields remain reportable. Style values are script-space, never already device-scaled.

Core numeric tokens accept optional sign and decimal digits with an optional fractional part, no exponent/locale separator/NaN/Inf. Integer fields reject fractions. Color style fields accept `&H` plus 1–8 hex digits and optional trailing `&`, zero-extended; decimal style colors are deferred to T026/source-backed compatibility work. RGB override colors accept 1–6 hex digits and preserve current opacity; alpha overrides accept 1–2 hex digits and invert transparency to opacity. Invalid/out-of-range supported-tag parameters diagnose and leave the prior state unchanged; strict mode returns InvalidScript. Parameterless `\r` is valid; T018 `\blur` and `\be` accept omitted values restoring the original event style. Their source-derived numeric-prefix grammar admits exponents and retains finite signed targets; static evaluation clamps negative Gaussian to zero while Style.blur remains nonnegative; other empty Core parameters are invalid until a legacy reset rule is enabled. Core `\b0` selects weight400, `\b1`/`\b-1` weight700, `\i0` false and `\i1`/`\i-1` true; other weights/forms require source-backed implementation. Font size must be positive; scales/border/blur nonnegative; signed spacing/shadow/position allowed. These restrictions intentionally keep the initial supported grammar small and explicit.

`OverrideOp` is a closed enum introduced by supported tasks: SetFont, SetSize, SetWeight, SetItalic, SetScale, SetSpacing, SetPaint(channel), SetOpacity(channel/all), SetBorder, SetShadow, SetBlur, SetPosition, Move, SetAlignment, SetOrigin, SetRotation, SetClip, ResetStyle, Transform, Fade, Karaoke and DrawingMode. Store typed parsed parameters and original order, not precomputed frame values. Unknown operations retain spans, not executable strings. Parser recovery must advance input; cap token count and nesting. Historical malformed/nested tag behavior follows pinterf; do not recursively parse arbitrary nesting as a guessed compatibility fix.

Core reset disambiguation follows owner-accepted [ADR-009](decisions/009-core-reset-token-disambiguation.md): bare `\r` is ResetStyle(None). After structural whitespace trimming, `\rName` is a named reset only when Name exactly matches a final declared style name (case-sensitive; forward definitions, last-definition-wins and synthesized Default apply). Other r-prefixed tokens are Unsupported with original spans and diagnostics, never implicit Default resets. Thus `\rnd` resets style nd if that style exists, otherwise it remains unsupported. Future extension admission must review collisions and profile policy; this is not a Legacy rule.

## C3. Evaluation and dependency classification

Owner-directed ASS compatibility batch: a LayoutRes pair is active only when
both axes are positive, superseding host storage/original-video size while
PlayRes remains the script coordinate system. Font construction and integer
position/clip rounding occur in layout pixels; layout-to-output scaling is a
separate map. See the exact [layout and font source paths](T018-CANDIDATE.md#layout-unknown-commands-vertical-fonts-and-opaque-boxes).
Unknown command tokens are consumed without executing nested contents; known
unimplemented tags remain explicit UnsupportedFeature. The parser retains raw
text boundaries so ignored blocks still delimit karaoke words. Longest known
prefix dispatch makes `board1` a normal-weight b assignment, not a new feature.

The same batch implements vertical @ requests and standard BorderStyle=3.
@ affects validated family lookup, font metrics, vertical variants, advances and
owned glyph contour orientation; it does not switch to a second paragraph or
effects renderer. Boxes use outline paint and source-defined word/cell geometry,
native padding, copied-style transforms, independent blur and box shadows.
Visible box support participates in the existing deterministic collision policy.
Styles Encoding 0/1 use the supplied Unicode face; legacy charset/cmap emulation,
fe and host video-space YCbCr mangling remain separate from RGB overlay rendering.

```rust
fn evaluate(track: &Track, event: EventId, now: TimeUs, profile: Profile)
    -> Result<EvaluatedEvent, RenderError>;
struct EvaluatedEvent {
    id: EventId, order: EventOrder, runs: Arc<[EvaluatedRun]>,
    placement: Placement, clip: ClipSpec, animation: ChangeClass,
    source_map: Option<Arc<SourceMappedText>>,
}
struct EventOrder { layer: i32, source_ordinal: u32 }
struct EvaluatedRun {
    text: Arc<str>, source: SourceSpan, shaping: ShapeStyle,
    decoded: Option<DecodedSpan>,
    geometry: GeometryStyle, effects: EffectStyle, paint: PaintSet,
    karaoke: Option<KaraokeSpan>,
}
```

Evaluate overrides left to right; text captures current state. `\r` resets style state, named `\rName` uses its exact final resolved style under ADR-009, and does not reset the event clock. Unresolved r-prefixed tokens stay Unsupported, so they do not reset style state. Standard ASS placement/alignment precedence follows pinterf's first-applicable behavior where source-verified; extensions remain separate. Unsupported operations generate diagnostics; a strict request fails with `UnsupportedFeature`, tolerant mode proceeds with a conspicuous report. Reference validation always records diagnostics and disallows unsupported behavior in a supported fixture.

Owner-authorized ordinary ASS event effects (pinterf RTS.cpp 2372-2399,
2446-2453, 2552-2605, 3157-3229): the first valid `\pos`/`\move` shares one
placement slot; the first `\org` and the first `\fad`/`\fade` have separate
slots, persistent across style resets. Invalid effect arities are ignored as in
that reader; numeric coordinates consume a real prefix, times a signed decimal
integer prefix. The six-argument move sorts reversed times; when both times are
nonpositive, use [0,event duration]. At/before t1 choose start, inside interpolate,
otherwise choose end. Endpoint coordinates truncate after script-to-layout scale
to integer eighth pixels; interpolation truncates in that space before separately
declared raster-target scaling. Origin follows the same coordinate conversion.
`\fr` aliases `\frz`, an ordered signed real z-rotation property, including
CalcAnimation in `\t`; omitted values restore the original event style angle.
Position, move, origin and fades inside `\t` remain immediate event effects,
not interpolated properties. Style reset restores rotation but not effect slots.

Owner-directed ordinary ASS coverage extends numeric CalcAnimation to frx/fry,
fax/fay and independent xbord/ybord/xshad/yshad. Border axes clamp nonnegative
after interpolation; shadow axes retain their signs. Combined bord/shad update
each current axis independently; shad clamps nonnegative as in pinterf while
xshad/yshad permit signed offsets. Underline/strikeout use decimal-prefix
0/1 switches (other or omitted values restore the original style). SSA a uses
the historical grid mapping and the same first-alignment slot as an.
These switches and karaoke commands apply immediately inside t. Karaoke clocks
are signed integer local milliseconds: k/K/kf/ko add a real centisecond value,
truncated after addition (missing means 1000ms); kt sets both clocks without
changing type. Each source text/space/newline word consumes its start-to-end
interval; NBSP remains inside a word. k/ko switch secondary to primary at start;
kf/K sweep in device x, retaining xy's z-angle colour reversal during the sweep.
ko suppresses outline before start, without suppressing the separate shadow.
No frame-history state or generic animation system is introduced. See the
[source locations and backend limitations](T018-CANDIDATE.md#ordinary-ass-coverage-batch).

Fades use integer local milliseconds. Two parameters encode alpha [255,0,255]
and times [-1,in,out,-1]. Seven parameters retain the three signed transparency
values and four signed times. Both tag names use the same xy arity dispatch.
A pair of -1 outer times substitutes [0,in,duration-out,duration]. Evaluate in
source order: before t1 a1; [t1,t2) truncated real interpolation a1 to a2;
[t2,t3) a2; [t3,t4) truncated interpolation a2 to a3; at/after t4 a3.
Do not reorder overlapping fade intervals. Event transparency multiplies normal
paint opacity using native MulDiv nearest rounding and BYTE conversion; values
at/below zero leave paint unchanged. Apply after all ordered style/paint overrides
to all words, preserving shaping/layout/source data. Event end remains inactive.
Evaluation retains one complete source map and checked decoded run ranges when every text operation has exact provenance. Otherwise ordinary evaluation remains available with no complete map, and exact shaping reports the missing provenance. The existing source field remains diagnostic provenance; it never becomes a fabricated escape/cluster span. Paint boundaries preserve decoded positions without forcing a shape split.

`ChangeClass` is an optimization hint with independent shape/layout/geometry/effect/paint/placement/clip flags; actual cache reuse compares full keys. It is never permission to omit a key field. Static results can be retained across all timestamps in their active interval. Animated values are evaluated each request, but equal normalized values reuse artifacts. Do not key every stage by time.

Owner-authorized T018 ASS transform amendment: `\t` uses pinterf `CalcAnimation`
(RTS.cpp 2647–2683, 2856–2871), not the former planned Core interpolation.
Local time and duration are integer milliseconds. Zero/omitted end means event
end; omitted start/acceleration are 0/1. Before start keep current, inside the
interval compute `(1-p)*current+p*target`, `p=pow((t-start)/(end-start),accel)`;
at/after end use target. The before-start test precedes the end test, including
reversed intervals. Differences below 0.0001 use target directly. Preserve raw
signed targets until field clamps. Color interpolation uses each native shifted
channel followed by integer truncation/masking; transparency is truncated before
opacity inversion. Nonfinite evaluation fails without partial output. Finite
zero/negative acceleration follows xy rather than a new positive-only grammar.
Children execute in source order against current state. Font family, bold,
italic, alignment, positioning, reset and q are immediate, not interpolated;
Taiyou warpmode remains a standalone event selector, unsupported inside transforms
in accordance with its existing extension contract. Flat bounded transform
delimiters retain disjoint header/child/tail source spans. Nested transforms
update xy's shared animation parameters; leaving them does not restore prior
parameters. No LegacyXyV1 admission or pixel-identity claim follows.

## C4. Font resolution, shaping and layout

```rust
struct FontBlob { id: FontId, bytes: Arc<[u8]> }
struct FontCatalog { revision: CatalogRevision, faces: Arc<[FontRecord]> }
struct FontRequest { family: Arc<str>, weight: u16, italic: bool, vertical: bool }
fn resolve_runs(catalog: &ValidatedFontCatalog, text: &str, style: &ShapeStyle)
    -> Result<Vec<ResolvedRun>, FontError>;
fn shape(ctx: &mut FontContext, input: &ResolvedRun)
    -> Result<ShapedRun, FontError>;
struct ShapedGlyph {
    font: FontId, glyph: u32, cluster: DecodedSpan,
    advance_x: FiniteF64, advance_y: FiniteF64,
    offset_x: FiniteF64, offset_y: FiniteF64,
}
struct ShapedRun {
    glyphs: Arc<[ShapedGlyph]>, metrics: RunMetrics, direction: Direction,
    decoded: DecodedSpan, source_map: Arc<SourceMappedText>, bidi_level: u8,
    script: u32, language: Arc<str>,
}
fn layout(input: &ShapedEvent, constraints: &LayoutConstraints)
    -> Result<LocalLayout, RenderError>;
fn resolve_collisions(events: &[CollisionInput], policy: &CollisionPolicy)
    -> Result<Vec<PlacedEvent>, RenderError>;
```

Font catalog is a deterministic ordered list supplied by the host. Match requested family case-insensitively using Unicode library normalization chosen in T008; exact same-family weight distance, then italic match, then catalog ordinal breaks ties. Never silently use host enumeration order in fixtures. Fallback considers a whole grapheme/shaping cluster with neighboring shaping context, not isolated combining marks. Retry missing-glyph runs against the next candidate, bounded by catalog length. If no face can shape a cluster, use explicit catalog fallback .notdef plus a diagnostic; no random installed font. Language, script, direction, features, variations and face identity belong to the shape key.

The owner-directed PR #35 host mode discovers OS-selected candidates before
publishing this catalog, following libass's platform-provider approach. Explicit
and embedded faces are admitted first. Providers do not split shaping clusters;
Taiyou validates and shapes their selected bytes normally. Only this explicit
host admission step may record a substitute for an absent family. Substitutions
are immutable catalog metadata and survive `FontCatalogBuilder::from_catalog`
when the renderer appends attachments. The library default and corpus adapters
remain explicit, with no system discovery. CLI system mode uses the owner-approved
256 MiB transient allowance; explicit mode and library defaults retain 128 MiB.
Both retain all other limits, including the 64 MiB aggregate font-byte cap.

T008's owner-directed ASS compatibility completion retains localized Unicode
family/typographic-family (name IDs 1/16), full, PostScript and WWS-family aliases
(4/6/21) from the validated face. Unicode-platform and Microsoft-platform name
strings decode as UTF-16BE, including legacy Microsoft encoding IDs. The same
Unicode default case fold applies to every alias; no locale, compatibility
normalization, punctuation stripping or unrelated-family substitution. Aliases
only broaden membership of a face in a requested family: weight distance, italic
match and original catalog ordinal still decide selection. FontId, preferred
family metadata, immutable font bytes and catalog face count are not duplicated.

Glyph cluster ranges are decoded-byte ranges bound to the run's immutable map. Original provenance is the complete ordered list of original intervals obtained from that map, including noncontiguous fi/combining clusters across overrides; never replace it with a continuous hull. Results retain source/map ownership after temporary/native owners are destroyed. A fallible event shaping coordinator may accept an EvaluatedEvent, validated catalog, explicit language/base-direction options and Limits, returning logical-order runs, retained break ranges, paint spans, bounded missing-cluster diagnostics and owned glyphs. The first logical scalar selects cluster paint. Hard-break delimitation is shaping preparation, not line layout or visual reordering. The plain-text resolve_runs entry point treats host text itself as its original source and must not imply ASS provenance.

HarfBuzz consumes immutable font bytes through OpenType font functions, integer design-unit scale and per-call buffer. Do not mix FreeType advances with HarfBuzz advances. Under the owner-directed T018 compatibility correction, ASS font_size is positive GDI cell height. Read validated per-face Windows ascent/descent and UPEM; round the requested cell height at64 units, round its mapped em height, then scale design units by em_height/(64*UPEM). Derive each ascent/descent by native nearest rounding followed by xy's (metric+4)>>3 conversion to eighth-units. Shaping advances, glyph ink and outline geometry share that scale. Catalog metrics remain immutable and resource-accounted; no system/GDI production dependency is introduced. Native cell metrics are an explicit fallback for faces without usable OS/2 values, without a GDI-equivalence claim. A request rounding to unspecified GDI default height is unsupported. Metric policy2 supersedes em policy1. Scale factors follow shaping and are accounted in advances. Core kerning on, standard ligatures on unless nonzero spacing; Historical metrics/features follow pinterf when implemented. Keep glyph positions and cluster maps through all later stages. Paint boundaries do not split shape runs in Core; map paint spans through clusters. If a cluster crosses a paint boundary, Core uses the first logical scalar's paint; legacy policy needs explicit fixtures. Bidi segmentation precedes shaping; reorder visual runs after line breaks, never reverse UTF-8 bytes or glyph arrays blindly.

Initial layout supports hard breaks and WrapStyle 2 (soft `\n` becomes a hard break here; otherwise a space). `\h` is nonbreaking space. Alignment 1..9 uses bottom/middle/top rows and left/center/right columns. Local layout stores baseline positions, logical advance box, ink box and collision box separately; whitespace has advances but no ink. For line height use max ascent+descent. Under the owner-directed T018 compatibility correction, a line containing only its terminating mandatory break uses half of each style/font metric in integer eighth-units (truncate after division by2), as pinterf GetNextLine1453–1472. Whitespace words retain full metrics. MakeLines1548–1565 creates no synthetic empty line after a terminal break; retain original decoded/source ranges. Core automatic wrapping follows the engine selection below; standard historical wrapping follows pinterf; `\\warpmode1` remains the separate Unicode engine. Wrapping may reshape line-boundary context: cache line shapes separately keyed by the break range and context; do not assume paragraph shaping can always be sliced.

For explicit `\warpmode1`, CoreV1 automatic WrapStyle0/1/3 follows owner-approved [ADR-013](decisions/013-core-unicode-wrapping.md) and [W2](research/T017-wrapping-policy-v2.md)'s pinned Unicode15.1 opportunities, Core tailoring, grapheme/shaping-cluster filters, overwide atoms, inherited margin width and fresh boundary shaping with original provenance. The line profile is Unicode15.1 UAX14 revision51 section8.2 Example7's complete Regex-Number behavior with its required LB13 adjustments, not the default LB25 approximation; grapheme boundaries use UAX29 revision43. Mode1 examines all legal ends without assuming monotonic widths. Modes0/3 use the normative backwards recursive binary64 DP comparator, including opposite width-vector ties and earliest decoded offsets; discarded suffixes are never reopened after prefix rounding. This is not exact global squared-slack minimization. Preserve explicit empty-line metrics and separate logical/ink/collision boxes; apply the current source-backed collision footprint and complete-active-set placement after chosen lines are fully shaped. This Unicode engine remains UnsupportedFeature until the data/conformance/native-work prerequisites and separately reviewed C7 caller limits are satisfied. Historical automatic wrapping is implemented directly from pinterf; authenticated captures are optional pixel evidence, not an implementation admission gate.

Core collision: eligible non-explicit events only, separate per layer, ordered by `(start, EventId)`. Place at normal anchor; bottom-aligned events move up, top/middle events down by the minimum displacement clearing occupied half-open collision rectangles. A deterministic union of open forbidden-anchor intervals may replace repeated canonical obstacle scans when it yields the same minimum placement. Allow placement beyond viewport then clip. Touching edges do not collide. Explicit `\pos`/`\move` and events with explicit `\org` are neither moved nor obstacles in Core and are structurally excluded before collision ordering/search. Recompute from the active set on every request; never retain historical positions. Return placements indexed by EventId; composition still uses layer/source order, not collision order.

Current collision implementation reserves the local logical/ink union, visible transformed geometric effect support, and logical first/top and last/bottom line borders independently of paint alpha. Border reservation uses existing SBAS/device mapping and nearest eighth-pixel widths; whitespace-only lines retain it and genuinely empty mandatory-break lines reset it. Nonempty lines recompute borders from retained words, excluding trimmed edge spaces and the terminating break style. Local metrics stay paint-independent. Fixed explicit `\org`, `\pos` and `\move` events bypass allocation. See [source findings and remaining differences](compatibility-notes.md#layout-positioning-and-collision). The retired ADR-012 invisible-border exclusion is not an active requirement. This description does not claim complete pinterf allocator or raster parity.

### C4 wrapping-engine selection — owner-directed mode0 implementation

The Taiyou-specific event-level `\warpmode` selects one wrapping engine: absent or `\warpmode0` selects source-derived pinterf xy-style word wrapping; `\warpmode1` explicitly opts into [ADR-013](decisions/013-core-unicode-wrapping.md). The owner requested working mode0, superseding the earlier selector-only proposal. [ADR-014](decisions/014-wrapping-engine-selection.md) records the exact scope and source locations. The integration base is PR29's human-merged `6cfa33ddb38c714a43a096abf53096f9f9aff69c`; preserve unrelated layout/backend safeguards while correcting source-verified historical incompatibilities.

Resolve the last valid standalone parsed selector in event source order to one immutable choice for the entire event. Bare/named `\r`, style/paint changes and breaks do not reset it. Nested children of unsupported operations do not execute. Strict malformed selector parameters return InvalidScript; recovery diagnoses and preserves the preceding selection. Preserve winning and losing original source spans; do not scan raw text for tags.

q remains independent: initialize it from Script Info (default0 when absent) and process overrides in source order using the review-directed decimal-prefix command-suffix behavior. Parentheses do not supply a q value. Empty or out-of-range q restores the script value; a nonempty parameter with no decimal prefix converts to0. Style reset does not reset q. Decode `\n` under the q state at its source position (space for0/1/3, newline for2); `\N` is mandatory and `\h` is NBSP. The final q controls line fitting, without retroactive escape decoding.

Mode0 q1 fits whole consecutive ASCII-space/nonspace groups to PlayResX minus inherited event left/right margins. Use the original event style for margin inheritance; do not subtract borders. q0 uses the remaining hard-line width target; q3 includes pinterf's final-word target adjustment. Accept an overwide first group and trim edge ASCII-space words after fitting while retaining consumed source provenance. Use checked eighth-unit policy widths from the existing HarfBuzz word advances and scale/spacing rules. No Unicode break opportunities or automatic split inside a nonspace word are introduced. Preserve Core empty-line metrics, independent logical/ink/collision boxes and visual run ordering after line decisions.

q2 uses the hard-break path under either engine, with no automatic wrapping. Under the owner-approved T018 compatibility correction, line finalization compacts leading/trailing ordinary ASCII-space words before alignment even for q2 (pinterf CLine::Compact1171–1200, GetNextLine1471/1502). Preserve decoded/source ranges, indivisible shaping clusters, NBSP and drawing words. A whitespace-only line retains full original font/style metrics with zero positioned advance; a genuinely empty mandatory-break line retains the half-height rule above. Compact each hard line independently; this does not change q0/1/3 fitting decisions. Mode1 q0/1/3 remains UnsupportedFeature in both policies until its Unicode, native-work/preflight and separately reviewed C7 gates pass; never silently fall back to mode0. No partial frame is published on failure. Engine choice does not change Profile, font metrics, effects or collision policy. Different backend widths may produce different breaks from GDI. This Core implementation admits no LegacyXyV1 capability and claims neither line-break-exact nor pixel-exact xy compatibility. No research-task admission is required for source-established behavior. No C7 defaults, dependency or T019 cache are added.

## C5. Geometry, stroke, raster and effects

```rust
enum PathOp { MoveTo(Point), LineTo(Point), QuadTo(Point,Point), CubicTo(Point,Point,Point), Close }
struct Outline { ops: Arc<[PathOp]>, bounds: Bounds, fill_rule: FillRule }
struct GeometryInstance { outline: Arc<Outline>, mapping: GeometryMap, phase: SubpixelPhase }
struct CoverageGroup { instances: Arc<[GeometryInstance]>, pass: CoveragePass, policy: CoveragePolicy }
fn rasterize(ctx: &mut RasterContext, group: &CoverageGroup)
    -> Result<Arc<Mask>, RenderError>;
struct Mask { bounds: PixelRect, stride: usize, data: Arc<[u8]>, xy_units: bool } // tagged coverage
fn apply_effects(mask: &Mask, effects: &EffectSpec, scratch: &mut Scratch)
    -> Result<Arc<Mask>, RenderError>;
```

Native font outlines are copied into owned path commands before leaving `FontContext`; no mutable `FT_Face` crosses jobs. Fill is nonzero winding initially. Use FreeType unhinted outline load, no embedded bitmap, no LCD rendering. Production rasterization consumes transformed Q6 outlines using nonzero-winding eighth-grid span coverage, accumulated in native 0..64 bytes. Source-mask bounds retain the reference extra eighth-unit extent and right/bottom pixel. This is scan conversion of the outlines, not quantization of FreeType A8 output. The separate FreeType A8 stage remains available for native integration tests. Exact native load flags, version and raster settings are recorded by T002/T012; outline/raster policy versions enter keys. Negative mask origins are valid; empty masks have zero dimensions and data length 0. For nonempty masks `stride>=width`, length is exactly `stride*height`, padding initialized to zero. Effects never mutate a cached input mask.

Transform order follows pinterf CWord::Transform: word-local fax/fay shear, local scale and placement, then z, x and y rotations about the event origin and perspective division. Native 64-unit path constants 20000/1000 correspond to 312.5/15.625 pixel units; clamp the perspective denominator, not the requested angles. Preserve the existing exact cardinal z mapping when x/y angles and shears are zero. Positive z rotation in y-down coordinates uses `(x',y')=(cos(a)*x+sin(a)*y,-sin(a)*x+cos(a)*y)`. Default origin is the alignment anchor; explicit org is script-space. Map into the content rectangle independently by each viewport/PlayRes ratio. Rotation does not change line breaks. The FreeType outline/Q6 backend still has documented GDI/xy raster and metric differences; these properties do not assert frozen-pixel identity.

Border expansion is **after** transform in device space, with each radius independently scaled by its content/PlayRes ratio only when ScaledBorderAndShadow is true. Equal radii retain the FreeType round-stroker baseline. Positive unequal radii use elliptical axis normalization before round stroking and restoration before rasterization. A single zero axis uses nonzero-winding eighth-pixel span union in the nonzero direction before coverage accumulation, following the degenerate pinterf widening path; no epsilon or square A8 maximum filter. Zero/zero has no border. Complete xy span/raster equivalence, including self-intersections, remains a measured compatibility gap. Output passes are full fill, outline region and shadow region; transparent fill must not erase an outline hole incorrectly.

Group policy is explicit because overlapping glyphs can darken differently. Core rasterizes combined contours of a maximal adjacent run with equal geometry/effect/paint identity as one nonzero-winding shape, yielding fill F and expanded E; outline coverage is saturating `E-F`. Shadow derives from E if border exists, otherwise F. Under the owner-directed T018 compatibility correction, omit the shadow pass when both evaluated ASS depths are zero, before Q6 quantization, regardless of paint opacity (pinterf RTS.cpp1243–1256). A nonzero depth quantized to zero retains its visible coincident pass. Paint identity influences grouping but not glyph outline identity. Historical group boundaries and coverage subtraction follow pinterf when implemented. Ordered pass list for Core: all shadow groups in event order, then all border groups, then all fill groups. Never composite each glyph's shadow/border/fill triplet if that changes this order.

Core effects (T018 owner-directed ASS compatibility amendment): translate shadow geometry before raster; filter masks before final clipping. Static \blur and real-valued \be follow the pinterf source path documented with file hashes/functions in [T018 candidate](T018-CANDIDATE.md), replacing the earlier planned integer Core kernel. Gaussian is effective only above double(0.333333f), or when positive BE enables the stage; a positive Gaussian then runs before BE. Per-axis sigma is blur times target scale, width=(ceil(3*sigma)|1), radius=width/2; normalized float coefficients, float intermediate axes and scalar +0.5 output conversion. BE uses float strength times the mean target scale, integer [1,2,1] outer-product passes truncated /16, then the source-defined fractional three-tap pass and actual scalar rounding expression. BE pads by round(target_axis*sqrt(be/4)) and retains the source integer pass's unchanged outer ring, including allocation-edge truncation; do not substitute full N-pixel padding. Target scale is output/original-video or explicit LayoutRes, independent of SBAS/font/PlayRes scale; LayoutRes supersedes host original storage size and supplies the separate layout-to-target ratios. For BorderStyle=1, filter expanded E when border exists and preserve F, otherwise filter F; for BorderStyle=3, filter body and independent box masks; border subtraction follows filtering. Translate and filter shadow E/F independently. Sources remain immutable; allocate the complete source-defined support before filtering, with zero exterior and checked existing radius/pass/memory limits. Excessive effects fail without partial output or silent truncation beyond the explicitly selected historical BE edge behavior. Effects preserve the source mask unit tag. Production xy masks remain in native 0..64 units through Gaussian and BE; normalize only at the paint/clip boundary. The separate A8 stage keeps its existing byte units. This scalar backend does not assert xy SIMD/tail or historical pixel identity.

Rectangular and vector clip slots are independent and intersect. Rectangular clip/iclip assignments are ordered and animatable: truncate each requested coordinate after adding 0.5, scale by the script-to-device ratio, CalcAnimation against the current integer edge and truncate. Half-open integer rectangles are binary clips; inverse complements their coverage. The first valid vector clip/iclip owns the vector slot, immediately even inside t; its optional drawing scale is at least 1 and divides coordinates by 2^(scale-1). It uses the normal polygon parser and A8 nonzero rasterization in its own viewport map, independent of text movement/rotation. Composition multiplies clip coverage after effects, never truncating their input halos. Clips survive style resets. Vector drawing pN uses the same divisor for N>=1; N=0 exits drawing. The owner-authorized T018 subset implements pinterf CPolygon m/n/l/b/s/p/c, uniform cubic B-splines, Q6 coordinate truncation, pbo baseline, and source-ordered drawing/text transitions. Malformed polygon tails are ignored as in that reader; no font glyphs are substituted for drawing text. Owned outlines and metrics retain decoded provenance and use the existing geometry, nonzero fill, border, shadow and filter pipeline.

## C6. Paint, composition and output

```rust
struct Rgba8 { r: u8, g: u8, b: u8, a: u8 } // straight paint
enum Paint { Solid(Rgba8) } // future variants via reviewed contract additions
enum PaintSpace { GlyphLocal, EventLocal, Device }
struct DrawOp {
    order: DrawOrder, mask: Arc<Mask>, offset: PixelOffset,
    paint: Paint, paint_space: PaintSpace, paint_map: PaintMap,
    clip: Arc<ResolvedClip>,
}
struct DrawOrder { layer: i32, event: EventId, pass: u16, group: u32 }
struct Frame { size: Size, stride: usize, pixels: Arc<[u8]>, diagnostics: Arc<[Diagnostic]> }
```

RGBA8 means memory bytes R,G,B,A, independent of endian. Output is **premultiplied**, top-down, `stride=width*4`, transparent black initial pixels. Mask storage explicitly distinguishes A8 coverage (0..255) from native xy coverage (0..64); zero is transparent in both. Production source masks and their filtered descendants retain native xy units. Colors are blended in their encoded RGB domain for legacy display behavior; no implicit linear-light conversion, YUV conversion, premultiplication guessing, or video-background blending. Host performs video-dependent YCbCr conversion. The script matrix is retained as metadata and does not reject or alter raw RGB overlays; the header alone cannot determine a video-space transform. Fully transparent output has RGB zero.

Define `mul255(x,y)=(x*y+127)/255` with integer division. For native xy coverage, first `sa=(mask*paint.a+32)>>6`, then `sa=mul255(sa,clip)` (pinterf FillAlphaMashBody). Resolved clip masks normalize native coverage with `(mask*255+32)>>6` before their existing clip combination. For separate A8 coverage, first `coverage=mul255(mask,clip)`; then `sa=mul255(paint.a,coverage)`; then premultiplied `sr=mul255(paint.r,sa)` and similarly g/b. Source-over each channel: `out=src+mul255(dst,255-sa)` including alpha. Use sufficient intermediates and clamp final channel to 255. Do not combine these rounded multiplications into one expression. Example: 50%-opacity red (a=128), full coverage over opaque blue gives `(128,0,127,255)`; over clear gives `(128,0,0,128)`. Composition key order is lexicographic; equal keys are an internal error. Do not gamma-correct alpha.

PNG export in CLI converts premultiplied RGB to straight with `a==0 ? 0 : min(255,(c*255+a/2)/a)`, then writes a documented encoded RGB PNG. Internal byte comparisons use raw premultiplied output, not lossy round trips through PNG. Frame changed detection compares exact complete draw-list identities (including position, paint, clip, order and viewport) or bytes; never infer unchanged from active event IDs alone.

## C7. Renderer, errors and limits

```rust
struct RendererConfig { workers: NonZeroUsize, profile: Profile, limits: Limits, cache: CacheConfig }
struct RenderRequest { now: TimeUs, viewport: Viewport, unsupported: UnsupportedPolicy }
fn render(&mut self, track: Arc<Track>, fonts: Arc<FontCatalog>, request: &RenderRequest)
    -> Result<Frame, RenderError>;
enum ErrorCode { InvalidArgument, InvalidEncoding, InvalidScript, UnsupportedFeature,
    FontUnavailable, NativeFailure, LimitExceeded, AllocationFailed, InternalError }
struct Diagnostic { code: DiagnosticCode, event: Option<EventId>, span: Option<SourceSpan>, detail: Arc<str> }
```

Diagnostics sort by `(event ordinal or header first, span start, code)` at publication, not arrival time. Cap at 1,024 entries plus a final suppressed-count record. Native return codes are preserved in debug detail. No callbacks execute from worker threads in v1. The render API accepts negative time (usually an empty frame), rejects zero viewport size, and returns full clear frames when nothing is active.

Initial configurable policy defaults (not performance promises): 16 MiB subtitle input, 100,000 events, 1 MiB text per event, 100,000 glyphs per event, 1,000,000 path segments per frame, 8 tag nesting levels, 16,384 maximum dimension, 32 million output pixels, 64 MiB registered font bytes, 128 MiB retained caches, 128 MiB transient work and scratch combined, 128 MiB output allocations per renderer origin including retained frames, 256-pixel filter radius, 64 be passes. Check multiplication/addition before allocation. A frame exceeding a limit fails as a whole. Limits are caller configuration, recorded in reports; tests use tiny limits to prove checks.

Budgets cover Taiyou-owned allocations and explicitly reserved native scratch estimates; they are not a hard process RSS guarantee. Native font library internal allocation needs measurement in R003. Output accounting uses a shared budget lease retained by frames even after renderer destruction; the charge is released when the last pixel Arc drops. Retained cache charges remain until final allocation owner drops, not just index eviction. Under transient pressure schedule fewer jobs or evict unused values; if the request still cannot fit, return LimitExceeded identically at all worker counts. Preflight logical frame reservations in stable order so worker count cannot change success/failure. Scratch concurrency fits inside the same reservation.

## C8. Concurrency and cache boundary

Only immutable Track/Catalog/Outline/Mask/Frame values are Send+Sync. Renderer is Send (move only when idle), not Sync; FontContext/RasterContext/Scratch are neither shared nor migrated during use. A context is created inside its worker; native objects are destroyed there. For the inline path the caller owns the context.

Cache interface: `lookup(&CanonicalKey)->Option<Arc<V>>`, `reserve(unique_requests)->Result<Reservation,...>`, `publish(key,Arc<V>,charge)`, `stats()`. Workers receive resolved input Arcs and a reserved output slot; cache locks never surround rendering, callbacks or native library calls. Coordinator owns miss grouping and publication. Keys and transitive dependency fields are specified in [CACHING.md](CACHING.md). Cache enabled/disabled, eviction, and pool size cannot alter output or diagnostics. Cache hit counts may differ by capacity; expensive construction counts for identical keys within a frame may not exceed one.

The blueprint has no implementation to migrate yet. After T003's interface review, an agent adding a field authorized below must still coordinate the shared-file edit; authorization to implement a stage is not authorization to choose incompatible field types. Public representations not specified here require a small contract addendum before dependent agents branch.

## C9. C ABI contract

Planned `include/taiyou.h`, generated or audited against `taiyou-capi`; no ABI is frozen until T028. All exported names start `taiyou_`; `extern "C"`, opaque renderer/track/catalog/frame pointers, fixed-width integer status/flags and `size_t` lengths. Never expose Rust structs, enums, Vec, bool, references or allocator ownership.

```c
uint32_t taiyou_abi_version(void); /* 1 at first release */
TaiyouStatus taiyou_renderer_create(const TaiyouConfig*, TaiyouRenderer** out);
TaiyouStatus taiyou_track_parse(const uint8_t*, size_t, TaiyouTrack** out);
TaiyouStatus taiyou_catalog_create(const TaiyouFontInput*, size_t, TaiyouCatalog** out);
TaiyouStatus taiyou_render(TaiyouRenderer*, const TaiyouTrack*, const TaiyouCatalog*,
                          const TaiyouRequest*, TaiyouFrame** out);
TaiyouStatus taiyou_frame_view(const TaiyouFrame*, TaiyouFrameView* out);
void taiyou_frame_release(TaiyouFrame*);
void taiyou_track_release(TaiyouTrack*);
void taiyou_catalog_release(TaiyouCatalog*);
void taiyou_renderer_release(TaiyouRenderer*);
```

Config/request/view start with `uint32_t struct_size, abi_version`; accept known prefix, ignore documented trailing fields, reject short prefix or unknown version, reserved bits must be zero. T028 specifies exact field offsets and C tests before stable release. Request carries signed int64 time_us, dimensions/content rectangle, profile fixed in config, and flags. Frame view carries width/height/stride/data length and `const uint8_t*`, format constant `RGBA8_PREMULT_ENCODED=1`. View memory is valid until that frame's release, including across subsequent render calls or renderer destruction. Input bytes/fonts are copied or retained internally before returning; no caller pointer is retained.

All out handles initialize to NULL on failure; null input with nonzero length is InvalidArgument. `release(NULL)` is a no-op; other handles must be live, correct type and released exactly once by the caller. Invalid arbitrary pointers cannot be made safe by a C library. Calls on one renderer and release against use must be externally serialized. Read-only frame views may be read concurrently while the handle stays alive. No caller frees Taiyou allocations. Diagnostic copy API uses caller buffer+capacity and required length; no borrowed thread-local last-error string. Catch unwind at each FFI entry and translate to InternalError; aborting native faults/OOM cannot be caught. Panic paths, 32-bit length overflow and C/C++ compilation are mandatory tests.

## C10. Shared record field catalog

This completes names used in the signatures above. All collections are immutable once published. `FiniteF64` fields use script units unless the field explicitly says otherwise. Maps are sorted arrays or ordered maps when iteration affects behavior. Native/library-specific types never appear here. Only introduce a future field/variant when its owning task starts.

| Type | Required fields and representation |
|---|---|
| `ParseOptions` | `encoding: InputEncoding`, `profile: Profile`, `limits: Limits`, `syntax: SyntaxPolicy` (Recover default or Strict); InputEncoding initially Utf8, explicit legacy variant introduced T026. Strict returns InvalidScript on malformed known syntax; Recover emits diagnostics and applies the specified recovery. Invalid encoding and limit failures are always fatal. Render-time UnsupportedPolicy separately governs unimplemented features. |
| `ScriptInfo` | `play_res: Size`, `layout_res: Option<Size>`, `wrap: WrapMode`, `scaled_border_shadow: bool`, `ycbcr_matrix: Arc<str>`, `collision_header: Option<Arc<str>>`, `metadata: Arc<[(Arc<str>,Arc<str>)]>`; unknown metadata is retained but not executed |
| `Margins` | left/right/vertical u32 script-unit counts; zero event margin inherits style margin, nonzero overrides; style zero means zero |
| `ShapeStyle` | FontRequest, font_size FiniteF64 positive, spacing FiniteF64, language optional BCP47 string, encoding i32, direction Auto/Ltr/Rtl, sorted OpenType feature pairs `(u32 tag,u32 value)`, sorted variation pairs `(u32 tag,FiniteF64)`, metric_policy u32; size and all options included in key |
| `GeometryStyle` | scale_x/scale_y FiniteF64 factors (100% = 1), x/y/z rotations in degrees, fax/fay dimensionless shear, underline/strikeout bool; synthetic weight/slant determined by resolved font record |
| `EffectStyle` | border_style enum Outline/OpaqueBox, border_x/y nonnegative FiniteF64, shadow_x/y signed FiniteF64, blur nonnegative FiniteF64, be signed real FiniteF64, profile policy revision; OpaqueBox is unsupported until its pinterf behavior is implemented and tested |
| `PaintSet` | primary, secondary, outline, shadow straight Rgba8; separate optional per-run karaoke selector; no mask allocation |
| `Placement` | alignment enum values1..9, effective Margins, anchor Default/Explicit(ScriptPoint), origin Option<ScriptPoint>, collision_eligible bool; evaluated Move yields Explicit point; width constraints resolved before anchor |
| `ResolvedRun` | logical text Arc<str>, text-to-source interval map, FontId, ShapeStyle, script tag u32, language option, Direction; context prefix/suffix text with source ranges for shaping boundary correctness |
| `RunMetrics` | ascent/descent nonnegative FiniteF64, line_gap FiniteF64, units_per_em u16; glyph advances in ShapedGlyph are converted to script units, not raw HarfBuzz integer positions |
| `ShapedEvent` | EventId, EventOrder, Arc of shaped runs with their geometry/effect/paint associations, Placement, ClipSpec; does not own a mutable font context |
| `LayoutConstraints` | available_width nonnegative FiniteF64, WrapMode0..3, alignment, profile metric policy, effective scale factors; no absolute pixel anchor |
| `LocalLayout` | Arc of Line records, Arc of PositionedGlyph records, logical_bounds/ink_bounds/collision_bounds as script Bounds; each Line has logical source range, baseline, ascent/descent, advance and visual glyph-index range |
| `PositionedGlyph` | ShapedGlyph reference by run/glyph index, local ScriptPoint, source cluster and paint/effect association indices; line ordering is explicit |
| `CollisionInput` | EventId, EventOrder, event start TimeUs, LocalLayout Arc, natural anchor ScriptPoint, alignment and eligibility; CollisionPolicy is CoreV1 or approved versioned legacy policy |
| `PlacedEvent` | EventId, LocalLayout Arc, final anchor ScriptPoint, origin ScriptPoint, scene order; layout is not mutated |
| `Point`, `Bounds` | FiniteF64 x/y and min/max in explicitly tagged space; font extraction uses FontPoint/FontBounds wrappers, script mapping uses ScriptPoint/ScriptBounds, raster boundary uses DevicePoint/PixelRect; do not expose one untagged Point between stages |
| `GeometryMap` | Core affine 2×3 f64 matrix from script to device plus explicit origin already folded in and policy revision; font-to-script size/y-flip is a separate mapping; later Projective3x3 only after R004; serialized keys include full canonical coefficients |
| `SubpixelPhase`, `PixelOffset` | phase x/y u8 constrained0..63; offset x/y i32 in whole device pixels; transformed contour factorization records both, never rounds offset independently |
| `CoveragePass` | Shadow, Border(index u16), Fill; DrawOrder pass numbering0=shadow, 1=initial border, 2=fill for initial Core; multiple border allocation requires reviewed numbering extension |
| `CoveragePolicy` | profile+policy revision, group membership ordered by glyph instance identity, union/subtraction mode fixed by profile; no arbitrary callback |
| `EffectSpec` | resolved device border/shadow parameters as applicable, Gaussian float coefficients and per-axis radius u32, real BE resolved to integer passes and fractional coefficients, ordered effect kinds and selected pass; derived from EffectStyle before filtering |
| `ResolvedClip` | None, RectCoverage(bounds Q6,inverse), MaskCoverage(Arc<Mask>,PixelOffset,inverse); viewport bounds supplied for inverse sampling; out-of-mask coverage is0 before inversion |
| `PaintMap` | inverse mapping from device sample to declared paint space; solid paint uses Identity and does not require an invertible geometry map; non-solid singular-map behavior must be specified with that future feature |
| `Viewport` | full Size, content PixelRect contained in full frame, sample_aspect numerator/denominator positive u32; initial Core accepts only1/1, R004 adds approved behavior |
| `KaraokeSpan` | logical source span, local start/end TimeUs, mode KStep/KSweep/KOutline; zero duration retained; clusters reference this logical interval |
| `Limits` | named u64 byte/count caps and u32 dimension/radius/pass caps from C7; checked conversion to usize at allocation; nonzero where zero would prohibit any input, zero cache budget explicitly allowed |
| `RenderError`, `ParseError`, `FontError` | wrappers over ErrorCode with optional event/span/native numeric code and owned detail; no string matching for program flow; InternalError is never used to disguise UnsupportedFeature |
| `Scratch` | worker-private reusable vectors with accounted capacity, not a public shared type; reset between jobs; no borrowed slice escapes |

Standard opaque-box borders, vertical font names and LayoutRes are implemented by the owner-directed ASS batch above. Variable-font controls from subtitle tags and legacy codepages remain unsupported. Host variation coordinates may be added by T009 only if its selected font dependencies and tests cover them; otherwise reject explicitly. A field capable of representing a future value is not a declaration that it renders correctly.

T018 oversampling adapter: an explicit original-video size may differ from the output viewport. Device blur target scale is output/original per axis, independent of PlayRes and SBAS. The frozen corpus's 8x1 adapter renders at eight times horizontal resolution then uses libass compare's fixed-reciprocal premultiplied RGBA16 downsampling. Its explicit 512-radius caller budget leaves the production default 256 and all total memory/dimension limits intact. Display PNGs are previews; native raw bytes and RGBA16 comparison values are authoritative.
