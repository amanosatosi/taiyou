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
fn recovered_program_placement_matches_full_evaluator_and_explicit_frame_oracle() {
    let limits = Limits::default();
    let mut context = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(limits.clone());
    let path = PathBuf::from(std::env::var_os("RUNNER_TEMP").unwrap())
        .join("taiyou-shaping-fonts/latin.otf");
    builder
        .register(&mut context, &std::fs::read(path).unwrap(), 0)
        .unwrap();
    let fonts = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    for text in [
        r"literal \pos(1,2)",
        r"{\pos(10,10)\r}H",
        r"{\r\pos(10,10)\an7}H",
        r"{\pos(bad)\pos(12,14)}H",
        r"{\t(0,100,\pos(1,2))}H",
        r"{\pos(1,2)}{\rDefault\pos(20,20)}H",
        r"{\pos(1,2)H",
        r"{\rMissing\pos(1,2)}H",
    ] {
        let source = format!(
            "[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow,Alignment\nStyle: Default,Source Sans 3,12,0,0,7\n[Events]\nFormat: Layer,Start,End,Style,Text\nDialogue: 0,0:00:00.00,0:00:02.00,Default,{text}\nDialogue: 0,0:00:00.00,0:00:02.00,Default,H\nDialogue: 0,0:00:00.00,0:00:02.00,Default,H"
        );
        let parsed = taiyou_ass::parse_ass(
            source.as_bytes(),
            &ParseOptions::new(
                InputEncoding::Utf8,
                Profile::CoreV1,
                limits.clone(),
                SyntaxPolicy::Recover,
            ),
            &mut TrackIngestor::new(),
        )
        .unwrap();
        let track = parsed.track();
        for policy in [UnsupportedPolicy::Strict, UnsupportedPolicy::Tolerant] {
            let request = RenderRequest {
                now: TimeUs::from_micros(750_000),
                viewport: Viewport::full(Size::new(160, 100)).unwrap(),
                unsupported: policy,
            };
            let mut inputs = Vec::new();
            let mut natural = Vec::new();
            let mut failure = None;
            for event in track.events() {
                let evaluated = match taiyou_render::evaluate_with_policy(
                    track,
                    event.id(),
                    request.now,
                    Profile::CoreV1,
                    policy,
                    &limits,
                ) {
                    Ok((state, _)) => state,
                    Err(error) => {
                        failure = Some(error);
                        break;
                    }
                };
                let state = evaluated.placement();
                let anchor = alignment_anchor(state, track.script().play_res()).unwrap();
                natural.push(anchor);
                if state.collision_eligible() {
                    let shaped = shape_event(
                        &mut context,
                        &fonts,
                        &evaluated,
                        &Default::default(),
                        &limits,
                    )
                    .unwrap();
                    let local =
                        taiyou_render::layout(&shaped, &LayoutConstraints::new(state.alignment()))
                            .unwrap();
                    inputs.push(CollisionInput {
                        id: event.id(),
                        layer: event.layer(),
                        start: event.start(),
                        alignment: state.alignment(),
                        anchor,
                        bounds: collision_footprint(
                            &local,
                            DeviceMapping::new(
                                track.script().play_res(),
                                request.viewport.content(),
                            )
                            .unwrap(),
                            track.script().scaled_border_shadow(),
                            policy,
                        )
                        .unwrap(),
                        eligible: true,
                    });
                }
            }
            let actual = Renderer::new(Default::default()).unwrap().render(
                track.clone(),
                fonts.clone(),
                &request,
            );
            if let Some(error) = failure {
                assert_eq!(actual.unwrap_err().code(), error.code());
                continue;
            }
            let placements = resolve_collisions(&inputs, Profile::CoreV1, &limits).unwrap();
            let events: Vec<_> = track
                .events()
                .iter()
                .enumerate()
                .map(|(i, event)| {
                    let anchor = placements
                        .iter()
                        .find(|p| p.id == event.id())
                        .map_or(natural[i], |p| p.anchor);
                    let mut program = event.program().to_vec();
                    let end = event.source().end() as usize;
                    program.push(TextOp::Override(OverrideOp::SetPosition {
                        point: anchor,
                        source: SourceSpan::new(track.source(), end, end).unwrap(),
                    }));
                    Event::new(
                        event.id(),
                        event.start(),
                        event.end(),
                        event.style(),
                        event.source(),
                    )
                    .with_layer(event.layer())
                    .with_margins(event.margins())
                    .with_program(program.into())
                })
                .collect();
            let positioned = TrackIngestor::new()
                .ingest(
                    track.source_storage().clone(),
                    track.script().clone(),
                    track.styles().into(),
                    events.into(),
                    &limits,
                )
                .unwrap();
            let expected = Renderer::new(Default::default())
                .unwrap()
                .render(positioned, fonts.clone(), &request)
                .unwrap();
            let actual = actual.unwrap();
            assert_eq!(actual.pixels(), expected.pixels(), "{text} {policy:?}");
            assert_eq!(actual.diagnostics(), expected.diagnostics());
        }
    }
}
