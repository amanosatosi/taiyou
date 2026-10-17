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
}
impl RunMetrics {
    pub const fn new(ascent: FiniteF64, descent: FiniteF64, line_gap: FiniteF64) -> Self {
        Self {
            ascent,
            descent,
            line_gap,
        }
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
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapedGlyph {
    font: FontId,
    glyph: u32,
    cluster: DecodedSpan,
    position: GlyphPosition,
}
impl ShapedGlyph {
    pub fn new(font: FontId, glyph: u32, cluster: DecodedSpan, position: GlyphPosition) -> Self {
        Self {
            font,
            glyph,
            cluster,
            position,
        }
    }
    pub const fn font(&self) -> FontId {
        self.font
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
    glyphs: Arc<[ShapedGlyph]>,
    metrics: RunMetrics,
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
            glyphs,
            metrics,
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
    id: EventId,
    source_map: Arc<SourceMappedText>,
    runs: Arc<[ShapedRun]>,
    breaks: Arc<[DecodedSpan]>,
    paints: Arc<[PaintSpan]>,
    missing: Arc<[MissingCluster]>,
    diagnostics: Arc<[Diagnostic]>,
}
impl ShapedEvent {
    pub fn new(
        id: EventId,
        source_map: Arc<SourceMappedText>,
        runs: Arc<[ShapedRun]>,
        breaks: Arc<[DecodedSpan]>,
        paints: Arc<[PaintSpan]>,
        missing: Arc<[MissingCluster]>,
        diagnostics: Arc<[Diagnostic]>,
    ) -> Result<Self, FontError> {
        if runs.iter().any(|r| !r.decoded.belongs_to(&source_map))
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
            id,
            source_map,
            runs,
            breaks,
            paints,
            missing,
            diagnostics,
        })
    }
    pub const fn id(&self) -> EventId {
        self.id
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
