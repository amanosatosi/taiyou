use taiyou_render::renderer::*;

fn parsed(text: &str, q: u8, event_margin: u32) -> ParsedTrack {
    let source = format!(
        "[Script Info]\nPlayResX: 90\nPlayResY: 120\nWrapStyle: {q}\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow,Alignment,MarginL,MarginR,MarginV\nStyle: Default,Source Sans 3,20,0,0,7,10,10,5\n[Events]\nFormat: Start,End,MarginL,MarginR,Text\nDialogue: 0:00:00.00,0:00:02.00,{event_margin},{event_margin},{text}"
    );
    taiyou_ass::parse_ass(
        source.as_bytes(),
        &ParseOptions::new(
            InputEncoding::Utf8,
            Profile::CoreV1,
            Limits::default(),
            SyntaxPolicy::Strict,
        ),
        &mut TrackIngestor::new(),
    )
    .unwrap()
}

#[test]
fn q_prefix_parameters_resets_and_soft_escapes_follow_source_order() {
    for (tag, expected) in [
        (r"\q0", 0),
        (r"\q1", 1),
        (r"\q2", 2),
        (r"\q3", 3),
        (r"\q", 2),
        (r"\q4", 2),
        (r"\q-1", 2),
        (r"\qjunk", 0),
        (r"\q1.5", 1),
        (r"\q+3tail", 3),
        (r"\q(2)", 2),
        (r"\q1(foo)", 1),
        (r"\q1()", 1),
        (r"\q(,3)", 2),
        (r"\q2(3)", 2),
        (r"\q999999999999999999", 2),
    ] {
        let track = parsed(&format!("{{{tag}}}A"), 2, 0);
        let e = taiyou_render::evaluate(
            track.track(),
            EventId::new(0),
            TimeUs::from_micros(1),
            Profile::CoreV1,
        )
        .unwrap();
        assert_eq!(e.wrap_mode().get(), expected, "{tag}");
    }
    for header in 0..=3 {
        for tag in [r"\q(2)", r"\q(,3)", r"\q"] {
            let input = parsed(&format!("{{\\q1}}{{{tag}}}A\\nB"), header, 0);
            let event = taiyou_render::evaluate(
                input.track(),
                EventId::new(0),
                TimeUs::from_micros(1),
                Profile::CoreV1,
            )
            .unwrap();
            assert_eq!(event.wrap_mode().get(), header, "header={header} {tag}");
            assert_eq!(
                event.source_map().unwrap().text(),
                if header == 2 { "A\nB" } else { "A B" }
            );
        }
    }
    let input = parsed(r"{\q2}A\nB{\q0}C\nD{\r\q2}E\nF", 1, 0);
    let e = taiyou_render::evaluate(
        input.track(),
        EventId::new(0),
        TimeUs::from_micros(1),
        Profile::CoreV1,
    )
    .unwrap();
    assert_eq!(e.source_map().unwrap().text(), "A\nBC DE\nF");
    assert_eq!(e.wrap_mode().get(), 2);
    let input = parsed(r"{\warpmode1\q1\r\q2}A\nB\NC\hD", 0, 0);
    let e = taiyou_render::evaluate(
        input.track(),
        EventId::new(0),
        TimeUs::from_micros(1),
        Profile::CoreV1,
    )
    .unwrap();
    assert_eq!(e.warp_mode(), WarpMode::Unicode);
    assert_eq!(e.source_map().unwrap().text(), "A\nB\nC\u{a0}D");
}

#[cfg(feature = "native")]
#[test]
fn missing_wrapstyle_matches_q0_and_wraps_automatically() {
    use taiyou_render::{layout, shaping::shape_event};
    let explicit = parsed("AA AA AA", 0, 0);
    let source = explicit.track().source().replace("WrapStyle: 0\n", "");
    let missing = taiyou_ass::parse_ass(
        source.as_bytes(),
        &ParseOptions::new(
            InputEncoding::Utf8,
            Profile::CoreV1,
            Limits::default(),
            SyntaxPolicy::Strict,
        ),
        &mut TrackIngestor::new(),
    )
    .unwrap();
    assert_eq!(missing.track().script().wrap().get(), 0);
    let mut ctx = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    let font = std::path::PathBuf::from(std::env::var_os("RUNNER_TEMP").unwrap())
        .join("taiyou-shaping-fonts/latin.otf");
    builder
        .register(&mut ctx, &std::fs::read(font).unwrap(), 0)
        .unwrap();
    let fonts = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let mut expected_pixels = None;
    for input in [&explicit, &missing] {
        let evaluated = taiyou_render::evaluate(
            input.track(),
            EventId::new(0),
            TimeUs::from_micros(1),
            Profile::CoreV1,
        )
        .unwrap();
        let shaped = shape_event(
            &mut ctx,
            &fonts,
            &evaluated,
            &Default::default(),
            &Limits::default(),
        )
        .unwrap();
        let constraints = LayoutConstraints::new(Alignment::new(7).unwrap())
            .with_wrap(evaluated.wrap_mode())
            .with_width(FiniteF64::new(70.0).unwrap())
            .unwrap();
        let local = layout(&shaped, &constraints).unwrap();
        let lines: Vec<_> = local
            .lines()
            .iter()
            .map(|line| line.decoded().text().trim_matches(' '))
            .collect();
        assert_eq!(lines, ["AA AA", "AA"]);
        let request = RenderRequest {
            now: TimeUs::from_micros(1),
            viewport: Viewport::full(Size::new(90, 120)).unwrap(),
            unsupported: UnsupportedPolicy::Strict,
        };
        let frame = Renderer::new(Default::default())
            .unwrap()
            .render(input.track().clone(), fonts.clone(), &request)
            .unwrap();
        if let Some(pixels) = &expected_pixels {
            assert_eq!(frame.pixels(), pixels);
        } else {
            expected_pixels = Some(frame.pixels().to_vec());
        }
    }
}

#[cfg(feature = "native")]
#[test]
fn actual_automatic_lines_margins_overwide_words_and_seeks_render() {
    use taiyou_render::{layout, shaping::shape_event};
    let mut ctx = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    let font = std::path::PathBuf::from(std::env::var_os("RUNNER_TEMP").unwrap())
        .join("taiyou-shaping-fonts/latin.otf");
    builder
        .register(&mut ctx, &std::fs::read(font).unwrap(), 0)
        .unwrap();
    let fonts = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    for (q, text, margin, expected) in [
        (0, "AA AA AA", 0, vec!["AA AA", "AA"]),
        (1, "AA AA AA", 0, vec!["AA AA", "AA"]),
        (2, "AA AA AA", 0, vec!["AA AA AA"]),
        (3, "AA AA AA", 0, vec!["AA", "AA AA"]),
        (0, r"AA\N\NBB\N", 0, vec!["AA", "", "BB"]),
        (1, "  AA  ", 0, vec!["AA"]),
        (1, "AA AA AA", 25, vec!["AA", "AA", "AA"]),
        (1, r"AA\nBB\NCC\hDD", 25, vec!["AA", "BB", "CC\u{a0}DD"]),
        (
            1,
            "ABCDEFGHIJKLMNOPQRSTUV",
            25,
            vec!["ABCDEFGHIJKLMNOPQRSTUV"],
        ),
    ] {
        let input = parsed(text, q, margin);
        let evaluated = taiyou_render::evaluate(
            input.track(),
            EventId::new(0),
            TimeUs::from_micros(1),
            Profile::CoreV1,
        )
        .unwrap();
        let shaped = shape_event(
            &mut ctx,
            &fonts,
            &evaluated,
            &Default::default(),
            &Limits::default(),
        )
        .unwrap();
        let width = if margin == 0 {
            70.0
        } else {
            90.0 - 2.0 * f64::from(margin)
        };
        let constraints = LayoutConstraints::new(Alignment::new(7).unwrap())
            .with_wrap(evaluated.wrap_mode())
            .with_width(FiniteF64::new(width).unwrap())
            .unwrap();
        let local = layout(&shaped, &constraints).unwrap();
        let lines: Vec<_> = local
            .lines()
            .iter()
            .map(|l| l.decoded().text().trim_matches(' '))
            .collect();
        assert_eq!(lines, expected, "q{q} {text} margin={margin}");
        for glyph in local.glyphs() {
            assert!(
                !glyph
                    .cluster()
                    .original_ranges(&Limits::default())
                    .unwrap()
                    .is_empty()
            );
        }
        let mut renderer = Renderer::new(Default::default()).unwrap();
        for time in [1, 500_000, 2_000_000, 750_000, 0, 500_000] {
            let request = RenderRequest {
                now: TimeUs::from_micros(time),
                viewport: Viewport::full(Size::new(90, 120)).unwrap(),
                unsupported: UnsupportedPolicy::Strict,
            };
            let frame = renderer
                .render(input.track().clone(), fonts.clone(), &request)
                .unwrap();
            let fresh = Renderer::new(Default::default())
                .unwrap()
                .render(input.track().clone(), fonts.clone(), &request)
                .unwrap();
            assert_eq!(frame.pixels(), fresh.pixels());
            assert_eq!(frame.diagnostics(), fresh.diagnostics());
            if time < 2_000_000 {
                assert!(frame.pixels().chunks_exact(4).any(|p| p[3] != 0));
            }
        }
        let mut baseline = None;
        for selector in ["", r"{\warpmode0}"] {
            let input = parsed(&format!("{selector}{text}"), q, margin);
            let request = RenderRequest {
                now: TimeUs::from_micros(1),
                viewport: Viewport::full(Size::new(90, 120)).unwrap(),
                unsupported: UnsupportedPolicy::Strict,
            };
            let frame = Renderer::new(Default::default())
                .unwrap()
                .render(input.track().clone(), fonts.clone(), &request)
                .unwrap();
            if let Some(pixels) = &baseline {
                assert_eq!(frame.pixels(), pixels);
            } else {
                baseline = Some(frame.pixels().to_vec());
            }
        }
    }
    // Exercise automatic line geometry through PR29's complete-active-set
    // collision pipeline: identical opaque events must occupy more pixels.
    for q in [0, 1, 3] {
        let single = parsed("AA AA AA", q, 0);
        let source = format!(
            "{}\nDialogue: 0:00:00.00,0:00:02.00,0,0,AA AA AA",
            single.track().source()
        );
        let pair = taiyou_ass::parse_ass(
            source.as_bytes(),
            &ParseOptions::new(
                InputEncoding::Utf8,
                Profile::CoreV1,
                Limits::default(),
                SyntaxPolicy::Strict,
            ),
            &mut TrackIngestor::new(),
        )
        .unwrap();
        let request = RenderRequest {
            now: TimeUs::from_micros(1),
            viewport: Viewport::full(Size::new(90, 120)).unwrap(),
            unsupported: UnsupportedPolicy::Strict,
        };
        let render = |input: &ParsedTrack| {
            Renderer::new(Default::default())
                .unwrap()
                .render(input.track().clone(), fonts.clone(), &request)
                .unwrap()
        };
        let one = render(&single);
        let two = render(&pair);
        let occupied = |frame: &Frame| frame.pixels().chunks_exact(4).filter(|p| p[3] != 0).count();
        assert!(occupied(&two) > occupied(&one), "q{q} collision placement");
        assert_eq!(two.pixels(), render(&pair).pixels());
    }
}
