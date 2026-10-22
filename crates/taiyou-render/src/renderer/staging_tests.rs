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
                if mib == 32 {
                    assert!(retained > 0 && retained < count as usize);
                    assert!(PREPARE_CALLS.with(Cell::get) < 2 * count as usize + 4);
                }
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

// Additive P0 review coverage. Existing staging fixtures/assertions are unchanged.
fn effects_track(border: f64) -> Arc<Track> {
    let texts: Vec<_> = (0..500)
        .map(|i| {
            if i < 498 {
                "Fansub typesetting"
            } else {
                "dialogue"
            }
        })
        .collect();
    let source: Arc<str> = Arc::from(texts.concat());
    let dialogue = Style::core_default()
        .with_font(FontRequest::new(
            Arc::from("Source Sans 3"),
            400,
            false,
            false,
        ))
        .with_size(FiniteF64::new(20.0).unwrap())
        .unwrap()
        .with_border(FiniteF64::ZERO)
        .unwrap()
        .with_shadow(FiniteF64::ZERO)
        .with_alignment(Alignment::new(7).unwrap())
        .with_margins(Margins::new(13, 21, 9))
        .with_colors(
            Rgba8::new(255, 255, 255, 255),
            Rgba8::new(255, 255, 0, 255),
            Rgba8::new(0, 0, 0, 255),
            Rgba8::new(0, 0, 0, 0),
        );
    let effect = dialogue
        .clone()
        .with_size(FiniteF64::new(5.0).unwrap())
        .unwrap()
        .with_border(FiniteF64::new(border).unwrap())
        .unwrap()
        .with_shadow(FiniteF64::new(1.25).unwrap())
        .with_colors(
            Rgba8::new(255, 255, 255, 255),
            Rgba8::new(255, 255, 0, 255),
            Rgba8::new(0, 0, 0, 255),
            Rgba8::new(0, 0, 0, if border == 0.5 { 0 } else { 255 }),
        );
    let mut offset = 0;
    let events: Vec<_> = texts
        .into_iter()
        .enumerate()
        .map(|(i, text)| {
            let span = SourceSpan::new(&source, offset, offset + text.len()).unwrap();
            offset += text.len();
            let mut program = Vec::new();
            if i < 498 {
                program.push(TextOp::Override(OverrideOp::SetPosition {
                    point: ScriptPoint::new(
                        FiniteF64::new(((i % 10) * 16) as f64).unwrap(),
                        FiniteF64::new(((i / 10 % 10) * 10) as f64).unwrap(),
                    ),
                    source: SourceSpan::new(&source, span.start() as usize, span.start() as usize)
                        .unwrap(),
                }));
            }
            program.push(TextOp::Text {
                text: Arc::from(text),
                source: span,
            });
            Event::new(
                EventId::new(i as u32),
                TimeUs::from_micros(0),
                TimeUs::from_micros(2_000_000),
                StyleId::new(if i < 498 { 1 } else { 0 }),
                span,
            )
            .with_program(program.into())
        })
        .collect();
    TrackIngestor::new()
        .ingest(
            source,
            ScriptInfo::core_default()
                .with_play_res(Size::new(160, 100))
                .unwrap(),
            Arc::from([dialogue, effect]),
            events.into(),
            &Limits::default(),
        )
        .unwrap()
}

#[test]
fn positive_cell_stroke_budget_evidence() {
    let fonts = catalog();
    let font = fonts.faces()[0].blob().id();
    let (upem, ascent, descent) = fonts.ass_cell_metrics(font).unwrap();
    let mut report = format!("cell={upem},{ascent},{descent}\n");
    for border in [0.5, 3.0] {
        for restore_em in [false, true] {
            let input = effects_track(border);
            let input = if restore_em {
                let ratio = (f64::from(ascent) + f64::from(descent)) / f64::from(upem);
                let styles: Vec<_> = input
                    .styles()
                    .iter()
                    .map(|s| {
                        s.clone()
                            .with_size(FiniteF64::new(s.font_size().get() * ratio).unwrap())
                            .unwrap()
                    })
                    .collect();
                TrackIngestor::new()
                    .ingest(
                        Arc::from(input.source()),
                        input.script().clone(),
                        styles.into(),
                        input.events().to_vec().into(),
                        &Limits::default(),
                    )
                    .unwrap()
            } else {
                input
            };
            PREPARATION_ATTEMPTS.with(|n| n.set(0));
            PREPARE_CALLS.with(|n| n.set(0));
            let error = Renderer::new(Default::default())
                .unwrap()
                .render(input, fonts.clone(), &request(500_000))
                .unwrap_err();
            eprintln!(
                "CELL_STROKE border={border} restore_em={restore_em} prepares={} error={error:?}",
                PREPARE_CALLS.with(Cell::get)
            );
            assert_eq!(PREPARATION_ATTEMPTS.with(Cell::get), 1);
            assert_eq!(error.code(), ErrorCode::LimitExceeded);
            assert_eq!(error.resource_failure(), ResourceFailure::Other);
            let expected = match (border == 0.5, restore_em) {
                (true, false) => (423, 426),
                (true, true) => (441, 444),
                (false, false) => (225, 228),
                (false, true) => (235, 238),
            };
            assert_eq!(error.event(), Some(EventId::new(expected.0)));
            assert_eq!(PREPARE_CALLS.with(Cell::get), expected.1);
            if restore_em {
                assert!(error.detail().contains("check=frame-segments"));
            } else {
                // Increasing only the caller's frame segment budget moves the
                // failure forward. Coordinate/transient limits are unchanged.
                let config = RendererConfig {
                    limits: Limits::default()
                        .with_path_segments_per_frame(1_010_000)
                        .unwrap(),
                    ..Default::default()
                };
                let later = Renderer::new(config)
                    .unwrap()
                    .render(effects_track(border), fonts.clone(), &request(500_000))
                    .unwrap_err();
                assert_eq!(later.code(), ErrorCode::LimitExceeded);
                assert_eq!(later.resource_failure(), ResourceFailure::Other);
                assert!(later.event().unwrap() > EventId::new(expected.0));
                eprintln!("CELL_STROKE larger_segment_budget={later:?}");
            }

            report.push_str(&format!(
                "border={border} restore_em={restore_em} prepares={} error={error:?}\n",
                expected.1
            ));
        }
    }
    eprintln!("CELL_STROKE_BUDGET\n{report}");
    report_p0("cell-stroke", &report);
}

#[test]
fn fixed_segment_exhaustion_never_repeats_preparation() {
    let fonts = catalog();
    // Pre-cell-height geometry exhausted at 441/444 and 235/238. Those
    // counts were not an ASS contract; the fixed-budget/no-retry invariant is.
    for (border, event, prepares) in [(0.5, 423, 426), (3.0, 225, 228)] {
        let input = effects_track(border);
        let minimal = Renderer::new(Default::default())
            .unwrap()
            .render_staged(
                input.clone(),
                fonts.clone(),
                &request(500_000),
                false,
                &mut false,
            )
            .unwrap_err();
        let original = Renderer::new(Default::default())
            .unwrap()
            .render_staged(
                input.clone(),
                fonts.clone(),
                &request(500_000),
                true,
                &mut false,
            )
            .unwrap_err();
        for _ in 0..2 {
            PREPARATION_ATTEMPTS.with(|n| n.set(0));
            PREPARE_CALLS.with(|n| n.set(0));
            let error = Renderer::new(Default::default())
                .unwrap()
                .render(input.clone(), fonts.clone(), &request(500_000))
                .unwrap_err();
            assert_eq!(PREPARATION_ATTEMPTS.with(Cell::get), 1);
            assert_eq!(PREPARE_CALLS.with(Cell::get), prepares);
            assert_eq!(error.code(), ErrorCode::LimitExceeded);
            assert_eq!(error.resource_failure(), ResourceFailure::Other);
            assert_eq!(error.event(), Some(EventId::new(event)));
            assert_eq!(
                (error.code(), error.event(), error.span()),
                (minimal.code(), minimal.event(), minimal.span())
            );
            assert_eq!(
                error, original,
                "return the first attempt's complete original error"
            );
            // With current geometry, the remaining segment allowance is too
            // small for the next input outline, before FT stroker output.
            assert_eq!(
                error.detail(),
                "geometry coordinate/count/reservation limit"
            );
        }
        let config = RendererConfig {
            limits: Limits::default()
                .with_path_segments_per_frame(4_000_000)
                .unwrap(),
            ..Default::default()
        };
        let a = Renderer::new(config.clone())
            .unwrap()
            .render_staged(
                input.clone(),
                fonts.clone(),
                &request(500_000),
                false,
                &mut false,
            )
            .unwrap();
        PREPARATION_ATTEMPTS.with(|n| n.set(0));
        PREPARE_CALLS.with(|n| n.set(0));
        let b = Renderer::new(config)
            .unwrap()
            .render(input, fonts.clone(), &request(500_000))
            .unwrap();
        assert_eq!(PREPARATION_ATTEMPTS.with(Cell::get), 1);
        assert_eq!(PREPARE_CALLS.with(Cell::get), 500);
        assert_eq!(a.pixels(), b.pixels());
        assert_eq!(a.diagnostics(), b.diagnostics());
        eprintln!(
            "P0_SEGMENT border={border} event={event} default_prepares={prepares} attempts=1 expanded_prepares=500"
        );
        report_p0(
            &format!("segments-{border}"),
            &format!(
                "border={border} event={event} default_prepares={prepares} default_attempts=1 expanded_prepares={} expanded_attempts={} rgba_bytes={}\noriginal_error={original:?}\n",
                PREPARE_CALLS.with(Cell::get),
                PREPARATION_ATTEMPTS.with(Cell::get),
                a.pixels().len()
            ),
        );
    }
}

#[test]
fn genuine_transient_pressure_retries_once_without_publishing_failed_attempt() {
    let fonts = catalog();
    let input = track(8, 0);
    let config = |bytes| RendererConfig {
        limits: Limits::default().with_transient_bytes(bytes).unwrap(),
        ..Default::default()
    };
    // Find this pinned fixture's exact logical-budget boundary, not a host-RAM
    // estimate or a manufactured error. Minimal staging is monotone in bytes.
    let (mut low, mut high) = (1, 64 * 1024 * 1024);
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        if Renderer::new(config(middle))
            .unwrap()
            .render_staged(
                input.clone(),
                fonts.clone(),
                &request(750_000),
                false,
                &mut false,
            )
            .is_ok()
        {
            high = middle;
        } else {
            low = middle;
        }
    }
    let a = Renderer::new(config(high))
        .unwrap()
        .render_staged(
            input.clone(),
            fonts.clone(),
            &request(750_000),
            false,
            &mut false,
        )
        .unwrap();
    let mut optional = false;
    let error = Renderer::new(config(high))
        .unwrap()
        .render_staged(
            input.clone(),
            fonts.clone(),
            &request(750_000),
            true,
            &mut optional,
        )
        .unwrap_err();
    assert!(optional);
    assert_eq!(error.resource_failure(), ResourceFailure::TransientMemory);
    let mut renderer = Renderer::new(config(high)).unwrap();
    let mut attempts = Vec::new();
    for _ in 0..2 {
        PREPARATION_ATTEMPTS.with(|n| n.set(0));
        let b = renderer
            .render(input.clone(), fonts.clone(), &request(750_000))
            .unwrap();
        assert_eq!(PREPARATION_ATTEMPTS.with(Cell::get), 2);
        attempts.push(PREPARATION_ATTEMPTS.with(Cell::get));
        assert_eq!(a.pixels(), b.pixels());
        assert_eq!(a.diagnostics(), b.diagnostics());
    }
    // A failed minimal attempt still returns its own original contextual error.
    let minimal_error = Renderer::new(config(low))
        .unwrap()
        .render_staged(
            input.clone(),
            fonts.clone(),
            &request(750_000),
            false,
            &mut false,
        )
        .unwrap_err();
    let error = Renderer::new(config(low))
        .unwrap()
        .render(input, fonts, &request(750_000))
        .unwrap_err();
    assert_eq!(
        (error.code(), error.event(), error.span(), error.detail()),
        (
            minimal_error.code(),
            minimal_error.event(),
            minimal_error.span(),
            minimal_error.detail()
        )
    );
    eprintln!(
        "P0_MEMORY minimal_success_bytes={high} minimal_failure_bytes={low} retry_attempts=2"
    );
    report_p0(
        "memory",
        &format!(
            "minimal_success_bytes={high} minimal_failure_bytes={low} repeated_retry_attempts={attempts:?} rgba_bytes={}\nminimal_error={minimal_error:?}\n",
            a.pixels().len()
        ),
    );
}

fn report_p0(name: &str, report: &str) {
    if let Some(folder) = std::env::var_os("TAIYOU_RENDER_SAMPLE_DIR") {
        let toolchain = std::env::var("RUSTUP_TOOLCHAIN").unwrap_or_default();
        let suffix = if toolchain.starts_with("1.85.") {
            "msrv"
        } else {
            "native"
        };
        std::fs::write(
            std::path::PathBuf::from(folder).join(format!("P0-{name}-{suffix}.txt")),
            format!("toolchain={toolchain}\n{report}"),
        )
        .unwrap();
    }
}

#[test]
fn allocation_and_transient_classification_preserve_c7_identity() {
    let source = "allocation";
    let span = SourceSpan::new(source, 0, source.len()).unwrap();
    for error in [
        RenderError::transient_limit("pressure"),
        RenderError::new(ErrorCode::AllocationFailed, "allocation"),
    ] {
        let code = error.code();
        let contextual = error
            .with_context(Some(EventId::new(7)), Some(span))
            .with_native_code(19);
        assert_eq!(
            contextual.resource_failure(),
            ResourceFailure::TransientMemory
        );
        assert_eq!(contextual.code(), code);
        assert_eq!(contextual.event(), Some(EventId::new(7)));
        assert_eq!(contextual.span(), Some(span));
        assert_eq!(contextual.native_code(), Some(19));
    }
    assert_eq!(
        RenderError::new(ErrorCode::LimitExceeded, "same text").resource_failure(),
        ResourceFailure::Other
    );
    assert_eq!(
        RenderError::transient_limit("same text").resource_failure(),
        ResourceFailure::TransientMemory
    );
}

#[test]
fn consumed_retention_slots_do_not_provoke_a_full_frame_retry() {
    let fonts = catalog();
    let input = track(256, 4);
    let config = RendererConfig {
        limits: Limits::default()
            .with_transient_bytes(32 * 1024 * 1024)
            .unwrap(),
        ..Default::default()
    };
    PREPARATION_ATTEMPTS.with(|n| n.set(0));
    PREPARE_CALLS.with(|n| n.set(0));
    let adaptive = Renderer::new(config.clone())
        .unwrap()
        .render(input.clone(), fonts.clone(), &request(750_000))
        .unwrap();
    assert_eq!(PREPARATION_ATTEMPTS.with(Cell::get), 1);
    assert!(PREPARE_CALLS.with(Cell::get) < 2 * 256 + 4);
    let minimal = Renderer::new(config)
        .unwrap()
        .render_staged(input, fonts, &request(750_000), false, &mut false)
        .unwrap();
    assert_eq!(adaptive.pixels(), minimal.pixels());
    assert_eq!(adaptive.diagnostics(), minimal.diagnostics());
}
