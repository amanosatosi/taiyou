//! C4 logical-order Unicode itemization, context-preserving fallback and shaping.
//! Visual line reordering, spacing placement and all layout remain later stages.
use crate::{FontContext, ValidatedFontCatalog, fold_family};
use std::{ops::Range, sync::Arc};
use taiyou_types::*;
use unicode_bidi::{BidiClass, BidiInfo, Level};
use unicode_script::{Script, UnicodeScript};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapeOptions {
    direction: Direction,
    language: Arc<str>,
    layout_scale: [FiniteF64; 2],
    kerning: bool,
}
impl Default for ShapeOptions {
    fn default() -> Self {
        Self {
            direction: Direction::Auto,
            language: Arc::from("und"),
            layout_scale: [FiniteF64::ONE; 2],
            kerning: true,
        }
    }
}
impl ShapeOptions {
    /// Pair kerning for this shaping request. The ASS coordinator resolves its
    /// script header separately; direct shaping retains its existing default.
    pub fn with_kerning(mut self, enabled: bool) -> Self {
        self.kerning = enabled;
        self
    }
    /// Layout pixels per PlayRes unit; glyph metrics are returned in script units.
    pub fn with_layout_scale(mut self, scale: [f64; 2]) -> Result<Self, FontError> {
        if scale.iter().any(|s| !s.is_finite() || *s <= 0.0) {
            return Err(error(ErrorCode::InvalidArgument, "invalid layout scale"));
        }
        self.layout_scale = [
            FiniteF64::new(scale[0]).map_err(|_| limited())?,
            FiniteF64::new(scale[1]).map_err(|_| limited())?,
        ];
        Ok(self)
    }
    pub fn with_direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }
    pub fn with_language(mut self, language: &str) -> Result<Self, FontError> {
        if language.is_empty()
            || language.len() > 63
            || !language
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return Err(error(
                ErrorCode::InvalidArgument,
                "language must be a bounded ASCII language tag",
            ));
        }
        self.language = Arc::from(language.to_ascii_lowercase());
        Ok(self)
    }
    pub fn language(&self) -> &str {
        &self.language
    }
    pub const fn direction(&self) -> Direction {
        self.direction
    }
}
pub(crate) struct NativeRequest<'a> {
    pub text: &'a str,
    pub context: Range<usize>,
    pub range: Range<usize>,
    pub rtl: bool,
    pub script: u32,
    pub language: &'a str,
    pub ligatures: bool,
    pub kerning: bool,
    pub vertical: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RawGlyph {
    pub vertical_offset: Option<[i32; 2]>,
    pub ink: Option<[i32; 4]>,
    pub glyph: u32,
    pub cluster: u32,
    pub advance_x: i32,
    pub advance_y: i32,
    pub offset_x: i32,
    pub offset_y: i32,
}
pub(crate) struct RawRun {
    pub glyphs: Vec<RawGlyph>,
    pub upem: u32,
}
#[derive(Clone)]
struct Item {
    range: Range<usize>,
    context: Range<usize>,
    level: u8,
    script: u32,
}
/// One logical bidi/script/style item with an owned checked map and catalog.
/// Native handles are created only during shape(), never stored here.
#[derive(Clone)]
pub struct ResolvedRun {
    catalog: ValidatedFontCatalog,
    primary: FontId,
    decoded: DecodedSpan,
    item: Item,
    style: ShapeStyle,
    options: ShapeOptions,
    limits: Limits,
}
impl ResolvedRun {
    /// Deterministically requested face before whole-cluster fallback.
    pub const fn font(&self) -> FontId {
        self.primary
    }
    pub fn decoded(&self) -> &DecodedSpan {
        &self.decoded
    }
    pub fn style(&self) -> &ShapeStyle {
        &self.style
    }
    pub const fn script(&self) -> u32 {
        self.item.script
    }
    pub const fn direction(&self) -> Direction {
        if self.item.level % 2 == 0 {
            Direction::Ltr
        } else {
            Direction::Rtl
        }
    }
    pub fn language(&self) -> &str {
        self.options.language()
    }
    pub fn context(&self) -> Result<DecodedSpan, FontError> {
        self.decoded
            .source_map()
            .decoded_span(self.item.context.start as u32..self.item.context.end as u32)
    }
}
fn error(code: ErrorCode, detail: &str) -> FontError {
    FontError::new(code, detail)
}
fn limited() -> FontError {
    error(
        ErrorCode::LimitExceeded,
        "shaping byte/glyph reservation exceeded",
    )
}
fn reserve<T>(v: &mut Vec<T>, count: usize, limits: &Limits) -> Result<(), FontError> {
    let cost = v
        .len()
        .checked_add(count)
        .and_then(|n| n.checked_mul(std::mem::size_of::<T>()))
        .and_then(|n| n.checked_mul(3))
        .and_then(|n| n.checked_add(1024))
        .ok_or_else(limited)?;
    if cost as u64 > limits.transient_bytes() {
        return Err(limited().with_resource_failure(ResourceFailure::TransientMemory));
    }
    v.try_reserve_exact(count)
        .map_err(|_| error(ErrorCode::AllocationFailed, "shaping allocation failed"))
}
fn preflight(text: &str, limits: &Limits) -> Result<(), FontError> {
    if text.len() as u64 > limits.text_bytes_per_event() || text.len() > u32::MAX as usize {
        return Err(limited());
    }
    let cost = (text.len() as u64)
        .checked_mul(1024)
        .and_then(|n| n.checked_add(4096))
        .ok_or_else(limited)?;
    if cost > limits.transient_bytes() {
        return Err(limited().with_resource_failure(ResourceFailure::TransientMemory));
    }
    Ok(())
}
fn specific(script: Script) -> bool {
    !matches!(script, Script::Common | Script::Inherited | Script::Unknown)
}
fn itemize(
    text: &str,
    options: &ShapeOptions,
    limits: &Limits,
) -> Result<(Vec<Item>, Vec<Range<u32>>), FontError> {
    preflight(text, limits)?;
    let base = match options.direction {
        Direction::Auto => None,
        Direction::Ltr => Some(Level::ltr()),
        Direction::Rtl => Some(Level::rtl()),
    };
    let bidi = BidiInfo::new(text, base);
    let mut items: Vec<Item> = Vec::new();
    let mut breaks = Vec::new();
    for paragraph in &bidi.paragraphs {
        if paragraph.range.is_empty() {
            continue;
        }
        // Hard lines are paragraphs in the initial WrapStyle-2 stage. Resolve L1
        // trailing whitespace here; L2 visual run ordering remains in layout.
        // Keep the L1 working vector paragraph-sized: reordered_levels clones
        // its analysis vector, so a whole-event call per line would be quadratic.
        let paragraph_text = &text[paragraph.range.clone()];
        let local = BidiInfo::new(paragraph_text, Some(paragraph.level));
        let local_paragraph = local
            .paragraphs
            .first()
            .ok_or_else(|| error(ErrorCode::InvalidArgument, "missing paragraph analysis"))?;
        let line_levels = local.reordered_levels(local_paragraph, 0..paragraph_text.len());
        let mut end = paragraph.range.end;
        while end > paragraph.range.start && bidi.original_classes[end - 1] == BidiClass::B {
            end = text[..end]
                .char_indices()
                .next_back()
                .map_or(end, |(i, _)| i);
        }
        if end < paragraph.range.end {
            reserve(&mut breaks, 1, limits)?;
            breaks.push(end as u32..paragraph.range.end as u32);
        }
        let context = paragraph.range.start..end;
        let mut chars = Vec::new();
        reserve(&mut chars, text[context.clone()].chars().count(), limits)?;
        for (offset, ch) in text[context.clone()].char_indices() {
            chars.push((
                context.start + offset,
                ch,
                ch.script(),
                crate::native::is_mark(ch)?,
            ));
        }
        // Resolve Common/Inherited with Script_Extensions and neighboring strong
        // scripts. Marks inherit their base's script; no byte reversal occurs.
        let mut previous = None;
        let mut following = Vec::new();
        reserve(&mut following, chars.len(), limits)?;
        following.resize(chars.len(), None);
        let mut next = None;
        for i in (0..chars.len()).rev() {
            following[i] = next;
            if specific(chars[i].2) && !chars[i].3 {
                next = Some(chars[i].2);
            }
        }
        for i in 0..chars.len() {
            let (offset, ch, intrinsic, mark) = chars[i];
            let after_control = i == 0
                || chars[i - 1].1.is_control()
                || matches!(
                    bidi.original_classes[chars[i - 1].0],
                    BidiClass::LRE
                        | BidiClass::RLE
                        | BidiClass::LRO
                        | BidiClass::RLO
                        | BidiClass::PDF
                        | BidiClass::LRI
                        | BidiClass::RLI
                        | BidiClass::FSI
                        | BidiClass::PDI
                );
            let attached = mark
                && !after_control
                && items
                    .last()
                    .is_some_and(|last| last.context == context && last.range.end == offset);
            let script = if specific(intrinsic) {
                intrinsic
            } else if intrinsic == Script::Inherited {
                previous.or(following[i]).unwrap_or(intrinsic)
            } else {
                let extension = ch.script_extension();
                previous
                    .filter(|s| extension.contains_script(*s))
                    .or_else(|| following[i].filter(|s| extension.contains_script(*s)))
                    .or_else(|| extension.iter().find(|s| specific(*s)))
                    .unwrap_or(intrinsic)
            };
            if specific(script) && !attached {
                previous = Some(script);
            }
            let (level, tag) = if let Some(base) = items.last().filter(|_| attached) {
                (base.level, base.script)
            } else {
                (
                    line_levels[offset - paragraph.range.start].number(),
                    script.as_iso15924_tag(),
                )
            };
            let scalar_end = offset + ch.len_utf8();
            if let Some(last) = items.last_mut().filter(|last| {
                last.context == context
                    && last.range.end == offset
                    && last.level == level
                    && last.script == tag
            }) {
                last.range.end = scalar_end;
            } else {
                reserve(&mut items, 1, limits)?;
                items.push(Item {
                    range: offset..scalar_end,
                    context: context.clone(),
                    level,
                    script: tag,
                });
            }
        }
    }
    Ok((items, breaks))
}
fn priorities(
    catalog: &ValidatedFontCatalog,
    style: &ShapeStyle,
    limits: &Limits,
) -> Result<Vec<usize>, FontError> {
    if !matches!(style.encoding(), 0 | 1)
        || !style.features().is_empty()
        || !style.variations().is_empty()
    {
        return Err(error(
            ErrorCode::UnsupportedFeature,
            "legacy/custom variation shaping is not enabled",
        ));
    }
    let family = fold_family(
        style
            .font()
            .family()
            .strip_prefix('@')
            .unwrap_or(style.font().family()),
        limits,
    )?;
    if !(0..catalog.faces().len()).any(|index| catalog.matches_family(index, &family)) {
        return Err(error(
            ErrorCode::FontUnavailable,
            "requested family is absent from validated catalog",
        ));
    }
    let mut indices = Vec::new();
    reserve(&mut indices, catalog.faces().len(), limits)?;
    indices.extend(0..catalog.faces().len());
    indices.sort_unstable_by_key(|&i| {
        let face = &catalog.faces()[i];
        let same = catalog.matches_family(i, &family);
        (
            !same,
            if same {
                face.weight().abs_diff(style.font().weight())
            } else {
                0
            },
            if same {
                face.italic() != style.font().italic()
            } else {
                false
            },
            i,
        )
    });
    Ok(indices)
}
fn resolve_mapped(
    catalog: &ValidatedFontCatalog,
    map: &Arc<SourceMappedText>,
    styles: &[(DecodedSpan, ShapeStyle)],
    options: &ShapeOptions,
    limits: &Limits,
    drawings: &[DecodedSpan],
) -> Result<(Vec<ResolvedRun>, Vec<DecodedSpan>), FontError> {
    // Drawing commands are not text: neutralize their bytes for bidi analysis,
    // while keeping all original decoded byte offsets and native text ranges.
    let mut analysis = Vec::new();
    if !drawings.is_empty() {
        reserve(&mut analysis, map.text().len(), limits)?;
        analysis.extend_from_slice(map.text().as_bytes());
        for span in drawings {
            analysis[span.start() as usize..span.end() as usize].fill(b' ');
        }
    }
    let text = if analysis.is_empty() {
        map.text()
    } else {
        std::str::from_utf8(&analysis)
            .map_err(|_| error(ErrorCode::InvalidArgument, "invalid drawing analysis"))?
    };
    let (items, breaks) = itemize(text, options, limits)?;
    let mut result = Vec::new();
    let mut item_cursor = 0;
    for (span, style) in styles {
        if !span.belongs_to(map) {
            return Err(error(
                ErrorCode::InvalidArgument,
                "style span belongs to another map",
            ));
        }
        let order = priorities(catalog, style, limits)?;
        let primary = catalog.faces()[*order
            .first()
            .ok_or_else(|| error(ErrorCode::FontUnavailable, "empty font catalog"))?]
        .blob()
        .id();
        while item_cursor < items.len() && items[item_cursor].range.end <= span.start() as usize {
            item_cursor += 1;
        }
        for item in items[item_cursor..]
            .iter()
            .take_while(|item| item.range.start < span.end() as usize)
        {
            let start = item.range.start.max(span.start() as usize);
            let end = item.range.end.min(span.end() as usize);
            if start >= end {
                continue;
            }
            let mut item = item.clone();
            item.range = start..end;
            let left = drawings.partition_point(|d| d.end() as usize <= start);
            if left > 0 {
                item.context.start = item.context.start.max(drawings[left - 1].end() as usize);
            }
            let right = drawings.partition_point(|d| (d.start() as usize) < end);
            if let Some(drawing) = drawings.get(right) {
                item.context.end = item.context.end.min(drawing.start() as usize);
            }
            reserve(&mut result, 1, limits)?;
            result.push(ResolvedRun {
                catalog: catalog.clone(),
                primary,
                decoded: map.decoded_span(start as u32..end as u32)?,
                item,
                style: style.clone(),
                options: options.clone(),
                limits: limits.clone(),
            });
        }
    }
    result.sort_unstable_by_key(|r| r.decoded.start());
    let mut owned_breaks = Vec::new();
    reserve(&mut owned_breaks, breaks.len(), limits)?;
    for range in breaks {
        owned_breaks.push(map.decoded_span(range)?);
    }
    Ok((result, owned_breaks))
}
pub fn resolve_runs(
    catalog: &ValidatedFontCatalog,
    text: &str,
    style: &ShapeStyle,
) -> Result<Vec<ResolvedRun>, FontError> {
    let limits = Limits::default();
    preflight(text, &limits)?;
    let source: Arc<str> = Arc::from(text);
    let literal =
        SourceSpan::new(&source, 0, source.len()).map_err(|e| error(e.code(), e.detail()))?;
    let map = Arc::new(SourceMappedText::from_pieces(
        source,
        &[TextSourcePiece::Literal(literal)],
        WrapMode::new(2).map_err(|e| error(e.code(), e.detail()))?,
        &limits,
    )?);
    let span = map.decoded_span(0..text.len() as u32)?;
    Ok(resolve_mapped(
        catalog,
        &map,
        &[(span, style.clone())],
        &ShapeOptions::default(),
        &limits,
        &[],
    )?
    .0)
}
struct Candidate {
    index: usize,
    raw: RawRun,
    missing: Vec<u32>,
}
impl Candidate {
    fn supports(&self, a: u32, b: u32) -> bool {
        self.missing
            .get(self.missing.partition_point(|&x| x < a))
            .is_none_or(|&x| x >= b)
    }
}
fn boundaries(raw: &RawRun, start: u32, end: u32, limits: &Limits) -> Result<Vec<u32>, FontError> {
    let mut points = Vec::new();
    reserve(&mut points, raw.glyphs.len() + 2, limits)?;
    points.extend(raw.glyphs.iter().map(|g| g.cluster));
    points.push(start);
    points.push(end);
    points.sort_unstable();
    points.dedup();
    Ok(points)
}
fn request<'a>(input: &'a ResolvedRun, range: Range<usize>) -> NativeRequest<'a> {
    NativeRequest {
        text: input.decoded.source_map().text(),
        context: input.item.context.clone(),
        range,
        rtl: input.item.level % 2 != 0,
        script: input.item.script,
        language: input.options.language(),
        ligatures: input.style.spacing().get() == 0.0,
        kerning: input.options.kerning,
        vertical: input.style.font().vertical() || input.style.font().family().starts_with('@'),
    }
}
fn shape_item(
    ctx: &mut FontContext,
    input: &ResolvedRun,
    collect_ink: bool,
) -> Result<(ShapedRun, Vec<MissingCluster>), FontError> {
    let limits = &input.limits;
    let order = priorities(&input.catalog, &input.style, limits)?;
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut points: Vec<u32> = Vec::new();
    let mut retained = 0_u64;
    for index in order {
        let mut remaining = limits.clone();
        remaining = remaining
            .with_transient_bytes(
                limits
                    .transient_bytes()
                    .checked_sub(retained)
                    .filter(|&n| n != 0)
                    .ok_or_else(|| {
                        limited().with_resource_failure(ResourceFailure::TransientMemory)
                    })?,
            )
            .map_err(|_| limited())?;
        let raw = input.catalog.shape_raw(
            ctx,
            input.catalog.faces()[index].blob().id(),
            &request(input, input.item.range.clone()),
            &remaining,
            collect_ink,
        )?;
        retained = retained
            .checked_add(
                (raw.glyphs.len() as u64)
                    .checked_mul((3 * std::mem::size_of::<RawGlyph>() + 32) as u64)
                    .ok_or_else(limited)?,
            )
            .ok_or_else(limited)?;
        if retained > limits.transient_bytes() {
            return Err(limited().with_resource_failure(ResourceFailure::TransientMemory));
        }
        let candidate_points =
            boundaries(&raw, input.decoded.start(), input.decoded.end(), limits)?;
        if candidates.is_empty() {
            points = candidate_points;
        } else {
            points.retain(|p| candidate_points.binary_search(p).is_ok());
        }
        let mut missing = Vec::new();
        reserve(&mut missing, raw.glyphs.len(), limits)?;
        missing.extend(
            raw.glyphs
                .iter()
                .filter(|g| g.glyph == 0)
                .map(|g| g.cluster),
        );
        missing.sort_unstable();
        missing.dedup();
        reserve(&mut candidates, 1, limits)?;
        candidates.push(Candidate {
            index,
            raw,
            missing,
        });
        if points
            .windows(2)
            .all(|p| candidates.iter().any(|c| c.supports(p[0], p[1])))
        {
            break;
        }
    }
    let map = input.decoded.source_map();
    let mut groups: Vec<(u32, u32, usize, bool)> = Vec::new();
    for pair in points.windows(2) {
        let selected = candidates.iter().position(|c| c.supports(pair[0], pair[1]));
        let selected_index = selected.unwrap_or(0);
        if let Some(last) = groups
            .last_mut()
            .filter(|g| g.2 == selected_index && g.3 == selected.is_none() && g.1 == pair[0])
        {
            last.1 = pair[1];
        } else {
            reserve(&mut groups, 1, limits)?;
            groups.push((pair[0], pair[1], selected_index, selected.is_none()));
        }
    }
    if input.item.level % 2 != 0 {
        groups.reverse();
    } // reorder font groups only; native glyph arrays are never reversed.
    let mut glyphs = Vec::new();
    let mut missing = Vec::new();
    let mut face_metrics = Vec::new();
    for (a, b, selected, notdef) in groups {
        let face = input.catalog.faces()[candidates[selected].index].blob();
        // Reshape selected font ranges with paragraph context: do not carry
        // kerning/GPOS across a font boundary by slicing an earlier whole run.
        let reshaped;
        let raw = if a as usize == input.item.range.start && b as usize == input.item.range.end {
            &candidates[selected].raw
        } else {
            reshaped = input.catalog.shape_raw(
                ctx,
                face.id(),
                &request(input, a as usize..b as usize),
                limits,
                collect_ink,
            )?;
            &reshaped
        };
        let ends = boundaries(raw, a, b, limits)?;
        let [lx, ly] = input.options.layout_scale.map(FiniteF64::get);
        let cell_size =
            FiniteF64::new(input.style.font_size().get() * ly).map_err(|_| limited())?;
        let vertical =
            input.style.font().vertical() || input.style.font().family().starts_with('@');
        let scale = input
            .catalog
            .ass_oriented_font_scale(face.id(), cell_size, vertical)?
            .get();
        let baseline_offset =
            input
                .catalog
                .ass_oriented_baseline_offset(face.id(), cell_size, vertical)?
                / ly;
        if u32::from(input.catalog.ass_cell_metrics(face.id())?.0) != raw.upem {
            return Err(error(
                ErrorCode::NativeFailure,
                "font UPEM changed after validation",
            ));
        }
        let finite = |v| FiniteF64::new(v).map_err(|_| limited());
        reserve(&mut face_metrics, 1, limits)?;
        face_metrics.push(FaceMetrics {
            font: face.id(),
            glyphs: glyphs.len()
                ..glyphs
                    .len()
                    .checked_add(raw.glyphs.len())
                    .ok_or_else(limited)?,
            metrics: {
                let m = input
                    .catalog
                    .ass_oriented_line_metrics(face.id(), cell_size, vertical)?;
                RunMetrics::new(
                    finite(m.ascent().get() / ly)?,
                    finite(m.descent().get() / ly)?,
                    finite(m.line_gap().get() / ly)?,
                    m.units_per_em(),
                )?
            },
        });
        if glyphs
            .len()
            .checked_add(raw.glyphs.len())
            .is_none_or(|n| n as u64 > limits.glyphs_per_event())
        {
            return Err(limited());
        }
        reserve(&mut glyphs, raw.glyphs.len(), limits)?;
        for g in &raw.glyphs {
            let end = *ends
                .get(ends.partition_point(|&p| p <= g.cluster))
                .ok_or_else(|| error(ErrorCode::NativeFailure, "missing HarfBuzz cluster end"))?;
            let cluster = map.decoded_span(g.cluster..end)?;
            let position = GlyphPosition::new(
                finite(f64::from(g.advance_x) * scale / lx)?,
                finite(-f64::from(g.advance_y) * scale / ly)?,
                finite(f64::from(g.offset_x) * scale / lx)?,
                finite(-f64::from(g.offset_y) * scale / ly + baseline_offset)?,
            );
            let ink = if cluster.text().chars().all(char::is_whitespace) {
                GlyphInk::Empty
            } else if let Some(bounds) = g.ink {
                let [x, y, w, h] = bounds;
                if w == 0 || h == 0 {
                    GlyphInk::Empty
                } else {
                    let italic = input
                        .catalog
                        .synthetic_italic_shear(face.id(), input.style.font().italic())?
                        .get();
                    let mut bounds: Option<(f64, f64, f64, f64)> = None;
                    // Ink and copied outline use the same synthetic font map,
                    // before vertical rotation and script/device scaling.
                    for x in [f64::from(x), (i64::from(x) + i64::from(w)) as f64] {
                        for y in [f64::from(y), (i64::from(y) + i64::from(h)) as f64] {
                            let (x, y) = (x + italic * y, y);
                            let (x, y) = if let Some([dv, desc]) = g.vertical_offset {
                                (f64::from(dv) - y, x + f64::from(desc))
                            } else {
                                (x, y)
                            };
                            let (x, y) = (x * scale / lx, -y * scale / ly);
                            bounds = Some(bounds.map_or((x, y, x, y), |(a, b, c, d)| {
                                (a.min(x), b.min(y), c.max(x), d.max(y))
                            }));
                        }
                    }
                    let (x0, y0, x1, y1) = bounds.ok_or_else(limited)?;
                    GlyphInk::Bounds(
                        ScriptBounds::new(x0, y0, x1, y1)
                            .map_err(|e| error(e.code(), e.detail()))?,
                    )
                }
            } else {
                GlyphInk::Unknown
            };
            glyphs.push(
                ShapedGlyph::new(face.id(), g.glyph, cluster.clone(), position)
                    .with_ink(ink)
                    .with_vertical_offset(
                        g.vertical_offset
                            .map(|[x, y]| FontPoint::new(f64::from(x), f64::from(y)))
                            .transpose()
                            .map_err(|_| limited())?,
                    ),
            );
            if g.glyph == 0
                && missing
                    .last()
                    .is_none_or(|m: &MissingCluster| m.decoded != cluster)
            {
                reserve(&mut missing, 1, limits)?;
                missing.push(MissingCluster {
                    font: face.id(),
                    decoded: cluster,
                });
            }
            let _ = notdef;
        }
    }
    let metrics = face_metrics
        .first()
        .ok_or_else(|| error(ErrorCode::NativeFailure, "empty native font groups"))?
        .metrics;
    let mut diagnostics = DiagnosticBuffer::new();
    for item in &missing {
        diagnostics
            .push(Diagnostic::new(
                DiagnosticCode::MissingGlyph,
                None,
                None,
                &format!(
                    "missing glyph in decoded bytes {}..{}; exact original intervals retained",
                    item.decoded.start(),
                    item.decoded.end()
                ),
            ))
            .map_err(|e| error(e.code(), e.detail()))?;
    }
    Ok((
        ShapedRun::new(
            glyphs.into(),
            metrics,
            input.decoded.clone(),
            input.item.level,
            input.item.script,
            input.options.language.clone(),
        )?
        .with_face_metrics(face_metrics.into())?
        .with_diagnostics(
            diagnostics
                .finish()
                .map_err(|e| error(e.code(), e.detail()))?
                .into(),
        )?,
        missing,
    ))
}
pub fn shape(ctx: &mut FontContext, input: &ResolvedRun) -> Result<ShapedRun, FontError> {
    Ok(shape_item(ctx, input, true)?.0)
}
// Match xy's word context: ASCII spaces delimit words; NBSP does not. This is
// confined to the new automatic path; existing q2 shaping stays unchanged.
fn xy_word_items(
    inputs: Vec<ResolvedRun>,
    map: &Arc<SourceMappedText>,
    limits: &Limits,
) -> Result<Vec<ResolvedRun>, FontError> {
    let mut words = Vec::new();
    let mut start = 0;
    for (at, ch) in map.text().char_indices() {
        if matches!(ch, ' ' | '\n' | '\r' | '\u{2029}') {
            if start < at {
                reserve(&mut words, 1, limits)?;
                words.push(start..at);
            }
            reserve(&mut words, 1, limits)?;
            words.push(at..at + ch.len_utf8());
            start = at + ch.len_utf8();
        }
    }
    if start < map.text().len() {
        reserve(&mut words, 1, limits)?;
        words.push(start..map.text().len());
    }
    let mut result = Vec::new();
    let mut cursor = 0;
    for input in inputs {
        while cursor < words.len() && words[cursor].end <= input.item.range.start {
            cursor += 1;
        }
        for word in words[cursor..]
            .iter()
            .take_while(|w| w.start < input.item.range.end)
        {
            let range = word.start.max(input.item.range.start)..word.end.min(input.item.range.end);
            let mut part = input.clone();
            part.decoded = map.decoded_span(range.start as u32..range.end as u32)?;
            part.item.range = range;
            part.item.context =
                word.start.max(input.item.context.start)..word.end.min(input.item.context.end);
            reserve(&mut result, 1, limits)?;
            result.push(part);
        }
    }
    Ok(result)
}

pub fn shape_event(
    ctx: &mut FontContext,
    catalog: &ValidatedFontCatalog,
    event: &EvaluatedEvent,
    options: &ShapeOptions,
    limits: &Limits,
) -> Result<ShapedEvent, FontError> {
    shape_event_impl(ctx, catalog, event, options, limits, true)
}

/// W2 experiment only: identical boundary shaping, fallback, clusters and
/// advances, without native ink extraction. Unknown ink deliberately prevents
/// publishing this artifact through ordinary layout/rendering. No wrapping
/// selection or production automatic mode is enabled by this API.
pub fn shape_event_advances_experiment(
    ctx: &mut FontContext,
    catalog: &ValidatedFontCatalog,
    event: &EvaluatedEvent,
    options: &ShapeOptions,
    limits: &Limits,
) -> Result<ShapedEvent, FontError> {
    shape_event_impl(ctx, catalog, event, options, limits, false)
}

fn shape_event_impl(
    ctx: &mut FontContext,
    catalog: &ValidatedFontCatalog,
    event: &EvaluatedEvent,
    options: &ShapeOptions,
    limits: &Limits,
    collect_ink: bool,
) -> Result<ShapedEvent, FontError> {
    if event.wrap_mode().get() != 2 && event.warp_mode() == WarpMode::Unicode {
        return Err(error(
            ErrorCode::UnsupportedFeature,
            "warpmode1 automatic wrapping is not admitted",
        ));
    }
    let map = event.source_map().ok_or_else(|| {
        error(
            ErrorCode::InvalidArgument,
            "exact shaping requires explicit original-source provenance",
        )
    })?;
    preflight(map.text(), limits)?;
    let mut styles: Vec<(DecodedSpan, ShapeStyle)> = Vec::new();
    let mut paints = Vec::new();
    let mut previous_scale = None;
    let mut previous_karaoke = None;
    for run in event.runs() {
        let span = run.decoded().ok_or_else(|| {
            error(
                ErrorCode::InvalidArgument,
                "evaluated run lacks decoded provenance",
            )
        })?;
        if !span.belongs_to(map) || span.text() != run.text() {
            return Err(error(
                ErrorCode::InvalidArgument,
                "evaluated text uses a foreign map",
            ));
        }
        reserve(&mut paints, 1, limits)?;
        paints.push(PaintSpan {
            decoded: span.clone(),
            paint: run.paint().clone(),
        });
        if span.is_empty() || run.drawing().is_some() {
            previous_scale = None;
            continue;
        }
        let scale = (run.geometry().scale_x(), run.geometry().scale_y());
        if let Some((last, style)) = styles.last_mut().filter(|(last, style)| {
            last.end() == span.start()
                && style == run.shaping()
                && previous_scale == Some(scale)
                && previous_karaoke == run.karaoke()
        }) {
            *last = map.decoded_span(last.start()..span.end())?;
            let _ = style;
        } else {
            reserve(&mut styles, 1, limits)?;
            styles.push((span.clone(), run.shaping().clone()));
        }
        previous_scale = Some(scale);
        previous_karaoke = run.karaoke();
    }
    let mut drawings = Vec::new();
    for run in event.runs().iter().filter(|r| r.drawing().is_some()) {
        reserve(&mut drawings, 1, limits)?;
        drawings.push(run.decoded().ok_or_else(limited)?.clone());
    }
    let (resolved, breaks) = resolve_mapped(catalog, map, &styles, options, limits, &drawings)?;
    let resolved = if event.wrap_mode().get() != 2 {
        xy_word_items(resolved, map, limits)?
    } else {
        resolved
    };
    let mut runs = Vec::new();
    let mut missing = Vec::new();
    let mut diagnostics = DiagnosticBuffer::new();
    let mut count = 0_u64;
    for input in &resolved {
        let (run, absent) = shape_item(ctx, input, collect_ink)?;
        count = count
            .checked_add(run.glyphs().len() as u64)
            .ok_or_else(limited)?;
        if count > limits.glyphs_per_event() {
            return Err(limited());
        }
        for item in absent {
            diagnostics.push(Diagnostic::new(DiagnosticCode::MissingGlyph, Some(event.id()), None, &format!("missing glyph in decoded bytes {}..{}; original intervals retained in shaped result", item.decoded.start(), item.decoded.end()))).map_err(|e| error(e.code(), e.detail()))?;
            if missing.len() < MAX_DIAGNOSTICS {
                reserve(&mut missing, 1, limits)?;
                missing.push(item);
            }
        }
        reserve(&mut runs, 1, limits)?;
        runs.push(run);
    }
    for evaluated in event.runs().iter().filter(|r| r.drawing().is_some()) {
        let drawing = evaluated.drawing().ok_or_else(limited)?;
        let span = evaluated.decoded().ok_or_else(limited)?;
        if span.is_empty() {
            continue;
        }
        count = count.checked_add(1).ok_or_else(limited)?;
        if count > limits.glyphs_per_event() {
            return Err(limited());
        }
        let metrics = RunMetrics::new(
            FiniteF64::new(drawing.ascent.get().max(0.0)).map_err(|_| limited())?,
            FiniteF64::new(drawing.descent.get().max(0.0)).map_err(|_| limited())?,
            FiniteF64::ZERO,
            1,
        )?;
        reserve(&mut runs, 1, limits)?;
        runs.push(
            ShapedRun::new(
                Arc::from([]),
                metrics,
                span.clone(),
                0,
                u32::from_be_bytes(*b"Zyyy"),
                Arc::from("und"),
            )?
            .with_drawing(drawing.clone())?,
        );
    }
    runs.sort_unstable_by_key(|r| r.decoded().start());
    let empty_lines = empty_line_metrics(ctx, catalog, event, map, &breaks, options, limits)?;
    ShapedEvent::new(
        event,
        map.clone(),
        runs.into(),
        breaks.into(),
        paints.into(),
        missing.into(),
        diagnostics
            .finish()
            .map_err(|e| error(e.code(), e.detail()))?
            .into(),
    )?
    .with_empty_lines(empty_lines.into())
}

fn empty_line_metrics(
    ctx: &mut FontContext,
    catalog: &ValidatedFontCatalog,
    event: &EvaluatedEvent,
    map: &Arc<SourceMappedText>,
    breaks: &[DecodedSpan],
    options: &ShapeOptions,
    limits: &Limits,
) -> Result<Vec<EmptyLineMetrics>, FontError> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut previous: Option<(FontId, RawRun)> = None;
    for i in 0..=breaks.len() {
        let end = breaks.get(i).map_or(map.text().len() as u32, |b| b.start());
        if start == end {
            let evaluated = event.runs().get(
                event
                    .runs()
                    .partition_point(|r| r.decoded().is_some_and(|s| s.end() <= start)),
            );
            let (style, scale_y) = if let Some(run) = evaluated {
                (run.shaping().clone(), run.geometry().scale_y())
            } else if let Some(style) = event.terminal_style() {
                (ShapeStyle::from_style(style), style.scale_y())
            } else if let Some(run) = event.runs().last() {
                (run.shaping().clone(), run.geometry().scale_y())
            } else {
                return Err(error(
                    ErrorCode::InvalidArgument,
                    "empty event lacks explicit style metrics",
                ));
            };
            let order = priorities(catalog, &style, limits)?;
            let face = catalog.faces()[order[0]].blob();
            if previous.as_ref().is_none_or(|(id, _)| *id != face.id()) {
                let request = NativeRequest {
                    text: "",
                    context: 0..0,
                    range: 0..0,
                    rtl: false,
                    script: u32::from_be_bytes(*b"Zyyy"),
                    language: options.language(),
                    ligatures: true,
                    kerning: options.kerning,
                    vertical: style.font().vertical() || style.font().family().starts_with('@'),
                };
                previous = Some((
                    face.id(),
                    catalog.shape_raw(ctx, face.id(), &request, limits, true)?,
                ));
            }
            let raw = &previous
                .as_ref()
                .ok_or_else(|| error(ErrorCode::NativeFailure, "missing font metrics"))?
                .1;
            if u32::from(catalog.ass_cell_metrics(face.id())?.0) != raw.upem {
                return Err(error(
                    ErrorCode::NativeFailure,
                    "font UPEM changed after validation",
                ));
            }
            let ly = options.layout_scale[1].get();
            let cell = FiniteF64::new(style.font_size().get() * ly).map_err(|_| limited())?;
            let m = catalog.ass_oriented_line_metrics(
                face.id(),
                cell,
                style.font().vertical() || style.font().family().starts_with('@'),
            )?;
            let f = |n: f64| FiniteF64::new(n).map_err(|_| limited());
            let metrics = RunMetrics::new(
                f(m.ascent().get() / ly)?,
                f(m.descent().get() / ly)?,
                f(m.line_gap().get() / ly)?,
                m.units_per_em(),
            )?;
            reserve(&mut result, 1, limits)?;
            result.push(EmptyLineMetrics {
                decoded: map.decoded_span(start..start)?,
                font: face.id(),
                metrics,
                scale_y,
            });
        }
        if let Some(b) = breaks.get(i) {
            start = b.end();
        }
    }
    Ok(result)
}
