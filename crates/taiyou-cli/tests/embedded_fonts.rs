#![cfg(feature = "native")]
use std::{path::PathBuf, sync::Arc};
use taiyou_render::renderer::*;

fn encode(bytes: &[u8]) -> String {
    let mut text = String::new();
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = chunk.get(1).copied().unwrap_or(0);
        let c = chunk.get(2).copied().unwrap_or(0);
        for value in [
            a >> 2,
            ((a & 3) << 4) | (b >> 4),
            ((b & 15) << 2) | (c >> 6),
            c & 63,
        ]
        .into_iter()
        .take(chunk.len() + 1)
        {
            text.push(char::from(value + 33));
        }
    }
    text.as_bytes()
        .chunks(80)
        .map(|line| std::str::from_utf8(line).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}
fn source(family: &str) -> String {
    format!(
        "[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow\nStyle: Default,{family},20,0,0\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:02.00,Embedded\n"
    )
}
fn parse(source: &str) -> ParsedTrack {
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
fn empty() -> Arc<ValidatedFontCatalog> {
    FontCatalogBuilder::new(Limits::default())
        .snapshot(&mut CatalogIngestor::new())
        .unwrap()
}
#[test]
fn embedded_collection_resolves_face_one_with_one_shared_font_byte_budget() {
    let collection = std::fs::read(std::env::var("TAIYOU_TEST_COLLECTION").unwrap()).unwrap();
    let limits = Limits::default()
        .with_font_bytes(collection.len() as u64)
        .unwrap();
    let mut context = FontContext::with_limits(limits.clone()).unwrap();
    let mut builder = FontCatalogBuilder::new(limits.clone());
    builder.register(&mut context, &collection, 0).unwrap();
    let first_only = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    // The unchanged two-face Source Sans collection has the same typographic
    // family on both faces. Its Bold PostScript family alias belongs only to 1.
    let family = "SourceSans3-Bold";
    builder.register(&mut context, &collection, 1).unwrap();
    let complete = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let request = RenderRequest {
        now: TimeUs::from_micros(1_000_000),
        viewport: Viewport::full(Size::new(160, 100)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    };
    let explicit = parse(&source(family));
    let error = Renderer::new(RendererConfig::default())
        .unwrap()
        .render_parsed(&explicit, first_only, &request)
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::FontUnavailable);
    let expected = Renderer::new(RendererConfig::default())
        .unwrap()
        .render_parsed(&explicit, complete, &request)
        .unwrap();
    assert!(expected.pixels().iter().any(|b| *b != 0));
    let input =
        source(family) + "[Fonts]\nfontname: ../../collection.otc\n" + &encode(&collection) + "\n";
    let parsed = parse(&input);
    assert_eq!(parsed.embedded_fonts().len(), 1);
    assert_eq!(parsed.embedded_fonts()[0].bytes(), collection);
    let mut renderer = Renderer::new(RendererConfig {
        limits,
        ..Default::default()
    })
    .unwrap();
    for now in [1_000_000, 0, 1_500_000, 1_000_000] {
        let request = RenderRequest {
            now: TimeUs::from_micros(now),
            ..request
        };
        let actual = renderer.render_parsed(&parsed, empty(), &request).unwrap();
        assert_eq!(actual.pixels(), expected.pixels());
        assert_eq!(actual.diagnostics(), expected.diagnostics());
    }
}
#[test]
fn embedded_single_multiple_localized_aliases_and_seek_match_explicit_font_pixels() {
    let path = PathBuf::from(std::env::var("TAIYOU_TEST_FONT").unwrap());
    let regular = std::fs::read(path.with_file_name("alias-regular.otf")).unwrap();
    let bold = std::fs::read(path.with_file_name("alias-bold.otf")).unwrap();
    let mut ctx = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    builder.register(&mut ctx, &regular, 0).unwrap();
    builder.register(&mut ctx, &bold, 0).unwrap();
    let catalog = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let request = RenderRequest {
        now: TimeUs::from_micros(1_000_000),
        viewport: Viewport::full(Size::new(160, 100)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    };
    for family in [
        "たいよう 別名",
        "Taiyou Alias Family",
        "Taiyou Typographic Family",
    ] {
        let explicit = parse(&source(family));
        let expected = Renderer::new(RendererConfig::default())
            .unwrap()
            .render_parsed(&explicit, catalog.clone(), &request)
            .unwrap();
        assert!(expected.pixels().iter().any(|b| *b != 0));
        for multiple in [false, true] {
            let mut input = source(family)
                + "[Fonts]\nfontname: ../../never-created.ttf\n"
                + &encode(&regular)
                + "\n";
            if multiple {
                input += &format!("fontname: C:\\never-created-bold.ttf\n{}\n", encode(&bold));
            }
            let parsed = parse(&input);
            assert_eq!(parsed.embedded_fonts().len(), if multiple { 2 } else { 1 });
            let mut renderer = Renderer::new(RendererConfig::default()).unwrap();
            for now in [1_000_000, 0, 1_500_000, 1_000_000] {
                let mut request = request;
                request.now = TimeUs::from_micros(now);
                let actual = renderer.render_parsed(&parsed, empty(), &request).unwrap();
                assert_eq!(actual.pixels(), expected.pixels());
                assert_eq!(actual.diagnostics(), expected.diagnostics());
            }
        }
    }
}
#[test]
fn embedded_bytes_cannot_bypass_native_validation_or_combined_catalog_limit() {
    let parsed = parse(&(source("Absent") + "[Fonts]\nfontname: font.ttf\n!!!!\n"));
    let request = RenderRequest {
        now: TimeUs::from_micros(1),
        viewport: Viewport::full(Size::new(160, 100)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    };
    let error = Renderer::new(RendererConfig::default())
        .unwrap()
        .render_parsed(&parsed, empty(), &request)
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::NativeFailure);
    assert_eq!(error.span(), Some(parsed.embedded_fonts()[0].source()));
    let font = std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap()).unwrap();
    let parsed =
        parse(&(source("Source Sans 3") + "[Fonts]\nfontname: font.ttf\n" + &encode(&font) + "\n"));
    let limits = Limits::default()
        .with_font_bytes(font.len() as u64 - 1)
        .unwrap();
    let error = Renderer::new(RendererConfig {
        limits,
        ..Default::default()
    })
    .unwrap()
    .render_parsed(&parsed, empty(), &request)
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::LimitExceeded);
    let bold = std::fs::read(
        PathBuf::from(std::env::var("TAIYOU_TEST_FONT").unwrap()).with_file_name("alias-bold.otf"),
    )
    .unwrap();
    let combined =
        parse(&(source("Source Sans 3") + "[Fonts]\nfontname: bold.otf\n" + &encode(&bold) + "\n"));
    let mut context = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    builder.register(&mut context, &font, 0).unwrap();
    let host = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let limits = Limits::default()
        .with_font_bytes(font.len().max(bold.len()) as u64)
        .unwrap();
    let error = Renderer::new(RendererConfig {
        limits,
        ..Default::default()
    })
    .unwrap()
    .render_parsed(&combined, host, &request)
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::LimitExceeded);
}
