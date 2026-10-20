#![cfg(feature = "native")]
use std::sync::Arc;
use taiyou_render::renderer::*;

fn fonts() -> Arc<ValidatedFontCatalog> {
    let mut ctx = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    let bytes = std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap()).unwrap();
    builder.register(&mut ctx, &bytes, 0).unwrap();
    builder.snapshot(&mut CatalogIngestor::new()).unwrap()
}
fn source(shadow: &str, tags: &str, text: &str) -> Arc<Track> {
    let source = format!(
        "[Script Info]\nPlayResX: 200\nPlayResY: 120\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow,Alignment,MarginL,MarginR,MarginV\nStyle: Default,Source Sans 3,30,0,{shadow},7,10,10,10\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:02.00,{{{tags}}}{text}\n"
    );
    taiyou_ass::parse_ass(
        source.as_bytes(),
        &ParseOptions::default(),
        &mut TrackIngestor::new(),
    )
    .unwrap()
    .track()
    .clone()
}
fn render(track: Arc<Track>, fonts: Arc<ValidatedFontCatalog>) -> Frame {
    Renderer::new(RendererConfig::default())
        .unwrap()
        .render(
            track,
            fonts,
            &RenderRequest {
                now: TimeUs::from_micros(500_000),
                viewport: Viewport::full(Size::new(200, 120)).unwrap(),
                unsupported: UnsupportedPolicy::Strict,
            },
        )
        .unwrap()
}

#[test]
fn zero_style_and_override_depth_suppress_shadow_without_darkening_edges() {
    let fonts = fonts();
    for border in [0, 2] {
        let tags = format!("\\bord{border}\\1c&HFFFFFF&\\3c&HFFFFFF&");
        let zero = render(source("0", &tags, "Hg"), fonts.clone());
        let hidden = render(
            source("0", &format!("{tags}\\4a&HFF&"), "Hg"),
            fonts.clone(),
        );
        let override_zero = render(source("4", &format!("{tags}\\shad0"), "Hg"), fonts.clone());
        assert_eq!(zero.pixels(), hidden.pixels());
        assert_eq!(override_zero.pixels(), hidden.pixels());
        assert!(
            zero.pixels()
                .chunks_exact(4)
                .any(|p| p[3] > 0 && p[3] < 255)
        );
        let positive = render(source("4", &tags, "Hg"), fonts.clone());
        assert_ne!(positive.pixels(), zero.pixels());
        let invisible_body = format!("{tags}\\1a&HFF&\\3a&HFF&");
        let absent = render(source("0", &invisible_body, "Hg"), fonts.clone());
        assert!(absent.pixels().iter().all(|&p| p == 0));
        let visible = render(source("4", &invisible_body, "Hg"), fonts.clone());
        assert!(visible.pixels().iter().any(|&p| p != 0));
    }
}

fn local(track: &Track, fonts: &ValidatedFontCatalog) -> LocalLayout {
    let evaluated = taiyou_render::evaluate(
        track,
        EventId::new(0),
        TimeUs::from_micros(500_000),
        Profile::CoreV1,
    )
    .unwrap();
    let shaped = taiyou_render::shaping::shape_event(
        &mut FontContext::new().unwrap(),
        fonts,
        &evaluated,
        &Default::default(),
        &Limits::default(),
    )
    .unwrap();
    taiyou_render::layout(
        &shaped,
        &LayoutConstraints::new(Alignment::new(7).unwrap()).with_wrap(evaluated.wrap_mode()),
    )
    .unwrap()
}

#[test]
fn hard_empty_lines_use_half_metrics_and_no_synthetic_terminal_line() {
    let fonts = fonts();
    let normal = local(&source("0", "", "AB"), &fonts);
    let a = normal.lines()[0].ascent().get();
    let d = normal.lines()[0].descent().get();
    let ah = ((a * 8.0).trunc() * 0.5).floor() / 8.0;
    let dh = ((d * 8.0).trunc() * 0.5).floor() / 8.0;
    for (text, empty) in [(r"A\N\NB", 1), (r"A\N\N\NB", 2)] {
        let layout = local(&source("0", "", text), &fonts);
        assert_eq!(layout.lines().len(), empty + 2);
        assert_eq!(layout.lines()[0].baseline().y().get(), a);
        for i in 0..empty {
            let line = &layout.lines()[i + 1];
            assert_eq!(line.ascent().get(), ah);
            assert_eq!(line.descent().get(), dh);
            assert_eq!(line.baseline().y().get(), a + d + i as f64 * (ah + dh) + ah);
        }
        assert_eq!(
            layout.lines()[empty + 1].baseline().y().get(),
            a + d + empty as f64 * (ah + dh) + a
        );
    }
    for (text, count) in [(r"\N", 1), (r"\N\N", 2)] {
        let layout = local(&source("0", "", text), &fonts);
        assert_eq!(layout.lines().len(), count);
        for (i, line) in layout.lines().iter().enumerate() {
            assert_eq!(line.baseline().y().get(), i as f64 * (ah + dh) + ah);
            assert_eq!(line.ascent().get(), ah);
            assert_eq!(line.descent().get(), dh);
        }
        assert_eq!(layout.logical_bounds().height(), count as f64 * (ah + dh));
    }
}
