use std::sync::Arc;
use taiyou_types::*;
#[test]
fn signed_blur_target_is_valid_but_static_style_clamps_to_zero() {
    let negative = FiniteF64::new(-5.0).unwrap();
    let span = SourceSpan::new("A", 0, 0).unwrap();
    let op = OverrideOp::SetBlur {
        value: Some(negative),
        source: span,
    };
    op.validate().unwrap();
    assert!(Style::core_default().with_blur(negative).is_err());
    let event = Event::new(
        EventId::new(0),
        TimeUs::from_micros(0),
        TimeUs::from_micros(2),
        StyleId::new(0),
        SourceSpan::new("A", 0, 1).unwrap(),
    )
    .with_program(Arc::from([
        TextOp::Override(op),
        TextOp::Text {
            text: Arc::from("A"),
            source: SourceSpan::new("A", 0, 1).unwrap(),
        },
    ]));
    let style = Style::core_default()
        .with_blur(FiniteF64::new(2.0).unwrap())
        .unwrap();
    let track = TrackIngestor::new()
        .ingest(
            Arc::from("A"),
            ScriptInfo::core_default(),
            Arc::from([style]),
            Arc::from([event]),
            &Limits::default(),
        )
        .unwrap();
    let evaluated = taiyou_render::evaluate(
        &track,
        EventId::new(0),
        TimeUs::from_micros(1),
        Profile::CoreV1,
    )
    .unwrap();
    assert_eq!(evaluated.runs()[0].effects().blur(), FiniteF64::ZERO);
}

#[test]
fn blur_empty_restores_original_style_after_named_reset_and_keeps_fractional_be() {
    let n = |v| FiniteF64::new(v).unwrap();
    let base = Style::core_default()
        .with_blur(n(2.0))
        .unwrap()
        .with_be(n(1.5));
    let named = Style::core_default()
        .with_name(Arc::from("Named"))
        .with_blur(n(4.0))
        .unwrap()
        .with_be(n(3.5));
    let span = SourceSpan::new("ABCD", 0, 0).unwrap();
    let event = Event::new(
        EventId::new(0),
        TimeUs::from_micros(0),
        TimeUs::from_micros(2),
        StyleId::new(0),
        SourceSpan::new("ABCD", 0, 4).unwrap(),
    )
    .with_program(Arc::from([
        TextOp::Override(OverrideOp::ResetStyle {
            name: Some(Arc::from("Named")),
            source: span,
        }),
        TextOp::Text {
            text: Arc::from("A"),
            source: SourceSpan::new("ABCD", 0, 1).unwrap(),
        },
        TextOp::Override(OverrideOp::SetBlur {
            value: None,
            source: SourceSpan::new("ABCD", 1, 1).unwrap(),
        }),
        TextOp::Override(OverrideOp::SetBe {
            value: None,
            source: SourceSpan::new("ABCD", 1, 1).unwrap(),
        }),
        TextOp::Text {
            text: Arc::from("B"),
            source: SourceSpan::new("ABCD", 1, 2).unwrap(),
        },
        TextOp::Override(OverrideOp::SetBlur {
            value: Some(n(0.0)),
            source: SourceSpan::new("ABCD", 2, 2).unwrap(),
        }),
        TextOp::Override(OverrideOp::SetBe {
            value: Some(n(0.25)),
            source: SourceSpan::new("ABCD", 2, 2).unwrap(),
        }),
        TextOp::Text {
            text: Arc::from("C"),
            source: SourceSpan::new("ABCD", 2, 3).unwrap(),
        },
        TextOp::Override(OverrideOp::ResetStyle {
            name: None,
            source: SourceSpan::new("ABCD", 3, 3).unwrap(),
        }),
        TextOp::Text {
            text: Arc::from("D"),
            source: SourceSpan::new("ABCD", 3, 4).unwrap(),
        },
    ]));
    let track = TrackIngestor::new()
        .ingest(
            Arc::from("ABCD"),
            ScriptInfo::core_default(),
            Arc::from([base, named]),
            Arc::from([event]),
            &Limits::default(),
        )
        .unwrap();
    let e = taiyou_render::evaluate(
        &track,
        EventId::new(0),
        TimeUs::from_micros(1),
        Profile::CoreV1,
    )
    .unwrap();
    let values: Vec<_> = e
        .runs()
        .iter()
        .map(|r| (r.effects().blur().get(), r.effects().be().get()))
        .collect();
    assert_eq!(values, [(4.0, 3.5), (2.0, 1.5), (0.0, 0.25), (2.0, 1.5)]);
}
