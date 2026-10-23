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
    let source = format!(
        "[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow,Alignment,MarginL,MarginR,MarginV\nStyle: Default,Source Sans 3,30,0,0,5,0,0,0\n[Events]\nFormat: Start,End,Text\n{events}"
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
            assert_eq!(actual.pixels(), expected.pixels(), "order {a:?}/{b:?}, {ms}");
            assert_eq!(actual.diagnostics(), expected.diagnostics());
        }
    }
}
