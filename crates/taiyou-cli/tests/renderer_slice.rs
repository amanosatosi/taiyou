#![cfg(feature = "native")]
use std::{path::PathBuf, sync::Arc};
use taiyou_render::renderer::*;

mod output_safety;

fn limits() -> Limits {
    Limits::default()
        .with_cache_bytes(0)
        .with_transient_bytes(512 * 1024 * 1024)
        .unwrap()
}
fn catalog(all: bool) -> Arc<ValidatedFontCatalog> {
    let mut context = FontContext::with_limits(limits()).unwrap();
    let mut builder = FontCatalogBuilder::new(limits());
    for (id, ext) in [
        ("latin", "otf"),
        ("arabic", "ttf"),
        ("indic", "ttf"),
        ("cjk", "otf"),
    ] {
        if !all && id != "latin" {
            continue;
        }
        let file = PathBuf::from(std::env::var("RUNNER_TEMP").unwrap())
            .join("taiyou-shaping-fonts")
            .join(format!("{id}.{ext}"));
        builder
            .register(&mut context, &std::fs::read(file).unwrap(), 0)
            .unwrap();
    }
    builder.snapshot(&mut CatalogIngestor::new()).unwrap()
}
fn source(events: &str) -> String {
    format!(
        "[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,BackColour,Outline,Shadow,Alignment,MarginL,MarginR,MarginV\nStyle: Default,Source Sans 3,20,&H00FFFFFF,&HFF000000,0,0,7,10,10,10\n[Events]\nFormat: Layer,Start,End,Style,Text\n{events}"
    )
}
fn dialogue(text: &str) -> String {
    format!("Dialogue: 0,0:00:00.10,0:00:01.00,Default,{text}")
}
fn track(events: &str) -> Arc<Track> {
    taiyou_ass::parse_ass(
        source(events).as_bytes(),
        &ParseOptions::new(
            InputEncoding::Utf8,
            Profile::CoreV1,
            limits(),
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
fn renderer(limits: Limits) -> Renderer {
    Renderer::new(RendererConfig {
        limits,
        ..Default::default()
    })
    .unwrap()
}
fn reference(a: u8) -> Option<Vec<u8>> {
    std::env::var_os("TAIYOU_RENDER_REFERENCE_DIR")
        .map(|d| std::fs::read(PathBuf::from(d).join(format!("align{a}.a8"))).unwrap())
}
fn rgba(mask: &[u8], color: [u8; 4]) -> Vec<u8> {
    mask.iter()
        .flat_map(|v| {
            let a = (u32::from(color[3]) * u32::from(*v) + 127) / 255;
            [
                (u32::from(color[0]) * a + 127) / 255,
                (u32::from(color[1]) * a + 127) / 255,
                (u32::from(color[2]) * a + 127) / 255,
                a,
            ]
            .map(|v| v as u8)
        })
        .collect()
}
#[test]
fn renderer_slice_timing_seeks_owned_pixels() {
    let fonts = catalog(false);
    let track = track(&dialogue("{\\fs40}H"));
    let mut renderer = renderer(limits());
    let first = renderer
        .render(track.clone(), fonts.clone(), &request(100_000))
        .unwrap();
    assert!(first.pixels().chunks_exact(4).any(|p| p == [255; 4]));
    for now in [-100, 0, 99_999, 1_000_000, 2_000_000] {
        assert!(
            renderer
                .render(track.clone(), fonts.clone(), &request(now))
                .unwrap()
                .pixels()
                .iter()
                .all(|p| *p == 0)
        );
    }
    for now in [999_999, 100_000, 500_000, 200_000, 999_999, 300_000] {
        assert_eq!(
            renderer
                .render(track.clone(), fonts.clone(), &request(now))
                .unwrap()
                .pixels(),
            first.pixels()
        );
    }
    let pixels = first.pixels().to_vec();
    drop(renderer);
    drop(track);
    drop(fonts);
    assert_eq!(first.pixels(), pixels);
    assert_eq!(first.charged_bytes(), 160 * 100 * 4);
}
#[test]
fn renderer_slice_nine_alignments_independent_pixels_and_color() {
    let fonts = catalog(false);
    let mut renderer = renderer(limits());
    for a in 1..=9 {
        let track = track(&dialogue(&format!(
            "{{\\an{a}\\1c&H2850C8&\\alpha&H7F&\\4a&HFF&}}H"
        )));
        let frame = renderer
            .render(track, fonts.clone(), &request(500_000))
            .unwrap();
        assert!(frame.diagnostics().is_empty());
        if let Some(mask) = reference(a) {
            let expected = rgba(&mask, [200, 80, 40, 128]);
            if let Some(folder) = std::env::var_os("TAIYOU_RENDER_SAMPLE_DIR") {
                let folder = PathBuf::from(folder);
                std::fs::write(folder.join(format!("align{a}.actual.rgba")), frame.pixels())
                    .unwrap();
                std::fs::write(folder.join(format!("align{a}.expected.rgba")), &expected).unwrap();
            }
            assert_eq!(frame.pixels(), expected, "alignment {a}");
            let expected_peak = (u32::from(*mask.iter().max().unwrap()) * 128 + 127) / 255;
            assert_eq!(
                frame.pixels().chunks_exact(4).map(|p| p[3]).max(),
                Some(expected_peak as u8)
            );
        }
        // Fractional alignment can leave a thin H with no fully covered pixel.
        // Exact reference pixels above, rather than an assumed opaque interior,
        // determine the independently justified peak opacity for each phase.
        let peak = frame.pixels().chunks_exact(4).map(|p| p[3]).max().unwrap();
        assert!(peak > 0 && peak <= 128);
    }
}
#[test]
fn renderer_slice_layer_source_order_and_paint_boundaries() {
    let fonts = catalog(false);
    let mut renderer = renderer(limits());
    let a = "Dialogue: 1,0:00:00.10,0:00:01.00,Default,{\\pos(10,10)\\1c&HFF0000&}H";
    let b = "Dialogue: -1,0:00:00.10,0:00:01.00,Default,{\\pos(10,10)\\1c&H0000FF&\\alpha&H7F&\\4a&HFF&}H";
    let two = renderer
        .render(
            track(&format!("{a}\n{b}")),
            fonts.clone(),
            &request(500_000),
        )
        .unwrap();
    let blue = renderer
        .render(track(a), fonts.clone(), &request(500_000))
        .unwrap();
    let red = renderer
        .render(track(b), fonts.clone(), &request(500_000))
        .unwrap();
    let base =
        reference(7).unwrap_or_else(|| blue.pixels().chunks_exact(4).map(|p| p[3]).collect());
    let mut expected = rgba(&base, [255, 0, 0, 128]);
    let over = rgba(&base, [0, 0, 255, 255]);
    for (dst, src) in expected.chunks_exact_mut(4).zip(over.chunks_exact(4)) {
        for i in 0..4 {
            dst[i] = (u32::from(src[i])
                + (u32::from(dst[i]) * (255 - u32::from(src[3])) + 127) / 255)
                .min(255) as u8;
        }
    }
    assert_eq!(two.pixels(), expected);
    assert_ne!(red.pixels(), blue.pixels());
    let same_layer = a.replace("Dialogue: 1,", "Dialogue: -1,");
    let source_order = renderer
        .render(
            track(&format!("{same_layer}\n{b}")),
            fonts.clone(),
            &request(500_000),
        )
        .unwrap();
    assert_ne!(source_order.pixels(), two.pixels());
    let painted = renderer
        .render(
            track(&dialogue("{\\fs40}H{\\1c&HFF0000&}H")),
            fonts,
            &request(500_000),
        )
        .unwrap();
    assert!(painted.pixels().chunks_exact(4).any(|p| p == [255; 4]));
    assert!(
        painted
            .pixels()
            .chunks_exact(4)
            .any(|p| p == [0, 0, 255, 255])
    );
}
#[test]
fn renderer_slice_position_breaks_fallback_and_clipping() {
    let fonts = catalog(true);
    let mut renderer = renderer(limits());
    let clipped = renderer
        .render(
            track(&dialogue("{\\pos(-2,-8)}H")),
            fonts.clone(),
            &request(500_000),
        )
        .unwrap();
    if let Some(mask) = reference(10) {
        assert_eq!(clipped.pixels(), rgba(&mask, [255; 4]));
    }
    for text in [
        "H\\NH",
        "H\\nH",
        "\\NH\\N",
        "  ",
        "",
        "Oé ب क 語",
        "f{\\1c&HFF0000&}i",
    ] {
        let t = track(&dialogue(text));
        let a = renderer
            .render(t.clone(), fonts.clone(), &request(500_000))
            .unwrap();
        let b = renderer
            .render(t, fonts.clone(), &request(100_000))
            .unwrap();
        assert_eq!(a.pixels(), b.pixels());
        assert!(a.diagnostics().is_empty(), "{text}: {:?}", a.diagnostics());
    }
    let empty = renderer
        .render(track(""), fonts, &request(500_000))
        .unwrap();
    assert!(empty.pixels().iter().all(|v| *v == 0));
}
#[test]
fn renderer_slice_unsupported_errors_diagnostics_and_limits() {
    let fonts = catalog(false);
    let mut r = renderer(limits());
    for text in ["{\\fe128}H", "{\\fe30}H", "{\\fe20}H"] {
        let t = track(&dialogue(text));
        let e = r
            .render(t.clone(), fonts.clone(), &request(500_000))
            .unwrap_err();
        assert_eq!(e.code(), ErrorCode::UnsupportedFeature);
        assert!(e.event().is_some() && e.span().is_some());
        let mut req = request(500_000);
        req.unsupported = UnsupportedPolicy::Tolerant;
        let f = r.render(t, fonts.clone(), &req).unwrap();
        assert!(
            f.diagnostics()
                .iter()
                .any(|d| d.code() == DiagnosticCode::UnsupportedFeature)
        );
    }
    let effect = source(&dialogue("H"))
        .replace("&H00FFFFFF,&HFF000000,0,0,7", "&H00FFFFFF,&HFF000000,2,2,7");
    let t = taiyou_ass::parse_ass(
        effect.as_bytes(),
        &Default::default(),
        &mut Default::default(),
    )
    .unwrap()
    .track()
    .clone();
    let outlined = r.render(t, fonts.clone(), &request(500_000)).unwrap();
    let fill_only = r
        .render(track(&dialogue("H")), fonts.clone(), &request(500_000))
        .unwrap();
    assert_ne!(outlined.pixels(), fill_only.pixels());
    assert!(
        Renderer::new(RendererConfig {
            workers: std::num::NonZeroUsize::new(2).unwrap(),
            ..Default::default()
        })
        .is_err()
    );
    assert!(
        Renderer::new(RendererConfig {
            profile: Profile::LegacyXyV1,
            ..Default::default()
        })
        .is_err()
    );
    let t = track(&dialogue("H"));
    for cap in [
        limits().with_transient_bytes(1000).unwrap(),
        limits().with_output_pixels(1).unwrap(),
        limits().with_path_segments_per_frame(1).unwrap(),
    ] {
        assert_eq!(
            renderer(cap)
                .render(t.clone(), fonts.clone(), &request(500_000))
                .unwrap_err()
                .code(),
            ErrorCode::LimitExceeded
        );
    }
    let mut small = renderer(limits().with_output_bytes(160 * 100 * 4).unwrap());
    let f = small
        .render(t.clone(), fonts.clone(), &request(500_000))
        .unwrap();
    let copy = f.clone();
    drop(f);
    assert_eq!(
        small
            .render(t.clone(), fonts.clone(), &request(500_000))
            .unwrap_err()
            .code(),
        ErrorCode::LimitExceeded
    );
    drop(copy);
    assert!(small.render(t, fonts, &request(500_000)).is_ok());
}
#[test]
fn renderer_slice_png_conversion_is_exact_and_preserves_frame() {
    let limits = limits();
    let mut output = OutputBudget::new(&limits);
    let raw = vec![0, 0, 0, 0, 64, 32, 1, 128, 255, 127, 0, 255];
    let frame = Frame::from_owned_pixels(
        Size::new(3, 1),
        raw.clone(),
        &limits,
        output.reserve(12).unwrap(),
    )
    .unwrap();
    let mut encoded = Vec::new();
    taiyou_cli::render::write_png(&frame, &mut encoded, &limits).unwrap();
    let decoder = png::Decoder::new(std::io::Cursor::new(encoded));
    let mut reader = decoder.read_info().unwrap();
    let mut data = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut data).unwrap();
    assert_eq!(
        &data[..info.buffer_size()],
        [0, 0, 0, 0, 128, 64, 2, 128, 255, 127, 0, 255]
    );
    assert_eq!(frame.pixels(), raw);
}

#[test]
fn renderer_slice_cumulative_event_reservations() {
    let fonts = catalog(false);
    let t = track(&dialogue("H"));
    // Find a bounded independently observable successful single-event budget,
    // then prove many individually valid events cannot each reuse that budget.
    let (mut low, mut high) = (1024_u64, 64 * 1024 * 1024_u64);
    for _ in 0..16 {
        let middle = (low + high) / 2;
        let result = renderer(limits().with_transient_bytes(middle).unwrap()).render(
            t.clone(),
            fonts.clone(),
            &request(500_000),
        );
        match result {
            Ok(_) => high = middle,
            Err(e) => {
                assert_eq!(e.code(), ErrorCode::LimitExceeded);
                low = middle + 1;
            }
        }
    }
    let cap = limits().with_transient_bytes(high + 1024).unwrap();
    assert!(
        renderer(cap.clone())
            .render(t, fonts.clone(), &request(500_000))
            .is_ok()
    );
    let many = (0..40)
        .map(|_| dialogue("H"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        renderer(cap)
            .render(track(&many), fonts.clone(), &request(500_000))
            .unwrap_err()
            .code(),
        ErrorCode::LimitExceeded
    );
    assert_eq!(
        renderer(limits().with_path_segments_per_frame(128).unwrap())
            .render(track(&many), fonts, &request(500_000))
            .unwrap_err()
            .code(),
        ErrorCode::LimitExceeded
    );
}

#[test]
fn renderer_slice_cli_rejects_invalid_input_and_flags_without_output() {
    use std::process::Command;
    let directory =
        std::env::temp_dir().join(format!("taiyou-t014-invalid-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let ass = directory.join("invalid.ass");
    let png = directory.join("output.png");
    std::fs::write(&ass, [0xff, 0xfe]).unwrap();
    assert!(!png.exists());
    let font =
        PathBuf::from(std::env::var("RUNNER_TEMP").unwrap()).join("taiyou-shaping-fonts/latin.otf");
    let base = vec![
        "render".into(),
        "--input".into(),
        ass.into_os_string(),
        "--font".into(),
        font.into_os_string(),
        "--time-us".into(),
        "500000".into(),
        "--width".into(),
        "160".into(),
        "--height".into(),
        "100".into(),
        "--profile".into(),
        "core-v1".into(),
        "--output".into(),
        png.clone().into_os_string(),
    ];
    for (args, code) in [
        (base.clone(), "InvalidEncoding"),
        (
            {
                let mut a = base.clone();
                a[8] = "0".into();
                a
            },
            "InvalidArgument",
        ),
        (
            {
                let mut a = base.clone();
                a[6] = "NaN".into();
                a
            },
            "InvalidArgument",
        ),
        (
            {
                let mut a = base.clone();
                a.extend(["--wat".into(), "1".into()]);
                a
            },
            "InvalidArgument",
        ),
        (
            {
                let mut a = base.clone();
                a.drain(3..5);
                a
            },
            "FontUnavailable",
        ),
        (
            {
                let mut a = base.clone();
                a.extend(["--width".into(), "160".into()]);
                a
            },
            "InvalidArgument",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_taiyou"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8(output.stderr).unwrap().contains(code));
        assert!(!png.exists());
    }
}

#[test]
fn renderer_slice_parsed_diagnostics_and_header_capabilities() {
    let fonts = catalog(false);
    let mut renderer = renderer(limits());
    let malformed = source("Dialogue: broken");
    let parsed = taiyou_ass::parse_ass(
        malformed.as_bytes(),
        &Default::default(),
        &mut Default::default(),
    )
    .unwrap();
    assert_eq!(
        renderer
            .render_parsed(&parsed, fonts.clone(), &request(500_000))
            .unwrap_err()
            .code(),
        ErrorCode::InvalidScript
    );
    let mut tolerant = request(500_000);
    tolerant.unsupported = UnsupportedPolicy::Tolerant;
    let frame = renderer
        .render_parsed(&parsed, fonts.clone(), &tolerant)
        .unwrap();
    assert!(
        frame
            .diagnostics()
            .iter()
            .any(|d| d.code() == DiagnosticCode::MalformedRecord)
    );
    let colored = dialogue(r"{\1c&H2163B7&\1a&H40&\3c&H1F8A52&\3a&H50&\4c&H964C20&\4a&H60&\bord2\shad3}H");
    let reference = renderer
        .render(track(&colored), fonts.clone(), &request(500_000))
        .unwrap();
    assert!(reference.pixels().chunks_exact(4).any(|p| p[3] > 0 && p[0] != p[1]));
    for matrix in ["None", "TV.601", "TV.709", "PC.601", "PC.709", "TV.240M", "PC.240M", "TV.FCC", "PC.FCC"] {
        let header = format!("YCbCr Matrix: {matrix}");
        let text =
            source(&colored).replace("WrapStyle: 2", &format!("WrapStyle: 2\n{header}"));
        let parsed = taiyou_ass::parse_ass(
            text.as_bytes(),
            &Default::default(),
            &mut Default::default(),
        )
        .unwrap();
        assert_eq!(parsed.track().script().ycbcr_matrix(), matrix);
        for request in [&request(500_000), &tolerant] {
            let frame = renderer.render_parsed(&parsed, fonts.clone(), request).unwrap();
            assert_eq!(frame.pixels(), reference.pixels(), "raw RGB {matrix}");
            assert!(frame.diagnostics().iter().all(|d| d.code() != DiagnosticCode::UnsupportedFeature));
        }
    }
    let empty = FontCatalogBuilder::new(limits())
        .snapshot(&mut Default::default())
        .unwrap();
    // Validated empty catalogs are valid data; active glyph work needs a font.
    assert_eq!(
        renderer
            .render(track(&dialogue("H")), empty, &request(500_000))
            .unwrap_err()
            .code(),
        ErrorCode::FontUnavailable
    );
}
