use taiyou_render::renderer::*;

fn parsed(tags: &str) -> ParsedTrack {
    let ass = format!(
        "[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow\nStyle: Default,Source Sans 3,30,0,0\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:01.00,0:00:03.00,{{{tags}}}Ag\n"
    );
    taiyou_ass::parse_ass(
        ass.as_bytes(),
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
fn at(tags: &str, local_ms: i64) -> EvaluatedEvent {
    let p = parsed(tags);
    taiyou_render::evaluate(
        p.track(),
        p.track().events()[0].id(),
        TimeUs::from_micros((1000 + local_ms) * 1000),
        Profile::CoreV1,
    )
    .unwrap()
}
#[test]
fn transform_timing_acceleration_signed_targets_and_source_order() {
    assert_eq!(at(r"\blur4\t(,\blur0)", 1000).runs()[0].effects().blur().get(),2.0);
    assert_eq!(at(r"\blur4\t(0,500,\blur0", 250).runs()[0].effects().blur().get(),2.0);
    assert_eq!(at(r"\blur4\t(0,1,2,3,\blur0)", 1000).runs()[0].effects().blur().get(),4.0);
    let immediate = at(r"\t(\pos(10,20)\blur6)",0);
    assert_eq!(immediate.placement().anchor(),Anchor::Explicit(ScriptPoint::new(FiniteF64::new(10.0).unwrap(),FiniteF64::new(20.0).unwrap())));
    assert_eq!(immediate.runs()[0].effects().blur().get(),6.0); // first ')' ended t before blur
    for (ms, expected) in [(0, 4.0), (100, 4.0), (600, 2.0), (1100, 0.0), (1999, 0.0)] {
        let e = at(r"\blur4\t(100,1100,\blur0)", ms);
        assert_eq!(e.runs()[0].effects().blur().get(), expected);
        assert!(!e.animation().is_static());
    }
    assert_eq!(
        at(r"\blur4\t(\blur-4)", 500).runs()[0]
            .effects()
            .blur()
            .get(),
        2.0
    );
    assert_eq!(
        at(r"\blur4\t(\blur-4)", 1000).runs()[0]
            .effects()
            .blur()
            .get(),
        0.0
    );
    assert_eq!(
        at(r"\blur4\t(2,\blur0)", 1000).runs()[0]
            .effects()
            .blur()
            .get(),
        3.0
    );
    assert_eq!(
        at(r"\blur4\t(\blur0)\t(\blur8)", 1000).runs()[0]
            .effects()
            .blur()
            .get(),
        5.0
    );
    assert_eq!(
        at(r"\blur4\t(\blur0)\blur7", 1000).runs()[0]
            .effects()
            .blur()
            .get(),
        7.0
    );
    assert_eq!(
        at(r"\blur4\t(1000,500,\blur0)", 750).runs()[0]
            .effects()
            .blur()
            .get(),
        4.0
    );
    assert_eq!(
        at(r"\blur4\t(1000,500,\blur0)", 1000).runs()[0]
            .effects()
            .blur()
            .get(),
        0.0
    );
    assert_eq!(
        at(r"\be4\t(\be-4)", 1500).runs()[0].effects().be().get(),
        -2.0
    );
}
#[test]
fn supported_numeric_paint_and_immediate_tags_follow_xy_order() {
    let e = at(
        r"\bord2\shad2\t(\bord6\shad6\fs50\fscx200\fscy50\fsp2\1c&HFF0000&\2c&H00FF00&\3c&H0000FF&\4c&HFFFFFF&\alpha&HFF&)",
        1000,
    );
    let r = &e.runs()[0];
    assert_eq!(r.effects().border_x().get(), 4.0);
    assert_eq!(r.effects().shadow_x().get(), 4.0);
    assert_eq!(r.shaping().font_size().get(), 40.0);
    assert_eq!(r.shaping().spacing().get(), 1.0);
    assert_eq!(r.geometry().scale_x().get(), 1.5);
    assert_eq!(r.geometry().scale_y().get(), 0.75);
    assert_eq!(r.paint().primary().channels(), [127, 127, 255, 128]);
    let e = at(r"\t(\fnOther\b1\i1\an7\q1\warpmode0)", 0);
    assert_eq!(e.runs()[0].shaping().font().family(), "Other");
    assert_eq!(e.runs()[0].shaping().font().weight(), 700);
    assert!(e.runs()[0].shaping().font().italic());
    assert_eq!(e.placement().alignment().get(), 7);
    assert_eq!(e.wrap_mode().get(), 1);
    assert_eq!(
        at(r"\blur4\t(\r\blur)", 1000).runs()[0]
            .effects()
            .blur()
            .get(),
        0.0
    );
}

#[cfg(feature = "native")]
#[test]
fn animated_frames_equal_static_states_and_seek_without_history() {
    use std::sync::Arc;
    use taiyou_render::renderer::*;
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
    let animated = parsed(r"\pos(80,60)\blur4\bord2\t(\blur0\bord6\be2)");
    let mut renderer = Renderer::new(RendererConfig::default()).unwrap();
    for ms in [1000, 0, 1500, 500, 1000] {
        let request = RenderRequest {
            now: TimeUs::from_micros((1000 + ms) * 1000),
            viewport: Viewport::full(Size::new(160, 100)).unwrap(),
            unsupported: UnsupportedPolicy::Strict,
        };
        let actual = renderer
            .render(animated.track().clone(), fonts.clone(), &request)
            .unwrap();
        let p = ms as f64 / 2000.0;
        let expected = parsed(&format!(
            "\\pos(80,60)\\blur{}\\bord{}\\be{}",
            4.0 * (1.0 - p),
            2.0 + 4.0 * p,
            2.0 * p
        ));
        let fresh = Renderer::new(RendererConfig::default())
            .unwrap()
            .render(Arc::clone(expected.track()), fonts.clone(), &request)
            .unwrap();
        assert_eq!(actual.pixels(), fresh.pixels());
        assert!(actual.pixels().iter().any(|b| *b != 0));
    }
}
