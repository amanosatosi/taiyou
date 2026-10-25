#![cfg(feature = "native")]
use taiyou_render::shaping::*;
#[test]
fn shaping_parser_utf8_combining_and_ligature_gaps_survive_input_release() {
    for (text, expected) in [
        ("f{\\c&HFF0000&}f", ["f", "f"]),
        ("éa{\\c&HFF0000&}\u{301}", ["a", "\u{301}"]),
    ] {
        let shaped = {
            let input = format!(
                "[V4+ Styles]\nFormat: Name,Fontname,Fontsize\nStyle: Default,Source Sans 3,1000\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:01.00,{text}"
            );
            let options = Default::default();
            let parsed =
                taiyou_ass::parse_ass(input.as_bytes(), &options, &mut Default::default()).unwrap();
            let track = parsed.track();
            let event = &track.events()[0];
            let evaluated =
                taiyou_render::evaluate(track, event.id(), event.start(), options.profile())
                    .unwrap();
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
        let requested = expected.concat();
        let start = shaped.source_map().text().rfind(&requested).unwrap() as u32;
        let mut spans: Vec<_> = shaped.runs()[0]
            .glyphs()
            .iter()
            .filter(|g| g.cluster().start() >= start)
            .flat_map(|g| g.cluster().original_ranges(&Default::default()).unwrap())
            .collect();
        spans.sort_by_key(|s| s.start());
        spans.dedup();
        assert_eq!(spans.len(), 2);
        assert!(spans[0].end() < spans[1].start());
        for (span, expected) in spans.iter().zip(expected) {
            assert_eq!(
                span.slice(shaped.source_map().original_source()),
                Some(expected)
            );
        }
    }
}
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
    let second = shaped.runs()[0].glyphs()[1]
        .cluster()
        .original_ranges(&Default::default())
        .unwrap();
    assert_eq!(ranges.len(), 1);
    assert_eq!(
        ranges[0].slice(shaped.source_map().original_source()),
        Some("f")
    );
    assert_eq!(
        second[0].slice(shaped.source_map().original_source()),
        Some("i")
    );
    assert!(ranges[0].end() < second[0].start());
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
