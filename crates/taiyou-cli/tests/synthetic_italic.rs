#![cfg(feature = "native")]
use std::{path::PathBuf, sync::Arc};
use taiyou_render::renderer::*;

fn font(name: &str) -> Arc<ValidatedFontCatalog> {
    let mut context = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    let path = PathBuf::from(std::env::var("TAIYOU_TEST_FONT").unwrap()).with_file_name(name);
    builder
        .register(&mut context, &std::fs::read(path).unwrap(), 0)
        .unwrap();
    builder.snapshot(&mut CatalogIngestor::new()).unwrap()
}
fn track(italic: bool) -> Arc<Track> {
    let text = format!(
        "[Script Info]\nPlayResX: 200\nPlayResY: 100\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Italic,Outline,Shadow,Alignment\nStyle: Default,Source Sans 3,50,{},0,0,7\n[Events]\nFormat: Start,End,Style,Text\nDialogue: 0:00:00.00,0:00:02.00,Default,{{\\pos(20,20)}}AH",
        if italic { -1 } else { 0 }
    );
    taiyou_ass::parse_ass(
        text.as_bytes(),
        &ParseOptions::default(),
        &mut TrackIngestor::new(),
    )
    .unwrap()
    .track()
    .clone()
}
fn request(time: i64) -> RenderRequest {
    RenderRequest {
        now: TimeUs::from_micros(time),
        viewport: Viewport::full(Size::new(200, 100)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    }
}
#[test]
fn missing_italic_face_is_synthesized_once_and_is_seek_deterministic() {
    let fonts = font("font.otf");
    let mut renderer = Renderer::new(RendererConfig::default()).unwrap();
    let upright = renderer
        .render(track(false), fonts.clone(), &request(0))
        .unwrap();
    let italic_track = track(true);
    let italic = renderer
        .render(italic_track.clone(), fonts.clone(), &request(0))
        .unwrap();
    assert!(upright.pixels().iter().any(|&v| v != 0));
    assert!(italic.pixels().iter().any(|&v| v != 0));
    assert_ne!(upright.pixels(), italic.pixels());
    renderer
        .render(italic_track.clone(), fonts.clone(), &request(1_000_000))
        .unwrap();
    let sought = renderer.render(italic_track, fonts, &request(0)).unwrap();
    assert_eq!(italic.pixels(), sought.pixels());
}
#[test]
fn actual_italic_face_does_not_receive_a_second_slant() {
    let fonts = font("flagged-italic.otf");
    assert!(fonts.faces()[0].italic());
    let mut renderer = Renderer::new(RendererConfig::default()).unwrap();
    let a = renderer
        .render(track(false), fonts.clone(), &request(0))
        .unwrap();
    let b = renderer.render(track(true), fonts, &request(0)).unwrap();
    assert_eq!(a.pixels(), b.pixels());
}
