#![cfg(feature = "native")]
use taiyou_render::renderer::*;

fn parsed(text: &str) -> ParsedTrack {
    let source = format!(
        "[Script Info]\nPlayResX: 160\nPlayResY: 120\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow,Alignment\nStyle: Default,Source Sans 3,30,0,0,5\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:02.00,{text}\n"
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
fn q2_compacts_ascii_edges_without_erasing_source_nbsp_or_blank_metrics() {
    use taiyou_render::{layout, shaping::shape_event};
    let mut context = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    builder
        .register(
            &mut context,
            &std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap()).unwrap(),
            0,
        )
        .unwrap();
    let fonts = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let request = RenderRequest {
        now: TimeUs::from_micros(500_000),
        viewport: Viewport::full(Size::new(160, 120)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    };
    let frame = |text: &str| {
        Renderer::new(RendererConfig::default())
            .unwrap()
            .render(parsed(text).track().clone(), fonts.clone(), &request)
            .unwrap()
    };
    let expected = frame("ABC");
    for input in ["   ABC", "ABC   ", "   ABC   "] {
        assert_eq!(frame(input).pixels(), expected.pixels(), "{input}");
        let p = parsed(input);
        let event = taiyou_render::evaluate(
            p.track(),
            p.track().events()[0].id(),
            request.now,
            Profile::CoreV1,
        )
        .unwrap();
        assert_eq!(event.source_map().unwrap().text(), input);
        let shaped = shape_event(
            &mut context,
            &fonts,
            &event,
            &Default::default(),
            &Limits::default(),
        )
        .unwrap();
        let local = layout(&shaped, &LayoutConstraints::new(Alignment::new(5).unwrap())).unwrap();
        assert_eq!(local.lines()[0].decoded().text(), input);
        assert!(local.glyphs().iter().all(|g| g.cluster().text() != " "));
        assert_eq!(
            local.lines()[0]
                .decoded()
                .original_ranges(&Limits::default())
                .unwrap()[0]
                .slice(p.track().source()),
            Some(input)
        );
    }
    assert_eq!(
        frame(r"   ABC   \N   DEF   ").pixels(),
        frame(r"ABC\NDEF").pixels()
    );
    let p = parsed(r"\hABC\h");
    let event = taiyou_render::evaluate(
        p.track(),
        p.track().events()[0].id(),
        request.now,
        Profile::CoreV1,
    )
    .unwrap();
    assert_eq!(event.source_map().unwrap().text(), "\u{a0}ABC\u{a0}");
    let shaped = shape_event(
        &mut context,
        &fonts,
        &event,
        &Default::default(),
        &Limits::default(),
    )
    .unwrap();
    let local = layout(&shaped, &LayoutConstraints::new(Alignment::new(5).unwrap())).unwrap();
    assert!(
        local
            .glyphs()
            .iter()
            .any(|g| g.cluster().text() == "\u{a0}")
    );
    let blank = parsed(r"   \N\NABC");
    let event = taiyou_render::evaluate(
        blank.track(),
        blank.track().events()[0].id(),
        request.now,
        Profile::CoreV1,
    )
    .unwrap();
    let shaped = shape_event(
        &mut context,
        &fonts,
        &event,
        &Default::default(),
        &Limits::default(),
    )
    .unwrap();
    let local = layout(&shaped, &LayoutConstraints::new(Alignment::new(5).unwrap())).unwrap();
    assert_eq!(local.lines().len(), 3);
    assert!(local.lines()[0].glyph_range().is_empty());
    assert_eq!(local.lines()[0].advance().get(), 0.0);
    assert_eq!(local.lines()[0].ascent(), local.lines()[2].ascent());
    assert!(local.lines()[1].ascent().get() < local.lines()[0].ascent().get());
    assert_eq!(local.lines()[0].decoded().text(), "   ");
}
