//! Checked arithmetic and session/source identities (contract C1).

use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

use crate::{ErrorCode, ParseError};

fn invalid(detail: &str) -> ParseError {
    ParseError::new(ErrorCode::InvalidArgument, detail)
}

/// Absolute signed microseconds; negative render-request times are valid.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TimeUs(i64);

impl TimeUs {
    pub const fn from_micros(value: i64) -> Self {
        Self(value)
    }

    pub const fn as_micros(self) -> i64 {
        self.0
    }

    pub fn from_millis(value: i64) -> Result<Self, ParseError> {
        value
            .checked_mul(1_000)
            .map(Self)
            .ok_or_else(|| invalid("millisecond conversion overflow"))
    }

    pub fn from_centis(value: i64) -> Result<Self, ParseError> {
        value
            .checked_mul(10_000)
            .map(Self)
            .ok_or_else(|| invalid("centisecond conversion overflow"))
    }

    /// Converts ASS timestamp components, without parsing or accepting malformed grammar.
    pub fn from_ass_components(
        hours: u64,
        minutes: u8,
        seconds: u8,
        centis: u8,
    ) -> Result<Self, ParseError> {
        if minutes >= 60 || seconds >= 60 || centis >= 100 {
            return Err(invalid("ASS timestamp component out of range"));
        }
        let value = hours
            .checked_mul(3_600)
            .and_then(|v| v.checked_add(u64::from(minutes) * 60))
            .and_then(|v| v.checked_add(u64::from(seconds)))
            .and_then(|v| v.checked_mul(100))
            .and_then(|v| v.checked_add(u64::from(centis)))
            .and_then(|v| v.checked_mul(10_000))
            .and_then(|v| i64::try_from(v).ok());
        value
            .map(Self)
            .ok_or_else(|| invalid("ASS timestamp overflow"))
    }

    pub fn checked_add(self, other: Self) -> Result<Self, ParseError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or_else(|| invalid("time addition overflow"))
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, ParseError> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or_else(|| invalid("time subtraction overflow"))
    }
}

/// Finite f64 with canonical positive zero, suitable for exact canonical keys.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FiniteF64(f64);

impl FiniteF64 {
    pub const ZERO: Self = Self(0.0);
    pub const ONE: Self = Self(1.0);

    pub(crate) const CORE_FONT_SIZE: Self = Self(20.0);
    pub(crate) const CORE_BORDER: Self = Self(2.0);

    pub fn new(value: f64) -> Result<Self, ParseError> {
        if !value.is_finite() {
            return Err(invalid("number must be finite"));
        }
        Ok(Self(if value == 0.0 { 0.0 } else { value }))
    }

    pub const fn get(self) -> f64 {
        self.0
    }

    pub const fn bits(self) -> u64 {
        self.0.to_bits()
    }

    pub fn checked_add(self, rhs: Self) -> Result<Self, ParseError> {
        Self::new(self.0 + rhs.0)
    }

    pub fn checked_sub(self, rhs: Self) -> Result<Self, ParseError> {
        Self::new(self.0 - rhs.0)
    }

    pub fn checked_mul(self, rhs: Self) -> Result<Self, ParseError> {
        Self::new(self.0 * rhs.0)
    }

    pub fn checked_div(self, rhs: Self) -> Result<Self, ParseError> {
        Self::new(self.0 / rhs.0)
    }
}

impl Eq for FiniteF64 {}

impl PartialOrd for FiniteF64 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FiniteF64 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Hash for FiniteF64 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bits().hash(state);
    }
}

/// Signed device coordinate in i32 ticks, exactly 64 ticks per pixel.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Q6(i32);

impl Q6 {
    pub const fn from_ticks(ticks: i32) -> Self {
        Self(ticks)
    }

    pub const fn ticks(self) -> i32 {
        self.0
    }

    /// Quantizes once: nearest tick, ties away from zero, checked before narrowing.
    pub fn from_pixels(pixels: FiniteF64) -> Result<Self, ParseError> {
        let rounded = (pixels.get() * 64.0).round();
        if !rounded.is_finite() || rounded < f64::from(i32::MIN) || rounded > f64::from(i32::MAX) {
            return Err(invalid("Q6 coordinate overflow"));
        }
        // The finite rounded value was checked against i32 bounds above. Rust has
        // no TryFrom<f64> for integers; this cast to i64 is exact in that range.
        let ticks = i32::try_from(rounded as i64).map_err(|_| invalid("Q6 coordinate overflow"))?;
        Ok(Self(ticks))
    }

    pub fn as_pixels(self) -> FiniteF64 {
        FiniteF64(f64::from(self.0) / 64.0)
    }

    /// Floor offset and phase 0..63. The phase stays i32 to avoid narrowing.
    pub fn split(self) -> (i32, i32) {
        (self.0.div_euclid(64), self.0.rem_euclid(64))
    }

    pub fn checked_add(self, rhs: Self) -> Result<Self, ParseError> {
        Self::narrow(i64::from(self.0) + i64::from(rhs.0))
    }

    pub fn checked_sub(self, rhs: Self) -> Result<Self, ParseError> {
        Self::narrow(i64::from(self.0) - i64::from(rhs.0))
    }

    pub fn checked_mul(self, rhs: i32) -> Result<Self, ParseError> {
        Self::narrow(i64::from(self.0) * i64::from(rhs))
    }

    pub fn checked_neg(self) -> Result<Self, ParseError> {
        Self::narrow(-i64::from(self.0))
    }

    fn narrow(ticks: i64) -> Result<Self, ParseError> {
        i32::try_from(ticks)
            .map(Self)
            .map_err(|_| invalid("Q6 arithmetic overflow"))
    }
}

/// Half-open UTF-8 byte span, checked against source at construction/publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SourceSpan {
    start: u32,
    end: u32,
}

impl SourceSpan {
    pub fn new(source: &str, start: usize, end: usize) -> Result<Self, ParseError> {
        if start > end || source.get(start..end).is_none() {
            return Err(invalid("source span is not a valid UTF-8 byte range"));
        }
        Ok(Self {
            start: u32::try_from(start).map_err(|_| invalid("source span exceeds u32"))?,
            end: u32::try_from(end).map_err(|_| invalid("source span exceeds u32"))?,
        })
    }

    pub const fn start(self) -> u32 {
        self.start
    }
    pub const fn end(self) -> u32 {
        self.end
    }
    pub const fn len(self) -> u32 {
        self.end - self.start
    }
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub fn slice(self, source: &str) -> Option<&str> {
        let start = usize::try_from(self.start).ok()?;
        let end = usize::try_from(self.end).ok()?;
        source.get(start..end)
    }

    pub fn contains(self, other: Self) -> bool {
        self.start <= other.start && other.end <= self.end
    }
}

/// Stable source-record ordinal. Track publication checks ordering and uniqueness.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EventId(u32);
impl EventId {
    pub const fn new(ordinal: u32) -> Self {
        Self(ordinal)
    }
    pub const fn ordinal(self) -> u32 {
        self.0
    }
}

/// Index into the immutable style-definition array; checked at Track publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StyleId(u32);
impl StyleId {
    pub const fn new(index: u32) -> Self {
        Self(index)
    }
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Session identity minted only by the renderer-owned TrackIngestor.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TrackRevision(pub(crate) u64);
impl TrackRevision {
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Catalog snapshot identity; allocation belongs to the future catalog owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CatalogRevision(pub(crate) u64);
impl CatalogRevision {
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Font content SHA-256 and collection face index, never a native pointer.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FontId {
    sha256: [u8; 32],
    face_index: u32,
}
impl FontId {
    pub const fn new(sha256: [u8; 32], face_index: u32) -> Self {
        Self { sha256, face_index }
    }
    pub const fn sha256(&self) -> &[u8; 32] {
        &self.sha256
    }
    pub const fn face_index(self) -> u32 {
        self.face_index
    }
}

/// LegacyXyV1 is representable, but enabling its behavior remains R001-gated.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Profile {
    CoreV1,
    LegacyXyV1,
}
