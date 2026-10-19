use super::*;

fn catalog() -> Arc<ValidatedFontCatalog> {
    let path = std::path::PathBuf::from(std::env::var_os("RUNNER_TEMP").unwrap())
        .join("taiyou-shaping-fonts/latin.otf");
    let mut context = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    builder
        .register(&mut context, &std::fs::read(path).unwrap(), 0)
        .unwrap();
    builder.snapshot(&mut CatalogIngestor::new()).unwrap()
}
fn track(count: u32, positioned: u32) -> Arc<Track> {
    let text = "HHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH";
    let source: Arc<str> = Arc::from(text.repeat((count + positioned) as usize));
    let style = Style::core_default()
        .with_font(FontRequest::new(
            Arc::from("Source Sans 3"),
            400,
            false,
            false,
        ))
        .with_size(FiniteF64::new(5.0).unwrap())
        .unwrap()
        .with_border(FiniteF64::new(0.5).unwrap())
        .unwrap()
        .with_shadow(FiniteF64::new(1.25).unwrap());
    let events: Vec<_> = (0..count + positioned)
        .map(|id| {
            let start = id as usize * text.len();
            let span = SourceSpan::new(&source, start, start + text.len()).unwrap();
            let mut program = vec![TextOp::Text {
                text: Arc::from(text),
                source: span,
            }];
            if id >= count {
                program.insert(
                    0,
                    TextOp::Override(OverrideOp::SetPosition {
                        point: ScriptPoint::new(
                            FiniteF64::new(-100.0).unwrap(),
                            FiniteF64::new(-100.0).unwrap(),
                        ),
                        source: SourceSpan::new(&source, start, start).unwrap(),
                    }),
                );
            }
            Event::new(
                EventId::new(id),
                TimeUs::from_micros(if id % 2 == 0 { 0 } else { 200_000 }),
                TimeUs::from_micros(if id == 0 { 1_000_000 } else { 2_000_000 }),
                StyleId::new(0),
                span,
            )
            .with_layer((id % 3) as i32)
            .with_program(program.into())
        })
        .collect();
    TrackIngestor::new()
        .ingest(
            source,
            ScriptInfo::core_default(),
            Arc::from([style]),
            events.into(),
            &Limits::default(),
        )
        .unwrap()
}
fn request(now: i64) -> RenderRequest {
    RenderRequest {
        now: TimeUs::from_micros(now),
        viewport: Viewport::full(Size::new(160, 100)).unwrap(),
        unsupported: UnsupportedPolicy::Strict,
    }
}
#[test]
fn adaptive_full_partial_minimal_and_limit_results_agree() {
    let fonts = catalog();
    let count = 256;
    let track = track(count, 4);
    let (mut full, mut partial, mut succeeded, mut failed) = (false, false, false, false);
    for mib in [1, 18, 20, 22, 24, 32, 128] {
        let config = RendererConfig {
            limits: Limits::default()
                .with_transient_bytes(mib * 1024 * 1024)
                .unwrap(),
            ..Default::default()
        };
        let minimal = Renderer::new(config.clone()).unwrap().render_staged(
            track.clone(),
            fonts.clone(),
            &request(750_000),
            false,
            &mut false,
        );
        PREPARE_CALLS.with(|n| n.set(0));
        RETAINED_CALLS.with(|n| n.set(0));
        let adaptive =
            Renderer::new(config)
                .unwrap()
                .render(track.clone(), fonts.clone(), &request(750_000));
        let retained = RETAINED_CALLS.with(Cell::get);
        full |= retained == count as usize;
        match (minimal, adaptive) {
            (Ok(a), Ok(b)) => {
                succeeded = true;
                partial |= retained > 0 && retained < count as usize;
                assert_eq!(a.pixels(), b.pixels(), "budget {mib}");
                assert_eq!(a.diagnostics(), b.diagnostics());
                if mib == 128 {
                    assert_eq!(PREPARE_CALLS.with(Cell::get), count as usize + 4);
                }
            }
            (Err(a), Err(b)) => {
                failed = true;
                assert_eq!(
                    (a.code(), a.event(), a.span()),
                    (b.code(), b.event(), b.span())
                );
            }
            _ => panic!("optional staging changed success at {mib} MiB"),
        }
    }
    assert!(full && partial && succeeded && failed);
}
#[test]
fn retained_layouts_are_frame_local_and_position_only_has_no_prepass() {
    let fonts = catalog();
    let input = track(8, 3);
    let mut renderer = Renderer::new(Default::default()).unwrap();
    for now in [1_250_000, 750_000, 1_250_000, 500_000, 0, 1_250_000] {
        let a = renderer
            .render(input.clone(), fonts.clone(), &request(now))
            .unwrap();
        let b = Renderer::new(Default::default())
            .unwrap()
            .render_staged(
                input.clone(),
                fonts.clone(),
                &request(now),
                false,
                &mut false,
            )
            .unwrap();
        assert_eq!(a.pixels(), b.pixels());
        assert_eq!(a.diagnostics(), b.diagnostics());
    }
    for count in [0, 1] {
        PREPARE_CALLS.with(|n| n.set(0));
        renderer
            .render(track(count, 100), fonts.clone(), &request(750_000))
            .unwrap();
        assert_eq!(PREPARE_CALLS.with(Cell::get), count as usize + 100);
    }
}
