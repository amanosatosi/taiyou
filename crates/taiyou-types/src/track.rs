//! Immutable static ASS records and a renderer-session-owned ingestion coordinator.

use std::sync::Arc;

use crate::{
    Diagnostic, ErrorCode, EventId, FiniteF64, ParseError, Profile, Rgba8, Size, SourceSpan,
    StyleId, TimeUs, TrackRevision,
};

fn invalid(detail: &str) -> ParseError {
    ParseError::new(ErrorCode::InvalidArgument, detail)
}
fn limit(detail: &str) -> ParseError {
    ParseError::new(ErrorCode::LimitExceeded, detail)
}

/// Configurable byte/count caps use u64; dimension/radius/pass caps use u32 (C7).
/// Consuming setters create a new configuration; no published snapshot is mutable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Limits {
    input_bytes: u64,
    events: u64,
    text_bytes_per_event: u64,
    glyphs_per_event: u64,
    path_segments_per_frame: u64,
    tag_nesting: u32,
    max_dimension: u32,
    output_pixels: u64,
    font_bytes: u64,
    cache_bytes: u64,
    transient_bytes: u64,
    output_bytes: u64,
    filter_radius: u32,
    be_passes: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            input_bytes: 16 * 1024 * 1024,
            events: 100_000,
            text_bytes_per_event: 1024 * 1024,
            glyphs_per_event: 100_000,
            path_segments_per_frame: 1_000_000,
            tag_nesting: 8,
            max_dimension: 16_384,
            output_pixels: 32_000_000,
            font_bytes: 64 * 1024 * 1024,
            cache_bytes: 128 * 1024 * 1024,
            transient_bytes: 128 * 1024 * 1024,
            output_bytes: 128 * 1024 * 1024,
            filter_radius: 256,
            be_passes: 64,
        }
    }
}

macro_rules! positive_limit {
    ($get:ident, $with:ident, $ty:ty) => {
        pub const fn $get(&self) -> $ty {
            self.$get
        }
        pub fn $with(mut self, value: $ty) -> Result<Self, ParseError> {
            if value == 0 {
                return Err(invalid(concat!(
                    stringify!($get),
                    " limit must be positive"
                )));
            }
            self.$get = value;
            Ok(self)
        }
    };
}

impl Limits {
    positive_limit!(input_bytes, with_input_bytes, u64);
    positive_limit!(events, with_events, u64);
    positive_limit!(text_bytes_per_event, with_text_bytes_per_event, u64);
    positive_limit!(glyphs_per_event, with_glyphs_per_event, u64);
    positive_limit!(path_segments_per_frame, with_path_segments_per_frame, u64);
    positive_limit!(tag_nesting, with_tag_nesting, u32);
    positive_limit!(max_dimension, with_max_dimension, u32);
    positive_limit!(output_pixels, with_output_pixels, u64);
    positive_limit!(font_bytes, with_font_bytes, u64);
    positive_limit!(transient_bytes, with_transient_bytes, u64);
    positive_limit!(output_bytes, with_output_bytes, u64);

    /// Zero disables retention without disabling uncached work.
    pub fn with_cache_bytes(mut self, value: u64) -> Self {
        self.cache_bytes = value;
        self
    }
    pub const fn cache_bytes(&self) -> u64 {
        self.cache_bytes
    }
    /// Zero permits only zero-radius effects.
    pub fn with_filter_radius(mut self, value: u32) -> Self {
        self.filter_radius = value;
        self
    }
    pub const fn filter_radius(&self) -> u32 {
        self.filter_radius
    }
    /// Zero permits only the no-be path.
    pub fn with_be_passes(mut self, value: u32) -> Self {
        self.be_passes = value;
        self
    }
    pub const fn be_passes(&self) -> u32 {
        self.be_passes
    }

    pub fn check_input_bytes(&self, length: usize) -> Result<(), ParseError> {
        let length =
            u64::try_from(length).map_err(|_| limit("input length conversion overflow"))?;
        if length > self.input_bytes || length > u64::from(u32::MAX) {
            return Err(limit("subtitle input exceeds byte/span limit"));
        }
        Ok(())
    }

    /// Checks a future frame allocation without allocating or rendering anything.
    pub fn check_output_size(&self, size: Size) -> Result<(usize, usize), ParseError> {
        if size.is_empty() {
            return Err(invalid("output dimensions must be positive"));
        }
        if size.width() > self.max_dimension
            || size.height() > self.max_dimension
            || size.pixels() > self.output_pixels
        {
            return Err(limit("output dimensions or pixel count exceed limits"));
        }
        let layout = size.rgba_layout()?;
        let bytes =
            u64::try_from(layout.1).map_err(|_| limit("output length conversion overflow"))?;
        if bytes > self.output_bytes {
            return Err(limit("output byte limit exceeded"));
        }
        Ok(layout)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputEncoding {
    Utf8,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SyntaxPolicy {
    Recover,
    Strict,
}

/// C10 parse configuration. No unreviewed session/revision field is added here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseOptions {
    encoding: InputEncoding,
    profile: Profile,
    limits: Limits,
    syntax: SyntaxPolicy,
}
impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            encoding: InputEncoding::Utf8,
            profile: Profile::CoreV1,
            limits: Limits::default(),
            syntax: SyntaxPolicy::Recover,
        }
    }
}
impl ParseOptions {
    pub fn new(
        encoding: InputEncoding,
        profile: Profile,
        limits: Limits,
        syntax: SyntaxPolicy,
    ) -> Self {
        Self {
            encoding,
            profile,
            limits,
            syntax,
        }
    }
    pub const fn encoding(&self) -> InputEncoding {
        self.encoding
    }
    pub const fn profile(&self) -> Profile {
        self.profile
    }
    pub fn limits(&self) -> &Limits {
        &self.limits
    }
    pub const fn syntax(&self) -> SyntaxPolicy {
        self.syntax
    }
}

/// Alignment grid values 1..9 in ASS bottom/middle/top row order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Alignment(u8);
impl Alignment {
    pub fn new(value: u8) -> Result<Self, ParseError> {
        if !(1..=9).contains(&value) {
            return Err(invalid("alignment must be 1..9"));
        }
        Ok(Self(value))
    }
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Header representation only. Rendering modes other than 2 remain T017-gated.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct WrapMode(u8);
impl WrapMode {
    pub fn new(value: u8) -> Result<Self, ParseError> {
        if value > 3 {
            return Err(invalid("WrapStyle must be 0..3"));
        }
        Ok(Self(value))
    }
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Script-unit margins. Zero event margins inherit; zero style margins mean zero.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Margins {
    left: u32,
    right: u32,
    vertical: u32,
}
impl Margins {
    pub const fn new(left: u32, right: u32, vertical: u32) -> Self {
        Self {
            left,
            right,
            vertical,
        }
    }
    pub const fn left(self) -> u32 {
        self.left
    }
    pub const fn right(self) -> u32 {
        self.right
    }
    pub const fn vertical(self) -> u32 {
        self.vertical
    }
    pub fn inherit(self, style: Self) -> Self {
        Self {
            left: if self.left == 0 {
                style.left
            } else {
                self.left
            },
            right: if self.right == 0 {
                style.right
            } else {
                self.right
            },
            vertical: if self.vertical == 0 {
                style.vertical
            } else {
                self.vertical
            },
        }
    }
}

/// Immutable family request. Native resolution and catalogs belong to T008.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct FontRequest {
    family: Arc<str>,
    weight: u16,
    italic: bool,
    vertical: bool,
}
impl FontRequest {
    pub fn new(family: Arc<str>, weight: u16, italic: bool, vertical: bool) -> Self {
        Self {
            family,
            weight,
            italic,
            vertical,
        }
    }
    pub fn family(&self) -> &str {
        &self.family
    }
    pub const fn weight(&self) -> u16 {
        self.weight
    }
    pub const fn italic(&self) -> bool {
        self.italic
    }
    pub const fn vertical(&self) -> bool {
        self.vertical
    }
}

/// OpaqueBox can be recorded but its rendering remains R001/R002-gated.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum BorderStyle {
    Outline,
    OpaqueBox,
}

/// Static style values in script space; scales are factors (100% = 1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Style {
    name: Arc<str>,
    font: FontRequest,
    font_size: FiniteF64,
    primary: Rgba8,
    secondary: Rgba8,
    outline: Rgba8,
    shadow_color: Rgba8,
    underline: bool,
    strikeout: bool,
    scale_x: FiniteF64,
    scale_y: FiniteF64,
    spacing: FiniteF64,
    angle: FiniteF64,
    border_style: BorderStyle,
    border: FiniteF64,
    shadow: FiniteF64,
    alignment: Alignment,
    margins: Margins,
    encoding: i32,
}
impl Style {
    /// The explicit CoreV1 defaults in C2; never an inferred Legacy default.
    pub fn core_default() -> Self {
        Self {
            name: Arc::from("Default"),
            font: FontRequest::new(Arc::from("Arial"), 400, false, false),
            font_size: FiniteF64::CORE_FONT_SIZE,
            primary: Rgba8::new(255, 255, 255, 255),
            secondary: Rgba8::new(255, 255, 0, 255),
            outline: Rgba8::new(0, 0, 0, 255),
            shadow_color: Rgba8::new(0, 0, 0, 255),
            underline: false,
            strikeout: false,
            scale_x: FiniteF64::ONE,
            scale_y: FiniteF64::ONE,
            spacing: FiniteF64::ZERO,
            angle: FiniteF64::ZERO,
            border_style: BorderStyle::Outline,
            border: FiniteF64::CORE_BORDER,
            shadow: FiniteF64::CORE_BORDER,
            alignment: Alignment(2),
            margins: Margins::new(10, 10, 10),
            encoding: 1,
        }
    }
    pub fn with_name(mut self, name: Arc<str>) -> Self {
        self.name = name;
        self
    }
    pub fn with_font(mut self, font: FontRequest) -> Self {
        self.font = font;
        self
    }
    pub fn with_size(mut self, size: FiniteF64) -> Result<Self, ParseError> {
        if size.get() <= 0.0 {
            return Err(invalid("font size must be positive"));
        }
        self.font_size = size;
        Ok(self)
    }
    pub fn with_scale(mut self, x: FiniteF64, y: FiniteF64) -> Result<Self, ParseError> {
        if x.get() < 0.0 || y.get() < 0.0 {
            return Err(invalid("style scales must be nonnegative"));
        }
        self.scale_x = x;
        self.scale_y = y;
        Ok(self)
    }
    pub fn with_border(mut self, border: FiniteF64) -> Result<Self, ParseError> {
        if border.get() < 0.0 {
            return Err(invalid("style border must be nonnegative"));
        }
        self.border = border;
        Ok(self)
    }
    pub fn with_colors(
        mut self,
        primary: Rgba8,
        secondary: Rgba8,
        outline: Rgba8,
        shadow: Rgba8,
    ) -> Self {
        self.primary = primary;
        self.secondary = secondary;
        self.outline = outline;
        self.shadow_color = shadow;
        self
    }
    pub fn with_decoration(mut self, underline: bool, strikeout: bool) -> Self {
        self.underline = underline;
        self.strikeout = strikeout;
        self
    }
    pub fn with_spacing(mut self, spacing: FiniteF64) -> Self {
        self.spacing = spacing;
        self
    }
    pub fn with_angle(mut self, angle: FiniteF64) -> Self {
        self.angle = angle;
        self
    }
    pub fn with_border_style(mut self, style: BorderStyle) -> Self {
        self.border_style = style;
        self
    }
    pub fn with_shadow(mut self, shadow: FiniteF64) -> Self {
        self.shadow = shadow;
        self
    }
    pub fn with_alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }
    pub fn with_margins(mut self, margins: Margins) -> Self {
        self.margins = margins;
        self
    }
    pub fn with_encoding(mut self, encoding: i32) -> Self {
        self.encoding = encoding;
        self
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn font(&self) -> &FontRequest {
        &self.font
    }
    pub const fn font_size(&self) -> FiniteF64 {
        self.font_size
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
    pub const fn shadow_color(&self) -> Rgba8 {
        self.shadow_color
    }
    pub const fn underline(&self) -> bool {
        self.underline
    }
    pub const fn strikeout(&self) -> bool {
        self.strikeout
    }
    pub const fn scale_x(&self) -> FiniteF64 {
        self.scale_x
    }
    pub const fn scale_y(&self) -> FiniteF64 {
        self.scale_y
    }
    pub const fn spacing(&self) -> FiniteF64 {
        self.spacing
    }
    pub const fn angle(&self) -> FiniteF64 {
        self.angle
    }
    pub const fn border_style(&self) -> BorderStyle {
        self.border_style
    }
    pub const fn border(&self) -> FiniteF64 {
        self.border
    }
    pub const fn shadow(&self) -> FiniteF64 {
        self.shadow
    }
    pub const fn alignment(&self) -> Alignment {
        self.alignment
    }
    pub const fn margins(&self) -> Margins {
        self.margins
    }
    pub const fn encoding(&self) -> i32 {
        self.encoding
    }
}

/// Header metadata is retained as ordered immutable pairs, never executed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScriptInfo {
    play_res: Size,
    layout_res: Option<Size>,
    wrap: WrapMode,
    scaled_border_shadow: bool,
    ycbcr_matrix: Arc<str>,
    collision_header: Option<Arc<str>>,
    metadata: Arc<[(Arc<str>, Arc<str>)]>,
}
impl ScriptInfo {
    /// Already-normalized defaults; T004 diagnoses missing/specified headers.
    pub fn core_default() -> Self {
        Self {
            play_res: Size::new(384, 288),
            layout_res: None,
            wrap: WrapMode(2),
            scaled_border_shadow: false,
            ycbcr_matrix: Arc::from(""),
            collision_header: None,
            metadata: Arc::from([]),
        }
    }
    pub fn with_play_res(mut self, size: Size) -> Result<Self, ParseError> {
        if size.is_empty() {
            return Err(invalid("PlayRes must be positive on both axes"));
        }
        self.play_res = size;
        Ok(self)
    }
    pub fn with_layout_res(mut self, size: Option<Size>) -> Result<Self, ParseError> {
        if size.is_some_and(Size::is_empty) {
            return Err(invalid("LayoutRes must be positive on both axes"));
        }
        self.layout_res = size;
        Ok(self)
    }
    pub fn with_wrap(mut self, wrap: WrapMode) -> Self {
        self.wrap = wrap;
        self
    }
    pub fn with_scaled_border_shadow(mut self, value: bool) -> Self {
        self.scaled_border_shadow = value;
        self
    }
    pub fn with_ycbcr_matrix(mut self, value: Arc<str>) -> Self {
        self.ycbcr_matrix = value;
        self
    }
    pub fn with_collision_header(mut self, value: Option<Arc<str>>) -> Self {
        self.collision_header = value;
        self
    }
    pub fn with_metadata(mut self, value: Arc<[(Arc<str>, Arc<str>)]>) -> Self {
        self.metadata = value;
        self
    }
    pub const fn play_res(&self) -> Size {
        self.play_res
    }
    pub const fn layout_res(&self) -> Option<Size> {
        self.layout_res
    }
    pub const fn wrap(&self) -> WrapMode {
        self.wrap
    }
    pub const fn scaled_border_shadow(&self) -> bool {
        self.scaled_border_shadow
    }
    pub fn ycbcr_matrix(&self) -> &str {
        &self.ycbcr_matrix
    }
    pub fn collision_header(&self) -> Option<&str> {
        self.collision_header.as_deref()
    }
    pub fn metadata(&self) -> &[(Arc<str>, Arc<str>)] {
        &self.metadata
    }
}

/// Ordered text and static Core overrides. Drawing/animation remain unsupported.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TextOp {
    Text { text: Arc<str>, source: SourceSpan },
    HardBreak,
    SoftBreak,
    NoBreakSpace,
    Override(OverrideOp),
    Unsupported { name: Arc<str>, source: SourceSpan },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PaintChannel {
    Primary,
    Secondary,
    Outline,
    Shadow,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ScaleAxis {
    X,
    Y,
}

/// Static subset authorized by T006. Numbers are script-space; scale is a factor.
/// SetPaint changes RGB only (stored alpha must be 255); SetOpacity stores opacity
/// after transparency inversion, with None selecting all channels. Source spans
/// include the original backslash. Publication validates host-created operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OverrideOp {
    SetFont {
        family: Arc<str>,
        source: SourceSpan,
    },
    SetSize {
        size: FiniteF64,
        source: SourceSpan,
    },
    SetWeight {
        weight: u16,
        source: SourceSpan,
    },
    SetItalic {
        italic: bool,
        source: SourceSpan,
    },
    SetScale {
        axis: ScaleAxis,
        factor: FiniteF64,
        source: SourceSpan,
    },
    SetSpacing {
        spacing: FiniteF64,
        source: SourceSpan,
    },
    SetPaint {
        channel: PaintChannel,
        color: Rgba8,
        source: SourceSpan,
    },
    SetOpacity {
        channel: Option<PaintChannel>,
        opacity: u8,
        source: SourceSpan,
    },
    SetAlignment {
        alignment: Alignment,
        source: SourceSpan,
    },
    SetPosition {
        point: crate::ScriptPoint,
        source: SourceSpan,
    },
    ResetStyle {
        name: Option<Arc<str>>,
        source: SourceSpan,
    },
}
impl OverrideOp {
    pub fn source(&self) -> SourceSpan {
        match self {
            Self::SetFont { source, .. }
            | Self::SetSize { source, .. }
            | Self::SetWeight { source, .. }
            | Self::SetItalic { source, .. }
            | Self::SetScale { source, .. }
            | Self::SetSpacing { source, .. }
            | Self::SetPaint { source, .. }
            | Self::SetOpacity { source, .. }
            | Self::SetAlignment { source, .. }
            | Self::SetPosition { source, .. }
            | Self::ResetStyle { source, .. } => *source,
        }
    }
    pub fn validate(&self) -> Result<(), ParseError> {
        let valid = match self {
            Self::SetFont { family, .. } => !family.is_empty(),
            Self::SetSize { size, .. } => size.get() > 0.0,
            Self::SetScale { factor, .. } => factor.get() >= 0.0,
            Self::SetWeight { weight, .. } => matches!(weight, 400 | 700),
            Self::SetPaint { color, .. } => color.a() == 255,
            _ => true,
        };
        if valid {
            Ok(())
        } else {
            Err(invalid("override parameter outside Core bounds"))
        }
    }
}

/// Static event with immutable ordered text operations. Nonpositive intervals are
/// retained but inactive; the ingestion owner publishes the required diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    id: EventId,
    layer: i32,
    start: TimeUs,
    end: TimeUs,
    style: StyleId,
    margins: Margins,
    actor: Arc<str>,
    effect: Arc<str>,
    program: Arc<[TextOp]>,
    source: SourceSpan,
}
impl Event {
    pub fn new(
        id: EventId,
        start: TimeUs,
        end: TimeUs,
        style: StyleId,
        source: SourceSpan,
    ) -> Self {
        Self {
            id,
            layer: 0,
            start,
            end,
            style,
            margins: Margins::new(0, 0, 0),
            actor: Arc::from(""),
            effect: Arc::from(""),
            program: Arc::from([]),
            source,
        }
    }
    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }
    pub fn with_margins(mut self, margins: Margins) -> Self {
        self.margins = margins;
        self
    }
    pub fn with_actor(mut self, actor: Arc<str>) -> Self {
        self.actor = actor;
        self
    }
    pub fn with_effect(mut self, effect: Arc<str>) -> Self {
        self.effect = effect;
        self
    }
    pub fn with_program(mut self, program: Arc<[TextOp]>) -> Self {
        self.program = program;
        self
    }
    pub const fn id(&self) -> EventId {
        self.id
    }
    pub const fn layer(&self) -> i32 {
        self.layer
    }
    pub const fn start(&self) -> TimeUs {
        self.start
    }
    pub const fn end(&self) -> TimeUs {
        self.end
    }
    pub const fn style(&self) -> StyleId {
        self.style
    }
    pub const fn margins(&self) -> Margins {
        self.margins
    }
    pub fn actor(&self) -> &str {
        &self.actor
    }
    pub fn effect(&self) -> &str {
        &self.effect
    }
    pub fn program(&self) -> &[TextOp] {
        &self.program
    }
    pub fn program_storage(&self) -> &Arc<[TextOp]> {
        &self.program
    }
    pub const fn source(&self) -> SourceSpan {
        self.source
    }
    pub fn is_active(&self, now: TimeUs) -> bool {
        self.start <= now && now < self.end
    }
    pub fn has_nonpositive_duration(&self) -> bool {
        self.end <= self.start
    }
}

/// Shared immutable snapshot. Cloning shares all variable-size storage and keeps
/// the same revision; editing produces a distinct snapshot through TrackIngestor.
#[derive(Clone, Debug)]
pub struct Track {
    source: Arc<str>,
    script: ScriptInfo,
    styles: Arc<[Style]>,
    events: Arc<[Event]>,
    revision: TrackRevision,
}
impl Track {
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn source_storage(&self) -> &Arc<str> {
        &self.source
    }
    pub fn script(&self) -> &ScriptInfo {
        &self.script
    }
    pub fn styles(&self) -> &[Style] {
        &self.styles
    }
    pub fn style_storage(&self) -> &Arc<[Style]> {
        &self.styles
    }
    pub fn events(&self) -> &[Event] {
        &self.events
    }
    pub fn event_storage(&self) -> &Arc<[Event]> {
        &self.events
    }
    pub const fn revision(&self) -> TrackRevision {
        self.revision
    }
}

/// One non-cloneable ingestion owner per renderer session. The renderer owns this
/// coordinator and all tracks it admits; revisions start at 0 and never recycle.
/// Independent owners define independent sessions, not a process-global namespace.
/// How the future parse_ass signature receives this owner remains issue #3;
/// no implicit global allocator or ParseOptions field is introduced here.
#[derive(Debug)]
pub struct TrackIngestor {
    next_revision: Option<u64>,
}
impl Default for TrackIngestor {
    fn default() -> Self {
        Self {
            next_revision: Some(0),
        }
    }
}
impl TrackIngestor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates cross-record/source invariants before minting a new identity.
    /// Host-owned Arcs are retained; no unbounded copy of their contents is made.
    pub fn ingest(
        &mut self,
        source: Arc<str>,
        script: ScriptInfo,
        styles: Arc<[Style]>,
        events: Arc<[Event]>,
        limits: &Limits,
    ) -> Result<Arc<Track>, ParseError> {
        limits.check_input_bytes(source.len())?;
        if u64::try_from(events.len()).map_err(|_| limit("event count conversion overflow"))?
            > limits.events
        {
            return Err(limit("event count limit exceeded"));
        }
        if styles.len() > usize::try_from(u32::MAX).unwrap_or(usize::MAX) {
            return Err(limit("style index space exhausted"));
        }
        let mut previous: Option<&Event> = None;
        for event in events.iter() {
            if event.source.slice(&source).is_none() {
                return Err(invalid("event source span is invalid for track"));
            }
            let index = usize::try_from(event.style.index())
                .map_err(|_| invalid("style index conversion overflow"))?;
            if index >= styles.len() {
                return Err(invalid("event style index is outside track styles"));
            }
            if previous
                .is_some_and(|last| last.id >= event.id || last.source.end() > event.source.start())
            {
                return Err(invalid(
                    "event IDs and spans must follow stable source order",
                ));
            }
            let mut text_bytes = 0_u64;
            let mut text_end = event.source.start();
            for operation in event.program.iter() {
                let (text, span) = match operation {
                    TextOp::Text { text, source } => (Some(text), *source),
                    TextOp::Override(op) => {
                        op.validate()?;
                        (None, op.source())
                    }
                    TextOp::Unsupported { source, .. } => (None, *source),
                    TextOp::HardBreak | TextOp::SoftBreak | TextOp::NoBreakSpace => continue,
                };
                if span.start() < text_end
                    || !event.source.contains(span)
                    || span.slice(&source).is_none()
                    || text.is_some_and(|text| span.slice(&source) != Some(text.as_ref()))
                {
                    return Err(invalid(
                        "literal text spans must match track source in source order",
                    ));
                }
                text_end = span.end();
                let length = u64::try_from(text.map_or(0, |text| text.len()))
                    .map_err(|_| limit("text length conversion overflow"))?;
                text_bytes = text_bytes
                    .checked_add(length)
                    .ok_or_else(|| limit("text byte count overflow"))?;
            }
            if text_bytes > limits.text_bytes_per_event {
                return Err(limit("event text byte limit exceeded"));
            }
            previous = Some(event);
        }
        let revision = self
            .next_revision
            .ok_or_else(|| limit("track revision space exhausted"))?;
        self.next_revision = revision.checked_add(1);
        Ok(Arc::new(Track {
            source,
            script,
            styles,
            events,
            revision: TrackRevision(revision),
        }))
    }
}

/// Parser result ownership from C2; diagnostics are ordered at publication by
/// DiagnosticBuffer. The track itself is always an immutable shared snapshot.
#[derive(Clone, Debug)]
pub struct ParsedTrack {
    track: Arc<Track>,
    diagnostics: Vec<Diagnostic>,
}
impl ParsedTrack {
    pub fn new(
        track: Arc<Track>,
        mut diagnostics: crate::DiagnosticBuffer,
    ) -> Result<Self, ParseError> {
        for event in track.events() {
            if event.has_nonpositive_duration() {
                diagnostics.push(Diagnostic::new(
                    crate::DiagnosticCode::InactiveEvent,
                    Some(event.id()),
                    Some(event.source()),
                    "event end is not after start; event is inactive",
                ))?;
            }
        }
        Ok(Self {
            track,
            diagnostics: diagnostics.finish()?,
        })
    }
    pub fn track(&self) -> &Arc<Track> {
        &self.track
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhausted_revision_owner_never_recycles_or_panics() {
        let mut owner = TrackIngestor {
            next_revision: Some(u64::MAX),
        };
        let make = |owner: &mut TrackIngestor| {
            owner.ingest(
                Arc::from(""),
                ScriptInfo::core_default(),
                Arc::from([]),
                Arc::from([]),
                &Limits::default(),
            )
        };
        assert_eq!(make(&mut owner).unwrap().revision().get(), u64::MAX);
        assert_eq!(
            make(&mut owner).unwrap_err().code(),
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            make(&mut owner).unwrap_err().code(),
            ErrorCode::LimitExceeded
        );
    }
}
