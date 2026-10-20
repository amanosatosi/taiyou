//! Owned C4 results. Decoded cluster spans retain their actual source-map owner.
use crate::*;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlyphPosition {
    advance_x: FiniteF64,
    advance_y: FiniteF64,
    offset_x: FiniteF64,
    offset_y: FiniteF64,
}
impl GlyphPosition {
    pub const fn new(
        advance_x: FiniteF64,
        advance_y: FiniteF64,
        offset_x: FiniteF64,
        offset_y: FiniteF64,
    ) -> Self {
        Self {
            advance_x,
            advance_y,
            offset_x,
            offset_y,
        }
    }
    pub const fn advance_x(&self) -> FiniteF64 {
        self.advance_x
    }
    pub const fn advance_y(&self) -> FiniteF64 {
        self.advance_y
    }
    pub const fn offset_x(&self) -> FiniteF64 {
        self.offset_x
    }
    pub const fn offset_y(&self) -> FiniteF64 {
        self.offset_y
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RunMetrics {
    ascent: FiniteF64,
    descent: FiniteF64,
    line_gap: FiniteF64,
    units_per_em: u16,
}
impl RunMetrics {
    pub fn new(
        ascent: FiniteF64,
        descent: FiniteF64,
        line_gap: FiniteF64,
        units_per_em: u16,
    ) -> Result<Self, FontError> {
        if ascent.get() < 0.0 || descent.get() < 0.0 || units_per_em == 0 {
            return Err(FontError::new(
                ErrorCode::InvalidArgument,
                "invalid font metrics",
            ));
        }
        Ok(Self {
            ascent,
            descent,
            line_gap,
            units_per_em,
        })
    }
    pub const fn ascent(&self) -> FiniteF64 {
        self.ascent
    }
    pub const fn descent(&self) -> FiniteF64 {
        self.descent
    }
    pub const fn line_gap(&self) -> FiniteF64 {
        self.line_gap
    }
    pub const fn units_per_em(&self) -> u16 {
        self.units_per_em
    }
}
/// Per-face metrics remain meaningful even when fallback uses different UPEMs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FaceMetrics {
    pub font: FontId,
    pub metrics: RunMetrics,
    pub glyphs: std::ops::Range<usize>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapedGlyph {
    ink: GlyphInk,
    font: FontId,
    glyph: u32,
    cluster: DecodedSpan,
    position: GlyphPosition,
}
impl ShapedGlyph {
    pub fn new(font: FontId, glyph: u32, cluster: DecodedSpan, position: GlyphPosition) -> Self {
        Self {
            ink: GlyphInk::Unknown,
            font,
            glyph,
            cluster,
            position,
        }
    }
    pub const fn font(&self) -> FontId {
        self.font
    }
    pub fn with_ink(mut self, ink: GlyphInk) -> Self {
        self.ink = ink;
        self
    }
    pub const fn ink(&self) -> GlyphInk {
        self.ink
    }
    pub const fn glyph(&self) -> u32 {
        self.glyph
    }
    pub fn cluster(&self) -> &DecodedSpan {
        &self.cluster
    }
    pub const fn position(&self) -> GlyphPosition {
        self.position
    }
    pub const fn advance_x(&self) -> FiniteF64 {
        self.position.advance_x
    }
    pub const fn advance_y(&self) -> FiniteF64 {
        self.position.advance_y
    }
    pub const fn offset_x(&self) -> FiniteF64 {
        self.position.offset_x
    }
    pub const fn offset_y(&self) -> FiniteF64 {
        self.position.offset_y
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapedRun {
    drawing: Option<Arc<AssDrawing>>,
    glyphs: Arc<[ShapedGlyph]>,
    metrics: RunMetrics,
    face_metrics: Arc<[FaceMetrics]>,
    decoded: DecodedSpan,
    bidi_level: u8,
    script: u32,
    language: Arc<str>,
    diagnostics: Arc<[Diagnostic]>,
}
impl ShapedRun {
    pub fn new(
        glyphs: Arc<[ShapedGlyph]>,
        metrics: RunMetrics,
        decoded: DecodedSpan,
        bidi_level: u8,
        script: u32,
        language: Arc<str>,
    ) -> Result<Self, FontError> {
        for glyph in glyphs.iter() {
            if !glyph.cluster.belongs_to(decoded.source_map())
                || glyph.cluster.is_empty()
                || glyph.cluster.start() < decoded.start()
                || glyph.cluster.end() > decoded.end()
            {
                return Err(FontError::new(
                    ErrorCode::InvalidArgument,
                    "glyph cluster belongs to another source map or escapes run",
                ));
            }
        }
        Ok(Self {
            drawing: None,
            glyphs,
            metrics,
            face_metrics: Arc::from([]),
            decoded,
            bidi_level,
            script,
            language,
            diagnostics: Arc::from([]),
        })
    }
    pub fn glyphs(&self) -> &[ShapedGlyph] {
        &self.glyphs
    }
    pub fn with_drawing(mut self, drawing: Arc<AssDrawing>) -> Result<Self, FontError> {
        if !self.glyphs.is_empty() { return Err(FontError::new(ErrorCode::InvalidArgument, "drawing run contains font glyphs")); }
        self.drawing = Some(drawing);
        Ok(self)
    }
    pub fn drawing(&self) -> Option<&Arc<AssDrawing>> { self.drawing.as_ref() }
    pub fn with_face_metrics(mut self, metrics: Arc<[FaceMetrics]>) -> Result<Self, FontError> {
        if metrics.is_empty() || metrics[0].metrics != self.metrics {
            return Err(FontError::new(
                ErrorCode::InvalidArgument,
                "incomplete face metrics",
            ));
        }
        let mut end = 0;
        for group in metrics.iter() {
            if group.glyphs.start != end
                || self
                    .glyphs
                    .get(group.glyphs.clone())
                    .is_none_or(|glyphs| glyphs.iter().any(|g| g.font != group.font))
            {
                return Err(FontError::new(
                    ErrorCode::InvalidArgument,
                    "invalid face metric groups",
                ));
            }
            end = group.glyphs.end;
        }
        if end != self.glyphs.len() {
            return Err(FontError::new(
                ErrorCode::InvalidArgument,
                "incomplete face metric groups",
            ));
        }
        self.face_metrics = metrics;
        Ok(self)
    }
    /// Native font groups in glyph order; layout can aggregate script metrics.
    pub fn face_metrics(&self) -> &[FaceMetrics] {
        &self.face_metrics
    }
    pub fn with_diagnostics(mut self, diagnostics: Arc<[Diagnostic]>) -> Result<Self, FontError> {
        if diagnostics.len() > MAX_DIAGNOSTICS + 1 {
            return Err(FontError::new(
                ErrorCode::LimitExceeded,
                "shaping diagnostics exceed bounded publication",
            ));
        }
        self.diagnostics = diagnostics;
        Ok(self)
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    /// Metrics of the first native font group. All fallback groups retain their own.
    pub const fn metrics(&self) -> RunMetrics {
        self.metrics
    }
    pub fn decoded(&self) -> &DecodedSpan {
        &self.decoded
    }
    pub fn source_map(&self) -> &Arc<SourceMappedText> {
        self.decoded.source_map()
    }
    pub const fn bidi_level(&self) -> u8 {
        self.bidi_level
    }
    pub const fn direction(&self) -> Direction {
        if self.bidi_level % 2 == 0 {
            Direction::Ltr
        } else {
            Direction::Rtl
        }
    }
    pub const fn script(&self) -> u32 {
        self.script
    }
    pub fn language(&self) -> &str {
        &self.language
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingCluster {
    pub font: FontId,
    pub decoded: DecodedSpan,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaintSpan {
    pub decoded: DecodedSpan,
    pub paint: PaintSet,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapedEvent {
    empty_lines: Arc<[EmptyLineMetrics]>,
    evaluated: EvaluatedEvent,
    source_map: Arc<SourceMappedText>,
    runs: Arc<[ShapedRun]>,
    breaks: Arc<[DecodedSpan]>,
    paints: Arc<[PaintSpan]>,
    missing: Arc<[MissingCluster]>,
    diagnostics: Arc<[Diagnostic]>,
}
impl ShapedEvent {
    pub fn new(
        evaluated: &EvaluatedEvent,
        source_map: Arc<SourceMappedText>,
        runs: Arc<[ShapedRun]>,
        breaks: Arc<[DecodedSpan]>,
        paints: Arc<[PaintSpan]>,
        missing: Arc<[MissingCluster]>,
        diagnostics: Arc<[Diagnostic]>,
    ) -> Result<Self, FontError> {
        if evaluated
            .source_map()
            .is_none_or(|map| !Arc::ptr_eq(map, &source_map))
            || runs.iter().any(|r| !r.decoded.belongs_to(&source_map))
            || breaks.iter().any(|s| !s.belongs_to(&source_map))
            || paints.iter().any(|s| !s.decoded.belongs_to(&source_map))
            || missing.iter().any(|s| !s.decoded.belongs_to(&source_map))
        {
            return Err(FontError::new(
                ErrorCode::InvalidArgument,
                "shaped event contains a foreign decoded map",
            ));
        }
        let (mut r, mut b, mut cursor) = (0, 0, 0);
        while r < runs.len() || b < breaks.len() {
            let next = if b == breaks.len()
                || (r < runs.len() && runs[r].decoded.start() < breaks[b].start())
            {
                let span = &runs[r].decoded;
                r += 1;
                span
            } else {
                let span = &breaks[b];
                b += 1;
                span
            };
            if next.is_empty() || next.start() != cursor {
                return Err(FontError::new(
                    ErrorCode::InvalidArgument,
                    "logical shape/break ranges overlap or omit decoded text",
                ));
            }
            cursor = next.end();
        }
        if cursor as usize != source_map.text().len()
            || diagnostics.len() > MAX_DIAGNOSTICS + 1
            || missing.len() > MAX_DIAGNOSTICS
        {
            return Err(FontError::new(
                ErrorCode::InvalidArgument,
                "shaped publication extent or diagnostic bound is invalid",
            ));
        }
        let mut end = 0;
        for p in paints.iter() {
            if p.decoded.start() != end {
                return Err(FontError::new(
                    ErrorCode::InvalidArgument,
                    "paint spans do not follow decoded text",
                ));
            }
            end = p.decoded.end();
        }
        if end as usize != source_map.text().len() {
            return Err(FontError::new(
                ErrorCode::InvalidArgument,
                "paint spans do not cover decoded text",
            ));
        }
        Ok(Self {
            empty_lines: Arc::from([]),
            evaluated: evaluated.clone(),
            source_map,
            runs,
            breaks,
            paints,
            missing,
            diagnostics,
        })
    }
    pub const fn id(&self) -> EventId {
        self.evaluated.id()
    }
    pub fn with_empty_lines(mut self, lines: Arc<[EmptyLineMetrics]>) -> Result<Self, FontError> {
        let mut previous = None;
        for line in lines.iter() {
            if !line.decoded.belongs_to(&self.source_map)
                || !line.decoded.is_empty()
                || line.scale_y.get() < 0.0
                || previous.is_some_and(|p| p >= line.decoded.start())
            {
                return Err(FontError::new(
                    ErrorCode::InvalidArgument,
                    "invalid empty-line metrics",
                ));
            }
            previous = Some(line.decoded.start());
        }
        self.empty_lines = lines;
        Ok(self)
    }
    pub fn empty_lines(&self) -> &[EmptyLineMetrics] {
        &self.empty_lines
    }
    pub const fn order(&self) -> EventOrder {
        self.evaluated.order()
    }
    pub fn placement(&self) -> &Placement {
        self.evaluated.placement()
    }
    pub const fn clip(&self) -> ClipSpec {
        self.evaluated.clip()
    }
    /// Original decoded ranges retain geometry, effects and paint associations.
    pub fn evaluated(&self) -> &EvaluatedEvent {
        &self.evaluated
    }
    pub fn source_map(&self) -> &Arc<SourceMappedText> {
        &self.source_map
    }
    pub fn runs(&self) -> &[ShapedRun] {
        &self.runs
    }
    pub fn breaks(&self) -> &[DecodedSpan] {
        &self.breaks
    }
    pub fn missing(&self) -> &[MissingCluster] {
        &self.missing
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    pub fn paint_for(&self, cluster: &DecodedSpan) -> Result<&PaintSet, FontError> {
        if !cluster.belongs_to(&self.source_map) || cluster.is_empty() {
            return Err(FontError::new(
                ErrorCode::InvalidArgument,
                "paint lookup requires a cluster from this map",
            ));
        }
        let i = self
            .paints
            .partition_point(|p| p.decoded.end() <= cluster.start());
        self.paints.get(i).map(|p| &p.paint).ok_or_else(|| {
            FontError::new(ErrorCode::InvalidArgument, "cluster has no logical paint")
        })
    }
}
