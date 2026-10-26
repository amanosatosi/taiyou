//! Supplemental exact-output/cache regressions, independent of frozen oracles.
#![cfg(feature = "native")]
use std::sync::Arc;
use taiyou_render::renderer::*;

fn fonts(path: &str) -> Arc<ValidatedFontCatalog> {
    let bytes = std::fs::read(std::env::var(path).unwrap()).unwrap();
    let mut context = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    builder.register(&mut context, &bytes, 0).unwrap();
    builder.snapshot(&mut CatalogIngestor::new()).unwrap()
}
fn track(text: &str, ingestion: &mut TrackIngestor) -> Arc<Track> {
    let source = format!(
        "[Script Info]\nPlayResX: 1920\nPlayResY: 1080\nWrapStyle: 2\nScaledBorderAndShadow: yes\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow\nStyle: Default,Source Sans 3,24,2,2\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:10.00,{text}\n"
    );
    taiyou_ass::parse_ass(source.as_bytes(), &ParseOptions::default(), ingestion)
        .unwrap()
        .track()
        .clone()
}
fn config(cache: CacheConfig, budget: u64) -> RendererConfig {
    RendererConfig {
        cache,
        limits: Limits::default()
            .with_transient_bytes(512 * 1024 * 1024)
            .unwrap()
            .with_path_segments_per_frame(32_000_000)
            .unwrap()
            .with_cache_bytes(budget),
        ..Default::default()
    }
}
fn request(width: u32, now: i64) -> RenderRequest {
    RenderRequest {
        now: TimeUs::from_micros(now),
        viewport: Viewport::full(Size::new(width, width * 9 / 16)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    }
}
fn compare(
    renderer: &mut Renderer,
    track: Arc<Track>,
    fonts: Arc<ValidatedFontCatalog>,
    request: &RenderRequest,
) {
    let expected = Renderer::new(config(CacheConfig::Disabled, 0))
        .unwrap()
        .render(track.clone(), fonts.clone(), request)
        .unwrap();
    let actual = renderer.render(track, fonts, request).unwrap();
    assert_eq!(expected.pixels(), actual.pixels());
    assert_eq!(expected.diagnostics(), actual.diagnostics());
}
#[test]
fn cache_off_on_exact_across_frames_effects_tracks_fonts_and_4k() {
    let fonts = fonts("TAIYOU_TEST_FONT");
    let mut ingestion = TrackIngestor::new();
    let cases = [
        "Hg",
        r"{\p1}m 0 0 b 30 -20 70 60 90 0 l 90 40 0 40",
        r"{\bord12\shad6}Hg",
        r"{\bord4\blur6\be1.5}Hg",
        r"{\kf80}H{\kf120}g",
        r"{\t(0,5000,\frz30\fscx140\blur3)}Hg",
        r"{\t(0,5000,\1c&HFF0080&)}Hg",
        r"{\move(200,200,800,500,0,5000)}Hg",
        r"{\clip(m 0 0 l 1800 0 1800 1000 0 1000)\bord5}Hg",
        r"{\frx20\ybord0\xbord3}Hg",
        r"{\1a&H80&\2a&H80&\bord3\shad4\blur1}Hg",
    ];
    let mut cached = Renderer::new(config(CacheConfig::Bounded, 16 * 1024 * 1024)).unwrap();
    for width in [1920, 3840] {
        for text in cases {
            let track = track(text, &mut ingestion);
            for now in [1_000_000, 1_000_000, 1_016_667, 7_500_000, 0, 1_000_000] {
                compare(
                    &mut cached,
                    track.clone(),
                    fonts.clone(),
                    &request(width, now),
                );
                assert!(cached.cache_stats().retained_bytes <= 16 * 1024 * 1024);
            }
        }
    }
    // Catalog revisions can coincide across independent host ingestors. Keys
    // must still reflect the actual newly resolved outline, not just that ID.
    let other_fonts = fonts_from_bold();
    let text = track("Hg", &mut ingestion);
    compare(
        &mut cached,
        text.clone(),
        other_fonts,
        &request(1920, 1_000_000),
    );
    compare(&mut cached, text, fonts, &request(1920, 1_000_000));
}
fn fonts_from_bold() -> Arc<ValidatedFontCatalog> {
    fonts("TAIYOU_TEST_BOLD")
}

#[test]
fn warm_paint_and_integer_placement_reuse_but_phase_changes_rebuild() {
    let fonts = fonts("TAIYOU_TEST_FONT");
    let mut ingestion = TrackIngestor::new();
    let mut cached = Renderer::new(config(CacheConfig::Bounded, 16 * 1024 * 1024)).unwrap();
    let text = track(r"{\pos(600,500)\t(0,5000,\1c&HFF0080&)}Hg", &mut ingestion);
    compare(
        &mut cached,
        text.clone(),
        fonts.clone(),
        &request(1920, 1_000_000),
    );
    compare(&mut cached, text, fonts.clone(), &request(1920, 2_000_000));
    assert!(cached.cache_stats().hits > 0);
    assert_eq!(cached.cache_stats().misses, 0);
    let moved = track(r"{\pos(610,510)}Hg", &mut ingestion);
    compare(&mut cached, moved, fonts.clone(), &request(1920, 1_000_000));
    assert!(cached.cache_stats().hits > 0);
    assert_eq!(cached.cache_stats().misses, 0);
    let fractional = track(r"{\pos(610.125,510.125)}Hg", &mut ingestion);
    compare(&mut cached, fractional, fonts, &request(1920, 1_000_000));
    assert!(cached.cache_stats().misses > 0);
}

#[test]
fn zero_tiny_eviction_oversize_and_retained_frame_lifetime() {
    let fonts = fonts("TAIYOU_TEST_FONT");
    let mut ingestion = TrackIngestor::new();
    for budget in [0, 1, 8192, 32_768] {
        let mut cached = Renderer::new(config(CacheConfig::Bounded, budget)).unwrap();
        let old = cached
            .render(track("H", &mut ingestion), fonts.clone(), &request(1920, 1))
            .unwrap();
        let original = old.pixels().to_vec();
        for n in 0..24 {
            let text = track(&format!(r"{{\frz{n}\bord4}}Hg"), &mut ingestion);
            compare(&mut cached, text, fonts.clone(), &request(1920, 1_000_000));
            assert!(cached.cache_stats().retained_bytes <= budget);
        }
        drop(cached);
        assert_eq!(old.pixels(), original);
    }
}

#[test]
fn worker_cache_matrix_exact_with_seeks_and_resource_failures() {
    use std::num::NonZeroUsize;
    let fonts = fonts("TAIYOU_TEST_FONT");
    let mut ingestion = TrackIngestor::new();
    let cases = [
        "Hg",
        r"{\p1}m 0 0 b 30 -20 70 60 90 0 l 90 40 0 40",
        r"{\bord12\shad6}Hg",
        r"{\bord4\blur6\be1.5}Hg",
        r"{\kf80}H{\kf120}g",
        r"{\t(0,5000,\frz30\fscx140\blur3)}Hg",
        r"{\move(200,200,800,500,0,5000)}Hg",
        r"{\clip(m 0 0 l 1800 0 1800 1000 0 1000)\bord5}Hg",
        r"{\iclip(400,400,800,600)\1a&H80&\bord4\blur3}Hg",
    ];
    for workers in [1, 2, 4, 8] {
        let mut cfg = config(CacheConfig::Bounded, 16 * 1024 * 1024);
        cfg.workers = NonZeroUsize::new(workers).unwrap();
        let mut renderer = Renderer::new(cfg).unwrap();
        for (i, text) in cases.iter().enumerate() {
            let text = track(text, &mut ingestion);
            for width in [960, 3840] {
                if width == 3840 && ![3, 5, 8].contains(&i) {
                    continue;
                }
                for now in [1_000_000, 1_000_000, 1_016_667, 7_500_000, 0, 1_000_000] {
                    compare(
                        &mut renderer,
                        text.clone(),
                        fonts.clone(),
                        &request(width, now),
                    );
                    assert!(renderer.cache_stats().retained_bytes <= 16 * 1024 * 1024);
                }
            }
        }
        let text = track("Hg", &mut ingestion);
        compare(
            &mut renderer,
            text.clone(),
            fonts_from_bold(),
            &request(960, 1),
        );
        compare(&mut renderer, text.clone(), fonts.clone(), &request(960, 1));
        let frame = renderer
            .render(text.clone(), fonts.clone(), &request(960, 1))
            .unwrap();
        let bytes = frame.pixels().to_vec();
        drop(renderer);
        assert_eq!(frame.pixels(), bytes);
        let mut low = config(CacheConfig::Bounded, 8192);
        low.limits = low.limits.with_transient_bytes(1).unwrap();
        let expected = Renderer::new(low.clone())
            .unwrap()
            .render(text.clone(), fonts.clone(), &request(960, 1))
            .unwrap_err();
        low.workers = NonZeroUsize::new(workers).unwrap();
        let actual = Renderer::new(low)
            .unwrap()
            .render(text, fonts.clone(), &request(960, 1))
            .unwrap_err();
        assert_eq!(actual.code(), expected.code());
        assert_eq!(actual.resource_failure(), expected.resource_failure());
    }
}
