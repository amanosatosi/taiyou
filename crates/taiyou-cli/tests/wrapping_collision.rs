#![cfg(feature = "native")]

use std::{path::PathBuf, sync::Arc};
use taiyou_render::{layout, renderer::*, shaping::shape_event};

fn catalog() -> Arc<ValidatedFontCatalog> {
    let limits = Limits::default();
    let mut ctx = FontContext::with_limits(limits.clone()).unwrap();
    let mut builder = FontCatalogBuilder::new(limits);
    let file = PathBuf::from(std::env::var("RUNNER_TEMP").unwrap())
        .join("taiyou-shaping-fonts/latin.otf");
    builder.register(&mut ctx, &std::fs::read(file).unwrap(), 0).unwrap();
    builder.snapshot(&mut CatalogIngestor::new()).unwrap()
}
fn source(alignment: u8, events: &str) -> String {
    format!("[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,PrimaryColour,BackColour,Outline,Shadow,Alignment,MarginL,MarginR,MarginV\nStyle: Default,Source Sans 3,20,&H00FFFFFF,&HFF000000,0,0,{alignment},13,21,9\n[Events]\nFormat: Layer,Start,End,Style,Text\n{events}")
}
fn row(layer: i32, start: &str, end: &str, text: &str) -> String {
    format!("Dialogue: {layer},{start},{end},Default,{text}\n")
}
fn track(source: &str) -> Arc<Track> {
    taiyou_ass::parse_ass(source.as_bytes(), &ParseOptions::new(
        InputEncoding::Utf8, Profile::CoreV1, Limits::default(), SyntaxPolicy::Strict,
    ), &mut TrackIngestor::new()).unwrap().track().clone()
}
fn request(now: i64, size: Size) -> RenderRequest {
    RenderRequest { now: TimeUs::from_micros(now), viewport: Viewport::full(size).unwrap(), unsupported: UnsupportedPolicy::Strict }
}
fn renderer() -> Renderer { Renderer::new(RendererConfig::default()).unwrap() }
fn frame(input: &str, fonts: Arc<ValidatedFontCatalog>, size: Size) -> Frame {
    renderer().render(track(input), fonts, &request(750_000, size)).unwrap()
}
fn local(input: &str, fonts: &ValidatedFontCatalog) -> LocalLayout {
    let track = track(input);
    let evaluated = taiyou_render::evaluate(&track, EventId::new(0), TimeUs::from_micros(750_000), Profile::CoreV1).unwrap();
    let mut ctx = FontContext::with_limits(Limits::default()).unwrap();
    let shaped = shape_event(&mut ctx, fonts, &evaluated, &Default::default(), &Limits::default()).unwrap();
    layout(&shaped, &LayoutConstraints::new(evaluated.placement().alignment())).unwrap()
}
fn anchor(alignment: u8) -> (f64, f64) {
    let index = alignment - 1;
    let x = [13.0, 76.0, 139.0][usize::from(index % 3)];
    let y = [91.0, 50.0, 9.0][usize::from(index / 3)];
    (x, y)
}
fn pos(x: f64, y: f64, text: &str) -> String { format!("{{\\pos({x},{y})}}{text}") }

#[test]
fn real_pipeline_all_alignments_stack_with_local_metrics_and_preserve_effect_passes() {
    let fonts = catalog();
    for alignment in 1..=9 {
        for text in ["H", r"{\bord2\shad1\4a&H70&\1a&H80&}H", r"H\N\N{\fs30}H", r"H\nH\hH"] {
            let a = row(0, "0:00:00.00", "0:00:02.00", text);
            let input = source(alignment, &format!("{a}{a}"));
            let layout = local(&input, &fonts);
            // Uses the existing local box, not bitmap/stroke support. For two
            // identical events, minimum displacement is exactly its height.
            let height = layout.collision_bounds().height();
            assert!(height > 0.0);
            let (x, y) = anchor(alignment);
            let dy = if alignment <= 3 { -height } else { height };
            let explicit = source(alignment, &format!("{}{}",
                row(0, "0:00:00.00", "0:00:02.00", &pos(x, y, text)),
                row(0, "0:00:00.00", "0:00:02.00", &pos(x, y + dy, text))));
            for size in [Size::new(160, 100), Size::new(320, 300)] {
                let actual = frame(&input, fonts.clone(), size);
                let expected = frame(&explicit, fonts.clone(), size);
                assert_eq!(actual.pixels(), expected.pixels(), "alignment {alignment}, {text}");
            }
        }
    }
}

#[test]
fn start_order_differs_from_source_paint_order_and_equal_start_uses_id() {
    let fonts = catalog();
    let red = r"{\bord2\shad1\4a&H80&\1c&H0000FF&\1a&H80&}H";
    let blue = r"{\bord1\shad0\4a&H00&\1c&HFF0000&}H";
    let height = local(&source(7, &row(0, "0:00:00.00", "0:00:02.00", "H")), &fonts).collision_bounds().height();
    for (start, first_y, second_y) in [("0:00:00.50", 9.0 + height, 9.0), ("0:00:00.00", 9.0, 9.0 + height)] {
        let actual = source(7, &format!("{}{}", row(0, start, "0:00:02.00", red), row(0, "0:00:00.00", "0:00:02.00", blue)));
        let explicit = source(7, &format!("{}{}", row(0, start, "0:00:02.00", &pos(13.0, first_y, red)), row(0, "0:00:00.00", "0:00:02.00", &pos(13.0, second_y, blue))));
        assert_eq!(frame(&actual, fonts.clone(), Size::new(160, 100)).pixels(), frame(&explicit, fonts.clone(), Size::new(160, 100)).pixels());
    }
}

#[test]
fn layers_explicit_positions_empty_events_and_disjoint_text_keep_natural_anchors() {
    let fonts = catalog();
    let size = Size::new(160, 100);
    for (first, second) in [
        (row(1, "0:00:00.00", "0:00:02.00", "H"), "H"),
        (row(0, "0:00:00.00", "0:00:02.00", r"{\pos(13,9)}H"), "H"),
        (row(0, "0:00:00.00", "0:00:02.00", "H"), r"{\pos(13,9)}H"),
        (row(0, "0:00:00.00", "0:00:02.00", r"{\an9}H"), "H"),
        (row(0, "0:00:00.00", "0:00:02.00", ""), "H"),
        (row(0, "0:00:00.00", "0:00:02.00", r"\N\N"), "H"),
    ] {
        let actual = source(7, &format!("{first}{}", row(0, "0:00:00.00", "0:00:02.00", second)));
        // The natural second event equals an explicit second event. The first
        // stays identical, preserving all existing composition behavior.
        let expected = source(7, &format!("{first}{}", row(0, "0:00:00.00", "0:00:02.00", &pos(13.0, 9.0, second))));
        assert_eq!(frame(&actual, fonts.clone(), size).pixels(), frame(&expected, fonts.clone(), size).pixels());
    }
}

#[test]
fn fresh_forward_backward_randomized_seeks_and_exact_end_are_byte_identical() {
    let fonts = catalog();
    let input = source(2, &format!("{}{}",
        row(0, "0:00:00.00", "0:00:01.00", r"{\bord1\shad1\4a&H00&}H"),
        row(0, "0:00:00.50", "0:00:02.00", r"{\bord1\shad1\4a&H00&\1c&HFF0000&}H")));
    let parsed = track(&input);
    let mut sequential = renderer();
    let size = Size::new(160, 100);
    for now in [-1, 0, 499_999, 500_000, 750_000, 999_999, 1_000_000, 1_250_000, 2_000_000, 1_250_000, 750_000, 0, 1_999_999, 500_000, 1_250_000, 999_999] {
        let req = request(now, size);
        let actual = sequential.render(parsed.clone(), fonts.clone(), &req).unwrap();
        let fresh = renderer().render(parsed.clone(), fonts.clone(), &req).unwrap();
        assert_eq!(actual.pixels(), fresh.pixels(), "time {now}");
    }
    let solo = source(2, &row(0, "0:00:00.50", "0:00:02.00", r"{\bord1\shad1\4a&H00&\1c&HFF0000&}H"));
    let at_end = sequential.render(parsed, fonts.clone(), &request(1_000_000, size)).unwrap();
    assert_eq!(at_end.pixels(), frame(&solo, fonts, size).pixels());
}

#[test]
fn offscreen_stacks_are_clipped_only_at_composition_and_wrap_modes_stay_gated() {
    let fonts = catalog();
    let a = row(0, "0:00:00.00", "0:00:02.00", "H");
    let input = source(2, &a.repeat(8));
    let height = local(&input, &fonts).collision_bounds().height();
    let mut explicit = String::new();
    for i in 0..8 { explicit.push_str(&row(0, "0:00:00.00", "0:00:02.00", &pos(76.0, 91.0 - f64::from(i) * height, "H"))); }
    assert_eq!(frame(&input, fonts.clone(), Size::new(160, 100)).pixels(), frame(&source(2, &explicit), fonts.clone(), Size::new(160, 100)).pixels());
    for mode in [0, 1, 3] {
        let unsupported = input.replace("WrapStyle: 2", &format!("WrapStyle: {mode}"));
        let error = renderer().render(track(&unsupported), fonts.clone(), &request(750_000, Size::new(160, 100))).unwrap_err();
        assert_eq!(error.code(), ErrorCode::UnsupportedFeature);
    }
}
