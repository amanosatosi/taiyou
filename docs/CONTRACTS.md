# Contracts

Normative v0.1. Rust below is interface notation, not compilable code or existing APIs. T003 establishes the initial subset; later tasks add only fields explicitly authorized here. Types live in `taiyou-types` unless noted. Private fields and checked constructors enforce invariants. `Arc` means shared immutable ownership; borrowed inputs last only for the call. Every stage is fallible where input or allocation can fail.

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

`CoreV1` is a precisely specified development profile, not a VSFilter claim. `LegacyXyV1` is enabled only feature by feature after R001 evidence; unsupported rules cannot silently borrow Core behavior and count as compatible. Profile and semantic revision are part of every affected key.

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

V1 input accepts UTF-8 and UTF-8 BOM, CRLF/LF/CR. Invalid UTF-8 is an error, never guessed using OS locale. T026 adds explicit host-selected legacy decoding and records encoding provenance. Section/field names are ASCII case-insensitive; preserve text and font names. Parse `Format:` dynamically with the text field last for the Core subset; reject unsupported text-not-last records with diagnostic rather than splitting text commas incorrectly. Trim structural whitespace only. Unknown sections are ignored with bounded diagnostics. The owner-directed T008 compatibility follow-up admits ASS `[Fonts]` / `fontname:` attachments: offset-33 six-bit encoding, four characters per three bytes, two/three-character final groups yielding one/two bytes. Empty, one-character-tail or invalid-character payloads are malformed records (Strict rejects; Recover discards the attachment with a diagnostic); resource failures remain fatal. Blank lines, the next fontname, a standard ASS section or EOF finalize a payload. Names are metadata only; no attachment filesystem writes. ParsedTrack retains bounded decoded bytes and original spans; render_parsed registers them after host faces through the normal validated catalog, with aggregate font-byte limits. Native-invalid fonts cannot bypass validation. Unsupported SSA V4 styles return UnsupportedFeature before font resolution; their mapping remains T026 work. Other binary attachments remain unsupported. No implicit filesystem font paths.

Core style defaults: family `Arial` requested through the catalog, size 20 script units, bold/italic/underline/strikeout false, scale x/y 100%, spacing 0, angle 0, BorderStyle 1, border 2, shadow 2, alignment 2, margins 10, Encoding 1; opaque white primary, opaque yellow secondary, opaque black outline/shadow. Missing `Default` synthesizes this style with a diagnostic; unknown event style resolves to Default with diagnostic. Duplicate names: last definition wins, resolved after all sections. Preserve all definitions for diagnostics. Core default PlayRes is 384×288 only if both absent; if exactly one absent return `UnsupportedFeature` until R001 supplies a legacy rule. Nonpositive specified resolution is invalid. Core default WrapStyle 0; Core default ScaledBorderAndShadow false. Legacy defaults are separately frozen by R001, not inferred from these choices.

`Style` contains named fields matching the defaults plus name, font request, four straight RGBA colors, alignment 1..9, margins and encoding. Source ASS `&HAABBGGRR` maps to `r=RR,g=GG,b=BB,opacity=255-AA`. Header fields retain PlayRes, LayoutRes if present, WrapStyle, ScaledBorderAndShadow, YCbCr Matrix, collisions and unknown metadata; unsupported fields remain reportable. Style values are script-space, never already device-scaled.

Core numeric tokens accept optional sign and decimal digits with an optional fractional part, no exponent/locale separator/NaN/Inf. Integer fields reject fractions. Color style fields accept `&H` plus 1–8 hex digits and optional trailing `&`, zero-extended; decimal style colors are deferred to R001/T026. RGB override colors accept 1–6 hex digits and preserve current opacity; alpha overrides accept 1–2 hex digits and invert transparency to opacity. Invalid/out-of-range supported-tag parameters diagnose and leave the prior state unchanged; strict mode returns InvalidScript. Parameterless `\r` is valid; other empty Core parameters are invalid until a legacy reset rule is enabled. Core `\b0` selects weight400, `\b1`/`\b-1` weight700, `\i0` false and `\i1`/`\i-1` true; other weights/forms await R001. Font size must be positive; scales/border/blur nonnegative; signed spacing/shadow/position allowed. These restrictions intentionally keep the initial supported grammar small and explicit.

`OverrideOp` is a closed enum introduced by supported tasks: SetFont, SetSize, SetWeight, SetItalic, SetScale, SetSpacing, SetPaint(channel), SetOpacity(channel/all), SetBorder, SetShadow, SetBlur, SetPosition, Move, SetAlignment, SetOrigin, SetRotation, SetClip, ResetStyle, Transform, Fade, Karaoke and DrawingMode. Store typed parsed parameters and original order, not precomputed frame values. Unknown operations retain spans, not executable strings. Parser recovery must advance input; cap token count and nesting. R001 owns malformed/nested tag tokenization for Legacy; do not recursively parse arbitrary nesting as a guessed compatibility fix.

Core reset disambiguation follows owner-accepted [ADR-009](decisions/009-core-reset-token-disambiguation.md): bare `\r` is ResetStyle(None). After structural whitespace trimming, `\rName` is a named reset only when Name exactly matches a final declared style name (case-sensitive; forward definitions, last-definition-wins and synthesized Default apply). Other r-prefixed tokens are Unsupported with original spans and diagnostics, never implicit Default resets. Thus `\rnd` resets style nd if that style exists, otherwise it remains unsupported. Future extension admission must review collisions and profile policy; this is not a Legacy rule.

## C3. Evaluation and dependency classification

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

Evaluate overrides left to right; text captures current state. `\r` resets style state, named `\rName` uses its exact final resolved style under ADR-009, and does not reset the event clock. Unresolved r-prefixed tokens stay Unsupported, so they do not reset style state. Core repeated event placement uses last applicable operation; Legacy precedence is R001-gated. Unsupported operations generate diagnostics; a strict request fails with `UnsupportedFeature`, tolerant mode proceeds with a conspicuous report. Reference validation always records diagnostics and disallows unsupported behavior in a supported fixture.

Evaluation retains one complete source map and checked decoded run ranges when every text operation has exact provenance. Otherwise ordinary evaluation remains available with no complete map, and exact shaping reports the missing provenance. The existing source field remains diagnostic provenance; it never becomes a fabricated escape/cluster span. Paint boundaries preserve decoded positions without forcing a shape split.

`ChangeClass` is an optimization hint with independent shape/layout/geometry/effect/paint/placement/clip flags; actual cache reuse compares full keys. It is never permission to omit a key field. Static results can be retained across all timestamps in their active interval. Animated values are evaluated each request, but equal normalized values reuse artifacts. Do not key every stage by time.

Core `\t(t1,t2,accel,...)` uses `p=clamp((local-t1)/(t2-t1),0,1)^accel`; accel must be finite positive; omitted interval is event duration and omitted accel is 1. For `t2<=t1`, Core switches at t2. Capture the current property value when encountering that transform during the ordered evaluation, interpolate to its target, and then continue with later operations. Only enumerated animatable numeric properties are interpolated; integer channels round once to nearest with ties away, then clamp. R001 supplies legacy differences and tests before enabling them.

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

HarfBuzz consumes immutable font bytes through OpenType font functions, integer design-unit scale and per-call buffer. Do not mix FreeType advances with HarfBuzz advances. Core maps one em to `font_size` script units; scale factors follow shaping and are accounted in advances. Core kerning on, standard ligatures on unless nonzero spacing; Legacy metrics/features are R001-gated. Keep glyph positions and cluster maps through all later stages. Paint boundaries do not split shape runs in Core; map paint spans through clusters. If a cluster crosses a paint boundary, Core uses the first logical scalar's paint; legacy policy needs explicit fixtures. Bidi segmentation precedes shaping; reorder visual runs after line breaks, never reverse UTF-8 bytes or glyph arrays blindly.

Initial layout supports hard breaks and WrapStyle 2 (soft `\n` becomes a hard break here; otherwise a space). `\h` is nonbreaking space. Alignment 1..9 uses bottom/middle/top rows and left/center/right columns. Local layout stores baseline positions, logical advance box, ink box and collision box separately; whitespace has advances but no ink. For Core line height use max ascent+descent, including an empty line's style metrics. Core automatic wrapping follows the engine selection below; historical Legacy wrapping remains R001-gated. Wrapping may reshape line-boundary context: cache line shapes separately keyed by the break range and context; do not assume paragraph shaping can always be sliced.

For explicit `\warpmode1`, CoreV1 automatic WrapStyle0/1/3 follows owner-approved [ADR-013](decisions/013-core-unicode-wrapping.md) and [W2](research/T017-wrapping-policy-v2.md)'s pinned Unicode15.1 opportunities, Core tailoring, grapheme/shaping-cluster filters, overwide atoms, inherited margin width and fresh boundary shaping with original provenance. The line profile is Unicode15.1 UAX14 revision51 section8.2 Example7's complete Regex-Number behavior with its required LB13 adjustments, not the default LB25 approximation; grapheme boundaries use UAX29 revision43. Mode1 examines all legal ends without assuming monotonic widths. Modes0/3 use the normative backwards recursive binary64 DP comparator, including opposite width-vector ties and earliest decoded offsets; discarded suffixes are never reopened after prefix rounding. This is not exact global squared-slack minimization. Preserve explicit empty-line metrics and separate logical/ink/collision boxes; apply ADR-012 footprints and complete-active-set placement after chosen lines are fully shaped. This Unicode engine remains UnsupportedFeature until the data/conformance/native-work prerequisites and separately reviewed C7 caller limits are satisfied. Legacy automatic wrapping still requires authenticated R001 evidence and separate admission.

Core collision: eligible non-explicit events only, separate per layer, ordered by `(start, EventId)`. Place at normal anchor; bottom-aligned events move up, top/middle events down by the minimum displacement clearing occupied half-open collision rectangles. A deterministic union of open forbidden-anchor intervals may replace repeated canonical obstacle scans when it yields the same minimum placement. Allow placement beyond viewport then clip. Touching edges do not collide. Explicit `\pos`/`\move` are neither moved nor obstacles in Core and are structurally excluded before collision ordering/search. Recompute from the active set on every request; never retain historical positions. Return placements indexed by EventId; composition still uses layer/source order, not collision order.

Under owner-approved [ADR-012](decisions/012-core-collision-footprint.md), Core policy `core-collision-geometric-2` reserves the existing local logical/ink union plus visible transformed glyph-ink effect hulls: fill ink, the device-space Q6 border-expanded hull for visible outline, and the Q6-offset shadow source hull for visible shadow. Transform ink rectangles before border expansion and shadow translation; convert device hulls back to script space. Use actual C5 SBAS scaling/unsupported cases, per-run effects and opacity, checked finite arithmetic and no extra box rounding or antialias padding. Invisible effects add no padding; the base layout reservation remains. This guarantees geometric footprint separation, not disjoint antialiased boundary pixels. Default rotation origin follows the displaced anchor; future fixed explicit origins require a separate placement decision and are unsupported for this translation-only footprint. Local layout remains paint-independent. Legacy behavior, including historical border inflation, remains R001/R002-gated.

### C4 wrapping-engine selection — owner-directed mode0 implementation

The Taiyou-specific event-level `\warpmode` selects one wrapping engine: absent or `\warpmode0` selects source-derived pinterf xy-style word wrapping; `\warpmode1` explicitly opts into [ADR-013](decisions/013-core-unicode-wrapping.md). The owner requested working mode0, superseding the earlier selector-only proposal. [ADR-014](decisions/014-wrapping-engine-selection.md) records the exact scope and source locations. The integration base is PR29's human-merged `6cfa33ddb38c714a43a096abf53096f9f9aff69c`; preserve its approved F1 and collision/backend policy.

Resolve the last valid standalone parsed selector in event source order to one immutable choice for the entire event. Bare/named `\r`, style/paint changes and breaks do not reset it. Nested children of unsupported operations do not execute. Strict malformed selector parameters return InvalidScript; recovery diagnoses and preserves the preceding selection. Preserve winning and losing original source spans; do not scan raw text for tags.

q remains independent: initialize it from Script Info (default0 when absent) and process overrides in source order using the review-directed decimal-prefix command-suffix behavior. Parentheses do not supply a q value. Empty or out-of-range q restores the script value; a nonempty parameter with no decimal prefix converts to0. Style reset does not reset q. Decode `\n` under the q state at its source position (space for0/1/3, newline for2); `\N` is mandatory and `\h` is NBSP. The final q controls line fitting, without retroactive escape decoding.

Mode0 q1 fits whole consecutive ASCII-space/nonspace groups to PlayResX minus inherited event left/right margins. Use the original event style for margin inheritance; do not subtract borders. q0 uses the remaining hard-line width target; q3 includes pinterf's final-word target adjustment. Accept an overwide first group and trim edge ASCII-space words after fitting while retaining consumed source provenance. Use checked eighth-unit policy widths from the existing HarfBuzz word advances and scale/spacing rules. No Unicode break opportunities or automatic split inside a nonspace word are introduced. Preserve Core empty-line metrics, independent logical/ink/collision boxes and visual run ordering after line decisions.

q2 always uses the existing hard-break path under either engine. Mode1 q0/1/3 remains UnsupportedFeature in both policies until its Unicode, native-work/preflight and separately reviewed C7 gates pass; never silently fall back to mode0. No partial frame is published on failure. Engine choice does not change Profile, font metrics, effects or collision policy. Different backend widths may produce different breaks from GDI. This Core implementation admits no LegacyXyV1 capability and claims neither line-break-exact nor pixel-exact xy compatibility. R001 authenticated evidence requirements remain unchanged. No C7 defaults, dependency or T019 cache are added.

## C5. Geometry, stroke, raster and effects

```rust
enum PathOp { MoveTo(Point), LineTo(Point), QuadTo(Point,Point), CubicTo(Point,Point,Point), Close }
struct Outline { ops: Arc<[PathOp]>, bounds: Bounds, fill_rule: FillRule }
struct GeometryInstance { outline: Arc<Outline>, mapping: GeometryMap, phase: SubpixelPhase }
struct CoverageGroup { instances: Arc<[GeometryInstance]>, pass: CoveragePass, policy: CoveragePolicy }
fn rasterize(ctx: &mut RasterContext, group: &CoverageGroup)
    -> Result<Arc<Mask>, RenderError>;
struct Mask { bounds: PixelRect, stride: usize, data: Arc<[u8]> } // A8
fn apply_effects(mask: &Mask, effects: &EffectSpec, scratch: &mut Scratch)
    -> Result<Arc<Mask>, RenderError>;
```

Native font outlines are copied into owned path commands before leaving `FontContext`; no mutable `FT_Face` crosses jobs. Fill is nonzero winding initially. Use FreeType unhinted outline load, no embedded bitmap, no LCD rendering. Rasterization consumes transformed Q6 outlines using grayscale antialiasing. Exact native load flags, version and raster settings are recorded by T002/T012; outline/raster policy versions enter keys. Negative mask origins are valid; empty masks have zero dimensions and data length 0. For nonempty masks `stride>=width`, length is exactly `stride*height`, padding initialized to zero. Effects never mutate a cached input mask.

Core transform order: map font coordinates to script (y flip), apply font scale/spacing in layout, rotate around event origin, then map script into content rectangle. Positive ASS z rotation is counterclockwise visually: in y-down coordinates use `(x',y')=(cos(a)*x+sin(a)*y,-sin(a)*x+cos(a)*y)` about origin, with degrees converted to radians for trigonometry. Default origin is alignment anchor; explicit `\org` is in script space. Core mapping uses content width/PlayResX and height/PlayResY independently; square pixels only until R004. Legacy LayoutRes, pixel aspect and border scaling are explicit R004 inputs. Rotation does not change line breaks.

Core border expansion is **after** transform in device space, with radius scaled by content/PlayRes only when ScaledBorderAndShadow is true. Isotropic round joins/caps use the FreeType stroker baseline; anisotropic widths, zero axis, self-intersections and VSFilter widening are R002-gated. Never replace stroke with a square max filter without evidence. Output passes are full fill, outline region and shadow region; transparent fill must not erase an outline hole incorrectly.

Group policy is explicit because overlapping glyphs can darken differently. Core rasterizes combined contours of a maximal adjacent run with equal geometry/effect/paint identity as one nonzero-winding shape, yielding fill F and expanded E; outline coverage is saturating `E-F`. Shadow derives from E if border exists, otherwise F. Paint identity influences grouping but not glyph outline identity. Legacy group boundaries and coverage subtraction are R002-gated. Ordered pass list for Core: all shadow groups in event order, then all border groups, then all fill groups. Never composite each glyph's shadow/border/fill triplet if that changes this order.

Core effects: translate shadow geometry before raster (fractional offsets affect phase); blur independently selected pass masks; clip after blur. `\be n` is n repeated separable [1,2,1]/4 passes, rounded `(sum+2)/4` after each axis, zero outside bounds. Gaussian Core sigma is the positive `\blur` value in device pixels (script scaling when SBAS true); support radius `ceil(3*sigma)`. Compute symmetric coefficients normalized to sum 65536, round nearest, adjust center to exact sum, i64 accumulation, `(sum+32768)>>16` per axis. `sigma=0` bypasses; when both exist apply Gaussian then be. R002 must confirm legacy parameter mapping, kernel quantization, border/fill selection and pass order; Core's kernel is not presented as xy-equivalent. Expand allocations by exact support before filtering. Oversized sigma/support returns a limit error, never silently truncates.

Clips are `None | Rect(script bounds,inverse) | Path(outline,scale,inverse)`. They use their own script-to-device map, not text rotation or movement. Rect edges in Core quantize to Q6 and use pixel area coverage; path clips rasterize to A8, inverse uses `255-coverage` inside the viewport. Inverse outside the frame needs no allocation. Effective coverage is multiply-and-round with clip at composition. Vector drawing `\pN` uses coordinate divisor `2^(N-1)` for N>=1; N=0 exits drawing. Path grammar and spline lowering are T021/R001; unknown commands diagnose, never execute code.

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

RGBA8 means memory bytes R,G,B,A, independent of endian. Output is **premultiplied**, top-down, `stride=width*4`, transparent black initial pixels. A8 masks are linear geometric coverage, 0 transparent and 255 full coverage. Colors are blended in their encoded RGB domain for legacy display behavior; no implicit linear-light conversion, YUV conversion, premultiplication guessing, or video-background blending. Host performs YCbCr conversion; the unsupported legacy color-matrix emulation remains visible in compatibility reports. Fully transparent output has RGB zero.

Define `mul255(x,y)=(x*y+127)/255` with integer division. First `coverage=mul255(mask,clip)`; then `sa=mul255(paint.a,coverage)`; then premultiplied `sr=mul255(paint.r,sa)` and similarly g/b. Source-over each channel: `out=src+mul255(dst,255-sa)` including alpha. Use sufficient intermediates and clamp final channel to 255. Do not combine these rounded multiplications into one expression. Example: 50%-opacity red (a=128), full coverage over opaque blue gives `(128,0,127,255)`; over clear gives `(128,0,0,128)`. Composition key order is lexicographic; equal keys are an internal error. Do not gamma-correct alpha.

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
| `GeometryStyle` | scale_x/scale_y FiniteF64 factors (100% = 1), rotation_z FiniteF64 degrees, underline/strikeout bool; synthetic weight/slant determined by resolved font record; R004 adds x/y rotation and shear with approved units |
| `EffectStyle` | border_style enum Outline/OpaqueBox, border_x/y nonnegative FiniteF64, shadow_x/y signed FiniteF64, blur nonnegative FiniteF64, be u32, profile policy revision; OpaqueBox is unsupported until R001/R002 specify it |
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
| `EffectSpec` | resolved device border/shadow parameters as applicable, Gaussian coefficient Arc<[u32]> and radius u32, be passes u32, ordered effect kinds and selected pass; derived from EffectStyle before filtering |
| `ResolvedClip` | None, RectCoverage(bounds Q6,inverse), MaskCoverage(Arc<Mask>,PixelOffset,inverse); viewport bounds supplied for inverse sampling; out-of-mask coverage is0 before inversion |
| `PaintMap` | inverse mapping from device sample to declared paint space; solid paint uses Identity and does not require an invertible geometry map; non-solid singular-map behavior must be specified with that future feature |
| `Viewport` | full Size, content PixelRect contained in full frame, sample_aspect numerator/denominator positive u32; initial Core accepts only1/1, R004 adds approved behavior |
| `KaraokeSpan` | logical source span, local start/end TimeUs, mode KStep/KSweep/KOutline; zero duration retained; clusters reference this logical interval |
| `Limits` | named u64 byte/count caps and u32 dimension/radius/pass caps from C7; checked conversion to usize at allocation; nonzero where zero would prohibit any input, zero cache budget explicitly allowed |
| `RenderError`, `ParseError`, `FontError` | wrappers over ErrorCode with optional event/span/native numeric code and owned detail; no string matching for program flow; InternalError is never used to disguise UnsupportedFeature |
| `Scratch` | worker-private reusable vectors with accounted capacity, not a public shared type; reset between jobs; no borrowed slice escapes |

Initial Core opacity-box borders, non-square pixels, variable-font controls from subtitle tags, vertical font names and legacy codepages are unsupported until their named gate. Host variation coordinates may be added by T009 only if its selected font dependencies and tests cover them; otherwise reject explicitly. A field capable of representing a future value is not a declaration that it renders correctly.
