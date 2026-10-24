use std::{ops::Range, sync::Arc};
use taiyou_render::layout;
use taiyou_types::*;
fn f(n: f64) -> FiniteF64 {
    FiniteF64::new(n).unwrap()
}
fn font(n: u8) -> FontId {
    FontId::new([n; 32], 0)
}
fn metrics(a: f64, d: f64) -> RunMetrics {
    RunMetrics::new(f(a), f(d), f(99.0), 1000).unwrap()
}
struct G {
    id: u32,
    span: Range<u32>,
    advance: f64,
    x: f64,
    y: f64,
    ink: GlyphInk,
    font: FontId,
}
fn g(id: u32, span: Range<u32>, advance: f64) -> G {
    G {
        id,
        span,
        advance,
        x: 0.0,
        y: 0.0,
        ink: GlyphInk::Bounds(ScriptBounds::new(0.0, -6.0, advance.max(1.0), 1.0).unwrap()),
        font: font(1),
    }
}
struct R {
    span: Range<u32>,
    level: u8,
    metrics: RunMetrics,
    glyphs: Vec<G>,
    groups: Vec<FaceMetrics>,
}
fn r(span: Range<u32>, level: u8, a: f64, d: f64, glyphs: Vec<G>) -> R {
    R {
        span,
        level,
        metrics: metrics(a, d),
        glyphs,
        groups: Vec::new(),
    }
}
fn event(text: &str, runs: Vec<R>, empty: &[(u32, f64, f64)], style: Style) -> ShapedEvent {
    let source: Arc<str> = Arc::from(text);
    let source_span = SourceSpan::new(&source, 0, source.len()).unwrap();
    let map = Arc::new(
        SourceMappedText::from_pieces(
            source,
            &[TextSourcePiece::Literal(source_span)],
            WrapMode::new(2).unwrap(),
            &Limits::default(),
        )
        .unwrap(),
    );
    let evaluated = EvaluatedEvent::new(
        EventId::new(0),
        EventOrder::new(0, 0),
        Arc::from([EvaluatedRun::new(Arc::from(text), source_span, &style)
            .with_decoded(map.decoded_span(0..text.len() as u32).unwrap())
            .unwrap()]),
        Placement::new(&style, Margins::new(0, 0, 0), Anchor::Default),
        ChangeClass::default(),
    )
    .with_terminal_style(style.clone())
    .with_source_map(map.clone())
    .unwrap();
    let shaped: Vec<_> = runs
        .into_iter()
        .map(|run| {
            let glyphs: Vec<_> = run
                .glyphs
                .into_iter()
                .map(|g| {
                    ShapedGlyph::new(
                        g.font,
                        g.id,
                        map.decoded_span(g.span).unwrap(),
                        GlyphPosition::new(f(g.advance), f(0.0), f(g.x), f(g.y)),
                    )
                    .with_ink(g.ink)
                })
                .collect();
            let count = glyphs.len();
            let groups = if run.groups.is_empty() {
                vec![FaceMetrics {
                    font: font(1),
                    metrics: run.metrics,
                    glyphs: 0..count,
                }]
            } else {
                run.groups
            };
            ShapedRun::new(
                glyphs.into(),
                run.metrics,
                map.decoded_span(run.span).unwrap(),
                run.level,
                u32::from_be_bytes(*b"Latn"),
                Arc::from("und"),
            )
            .unwrap()
            .with_face_metrics(groups.into())
            .unwrap()
        })
        .collect();
    let breaks: Vec<_> = text
        .char_indices()
        .filter(|(_, ch)| *ch == '\n')
        .map(|(i, _)| map.decoded_span(i as u32..i as u32 + 1).unwrap())
        .collect();
    let empty: Vec<_> = empty
        .iter()
        .map(|&(at, a, d)| EmptyLineMetrics {
            decoded: map.decoded_span(at..at).unwrap(),
            font: font(1),
            metrics: metrics(a, d),
            scale_y: f(1.0),
        })
        .collect();
    ShapedEvent::new(
        &evaluated,
        map.clone(),
        shaped.into(),
        breaks.into(),
        Arc::from([PaintSpan {
            decoded: map.decoded_span(0..text.len() as u32).unwrap(),
            paint: PaintSet::from_style(&style),
        }]),
        Arc::from([]),
        Arc::from([]),
    )
    .unwrap()
    .with_empty_lines(empty.into())
    .unwrap()
}
fn constraints(n: u8) -> LayoutConstraints {
    LayoutConstraints::new(Alignment::new(n).unwrap())
}
#[test]
fn native_word_extents_round_before_alignment_without_rounding_each_glyph() {
    // Authenticated GDI witnesses: Fansub Block's A measures 6210 native Q6
    // ticks. CText stores (6210+4)>>3 = 776 eighth-pixels, i.e. 97 pixels.
    let input = event(
        "A",
        vec![r(0..1, 0, 8.0, 2.0, vec![g(1, 0..1, 6210.0 / 64.0)])],
        &[],
        Style::core_default(),
    );
    for alignment in 1..=9 {
        let output = layout(&input, &constraints(alignment)).unwrap();
        let x = -97.0 * f64::from((alignment - 1) % 3) / 2.0;
        assert_eq!(output.lines()[0].advance(), f(97.0));
        assert_eq!(output.glyphs()[0].position().x(), f(x));
        assert_eq!(
            output.shaped().runs()[0].glyphs()[0].advance_x(),
            f(6210.0 / 64.0)
        );
    }
    // A script/font item boundary inside the same evaluated word is not a word
    // extent boundary. Keep its internal advance, then round the combined word.
    let input = event(
        "ab",
        vec![
            r(0..1, 0, 8.0, 2.0, vec![g(1, 0..1, 25.546875)]),
            r(1..2, 0, 8.0, 2.0, vec![g(2, 1..2, 25.546875)]),
        ],
        &[],
        Style::core_default(),
    );
    let output = layout(&input, &constraints(5)).unwrap();
    assert_eq!(output.lines()[0].advance(), f(51.125));
    assert_eq!(output.glyphs()[0].position().x(), f(-25.5));
    assert_eq!(output.glyphs()[1].position().x(), f(0.046875));
}

#[test]
fn following_words_use_rounded_extents_in_the_physical_metric_grid() {
    let input = event(
        "ab cd",
        vec![r(
            0..5,
            0,
            8.0,
            2.0,
            vec![
                g(1, 0..1, 26.515625),
                g(2, 1..2, 26.515625),
                g(3, 2..3, 0.28125),
                g(4, 3..4, 10.03125),
                g(5, 4..5, 10.03125),
            ],
        )],
        &[],
        Style::core_default(),
    );
    let output = layout(&input, &constraints(7)).unwrap();
    assert_eq!(output.lines()[0].advance(), f(73.375));
    assert_eq!(
        output
            .glyphs()
            .iter()
            .map(|g| g.position().x().get())
            .collect::<Vec<_>>(),
        [0.0, 26.515625, 53.0, 53.25, 63.28125]
    );
    let input = event(
        "ab",
        vec![r(
            0..2,
            0,
            8.0,
            2.0,
            vec![g(1, 0..1, 26.515625), g(2, 1..2, 26.515625)],
        )],
        &[],
        Style::core_default(),
    );
    let output = layout(
        &input,
        &constraints(5).with_metric_scale_x(f(2.0)).unwrap(),
    )
    .unwrap();
    assert_eq!(output.lines()[0].advance(), f(53.0625));
    assert_eq!(output.glyphs()[0].position().x(), f(-26.5));
    for bad in [0.0, -1.0] {
        assert_eq!(
            constraints(5)
                .with_metric_scale_x(f(bad))
                .unwrap_err()
                .code(),
            ErrorCode::InvalidArgument
        );
    }
}
fn multiline() -> ShapedEvent {
    let a = g(1, 0..1, 10.0);
    let mut b = g(2, 1..2, 10.0);
    b.font = font(2);
    let mut first = r(0..2, 0, 8.0, 2.0, vec![a, b]);
    first.groups = vec![
        FaceMetrics {
            font: font(1),
            metrics: metrics(8.0, 2.0),
            glyphs: 0..1,
        },
        FaceMetrics {
            font: font(2),
            metrics: metrics(12.0, 4.0),
            glyphs: 1..2,
        },
    ];
    event(
        "ab\n\nc\n",
        vec![first, r(4..5, 0, 10.0, 3.0, vec![g(3, 4..5, 5.0)])],
        &[(3, 6.0, 2.0), (6, 9.0, 1.0)],
        Style::core_default(),
    )
}
#[test]
fn layout_numeric_all_nine_alignments_exact_baselines_and_fallback_metrics() {
    for alignment in 1..=9 {
        let input = multiline();
        let output = layout(&input, &constraints(alignment)).unwrap();
        let h = f64::from((alignment - 1) % 3) * 0.5;
        let v = 1.0 - f64::from((alignment - 1) / 3) * 0.5;
        let dy = -33.0 * v;
        assert_eq!(output.lines().len(), 3);
        assert_eq!(
            output.logical_bounds(),
            ScriptBounds::new(-20.0 * h, dy, 20.0 * (1.0 - h), dy + 33.0).unwrap()
        );
        for (i, ((baseline, width), (a, d))) in [12.0, 19.0, 30.0]
            .into_iter()
            .zip([20.0, 0.0, 5.0])
            .zip([(12.0, 4.0), (3.0, 1.0), (10.0, 3.0)])
            .enumerate()
        {
            let line = &output.lines()[i];
            assert_eq!(
                line.baseline(),
                ScriptPoint::new(f(-width * h), f(baseline + dy))
            );
            assert_eq!(
                (
                    line.ascent().get(),
                    line.descent().get(),
                    line.advance().get()
                ),
                (a, d, width)
            );
        }
        assert_eq!(output.lines()[1].glyph_range(), 2..2);
        assert_eq!(output.lines()[2].glyph_range(), 2..3);
        assert_eq!(output.shaped().runs()[0].glyphs()[1].font(), font(2));
    }
}
#[test]
fn layout_numeric_break_only_and_zero_text_have_actual_empty_metrics() {
    for (text, empty, height) in [
        (
            "\n\n",
            vec![(0, 8.0, 2.0), (1, 12.0, 3.0), (2, 6.0, 1.0)],
            12.5,
        ),
        ("", vec![(0, 5.0, 2.0)], 7.0),
    ] {
        let e = event(text, vec![], &empty, Style::core_default());
        let l = layout(&e, &constraints(7)).unwrap();
        assert_eq!(
            l.lines().len(),
            if text.ends_with('\n') {
                empty.len() - 1
            } else {
                empty.len()
            }
        );
        assert_eq!(l.logical_bounds().height(), height);
        assert!(l.glyphs().is_empty());
        assert_eq!(l.ink_bounds(), None);
    }
}
#[test]
fn layout_numeric_compacts_ascii_edge_but_preserves_nbsp_advance_and_source() {
    let mut a = g(1, 0..1, 3.0);
    a.ink = GlyphInk::Empty;
    let mut b = g(2, 1..3, 4.0);
    b.ink = GlyphInk::Empty;
    let e = event(
        " \u{a0}",
        vec![r(0..3, 0, 8.0, 2.0, vec![a, b])],
        &[],
        Style::core_default(),
    );
    let l = layout(&e, &constraints(7)).unwrap();
    assert_eq!(
        l.logical_bounds(),
        ScriptBounds::new(0.0, 0.0, 4.0, 10.0).unwrap()
    );
    assert_eq!(l.ink_bounds(), None);
    assert_eq!(l.collision_bounds(), l.logical_bounds());
    assert_eq!(e.source_map().text(), " \u{a0}");
    assert_eq!(e.runs()[0].glyphs().len(), 2);
    assert_eq!(l.glyphs().len(), 1);
    assert_eq!(l.glyphs()[0].cluster().text(), "\u{a0}");
    assert_eq!(l.glyphs()[0].position().x().get(), 0.0);
}
#[test]
fn layout_numeric_negative_offsets_and_mark_ink_are_distinct_from_logical_box() {
    let a = g(1, 0..3, 10.0);
    let mut mark = g(2, 0..3, 0.0);
    mark.x = -14.0;
    mark.y = -5.0;
    mark.ink = GlyphInk::Bounds(ScriptBounds::new(-2.0, -9.0, 2.0, -7.0).unwrap());
    let e = event(
        "a\u{301}",
        vec![r(0..3, 0, 8.0, 2.0, vec![a, mark])],
        &[],
        Style::core_default(),
    );
    let l = layout(&e, &constraints(7)).unwrap();
    assert_eq!(
        l.logical_bounds(),
        ScriptBounds::new(0.0, 0.0, 10.0, 10.0).unwrap()
    );
    assert_eq!(l.glyphs()[1].position(), ScriptPoint::new(f(-4.0), f(3.0)));
    assert_eq!(
        l.ink_bounds(),
        Some(ScriptBounds::new(-6.0, -6.0, 10.0, 9.0).unwrap())
    );
    assert_eq!(
        l.collision_bounds(),
        ScriptBounds::new(-6.0, -6.0, 10.0, 10.0).unwrap()
    );
    assert_eq!(l.glyphs()[0].cluster(), l.glyphs()[1].cluster());
}
#[test]
fn layout_numeric_bidi_reorders_run_groups_without_reversing_native_arrays() {
    let e = event(
        "abcde12z",
        vec![
            r(0..1, 0, 8.0, 2.0, vec![g(10, 0..1, 1.0)]),
            r(1..3, 1, 8.0, 2.0, vec![g(12, 2..3, 1.0), g(11, 1..2, 1.0)]),
            r(3..5, 1, 8.0, 2.0, vec![g(14, 4..5, 1.0), g(13, 3..4, 1.0)]),
            r(5..7, 2, 8.0, 2.0, vec![g(15, 5..6, 1.0), g(16, 6..7, 1.0)]),
            r(7..8, 0, 8.0, 2.0, vec![g(17, 7..8, 1.0)]),
        ],
        &[],
        Style::core_default(),
    );
    let l = layout(&e, &constraints(7)).unwrap();
    let ids: Vec<_> = l
        .glyphs()
        .iter()
        .map(|g| l.shaped().runs()[g.run_index()].glyphs()[g.glyph_index()].glyph())
        .collect();
    assert_eq!(ids, [10, 15, 16, 14, 13, 12, 11, 17]);
    assert_eq!(
        l.glyphs()
            .iter()
            .map(|g| g.position().x().get())
            .collect::<Vec<_>>(),
        [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]
    );
}
#[test]
fn layout_numeric_scaling_cluster_spacing_and_paint_independence() {
    let style = Style::core_default()
        .with_scale(f(2.0), f(3.0))
        .unwrap()
        .with_spacing(f(1.0));
    let build = |style| {
        event(
            "ab",
            vec![r(
                0..2,
                0,
                8.0,
                2.0,
                vec![g(1, 0..1, 10.0), g(2, 1..2, 5.0)],
            )],
            &[],
            style,
        )
    };
    let a = build(style.clone());
    let changed = style.clone().with_colors(
        Rgba8::new(7, 8, 9, 255),
        style.secondary(),
        style.outline(),
        style.shadow_color(),
    );
    let b = build(changed);
    let l = layout(&a, &constraints(7)).unwrap();
    assert_eq!(
        l.logical_bounds(),
        ScriptBounds::new(0.0, 0.0, 34.0, 30.0).unwrap()
    );
    assert_eq!(l.glyphs()[1].position().x().get(), 22.0);
    assert!(l.geometry_eq(&layout(&b, &constraints(7)).unwrap()));
    assert_eq!(l, layout(&a, &constraints(7)).unwrap());
}
#[test]
fn layout_numeric_checked_limits_unknown_ink_and_invalid_publication() {
    let e = multiline();
    for limits in [
        Limits::default().with_transient_bytes(1).unwrap(),
        Limits::default().with_glyphs_per_event(1).unwrap(),
        Limits::default().with_max_dimension(1).unwrap(),
        Limits::default().with_text_bytes_per_event(1).unwrap(),
    ] {
        assert_eq!(
            layout(&e, &constraints(7).with_limits(limits))
                .unwrap_err()
                .code(),
            ErrorCode::LimitExceeded
        );
    }
    let mut unknown = g(1, 0..1, 2.0);
    unknown.ink = GlyphInk::Unknown;
    let e = event(
        "a",
        vec![r(0..1, 0, 8.0, 2.0, vec![unknown])],
        &[],
        Style::core_default(),
    );
    assert_eq!(
        layout(&e, &constraints(7)).unwrap_err().code(),
        ErrorCode::UnsupportedFeature
    );
    assert!(ScriptBounds::new(f64::INFINITY, 0.0, 1.0, 1.0).is_err());
    assert!(ScriptBounds::new(2.0, 0.0, 1.0, 1.0).is_err());
    assert!(constraints(7).with_scale(f(-1.0), f(1.0)).is_err());
    assert_eq!(
        layout(&e, &constraints(7).with_wrap(WrapMode::new(0).unwrap()))
            .unwrap_err()
            .code(),
        ErrorCode::UnsupportedFeature
    );
    assert_eq!(
        layout(&e, &constraints(7).with_profile(Profile::LegacyXyV1))
            .unwrap_err()
            .code(),
        ErrorCode::UnsupportedFeature
    );
    let blank = event("", vec![], &[], Style::core_default());
    assert_eq!(
        layout(&blank, &constraints(7)).unwrap_err().code(),
        ErrorCode::InvalidArgument
    );
}
