//! Workload experiment only. No production automatic line breaker is enabled.
#![cfg(feature = "native")]
use std::{path::PathBuf, time::Instant};
use taiyou_render::{renderer::*, shaping::shape_event};

#[test]
fn candidate_search_workloads_are_bounded_and_reported_on_actions() {
    if !cfg!(target_os="linux") || std::env::var("RUSTUP_TOOLCHAIN").unwrap_or_default() != "1.99.1" { return; }
    let Some(folder) = std::env::var_os("TAIYOU_RENDER_SAMPLE_DIR") else { return; };
    let mut ctx = FontContext::with_limits(Limits::default()).unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    for (name,ext) in [("latin","otf"),("arabic","ttf"),("indic","ttf"),("cjk","otf")] {
        let path = PathBuf::from(std::env::var("RUNNER_TEMP").unwrap()).join(format!("taiyou-shaping-fonts/{name}.{ext}"));
        builder.register(&mut ctx,&std::fs::read(path).unwrap(),0).unwrap();
    }
    let fonts = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let cases = [
        ("ordinary-latin", "ordinary subtitles need stable line breaking", " ", 2, "Source Sans 3"),
        ("ordinary-arabic", "مرحبا بالعالم هذا اختبار", " ", 4, "Noto Naskh Arabic"),
        ("ordinary-indic", "नमस्ते दुनिया परीक्षण", " ", 4, "Noto Sans Devanagari"),
        ("ordinary-cjk", "日本語字幕表示確認", "", 4, "Noto Sans CJK JP"),
        ("long-latin-typesetting", "ordinary typesetting words and line metrics", " ", 34, "Source Sans 3"),
        ("long-cjk-typesetting", "日本語字幕表示確認", "", 25, "Noto Sans CJK JP"),
    ];
    let mut report = String::new();
    for (name, text, delimiter, repeats, family) in cases {
        // These independently authored opportunities are complete word/ideograph
        // units selected for the experiment. They are NOT a UAX14 implementation
        // or evidence that the proposed production boundary algorithm exists.
        let units: Vec<String> = if delimiter.is_empty() { text.chars().map(|c| c.to_string()).collect() } else { text.split(delimiter).map(str::to_owned).collect() };
        let units: Vec<_> = (0..repeats).flat_map(|_| units.iter().cloned()).collect();
        let mut candidates = 0_u64;
        let mut glyphs = 0_u64;
        let mut peak_reservation = 0_u64;
        let start = Instant::now();
        let mut status = "complete";
        'search: for first in 0..units.len() {
            for last in first+1..=units.len() {
                if candidates == 10_000 { status="candidate-limit"; break 'search; }
                if start.elapsed().as_secs() >= 60 { status="experiment-time-budget"; break 'search; }
                let candidate = units[first..last].join(delimiter);
                let source = format!("[Script Info]\nPlayResX: 384\nPlayResY: 288\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow\nStyle: Default,{family},20,0,0\n[Events]\nFormat: Layer,Start,End,Style,Text\nDialogue: 0,0:00:00.00,0:00:02.00,Default,{candidate}");
                let parsed = taiyou_ass::parse_ass(source.as_bytes(),&ParseOptions::new(InputEncoding::Utf8,Profile::CoreV1,Limits::default(),SyntaxPolicy::Strict),&mut TrackIngestor::new()).unwrap();
                let evaluated = taiyou_render::evaluate(parsed.track(),EventId::new(0),TimeUs::from_micros(750_000),Profile::CoreV1).unwrap();
                let shaped = shape_event(&mut ctx,&fonts,&evaluated,&Default::default(),&Limits::default()).unwrap();
                let produced: usize = shaped.runs().iter().map(|r| r.glyphs().len()).sum();
                candidates += 1;
                glyphs += produced as u64;
                peak_reservation = peak_reservation.max(3 * (produced as u64 * std::mem::size_of::<ShapedGlyph>() as u64 + shaped.runs().len() as u64 * std::mem::size_of::<ShapedRun>() as u64) + shaped.source_map().text().len() as u64 * 256);
                if glyphs > 1_000_000 { status="glyph-work-limit"; break 'search; }
            }
        }
        report.push_str(&format!("{{\"case\":\"{name}\",\"units\":{},\"full_candidate_count\":{},\"evaluated_candidates\":{candidates},\"shaped_glyphs\":{glyphs},\"elapsed_ns\":{},\"peak_candidate_reservation_bytes\":{peak_reservation},\"status\":\"{status}\",\"boundary_source\":\"independent word/ideograph opportunity workload; not UAX14 conformance\"}}\n",units.len(), units.len()*(units.len()+1)/2,start.elapsed().as_nanos()));
        assert!(candidates <= 10_000);
        if name.starts_with("ordinary") { assert_eq!(status,"complete", "{name}"); }
    }
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(PathBuf::from(folder).join("T017-wrap-workload-limits.jsonl"),report).unwrap();
}
