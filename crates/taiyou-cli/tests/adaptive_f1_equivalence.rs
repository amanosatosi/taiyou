#![cfg(feature = "native")]
use std::path::PathBuf;
use taiyou_render::{
    collision::{CollisionInput, resolve_collisions},
    collision_footprint::collision_footprint,
    geometry::DeviceMapping,
    renderer::*,
    shaping::shape_event,
};

#[test]
fn measured_effects_scene_matches_independent_f1_anchors_not_pre_f1_anchors() {
    let source = include_str!(
        "../../../experiments/t017/results/ae467c9/optimized-initial/adaptive-256-multiglyph.ass"
    );
    let limits = Limits::default();
    let parsed = taiyou_ass::parse_ass(
        source.as_bytes(),
        &ParseOptions::new(
            InputEncoding::Utf8,
            Profile::CoreV1,
            limits.clone(),
            SyntaxPolicy::Strict,
        ),
        &mut TrackIngestor::new(),
    )
    .unwrap();
    let track = parsed.track();
    let mut context = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(limits.clone());
    let font = PathBuf::from(std::env::var_os("RUNNER_TEMP").unwrap())
        .join("taiyou-shaping-fonts/latin.otf");
    builder
        .register(&mut context, &std::fs::read(font).unwrap(), 0)
        .unwrap();
    let fonts = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let evaluated = taiyou_render::evaluate(
        track,
        EventId::new(0),
        TimeUs::from_micros(750_000),
        Profile::CoreV1,
    )
    .unwrap();
    let shaped = shape_event(
        &mut context,
        &fonts,
        &evaluated,
        &Default::default(),
        &limits,
    )
    .unwrap();
    let local = taiyou_render::layout(&shaped, &LayoutConstraints::new(Alignment::new(7).unwrap()))
        .unwrap();
    // Unit mapping, no rotation/scale, visible fill/outline/shadow, Q6-exact
    // radius .5 and offset (1.25,1.25). Independently union the analytic hulls;
    // do not call the effect-bounds implementation to establish this expectation.
    let base = local.collision_bounds();
    let mut expected = base;
    for positioned in local.glyphs() {
        let glyph = &shaped.runs()[positioned.run_index()].glyphs()[positioned.glyph_index()];
        let GlyphInk::Bounds(ink) = glyph.ink() else {
            panic!("H must have ink")
        };
        let x = positioned.position().x().get();
        let y = positioned.position().y().get();
        expected = expected
            .union(
                ScriptBounds::new(
                    ink.x0().get() + x - 0.5,
                    ink.y0().get() + y - 0.5,
                    ink.x1().get() + x + 0.5 + 1.25,
                    ink.y1().get() + y + 0.5 + 1.25,
                )
                .unwrap(),
            )
            .unwrap();
    }
    assert!(expected.height() > base.height());
    assert_eq!(
        collision_footprint(
            &local,
            DeviceMapping::new(Size::new(160, 100), PixelRect::new(0, 0, 160, 100).unwrap())
                .unwrap(),
            false,
            UnsupportedPolicy::Strict,
        )
        .unwrap(),
        expected
    );
    let input = |id, bounds| CollisionInput {
        id: EventId::new(id),
        layer: 0,
        start: TimeUs::from_micros(0),
        alignment: Alignment::new(7).unwrap(),
        anchor: ScriptPoint::new(FiniteF64::new(13.0).unwrap(), FiniteF64::new(9.0).unwrap()),
        bounds,
        eligible: true,
    };
    let placements = resolve_collisions(
        &(0..256).map(|id| input(id, expected)).collect::<Vec<_>>(),
        Profile::CoreV1,
        &limits,
    )
    .unwrap();
    let mut y = 9.0;
    for placement in &placements {
        assert_eq!(placement.anchor.y().get(), y);
        y = (y + expected.y1().get()) - expected.y0().get();
    }
    let explicit = |bounds: ScriptBounds| {
        let mut y = 9.0;
        let events: Vec<_> = track
            .events()
            .iter()
            .map(|event| {
                let mut program = event.program().to_vec();
                program.push(TextOp::Override(OverrideOp::SetPosition {
                    point: ScriptPoint::new(
                        FiniteF64::new(13.0).unwrap(),
                        FiniteF64::new(y).unwrap(),
                    ),
                    source: SourceSpan::new(
                        track.source(),
                        event.source().end() as usize,
                        event.source().end() as usize,
                    )
                    .unwrap(),
                }));
                y = (y + bounds.y1().get()) - bounds.y0().get();
                Event::new(
                    event.id(),
                    event.start(),
                    event.end(),
                    event.style(),
                    event.source(),
                )
                .with_program(program.into())
            })
            .collect();
        TrackIngestor::new()
            .ingest(
                track.source_storage().clone(),
                track.script().clone(),
                track.styles().into(),
                events.into(),
                &limits,
            )
            .unwrap()
    };
    let request = RenderRequest {
        now: TimeUs::from_micros(750_000),
        viewport: Viewport::full(Size::new(160, 100)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    };
    let render = |track| {
        Renderer::new(Default::default())
            .unwrap()
            .render(track, fonts.clone(), &request)
            .unwrap()
    };
    let actual = render(track.clone());
    let f1 = render(explicit(expected));
    let old_anchors = render(explicit(base));
    assert_eq!(actual.pixels(), f1.pixels());
    assert_eq!(actual.diagnostics(), f1.diagnostics());
    assert_ne!(actual.pixels(), old_anchors.pixels());
    println!(
        "T017_F1_ANCHORS: base={base:?} approved={expected:?} second_y={} events=256",
        placements[1].anchor.y().get()
    );
}
