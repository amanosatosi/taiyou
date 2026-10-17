#![cfg(feature = "native")]
use taiyou_render::{
    layout::{LayoutConstraints, layout},
    shaping::*,
};
#[test]
fn layout_pipeline_scale_boundaries_preserve_context_and_paint_boundaries_coalesce() {
    for (text, glyph_count) in [("f{\\fscx200}f", 2), ("f{\\c&HFF0000&}f", 1)] {
        let input = format!("[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Alignment\nStyle: Default,Source Sans 3,20,7\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:01.00,{text}");
        let options = Default::default();
        let parsed = taiyou_ass::parse_ass(input.as_bytes(), &options, &mut Default::default()).unwrap();
        let event = &parsed.track().events()[0];
        let evaluated = taiyou_render::evaluate(parsed.track(), event.id(), event.start(), options.profile()).unwrap();
        let mut ctx = FontContext::new().unwrap();
        let mut builder = FontCatalogBuilder::new(Default::default());
        builder.register(&mut ctx, &std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap()).unwrap(), 0).unwrap();
        let fonts = builder.snapshot(&mut Default::default()).unwrap();
        let s = shape_event(&mut ctx, &fonts, &evaluated, &ShapeOptions::default(), &Default::default()).unwrap();
        let l = layout(&s, &LayoutConstraints::new(s.placement().alignment())).unwrap();
        assert_eq!(l.glyphs().len(), glyph_count);
        if glyph_count == 2 {
            let advance = 292.0 * (20.0 / 1000.0);
            assert_eq!(l.glyphs()[1].position().x().get(), advance);
            assert_eq!(l.lines()[0].advance().get(), advance + advance * 2.0);
        }
    }
}
#[test]
fn layout_pipeline_actual_ass_escapes_alignment_and_retained_source() {
    for alignment in 1..=9 {
        let output = {
            let input = format!(
                "[Script Info]\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize\nStyle: Default,Source Sans 3,20\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:01.00,{{\\an{alignment}}}\\Nf{{\\c&HFF0000&}}f\\N\\nA\\hV\\N{{\\fs40}}"
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
            let shaped = shape_event(
                &mut ctx,
                &fonts,
                &evaluated,
                &ShapeOptions::default(),
                &Default::default(),
            )
            .unwrap();
            assert_eq!(shaped.placement().alignment().get(), alignment);
            layout(
                &shaped,
                &LayoutConstraints::new(shaped.placement().alignment()),
            )
            .unwrap()
        };
        assert_eq!(output.lines().len(), 5);
        assert!(output.lines()[0].glyph_range().is_empty());
        assert!(output.lines()[2].glyph_range().is_empty());
        assert!(output.lines()[4].glyph_range().is_empty());
        assert_eq!(output.lines()[4].ascent().get(), 40.0);
        let ligature = output
            .glyphs()
            .iter()
            .find(|g| g.cluster().text() == "ff")
            .unwrap();
        let spans = ligature
            .cluster()
            .original_ranges(&Default::default())
            .unwrap();
        assert_eq!(spans.len(), 2);
        assert!(spans[0].end() < spans[1].start());
        assert!(output.glyphs().iter().any(|g| {
            g.cluster()
                .original_ranges(&Default::default())
                .unwrap()
                .iter()
                .any(|s| s.slice(output.shaped().source_map().original_source()) == Some("\\h"))
        }));
        let bounds = output.logical_bounds();
        match (alignment - 1) % 3 {
            0 => assert_eq!(bounds.x0().get(), 0.0),
            1 => assert_eq!(bounds.x0().get(), -bounds.x1().get()),
            _ => assert_eq!(bounds.x1().get(), 0.0),
        };
        match (alignment - 1) / 3 {
            0 => assert!(bounds.y1().get().abs() < 1e-12),
            1 => assert!((bounds.y0().get() + bounds.y1().get()).abs() < 1e-12),
            _ => assert_eq!(bounds.y0().get(), 0.0),
        };
    }
}
#[test]
fn layout_pipeline_empty_and_break_only_events_have_real_metrics() {
    for text in ["", "\\N", "\\N\\N", "{\\fs40}"] {
        let input = format!(
            "[V4+ Styles]\nFormat: Name,Fontname,Fontsize\nStyle: Default,Source Sans 3,20\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:01.00,{text}"
        );
        let options = Default::default();
        let parsed =
            taiyou_ass::parse_ass(input.as_bytes(), &options, &mut Default::default()).unwrap();
        let event = &parsed.track().events()[0];
        let evaluated =
            taiyou_render::evaluate(parsed.track(), event.id(), event.start(), options.profile())
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
        let s = shape_event(
            &mut ctx,
            &fonts,
            &evaluated,
            &ShapeOptions::default(),
            &Default::default(),
        )
        .unwrap();
        let l = layout(&s, &LayoutConstraints::new(s.placement().alignment())).unwrap();
        assert!(l.glyphs().is_empty());
        assert_eq!(l.ink_bounds(), None);
        assert!(l.logical_bounds().height() > 0.0);
        assert_eq!(
            l.lines()[0].ascent().get(),
            if text.contains("40") { 40.0 } else { 20.0 }
        );
    }
}
