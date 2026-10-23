#![cfg(feature = "native")]
use std::{path::PathBuf, sync::Arc};
use taiyou_font::*;
use taiyou_types::*;

fn limits() -> Limits {
    Limits::default()
        .with_transient_bytes(512 * 1024 * 1024)
        .unwrap()
}
fn fixture(id: &str) -> Vec<u8> {
    let suffix = if matches!(id, "arabic" | "indic") {
        ".ttf"
    } else {
        ".otf"
    };
    std::fs::read(
        PathBuf::from(std::env::var("RUNNER_TEMP").unwrap())
            .join("taiyou-shaping-fonts")
            .join(format!("{id}{suffix}")),
    )
    .unwrap()
}
fn catalog() -> (FontContext, Arc<ValidatedFontCatalog>, Vec<FontId>) {
    let mut ctx = FontContext::with_limits(limits()).unwrap();
    let mut builder = FontCatalogBuilder::new(limits());
    let mut ids = Vec::new();
    for id in ["latin", "arabic", "indic", "cjk"] {
        ids.push(builder.register(&mut ctx, &fixture(id), 0).unwrap());
    }
    let catalog = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    (ctx, catalog, ids)
}
fn style(family: &str) -> Style {
    Style::core_default()
        .with_font(FontRequest::new(Arc::from(family), 400, false, false))
        .with_size(FiniteF64::new(1000.0).unwrap())
        .unwrap()
}
fn evaluated(map: Arc<SourceMappedText>, spans: &[(u32, u32, Style)]) -> EvaluatedEvent {
    let source = SourceSpan::new(map.original_source(), 0, map.original_source().len()).unwrap();
    let runs: Vec<_> = spans
        .iter()
        .map(|(a, b, s)| {
            EvaluatedRun::new(Arc::from(&map.text()[*a as usize..*b as usize]), source, s)
                .with_decoded(map.decoded_span(*a..*b).unwrap())
                .unwrap()
        })
        .collect();
    EvaluatedEvent::new(
        EventId::new(0),
        EventOrder::new(0, 0),
        runs.into(),
        Placement::new(&spans[0].2, Margins::new(0, 0, 0), Anchor::Default),
        ChangeClass::default(),
    )
    .with_source_map(map)
    .unwrap()
}
fn plain(text: &str, style: &Style) -> EvaluatedEvent {
    let source: Arc<str> = Arc::from(text);
    let span = SourceSpan::new(&source, 0, source.len()).unwrap();
    let map = Arc::new(
        SourceMappedText::from_pieces(
            source,
            &[TextSourcePiece::Literal(span)],
            WrapMode::new(2).unwrap(),
            &limits(),
        )
        .unwrap(),
    );
    evaluated(map, &[(0, text.len() as u32, style.clone())])
}
#[test]
fn synthetic_italic_preserves_shaping_positions_and_updates_ink_support() {
    let (mut context, catalog, _) = catalog();
    let family = catalog.faces()[0].family();
    let regular = style(family);
    let italic = regular
        .clone()
        .with_font(regular.font().clone().with_italic(true));
    let a = shape_event(
        &mut context,
        &catalog,
        &plain("AB", &regular),
        &Default::default(),
        &limits(),
    )
    .unwrap();
    let b = shape_event(
        &mut context,
        &catalog,
        &plain("AB", &italic),
        &Default::default(),
        &limits(),
    )
    .unwrap();
    let a = &a.runs()[0];
    let b = &b.runs()[0];
    assert_eq!(a.glyphs().len(), b.glyphs().len());
    for (a, b) in a.glyphs().iter().zip(b.glyphs()) {
        assert_eq!(a.font(), b.font());
        assert_eq!(a.glyph(), b.glyph());
        assert_eq!(a.position(), b.position());
        let (GlyphInk::Bounds(a), GlyphInk::Bounds(b)) = (a.ink(), b.ink()) else {
            panic!("nonempty test glyph ink")
        };
        assert_eq!(a.y0(), b.y0());
        assert_eq!(a.y1(), b.y1());
        assert!(b.width() > a.width());
    }
}
struct Expected {
    upem: f64,
    metrics: [i32; 3],
    glyphs: Vec<[i64; 6]>,
}
fn reference(id: &str) -> Expected {
    let contents =
        std::fs::read_to_string(std::env::var("TAIYOU_SHAPING_REFERENCES").unwrap()).unwrap();
    let mut lines = contents.lines();
    while let Some(line) = lines.next() {
        let header: Vec<_> = line.split_whitespace().collect();
        if header.first() == Some(&"case") && header[1] == id {
            let mut glyphs = Vec::new();
            for line in lines.by_ref() {
                if line == "end" {
                    break;
                }
                let values: Vec<i64> = line
                    .split_whitespace()
                    .skip(1)
                    .map(|n| n.parse().unwrap())
                    .collect();
                glyphs.push(values.try_into().unwrap());
            }
            return Expected {
                upem: header[2].parse().unwrap(),
                metrics: [
                    header[3].parse().unwrap(),
                    header[4].parse().unwrap(),
                    header[5].parse().unwrap(),
                ],
                glyphs,
            };
        }
    }
    panic!("missing independently produced provisional reference {id}")
}
fn assert_reference(run: &ShapedRun, id: &str, size: f64) {
    let expected = reference(id);
    assert!(expected.metrics[0] > expected.metrics[1]);
    let (scale, ascent, descent, baseline_offset) = cell_reference(id, size);
    assert_eq!(f64::from(run.metrics().units_per_em()), expected.upem);
    assert!(run.face_metrics().iter().all(|m| {
        run.glyphs().iter().any(|g| g.font() == m.font) && m.metrics.units_per_em() != 0
    }));
    assert_eq!(run.glyphs().len(), expected.glyphs.len(), "{id}");
    let mut starts: Vec<_> = expected.glyphs.iter().map(|g| g[1] as u32).collect();
    starts.push(run.decoded().end());
    starts.sort_unstable();
    starts.dedup();
    for (glyph, raw) in run.glyphs().iter().zip(&expected.glyphs) {
        assert_eq!(glyph.glyph(), raw[0] as u32, "{id}");
        assert_eq!(glyph.cluster().start(), raw[1] as u32, "{id}");
        assert_eq!(
            glyph.cluster().end(),
            starts[starts.partition_point(|&p| p <= raw[1] as u32)],
            "{id}"
        );
        assert_eq!(
            [
                glyph.advance_x().get(),
                glyph.advance_y().get(),
                glyph.offset_x().get(),
                glyph.offset_y().get()
            ],
            [
                raw[2] as f64 * scale,
                -(raw[3] as f64) * scale,
                raw[4] as f64 * scale,
                -(raw[5] as f64) * scale + baseline_offset
            ],
            "{id}"
        );
    }
    assert_eq!(run.metrics().ascent().get(), ascent);
    assert_eq!(run.metrics().descent().get(), descent);
    assert_eq!(run.metrics().line_gap().get(), 0.0);
}
fn cell_reference(id: &str, size: f64) -> (f64, f64, f64, f64) {
    let path =
        PathBuf::from(std::env::var("RUNNER_TEMP").unwrap()).join("taiyou-ass-cell-inputs.tsv");
    let contents = std::fs::read_to_string(path).unwrap();
    let input = contents
        .lines()
        .find(|l| l.split_whitespace().next() == Some(format!("case:{id}").as_str()))
        .unwrap();
    let values: Vec<f64> = input
        .split_whitespace()
        .skip(1)
        .map(|s| s.parse().unwrap())
        .collect();
    let native_em =
        (((size * 64.0 + 0.5).floor() * values[0] / (values[1] + values[2])) + 0.5).floor();
    let scale = native_em / (64.0 * values[0]);
    let metric = |v: f64| (((v * native_em / values[0] + 0.5).floor() + 4.0) / 8.0).floor() / 8.0;
    let ascent = metric(values[1]);
    let baseline_offset = (values[1] * native_em / values[0] + 0.5).floor() / 64.0 - ascent;
    (scale, ascent, metric(values[2]), baseline_offset)
}
#[test]
fn shaping_exact_provisional_native_sequences_metrics_and_advances() {
    let (mut ctx, catalog, _) = catalog();
    for (id, text, family, language) in [
        ("fi", "fi", "Source Sans 3", "en"),
        ("office", "office AV", "Source Sans 3", "en"),
        ("combining", "a\u{301}", "Source Sans 3", "en"),
        ("arabic", "السَّلَامُ", "Noto Naskh Arabic", "ar"),
        ("indic", "किरण क्षि", "Noto Sans Devanagari", "hi"),
        ("cjk", "日本語", "Noto Sans CJK JP", "ja"),
    ] {
        let event = plain(text, &style(family));
        let options = ShapeOptions::default().with_language(language).unwrap();
        let result = shape_event(&mut ctx, &catalog, &event, &options, &limits()).unwrap();
        assert!(result.diagnostics().is_empty(), "{id}");
        assert_eq!(result.runs().len(), 1, "{id}");
        assert_reference(&result.runs()[0], id, 1000.0);
    }
}
#[test]
fn shaping_kerning_ligatures_spacing_and_design_unit_scaling() {
    let (mut ctx, catalog, _) = catalog();
    let s = style("Source Sans 3");
    let shaped = |ctx: &mut FontContext, text: &str, s: &Style| {
        shape_event(
            ctx,
            &catalog,
            &plain(text, s),
            &ShapeOptions::default().with_language("en").unwrap(),
            &limits(),
        )
        .unwrap()
    };
    let fi = shaped(&mut ctx, "ff", &s);
    let no = shaped(
        &mut ctx,
        "ff",
        &s.clone().with_spacing(FiniteF64::new(1.0).unwrap()),
    );
    assert_reference(&fi.runs()[0], "ff", 1000.0);
    assert_reference(&no.runs()[0], "ff-spacing", 1000.0);
    assert!(fi.runs()[0].glyphs().len() < no.runs()[0].glyphs().len());
    let av = shaped(&mut ctx, "AV", &s);
    let a = shaped(&mut ctx, "A", &s);
    let v = shaped(&mut ctx, "V", &s);
    assert_reference(&av.runs()[0], "kern", 1000.0);
    assert_reference(&a.runs()[0], "A", 1000.0);
    assert_reference(&v.runs()[0], "V", 1000.0);
    let width = |r: &ShapedEvent| {
        r.runs()
            .iter()
            .flat_map(|r| r.glyphs())
            .map(|g| g.advance_x().get())
            .sum::<f64>()
    };
    assert!(width(&av) < width(&a) + width(&v));
    assert_reference(
        &shaped(
            &mut ctx,
            "fi",
            &s.with_size(FiniteF64::new(20.0).unwrap()).unwrap(),
        )
        .runs()[0],
        "fi",
        20.0,
    );
}
#[test]
fn kerning_option_preserves_default_and_can_request_additive_advances() {
    let (mut ctx, catalog, _) = catalog();
    let style = style("Source Sans 3");
    let width = |ctx: &mut FontContext, text: &str, options: &ShapeOptions| {
        shape_event(ctx, &catalog, &plain(text, &style), options, &limits())
            .unwrap()
            .runs()
            .iter()
            .flat_map(|run| run.glyphs())
            .map(|glyph| glyph.advance_x().get())
            .sum::<f64>()
    };
    let default = ShapeOptions::default();
    let additive = default.clone().with_kerning(false);
    let explicit = default.clone().with_kerning(true);
    for (pair, left, right) in [("AV", "A", "V"), ("To", "T", "o")] {
        let sum = width(&mut ctx, left, &additive) + width(&mut ctx, right, &additive);
        assert!((width(&mut ctx, pair, &additive) - sum).abs() < 1e-10);
        assert!(width(&mut ctx, pair, &default) < sum);
        assert_eq!(
            width(&mut ctx, pair, &default),
            width(&mut ctx, pair, &explicit)
        );
    }
    // Disabling kerning must retain ordinary GSUB ligatures and source clusters.
    let result = shape_event(
        &mut ctx,
        &catalog,
        &plain("ff", &style),
        &additive,
        &limits(),
    )
    .unwrap();
    assert_eq!(result.runs()[0].glyphs().len(), 1);
    assert_eq!(result.runs()[0].glyphs()[0].cluster().text(), "ff");
}

#[test]
fn shaping_noncontiguous_ligature_and_combining_paint_provenance() {
    let (mut ctx, catalog, _) = catalog();
    for (source, right, id) in [
        ("f{\\c&HFF0000&}i", "i", "fi"),
        ("f{\\c&HFF0000&}f", "f", "ff"),
        ("a{\\c&HFF0000&}\u{301}", "\u{301}", "combining"),
    ] {
        let a = SourceSpan::new(source, 0, 1).unwrap();
        let b = SourceSpan::new(source, source.rfind(right).unwrap(), source.len()).unwrap();
        let map = Arc::new(
            SourceMappedText::from_pieces(
                Arc::from(source),
                &[TextSourcePiece::Literal(a), TextSourcePiece::Literal(b)],
                WrapMode::new(2).unwrap(),
                &limits(),
            )
            .unwrap(),
        );
        let first = style("Source Sans 3");
        let second = first.clone().with_colors(
            Rgba8::new(255, 0, 0, 255),
            first.secondary(),
            first.outline(),
            first.shadow_color(),
        );
        let event = evaluated(
            map.clone(),
            &[(0, 1, first.clone()), (1, map.text().len() as u32, second)],
        );
        let result = shape_event(
            &mut ctx,
            &catalog,
            &event,
            &ShapeOptions::default().with_language("en").unwrap(),
            &limits(),
        )
        .unwrap();
        assert_eq!(result.runs().len(), 1);
        assert_reference(&result.runs()[0], id, 1000.0);
        let glyph = &result.runs()[0].glyphs()[0];
        if id == "fi" {
            assert_eq!(glyph.cluster().original_ranges(&limits()).unwrap(), [a]);
            assert_eq!(
                result.runs()[0].glyphs()[1]
                    .cluster()
                    .original_ranges(&limits())
                    .unwrap(),
                [b]
            );
        } else {
            assert_eq!(glyph.cluster().original_ranges(&limits()).unwrap(), [a, b]);
        }
        assert_eq!(
            result.paint_for(glyph.cluster()).unwrap(),
            &PaintSet::from_style(&first)
        );
    }
}
#[test]
fn shaping_bidi_logical_order_and_arabic_fallback_with_context() {
    let (mut ctx, catalog, ids) = catalog();
    let event = plain("abc سلام", &style("Source Sans 3"));
    let result = shape_event(
        &mut ctx,
        &catalog,
        &event,
        &ShapeOptions::default(),
        &limits(),
    )
    .unwrap();
    assert_eq!(result.runs().len(), 2);
    assert_eq!(result.runs()[0].bidi_level(), 0);
    assert_eq!(result.runs()[1].bidi_level(), 1);
    assert_reference(&result.runs()[0], "mixed-latin", 1000.0);
    assert_reference(&result.runs()[1], "mixed-arabic", 1000.0);
    assert!(result.runs()[0].glyphs().iter().all(|g| g.font() == ids[0]));
    assert!(result.runs()[1].glyphs().iter().all(|g| g.font() == ids[1]));
    assert!(
        result.runs()[1]
            .glyphs()
            .windows(2)
            .all(|g| g[0].cluster().start() >= g[1].cluster().start())
    );
}
#[test]
fn shaping_arabic_style_boundary_retains_neighboring_joining_context() {
    let (mut ctx, catalog, _) = catalog();
    let first = style("Noto Naskh Arabic");
    let second = first
        .clone()
        .with_size(FiniteF64::new(20.0).unwrap())
        .unwrap();
    let map = plain("سلام", &first).source_map().unwrap().clone();
    let event = evaluated(map, &[(0, 2, first), (2, 8, second)]);
    let result = shape_event(
        &mut ctx,
        &catalog,
        &event,
        &ShapeOptions::default().with_language("ar").unwrap(),
        &limits(),
    )
    .unwrap();
    assert_eq!(result.runs().len(), 2);
    let expected = reference("joining");
    for (run, size) in result.runs().iter().zip([1000.0, 20.0]) {
        for glyph in run.glyphs() {
            let raw = expected
                .glyphs
                .iter()
                .find(|g| g[1] as u32 == glyph.cluster().start())
                .unwrap();
            assert_eq!(glyph.glyph(), raw[0] as u32);
            assert_eq!(
                glyph.advance_x().get(),
                raw[2] as f64 * cell_reference("joining", size).0
            );
        }
    }
    assert!(result.missing().is_empty());
}
#[test]
fn shaping_cross_script_marks_are_not_separated_before_fallback() {
    let (mut ctx, catalog, _) = catalog();
    for text in ["A\u{94d}", "A\u{93e}", "A\u{488}"] {
        let result = shape_event(
            &mut ctx,
            &catalog,
            &plain(text, &style("Source Sans 3")),
            &ShapeOptions::default(),
            &limits(),
        )
        .unwrap();
        assert_eq!(result.runs().len(), 1);
        let glyphs = result.runs()[0].glyphs();
        assert!(!glyphs.is_empty());
        assert!(glyphs.iter().all(|g| {
            g.cluster().start() == 0
                && g.cluster().end() as usize == text.len()
                && g.font() == glyphs[0].font()
        }));
    }
}
#[test]
fn shaping_whole_cluster_fallback_and_explicit_notdef() {
    let (mut ctx, catalog, ids) = catalog();
    let result = shape_event(
        &mut ctx,
        &catalog,
        &plain("क्षि", &style("Source Sans 3")),
        &ShapeOptions::default(),
        &limits(),
    )
    .unwrap();
    assert!(
        result
            .runs()
            .iter()
            .flat_map(|r| r.glyphs())
            .all(|g| g.font() == ids[2] && g.glyph() != 0)
    );
    assert!(result.missing().is_empty());
    let absent = shape_event(
        &mut ctx,
        &catalog,
        &plain("\u{10ffff}", &style("Source Sans 3")),
        &ShapeOptions::default(),
        &limits(),
    )
    .unwrap();
    assert!(
        absent
            .runs()
            .iter()
            .flat_map(|r| r.glyphs())
            .any(|g| g.glyph() == 0)
    );
    assert!(!absent.missing().is_empty());
    assert!(
        absent
            .diagnostics()
            .iter()
            .any(|d| d.code() == DiagnosticCode::MissingGlyph)
    );
    assert_eq!(
        absent.missing()[0]
            .decoded
            .original_ranges(&limits())
            .unwrap()[0]
            .slice(absent.source_map().original_source()),
        Some("\u{10ffff}")
    );
}
#[test]
fn shaping_escapes_hard_breaks_and_owned_context_lifetime() {
    let result = {
        let (mut ctx, catalog, _) = catalog();
        let source = "\u{feff}header\r\nf\\Nfi\\nA\\hV";
        let span = |s: &str| {
            let i = source.find(s).unwrap();
            SourceSpan::new(source, i, i + s.len()).unwrap()
        };
        let pieces = [
            TextSourcePiece::Literal(span("f")),
            TextSourcePiece::HardBreak(span("\\N")),
            TextSourcePiece::Literal(span("fi")),
            TextSourcePiece::SoftBreak(span("\\n")),
            TextSourcePiece::Literal(span("A")),
            TextSourcePiece::NoBreakSpace(span("\\h")),
            TextSourcePiece::Literal(span("V")),
        ];
        let map = Arc::new(
            SourceMappedText::from_pieces(
                Arc::from(source),
                &pieces,
                WrapMode::new(2).unwrap(),
                &limits(),
            )
            .unwrap(),
        );
        let event = evaluated(
            map.clone(),
            &[(0, map.text().len() as u32, style("Source Sans 3"))],
        );
        shape_event(
            &mut ctx,
            &catalog,
            &event,
            &ShapeOptions::default(),
            &limits(),
        )
        .unwrap()
    };
    assert_eq!(result.breaks().len(), 2);
    assert_eq!(
        result.breaks()[0].original_ranges(&limits()).unwrap()[0]
            .slice(result.source_map().original_source()),
        Some("\\N")
    );
    assert_eq!(
        result.breaks()[1].original_ranges(&limits()).unwrap()[0]
            .slice(result.source_map().original_source()),
        Some("\\n")
    );
    assert!(result.runs().iter().flat_map(|r| r.glyphs()).any(|g| {
        g.cluster()
            .original_ranges(&limits())
            .unwrap()
            .iter()
            .any(|s| s.slice(result.source_map().original_source()) == Some("\\h"))
    }));
}
#[test]
fn shaping_limits_unmappable_inputs_repeatability_and_foreign_maps() {
    let (mut ctx, catalog, _) = catalog();
    let event = plain("office AV", &style("Source Sans 3"));
    let options = ShapeOptions::default();
    let a = shape_event(&mut ctx, &catalog, &event, &options, &limits()).unwrap();
    let b = shape_event(&mut ctx, &catalog, &event, &options, &limits()).unwrap();
    assert_eq!(a, b);
    assert_eq!(a.order(), event.order());
    assert_eq!(a.placement(), event.placement());
    assert_eq!(a.clip(), event.clip());
    assert_eq!(a.evaluated(), &event);
    let zero = FiniteF64::new(0.0).unwrap();
    let negative = FiniteF64::new(-1.0).unwrap();
    assert!(RunMetrics::new(zero, zero, zero, 0).is_err());
    assert!(RunMetrics::new(negative, zero, zero, 1000).is_err());
    assert!(RunMetrics::new(zero, negative, zero, 1000).is_err());
    assert!(RunMetrics::new(zero, zero, negative, 1000).is_ok());
    let mut wrong = a.runs()[0].face_metrics()[0].clone();
    wrong.glyphs = 1..usize::MAX;
    assert!(
        a.runs()[0]
            .clone()
            .with_face_metrics(Arc::from([wrong]))
            .is_err()
    );
    for bound in [
        limits().with_transient_bytes(1).unwrap(),
        limits().with_text_bytes_per_event(1).unwrap(),
        limits().with_glyphs_per_event(1).unwrap(),
    ] {
        assert_eq!(
            shape_event(&mut ctx, &catalog, &event, &options, &bound)
                .unwrap_err()
                .code(),
            ErrorCode::LimitExceeded
        );
    }
    let foreign = plain("office AV", &style("Source Sans 3"));
    assert!(a.paint_for(foreign.runs()[0].decoded().unwrap()).is_err());
    let span = foreign.runs()[0].decoded().unwrap().clone();
    let glyph = ShapedGlyph::new(
        catalog.faces()[0].blob().id(),
        1,
        span,
        GlyphPosition::new(
            FiniteF64::new(0.0).unwrap(),
            FiniteF64::new(0.0).unwrap(),
            FiniteF64::new(0.0).unwrap(),
            FiniteF64::new(0.0).unwrap(),
        ),
    );
    assert!(
        ShapedRun::new(
            Arc::from([glyph]),
            a.runs()[0].metrics(),
            event.runs()[0].decoded().unwrap().clone(),
            0,
            u32::from_be_bytes(*b"Latn"),
            Arc::from("und")
        )
        .is_err()
    );
    assert!(ShapeOptions::default().with_language("en\0bad").is_err());
    let unmapped = EvaluatedEvent::new(
        event.id(),
        event.order(),
        Arc::from(event.runs()),
        event.placement().clone(),
        event.animation(),
    );
    assert_eq!(
        shape_event(&mut ctx, &catalog, &unmapped, &options, &limits())
            .unwrap_err()
            .code(),
        ErrorCode::InvalidArgument
    );
}
#[test]
fn shaping_missing_diagnostics_are_bounded_and_available_without_event_coordinator() {
    let mut ctx = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(limits());
    builder.register(&mut ctx, &fixture("latin"), 0).unwrap();
    let catalog = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let s = style("Source Sans 3");
    let text = "\u{10ffff}".repeat(MAX_DIAGNOSTICS + 3);
    let result = shape_event(
        &mut ctx,
        &catalog,
        &plain(&text, &s),
        &ShapeOptions::default(),
        &limits(),
    )
    .unwrap();
    assert_eq!(result.missing().len(), MAX_DIAGNOSTICS);
    assert_eq!(result.diagnostics().len(), MAX_DIAGNOSTICS + 1);
    assert_eq!(
        result.diagnostics().last().unwrap().code(),
        DiagnosticCode::DiagnosticsSuppressed
    );
    let input = resolve_runs(&catalog, "\u{10ffff}", &ShapeStyle::from_style(&s)).unwrap();
    let run = shape(&mut ctx, &input[0]).unwrap();
    assert_eq!(run.glyphs()[0].glyph(), 0);
    assert_eq!(run.diagnostics()[0].code(), DiagnosticCode::MissingGlyph);
}
#[test]
fn shaping_collection_face_identity_uses_requested_weight() {
    let mut ctx = FontContext::with_limits(limits()).unwrap();
    let mut builder = FontCatalogBuilder::new(limits());
    let bytes = std::fs::read(std::env::var("TAIYOU_TEST_COLLECTION").unwrap()).unwrap();
    let a = builder.register(&mut ctx, &bytes, 0).unwrap();
    let b = builder.register(&mut ctx, &bytes, 1).unwrap();
    let catalog = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let s = style("Source Sans 3").with_font(FontRequest::new(
        Arc::from("Source Sans 3"),
        700,
        false,
        false,
    ));
    let result = shape_event(
        &mut ctx,
        &catalog,
        &plain("fi", &s),
        &ShapeOptions::default().with_language("en").unwrap(),
        &limits(),
    )
    .unwrap();
    assert_ne!(a, b);
    assert!(result.runs()[0].glyphs().iter().all(|g| g.font() == b));
    assert_reference(&result.runs()[0], "collection", 1000.0);
}
