#![cfg(feature = "native")]

use std::sync::Arc;
use taiyou_render::{collision_footprint::collision_footprint, geometry::DeviceMapping};
use taiyou_render::{layout, renderer::*};

fn fonts() -> Arc<ValidatedFontCatalog> {
    let mut context = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    builder
        .register(
            &mut context,
            &std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap()).unwrap(),
            0,
        )
        .unwrap();
    builder.snapshot(&mut CatalogIngestor::new()).unwrap()
}

fn parsed(events: &str) -> ParsedTrack {
    parsed_header(events, "")
}

fn parsed_header(events: &str, header: &str) -> ParsedTrack {
    let source = format!(
        "[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\n{header}\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow,Alignment,MarginL,MarginR,MarginV,PrimaryColour,SecondaryColour\nStyle: Default,Source Sans 3,30,0,0,5,0,0,0,&H00FFFFFF,&H000000FF\n[Events]\nFormat: Start,End,Text\n{events}"
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
fn ass_kerning_defaults_off_and_header_selection_survives_resets_and_karaoke() {
    let fonts = fonts();
    let request = RenderRequest {
        now: TimeUs::from_micros(500_000),
        viewport: Viewport::full(Size::new(160, 100)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    };
    for body in ["AV To", r"{\kf100}AV To", r"{\r}AV To"] {
        let events = format!("Dialogue: 0:00:00.00,0:00:05.00,{body}\n");
        let render = |header| {
            Renderer::new(Default::default())
                .unwrap()
                .render_parsed(&parsed_header(&events, header), fonts.clone(), &request)
                .unwrap()
                .pixels()
                .to_vec()
        };
        let default = render("");
        let enabled = render("Kerning: yes");
        assert_ne!(default, enabled, "{body}");
        for header in [
            "Kerning: no",
            "Kerning: 0",
            "Kerning: -1",
            "Kerning: invalid",
        ] {
            assert_eq!(default, render(header), "{header}, {body}");
        }
        for header in [
            "Kerning: 1",
            "Kerning: +002tail",
            "Kerning: YES",
            "Kerning: no\nKerning: yes",
        ] {
            assert_eq!(enabled, render(header), "{header}, {body}");
        }
        assert_eq!(default, render("Kerning: yes\nKerning: no"));
    }
}

#[test]
fn adjacent_drawing_words_keep_independent_coverage_and_alpha() {
    // pinterf CPolygon::Append never joins polygons. libass split_style_runs
    // also starts a bitmap run on both sides of a drawing. Two separately
    // positioned events therefore provide a complete-buffer control for fill.
    let fonts = fonts();
    let square = "m 0 0 l 20 0 20 20 0 20";
    for (x, next, seam_alpha) in [("20.125", "40.125", 227), ("20.5", "40.5", 192)] {
        for alpha in ["00", "80"] {
            let paint = format!(r"\an7\1a&H{alpha}&");
            let joined = parsed(&format!(
                "Dialogue: 0:00:00.00,0:00:05.00,{{{paint}\\pos({x},20)\\p1}}{square}{{\\p0}}{{\\p1}}{square}\n"
            ));
            let separate = parsed(&format!(
                "Dialogue: 0:00:00.00,0:00:05.00,{{{paint}\\pos({x},20)\\p1}}{square}\nDialogue: 0:00:00.00,0:00:05.00,{{{paint}\\pos({next},20)\\p1}}{square}\n"
            ));
            let render = |track: &ParsedTrack| {
                Renderer::new(Default::default())
                    .unwrap()
                    .render_parsed(
                        track,
                        fonts.clone(),
                        &RenderRequest {
                            now: TimeUs::from_micros(500_000),
                            viewport: Viewport::full(Size::new(160, 100)).unwrap(),
                            unsupported: UnsupportedPolicy::Strict,
                        },
                    )
                    .unwrap()
            };
            let actual = render(&joined);
            assert_eq!(
                actual.pixels(),
                render(&separate).pixels(),
                "x={x}, alpha={alpha}"
            );
            if alpha == "00" {
                assert_eq!(actual.pixels()[(25 * 160 + 40) * 4 + 3], seam_alpha);
            }
            // Contours inside a single drawing still share nonzero winding.
            let single = parsed(&format!(
                "Dialogue: 0:00:00.00,0:00:05.00,{{{paint}\\pos({x},20)\\p1}}{square} m 20 0 l 40 0 40 20 20 20\n"
            ));
            assert_ne!(actual.pixels(), render(&single).pixels());
        }
    }
}

#[test]
fn opaque_karaoke_colors_keep_outline_under_body_antialiasing() {
    let fonts = fonts();
    let square = "m 0 0 l 20 0 20 20 0 20";
    for (primary, secondary) in [("00", "00"), ("80", "00"), ("00", "80")] {
        let actual = parsed(&format!(
            "Dialogue: 0:00:00.00,0:00:05.00,{{\\an7\\pos(20.5,20)\\xbord2\\ybord0\\3c&H0000FF&\\1a&H{primary}&\\2a&H{secondary}&\\p1}}{square}\n"
        ));
        // A horizontal-only outline has exact rectangular geometry. When both
        // karaoke colors are opaque its entire expanded rectangle is painted
        // behind the body; otherwise only the two side strips are painted.
        let red = r"\an7\bord0\1c&H0000FF&\1a&H00&\p1";
        let outline = if primary == "00" && secondary == "00" {
            format!(
                "Dialogue: 0:00:00.00,0:00:05.00,{{{red}\\pos(18.5,20)}}m 0 0 l 24 0 24 20 0 20\n"
            )
        } else {
            format!(
                "Dialogue: 0:00:00.00,0:00:05.00,{{{red}\\pos(18.5,20)}}m 0 0 l 2 0 2 20 0 20\nDialogue: 0:00:00.00,0:00:05.00,{{{red}\\pos(40.5,20)}}m 0 0 l 2 0 2 20 0 20\n"
            )
        };
        let expected = parsed(&format!(
            "{outline}Dialogue: 0:00:00.00,0:00:05.00,{{\\an7\\pos(20.5,20)\\bord0\\1a&H{primary}&\\p1}}{square}\n"
        ));
        let render = |track: &ParsedTrack| {
            Renderer::new(Default::default())
                .unwrap()
                .render_parsed(
                    track,
                    fonts.clone(),
                    &RenderRequest {
                        now: TimeUs::from_micros(500_000),
                        viewport: Viewport::full(Size::new(160, 100)).unwrap(),
                        unsupported: UnsupportedPolicy::Strict,
                    },
                )
                .unwrap()
        };
        assert_eq!(render(&actual).pixels(), render(&expected).pixels());
    }
}

fn local(body: &str, fonts: &ValidatedFontCatalog) -> LocalLayout {
    let p = parsed(&format!("Dialogue: 0:00:00.00,0:00:05.00,{body}\n"));
    let e = taiyou_render::evaluate(
        p.track(),
        EventId::new(0),
        TimeUs::from_micros(2_000_000),
        Profile::CoreV1,
    )
    .unwrap();
    let shaped = taiyou_render::shaping::shape_event(
        &mut FontContext::new().unwrap(),
        fonts,
        &e,
        &Default::default(),
        &Limits::default(),
    )
    .unwrap();
    layout(&shaped, &LayoutConstraints::new(e.placement().alignment())).unwrap()
}

#[test]
fn isolated_combining_marks_keep_literal_glyphs_and_zero_collision_width() {
    let fonts = fonts();
    for mark in ['\u{0300}', '\u{0301}', '\u{0308}'] {
        let local = local(&mark.to_string(), &fonts);
        let glyphs: Vec<_> = local
            .shaped()
            .runs()
            .iter()
            .flat_map(|r| r.glyphs())
            .collect();
        assert_eq!(glyphs.len(), 1, "invented base for {mark:?}");
        assert_ne!(glyphs[0].glyph(), 0);
        assert_eq!(glyphs[0].cluster().text(), mark.to_string());
        assert_eq!(glyphs[0].advance_x().get(), 0.0);
        assert_eq!(local.logical_bounds().width(), 0.0);
        assert!(local.ink_bounds().unwrap().width() > 0.0);
        let footprint = collision_footprint(
            &local,
            DeviceMapping::new(Size::new(160, 100), PixelRect::new(0, 0, 160, 100).unwrap())
                .unwrap(),
            false,
            UnsupportedPolicy::Strict,
        )
        .unwrap();
        assert_eq!(footprint.width(), 0.0, "ink must not reserve advance width");
    }
    let bordered = local("{\\bord1}\u{0300}", &fonts);
    let footprint = collision_footprint(
        &bordered,
        DeviceMapping::new(Size::new(160, 100), PixelRect::new(0, 0, 160, 100).unwrap()).unwrap(),
        false,
        UnsupportedPolicy::Strict,
    )
    .unwrap();
    assert!(
        footprint.width() > 0.0,
        "horizontal outline still reserves space"
    );
}

#[test]
fn zero_advance_event_does_not_displace_neighbor_in_either_order_or_seek_direction() {
    let fonts = fonts();
    let request = RenderRequest {
        now: TimeUs::from_micros(2_000_000),
        viewport: Viewport::full(Size::new(160, 100)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    };
    for (a, b) in [("\u{0300}", "A"), ("A", "\u{0300}")] {
        let input = |position: &str| {
            parsed(&format!(
                "Dialogue: 0:00:00.00,0:00:05.00,{{{position}}}{a}\nDialogue: 0:00:01.00,0:00:05.00,{{{position}}}{b}\n"
            ))
        };
        let ordinary = input("");
        let positioned = input(r"\pos(80,50)");
        let mut renderer = Renderer::new(Default::default()).unwrap();
        for ms in [2000, 4000, 1500, 2500, 2000] {
            let request = RenderRequest {
                now: TimeUs::from_micros(ms * 1000),
                ..request
            };
            let actual = renderer
                .render_parsed(&ordinary, fonts.clone(), &request)
                .unwrap();
            let expected = Renderer::new(Default::default())
                .unwrap()
                .render_parsed(&positioned, fonts.clone(), &request)
                .unwrap();
            assert!(actual.pixels().chunks_exact(4).any(|p| p[3] != 0));
            assert_eq!(
                actual.pixels(),
                expected.pixels(),
                "order {a:?}/{b:?}, {ms}"
            );
            assert_eq!(actual.diagnostics(), expected.diagnostics());
        }
    }
}
