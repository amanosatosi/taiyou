//! Additive bounded warm-render/seek observations. No historical rerun or oracle.
#![cfg(feature = "native")]
use std::{io::Write, path::PathBuf, process::Stdio, time::Instant};
use taiyou_render::renderer::*;

fn sha256(bytes: &[u8]) -> String {
    let mut child = std::process::Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let hash = String::from_utf8(output.stdout).unwrap();
    let hash = hash.split_whitespace().next().unwrap().to_owned();
    assert!(hash.len() == 64 && hash.bytes().all(|c| c.is_ascii_hexdigit()));
    hash
}
fn process_cpu_ticks() -> (u64, u64) {
    let source = std::fs::read_to_string("/proc/self/stat").unwrap();
    let fields: Vec<_> = source[source.rfind(')').unwrap() + 1..]
        .split_whitespace()
        .collect();
    (fields[11].parse().unwrap(), fields[12].parse().unwrap())
}
#[test]
fn warm_forward_backward_and_random_requests_match_fresh_frames() {
    if !cfg!(target_os = "linux")
        || !std::env::var("RUSTUP_TOOLCHAIN")
            .unwrap_or_default()
            .starts_with("1.99.1")
    {
        return;
    }
    let Some(folder) = std::env::var_os("TAIYOU_RENDER_SAMPLE_DIR") else {
        return;
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut ctx = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    builder
        .register(
            &mut ctx,
            &std::fs::read(
                PathBuf::from(std::env::var_os("RUNNER_TEMP").unwrap())
                    .join("taiyou-shaping-fonts/latin.otf"),
            )
            .unwrap(),
            0,
        )
        .unwrap();
    let fonts = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let mut report = String::new();
    for name in [
        "normal-one",
        "normal-eight",
        "typesetting-100-border-0.5",
        "typesetting-hd-100",
    ] {
        let source = std::fs::read_to_string(root.join(format!(
            "experiments/t017/results/dc83cab/T017-optimized/{name}.ass"
        )))
        .unwrap()
        .replacen("0:00:02.00", "0:00:01.00", 1);
        let source_hash = sha256(source.as_bytes());
        std::fs::write(
            PathBuf::from(&folder).join(format!("T017-warm-{name}.ass")),
            &source,
        )
        .unwrap();
        let track = taiyou_ass::parse_ass(
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
        .clone();
        let mut renderer = Renderer::new(Default::default()).unwrap();
        for (sample, now) in [
            750_000, 1_250_000, 500_000, 0, 1_500_000, 750_000, 1_000_000, 500_000,
        ]
        .into_iter()
        .enumerate()
        {
            let request = RenderRequest {
                now: TimeUs::from_micros(now),
                viewport: Viewport::full(if name == "typesetting-hd-100" {
                    Size::new(1920, 1080)
                } else {
                    Size::new(160, 100)
                })
                .unwrap(),
                unsupported: UnsupportedPolicy::Strict,
            };
            let cpu_before = process_cpu_ticks();
            let start = Instant::now();
            let frame = renderer
                .render(track.clone(), fonts.clone(), &request)
                .unwrap();
            let render_ns = start.elapsed().as_nanos();
            let cpu_after = process_cpu_ticks();
            let start = Instant::now();
            let fresh = Renderer::new(Default::default())
                .unwrap()
                .render(track.clone(), fonts.clone(), &request)
                .unwrap();
            let fresh_ns = start.elapsed().as_nanos();
            assert_eq!(frame.pixels(), fresh.pixels());
            assert_eq!(frame.diagnostics(), fresh.diagnostics());
            let raw_hash = sha256(frame.pixels());
            report.push_str(&format!(
                "{{\"case\":\"{name}\",\"sample\":{sample},\"now_us\":{now},\"warm_render_ns\":{render_ns},\"fresh_render_ns\":{fresh_ns},\"user_ticks\":{},\"system_ticks\":{},\"rgba_sha256\":\"{raw_hash}\",\"source_sha256\":\"{source_hash}\",\"diagnostics\":{},\"byte_equivalence\":true}}\n",
                cpu_after.0 - cpu_before.0,
                cpu_after.1 - cpu_before.1,
                frame.diagnostics().len()
            ));
        }
    }
    std::fs::write(PathBuf::from(&folder).join("T017-warm-seek.jsonl"), report).unwrap();
    std::fs::write(
        PathBuf::from(&folder).join("T017-warm-process.txt"),
        format!(
            "debug build; registered catalog/parsed track reused; uncached public Renderer; first sample includes initial render; fresh Renderer measured separately; CPU ticks from caller process, not child sha256sum; HWM is process-wide, not per-case\nfont={:?}\nCLK_TCK={}\n{}\n{}",
            fonts.faces()[0].blob().id(),
            String::from_utf8(
                std::process::Command::new("getconf")
                    .arg("CLK_TCK")
                    .output()
                    .unwrap()
                    .stdout
            )
            .unwrap(),
            std::fs::read_to_string("/proc/cpuinfo").unwrap(),
            std::fs::read_to_string("/proc/self/status").unwrap()
        ),
    )
    .unwrap();
}
