//! Owned local script-space geometry. No viewport placement or raster offsets.
use crate::*;
use std::{ops::Range, sync::Arc};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScriptBounds { x0: FiniteF64, y0: FiniteF64, x1: FiniteF64, y1: FiniteF64 }
impl ScriptBounds {
    pub fn new(x0: f64, y0: f64, x1: f64, y1: f64) -> Result<Self, RenderError> {
        let finite = |v| FiniteF64::new(v).map_err(|_| RenderError::new(ErrorCode::LimitExceeded, "nonfinite layout geometry"));
        let result = Self { x0: finite(x0)?, y0: finite(y0)?, x1: finite(x1)?, y1: finite(y1)? };
        if x0 > x1 || y0 > y1 { return Err(RenderError::new(ErrorCode::InvalidArgument, "inverted script bounds")); }
        Ok(result)
    }
    pub const fn x0(self) -> FiniteF64 { self.x0 }
    pub const fn y0(self) -> FiniteF64 { self.y0 }
    pub const fn x1(self) -> FiniteF64 { self.x1 }
    pub const fn y1(self) -> FiniteF64 { self.y1 }
    pub fn width(self) -> f64 { self.x1.get() - self.x0.get() }
    pub fn height(self) -> f64 { self.y1.get() - self.y0.get() }
    pub fn union(self, b: Self) -> Result<Self, RenderError> {
        Self::new(self.x0.get().min(b.x0.get()), self.y0.get().min(b.y0.get()), self.x1.get().max(b.x1.get()), self.y1.get().max(b.y1.get()))
    }
    pub fn translated(self, x: f64, y: f64) -> Result<Self, RenderError> {
        Self::new(self.x0.get()+x, self.y0.get()+y, self.x1.get()+x, self.y1.get()+y)
    }
    pub fn scaled(self, x: f64, y: f64) -> Result<Self, RenderError> {
        if x < 0.0 || y < 0.0 { return Err(RenderError::new(ErrorCode::InvalidArgument, "negative local scale")); }
        Self::new(self.x0.get()*x, self.y0.get()*y, self.x1.get()*x, self.y1.get()*y)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GlyphInk { Unknown, Empty, Bounds(ScriptBounds) }

/// Actual font metrics for an empty decoded line; never font-size estimates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmptyLineMetrics { pub decoded: DecodedSpan, pub font: FontId, pub metrics: RunMetrics, pub scale_y: FiniteF64 }

#[derive(Clone, Debug)]
pub struct LayoutConstraints {
    width: FiniteF64, wrap: WrapMode, alignment: Alignment, profile: Profile,
    scale_x: FiniteF64, scale_y: FiniteF64, limits: Limits,
}
impl LayoutConstraints {
    pub fn new(alignment: Alignment) -> Self {
        Self { width: FiniteF64::ZERO, wrap: WrapMode::new(2).expect("constant wrap"), alignment, profile: Profile::CoreV1, scale_x: FiniteF64::new(1.0).expect("constant scale"), scale_y: FiniteF64::new(1.0).expect("constant scale"), limits: Limits::default() }
    }
    pub fn with_width(mut self, width: FiniteF64) -> Result<Self, RenderError> {
        if width.get() < 0.0 { return Err(RenderError::new(ErrorCode::InvalidArgument, "negative available width")); } self.width=width; Ok(self)
    }
    pub fn with_scale(mut self, x: FiniteF64, y: FiniteF64) -> Result<Self, RenderError> {
        if x.get() < 0.0 || y.get() < 0.0 { return Err(RenderError::new(ErrorCode::InvalidArgument, "negative scale")); } self.scale_x=x; self.scale_y=y; Ok(self)
    }
    pub fn with_wrap(mut self, wrap: WrapMode) -> Self { self.wrap=wrap; self }
    pub fn with_profile(mut self, profile: Profile) -> Self { self.profile=profile; self }
    pub fn with_limits(mut self, limits: Limits) -> Self { self.limits=limits; self }
    pub const fn alignment(&self) -> Alignment { self.alignment }
    pub const fn available_width(&self) -> FiniteF64 { self.width }
    pub const fn wrap(&self) -> WrapMode { self.wrap }
    pub const fn profile(&self) -> Profile { self.profile }
    pub const fn scale_x(&self) -> FiniteF64 { self.scale_x }
    pub const fn scale_y(&self) -> FiniteF64 { self.scale_y }
    pub fn limits(&self) -> &Limits { &self.limits }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Line {
    pub(crate) decoded: DecodedSpan, pub(crate) baseline: ScriptPoint,
    pub(crate) ascent: FiniteF64, pub(crate) descent: FiniteF64, pub(crate) advance: FiniteF64,
    pub(crate) logical: ScriptBounds, pub(crate) ink: Option<ScriptBounds>, pub(crate) glyphs: Range<usize>,
}
impl Line {
    pub fn new(decoded: DecodedSpan, baseline: ScriptPoint, metrics: (FiniteF64, FiniteF64, FiniteF64), logical: ScriptBounds, ink: Option<ScriptBounds>, glyphs: Range<usize>) -> Result<Self, RenderError> {
        if metrics.0.get()<0.0 || metrics.1.get()<0.0 || metrics.2.get()<0.0 || glyphs.start>glyphs.end { return Err(RenderError::new(ErrorCode::InvalidArgument, "invalid line")); }
        Ok(Self { decoded, baseline, ascent: metrics.0, descent: metrics.1, advance: metrics.2, logical, ink, glyphs })
    }
    pub fn decoded(&self) -> &DecodedSpan { &self.decoded }
    pub const fn baseline(&self) -> ScriptPoint { self.baseline }
    pub const fn ascent(&self) -> FiniteF64 { self.ascent }
    pub const fn descent(&self) -> FiniteF64 { self.descent }
    pub const fn advance(&self) -> FiniteF64 { self.advance }
    pub const fn logical_bounds(&self) -> ScriptBounds { self.logical }
    pub const fn ink_bounds(&self) -> Option<ScriptBounds> { self.ink }
    pub fn glyph_range(&self) -> Range<usize> { self.glyphs.clone() }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositionedGlyph {
    run: usize, glyph: usize, position: ScriptPoint, cluster: DecodedSpan, association: usize,
}
impl PositionedGlyph {
    pub fn new(run: usize, glyph: usize, position: ScriptPoint, cluster: DecodedSpan, association: usize) -> Self { Self { run, glyph, position, cluster, association } }
    pub const fn run_index(&self) -> usize { self.run }
    pub const fn glyph_index(&self) -> usize { self.glyph }
    pub const fn position(&self) -> ScriptPoint { self.position }
    pub fn cluster(&self) -> &DecodedSpan { &self.cluster }
    pub const fn association_index(&self) -> usize { self.association }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalLayout {
    input: Arc<ShapedEvent>, lines: Arc<[Line]>, glyphs: Arc<[PositionedGlyph]>,
    logical: ScriptBounds, ink: Option<ScriptBounds>, collision: ScriptBounds,
}
impl LocalLayout {
    pub fn new(input: Arc<ShapedEvent>, lines: Arc<[Line]>, glyphs: Arc<[PositionedGlyph]>, logical: ScriptBounds, ink: Option<ScriptBounds>, collision: ScriptBounds) -> Result<Self, RenderError> {
        let invalid=|| RenderError::new(ErrorCode::InvalidArgument, "invalid local layout publication");
        let mut end=0;
        for line in lines.iter() {
            if !line.decoded.belongs_to(input.source_map()) || line.glyphs.start!=end || line.glyphs.end>glyphs.len() { return Err(invalid()); } end=line.glyphs.end;
        }
        if end!=glyphs.len() { return Err(invalid()); }
        for g in glyphs.iter() {
            let actual=input.runs().get(g.run).and_then(|r| r.glyphs().get(g.glyph)).ok_or_else(invalid)?;
            let association=input.evaluated().runs().get(g.association).and_then(|r|r.decoded()).ok_or_else(invalid)?;
            if !g.cluster.belongs_to(input.source_map()) || g.cluster!=*actual.cluster() || g.cluster.start()<association.start() || g.cluster.start()>=association.end() { return Err(invalid()); }
        }
        Ok(Self { input, lines, glyphs, logical, ink, collision })
    }
    pub fn shaped(&self) -> &ShapedEvent { &self.input }
    pub fn lines(&self) -> &[Line] { &self.lines }
    pub fn glyphs(&self) -> &[PositionedGlyph] { &self.glyphs }
    pub const fn logical_bounds(&self) -> ScriptBounds { self.logical }
    pub const fn ink_bounds(&self) -> Option<ScriptBounds> { self.ink }
    pub const fn collision_bounds(&self) -> ScriptBounds { self.collision }
    /// Geometry comparison only, explicitly not a cache key or source identity.
    pub fn geometry_eq(&self, b: &Self) -> bool {
        self.logical==b.logical && self.ink==b.ink && self.collision==b.collision
            && self.lines.len()==b.lines.len() && self.glyphs.len()==b.glyphs.len()
            && self.lines.iter().zip(b.lines.iter()).all(|(a,b)| a.baseline==b.baseline && a.ascent==b.ascent && a.descent==b.descent && a.advance==b.advance && a.logical==b.logical && a.ink==b.ink && a.glyphs==b.glyphs && (a.decoded.start(),a.decoded.end())==(b.decoded.start(),b.decoded.end()))
            && self.glyphs.iter().zip(b.glyphs.iter()).all(|(a,g)| {
                let x=&self.input.runs()[a.run].glyphs()[a.glyph]; let y=&b.input.runs()[g.run].glyphs()[g.glyph];
                a.position==g.position && x.font()==y.font() && x.glyph()==y.glyph() && (a.cluster.start(),a.cluster.end())==(g.cluster.start(),g.cluster.end())
            })
    }
}
