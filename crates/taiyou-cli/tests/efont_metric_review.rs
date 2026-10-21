//! Focused evidence on the actual frozen embedded-font input; no golden update.
#![cfg(feature = "native")]
use std::{path::PathBuf, sync::Arc};
use taiyou_render::{layout, renderer::*, shaping::shape_event};

#[test]
fn embedded_fixture_isolates_cell_metrics_and_zero_depth_shadow() {
    let Some(output) = std::env::var_os("TAIYOU_REVIEW_OUTPUT") else {
        return;
    };
    let corpus = PathBuf::from(std::env::var("TAIYOU_CORPUS").unwrap());
    let root = PathBuf::from(output);
    std::fs::create_dir_all(&root).unwrap();
    let source = std::fs::read(corpus.join("regression/embedded-font/efont.ass")).unwrap();
    let parsed =
        taiyou_ass::parse_ass(&source, &Default::default(), &mut TrackIngestor::new()).unwrap();
    assert_eq!(parsed.embedded_fonts().len(), 1);
    let bytes = parsed.embedded_fonts()[0].bytes();
    std::fs::write(root.join("efont.otf"), bytes).unwrap();
    let mut context = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    builder.register(&mut context, bytes, 0).unwrap();
    let fonts = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let font = fonts.faces()[0].blob().id();
    let (upem, ascent, descent) = fonts.ass_cell_metrics(font).unwrap();
    let request = RenderRequest {
        now: TimeUs::from_micros(1_000_000),
        viewport: Viewport::full(Size::new(640, 480)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    };
    let mut evidence = format!("upem={upem} win_ascent={ascent} win_descent={descent}\n");
    for (label, restore_em, coincident) in [
        ("current", false, false),
        ("em-control", true, false),
        ("shadow-control", false, true),
        ("em-shadow-control", true, true),
    ] {
        let styles: Vec<_> = parsed
            .track()
            .styles()
            .iter()
            .map(|s| {
                let size = if restore_em {
                    s.font_size().get() * (f64::from(ascent) + f64::from(descent)) / f64::from(upem)
                } else {
                    s.font_size().get()
                };
                let s = s.clone().with_size(FiniteF64::new(size).unwrap()).unwrap();
                // A tiny positive depth quantizes to coincident coverage. This is
                // an owned diagnostic control for the old double draw, not a fix.
                if coincident {
                    s.with_shadow(FiniteF64::new(0.0001).unwrap())
                } else {
                    s
                }
            })
            .collect();
        let track = TrackIngestor::new()
            .ingest(
                Arc::from(parsed.track().source()),
                parsed.track().script().clone(),
                styles.into(),
                parsed.track().events().to_vec().into(),
                &Limits::default(),
            )
            .unwrap();
        let event =
            taiyou_render::evaluate(&track, track.events()[0].id(), request.now, Profile::CoreV1)
                .unwrap();
        let shaped = shape_event(
            &mut context,
            &fonts,
            &event,
            &Default::default(),
            &Limits::default(),
        )
        .unwrap();
        let local = layout(
            &shaped,
            &LayoutConstraints::new(shaped.placement().alignment()),
        )
        .unwrap();
        evidence.push_str(&format!(
            "{label} font_scale={} baseline={} ascent={} descent={}\n",
            fonts
                .ass_font_scale(font, event.runs()[0].shaping().font_size())
                .unwrap()
                .get(),
            local.lines()[0].baseline().y().get(),
            local.lines()[0].ascent().get(),
            local.lines()[0].descent().get()
        ));
        let frame = Renderer::new(Default::default())
            .unwrap()
            .render(track, fonts.clone(), &request)
            .unwrap();
        std::fs::write(root.join(format!("efont-{label}.rgba")), frame.pixels()).unwrap();
        if label == "current" {
            let empty = FontCatalogBuilder::new(Limits::default())
                .snapshot(&mut CatalogIngestor::new())
                .unwrap();
            let embedded = Renderer::new(Default::default())
                .unwrap()
                .render_parsed(&parsed, empty, &request)
                .unwrap();
            assert_eq!(embedded.pixels(), frame.pixels());
        }
    }
    std::fs::write(root.join("efont-metrics.txt"), &evidence).unwrap();
    eprintln!("{evidence}");
}
