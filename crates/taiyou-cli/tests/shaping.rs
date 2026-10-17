#![cfg(feature = "native")]
use taiyou_render::shaping::*;
#[test]
fn shaping_actual_parser_evaluator_source_pipeline_retains_exact_intervals() {
    let shaped = {
        let bytes = b"\xef\xbb\xbf[Script Info]\r\nWrapStyle: 2\r\n[V4+ Styles]\r\nFormat: Name, Fontname, Fontsize\r\nStyle: Default,Source Sans 3,1000\r\n[Events]\r\nFormat: Start,End,Name,Text\r\nDialogue: 0:00:00.00,0:00:01.00,actor\\N,f{\\c&HFF0000&}i\\Nfi\\nA\\hV".to_vec();
        let options = Default::default();
        let mut ingestion = Default::default();
        let parsed = taiyou_ass::parse_ass(&bytes, &options, &mut ingestion).unwrap();
        let track = parsed.track();
        let event = &track.events()[0];
        let evaluated =
            taiyou_render::evaluate(track, event.id(), event.start(), options.profile()).unwrap();
        assert_eq!(evaluated.source_map().unwrap().text(), "fi\nfi\nA\u{a0}V");
        let mut ctx = FontContext::new().unwrap();
        let mut builder = FontCatalogBuilder::new(Default::default());
        builder
            .register(
                &mut ctx,
                &std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap()).unwrap(),
                0,
            )
            .unwrap();
        let fonts = builder.snapshot(&mut Default::default()).unwrap();
        shape_event(
            &mut ctx,
            &fonts,
            &evaluated,
            &ShapeOptions::default(),
            &Default::default(),
        )
        .unwrap()
    };
    let first = &shaped.runs()[0].glyphs()[0];
    let ranges = first
        .cluster()
        .original_ranges(&Default::default())
        .unwrap();
    assert_eq!(ranges.len(), 2);
    assert_eq!(
        ranges[0].slice(shaped.source_map().original_source()),
        Some("f")
    );
    assert_eq!(
        ranges[1].slice(shaped.source_map().original_source()),
        Some("i")
    );
    assert!(ranges[0].end() < ranges[1].start());
    assert_eq!(shaped.breaks().len(), 2);
    for (span, expected) in shaped.breaks().iter().zip(["\\N", "\\n"]) {
        let source = span.original_ranges(&Default::default()).unwrap();
        assert_eq!(source.len(), 1);
        assert_eq!(
            source[0].slice(shaped.source_map().original_source()),
            Some(expected)
        );
    }
    assert!(
        shaped
            .source_map()
            .original_source()
            .starts_with('\u{feff}')
    );
}
