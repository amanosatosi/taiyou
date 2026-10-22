#![cfg(feature = "native")]
use std::sync::Arc;
use taiyou_render::renderer::*;

fn fonts() -> Arc<ValidatedFontCatalog> {
    let bytes = std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap()).unwrap();
    let mut ctx = FontContext::new().unwrap();
    let mut b = FontCatalogBuilder::new(Limits::default());
    b.register(&mut ctx, &bytes, 0).unwrap();
    b.snapshot(&mut CatalogIngestor::new()).unwrap()
}
fn parsed(tags: &str, paint: &str) -> Arc<Track> {
    let default_position = if paint.contains(r"\pos(") || tags.contains(r"\pos(") {
        ""
    } else {
        r"\pos(80,60)"
    };
    let source = format!(
        "[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow,BackColour\nStyle: Default,Source Sans 3,30,0,0,&HFF000000\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:02.00,{{{default_position}{paint}{tags}}}Hg\n"
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
    .track()
    .clone()
}
fn request(now: i64) -> RenderRequest {
    RenderRequest {
        now: TimeUs::from_micros(now),
        viewport: Viewport::full(Size::new(160, 100)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    }
}
fn frame(tags: &str, paint: &str) -> Frame {
    Renderer::new(RendererConfig::default())
        .unwrap()
        .render(parsed(tags, paint), fonts(), &request(500_000))
        .unwrap()
}
#[test]
fn real_blur_threshold_zero_negative_reset_and_source_order() {
    let baseline = frame("", "");
    for tags in [
        r"\blur0",
        r"\be0",
        r"\blur-3",
        r"\blur-5",
        r"\blur0.333332",
        r"\blur3\blur",
        r"\be3\be",
        r"\blur2\be2\r",
        r"\blur2\blur0\be1\be0",
    ] {
        assert_eq!(frame(tags, "").pixels(), baseline.pixels(), "{tags}");
    }
    for tags in [
        r"\blur0.333334",
        r"\blur1",
        r"\blur4",
        r"\be0.5",
        r"\be1",
        r"\be2.5",
        r"\blur1\be1",
    ] {
        assert_ne!(frame(tags, "").pixels(), baseline.pixels(), "{tags}");
    }
    assert_eq!(
        frame(r"\blur1\be1", "").pixels(),
        frame(r"\be1\blur1", "").pixels()
    );
}
#[test]
fn outline_body_shadow_and_frame_edge_halo_use_masks_before_composition() {
    // With an outline xy filters E, preserving the unfiltered body F.
    let invisible = r"\bord3\3a&HFF&\4a&HFF&";
    assert_eq!(
        frame("", invisible).pixels(),
        frame(r"\blur3\be1", invisible).pixels()
    );
    for paint in [
        r"\bord3\shad0",
        r"\bord0\shad4\4a&H00&",
        r"\bord3\shad4\4a&H70&",
        r"\bord0\pos(0,5)",
    ] {
        assert_ne!(
            frame("", paint).pixels(),
            frame(r"\blur2\be1", paint).pixels()
        );
    }
    let fonts = fonts();
    let track = parsed(r"\blur2\be1.5", r"\bord2\shad3\4a&H40&");
    let mut renderer = Renderer::new(RendererConfig::default()).unwrap();
    let expected = renderer
        .render(track.clone(), fonts.clone(), &request(500_000))
        .unwrap();
    for time in [1_500_000, 0, 500_000, 2_500_000, 250_000, 500_000] {
        let actual = renderer
            .render(track.clone(), fonts.clone(), &request(time))
            .unwrap();
        let fresh = Renderer::new(RendererConfig::default())
            .unwrap()
            .render(track.clone(), fonts.clone(), &request(time))
            .unwrap();
        assert_eq!(actual.pixels(), fresh.pixels());
        assert_eq!(actual.diagnostics(), fresh.diagnostics());
        if time < 2_000_000 {
            assert_eq!(actual.pixels(), expected.pixels());
        }
    }
}
#[test]
fn excessive_filter_rejects_frame_and_does_not_poison_next_render() {
    let fonts = fonts();
    let mut renderer = Renderer::new(RendererConfig::default()).unwrap();
    for tags in [r"\blur9999", r"\be9999"] {
        let error = renderer
            .render(parsed(tags, ""), fonts.clone(), &request(1))
            .unwrap_err();
        assert_eq!(error.code(), ErrorCode::LimitExceeded);
        assert_eq!(error.event(), Some(EventId::new(0)));
    }
    let track = parsed(r"\blur1", "");
    let actual = renderer
        .render(track.clone(), fonts.clone(), &request(1))
        .unwrap();
    let fresh = Renderer::new(RendererConfig::default())
        .unwrap()
        .render(track, fonts, &request(1))
        .unwrap();
    assert_eq!(actual.pixels(), fresh.pixels());
}
