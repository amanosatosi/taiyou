//! Provisional C4 source-map prerequisite, independent of glyph shaping.
//!
//! Pieces are explicit original-source intervals, never offsets inferred from
//! T007's coarse event spans. The parser/evaluator bridge still requires review.
use std::sync::Arc;
use crate::{ErrorCode, FontError, Limits, SourceSpan, WrapMode};

/// Exact source supplied by a producer that knows each token's provenance.
/// Literal bytes are copied verbatim; escapes must name their original two bytes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextSourcePiece {
    Literal(SourceSpan),
    HardBreak(SourceSpan),
    SoftBreak(SourceSpan),
    NoBreakSpace(SourceSpan),
}
impl TextSourcePiece {
    fn source(self) -> SourceSpan {
        match self {
            Self::Literal(s) | Self::HardBreak(s) | Self::SoftBreak(s) | Self::NoBreakSpace(s) => s,
        }
    }
    fn decoded(self, source: &str, wrap: WrapMode) -> Result<&str, FontError> {
        let raw = self
            .source()
            .slice(source)
            .ok_or_else(|| invalid("source interval is invalid"))?;
        match self {
            Self::Literal(_) => Ok(raw),
            Self::HardBreak(_) if raw == "\\N" => Ok("\n"),
            Self::SoftBreak(_) if raw == "\\n" => Ok(if wrap.get() == 2 { "\n" } else { " " }),
            Self::NoBreakSpace(_) if raw == "\\h" => Ok("\u{a0}"),
            _ => Err(invalid("escape requires its exact original source bytes")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Interval {
    decoded: SourceSpan,
    original: SourceSpan,
    literal: bool,
}

/// Immutable decoded text plus an ordered interval map into retained source.
/// Construction builds the text from verified pieces, so callers cannot attach
/// arbitrary decoded text to an unrelated range. It does not validate ASS grammar
/// or recover missing spans from an existing host-created TextOp program.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceMappedText {
    original: Arc<str>,
    text: Arc<str>,
    intervals: Arc<[Interval]>,
}
fn invalid(detail: &str) -> FontError {
    FontError::new(ErrorCode::InvalidArgument, detail)
}
fn limit() -> FontError {
    FontError::new(
        ErrorCode::LimitExceeded,
        "source-map byte/count budget exceeded",
    )
}
fn allocation() -> FontError {
    FontError::new(ErrorCode::AllocationFailed, "source-map allocation failed")
}
impl SourceMappedText {
    pub fn decoded_span(self: &Arc<Self>, range: std::ops::Range<u32>) -> Result<DecodedSpan, FontError> {
        let start = usize::try_from(range.start).map_err(|_| invalid("decoded range overflow"))?;
        let end = usize::try_from(range.end).map_err(|_| invalid("decoded range overflow"))?;
        if self.text.get(start..end).is_none() {
            return Err(invalid("decoded span is not a UTF-8 range in its owning map"));
        }
        Ok(DecodedSpan { map: self.clone(), start: range.start, end: range.end })
    }
    pub fn from_pieces(
        original: Arc<str>,
        pieces: &[TextSourcePiece],
        wrap: WrapMode,
        limits: &Limits,
    ) -> Result<Self, FontError> {
        let source_len = u64::try_from(original.len()).map_err(|_| limit())?;
        let count = u64::try_from(pieces.len()).map_err(|_| limit())?;
        if source_len > limits.input_bytes()
            || source_len > u64::from(u32::MAX)
            || count > limits.text_bytes_per_event()
        {
            return Err(limit());
        }
        let mut length = 0_usize;
        let mut previous_end = 0;
        for piece in pieces {
            let span = piece.source();
            if span.start() < previous_end {
                return Err(invalid("source pieces overlap or are out of order"));
            }
            let decoded = piece.decoded(&original, wrap)?;
            length = length.checked_add(decoded.len()).ok_or_else(limit)?;
            previous_end = span.end();
        }
        let bytes = u64::try_from(length).map_err(|_| limit())?;
        // Account for reserve/publication overlap. Original source is retained
        // by Arc rather than copied; its input extent is checked separately.
        let cost = count
            .checked_mul(std::mem::size_of::<Interval>() as u64)
            .and_then(|n| n.checked_add(bytes))
            .and_then(|n| n.checked_mul(3))
            .and_then(|n| n.checked_add(256))
            .ok_or_else(limit)?;
        if bytes > limits.text_bytes_per_event()
            || bytes > u64::from(u32::MAX)
            || cost > limits.transient_bytes()
        {
            return Err(limit());
        }
        let mut text = String::new();
        text.try_reserve_exact(length).map_err(|_| allocation())?;
        let mut intervals = Vec::new();
        intervals
            .try_reserve_exact(pieces.len())
            .map_err(|_| allocation())?;
        for piece in pieces {
            let start = text.len();
            text.push_str(piece.decoded(&original, wrap)?);
            if start != text.len() {
                intervals.push(Interval {
                    decoded: SourceSpan::new(&text, start, text.len()).map_err(|_| limit())?,
                    original: piece.source(),
                    literal: matches!(piece, TextSourcePiece::Literal(_)),
                });
            }
        }
        Ok(Self {
            original,
            text: text.into(),
            intervals: intervals.into(),
        })
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn original_source(&self) -> &str {
        &self.original
    }
    /// Map a UTF-8 decoded byte range to exact original intervals. Gaps caused by
    /// overrides remain gaps; this never invents a single contiguous cluster span.
    /// Empty ranges have no original interval. This is not glyph-cluster mapping.
    pub fn original_ranges(
        &self,
        range: std::ops::Range<u32>,
        limits: &Limits,
    ) -> Result<Vec<SourceSpan>, FontError> {
        let start = usize::try_from(range.start).map_err(|_| invalid("decoded range overflow"))?;
        let end = usize::try_from(range.end).map_err(|_| invalid("decoded range overflow"))?;
        if self.text.get(start..end).is_none() {
            return Err(invalid("decoded range is not a valid UTF-8 interval"));
        }
        if range.is_empty() {
            return Ok(Vec::new());
        }
        let overlaps =
            |item: &&Interval| item.decoded.start() < range.end && range.start < item.decoded.end();
        let count = self.intervals.iter().filter(overlaps).count();
        let cost = u64::try_from(count)
            .ok()
            .and_then(|n| n.checked_mul(3 * std::mem::size_of::<SourceSpan>() as u64))
            .and_then(|n| n.checked_add(128))
            .ok_or_else(limit)?;
        if cost > limits.transient_bytes() {
            return Err(limit());
        }
        let mut spans = Vec::new();
        spans.try_reserve_exact(count).map_err(|_| allocation())?;
        for item in self.intervals.iter().filter(overlaps) {
            let span = if item.literal {
                let start = item
                    .original
                    .start()
                    .checked_add(range.start.max(item.decoded.start()) - item.decoded.start())
                    .ok_or_else(limit)?;
                let end = item
                    .original
                    .start()
                    .checked_add(range.end.min(item.decoded.end()) - item.decoded.start())
                    .ok_or_else(limit)?;
                SourceSpan::new(
                    &self.original,
                    usize::try_from(start).map_err(|_| limit())?,
                    usize::try_from(end).map_err(|_| limit())?,
                )
                .map_err(|_| invalid("invalid literal mapping"))?
            } else {
                item.original
            };
            spans.push(span);
        }
        Ok(spans)
    }
}

/// Checked decoded range retaining its actual immutable owner. There is no
/// public constructor taking bare numbers without the owning map.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecodedSpan {
    map: Arc<SourceMappedText>,
    start: u32,
    end: u32,
}
impl DecodedSpan {
    pub const fn start(&self) -> u32 { self.start }
    pub const fn end(&self) -> u32 { self.end }
    pub const fn len(&self) -> u32 { self.end - self.start }
    pub const fn is_empty(&self) -> bool { self.start == self.end }
    pub fn source_map(&self) -> &Arc<SourceMappedText> { &self.map }
    pub fn belongs_to(&self, map: &Arc<SourceMappedText>) -> bool {
        // Ownership check only, never a persisted ID, order or cache key.
        Arc::ptr_eq(&self.map, map)
    }
    pub fn text(&self) -> &str { &self.map.text()[self.start as usize..self.end as usize] }
    pub fn original_ranges(&self, limits: &Limits) -> Result<Vec<SourceSpan>, FontError> {
        self.map.original_ranges(self.start..self.end, limits)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn span(s: &str, needle: &str) -> SourceSpan {
        let start = s.find(needle).unwrap();
        SourceSpan::new(s, start, start + needle.len()).unwrap()
    }
    fn build(s: &str, pieces: &[TextSourcePiece]) -> SourceMappedText {
        SourceMappedText::from_pieces(
            Arc::from(s),
            pieces,
            WrapMode::new(2).unwrap(),
            &Limits::default(),
        )
        .unwrap()
    }
    #[test]
    fn source_map_literal_utf8_combining_bom_crlf() {
        let source = "\u{feff}[Events]\r\nAéa\u{301}Z";
        let literal = span(source, "Aéa\u{301}Z");
        let mapped = build(source, &[TextSourcePiece::Literal(literal)]);
        assert_eq!(mapped.text(), literal.slice(source).unwrap());
        let boundaries: Vec<_> = mapped
            .text()
            .char_indices()
            .map(|(i, _)| i)
            .chain([mapped.text().len()])
            .collect();
        for &start in &boundaries {
            for &end in boundaries.iter().filter(|&&end| end > start) {
                let ranges = mapped
                    .original_ranges(start as u32..end as u32, &Limits::default())
                    .unwrap();
                assert_eq!(ranges.len(), 1);
                assert_eq!(ranges[0].slice(source).unwrap(), &mapped.text()[start..end]);
                assert_eq!(ranges[0].start(), literal.start() + start as u32);
            }
        }
        assert_eq!(mapped.original_source(), source);
    }
    #[test]
    fn source_map_decoded_escapes_keep_original_two_byte_spans() {
        let source = "\u{feff}header\r\nA\\Né\\n\\hZ";
        let pieces = [
            TextSourcePiece::Literal(span(source, "A")),
            TextSourcePiece::HardBreak(span(source, "\\N")),
            TextSourcePiece::Literal(span(source, "é")),
            TextSourcePiece::SoftBreak(span(source, "\\n")),
            TextSourcePiece::NoBreakSpace(span(source, "\\h")),
            TextSourcePiece::Literal(span(source, "Z")),
        ];
        let mapped = build(source, &pieces);
        assert_eq!(mapped.text(), "A\né\n\u{a0}Z");
        let expected: Vec<_> = pieces.iter().map(|p| p.source()).collect();
        assert_eq!(
            mapped
                .original_ranges(0..mapped.text().len() as u32, &Limits::default())
                .unwrap(),
            expected
        );
        assert_eq!(
            mapped.original_ranges(1..2, &Limits::default()).unwrap(),
            [span(source, "\\N")]
        );
        assert_eq!(
            mapped.original_ranges(4..5, &Limits::default()).unwrap(),
            [span(source, "\\n")]
        );
        assert_eq!(
            mapped.original_ranges(5..7, &Limits::default()).unwrap(),
            [span(source, "\\h")]
        );
    }
    #[test]
    fn source_map_soft_break_decode_follows_wrap_mode_only() {
        let source = "\\n";
        let piece = [TextSourcePiece::SoftBreak(span(source, source))];
        for wrap in 0..=3 {
            let mapped = SourceMappedText::from_pieces(
                Arc::from(source),
                &piece,
                WrapMode::new(wrap).unwrap(),
                &Limits::default(),
            )
            .unwrap();
            assert_eq!(mapped.text(), if wrap == 2 { "\n" } else { " " });
            assert_eq!(
                mapped.original_ranges(0..1, &Limits::default()).unwrap(),
                [span(source, source)]
            );
        }
    }
    #[test]
    fn source_map_keeps_override_and_paint_boundary_gaps() {
        for (source, left, right) in [
            ("f{\\c&H0000FF&}i", "f", "i"),
            ("a{\\c&HFFFFFF&}\u{301}", "a", "\u{301}"),
        ] {
            let pieces = [
                TextSourcePiece::Literal(span(source, left)),
                TextSourcePiece::Literal(span(source, right)),
            ];
            let mapped = build(source, &pieces);
            assert_eq!(mapped.text(), format!("{left}{right}"));
            let ranges = mapped
                .original_ranges(0..mapped.text().len() as u32, &Limits::default())
                .unwrap();
            assert_eq!(ranges, [span(source, left), span(source, right)]);
            assert!(ranges[0].end() < ranges[1].start());
        }
    }
    #[test]
    fn source_map_rejects_coarse_and_wrong_escape_claims() {
        let source = "Dialogue: \\N rest";
        for piece in [
            TextSourcePiece::HardBreak(span(source, source)),
            TextSourcePiece::SoftBreak(span(source, "\\N")),
            TextSourcePiece::NoBreakSpace(span(source, "rest")),
        ] {
            let error = SourceMappedText::from_pieces(
                Arc::from(source),
                &[piece],
                WrapMode::new(2).unwrap(),
                &Limits::default(),
            )
            .unwrap_err();
            assert_eq!(error.code(), ErrorCode::InvalidArgument);
        }
    }
    #[test]
    fn source_map_rejects_out_of_order_overlapping_and_foreign_extents() {
        let source = "ab";
        for pieces in [
            vec![
                TextSourcePiece::Literal(span(source, "b")),
                TextSourcePiece::Literal(span(source, "a")),
            ],
            vec![
                TextSourcePiece::Literal(span(source, "ab")),
                TextSourcePiece::Literal(span(source, "b")),
            ],
            vec![TextSourcePiece::Literal(
                SourceSpan::new("longer", 0, 6).unwrap(),
            )],
        ] {
            assert_eq!(
                SourceMappedText::from_pieces(
                    Arc::from(source),
                    &pieces,
                    WrapMode::new(2).unwrap(),
                    &Limits::default()
                )
                .unwrap_err()
                .code(),
                ErrorCode::InvalidArgument
            );
        }
    }
    #[test]
    fn source_map_query_rejects_split_utf8_and_invalid_ranges() {
        let mapped = build("é", &[TextSourcePiece::Literal(span("é", "é"))]);
        for (start, end) in [(0, 1), (1, 2), (2, 1), (0, 3), (u32::MAX, u32::MAX)] {
            assert_eq!(
                mapped
                    .original_ranges(start..end, &Limits::default())
                    .unwrap_err()
                    .code(),
                ErrorCode::InvalidArgument
            );
        }
        assert!(
            mapped
                .original_ranges(2..2, &Limits::default())
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn source_map_empty_input_and_retained_ownership() {
        let mapped = build("", &[]);
        assert!(mapped.text().is_empty());
        assert!(
            mapped
                .original_ranges(0..0, &Limits::default())
                .unwrap()
                .is_empty()
        );
        let retained = {
            let original: Arc<str> = Arc::from("A\\hB");
            let pieces = [
                TextSourcePiece::Literal(span(&original, "A")),
                TextSourcePiece::NoBreakSpace(span(&original, "\\h")),
                TextSourcePiece::Literal(span(&original, "B")),
            ];
            SourceMappedText::from_pieces(
                original,
                &pieces,
                WrapMode::new(2).unwrap(),
                &Limits::default(),
            )
            .unwrap()
        };
        assert_eq!(retained.text(), "A\u{a0}B");
        assert_eq!(
            retained.original_ranges(1..3, &Limits::default()).unwrap()[0]
                .slice(retained.original_source()),
            Some("\\h")
        );
    }
    #[test]
    fn source_map_preflights_text_input_transient_and_operation_limits() {
        let source = "ab";
        let piece = [TextSourcePiece::Literal(span(source, source))];
        for limits in [
            Limits::default().with_input_bytes(1).unwrap(),
            Limits::default().with_text_bytes_per_event(1).unwrap(),
            Limits::default().with_transient_bytes(1).unwrap(),
        ] {
            assert_eq!(
                SourceMappedText::from_pieces(
                    Arc::from(source),
                    &piece,
                    WrapMode::new(2).unwrap(),
                    &limits
                )
                .unwrap_err()
                .code(),
                ErrorCode::LimitExceeded
            );
        }
        let empty = TextSourcePiece::Literal(SourceSpan::new("", 0, 0).unwrap());
        assert_eq!(
            SourceMappedText::from_pieces(
                Arc::from(""),
                &[empty, empty],
                WrapMode::new(2).unwrap(),
                &Limits::default().with_text_bytes_per_event(1).unwrap()
            )
            .unwrap_err()
            .code(),
            ErrorCode::LimitExceeded
        );
        let mapped = build(source, &piece);
        assert_eq!(
            mapped
                .original_ranges(0..2, &Limits::default().with_transient_bytes(1).unwrap())
                .unwrap_err()
                .code(),
            ErrorCode::LimitExceeded
        );
    }
}
