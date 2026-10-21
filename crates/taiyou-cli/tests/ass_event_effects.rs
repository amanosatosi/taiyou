use taiyou_render::renderer::*;

fn parsed(body: &str) -> ParsedTrack {
    let ass = format!(
        "[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow,Alignment,Angle\nStyle: Default,Source Sans 3,30,0,0,7,0\nStyle: Other,Source Sans 3,30,0,0,7,30\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:01.00,0:00:03.00,{body}\n"
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
fn at_body(body: &str, us: i64) -> EvaluatedEvent {
    let p = parsed(body);
    taiyou_render::evaluate(
        p.track(),
        p.track().events()[0].id(),
        TimeUs::from_micros(1_000_000 + us),
        Profile::CoreV1,
    )
    .unwrap()
}
fn at(tags: &str, ms: i64) -> EvaluatedEvent {
    at_body(&format!("{{{tags}}}Ag"), ms * 1000)
}
fn xy(e: &EvaluatedEvent) -> (f64, f64) {
    let p = e
        .placement()
        .ass_anchor(1.0, 1.0)
        .unwrap()
        .expect("not explicit");
    (p.x().get(), p.y().get())
}
fn alpha(tags: &str, ms: i64) -> u8 {
    at(tags, ms).runs()[0].paint().primary().a()
}
#[test]
fn move_timing_slots_and_resets_follow_event_local_integer_milliseconds() {
    for (ms, x) in [(0, 0.0), (500, 25.0), (1000, 50.0), (1999, 99.875)] {
        assert_eq!(xy(&at(r"\move(0,0,100,100)", ms)), (x, x));
    }
    for (ms, x) in [
        (0, 0.0),
        (199, 0.0),
        (200, 0.0),
        (500, 50.0),
        (800, 100.0),
        (1999, 100.0),
    ] {
        for tag in [r"\move(0,0,100,100,200,800)", r"\move(0,0,100,100,800,200)"] {
            assert_eq!(xy(&at(tag, ms)), (x, x));
        }
    }
    assert_eq!(xy(&at(r"\move(0,0,100,100,400,400)", 400)), (0.0, 0.0));
    assert_eq!(xy(&at(r"\move(0,0,100,100,400,400)", 401)), (100.0, 100.0));
    assert_eq!(xy(&at(r"\move(0,0,100,100,0,0)", 1000)), (50.0, 50.0));
    assert_eq!(xy(&at(r"\move(0,0,100,100,-20,-10)", 1000)), (50.0, 50.0));
    for (tag, expected) in [
        (r"\pos(10,20)\move(0,0,100,100)", (10.0, 20.0)),
        (r"\move(0,0,100,100)\pos(10,20)", (25.0, 25.0)),
        (r"\move(0,0,100,100)\move(200,200,400,400)", (25.0, 25.0)),
        (r"\pos(10,20)\pos(30,40)", (10.0, 20.0)),
        (r"\move(0,0,100,100)\rOther", (25.0, 25.0)),
        (r"\t(1000,1500,\move(0,0,100,100))", (25.0, 25.0)),
        (r"\move(1,2,3)\pos(10,20)", (10.0, 20.0)),
    ] {
        let e = at(tag, 500);
        assert_eq!(xy(&e), expected, "{tag}");
        assert!(!e.placement().collision_eligible());
    }
    assert_eq!(
        xy(&at_body(r"{\move(0,0,100,100)}Ag", 500_999)),
        (25.0, 25.0)
    );
}
#[test]
fn origin_is_first_wins_independent_of_placement_and_persistent_across_resets() {
    for tag in [
        r"\org(1,2)\org(8,9)\pos(10,20)\r",
        r"\pos(10,20)\org(1,2)\rOther\org(8,9)",
        r"\t(1000,1500,\org(1,2))\pos(10,20)",
    ] {
        let e = at(tag, 0);
        let p = e.placement().origin().unwrap();
        assert_eq!((p.x().get(), p.y().get()), (1.0, 2.0));
        assert_eq!(xy(&e), (10.0, 20.0));
        assert!(!e.placement().collision_eligible());
    }
    let e = at(r"\org(1,2)", 0);
    assert_eq!(e.placement().anchor(), Anchor::Default);
    assert!(!e.placement().collision_eligible());
    assert_eq!(at("", 0).placement().origin(), None);
}
#[test]
fn rotation_uses_existing_geometry_and_ordered_transform_property() {
    for (tag, expected) in [
        (r"\fr-30.5", -30.5),
        (r"\frz765", 765.0),
        (r"\frz45\fr-30", -30.0),
        (r"\frz45\r", 0.0),
        (r"\frz45\rOther", 30.0),
        (r"\rOther\fr", 0.0),
        (r"\frz10\t(0,1000,\frz90)", 50.0),
        (r"\fr10\t(0,1000,\fr90)", 50.0),
        (r"\fr40\t(\fr)", 0.0),
    ] {
        assert_eq!(
            at(tag, 500).runs()[0].geometry().rotation_z().get(),
            expected,
            "{tag}"
        );
    }
    use taiyou_render::geometry::{DeviceMapping, GeometryMap};
    let point = |x, y| ScriptPoint::new(FiniteF64::new(x).unwrap(), FiniteF64::new(y).unwrap());
    let map = GeometryMap::new(
        (FiniteF64::ONE, FiniteF64::ONE),
        point(0.0, 0.0),
        point(20.0, 30.0),
        Some(point(0.0, 0.0)),
        FiniteF64::new(90.0).unwrap(),
        DeviceMapping::new(Size::new(160, 100), PixelRect::new(0, 0, 160, 100).unwrap()).unwrap(),
    )
    .unwrap();
    let p = map.point(FontPoint::ZERO).unwrap();
    assert_eq!((p.x().ticks(), p.y().ticks()), (30 * 64, -20 * 64));
}
#[test]
fn move_quantizes_after_scaling_with_native_truncation() {
    let point = |x, y| ScriptPoint::new(FiniteF64::new(x).unwrap(), FiniteF64::new(y).unwrap());
    let sample = MoveSample::new(
        point(0.12, -0.12),
        point(0.68, -0.68),
        FiniteF64::new(0.5).unwrap(),
    )
    .unwrap();
    let p = sample.position(2.0, 3.0).unwrap();
    assert_eq!((p.x().get(), p.y().get()), (0.3125, -0.375));
    assert_eq!(sample.position(1.0, 1.0).unwrap().x().get(), 0.25);
    for progress in [0.001, 0.13, 0.71, 0.999] {
        let same = MoveSample::new(
            point(123.125, -987.625),
            point(123.125, -987.625),
            FiniteF64::new(progress).unwrap(),
        )
        .unwrap();
        assert_eq!(same.position(1.0, 1.0).unwrap(), point(123.125, -987.625));
    }
    assert_eq!(
        MoveSample::new(point(1e300, 0.0), point(0.0, 0.0), FiniteF64::ZERO)
            .unwrap()
            .position(1.0, 1.0)
            .unwrap_err()
            .code(),
        ErrorCode::LimitExceeded
    );
}
#[test]
fn fad_fade_boundaries_transparency_and_first_effect_wins() {
    for (ms, a) in [
        (0, 0),
        (250, 128),
        (500, 255),
        (1000, 255),
        (1500, 255),
        (1750, 128),
        (1999, 1),
    ] {
        assert_eq!(alpha(r"\fad(500,500)", ms), a, "{ms}");
    }
    assert_eq!(alpha(r"\fad(4000,4000)", 1000), 64);
    for (ms, a) in [
        (0, 0),
        (99, 0),
        (100, 0),
        (200, 100),
        (300, 200),
        (500, 200),
        (700, 200),
        (800, 150),
        (900, 100),
        (1500, 100),
    ] {
        assert_eq!(alpha(r"\fade(255,55,155,100,300,700,900)", ms), a, "{ms}");
    }
    assert_eq!(alpha(r"\fade(255,55,155,100,100,900,900)", 100), 200);
    assert_eq!(alpha(r"\fade(255,55,155,100,100,900,900)", 900), 100);
    assert_eq!(alpha(r"\fad(0,0)\fade(255,255,255,0,0,0,0)", 500), 255);
    assert_eq!(alpha(r"\fade(100,100,100,0,0,0,0)\fad(500,500)", 500), 155);
    assert_eq!(alpha(r"\fad(1000,1000)\rOther\fad(0,0)", 500), 128);
    assert_eq!(alpha(r"\t(1000,1500,\fad(1000,0))", 500), 128);
    assert_eq!(alpha(r"\alpha&H80&\fad(1000,0)", 500), 64);
    let e = at_body(r"A{\fad(1000,0)}g", 500_000);
    assert!(e.runs().iter().all(|r| r.paint().primary().a() == 128));
    let a = at(r"\fad(1000,0)", 0);
    let b = at(r"\fad(1000,0)", 500);
    assert_eq!(a.runs()[0].shaping(), b.runs()[0].shaping());
    assert_eq!(a.runs()[0].geometry(), b.runs()[0].geometry());
    assert!(a.animation().paint());
    assert!(!a.animation().shape());
    let p = parsed(r"{\fad(500,500)}Ag");
    assert_eq!(
        taiyou_render::evaluate(
            p.track(),
            p.track().events()[0].id(),
            TimeUs::from_micros(3_000_000),
            Profile::CoreV1
        )
        .unwrap_err()
        .code(),
        ErrorCode::InvalidArgument
    );
}

#[cfg(feature = "native")]
mod pixels {
    use super::*;
    fn fonts() -> std::sync::Arc<ValidatedFontCatalog> {
        let mut context = FontContext::new().unwrap();
        let mut b = FontCatalogBuilder::new(Limits::default());
        b.register(
            &mut context,
            &std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap()).unwrap(),
            0,
        )
        .unwrap();
        b.snapshot(&mut CatalogIngestor::new()).unwrap()
    }
    fn request(ms: i64) -> RenderRequest {
        RenderRequest {
            now: TimeUs::from_micros(1_000_000 + ms * 1000),
            viewport: Viewport::full(Size::new(160, 100)).unwrap(),
            unsupported: UnsupportedPolicy::Strict,
        }
    }
    fn pixels(body: &str, ms: i64) -> Vec<u8> {
        Renderer::new(Default::default())
            .unwrap()
            .render_parsed(&parsed(body), fonts(), &request(ms))
            .unwrap()
            .pixels()
            .to_vec()
    }
    #[test]
    fn movement_origin_rotation_and_fades_render_through_normal_pipeline() {
        for word in ["Ag", r"{\p1}m 0 0 l 10 0 10 10 0 10"] {
            assert_eq!(
                pixels(&format!(r"{{\move(20,40,60,40)}}{word}"), 500),
                pixels(&format!(r"{{\pos(30,40)}}{word}"), 500)
            );
            let base = pixels(&format!(r"{{\pos(60,40)\fr90}}{word}"), 500);
            assert_eq!(
                base,
                pixels(&format!(r"{{\pos(60,40)\org(60,40)\frz450}}{word}"), 500)
            );
            assert_ne!(
                base,
                pixels(&format!(r"{{\pos(60,40)\org(40,40)\frz90}}{word}"), 500)
            );
            assert_eq!(
                pixels(
                    &format!(r"{{\pos(60,40)\frz0\t(0,1000,\frz90)}}{word}"),
                    500
                ),
                pixels(&format!(r"{{\pos(60,40)\frz45}}{word}"), 500)
            );
            assert_eq!(
                pixels(
                    &format!(r"{{\pos(60,40)\bord1\shad2\4a&H00&\fad(1000,0)}}{word}"),
                    500
                ),
                pixels(
                    &format!(r"{{\pos(60,40)\bord1\shad2\4a&H00&\alpha&H7F&}}{word}"),
                    500
                )
            );
        }
        assert!(pixels(r"{\fad(1000,0)}Ag", 0).iter().all(|b| *b == 0));
        assert!(pixels(r"{\fad(1000,0)}Ag", 2000).iter().all(|b| *b == 0));
    }
    #[test]
    fn fresh_forward_backward_random_seek_is_byte_and_diagnostic_identical() {
        let p = parsed(
            r"{\move(20,40,60,40)\org(40,40)\frz0\t(0,1000,\frz90)\fad(500,500)}Ag{\p1}m 0 0 l 10 0 10 10 0 10",
        );
        let catalog = fonts();
        let mut renderer = Renderer::new(Default::default()).unwrap();
        for ms in [
            0, 100, 500, 1000, 1750, 1999, 500, 0, 1800, 200, 900, 1500, 2000, 800,
        ] {
            let fresh = Renderer::new(Default::default())
                .unwrap()
                .render_parsed(&p, catalog.clone(), &request(ms))
                .unwrap();
            let actual = renderer
                .render_parsed(&p, catalog.clone(), &request(ms))
                .unwrap();
            assert_eq!(actual.pixels(), fresh.pixels(), "{ms}");
            assert_eq!(actual.diagnostics(), fresh.diagnostics());
        }
    }
    #[test]
    fn explicit_origin_and_motion_never_become_collision_obstacles() {
        for tag in [r"\org(0,0)", r"\move(20,40,60,40)"] {
            let p = parsed(&format!(r"{{\alpha&HFF&{tag}}}Ag"));
            let original = &p.track().events()[0];
            let automatic = Event::new(
                EventId::new(1),
                original.start(),
                original.end(),
                StyleId::new(0),
                original.source(),
            )
            .with_program(std::sync::Arc::from([original
                .program()
                .last()
                .unwrap()
                .clone()]));
            let make = |events: Vec<Event>| {
                TrackIngestor::new()
                    .ingest(
                        std::sync::Arc::from(p.track().source()),
                        p.track().script().clone(),
                        p.track().styles().to_vec().into(),
                        events.into(),
                        &Limits::default(),
                    )
                    .unwrap()
            };
            let catalog = fonts();
            let a = Renderer::new(Default::default())
                .unwrap()
                .render(
                    make(vec![automatic.clone()]),
                    catalog.clone(),
                    &request(500),
                )
                .unwrap();
            let b = Renderer::new(Default::default())
                .unwrap()
                .render(
                    make(vec![original.clone(), automatic]),
                    catalog,
                    &request(500),
                )
                .unwrap();
            assert_eq!(a.pixels(), b.pixels());
            assert_eq!(a.diagnostics(), b.diagnostics());
        }
    }
}
