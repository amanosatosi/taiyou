//! Actions-only performance smoke. No external renderer comparison is implied.
#[cfg(feature = "native")]
fn main() {
    use sha2::{Digest, Sha256};
    use std::{hint::black_box, io::Write, time::Instant};
    use taiyou_render::renderer::*;
    let digest_hex = |bytes: &[u8]| {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    let workers: usize = std::env::var("TAIYOU_PERF_WORKERS")
        .unwrap_or_else(|_| "1".into())
        .parse()
        .unwrap();
    let band_rows: usize = std::env::var("TAIYOU_PERF_BAND_ROWS")
        .unwrap_or_else(|_| "64".into())
        .parse()
        .unwrap();
    let fixed = std::env::var("TAIYOU_PERF_FIXED_SAMPLES").as_deref() == Ok("1");
    let samples: usize = std::env::var("TAIYOU_PERF_SAMPLES")
        .unwrap_or_else(|_| "31".into())
        .parse()
        .unwrap();
    assert!(samples > 0);
    // Explicit experiment allowance; production defaults are unchanged.
    let limits = Limits::default()
        .with_transient_bytes(512 * 1024 * 1024)
        .unwrap()
        .with_path_segments_per_frame(32_000_000)
        .unwrap();
    let mut failures =
        std::fs::File::create(std::env::var("TAIYOU_PERF_FAILURES").unwrap()).unwrap();
    writeln!(failures, "workload\twidth\tscenario\tresult").unwrap();
    let mut failed = false;
    let cache_mode = std::env::var("TAIYOU_PERF_CACHE").unwrap_or_else(|_| "off".into());
    assert!(["off", "masks", "on"].contains(&cache_mode.as_str()));
    let cached = cache_mode != "off";
    let cache_label = cache_mode.as_str();
    let config = RendererConfig {
        workers: std::num::NonZeroUsize::new(workers).unwrap(),
        limits: limits
            .clone()
            .with_cache_bytes(if cached { 128 * 1024 * 1024 } else { 0 }),
        cache: if cache_mode == "masks" {
            CacheConfig::MasksOnly
        } else if cached {
            CacheConfig::Bounded
        } else {
            CacheConfig::Disabled
        },
        ..Default::default()
    };
    let font_setup_start = Instant::now();
    let mut context = FontContext::with_limits(limits.clone()).unwrap();
    let mut builder = FontCatalogBuilder::new(limits.clone());
    builder
        .register(
            &mut context,
            &std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap()).unwrap(),
            0,
        )
        .unwrap();
    let fonts = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    drop(context);
    let font_setup_ns = font_setup_start.elapsed().as_nanos();
    let dialogue = "Reusable work, deterministic pixels.";
    let vector = r"{\p1}m 0 0 b 40 -50 80 90 120 0 b 160 -60 200 80 240 0 l 240 70 0 70{\p0}";
    let workloads = [
        ("dialogue", dialogue.to_owned(), 1),
        ("simultaneous", dialogue.to_owned(), 12),
        ("vector", vector.repeat(8), 1),
        ("thick-borders", format!(r"{{\bord14\shad6}}{dialogue}"), 3),
        (
            "large-blur",
            format!(r"{{\bord6\blur12\be1.5}}{dialogue}"),
            2,
        ),
        (
            "karaoke",
            r"{\kf40}Reusable {\kf60}geometry {\kf80}through time".into(),
            1,
        ),
        (
            "transforms",
            format!(r"{{\t(0,5000,\frz24\fscx140\blur4)}}{dialogue}"),
            2,
        ),
        (
            "clips",
            format!(r"{{\clip(m 100 100 l 1800 180 1700 1000 100 900)\bord5}}{dialogue}"),
            3,
        ),
        (
            "paint-animation",
            format!(r"{{\t(0,5000,\1c&H0040FF&\alpha&H80&)}}{dialogue}"),
            3,
        ),
        (
            "movement",
            format!(r"{{\move(300,300,1500,800,0,5000)}}{dialogue}"),
            2,
        ),
        (
            "combined-abuse",
            format!(
                r"{{\bord10\shad7\blur6\be1.5\frz8\clip(0,0,1900,1050)\t(0,5000,\frz30\fscx130\1c&HFF0080&)}}{vector}{dialogue}"
            ),
            8,
        ),
    ];
    println!(
        "workload,width,height,scenario,sample,workers,cache,total_ns,evaluation_ns,shaping_ns,layout_ns,collision_ns,geometry_ns,clip_ns,masks_ns,effects_ns,paint_ns,composition_ns,output_bytes,base_outlines_ns,scan_conversion_ns,border_expansion_ns,span_union_ns,mask_generation_ns,mask_combination_ns,cache_hits,cache_misses,cache_evictions,cache_entries,cache_bytes,span_rows,expanded_candidates,expanded_candidates_without_reuse,band_rows,rgba_sha256,diagnostic_sha256,font_setup_ns,source_setup_ns,renderer_setup_ns"
    );
    let mut ingestion = TrackIngestor::new();
    for (name, text, count) in workloads {
        let source_setup_start = Instant::now();
        if std::env::var("TAIYOU_PERF_WORKLOADS").is_ok_and(|v| !v.split(',').any(|s| s == name)) {
            continue;
        }
        let mut source = "[Script Info]\nPlayResX: 1920\nPlayResY: 1080\nWrapStyle: 2\nScaledBorderAndShadow: yes\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow\nStyle: Default,Source Sans 3,48,2,2\n[Events]\nFormat: Layer,Start,End,Text\n".to_owned();
        for n in 0..count {
            source.push_str(&format!("Dialogue: {n},0:00:00.00,0:00:10.00,{text}\n"));
        }
        let parsed = taiyou_ass::parse_ass(
            source.as_bytes(),
            &ParseOptions::new(
                InputEncoding::Utf8,
                Profile::CoreV1,
                limits.clone(),
                SyntaxPolicy::Strict,
            ),
            &mut ingestion,
        )
        .unwrap();
        let source_setup_ns = source_setup_start.elapsed().as_nanos();
        for (width, height) in [(1920, 1080), (3840, 2160)] {
            let default_request = RenderRequest {
                now: TimeUs::from_micros(1_000_000),
                viewport: Viewport::full(Size::new(width, height)).unwrap(),
                unsupported: UnsupportedPolicy::Strict,
            };
            let standard = Renderer::new(RendererConfig::default()).unwrap().render(
                parsed.track().clone(),
                fonts.clone(),
                &default_request,
            );
            if let Err(error) = standard {
                writeln!(
                    failures,
                    "{name}\t{width}\tproduction-default-preflight\t{error:?}"
                )
                .unwrap();
            }
            for scenario in ["cold", "same-time", "warm-repeat", "sequence", "seek"] {
                if std::env::var("TAIYOU_PERF_SCENARIOS")
                    .is_ok_and(|v| !v.split(',').any(|s| s == scenario))
                {
                    continue;
                }
                let renderer_start = Instant::now();
                let mut renderer = Renderer::new(config.clone()).unwrap();
                let initial_renderer_setup_ns = renderer_start.elapsed().as_nanos();
                renderer.set_composition_band_rows(std::num::NonZeroUsize::new(band_rows).unwrap());
                renderer.set_timing_enabled(true);
                let mut request = RenderRequest {
                    now: TimeUs::from_micros(1_000_000),
                    viewport: Viewport::full(Size::new(width, height)).unwrap(),
                    unsupported: UnsupportedPolicy::Strict,
                };
                // Correctness is strict and outside the timed region.
                let mut uncached_config = config.clone();
                uncached_config.cache = CacheConfig::Disabled;
                uncached_config.workers = std::num::NonZeroUsize::MIN;
                let expected = Renderer::new(uncached_config).unwrap().render(
                    parsed.track().clone(),
                    fonts.clone(),
                    &request,
                );
                let expected = match expected {
                    Ok(frame) => frame,
                    Err(error) => {
                        writeln!(failures, "{name}\t{width}\t{scenario}\t{error:?}").unwrap();
                        failed = true;
                        continue;
                    }
                };
                let actual = renderer
                    .render(parsed.track().clone(), fonts.clone(), &request)
                    .unwrap();
                assert_eq!(
                    expected.pixels(),
                    actual.pixels(),
                    "profiling changed pixels"
                );
                assert_eq!(expected.diagnostics(), actual.diagnostics());
                drop((actual, expected));
                let scenario_start = Instant::now();
                for sample in 0..samples {
                    let mut renderer_setup_ns = if sample == 0 {
                        initial_renderer_setup_ns
                    } else {
                        0
                    };
                    // Bound hosted-runner smoke cost; publish the actual count.
                    // This timer affects sampling only, never render decisions.
                    if !fixed && sample >= 3 && scenario_start.elapsed().as_secs() >= 2 {
                        break;
                    }
                    if scenario == "cold" {
                        drop(renderer);
                        let renderer_start = Instant::now();
                        renderer = Renderer::new(config.clone()).unwrap();
                        renderer_setup_ns = renderer_start.elapsed().as_nanos();
                        renderer.set_composition_band_rows(
                            std::num::NonZeroUsize::new(band_rows).unwrap(),
                        );
                        renderer.set_timing_enabled(true);
                    }
                    let now = match scenario {
                        "sequence" => 1_000_000 + sample as i64 * 16_667,
                        "seek" => [7_500_000, 100_000, 4_000_000, 1_000_000][sample % 4],
                        "warm-repeat" => 2_000_000,
                        _ => 1_000_000,
                    };
                    request.now = TimeUs::from_micros(now);
                    let start = Instant::now();
                    let frame =
                        match renderer.render(parsed.track().clone(), fonts.clone(), &request) {
                            Ok(frame) => frame,
                            Err(error) => {
                                writeln!(
                                    failures,
                                    "{name}\t{width}\t{scenario}/sample-{sample}\t{error:?}"
                                )
                                .unwrap();
                                failed = true;
                                continue;
                            }
                        };
                    let total = start.elapsed().as_nanos();
                    black_box(frame.pixels());
                    // Verification intentionally happens after the render timer.
                    let rgba_sha256 = digest_hex(frame.pixels());
                    let diagnostic_sha256 =
                        digest_hex(format!("{:?}", frame.diagnostics()).as_bytes());
                    let t = renderer.last_timings();
                    let c = renderer.cache_stats();
                    println!(
                        "{name},{width},{height},{scenario},{sample},{workers},{cache_label},{total},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{band_rows},{rgba_sha256},{diagnostic_sha256},{font_setup_ns},{source_setup_ns},{renderer_setup_ns}",
                        t.evaluation.as_nanos(),
                        t.shaping.as_nanos(),
                        t.layout.as_nanos(),
                        t.collision.as_nanos(),
                        t.geometry.as_nanos(),
                        t.clip.as_nanos(),
                        t.masks.as_nanos(),
                        t.effects.as_nanos(),
                        t.paint.as_nanos(),
                        t.composition.as_nanos(),
                        frame.pixels().len(),
                        t.base_outlines.as_nanos(),
                        t.scan_conversion.as_nanos(),
                        t.border_expansion.as_nanos(),
                        t.span_union.as_nanos(),
                        t.mask_generation.as_nanos(),
                        t.mask_combination.as_nanos(),
                        c.hits,
                        c.misses,
                        c.evictions,
                        c.entries,
                        c.retained_bytes,
                        t.span_rows,
                        t.expanded_candidates,
                        t.expanded_candidates_without_reuse
                    );
                }
            }
        }
    }
    if failed {
        std::process::exit(2);
    }
}
#[cfg(not(feature = "native"))]
fn main() {
    eprintln!("performance requires --features native");
    std::process::exit(1);
}
