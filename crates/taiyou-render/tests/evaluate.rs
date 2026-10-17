use std::sync::Arc;
use taiyou_render::{evaluate, evaluate_active, evaluate_with_policy};
use taiyou_types::*;
fn span(a: usize, b: usize) -> SourceSpan {
    SourceSpan::new("ABC", a, b).unwrap()
}
fn text(i: usize) -> TextOp {
    TextOp::Text {
        text: Arc::from(&"ABC"[i..i + 1]),
        source: span(i, i + 1),
    }
}
fn reset(name: Option<&str>, i: usize) -> TextOp {
    TextOp::Override(OverrideOp::ResetStyle {
        name: name.map(Arc::from),
        source: span(i, i),
    })
}
fn track(styles: Vec<Style>, program: Vec<TextOp>) -> Arc<Track> {
    let event = Event::new(
        EventId::new(0),
        TimeUs::from_micros(-100),
        TimeUs::from_micros(100),
        StyleId::new(0),
        span(0, 3),
    )
    .with_program(program.into());
    TrackIngestor::new()
        .ingest(
            Arc::from("ABC"),
            ScriptInfo::core_default(),
            styles.into(),
            Arc::from([event]),
            &Limits::default(),
        )
        .unwrap()
}
fn at(t: &Track, now: i64) -> EvaluatedEvent {
    evaluate(
        t,
        EventId::new(0),
        TimeUs::from_micros(now),
        Profile::CoreV1,
    )
    .unwrap()
}

#[test]
fn evaluate_ordered_color_and_resets_keep_shape_independent() {
    let t = track(
        vec![Style::core_default()],
        vec![
            TextOp::Override(OverrideOp::SetPaint {
                channel: PaintChannel::Primary,
                color: Rgba8::new(255, 0, 0, 255),
                source: span(0, 0),
            }),
            text(0),
            reset(None, 1),
            text(1),
        ],
    );
    let e = at(&t, 0);
    assert_eq!(e.runs()[0].paint().primary().channels(), [255, 0, 0, 255]);
    assert_eq!(
        e.runs()[1].paint().primary().channels(),
        [255, 255, 255, 255]
    );
    assert_eq!(e.runs()[0].shaping(), e.runs()[1].shaping());
    assert!(e.animation().paint());
    assert!(!e.animation().shape());
    assert!(!e.animation().geometry());
    assert!(!e.animation().placement());
    assert!(e.animation().is_static());
}
#[test]
fn evaluate_named_reset_uses_last_definition_then_unnamed_uses_event_style() {
    let styles = vec![
        Style::core_default(),
        Style::core_default()
            .with_name(Arc::from("nd"))
            .with_size(FiniteF64::new(30.0).unwrap())
            .unwrap(),
        Style::core_default()
            .with_name(Arc::from("nd"))
            .with_size(FiniteF64::new(40.0).unwrap())
            .unwrap(),
    ];
    let t = track(
        styles,
        vec![reset(Some("nd"), 0), text(0), reset(None, 1), text(1)],
    );
    let e = at(&t, -99);
    assert_eq!(e.runs()[0].shaping().font_size().get(), 40.0);
    assert_eq!(e.runs()[1].shaping().font_size().get(), 20.0);
    assert_eq!(at(&t, 99), e);
    assert_eq!(at(&t, -100), e);
}
#[test]
fn evaluate_repeated_tags_opacity_and_dependency_flags_are_exact() {
    let t = track(
        vec![Style::core_default()],
        vec![
            TextOp::Override(OverrideOp::SetSize {
                size: FiniteF64::new(30.0).unwrap(),
                source: span(0, 0),
            }),
            TextOp::Override(OverrideOp::SetSize {
                size: FiniteF64::new(45.0).unwrap(),
                source: span(0, 0),
            }),
            TextOp::Override(OverrideOp::SetOpacity {
                channel: None,
                opacity: 64,
                source: span(0, 0),
            }),
            TextOp::Override(OverrideOp::SetPaint {
                channel: PaintChannel::Primary,
                color: Rgba8::new(7, 8, 9, 255),
                source: span(0, 0),
            }),
            text(0),
        ],
    );
    let e = at(&t, 0);
    assert_eq!(e.runs()[0].shaping().font_size().get(), 45.0);
    assert_eq!(e.runs()[0].paint().primary().channels(), [7, 8, 9, 64]);
    assert_eq!(e.runs()[0].paint().shadow().a(), 64);
    assert!(e.animation().shape());
    assert!(e.animation().layout());
    assert!(e.animation().geometry());
    assert!(!e.animation().effects());
    assert!(!e.animation().clip());
}
#[test]
fn evaluate_placement_is_last_applicable_and_reset_does_not_reset_event_clock() {
    let p = ScriptPoint::new(FiniteF64::new(-2.0).unwrap(), FiniteF64::new(3.0).unwrap());
    let t = track(
        vec![Style::core_default()],
        vec![
            TextOp::Override(OverrideOp::SetPosition {
                point: p,
                source: span(0, 0),
            }),
            reset(None, 0),
            text(0),
        ],
    );
    let e = at(&t, -100);
    assert_eq!(e.placement().anchor(), Anchor::Explicit(p));
    assert!(!e.placement().collision_eligible());
    assert!(e.animation().placement());
    assert!(!e.animation().shape());
    assert_eq!(e, at(&t, 99));
}
#[test]
fn evaluate_selection_boundaries_negative_time_and_layer_order() {
    let mut events = Vec::new();
    for (i, layer) in [2, -1, 2].into_iter().enumerate() {
        events.push(
            Event::new(
                EventId::new(u32::try_from(i).unwrap()),
                TimeUs::from_micros(-10),
                TimeUs::from_micros(10),
                StyleId::new(0),
                SourceSpan::new("", 0, 0).unwrap(),
            )
            .with_layer(layer),
        );
    }
    let t = TrackIngestor::new()
        .ingest(
            Arc::from(""),
            ScriptInfo::core_default(),
            Arc::from([Style::core_default()]),
            events.into(),
            &Limits::default(),
        )
        .unwrap();
    for (time, count) in [(-11, 0), (-10, 3), (9, 3), (10, 0), (0, 3), (-10, 3)] {
        let result = evaluate_active(
            &t,
            TimeUs::from_micros(time),
            Profile::CoreV1,
            UnsupportedPolicy::Strict,
            &Limits::default(),
        )
        .unwrap();
        assert_eq!(result.events().len(), count);
        if count != 0 {
            assert_eq!(
                result
                    .events()
                    .iter()
                    .map(|e| e.id().ordinal())
                    .collect::<Vec<_>>(),
                [1, 0, 2]
            );
        }
    }
}
#[test]
fn evaluate_unsupported_policy_diagnostics_are_independent_and_preserve_state() {
    let t = track(
        vec![Style::core_default()],
        vec![
            text(0),
            TextOp::Unsupported {
                name: Arc::from("rnd"),
                source: span(1, 1),
            },
            reset(Some("Missing"), 1),
            text(1),
        ],
    );
    assert_eq!(
        evaluate(&t, EventId::new(0), TimeUs::from_micros(0), Profile::CoreV1)
            .unwrap_err()
            .code(),
        ErrorCode::UnsupportedFeature
    );
    let call = || {
        evaluate_with_policy(
            &t,
            EventId::new(0),
            TimeUs::from_micros(0),
            Profile::CoreV1,
            UnsupportedPolicy::Tolerant,
            &Limits::default(),
        )
        .unwrap()
    };
    let (e, d) = call();
    assert_eq!(d.len(), 2);
    assert_eq!(d[0].event(), Some(EventId::new(0)));
    assert_eq!(e.runs()[0].paint(), e.runs()[1].paint());
    assert_eq!(call(), (e, d));
    assert_eq!(
        evaluate_active(
            &t,
            TimeUs::from_micros(0),
            Profile::LegacyXyV1,
            UnsupportedPolicy::Tolerant,
            &Limits::default()
        )
        .unwrap_err()
        .code(),
        ErrorCode::UnsupportedFeature
    );
}
#[test]
fn evaluate_escapes_and_tiny_budget_fail_without_partial_output() {
    let t = track(
        vec![Style::core_default()],
        vec![
            TextOp::HardBreak,
            TextOp::SoftBreak,
            TextOp::NoBreakSpace,
            text(0),
        ],
    );
    let e = at(&t, 0);
    assert_eq!(
        e.runs().iter().map(|r| r.text()).collect::<Vec<_>>(),
        ["\n", "\n", "\u{a0}", "A"]
    );
    assert_eq!(e.runs()[0].source(), span(0, 3));
    assert_eq!(e.runs()[3].source(), span(0, 1));
    let limits = Limits::default().with_transient_bytes(1).unwrap();
    assert_eq!(
        evaluate_active(
            &t,
            TimeUs::from_micros(0),
            Profile::CoreV1,
            UnsupportedPolicy::Strict,
            &limits
        )
        .unwrap_err()
        .code(),
        ErrorCode::LimitExceeded
    );
    assert_eq!(
        evaluate(
            &t,
            EventId::new(0),
            TimeUs::from_micros(100),
            Profile::CoreV1
        )
        .unwrap_err()
        .code(),
        ErrorCode::InvalidArgument
    );
    assert_eq!(at(&t, 0), e);
}
