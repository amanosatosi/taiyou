use taiyou_render::renderer::*;

fn parsed(tags: &str) -> ParsedTrack {
    parsed_body(&format!("{{{tags}}}A"))
}
fn parsed_body(body: &str) -> ParsedTrack {
    parsed_body_effect(body, "")
}
fn parsed_body_effect(body: &str, effect: &str) -> ParsedTrack {
    parsed_body_effect_wrap(body, effect, 2)
}
fn parsed_body_effect_wrap(body: &str, effect: &str, wrap: u8) -> ParsedTrack {
    let ass = format!(
        "[Script Info]\nPlayResX: 120\nPlayResY: 120\nWrapStyle: {wrap}\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow\nStyle: Default,Source Sans 3,30,0,0\n[Events]\nFormat: Start,End,Effect,Text\nDialogue: 0:00:00.00,0:00:05.00,{effect},{body}\n"
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
#[test]
fn unknown_event_effects_are_preserved_and_known_motion_is_not_ignored() {
    for effect in [
        "fx",
        "custom;12",
        "BannerX;25",
        "Scroll sideways;0;90;25",
        "Banner",
        "Scroll up",
    ] {
        let p = parsed_body_effect("A", effect);
        assert_eq!(p.track().events()[0].effect(), effect);
        taiyou_render::evaluate(
            p.track(),
            EventId::new(0),
            TimeUs::from_micros(0),
            Profile::CoreV1,
        )
        .unwrap();
    }
    for effect in [
        "Banner;25",
        "Scroll up;0;90;25",
        "Scroll down;0;90;25",
        "bAnNeR;25",
        "scroll UP;0;90;25",
    ] {
        let p = parsed_body_effect_wrap(r"A\nB", effect, 0);
        assert_eq!(p.track().events()[0].effect(), effect);
        let evaluated = taiyou_render::evaluate(
            p.track(),
            EventId::new(0),
            TimeUs::from_micros(0),
            Profile::CoreV1,
        )
        .unwrap();
        assert!(!evaluated.placement().collision_eligible(), "{effect}");
        assert!(!evaluated.animation().is_static(), "{effect}");
        if effect.to_ascii_lowercase().starts_with("banner;") {
            assert_eq!(evaluated.wrap_mode().get(), 2);
            assert_eq!(evaluated.source_map().unwrap().text(), "A\nB");
        } else {
            assert_eq!(evaluated.wrap_mode().get(), 0);
            assert_eq!(evaluated.source_map().unwrap().text(), "A B");
        }
    }
    for effect in [
        "Banner;25;0;10",
        "Scroll up;0;90;25;10",
        "Scroll down;0;90;25;10",
    ] {
        let p = parsed_body_effect("A", effect);
        let error = taiyou_render::evaluate(
            p.track(),
            EventId::new(0),
            TimeUs::from_micros(0),
            Profile::CoreV1,
        )
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::UnsupportedFeature, "{effect}");
    }
}
fn at(tags: &str, ms: i64) -> EvaluatedEvent {
    let p = parsed(tags);
    taiyou_render::evaluate(
        p.track(),
        EventId::new(0),
        TimeUs::from_micros(ms * 1000),
        Profile::CoreV1,
    )
    .unwrap()
}
#[test]
fn bare_extended_alpha_narrows_after_animation() {
    for (ms, opacity) in [(2040, 47), (2560, 250), (4960, 5)] {
        assert_eq!(
            at(r"\1a0\t(\1a1FF)", ms).runs()[0].paint().primary().a(),
            opacity
        );
    }
    assert_eq!(at(r"\1a&H100&", 0).runs()[0].paint().primary().a(), 255);
    assert_eq!(at(r"\1a-1", 0).runs()[0].paint().primary().a(), 0);
    let p = parsed(r"\1a0\t(\1a1FF)");
    assert!(p.track().events()[0].program().iter().any(|op| matches!(op,TextOp::Override(OverrideOp::SetAlpha{value:Some(511),source,..}) if source.slice(p.track().source())==Some(r"\1a1FF"))));
}

#[test]
fn projected_and_axis_properties_are_ordered_and_animated() {
    let e = at(
        r"\frx10\fry20\fax0.2\fay-0.2\xbord2\ybord4\xshad-3\yshad5\t(0,1000,\frx30\fry40\fax0.6\fay0.2\xbord6\ybord8\xshad1\yshad9)",
        500,
    );
    let g = e.runs()[0].geometry();
    let f = e.runs()[0].effects();
    assert_eq!((g.rotation_x().get(), g.rotation_y().get()), (20.0, 30.0));
    assert!((g.shear_x().get() - 0.4).abs() < 1e-12);
    assert_eq!(g.shear_y().get(), 0.0);
    assert_eq!(
        (
            f.border_x().get(),
            f.border_y().get(),
            f.shadow_x().get(),
            f.shadow_y().get()
        ),
        (4.0, 6.0, -1.0, 7.0)
    );
    let e = at(r"\xbord2\ybord4\bord3\xshad-2\yshad6\shad4", 0);
    let f = e.runs()[0].effects();
    assert_eq!(
        (
            f.border_x().get(),
            f.border_y().get(),
            f.shadow_x().get(),
            f.shadow_y().get()
        ),
        (3.0, 3.0, 4.0, 4.0)
    );
    let e = at(r"\frx20\fry30\fax1\fay1\r", 0);
    assert_eq!(e.runs()[0].geometry().rotation_x().get(), 0.0);
    assert_eq!(e.runs()[0].geometry().shear_y().get(), 0.0);
}
#[test]
fn clipping_slots_and_legacy_style_switches() {
    let e = at(
        r"\clip(1.4,-2.7,90,100)\t(0,1000,\iclip(3,4,80,90))\clip(m 0 0 l 10 0 10 10 0 10)\iclip(m 0 0 l 1 0 1 1)\u1\s1\a6\an1",
        500,
    );
    assert_eq!(e.ass_clip().rectangles.len(), 2);
    assert_eq!(e.ass_clip().rectangles[0].coords, [1, -2, 90, 100]);
    assert_eq!(e.ass_clip().rectangles[1].progress.get(), 0.5);
    assert!(!e.ass_clip().vector.as_ref().unwrap().1);
    assert!(e.runs()[0].geometry().underline() && e.runs()[0].geometry().strikeout());
    assert_eq!(e.placement().alignment().get(), 8);
    assert!(
        !at(r"\u1\s1\t(1000,2000,\u0\s0)", 0).runs()[0]
            .geometry()
            .underline()
    );
}
#[test]
fn karaoke_source_clock_and_nonanimated_transform_commands() {
    let e = at(r"\kt25.5\kf75.25", 500);
    let k = e.runs()[0].karaoke().unwrap();
    assert_eq!(
        (k.kind, k.start_ms, k.end_ms, k.now_ms),
        (KaraokeTag::Sweep, 255, 1007, 500)
    );
    let e = at(r"\t(1000,2000,\kt30\ko20)", 0);
    let k = e.runs()[0].karaoke().unwrap();
    assert_eq!(
        (k.kind, k.start_ms, k.end_ms),
        (KaraokeTag::Outline, 300, 500)
    );
    let p = parsed(r"\k214748365");
    assert_eq!(
        taiyou_render::evaluate(
            p.track(),
            EventId::new(0),
            TimeUs::from_micros(0),
            Profile::CoreV1
        )
        .unwrap_err()
        .code(),
        ErrorCode::LimitExceeded
    );
}
#[test]
fn karaoke_words_escapes_and_provenance_survive_partitioning() {
    let p = parsed_body(r"{\kt50\kf100}AB\hCD EF{\k20}G");
    let e = taiyou_render::evaluate(
        p.track(),
        EventId::new(0),
        TimeUs::from_micros(1000000),
        Profile::CoreV1,
    )
    .unwrap();
    let a = e.runs()[0].karaoke().unwrap();
    for r in &e.runs()[..3] {
        let k = r.karaoke().unwrap();
        assert_eq!((k.start_ms, k.end_ms, k.word), (500, 1500, a.word));
    }
    let space = e.runs()[3].karaoke().unwrap();
    assert_eq!((space.start_ms, space.end_ms), (1500, 1500));
    let last = e.runs().last().unwrap().karaoke().unwrap();
    assert_eq!(
        (last.kind, last.start_ms, last.end_ms),
        (KaraokeTag::Instant, 1500, 1700)
    );
    for r in e.runs() {
        let d = r.decoded().unwrap();
        assert_eq!(d.text(), r.text());
        assert!(d.belongs_to(e.source_map().unwrap()));
    }
    assert_eq!(at(r"\shad-3", 0).runs()[0].effects().shadow_x().get(), 0.0);
    assert_eq!(
        at(r"\xshad-3", 0).runs()[0].effects().shadow_x().get(),
        -3.0
    );
    let e = at(r"\xbord2\ybord8\t(0,1000,\bord4)", 500);
    assert_eq!(
        (
            e.runs()[0].effects().border_x().get(),
            e.runs()[0].effects().border_y().get()
        ),
        (3.0, 6.0)
    );
    let p = parsed_body(r"{\kf100}A{}B");
    let e = taiyou_render::evaluate(
        p.track(),
        EventId::new(0),
        TimeUs::from_micros(500000),
        Profile::CoreV1,
    )
    .unwrap();
    assert_eq!(
        (
            e.runs()[0].karaoke().unwrap().start_ms,
            e.runs()[1].karaoke().unwrap().start_ms
        ),
        (0, 1000)
    );
}

#[cfg(feature = "native")]
mod pixels {
    use super::*;
    fn fonts() -> std::sync::Arc<ValidatedFontCatalog> {
        let mut ctx = FontContext::new().unwrap();
        let mut b = FontCatalogBuilder::new(Limits::default());
        b.register(
            &mut ctx,
            &std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap()).unwrap(),
            0,
        )
        .unwrap();
        b.snapshot(&mut CatalogIngestor::new()).unwrap()
    }
    fn render(tags: &str, ms: i64) -> Vec<u8> {
        render_body(&format!("{{{tags}}}A"), ms)
    }
    fn render_body(body: &str, ms: i64) -> Vec<u8> {
        render_body_effect(body, ms, "")
    }
    fn render_body_effect(body: &str, ms: i64, effect: &str) -> Vec<u8> {
        render_parsed_body(parsed_body_effect(body, effect), ms)
    }
    fn render_parsed_body(parsed: ParsedTrack, ms: i64) -> Vec<u8> {
        Renderer::new(Default::default())
            .unwrap()
            .render_parsed(
                &parsed,
                fonts(),
                &RenderRequest {
                    now: TimeUs::from_micros(ms * 1000),
                    viewport: Viewport::full(Size::new(120, 120)).unwrap(),
                    unsupported: UnsupportedPolicy::Strict,
                },
            )
            .unwrap()
            .pixels()
            .to_vec()
    }
    #[test]
    fn unknown_event_effects_leave_ordinary_and_animated_pixels_unchanged() {
        for body in ["A", r"{\fad(300,200)\k42}A{\kf245}B"] {
            for ms in [0, 1500, 100, 4800, 500] {
                let ordinary = render_body(body, ms);
                if ms > 0 {
                    assert!(ordinary.chunks_exact(4).any(|p| p[3] != 0));
                }
                for effect in [
                    "fx",
                    "custom;12",
                    "BannerX;25",
                    "Scroll sideways;0;90;25",
                    "Banner",
                ] {
                    assert_eq!(
                        render_body_effect(body, ms, effect),
                        ordinary,
                        "{effect} at {ms}"
                    );
                }
            }
        }
    }
    #[test]
    fn banner_and_scroll_match_positioned_clipped_drawings() {
        let drawing = "m 0 0 l 12 0 12 8 0 8";
        let body = format!(r"{{\an7\pos(33,40)\p1}}{drawing}");
        for (effect, position, clip) in [
            ("Banner;25", "80,40", "0,0,120,120"),
            ("bAnNeR;25;1", "28,40", "0,0,120,120"),
            ("Scroll up;20;90;25", "33,50", "0,20,120,90"),
            ("Scroll down;90;20;25", "33,52", "0,20,120,90"),
        ] {
            let expected = render_body(
                &format!(r"{{\an7\pos({position})\clip({clip})\p1}}{drawing}"),
                1000,
            );
            assert!(expected.chunks_exact(4).any(|p| p[3] != 0));
            assert_eq!(
                render_body_effect(&body, 1000, effect),
                expected,
                "{effect}"
            );
        }
        // At 125 ms the scroll overlaps the band edge: clipping is exercised.
        let expected = render_body(
            &format!(r"{{\an7\pos(33,17)\clip(0,20,120,90)\p1}}{drawing}"),
            125,
        );
        assert!(expected.chunks_exact(4).any(|p| p[3] != 0));
        assert_eq!(
            render_body_effect(&body, 125, "Scroll down;20;90;25"),
            expected
        );
        // Animated scroll coordinates do not depend on previous render calls.
        assert_eq!(
            render_body_effect(&body, 1000, "Banner;25"),
            render_body(&format!(r"{{\an7\pos(80,40)\p1}}{drawing}"), 1000)
        );
    }
    #[test]
    fn transition_delay_uses_layout_scale_and_absolute_event_time() {
        let drawing = "m 0 0 l 12 0 12 8 0 8";
        let body = format!(r"{{\an7\pos(33,40)\p1}}{drawing}");
        for (effect, now, x) in [
            ("Banner;0", 1020, "110"),
            ("Banner;-25", 1020, "110"),
            ("Banner;25", 2000, "78.375"),
        ] {
            let source = parsed_body_effect(&body, effect)
                .track()
                .source()
                .replace(
                    "WrapStyle: 2",
                    "WrapStyle: 2\nLayoutResX: 240\nLayoutResY: 240",
                )
                .replace("0:00:00.00,0:00:05.00", "0:00:01.00,0:00:05.00");
            let parsed = taiyou_ass::parse_ass(
                source.as_bytes(),
                &ParseOptions::default(),
                &mut TrackIngestor::new(),
            )
            .unwrap();
            assert_eq!(
                render_parsed_body(parsed, now),
                render_body(&format!(r"{{\an7\pos({x},40)\p1}}{drawing}"), now),
                "{effect}"
            );
        }
    }
    #[test]
    fn ssa_style_alpha_and_alignment_render_like_explicit_ass_normalization() {
        for (alpha, clamped) in [
            ("&HFF", 255),
            ("&H30", 48),
            ("0", 0),
            ("&HFFFFFFEA", 0),
            ("&H3FF", 255),
        ] {
            let header = "[Script Info]\nPlayResX: 120\nPlayResY: 120\nWrapStyle: 2\n";
            let events =
                "[Events]\nFormat: Start,End,Style,Text\nDialogue: 0:00:00.00,0:00:02.00,Default,W";
            let ssa = format!(
                "{header}[V4 Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,SecondaryColour,TertiaryColour,BackColour,Outline,Shadow,Alignment,AlphaLevel\nStyle: Default,Source Sans 3,32,&HFF123456,&HFF654321,&HABCDEF,&HFE13206A,3,4,9,{alpha}\n{events}"
            );
            let ass = format!(
                "{header}[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,SecondaryColour,OutlineColour,BackColour,Outline,Shadow,Alignment\nStyle: Default,Source Sans 3,32,&H{clamped:02X}123456,&H{clamped:02X}654321,&H{clamped:02X}13206A,&H8013206A,3,4,4\n{events}"
            );
            let draw = |source: &str| {
                let parsed = taiyou_ass::parse_ass(
                    source.as_bytes(),
                    &ParseOptions::default(),
                    &mut TrackIngestor::new(),
                )
                .unwrap();
                Renderer::new(Default::default())
                    .unwrap()
                    .render_parsed(
                        &parsed,
                        fonts(),
                        &RenderRequest {
                            now: TimeUs::from_micros(0),
                            viewport: Viewport::full(Size::new(120, 120)).unwrap(),
                            unsupported: UnsupportedPolicy::Strict,
                        },
                    )
                    .unwrap()
                    .pixels()
                    .to_vec()
            };
            let actual = draw(&ssa);
            assert!(
                actual.chunks_exact(4).any(|p| p[3] != 0),
                "shadow remains visible for {alpha}"
            );
            assert_eq!(actual, draw(&ass), "SSA normalization {alpha}");
        }
    }
    #[test]
    fn clip_and_inverse_partition_real_text_and_effects() {
        let full = render(r"\pos(60,60)\bord2\shad2", 0);
        let left = render(r"\pos(60,60)\bord2\shad2\clip(0,0,60,120)", 0);
        let right = render(r"\pos(60,60)\bord2\shad2\iclip(0,0,60,120)", 0);
        for (i, p) in full.chunks_exact(4).enumerate() {
            assert_eq!(
                p,
                if i % 120 < 60 {
                    &left[i * 4..i * 4 + 4]
                } else {
                    &right[i * 4..i * 4 + 4]
                }
            );
        }
        assert!(
            left.chunks_exact(4)
                .enumerate()
                .all(|(i, p)| i % 120 < 60 || p == [0, 0, 0, 0])
        );
        assert_eq!(
            render(r"\pos(60,60)\clip(0,0,60,120)", 0),
            render(r"\pos(60,60)\clip(m 0 0 l 60 0 60 120 0 120)", 0)
        );
        for (a, b, ms) in [
            (
                r"\iclip(0,0,60,120)",
                r"\iclip(m 0 0 l 60 0 60 120 0 120)",
                0,
            ),
            (
                r"\clip(0,0,60,120)",
                r"\clip(2,m 0 0 l 120 0 120 240 0 240)",
                0,
            ),
            (
                r"\clip(0,0,60,120)",
                r"\clip(0,0,40,120)\t(0,1000,\clip(0,0,80,120))",
                500,
            ),
            (
                r"\clip(m 0 0 l 60 0 60 120 0 120)",
                r"\t(1000,2000,\clip(m 0 0 l 60 0 60 120 0 120))",
                0,
            ),
            (r"\clip(0,0,60,120)", r"\clip(0,0,60,120)\r", 0),
            ("", r"\clip()", 0),
        ] {
            assert_eq!(
                render(&format!(r"\pos(60,60){a}"), ms),
                render(&format!(r"\pos(60,60){b}"), ms),
                "{b}"
            );
        }
    }
    #[test]
    fn projection_decorations_and_axis_effects_reach_pixels() {
        let plain = render(r"\pos(60,60)", 0);
        for tags in [
            r"\frx45",
            r"\fry45",
            r"\fax0.5",
            r"\fay0.5",
            r"\u1",
            r"\s1",
            r"\xbord2\ybord5",
            r"\xbord2\ybord0",
            r"\xbord0\ybord3",
            r"\xshad-3\yshad5",
        ] {
            assert_ne!(plain, render(&format!(r"\pos(60,60){tags}"), 0), "{tags}");
        }
        assert_eq!(
            render(r"\pos(60,60)\frx450\fry-270", 0),
            render(r"\pos(60,60)\frx90\fry90", 0)
        );
    }
    #[test]
    fn decorations_add_coverage_for_cff_and_truetype_contours() {
        let latin = std::path::PathBuf::from(std::env::var("RUNNER_TEMP").unwrap())
            .join("taiyou-shaping-fonts/latin.otf");
        for path in [latin.clone(), latin.with_file_name("indic.ttf")] {
            let mut context = FontContext::new().unwrap();
            let mut builder = FontCatalogBuilder::new(Limits::default());
            builder
                .register(&mut context, &std::fs::read(path).unwrap(), 0)
                .unwrap();
            let catalog = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
            let family = catalog.faces()[0].family();
            let draw = |tags: &str| {
                let track = parsed_body(&format!(r"{{\fn{family}\pos(60,60){tags}}}Hgyp"));
                Renderer::new(Default::default())
                    .unwrap()
                    .render_parsed(
                        &track,
                        catalog.clone(),
                        &RenderRequest {
                            now: TimeUs::from_micros(0),
                            viewport: Viewport::full(Size::new(120, 120)).unwrap(),
                            unsupported: UnsupportedPolicy::Strict,
                        },
                    )
                    .unwrap()
                    .pixels()
                    .to_vec()
            };
            let plain = draw("");
            assert!(plain.chunks_exact(4).any(|p| p[3] > 0));
            for tag in [r"\u1", r"\s1", r"\u1\s1"] {
                let decorated = draw(tag);
                assert_ne!(plain, decorated, "{family} {tag}");
                // Filled decorations must not cut holes into overlapping ink.
                assert!(
                    plain
                        .chunks_exact(4)
                        .zip(decorated.chunks_exact(4))
                        .all(|(a, b)| b[3] >= a[3]),
                    "{family} {tag}"
                );
                assert_eq!(decorated, draw(tag));
            }
        }
    }
    #[test]
    fn drawing_geometry_and_transform_tags_share_the_pipeline() {
        let draw = |tags: &str| {
            render_body(
                &format!(r"{{\pos(60,60)\p1{tags}}}m 0 0 l 20 0 20 10 0 10"),
                500,
            )
        };
        let base = draw("");
        assert!(base.chunks_exact(4).any(|p| p[3] > 0));
        for tag in [
            r"\frx40",
            r"\fry40",
            r"\fax0.5",
            r"\fay0.5",
            r"\xbord2\ybord5",
            r"\clip(0,0,60,120)",
        ] {
            assert_ne!(base, draw(tag), "{tag}");
        }
        assert_ne!(draw(r"\frx20\fax0.5"), draw(r"\org(0,0)\frx20\fax0.5"));
        assert_eq!(draw(r"\fax0.5"), draw(r"\org(0,0)\fax0.5"));
    }
    #[test]
    fn polygon_words_use_bounds_advances_without_text_spacing() {
        // pinterf CPolygon::ParseStr derives width from polygon bounds;
        // CText::CreatePath's per-character fontSpacing is not a polygon advance.
        // Separate p1/p0 words reduce drawImg's 784-square sprite to four cells.
        let squares = r"{\p1}m 0 0 l 0 2 2 2 2 0{\p0}".repeat(4);
        for position in ["20", "20.375"] {
            let draw = |spacing| {
                render_body(
                    &format!(r"{{\an7\pos({position},20)\fsp{spacing}}}{squares}"),
                    0,
                )
            };
            let expected = draw("0");
            assert!(expected.chunks_exact(4).any(|pixel| pixel[3] > 0));
            assert_eq!(expected, draw("-0.6"));
            assert_eq!(expected, draw("1.25"));
        }
    }
    #[test]
    fn karaoke_pixels_and_seeking() {
        let p = r"\pos(60,60)\1c&H0000FF&\2c&HFF0000&\kt50\kf100";
        assert_ne!(render(p, 0), render(p, 1000));
        assert_ne!(render(p, 1000), render(p, 2000));
        assert_eq!(render(&p.replace("kf100", "K100"), 1000), render(p, 1000));
        let instant = r"\pos(60,60)\kt50\k100\bord3";
        let outline = r"\pos(60,60)\kt50\ko100\bord3";
        assert_ne!(render(instant, 0), render(outline, 0));
        assert_eq!(render(instant, 500), render(outline, 500));
        let parsed = parsed(p);
        let fonts = fonts();
        let mut r = Renderer::new(Default::default()).unwrap();
        for ms in [1500, 500, 1000, 0, 2000, 999, 1000] {
            let got = r
                .render_parsed(
                    &parsed,
                    fonts.clone(),
                    &RenderRequest {
                        now: TimeUs::from_micros(ms * 1000),
                        viewport: Viewport::full(Size::new(120, 120)).unwrap(),
                        unsupported: UnsupportedPolicy::Strict,
                    },
                )
                .unwrap();
            assert_eq!(got.pixels(), render(p, ms));
        }
    }
}
