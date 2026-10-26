//! Isolated Actions-only common-input performance adapter. No renderer changes.
#[cfg(feature = "native")]
fn main() {
    use sha2::{Digest, Sha256};
    use std::{num::NonZeroUsize, time::Instant};
    use taiyou_render::renderer::*;
    assert_eq!(std::env::var("GITHUB_ACTIONS").as_deref(), Ok("true"));
    let args: Vec<_> = std::env::args().collect();
    assert!(args.len() >= 8);
    let width: u32 = args[2].parse().unwrap();
    let height: u32 = args[3].parse().unwrap();
    let workers: usize = args[4].parse().unwrap();
    let times: Vec<i64> = std::fs::read_to_string(&args[5])
        .unwrap()
        .lines()
        .map(|line| line.parse().unwrap())
        .collect();
    let cold = args[6] == "cold";
    let limits = Limits::default()
        .with_transient_bytes(512 * 1024 * 1024)
        .unwrap()
        .with_path_segments_per_frame(32_000_000)
        .unwrap();
    let start = Instant::now();
    let mut context = FontContext::with_limits(limits.clone()).unwrap();
    let mut builder = FontCatalogBuilder::new(limits.clone());
    for font in &args[7..] {
        builder
            .register(&mut context, &std::fs::read(font).unwrap(), 0)
            .unwrap();
    }
    let fonts = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    drop(context);
    let font_setup_ns = start.elapsed().as_nanos();
    let start = Instant::now();
    let parsed = taiyou_ass::parse_ass(
        &std::fs::read(&args[1]).unwrap(),
        &ParseOptions::default(),
        &mut TrackIngestor::new(),
    )
    .unwrap();
    let source_setup_ns = start.elapsed().as_nanos();
    let config = RendererConfig {
        workers: NonZeroUsize::new(workers).unwrap(),
        limits,
        ..Default::default()
    };
    let mut renderer: Option<Renderer> = None;
    let mut initial_setup = 0;
    let request = |milliseconds: i64| RenderRequest {
        now: TimeUs::from_micros(milliseconds.checked_mul(1000).unwrap()),
        viewport: Viewport::full(Size::new(width, height)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    };
    if !cold {
        let start = Instant::now();
        let mut warm = Renderer::new(config.clone()).unwrap();
        initial_setup = start.elapsed().as_nanos();
        drop(
            warm.render_parsed(&parsed, fonts.clone(), &request(times[0]))
                .unwrap(),
        );
        renderer = Some(warm);
    }
    let hex = |bytes: &[u8]| {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    println!(
        "sample,timestamp_ms,total_ns,renderer_setup_ns,font_setup_ns,source_setup_ns,rgba_digest,diagnostic_digest,nonzero_alpha,cache_hits,cache_misses,cache_evictions,cache_bytes"
    );
    for (sample, &milliseconds) in times.iter().enumerate() {
        let setup = if cold {
            drop(renderer.take());
            let start = Instant::now();
            renderer = Some(Renderer::new(config.clone()).unwrap());
            start.elapsed().as_nanos()
        } else if sample == 0 {
            initial_setup
        } else {
            0
        };
        let renderer = renderer.as_mut().unwrap();
        // Production timing disabled: no per-stage probes in primary samples.
        let start = Instant::now();
        let frame = renderer
            .render_parsed(&parsed, fonts.clone(), &request(milliseconds))
            .unwrap();
        let total = start.elapsed().as_nanos();
        let rgba = hex(frame.pixels());
        let diagnostics = hex(format!("{:?}", frame.diagnostics()).as_bytes());
        let nonzero = frame.pixels().chunks_exact(4).filter(|p| p[3] != 0).count();
        let stats = renderer.cache_stats();
        println!(
            "{sample},{milliseconds},{total},{setup},{font_setup_ns},{source_setup_ns},{rgba},{diagnostics},{nonzero},{},{},{},{}",
            stats.hits, stats.misses, stats.evictions, stats.retained_bytes
        );
    }
    // Supplemental observations after ALL primary samples. These never enter
    // primary distributions, warmup state or digest comparisons above.
    if std::env::var("SERIAL_PROBE").as_deref() == Ok("1") {
        let renderer = renderer.as_mut().unwrap();
        renderer.set_timing_enabled(true);
        let observation_time = times.last().unwrap() + if args[6] == "sequence" { 16 } else { 0 };
        let frame = renderer.render_parsed(&parsed, fonts.clone(), &request(observation_time)).unwrap();
        eprintln!("POST_SAMPLE_STAGES={:?}", renderer.last_timings());
        let limits = renderer.config().limits.clone();
        let mut budget = OutputBudget::new(&limits);
        for sample in 0..31 {
            // Copy source bytes and reserve BEFORE timing. Measure the public
            // publication checks/conversion alone, retaining their full work.
            let pixels = frame.pixels().to_vec();
            let reservation = budget.reserve(pixels.len() as u64).unwrap();
            let start = Instant::now();
            let published = Frame::from_owned_pixels(frame.size(), pixels, &limits, reservation).unwrap();
            let elapsed = start.elapsed().as_nanos();
            assert_eq!(published.pixels(), frame.pixels());
            eprintln!("POST_SAMPLE_PUBLICATION_NS={sample},{elapsed}");
        }
    }
}
#[cfg(not(feature = "native"))]
fn main() {
    panic!("performance_compare requires native");
}
