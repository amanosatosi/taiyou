//! Provisional T007 immutable Core evaluation interfaces; shared review required.
use crate::*;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedPolicy {
    Strict,
    Tolerant,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    Auto,
    Ltr,
    Rtl,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapeStyle {
    font: FontRequest,
    font_size: FiniteF64,
    spacing: FiniteF64,
    encoding: i32,
}
impl ShapeStyle {
    pub fn from_style(s: &Style) -> Self {
        Self {
            font: s.font().clone(),
            font_size: s.font_size(),
            spacing: s.spacing(),
            encoding: s.encoding(),
        }
    }
    pub fn font(&self) -> &FontRequest {
        &self.font
    }
    pub const fn font_size(&self) -> FiniteF64 {
        self.font_size
    }
    pub const fn spacing(&self) -> FiniteF64 {
        self.spacing
    }
    pub const fn encoding(&self) -> i32 {
        self.encoding
    }
    pub const fn direction(&self) -> Direction {
        Direction::Auto
    }
    pub fn language(&self) -> Option<&str> {
        None
    }
    pub fn features(&self) -> &[(u32, u32)] {
        &[]
    }
    pub fn variations(&self) -> &[(u32, FiniteF64)] {
        &[]
    }
    pub const fn metric_policy(&self) -> u32 {
        2
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometryStyle {
    scale_x: FiniteF64,
    scale_y: FiniteF64,
    rotation_z: FiniteF64,
    rotation_x: FiniteF64,
    rotation_y: FiniteF64,
    shear_x: FiniteF64,
    shear_y: FiniteF64,
    underline: bool,
    strikeout: bool,
}
impl GeometryStyle {
    pub fn from_style(s: &Style) -> Self {
        Self {
            scale_x: s.scale_x(),
            scale_y: s.scale_y(),
            rotation_z: s.angle(),
            rotation_x: s.rotation_x(),
            rotation_y: s.rotation_y(),
            shear_x: s.shear_x(),
            shear_y: s.shear_y(),
            underline: s.underline(),
            strikeout: s.strikeout(),
        }
    }
    pub const fn scale_x(&self) -> FiniteF64 {
        self.scale_x
    }
    pub const fn scale_y(&self) -> FiniteF64 {
        self.scale_y
    }
    pub const fn rotation_z(&self) -> FiniteF64 {
        self.rotation_z
    }
    pub const fn rotation_x(&self) -> FiniteF64 {
        self.rotation_x
    }
    pub const fn rotation_y(&self) -> FiniteF64 {
        self.rotation_y
    }
    pub const fn shear_x(&self) -> FiniteF64 {
        self.shear_x
    }
    pub const fn shear_y(&self) -> FiniteF64 {
        self.shear_y
    }
    pub const fn underline(&self) -> bool {
        self.underline
    }
    pub const fn strikeout(&self) -> bool {
        self.strikeout
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectStyle {
    border_style: BorderStyle,
    border: FiniteF64,
    border_y: FiniteF64,
    shadow: FiniteF64,
    shadow_y: FiniteF64,
    blur: FiniteF64,
    be: FiniteF64,
}
impl EffectStyle {
    pub fn from_style(s: &Style) -> Self {
        Self {
            border_style: s.border_style(),
            border: s.border(),
            border_y: s.border_y(),
            shadow: s.shadow(),
            shadow_y: s.shadow_y(),
            blur: s.blur(),
            be: s.be(),
        }
    }
    pub const fn border_style(&self) -> BorderStyle {
        self.border_style
    }
    pub const fn border_x(&self) -> FiniteF64 {
        self.border
    }
    pub const fn border_y(&self) -> FiniteF64 {
        self.border_y
    }
    pub const fn shadow_x(&self) -> FiniteF64 {
        self.shadow
    }
    pub const fn shadow_y(&self) -> FiniteF64 {
        self.shadow_y
    }
    pub const fn blur(&self) -> FiniteF64 {
        self.blur
    }
    pub const fn be(&self) -> FiniteF64 {
        self.be
    }
    pub const fn policy_revision(&self) -> u32 {
        2
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaintSet {
    primary: Rgba8,
    secondary: Rgba8,
    outline: Rgba8,
    shadow: Rgba8,
}
impl PaintSet {
    fn faded(mut self, alpha: i32) -> Self {
        // RTS.cpp CLine::PaintAll uses signed MulDiv, nearest rounding, then
        // BYTE alpha. A nonpositive event transparency leaves paint alone.
        if alpha <= 0 {
            return self;
        }
        let apply = |c: Rgba8| {
            let product = i64::from(c.a()) * (255 - i64::from(alpha));
            let n = if product >= 0 {
                (product + 127) / 255
            } else {
                -((-product + 127) / 255)
            };
            Rgba8::new(c.r(), c.g(), c.b(), n.rem_euclid(256) as u8)
        };
        self.primary = apply(self.primary);
        self.secondary = apply(self.secondary);
        self.outline = apply(self.outline);
        self.shadow = apply(self.shadow);
        self
    }
    pub fn from_style(s: &Style) -> Self {
        Self {
            primary: s.primary(),
            secondary: s.secondary(),
            outline: s.outline(),
            shadow: s.shadow_color(),
        }
    }
    pub const fn primary(&self) -> Rgba8 {
        self.primary
    }
    pub const fn secondary(&self) -> Rgba8 {
        self.secondary
    }
    pub const fn outline(&self) -> Rgba8 {
        self.outline
    }
    pub const fn shadow(&self) -> Rgba8 {
        self.shadow
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Anchor {
    Default,
    Explicit(ScriptPoint),
}
/// An absolute-time ASS move sample. Quantize endpoints after viewport scaling,
/// then truncate interpolation in xy's integer eighth-pixel coordinate system.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MoveSample {
    from: ScriptPoint,
    to: ScriptPoint,
    progress: FiniteF64,
}
impl MoveSample {
    pub fn new(
        from: ScriptPoint,
        to: ScriptPoint,
        progress: FiniteF64,
    ) -> Result<Self, RenderError> {
        if !(0.0..=1.0).contains(&progress.get()) {
            return Err(RenderError::new(
                ErrorCode::InvalidArgument,
                "move progress outside interval",
            ));
        }
        Ok(Self { from, to, progress })
    }
    /// Logical script position for evaluation without a viewport. Native
    /// truncation is deferred until the caller supplies layout scale.
    pub fn script_position(self) -> Result<ScriptPoint, RenderError> {
        let p = self.progress.get();
        let axis = |a, b| {
            FiniteF64::new((1.0 - p) * a + p * b).map_err(|_| {
                RenderError::new(ErrorCode::LimitExceeded, "ASS move coordinate overflow")
            })
        };
        Ok(ScriptPoint::new(
            axis(self.from.x().get(), self.to.x().get())?,
            axis(self.from.y().get(), self.to.y().get())?,
        ))
    }
    pub fn position(self, sx: f64, sy: f64) -> Result<ScriptPoint, RenderError> {
        let from = [
            ass_coordinate(self.from.x().get(), sx)?,
            ass_coordinate(self.from.y().get(), sy)?,
        ];
        let to = [
            ass_coordinate(self.to.x().get(), sx)?,
            ass_coordinate(self.to.y().get(), sy)?,
        ];
        // EF_MOVE bypasses interpolation when the quantized CPoints agree.
        let stationary = from == to;
        let axis = |a: f64, b: f64, scale: f64| {
            let p = self.progress.get();
            let n = if stationary {
                a
            } else {
                ((1.0 - p) * a + p * b).trunc()
            } / (8.0 * scale);
            FiniteF64::new(n).map_err(|_| {
                RenderError::new(ErrorCode::LimitExceeded, "ASS move coordinate overflow")
            })
        };
        Ok(ScriptPoint::new(
            axis(from[0], to[0], sx)?,
            axis(from[1], to[1], sy)?,
        ))
    }
}
fn ass_coordinate(value: f64, scale: f64) -> Result<f64, RenderError> {
    let n = value * scale * 8.0;
    if !scale.is_finite()
        || scale <= 0.0
        || !n.is_finite()
        || n < f64::from(i32::MIN)
        || n > f64::from(i32::MAX)
    {
        return Err(RenderError::new(
            ErrorCode::LimitExceeded,
            "ASS eighth-pixel coordinate exceeds native range",
        ));
    }
    Ok(n.trunc())
}
fn ass_point(point: ScriptPoint, sx: f64, sy: f64) -> Result<ScriptPoint, RenderError> {
    MoveSample::new(point, point, FiniteF64::ZERO)?.position(sx, sy)
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Placement {
    alignment: Alignment,
    margins: Margins,
    anchor: Anchor,
    origin: Option<ScriptPoint>,
    motion: Option<MoveSample>,
}
impl Placement {
    pub fn new(style: &Style, event_margins: Margins, anchor: Anchor) -> Self {
        Self {
            alignment: style.alignment(),
            margins: event_margins.inherit(style.margins()),
            anchor,
            origin: None,
            motion: None,
        }
    }
    pub const fn alignment(&self) -> Alignment {
        self.alignment
    }
    pub const fn margins(&self) -> Margins {
        self.margins
    }
    pub const fn anchor(&self) -> Anchor {
        self.anchor
    }
    pub const fn origin(&self) -> Option<ScriptPoint> {
        self.origin
    }
    pub fn with_origin(mut self, origin: Option<ScriptPoint>) -> Self {
        self.origin = origin;
        self
    }
    pub fn with_motion(mut self, motion: Option<MoveSample>) -> Self {
        self.motion = motion;
        self
    }
    pub fn ass_anchor(&self, sx: f64, sy: f64) -> Result<Option<ScriptPoint>, RenderError> {
        if let Some(motion) = self.motion {
            return motion.position(sx, sy).map(Some);
        }
        match self.anchor {
            Anchor::Explicit(p) => ass_point(p, sx, sy).map(Some),
            Anchor::Default => Ok(None),
        }
    }
    pub fn ass_origin(&self, sx: f64, sy: f64) -> Result<Option<ScriptPoint>, RenderError> {
        self.origin.map(|p| ass_point(p, sx, sy)).transpose()
    }
    pub fn collision_eligible(&self) -> bool {
        self.anchor == Anchor::Default && self.origin.is_none()
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClipSpec {
    None,
    Ass(AssClip),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipRectStep {
    pub coords: [i32; 4],
    pub progress: FiniteF64,
    pub inverse: bool,
}
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AssClip {
    pub rectangles: Arc<[ClipRectStep]>,
    pub vector: Option<(Arc<AssDrawing>, bool)>,
}

/// Independent dependency changes observed during ordered evaluation, not cache
/// keys or previous-frame dirty state. All T007 operations are time-invariant.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChangeClass {
    temporal: bool,
    shape: bool,
    layout: bool,
    geometry: bool,
    effects: bool,
    paint: bool,
    placement: bool,
    clip: bool,
}
impl ChangeClass {
    pub fn between_styles(a: &Style, b: &Style) -> Self {
        let shape = ShapeStyle::from_style(a) != ShapeStyle::from_style(b);
        let scale = a.scale_x() != b.scale_x() || a.scale_y() != b.scale_y();
        Self {
            temporal: false,
            shape,
            layout: shape || scale,
            geometry: shape || GeometryStyle::from_style(a) != GeometryStyle::from_style(b),
            effects: EffectStyle::from_style(a) != EffectStyle::from_style(b),
            paint: PaintSet::from_style(a) != PaintSet::from_style(b),
            placement: a.alignment() != b.alignment() || a.margins() != b.margins(),
            clip: false,
        }
    }
    pub fn with_placement(mut self) -> Self {
        self.placement = true;
        self
    }
    pub fn with_clip(mut self) -> Self {
        self.clip = true;
        self
    }
    pub fn with_animation(mut self) -> Self {
        self.temporal = true;
        self
    }
    pub fn with_event_paint(mut self) -> Self {
        self.paint = true;
        self
    }
    pub fn with_rotation_origin(mut self) -> Self {
        self.geometry = true;
        self.placement = true;
        self
    }
    pub fn union(self, b: Self) -> Self {
        Self {
            temporal: self.temporal || b.temporal,
            shape: self.shape || b.shape,
            layout: self.layout || b.layout,
            geometry: self.geometry || b.geometry,
            effects: self.effects || b.effects,
            paint: self.paint || b.paint,
            placement: self.placement || b.placement,
            clip: self.clip || b.clip,
        }
    }
    pub const fn shape(self) -> bool {
        self.shape
    }
    pub const fn layout(self) -> bool {
        self.layout
    }
    pub const fn geometry(self) -> bool {
        self.geometry
    }
    pub const fn effects(self) -> bool {
        self.effects
    }
    pub const fn paint(self) -> bool {
        self.paint
    }
    pub const fn placement(self) -> bool {
        self.placement
    }
    pub const fn clip(self) -> bool {
        self.clip
    }
    pub const fn is_static(self) -> bool {
        !self.temporal
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct EventOrder {
    layer: i32,
    source_ordinal: u32,
}
impl EventOrder {
    pub const fn new(layer: i32, source_ordinal: u32) -> Self {
        Self {
            layer,
            source_ordinal,
        }
    }
    pub const fn layer(self) -> i32 {
        self.layer
    }
    pub const fn source_ordinal(self) -> u32 {
        self.source_ordinal
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluatedRun {
    karaoke: Option<Karaoke>,
    drawing: Option<Arc<AssDrawing>>,
    text: Arc<str>,
    source: SourceSpan,
    shaping: ShapeStyle,
    geometry: GeometryStyle,
    effects: EffectStyle,
    paint: PaintSet,
    decoded: Option<DecodedSpan>,
}
impl EvaluatedRun {
    pub fn with_event_fade(mut self, alpha: i32) -> Self {
        self.paint = self.paint.faded(alpha);
        self
    }
    /// Literal spans are exact. Decoded C2 unit escapes use the containing event
    /// span, never invented offsets into decoded text. T009 must preserve that map.
    pub fn new(text: Arc<str>, source: SourceSpan, style: &Style) -> Self {
        Self {
            karaoke: None,
            drawing: None,
            text,
            source,
            shaping: ShapeStyle::from_style(style),
            geometry: GeometryStyle::from_style(style),
            effects: EffectStyle::from_style(style),
            paint: PaintSet::from_style(style),
            decoded: None,
        }
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn with_karaoke(mut self, karaoke: Option<Karaoke>) -> Self {
        self.karaoke = karaoke;
        self
    }
    pub const fn karaoke(&self) -> Option<Karaoke> {
        self.karaoke
    }
    pub fn with_drawing(mut self, drawing: Arc<AssDrawing>) -> Self {
        self.drawing = Some(drawing);
        self
    }
    pub fn drawing(&self) -> Option<&Arc<AssDrawing>> {
        self.drawing.as_ref()
    }
    pub fn with_decoded(mut self, decoded: DecodedSpan) -> Result<Self, RenderError> {
        if decoded.text() != self.text.as_ref() {
            return Err(RenderError::new(
                ErrorCode::InvalidArgument,
                "decoded range differs from evaluated text",
            ));
        }
        self.decoded = Some(decoded);
        Ok(self)
    }
    pub fn decoded(&self) -> Option<&DecodedSpan> {
        self.decoded.as_ref()
    }
    pub const fn source(&self) -> SourceSpan {
        self.source
    }
    pub fn shaping(&self) -> &ShapeStyle {
        &self.shaping
    }
    pub fn geometry(&self) -> &GeometryStyle {
        &self.geometry
    }
    pub fn effects(&self) -> &EffectStyle {
        &self.effects
    }
    pub fn paint(&self) -> &PaintSet {
        &self.paint
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Karaoke {
    pub kind: KaraokeTag,
    pub start_ms: i32,
    pub end_ms: i32,
    pub now_ms: i64,
    pub word: u32,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluatedEvent {
    ass_clip: AssClip,
    wrap_mode: WrapMode,
    warp_mode: WarpMode,
    warp_mode_source: Option<SourceSpan>,
    terminal_style: Option<Style>,
    id: EventId,
    order: EventOrder,
    runs: Arc<[EvaluatedRun]>,
    placement: Placement,
    animation: ChangeClass,
    source_map: Option<Arc<SourceMappedText>>,
}
impl EvaluatedEvent {
    pub fn new(
        id: EventId,
        order: EventOrder,
        runs: Arc<[EvaluatedRun]>,
        placement: Placement,
        animation: ChangeClass,
    ) -> Self {
        Self {
            ass_clip: AssClip::default(),
            terminal_style: None,
            wrap_mode: WrapMode::new(2).expect("constant wrap"),
            warp_mode: WarpMode::XyStyle,
            warp_mode_source: None,
            id,
            order,
            runs,
            placement,
            animation,
            source_map: None,
        }
    }
    pub const fn id(&self) -> EventId {
        self.id
    }
    pub fn with_ass_clip(mut self, clip: AssClip) -> Self {
        self.ass_clip = clip;
        self
    }
    pub fn ass_clip(&self) -> &AssClip {
        &self.ass_clip
    }
    pub const fn warp_mode(&self) -> WarpMode {
        self.warp_mode
    }
    pub const fn wrap_mode(&self) -> WrapMode {
        self.wrap_mode
    }
    pub fn with_wrap_mode(mut self, mode: WrapMode) -> Self {
        self.wrap_mode = mode;
        self
    }
    pub const fn warp_mode_source(&self) -> Option<SourceSpan> {
        self.warp_mode_source
    }
    pub fn with_warp_mode(mut self, mode: WarpMode, source: Option<SourceSpan>) -> Self {
        self.warp_mode = mode;
        self.warp_mode_source = source;
        self
    }
    pub fn with_terminal_style(mut self, style: Style) -> Self {
        self.terminal_style = Some(style);
        self
    }
    pub fn terminal_style(&self) -> Option<&Style> {
        self.terminal_style.as_ref()
    }
    pub fn source_map(&self) -> Option<&Arc<SourceMappedText>> {
        self.source_map.as_ref()
    }
    pub fn with_source_map(mut self, map: Arc<SourceMappedText>) -> Result<Self, RenderError> {
        let mut end = 0;
        for run in self.runs.iter() {
            let span = run.decoded().ok_or_else(|| {
                RenderError::new(
                    ErrorCode::InvalidArgument,
                    "evaluated run has no decoded provenance",
                )
            })?;
            if !span.belongs_to(&map) || span.start() != end || span.text() != run.text() {
                return Err(RenderError::new(
                    ErrorCode::InvalidArgument,
                    "evaluated range belongs to another map or is out of order",
                ));
            }
            end = span.end();
        }
        if end as usize != map.text().len() {
            return Err(RenderError::new(
                ErrorCode::InvalidArgument,
                "evaluated ranges do not cover decoded source",
            ));
        }
        self.source_map = Some(map);
        Ok(self)
    }
    pub const fn order(&self) -> EventOrder {
        self.order
    }
    pub fn runs(&self) -> &[EvaluatedRun] {
        &self.runs
    }
    pub fn placement(&self) -> &Placement {
        &self.placement
    }
    pub fn clip(&self) -> ClipSpec {
        if self.ass_clip.rectangles.is_empty() && self.ass_clip.vector.is_none() {
            ClipSpec::None
        } else {
            ClipSpec::Ass(self.ass_clip.clone())
        }
    }
    pub const fn animation(&self) -> ChangeClass {
        self.animation
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evaluation {
    events: Arc<[EvaluatedEvent]>,
    diagnostics: Arc<[Diagnostic]>,
}
impl Evaluation {
    pub fn new(events: Arc<[EvaluatedEvent]>, diagnostics: Arc<[Diagnostic]>) -> Self {
        Self {
            events,
            diagnostics,
        }
    }
    pub fn events(&self) -> &[EvaluatedEvent] {
        &self.events
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}
