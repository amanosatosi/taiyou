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
        1
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometryStyle {
    scale_x: FiniteF64,
    scale_y: FiniteF64,
    rotation_z: FiniteF64,
    underline: bool,
    strikeout: bool,
}
impl GeometryStyle {
    pub fn from_style(s: &Style) -> Self {
        Self {
            scale_x: s.scale_x(),
            scale_y: s.scale_y(),
            rotation_z: s.angle(),
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
    shadow: FiniteF64,
}
impl EffectStyle {
    pub fn from_style(s: &Style) -> Self {
        Self {
            border_style: s.border_style(),
            border: s.border(),
            shadow: s.shadow(),
        }
    }
    pub const fn border_style(&self) -> BorderStyle {
        self.border_style
    }
    pub const fn border_x(&self) -> FiniteF64 {
        self.border
    }
    pub const fn border_y(&self) -> FiniteF64 {
        self.border
    }
    pub const fn shadow_x(&self) -> FiniteF64 {
        self.shadow
    }
    pub const fn shadow_y(&self) -> FiniteF64 {
        self.shadow
    }
    pub const fn blur(&self) -> FiniteF64 {
        FiniteF64::ZERO
    }
    pub const fn be(&self) -> u32 {
        0
    }
    pub const fn policy_revision(&self) -> u32 {
        1
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Placement {
    alignment: Alignment,
    margins: Margins,
    anchor: Anchor,
}
impl Placement {
    pub fn new(style: &Style, event_margins: Margins, anchor: Anchor) -> Self {
        Self {
            alignment: style.alignment(),
            margins: event_margins.inherit(style.margins()),
            anchor,
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
        None
    }
    pub fn collision_eligible(&self) -> bool {
        self.anchor == Anchor::Default
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClipSpec {
    None,
}

/// Independent dependency changes observed during ordered evaluation, not cache
/// keys or previous-frame dirty state. All T007 operations are time-invariant.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChangeClass {
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
    pub fn union(self, b: Self) -> Self {
        Self {
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
        true
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
    text: Arc<str>,
    source: SourceSpan,
    shaping: ShapeStyle,
    geometry: GeometryStyle,
    effects: EffectStyle,
    paint: PaintSet,
    decoded: Option<DecodedSpan>,
}
impl EvaluatedRun {
    /// Literal spans are exact. Decoded C2 unit escapes use the containing event
    /// span, never invented offsets into decoded text. T009 must preserve that map.
    pub fn new(text: Arc<str>, source: SourceSpan, style: &Style) -> Self {
        Self {
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluatedEvent {
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
    pub const fn clip(&self) -> ClipSpec {
        ClipSpec::None
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
