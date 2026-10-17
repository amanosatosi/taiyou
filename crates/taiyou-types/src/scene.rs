//! Geometry/value records only; no rasterization or composition (C1/C6/C7).

use crate::{ErrorCode, FiniteF64, ParseError, Q6};

/// Script coordinates: top-left origin, x right, y down.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ScriptPoint {
    x: FiniteF64,
    y: FiniteF64,
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
