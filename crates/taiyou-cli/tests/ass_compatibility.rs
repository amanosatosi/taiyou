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
    taiyou_ass::parse_ass(source.as_bytes(), &ParseOptions::default(), &mut TrackIngestor::new())
        .unwrap().track().clone()
}
fn render(track: Arc<Track>, fonts: Arc<ValidatedFontCatalog>) -> Frame {
    Renderer::new(RendererConfig::default()).unwrap().render(track, fonts, &RenderRequest {
        now: TimeUs::from_micros(500_000),
        viewport: Viewport::full(Size::new(200,120)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    }).unwrap()
}

#[test]
fn zero_style_and_override_depth_suppress_shadow_without_darkening_edges() {
    let fonts = fonts();
    for border in [0, 2] {
        let tags = format!("\\bord{border}\\1c&HFFFFFF&\\3c&HFFFFFF&");
        let zero = render(source("0", &tags, "Hg"), fonts.clone());
        let hidden = render(source("0", &format!("{tags}\\4a&HFF&"), "Hg"), fonts.clone());
        let override_zero = render(source("4", &format!("{tags}\\shad0"), "Hg"), fonts.clone());
        assert_eq!(zero.pixels(), hidden.pixels());
        assert_eq!(override_zero.pixels(), hidden.pixels());
        assert!(zero.pixels().chunks_exact(4).any(|p| p[3] > 0 && p[3] < 255));
        let positive = render(source("4", &tags, "Hg"), fonts.clone());
        assert_ne!(positive.pixels(), zero.pixels());
        let invisible_body = format!("{tags}\\1a&HFF&\\3a&HFF&");
        let absent = render(source("0", &invisible_body, "Hg"), fonts.clone());
        assert!(absent.pixels().iter().all(|&p| p == 0));
        let visible = render(source("4", &invisible_body, "Hg"), fonts.clone());
        assert!(visible.pixels().iter().any(|&p| p != 0));
    }
}
