//! Independently authored W2 experiment. Does not implement automatic wrapping.
#![cfg(feature = "native")]
use std::{io::Write, path::PathBuf, sync::Arc, time::Instant};
use taiyou_render::{
    renderer::*,
    shaping::{shape_event, shape_event_advances_experiment},
};

fn event(
    source: Arc<str>,
    range: std::ops::Range<usize>,
    styles: &[Style],
    limits: &Limits,
) -> EvaluatedEvent {
    let original = SourceSpan::new(&source, range.start, range.end).unwrap();
    let map = Arc::new(
        SourceMappedText::from_pieces(
            source,
            &[TextSourcePiece::Literal(original)],
            WrapMode::new(2).unwrap(),
            limits,
        )
        .unwrap(),
    );
    let middle = map
        .text()
        .char_indices()
        .nth(map.text().chars().count() / 2)
        .map_or(0, |(at, _)| at) as u32;
    let ranges = if styles.len() == 1 {
        vec![(0, map.text().len() as u32, &styles[0])]
    } else {
        vec![
            (0, middle, &styles[0]),
            (middle, map.text().len() as u32, &styles[1]),
        ]
    };
    let runs: Vec<_> = ranges
        .into_iter()
        .filter(|(a, b, _)| a < b)
        .map(|(a, b, style)| {
            EvaluatedRun::new(
                Arc::from(&map.text()[a as usize..b as usize]),
                original,
                style,
            )
            .with_decoded(map.decoded_span(a..b).unwrap())
            .unwrap()
        })
        .collect();
    EvaluatedEvent::new(
        EventId::new(0),
        EventOrder::new(0, 0),
        runs.into(),
        Placement::new(&styles[0], Margins::new(0, 0, 0), Anchor::Default),
        ChangeClass::default(),
    )
    .with_terminal_style(styles[0].clone())
    .with_source_map(map)
    .unwrap()
}

// Logical extent only. No PositionedGlyph, ink bounds or LocalLayout allocation.
// Independent full layout below verifies the arithmetic and visual-run ordering.
fn width(input: &ShapedEvent) -> f64 {
    assert!(input.breaks().is_empty());
    let mut order: Vec<_> = (0..input.runs().len()).collect();
    let max = order
        .iter()
        .map(|&i| input.runs()[i].bidi_level())
        .max()
        .unwrap_or(0);
    if let Some(min) = order
        .iter()
        .map(|&i| input.runs()[i].bidi_level())
        .filter(|l| l % 2 == 1)
        .min()
    {
        for level in (min..=max).rev() {
            let mut i = 0;
            while i < order.len() {
                if input.runs()[order[i]].bidi_level() < level {
                    i += 1;
                    continue;
                }
                let start = i;
                while i < order.len() && input.runs()[order[i]].bidi_level() >= level {
                    i += 1;
                }
                order[start..i].reverse();
            }
        }
    }
    let (mut pen, mut min, mut max) = (0.0_f64, 0.0_f64, 0.0_f64);
    for i in order {
        let run = &input.runs()[i];
        for (i, glyph) in run.glyphs().iter().enumerate() {
            let at = glyph.cluster().start();
            let state = input
                .evaluated()
                .runs()
                .iter()
                .find(|r| r.decoded().is_some_and(|r| r.start() <= at && at < r.end()))
                .unwrap();
            let sx = state.geometry().scale_x().get();
            pen += glyph.advance_x().get() * sx;
            if run
                .glyphs()
                .get(i + 1)
                .is_none_or(|g| g.cluster() != glyph.cluster())
            {
                pen += state.shaping().spacing().get() * sx;
            }
            assert!(pen.is_finite());
            min = min.min(pen);
            max = max.max(pen);
        }
    }
    max - min
}
fn same_advances(a: &ShapedEvent, b: &ShapedEvent, limits: &Limits) {
    assert_eq!(a.runs().len(), b.runs().len());
    assert_eq!(a.diagnostics(), b.diagnostics());
    for (a, b) in a.runs().iter().zip(b.runs()) {
        assert_eq!(a.bidi_level(), b.bidi_level());
        assert_eq!(a.metrics(), b.metrics());
        assert_eq!(a.face_metrics(), b.face_metrics());
        assert_eq!(a.glyphs().len(), b.glyphs().len());
        for (a, b) in a.glyphs().iter().zip(b.glyphs()) {
            assert_eq!(
                (
                    a.font(),
                    a.glyph(),
                    a.advance_x(),
                    a.advance_y(),
                    a.offset_x(),
                    a.offset_y()
                ),
                (
                    b.font(),
                    b.glyph(),
                    b.advance_x(),
                    b.advance_y(),
                    b.offset_x(),
                    b.offset_y()
                )
            );
            assert_eq!(a.cluster(), b.cluster());
            assert_eq!(
                a.cluster().original_ranges(limits).unwrap(),
                b.cluster().original_ranges(limits).unwrap()
            );
        }
    }
    let local = taiyou_render::layout(
        a,
        &LayoutConstraints::new(a.placement().alignment()).with_limits(limits.clone()),
    )
    .unwrap();
    assert_eq!(local.logical_bounds().width(), width(b));
}

#[test]
fn boundary_widths_match_independent_full_line_shaping_and_report_costs() {
    let limits = Limits::default()
        .with_transient_bytes(512 * 1024 * 1024)
        .unwrap();
    let mut ctx = FontContext::with_limits(limits.clone()).unwrap();
    let mut builder = FontCatalogBuilder::new(limits.clone());
    for (id, ext) in [
        ("latin", "otf"),
        ("arabic", "ttf"),
        ("indic", "ttf"),
        ("cjk", "otf"),
    ] {
        builder
            .register(
                &mut ctx,
                &std::fs::read(
                    PathBuf::from(std::env::var_os("RUNNER_TEMP").unwrap())
                        .join(format!("taiyou-shaping-fonts/{id}.{ext}")),
                )
                .unwrap(),
                0,
            )
            .unwrap();
    }
    let fonts = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let style = |family: &str| {
        Style::core_default()
            .with_font(FontRequest::new(Arc::from(family), 400, false, false))
            .with_size(FiniteF64::new(20.0).unwrap())
            .unwrap()
    };
    for (text, family) in [
        ("office fi a\u{301} 👩\u{200d}👧", "Source Sans 3"),
        ("مرحبا بالعالم abc 123", "Noto Naskh Arabic"),
        ("नमस्ते दुनिया परीक्षण", "Noto Sans Devanagari"),
        ("日本語 字幕（確認）", "Noto Sans CJK JP"),
        ("  abc\u{a0}אבג مرحبا  ", "Source Sans 3"),
    ] {
        let source: Arc<str> = Arc::from(format!("prefix\n{text}\nsuffix"));
        for spacing in [0.0, 0.125, -30.25] {
            let styles = [
                style(family).with_spacing(FiniteF64::new(spacing).unwrap()),
                style("Source Sans 3").with_spacing(FiniteF64::new(spacing / 2.0).unwrap()),
            ];
            let input = event(source.clone(), 7..7 + text.len(), &styles, &limits);
            let full = shape_event(&mut ctx, &fonts, &input, &Default::default(), &limits).unwrap();
            let advances = shape_event_advances_experiment(
                &mut ctx,
                &fonts,
                &input,
                &Default::default(),
                &limits,
            )
            .unwrap();
            same_advances(&full, &advances, &limits);
        }
    }
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
    let cases = [
        (
            "latin",
            "ordinary subtitles need stable line breaking",
            " ",
            2,
            "Source Sans 3",
        ),
        (
            "arabic",
            "مرحبا بالعالم هذا اختبار",
            " ",
            4,
            "Noto Naskh Arabic",
        ),
        ("indic", "नमस्ते दुनिया परीक्षण", " ", 4, "Noto Sans Devanagari"),
        ("cjk", "日本語字幕表示確認", "", 4, "Noto Sans CJK JP"),
        (
            "long-latin",
            "ordinary typesetting words and line metrics",
            " ",
            34,
            "Source Sans 3",
        ),
        ("long-cjk", "日本語字幕表示確認", "", 25, "Noto Sans CJK JP"),
    ];
    let mut report = String::new();
    for (name, text, delimiter, repeats, family) in cases {
        let units: Vec<String> = if delimiter.is_empty() {
            text.chars().map(|c| c.to_string()).collect()
        } else {
            text.split(delimiter).map(str::to_owned).collect()
        };
        let units: Vec<_> = (0..repeats).flat_map(|_| units.iter().cloned()).collect();
        let text = units.join(delimiter);
        let source: Arc<str> = Arc::from(format!("prefix\n{text}\nsuffix"));
        let mut starts = vec![7];
        let mut ends = Vec::new();
        let mut at = 7;
        for unit in &units {
            at += unit.len();
            ends.push(at);
            at += delimiter.len();
            starts.push(at);
        }
        let start = Instant::now();
        let mut preparation_ns = 0;
        let mut shaping_ns = 0;
        let mut extent_ns = 0;
        let mut verified = 0;
        let mut glyphs = 0_u64;
        let mut candidates = 0_u64;
        let mut status = "complete";
        let mut costs = vec![vec![f64::NAN; units.len() + 1]; units.len()];
        'outer: for first in 0..units.len() {
            for last in first + 1..=units.len() {
                if start.elapsed().as_secs() >= 60 {
                    status = "experiment-time-budget";
                    break 'outer;
                }
                if candidates >= 32_000 || glyphs >= 16_000_000 {
                    status = "experimental-work-limit";
                    break 'outer;
                }
                let t = Instant::now();
                let input = event(
                    source.clone(),
                    starts[first]..ends[last - 1],
                    &[style(family)],
                    &limits,
                );
                preparation_ns += t.elapsed().as_nanos();
                let t = Instant::now();
                let shaped = shape_event_advances_experiment(
                    &mut ctx,
                    &fonts,
                    &input,
                    &Default::default(),
                    &limits,
                )
                .unwrap();
                shaping_ns += t.elapsed().as_nanos();
                let t = Instant::now();
                costs[first][last] = width(&shaped);
                extent_ns += t.elapsed().as_nanos();
                candidates += 1;
                glyphs += shaped
                    .runs()
                    .iter()
                    .map(|r| r.glyphs().len() as u64)
                    .sum::<u64>();
                if units.len() <= 36
                    || (first == 0 && last % 16 == 0)
                    || last == first + 1 && first % 16 == 0
                {
                    let full = shape_event(&mut ctx, &fonts, &input, &Default::default(), &limits)
                        .unwrap();
                    same_advances(&full, &shaped, &limits);
                    verified += 1;
                }
            }
        }
        let record = format!(
            "{{\"case\":\"{name}\",\"units\":{},\"all_candidates\":{},\"candidates\":{candidates},\"glyphs\":{glyphs},\"prepare_ns\":{preparation_ns},\"shape_ns\":{shaping_ns},\"logical_extent_ns\":{extent_ns},\"elapsed_ns\":{},\"full_line_verifications\":{verified},\"status\":\"{status}\",\"candidate_storage_bytes\":{},\"ass_parses_per_candidate\":0,\"evaluations_per_candidate\":0}}\n",
            units.len(),
            units.len() * (units.len() + 1) / 2,
            start.elapsed().as_nanos(),
            units.len() * (units.len() + 1) * 8
        );
        std::io::stdout()
            .write_all(format!("T017_WIDTH={record}").as_bytes())
            .unwrap();
        report.push_str(&record);
    }
    std::fs::write(
        PathBuf::from(folder).join("T017-width-prepass.jsonl"),
        report,
    )
    .unwrap();
}
