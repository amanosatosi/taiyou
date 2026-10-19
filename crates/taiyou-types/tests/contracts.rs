use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use taiyou_types::*;

fn finite(value: f64) -> FiniteF64 {
    FiniteF64::new(value).unwrap()
}

fn sample_event(source: &str, start: i64, end: i64) -> Event {
    let span = SourceSpan::new(source, 0, source.len()).unwrap();
    Event::new(
        EventId::new(0),
        TimeUs::from_micros(start),
        TimeUs::from_micros(end),
        StyleId::new(0),
        span,
    )
    .with_program(Arc::from([TextOp::Text {
        text: Arc::from(source),
        source: span,
    }]))
}

fn ingest(
    owner: &mut TrackIngestor,
    source: &str,
    events: Vec<Event>,
    limits: &Limits,
) -> Result<Arc<Track>, ParseError> {
    owner.ingest(
        Arc::from(source),
        ScriptInfo::core_default(),
        Arc::from([Style::core_default()]),
        events.into(),
        limits,
    )
}

#[test]
fn timestamp_conversions_are_exact_and_checked() {
    assert_eq!(
        TimeUs::from_ass_components(1, 2, 3, 45)
            .unwrap()
            .as_micros(),
        3_723_450_000
    );
    assert_eq!(
        TimeUs::from_ass_components(0, 0, 0, 1).unwrap(),
        TimeUs::from_centis(1).unwrap()
    );
    assert_eq!(TimeUs::from_millis(-123).unwrap().as_micros(), -123_000);
    assert_eq!(TimeUs::from_centis(-1).unwrap().as_micros(), -10_000);
    for (h, m, s, c) in [
        (u64::MAX, 0, 0, 0),
        (0, 60, 0, 0),
        (0, 0, 60, 0),
        (0, 0, 0, 100),
    ] {
        assert!(TimeUs::from_ass_components(h, m, s, c).is_err());
    }
    for value in [i64::MIN, i64::MAX] {
        assert!(TimeUs::from_millis(value).is_err());
        assert!(TimeUs::from_centis(value).is_err());
    }
    let largest_ms = i64::MAX / 1_000;
    assert!(TimeUs::from_millis(largest_ms).is_ok());
    assert!(TimeUs::from_millis(largest_ms + 1).is_err());
    assert!(
        TimeUs::from_micros(i64::MAX)
            .checked_add(TimeUs::from_micros(1))
            .is_err()
    );
    assert!(
        TimeUs::from_micros(i64::MIN)
            .checked_sub(TimeUs::from_micros(1))
            .is_err()
    );
    assert_eq!(
        TimeUs::from_micros(-1)
            .checked_add(TimeUs::from_micros(1))
            .unwrap(),
        TimeUs::from_micros(0)
    );
}

#[test]
fn finite_numbers_reject_nonfinite_results_and_canonicalize_zero() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(FiniteF64::new(value).is_err());
    }
    let negative_zero = finite(-0.0);
    assert_eq!(negative_zero.bits(), 0);
    assert_eq!(negative_zero, FiniteF64::ZERO);
    let hash = |value: FiniteF64| {
        let mut state = DefaultHasher::new();
        value.hash(&mut state);
        state.finish()
    };
    assert_eq!(hash(negative_zero), hash(FiniteF64::ZERO));
    assert_eq!(
        negative_zero.cmp(&FiniteF64::ZERO),
        std::cmp::Ordering::Equal
    );
    assert!(finite(f64::MAX).checked_mul(finite(2.0)).is_err());
    assert!(finite(f64::MAX).checked_add(finite(f64::MAX)).is_err());
    assert!(finite(-f64::MAX).checked_sub(finite(f64::MAX)).is_err());
    assert!(finite(1.0).checked_div(FiniteF64::ZERO).is_err());
    assert!(FiniteF64::ZERO.checked_div(FiniteF64::ZERO).is_err());
    assert_eq!(finite(6.0).checked_div(finite(2.0)).unwrap(), finite(3.0));
}

#[test]
fn q6_ties_floor_phase_and_overflow_follow_c1() {
    for (pixels, ticks) in [
        (0.5 / 64.0, 1),
        (-0.5 / 64.0, -1),
        (1.5 / 64.0, 2),
        (-1.5 / 64.0, -2),
    ] {
        assert_eq!(Q6::from_pixels(finite(pixels)).unwrap().ticks(), ticks);
    }
    assert_eq!(Q6::from_ticks(-1).split(), (-1, 63));
    for ticks in [
        i32::MIN,
        -129,
        -128,
        -127,
        -65,
        -64,
        -63,
        -1,
        0,
        1,
        63,
        64,
        65,
        i32::MAX,
    ] {
        let q = Q6::from_ticks(ticks);
        let (whole, phase) = q.split();
        assert!((0..64).contains(&phase));
        assert_eq!(i64::from(whole) * 64 + i64::from(phase), i64::from(ticks));
        assert_eq!(Q6::from_pixels(q.as_pixels()).unwrap(), q);
    }
    assert!(Q6::from_pixels(finite(f64::MAX)).is_err());
    assert!(Q6::from_pixels(finite((f64::from(i32::MAX) + 0.5) / 64.0)).is_err());
    assert!(Q6::from_pixels(finite((f64::from(i32::MIN) - 0.5) / 64.0)).is_err());
    assert!(
        Q6::from_ticks(i32::MAX)
            .checked_add(Q6::from_ticks(1))
            .is_err()
    );
    assert!(
        Q6::from_ticks(i32::MIN)
            .checked_sub(Q6::from_ticks(1))
            .is_err()
    );
    assert!(Q6::from_ticks(i32::MIN).checked_neg().is_err());
    assert!(Q6::from_ticks(i32::MAX).checked_mul(2).is_err());
    assert_eq!(Q6::from_ticks(-32).checked_mul(2).unwrap().split(), (-1, 0));
}

#[test]
fn spans_validate_utf8_boundaries_empty_ranges_and_containment() {
    let source = "aé日z";
    assert_eq!(
        SourceSpan::new(source, 1, 6).unwrap().slice(source),
        Some("é日")
    );
    assert!(SourceSpan::new(source, 2, 3).is_err());
    assert!(SourceSpan::new(source, 1, 4).is_err());
    assert!(SourceSpan::new(source, 6, 1).is_err());
    assert!(SourceSpan::new(source, 0, usize::MAX).is_err());
    assert!(
        SourceSpan::new(source, source.len(), source.len())
            .unwrap()
            .is_empty()
    );
    let whole = SourceSpan::new(source, 0, source.len()).unwrap();
    assert!(whole.contains(SourceSpan::new(source, 1, 3).unwrap()));
    assert_eq!(whole.len(), 7);
    assert_eq!(whole.slice("x"), None);
}

#[test]
fn rectangles_are_half_open_and_checked_across_full_i32_range() {
    let a = PixelRect::new(-1, -2, 1, 2).unwrap();
    assert!(a.contains_pixel(-1, -2));
    assert!(!a.contains_pixel(1, 0));
    assert!(!a.contains_pixel(0, 2));
    assert!(!a.intersects(PixelRect::new(1, -2, 2, 2).unwrap()));
    assert!(a.intersects(PixelRect::new(0, 0, 2, 3).unwrap()));
    for empty in [
        PixelRect::new(0, 0, 0, 4).unwrap(),
        PixelRect::new(0, 0, 4, 0).unwrap(),
    ] {
        assert!(empty.is_empty());
        assert!(!a.intersects(empty));
    }
    assert!(PixelRect::new(2, 0, 1, 0).is_err());
    let largest = PixelRect::new(i32::MIN, i32::MIN, i32::MAX, i32::MAX).unwrap();
    assert_eq!(largest.width(), u32::MAX);
    assert_eq!(largest.height(), u32::MAX);
    assert!(largest.checked_translate(1, 0).is_err());
    assert!(largest.checked_translate(0, -1).is_err());
    assert_eq!(
        a.checked_translate(2, 3).unwrap(),
        PixelRect::new(1, 1, 3, 5).unwrap()
    );
    assert!(Size::new(u32::MAX, u32::MAX).rgba_layout().is_err());
    assert_eq!(Size::new(2, 3).rgba_layout().unwrap(), (8, 24));
}

#[test]
fn ass_colors_are_straight_rgba_with_inverted_transparency() {
    assert_eq!(Rgba8::from_ass(0x800000FF).channels(), [255, 0, 0, 127]);
    assert_eq!(Rgba8::from_ass(0xFF332211).channels(), [17, 34, 51, 0]);
    assert_eq!(Rgba8::from_ass(0x00332211).channels(), [17, 34, 51, 255]);
}

#[test]
fn core_defaults_and_style_invariants_are_explicit() {
    let style = Style::core_default();
    assert_eq!(style.name(), "Default");
    assert_eq!(style.font().family(), "Arial");
    assert_eq!(style.font().weight(), 400);
    assert!(!style.font().italic());
    assert!(!style.underline());
    assert!(!style.strikeout());
    assert_eq!(style.font_size(), finite(20.0));
    assert_eq!(style.scale_x(), FiniteF64::ONE);
    assert_eq!(style.scale_y(), FiniteF64::ONE);
    assert_eq!(style.spacing(), FiniteF64::ZERO);
    assert_eq!(style.angle(), FiniteF64::ZERO);
    assert_eq!(style.border_style(), BorderStyle::Outline);
    assert_eq!(style.border(), finite(2.0));
    assert_eq!(style.shadow(), finite(2.0));
    assert_eq!(style.alignment().get(), 2);
    assert_eq!(style.margins(), Margins::new(10, 10, 10));
    assert_eq!(style.encoding(), 1);
    assert_eq!(style.primary().channels(), [255, 255, 255, 255]);
    assert_eq!(style.secondary().channels(), [255, 255, 0, 255]);
    assert_eq!(style.outline().channels(), [0, 0, 0, 255]);
    assert_eq!(style.shadow_color().channels(), [0, 0, 0, 255]);
    assert!(style.clone().with_size(FiniteF64::ZERO).is_err());
    assert!(style.clone().with_size(finite(-1.0)).is_err());
    assert!(style.clone().with_border(finite(-1.0)).is_err());
    assert!(
        style
            .clone()
            .with_scale(finite(-1.0), FiniteF64::ONE)
            .is_err()
    );
    assert!(style.with_scale(FiniteF64::ZERO, FiniteF64::ZERO).is_ok());
    for value in [0, 10, 255] {
        assert!(Alignment::new(value).is_err());
    }
    for value in 1..=9 {
        assert_eq!(Alignment::new(value).unwrap().get(), value);
    }
    assert!(WrapMode::new(4).is_err());
    assert_eq!(
        Margins::new(0, 2, 0).inherit(Margins::new(10, 10, 10)),
        Margins::new(10, 2, 10)
    );
    let script = ScriptInfo::core_default();
    assert_eq!(script.play_res(), Size::new(384, 288));
    assert_eq!(script.wrap().get(), 0);
    assert!(!script.scaled_border_shadow());
    assert!(script.clone().with_play_res(Size::new(0, 1)).is_err());
    assert!(script.with_layout_res(Some(Size::new(1, 0))).is_err());
}

#[test]
fn default_limits_and_tiny_allocation_limits_are_enforced() {
    let limits = Limits::default();
    assert_eq!(limits.input_bytes(), 16 * 1024 * 1024);
    assert_eq!(limits.events(), 100_000);
    assert_eq!(limits.text_bytes_per_event(), 1024 * 1024);
    assert_eq!(limits.glyphs_per_event(), 100_000);
    assert_eq!(limits.path_segments_per_frame(), 1_000_000);
    assert_eq!(limits.tag_nesting(), 8);
    assert_eq!(limits.max_dimension(), 16_384);
    assert_eq!(limits.output_pixels(), 32_000_000);
    assert_eq!(limits.font_bytes(), 64 * 1024 * 1024);
    assert_eq!(limits.cache_bytes(), 128 * 1024 * 1024);
    assert_eq!(limits.transient_bytes(), 128 * 1024 * 1024);
    assert_eq!(limits.output_bytes(), 128 * 1024 * 1024);
    assert_eq!(limits.filter_radius(), 256);
    assert_eq!(limits.be_passes(), 64);
    assert!(limits.clone().with_events(0).is_err());
    assert!(limits.clone().with_input_bytes(0).is_err());
    assert!(limits.check_input_bytes(usize::MAX).is_err());
    assert_eq!(limits.clone().with_cache_bytes(0).cache_bytes(), 0);
    assert!(limits.check_output_size(Size::new(0, 10)).is_err());
    assert!(limits.check_output_size(Size::new(16_385, 1)).is_err());
    assert!(limits.check_output_size(Size::new(8_000, 8_000)).is_err());
    assert_eq!(
        limits
            .clone()
            .with_output_bytes(15)
            .unwrap()
            .check_output_size(Size::new(2, 2))
            .unwrap_err()
            .code(),
        ErrorCode::LimitExceeded
    );
    assert!(
        limits
            .with_output_bytes(16)
            .unwrap()
            .check_output_size(Size::new(2, 2))
            .is_ok()
    );
}

#[test]
fn event_intervals_are_end_exclusive_and_do_not_subtract_overflowing_times() {
    let event = sample_event("x", -10, 10);
    for now in [-10, -1, 0, 9] {
        assert!(event.is_active(TimeUs::from_micros(now)));
    }
    for now in [-11, 10, i64::MAX] {
        assert!(!event.is_active(TimeUs::from_micros(now)));
    }
    let widest = sample_event("x", i64::MIN, i64::MAX);
    assert!(widest.is_active(TimeUs::from_micros(0)));
    for (start, end) in [(0, 0), (1, 0)] {
        let event = sample_event("x", start, end);
        assert!(!event.is_active(TimeUs::from_micros(0)));
        let track = ingest(
            &mut TrackIngestor::new(),
            "x",
            vec![event],
            &Limits::default(),
        )
        .unwrap();
        let parsed = ParsedTrack::new(track, DiagnosticBuffer::new()).unwrap();
        assert_eq!(parsed.diagnostics().len(), 1);
        assert_eq!(
            parsed.diagnostics()[0].code(),
            DiagnosticCode::InactiveEvent
        );
    }
}

#[test]
fn snapshot_clones_share_storage_and_new_ingestions_get_new_identities() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Track>();
    assert_send_sync::<Style>();
    assert_send_sync::<Event>();
    let mut owner = TrackIngestor::new();
    let track = ingest(
        &mut owner,
        "hello",
        vec![sample_event("hello", 0, 1)],
        &Limits::default(),
    )
    .unwrap();
    let clone = track.as_ref().clone();
    assert!(Arc::ptr_eq(track.source_storage(), clone.source_storage()));
    assert!(Arc::ptr_eq(track.style_storage(), clone.style_storage()));
    assert!(Arc::ptr_eq(track.event_storage(), clone.event_storage()));
    assert!(Arc::ptr_eq(
        track.events()[0].program_storage(),
        clone.events()[0].program_storage()
    ));
    assert_eq!(track.revision(), clone.revision());
    let edited_style = track.styles()[0].clone().with_name(Arc::from("Other"));
    assert_eq!(track.styles()[0].name(), "Default");
    assert_eq!(edited_style.name(), "Other");
    let next = ingest(
        &mut owner,
        "hello",
        vec![sample_event("hello", 0, 1)],
        &Limits::default(),
    )
    .unwrap();
    assert_ne!(track.revision(), next.revision());
    drop(track);
    assert_eq!(clone.source(), "hello");
    assert_eq!(clone.events().len(), 1);
}

#[test]
fn publication_rejects_cross_record_errors_without_consuming_identity() {
    let mut owner = TrackIngestor::new();
    let source = "ab";
    let span = SourceSpan::new(source, 0, 1).unwrap();
    let base = Event::new(
        EventId::new(0),
        TimeUs::from_micros(0),
        TimeUs::from_micros(1),
        StyleId::new(0),
        span,
    );
    let mismatch = base.clone().with_program(Arc::from([TextOp::Text {
        text: Arc::from("b"),
        source: span,
    }]));
    assert!(ingest(&mut owner, source, vec![mismatch], &Limits::default()).is_err());
    let outside = Event::new(
        EventId::new(0),
        base.start(),
        base.end(),
        StyleId::new(99),
        span,
    );
    assert!(ingest(&mut owner, source, vec![outside], &Limits::default()).is_err());
    assert!(
        ingest(
            &mut owner,
            source,
            vec![base.clone(), base.clone()],
            &Limits::default()
        )
        .is_err()
    );
    let track = ingest(&mut owner, source, vec![base], &Limits::default()).unwrap();
    assert_eq!(track.revision().get(), 0);
    let limited = Limits::default().with_input_bytes(1).unwrap();
    assert_eq!(
        ingest(&mut owner, source, vec![], &limited)
            .unwrap_err()
            .code(),
        ErrorCode::LimitExceeded
    );
    let limited = Limits::default().with_text_bytes_per_event(1).unwrap();
    assert_eq!(
        ingest(
            &mut owner,
            source,
            vec![sample_event(source, 0, 1)],
            &limited
        )
        .unwrap_err()
        .code(),
        ErrorCode::LimitExceeded
    );
    let limited = Limits::default().with_events(1).unwrap();
    assert_eq!(
        ingest(
            &mut owner,
            source,
            vec![sample_event(source, 0, 1), sample_event(source, 1, 2)],
            &limited
        )
        .unwrap_err()
        .code(),
        ErrorCode::LimitExceeded
    );
}

#[test]
fn diagnostics_are_bounded_sorted_and_suppression_is_explicit() {
    let mut buffer = DiagnosticBuffer::new();
    for ordinal in (0..1_030).rev() {
        buffer
            .push(Diagnostic::new(
                DiagnosticCode::UnknownStyle,
                Some(EventId::new(ordinal)),
                None,
                "unknown style",
            ))
            .unwrap();
    }
    buffer
        .push(Diagnostic::new(
            DiagnosticCode::MissingDefaultStyle,
            None,
            None,
            "missing Default",
        ))
        .unwrap();
    assert_eq!(buffer.suppressed_count(), 7);
    let diagnostics = buffer.finish().unwrap();
    assert_eq!(diagnostics.len(), MAX_DIAGNOSTICS + 1);
    assert_eq!(diagnostics[0].event(), None);
    assert_eq!(diagnostics[1].event(), Some(EventId::new(0)));
    assert_eq!(
        diagnostics[MAX_DIAGNOSTICS - 1].event(),
        Some(EventId::new(1022))
    );
    let summary = diagnostics.last().unwrap();
    assert_eq!(summary.code(), DiagnosticCode::DiagnosticsSuppressed);
    assert_eq!(summary.detail(), "7 diagnostics suppressed");
    assert!(DiagnosticBuffer::new().push(summary.clone()).is_err());
    let long = "日".repeat(MAX_DETAIL_BYTES);
    let error = ParseError::new(ErrorCode::NativeFailure, &long).with_native_code(-123);
    assert!(error.detail().len() <= MAX_DETAIL_BYTES);
    assert!(long.starts_with(error.detail()));
    assert_eq!(error.native_code(), Some(-123));
    assert!(error.to_string().contains("native code -123"));
    assert_eq!(error.code().as_str(), "NativeFailure");
}

#[test]
fn diagnostic_order_uses_header_event_then_span_then_code() {
    let span0 = SourceSpan::new("ab", 0, 1).unwrap();
    let span1 = SourceSpan::new("ab", 1, 2).unwrap();
    let mut buffer = DiagnosticBuffer::new();
    for (event, span, code) in [
        (
            Some(EventId::new(1)),
            Some(span0),
            DiagnosticCode::UnknownStyle,
        ),
        (
            Some(EventId::new(0)),
            Some(span1),
            DiagnosticCode::UnknownStyle,
        ),
        (
            Some(EventId::new(0)),
            Some(span0),
            DiagnosticCode::UnknownStyle,
        ),
        (
            Some(EventId::new(0)),
            Some(span0),
            DiagnosticCode::MalformedRecord,
        ),
        (None, Some(span1), DiagnosticCode::UnknownSection),
    ] {
        buffer
            .push(Diagnostic::new(code, event, span, "test"))
            .unwrap();
    }
    let result = buffer.finish().unwrap();
    assert_eq!(result[0].event(), None);
    assert_eq!(result[1].code(), DiagnosticCode::MalformedRecord);
    assert_eq!(result[2].span(), Some(span0));
    assert_eq!(result[3].span(), Some(span1));
    assert_eq!(result[4].event(), Some(EventId::new(1)));
}

#[test]
fn source_order_is_independent_of_time_layer_and_literal_span_order_is_checked() {
    let source = "ab";
    let first_span = SourceSpan::new(source, 0, 1).unwrap();
    let second_span = SourceSpan::new(source, 1, 2).unwrap();
    let first = Event::new(
        EventId::new(2),
        TimeUs::from_micros(100),
        TimeUs::from_micros(200),
        StyleId::new(0),
        first_span,
    )
    .with_layer(99);
    let second = Event::new(
        EventId::new(7),
        TimeUs::from_micros(-100),
        TimeUs::from_micros(0),
        StyleId::new(0),
        second_span,
    )
    .with_layer(-99);
    let mut owner = TrackIngestor::new();
    let track = ingest(&mut owner, source, vec![first, second], &Limits::default()).unwrap();
    assert_eq!(track.events()[0].id(), EventId::new(2));
    assert_eq!(track.events()[1].id(), EventId::new(7));
    let reversed = sample_event(source, 0, 1).with_program(Arc::from([
        TextOp::Text {
            text: Arc::from("b"),
            source: second_span,
        },
        TextOp::Text {
            text: Arc::from("a"),
            source: first_span,
        },
    ]));
    assert!(ingest(&mut owner, source, vec![reversed], &Limits::default()).is_err());
}
