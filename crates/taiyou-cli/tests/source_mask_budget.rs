//! Regression for source-mask bounds under a caller's constrained staging budget.
#![cfg(feature = "native")]
use std::{path::PathBuf, sync::Arc};
use taiyou_render::renderer::*;

fn fonts() -> Arc<ValidatedFontCatalog> {
    let path =
        PathBuf::from(std::env::var("RUNNER_TEMP").unwrap()).join("taiyou-shaping-fonts/latin.otf");
    let mut context = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    builder
        .register(&mut context, &std::fs::read(path).unwrap(), 0)
        .unwrap();
    builder.snapshot(&mut CatalogIngestor::new()).unwrap()
}
fn parsed(events: &str) -> ParsedTrack {
    let source = format!(
        "[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,BackColour,Outline,Shadow,Alignment,MarginL,MarginR,MarginV\nStyle: Default,Source Sans 3,20,&H00FFFFFF,&HFF000000,0,0,7,13,21,9\n[Events]\nFormat: Layer,Start,End,Style,Text\n{events}"
    );
    taiyou_ass::parse_ass(
        source.as_bytes(),
        &ParseOptions::default(),
        &mut TrackIngestor::new(),
    )
    .unwrap()
}
fn event(text: &str) -> String {
    format!("Dialogue: 0,0:00:00.00,0:00:02.00,Default,{text}\n")
}
fn renderer(mib: u64) -> Renderer {
    Renderer::new(RendererConfig {
        limits: Limits::default()
            .with_transient_bytes(mib * 1024 * 1024)
            .unwrap(),
        ..RendererConfig::default()
    })
    .unwrap()
}
fn request(ms: i64) -> RenderRequest {
    RenderRequest {
        now: TimeUs::from_micros(ms * 1000),
        viewport: Viewport::full(Size::new(160, 100)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    }
}
#[test]
fn completed_offscreen_masks_do_not_exhaust_later_shaping() {
    // Exact reduced workload exposed by the unchanged T017 executor. Collision
    // still sees all 256 events; only final invisible masks may be discarded.
    let p = parsed(
        &event(&format!(
            r"{{\fs5\bord0.5\shad1.25\4a&H00&}}{}",
            "H".repeat(32)
        ))
        .repeat(256),
    );
    let fonts = fonts();
    let expected = renderer(128)
        .render_parsed(&p, fonts.clone(), &request(750))
        .unwrap();
    let mut constrained = renderer(32);
    for ms in [750, 1250, 250, 750, 1999] {
        let actual = constrained
            .render_parsed(&p, fonts.clone(), &request(ms))
            .unwrap();
        assert_eq!(actual.pixels(), expected.pixels());
        assert_eq!(actual.diagnostics(), expected.diagnostics());
    }
    assert!(expected.pixels().iter().any(|&v| v != 0));
}
#[test]
fn offscreen_body_keeps_visible_shadow_and_blur_halo() {
    let fonts = fonts();
    // The body is offscreen but the displaced shadow is in the frame.
    // Sweep karaoke retains a filled shadow even when both body colors hide.
    let shadow = parsed(&event(r"{\pos(-2000,0)\1a&HFF&\2a&HFF&\kf100\xshad2000\yshad0\4a&H00&}H"));
    let frame = renderer(32)
        .render_parsed(&shadow, fonts.clone(), &request(750))
        .unwrap();
    assert!(frame.pixels().chunks_exact(4).any(|p| p[3] != 0));
    // The blur halo must be constructed before making the visibility decision.
    let halo = parsed(&event(r"{\pos(-8,5)\blur4\p1}m 0 0 l 4 0 4 10 0 10"));
    let frame = renderer(32)
        .render_parsed(&halo, fonts, &request(750))
        .unwrap();
    assert!(frame.pixels().chunks_exact(4).any(|p| p[3] != 0));
}
