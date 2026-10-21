//! Geometry/value records only; no rasterization or composition (C1/C6/C7).

use crate::{Diagnostic, EventId, Limits, RenderError};
use crate::{ErrorCode, FiniteF64, ParseError, Q6};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

/// Script coordinates: top-left origin, x right, y down.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ScriptPoint {
    x: FiniteF64,
    y: FiniteF64,
}

fn render_invalid(detail: &str) -> RenderError {
    RenderError::new(ErrorCode::InvalidArgument, detail)
}

/// Immutable A8 coverage. Padding is initialized zero, bounds are half-open and
/// may have negative origins. Canonical empty masks have zero width/height/stride.
#[derive(Clone, Debug)]
pub struct Mask {
    bounds: PixelRect,
    stride: usize,
    data: Arc<[u8]>,
}
impl Mask {
    pub fn new(bounds: PixelRect, stride: usize, data: Arc<[u8]>) -> Result<Self, RenderError> {
        let width = usize::try_from(bounds.width())
            .map_err(|_| render_invalid("mask width conversion overflow"))?;
        let height = usize::try_from(bounds.height())
            .map_err(|_| render_invalid("mask height conversion overflow"))?;
        if bounds.is_empty() {
            if width != 0 || height != 0 || stride != 0 || !data.is_empty() {
                return Err(render_invalid(
                    "empty mask must have zero dimensions/stride/data",
                ));
            }
        } else {
            if stride < width || stride.checked_mul(height) != Some(data.len()) {
                return Err(render_invalid("mask extent/stride/length mismatch"));
            }
            if data
                .chunks_exact(stride)
                .any(|row| row[width..].iter().any(|b| *b != 0))
            {
                return Err(render_invalid("mask padding must be zero"));
            }
        }
        Ok(Self {
            bounds,
            stride,
            data,
        })
    }
    pub const fn bounds(&self) -> PixelRect {
        self.bounds
    }
    pub const fn stride(&self) -> usize {
        self.stride
    }
    pub fn data(&self) -> &[u8] {
        &self.data
    }
    /// Device sample at an integer coordinate; out of bounds is zero coverage.
    pub fn coverage_at(&self, x: i64, y: i64, offset: PixelOffset) -> u8 {
        let x = x
            .checked_sub(i64::from(offset.x))
            .and_then(|v| v.checked_sub(i64::from(self.bounds.x0())));
        let y = y
            .checked_sub(i64::from(offset.y))
            .and_then(|v| v.checked_sub(i64::from(self.bounds.y0())));
        let (Some(x), Some(y)) = (x, y) else {
            return 0;
        };
        if x < 0
            || y < 0
            || x >= i64::from(self.bounds.width())
            || y >= i64::from(self.bounds.height())
        {
            return 0;
        }
        let index = usize::try_from(y)
            .ok()
            .and_then(|row| row.checked_mul(self.stride))
            .and_then(|row| usize::try_from(x).ok().and_then(|col| row.checked_add(col)));
        index
            .and_then(|index| self.data.get(index))
            .copied()
            .unwrap_or(0)
    }
}

/// Whole-device-pixel translation; fractional coordinates belong to rasterization.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub struct PixelOffset {
    x: i32,
    y: i32,
}
impl PixelOffset {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub const fn x(self) -> i32 {
        self.x
    }
    pub const fn y(self) -> i32 {
        self.y
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Paint {
    Solid(Rgba8),
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PaintSpace {
    GlyphLocal,
    EventLocal,
    Device,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PaintMap {
    Identity,
}

/// Resolved synthetic clip coverage for T013. Fractional rectangular/path clip
/// construction is deferred to its geometry task; no guessed quantization here.
#[derive(Clone, Debug)]
pub enum ResolvedClip {
    None,
    Rectangle { bounds: PixelRect, inverse: bool },
    Intersection(Arc<ResolvedClip>,Arc<ResolvedClip>),
    MaskCoverage {
        mask: Arc<Mask>,
        offset: PixelOffset,
        inverse: bool,
    },
}
impl ResolvedClip {
    pub fn coverage_at(&self, x: i64, y: i64) -> u8 {
        match self {
            Self::None => 255,
            Self::Rectangle { bounds, inverse } => {
                let inside=x>=i64::from(bounds.x0())&&x<i64::from(bounds.x1())&&y>=i64::from(bounds.y0())&&y<i64::from(bounds.y1());
                if inside != *inverse {255} else {0}
            }
            Self::Intersection(a,b) => ((u16::from(a.coverage_at(x,y))*u16::from(b.coverage_at(x,y))+127)/255) as u8,
            Self::MaskCoverage {
                mask,
                offset,
                inverse,
            } => {
                let coverage = mask.coverage_at(x, y, *offset);
                if *inverse { 255 - coverage } else { coverage }
            }
        }
    }
}

/// Lexicographic composition key from C6. Duplicate keys are an internal error.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct DrawOrder {
    layer: i32,
    event: EventId,
    pass: u16,
    group: u32,
}
impl DrawOrder {
    pub const fn new(layer: i32, event: EventId, pass: u16, group: u32) -> Self {
        Self {
            layer,
            event,
            pass,
            group,
        }
    }
    pub const fn event(self) -> EventId {
        self.event
    }
}

/// Immutable ordered solid draw operation; mask/clip Arcs survive their producers.
#[derive(Clone, Debug)]
pub struct DrawOp {
    order: DrawOrder,
    mask: Arc<Mask>,
    offset: PixelOffset,
    paint: Paint,
    paint_space: PaintSpace,
    paint_map: PaintMap,
    clip: Arc<ResolvedClip>,
}
impl DrawOp {
    pub fn new(
        order: DrawOrder,
        mask: Arc<Mask>,
        offset: PixelOffset,
        paint: Paint,
        paint_space: PaintSpace,
        paint_map: PaintMap,
        clip: Arc<ResolvedClip>,
    ) -> Self {
        Self {
            order,
            mask,
            offset,
            paint,
            paint_space,
            paint_map,
            clip,
        }
    }
    pub const fn order(&self) -> DrawOrder {
        self.order
    }
    pub fn mask(&self) -> &Mask {
        &self.mask
    }
    pub const fn offset(&self) -> PixelOffset {
        self.offset
    }
    pub const fn paint(&self) -> Paint {
        self.paint
    }
    pub const fn paint_space(&self) -> PaintSpace {
        self.paint_space
    }
    pub const fn paint_map(&self) -> PaintMap {
        self.paint_map
    }
    pub fn clip(&self) -> &ResolvedClip {
        &self.clip
    }
}

/// Coordinator-owned origin budget. Frames retain its accounting after this
/// owner is destroyed. Reservation requires an exclusive coordinator borrow;
/// immutable frames may be released on other threads. No scheduling decisions.
#[derive(Debug)]
pub struct OutputBudget {
    limit: u64,
    retained: Arc<AtomicU64>,
}
impl OutputBudget {
    pub fn new(limits: &Limits) -> Self {
        Self {
            limit: limits.output_bytes(),
            retained: Arc::new(AtomicU64::new(0)),
        }
    }
    pub const fn limit(&self) -> u64 {
        self.limit
    }
    pub fn retained_bytes(&self) -> u64 {
        self.retained.load(Ordering::Acquire)
    }
    pub fn reserve(&mut self, bytes: u64) -> Result<OutputReservation, RenderError> {
        // Explicit CAS keeps the same checked atomic reservation on the MSRV
        // and pinned compiler (fetch_update was deprecated after the MSRV).
        let mut current = self.retained.load(Ordering::Acquire);
        loop {
            let next = current
                .checked_add(bytes)
                .filter(|next| *next <= self.limit)
                .ok_or_else(|| {
                    RenderError::new(ErrorCode::LimitExceeded, "retained output budget exceeded")
                })?;
            match self.retained.compare_exchange_weak(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }
        Ok(OutputReservation {
            bytes,
            retained: Arc::clone(&self.retained),
        })
    }
}

/// Non-cloneable lease for one pixel allocation. Frame clones share one lease.
#[derive(Debug)]
pub struct OutputReservation {
    bytes: u64,
    retained: Arc<AtomicU64>,
}
impl OutputReservation {
    pub const fn bytes(&self) -> u64 {
        self.bytes
    }
}
impl Drop for OutputReservation {
    fn drop(&mut self) {
        self.retained.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}

/// Owned premultiplied encoded-domain RGBA8, top-down, tightly packed.
/// Pixel storage is not exposed as an Arc: callers retain a Frame clone so the
/// accounting lease always outlives every publicly retained pixel allocation.
#[derive(Clone, Debug)]
pub struct Frame {
    size: Size,
    stride: usize,
    pixels: Arc<[u8]>,
    diagnostics: Arc<[Diagnostic]>,
    lease: Arc<OutputReservation>,
}
impl Frame {
    pub fn from_owned_pixels(
        size: Size,
        pixels: Vec<u8>,
        limits: &Limits,
        reservation: OutputReservation,
    ) -> Result<Self, RenderError> {
        let (stride, length) = limits
            .check_output_size(size)
            .map_err(|e| RenderError::new(e.code(), e.detail()))?;
        if u64::try_from(pixels.capacity())
            .map_or(true, |capacity| capacity > limits.transient_bytes())
        {
            return Err(RenderError::transient_limit(
                "frame publication transient capacity exceeded",
            ));
        }
        if pixels.len() != length || u64::try_from(length).ok() != Some(reservation.bytes()) {
            return Err(render_invalid("frame length/reservation mismatch"));
        }
        if pixels
            .chunks_exact(4)
            .any(|pixel| pixel[..3].iter().any(|c| *c > pixel[3]))
        {
            return Err(render_invalid("frame RGB must be premultiplied by alpha"));
        }
        Ok(Self {
            size,
            stride,
            pixels: pixels.into(),
            diagnostics: Arc::from([]),
            lease: Arc::new(reservation),
        })
    }
    pub const fn size(&self) -> Size {
        self.size
    }
    pub const fn stride(&self) -> usize {
        self.stride
    }
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
    /// Publish bounded canonical diagnostics without copying pixels or changing
    /// their origin-budget lease.
    pub fn with_diagnostics(
        mut self,
        diagnostics: crate::DiagnosticBuffer,
    ) -> Result<Self, RenderError> {
        self.diagnostics = diagnostics
            .finish()
            .map_err(|e| RenderError::new(e.code(), e.detail()))?
            .into();
        Ok(self)
    }
    pub fn charged_bytes(&self) -> u64 {
        self.lease.bytes()
    }
}
impl ScriptPoint {
    pub const fn new(x: FiniteF64, y: FiniteF64) -> Self {
        Self { x, y }
    }
    pub const fn x(self) -> FiniteF64 {
        self.x
    }
    pub const fn y(self) -> FiniteF64 {
        self.y
    }
}

/// Physical output pixel coordinates, y down, with Q6 fractional placement.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct DevicePoint {
    x: Q6,
    y: Q6,
}
impl DevicePoint {
    pub const fn new(x: Q6, y: Q6) -> Self {
        Self { x, y }
    }
    pub const fn x(self) -> Q6 {
        self.x
    }
    pub const fn y(self) -> Q6 {
        self.y
    }
}

/// Dimensions in whole pixels (or script units for ScriptInfo). Empty sizes exist;
/// output requests and script resolutions separately require positive dimensions.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Size {
    width: u32,
    height: u32,
}
impl Size {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
    pub const fn width(self) -> u32 {
        self.width
    }
    pub const fn height(self) -> u32 {
        self.height
    }
    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
    pub fn pixels(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    /// Checked tightly packed RGBA byte stride and total length, without allocation.
    pub fn rgba_layout(self) -> Result<(usize, usize), ParseError> {
        let stride = usize::try_from(self.width)
            .ok()
            .and_then(|w| w.checked_mul(4));
        let height = usize::try_from(self.height).ok();
        match (stride, height) {
            (Some(stride), Some(height)) => stride.checked_mul(height).map(|len| (stride, len)),
            _ => None,
        }
        .ok_or_else(|| {
            ParseError::new(
                ErrorCode::LimitExceeded,
                "RGBA allocation arithmetic overflow",
            )
        })
    }
}

/// Half-open pixel bounds. Negative origins and zero area are valid.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct PixelRect {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}
impl PixelRect {
    pub fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Result<Self, ParseError> {
        if x1 < x0 || y1 < y0 {
            return Err(ParseError::new(
                ErrorCode::InvalidArgument,
                "reversed pixel rectangle",
            ));
        }
        Ok(Self { x0, y0, x1, y1 })
    }
    pub const fn x0(self) -> i32 {
        self.x0
    }
    pub const fn y0(self) -> i32 {
        self.y0
    }
    pub const fn x1(self) -> i32 {
        self.x1
    }
    pub const fn y1(self) -> i32 {
        self.y1
    }
    pub fn width(self) -> u32 {
        self.x1.abs_diff(self.x0)
    }
    pub fn height(self) -> u32 {
        self.y1.abs_diff(self.y0)
    }
    pub const fn is_empty(self) -> bool {
        self.x0 == self.x1 || self.y0 == self.y1
    }
    pub fn contains_pixel(self, x: i32, y: i32) -> bool {
        self.x0 <= x && x < self.x1 && self.y0 <= y && y < self.y1
    }
    pub fn intersects(self, rhs: Self) -> bool {
        !self.is_empty()
            && !rhs.is_empty()
            && self.x0 < rhs.x1
            && rhs.x0 < self.x1
            && self.y0 < rhs.y1
            && rhs.y0 < self.y1
    }
    pub fn checked_translate(self, x: i32, y: i32) -> Result<Self, ParseError> {
        let failure = || {
            ParseError::new(
                ErrorCode::InvalidArgument,
                "pixel rectangle translation overflow",
            )
        };
        Self::new(
            self.x0.checked_add(x).ok_or_else(failure)?,
            self.y0.checked_add(y).ok_or_else(failure)?,
            self.x1.checked_add(x).ok_or_else(failure)?,
            self.y1.checked_add(y).ok_or_else(failure)?,
        )
    }
}

/// Straight RGBA8 paint in explicit channel order. Alpha is opacity, not ASS transparency.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Rgba8 {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}
impl Rgba8 {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    /// Decodes the integer represented by an ASS &HAABBGGRR style token.
    pub fn from_ass(value: u32) -> Self {
        let [r, g, b, transparency] = value.to_le_bytes();
        Self {
            r,
            g,
            b,
            a: 255 - transparency,
        }
    }
    pub const fn r(self) -> u8 {
        self.r
    }
    pub const fn g(self) -> u8 {
        self.g
    }
    pub const fn b(self) -> u8 {
        self.b
    }
    pub const fn a(self) -> u8 {
        self.a
    }
    pub const fn channels(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}
