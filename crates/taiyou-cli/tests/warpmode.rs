use taiyou_render::renderer::*;

fn parse(text: &str, wrap: u8, syntax: SyntaxPolicy) -> Result<ParsedTrack, ParseError> {
    let input = format!(
        "[Script Info]\nWrapStyle: {wrap}\n[V4+ Styles]\nFormat: Name,Fontsize,Outline,Shadow\nStyle: Default,20,0,0\nStyle: Alt,24,0,0\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:02.00,{text}"
    );
    taiyou_ass::parse_ass(
        input.as_bytes(),
        &ParseOptions::new(
            InputEncoding::Utf8,
            Profile::CoreV1,
            Limits::default(),
            syntax,
        ),
        &mut TrackIngestor::new(),
    )
}

fn evaluated(
    parsed: &ParsedTrack,
    policy: UnsupportedPolicy,
) -> Result<EvaluatedEvent, RenderError> {
    taiyou_render::evaluate_with_policy(
        parsed.track(),
        EventId::new(0),
        TimeUs::from_micros(500_000),
        Profile::CoreV1,
        policy,
        &Limits::default(),
    )
    .map(|(event, _)| event)
}

#[test]
fn typed_selectors_have_exact_sources_and_event_wide_last_wins() {
    for (text, mode, winning) in [
        ("A", WarpMode::XyStyle, None),
        (r"{\warpmode0}A", WarpMode::XyStyle, Some(r"\warpmode0")),
        (r"{\warpmode1}A", WarpMode::Unicode, Some(r"\warpmode1")),
        (
            r"{\warpmode0}A\NB{\warpmode1}C",
            WarpMode::Unicode,
            Some(r"\warpmode1"),
        ),
        (
            r"{\warpmode1}A{\rAlt}B{\r}C",
            WarpMode::Unicode,
            Some(r"\warpmode1"),
        ),
        (
            r"{\warpmode1}A{\warpmode0}B",
            WarpMode::XyStyle,
            Some(r"\warpmode0"),
        ),
        (r"literal \warpmode1", WarpMode::XyStyle, None),
    ] {
        let parsed = parse(text, 2, SyntaxPolicy::Strict).unwrap();
        let event = evaluated(&parsed, UnsupportedPolicy::Strict).unwrap();
        assert_eq!(event.warp_mode(), mode);
        assert_eq!(
            event.warp_mode().get(),
            if mode == WarpMode::Unicode { 1 } else { 0 }
        );
        assert_eq!(
            event
                .warp_mode_source()
                .and_then(|s| s.slice(parsed.track().source())),
            winning
        );
        for operation in parsed.track().events()[0].program() {
            if let TextOp::Override(OverrideOp::SetWarpMode { source, .. }) = operation {
                assert!(
                    source
                        .slice(parsed.track().source())
                        .unwrap()
                        .starts_with(r"\warpmode")
                );
            }
        }
        assert_eq!(
            event.source_map().unwrap().text(),
            event.runs().iter().map(|r| r.text()).collect::<String>()
        );
    }
}

#[test]
fn invalid_parameters_recover_without_replacing_the_prior_selection() {
    for value in [
        "", "2", "10", "-1", "+1", "1.0", "1e0", "NaN", "inf", "1foo", "(1)", "\u{a0}1",
    ] {
        let input = format!("{{\\warpmode1\\warpmode{value}}}A");
        let error = parse(&input, 2, SyntaxPolicy::Strict).unwrap_err();
        assert_eq!(error.code(), ErrorCode::InvalidScript, "{value}");
        let parsed = parse(&input, 2, SyntaxPolicy::Recover).unwrap();
        assert!(
            parsed
                .diagnostics()
                .iter()
                .any(|d| d.code() == DiagnosticCode::MalformedRecord)
        );
        let event = evaluated(&parsed, UnsupportedPolicy::Strict).unwrap();
        assert_eq!(event.warp_mode(), WarpMode::Unicode);
        assert_eq!(
            event
                .warp_mode_source()
                .unwrap()
                .slice(parsed.track().source()),
            Some(r"\warpmode1")
        );
    }
    assert_eq!(
        evaluated(
            &parse("{\\warpmode \t1 }A", 2, SyntaxPolicy::Strict).unwrap(),
            UnsupportedPolicy::Strict
        )
        .unwrap()
        .warp_mode(),
        WarpMode::Unicode
    );
}

#[test]
fn unknown_and_nested_tags_never_execute_a_selector() {
    for text in [
        r"{\t(0,100,\warpmode1)}A",
        r"{\WarpMode1}A",
        r"{\warpmodes1}A",
    ] {
        let parsed = parse(text, 2, SyntaxPolicy::Strict).unwrap();
        assert_eq!(
            evaluated(&parsed, UnsupportedPolicy::Strict)
                .unwrap_err()
                .code(),
            ErrorCode::UnsupportedFeature
        );
        assert_eq!(
            evaluated(&parsed, UnsupportedPolicy::Tolerant)
                .unwrap()
                .warp_mode(),
            WarpMode::XyStyle
        );
    }
}

#[test]
fn hard_breaks_are_identical_and_automatic_engines_never_fall_back() {
    let expected = "A\nB\n\nC\u{a0}D";
    for tag in ["", r"{\warpmode0}", r"{\warpmode1}"] {
        let parsed = parse(&format!("{tag}A\\nB\\N\\NC\\hD"), 2, SyntaxPolicy::Strict).unwrap();
        assert_eq!(
            evaluated(&parsed, UnsupportedPolicy::Strict)
                .unwrap()
                .source_map()
                .unwrap()
                .text(),
            expected
        );
        for wrap in [0, 1, 3] {
            let parsed = parse(&format!("{tag}A B C"), wrap, SyntaxPolicy::Strict).unwrap();
            for policy in [UnsupportedPolicy::Strict, UnsupportedPolicy::Tolerant] {
                if !tag.contains('1') {
                    assert_eq!(evaluated(&parsed, policy).unwrap().wrap_mode().get(), wrap);
                    continue;
                }
                let error = evaluated(&parsed, policy).unwrap_err();
                assert_eq!(error.code(), ErrorCode::UnsupportedFeature);
                assert_eq!(error.event(), Some(EventId::new(0)));
                assert!(error.detail().contains(if tag.contains('1') {
                    "warpmode1"
                } else {
                    "warpmode0"
                }));
            }
        }
    }
}

#[cfg(feature = "native")]
#[test]
fn pinned_font_q2_pixels_match_for_both_tags_and_fresh_seeking() {
    let mut ctx = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    let font = std::path::PathBuf::from(std::env::var_os("RUNNER_TEMP").unwrap())
        .join("taiyou-shaping-fonts/latin.otf");
    builder
        .register(&mut ctx, &std::fs::read(font).unwrap(), 0)
        .unwrap();
    let fonts = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let mut baseline: Option<Vec<u8>> = None;
    for tag in ["", r"{\warpmode0}", r"{\warpmode1}"] {
        let parsed = parse(
            &format!("{{\\fnSource Sans 3}}{tag}A\\nB\\N\\NC\\hD"),
            2,
            SyntaxPolicy::Strict,
        )
        .unwrap();
        let mut renderer = Renderer::new(Default::default()).unwrap();
        for now in [500_000, 2_000_000, 0, 1_000_000, 500_000] {
            let request = RenderRequest {
                now: TimeUs::from_micros(now),
                viewport: Viewport::full(Size::new(160, 100)).unwrap(),
                unsupported: UnsupportedPolicy::Strict,
            };
            let frame = renderer
                .render(parsed.track().clone(), fonts.clone(), &request)
                .unwrap();
            let fresh = Renderer::new(Default::default())
                .unwrap()
                .render(parsed.track().clone(), fonts.clone(), &request)
                .unwrap();
            assert_eq!(frame.pixels(), fresh.pixels());
            assert_eq!(frame.diagnostics(), fresh.diagnostics());
            if now == 500_000 {
                if let Some(pixels) = &baseline {
                    assert_eq!(frame.pixels(), pixels.as_slice());
                } else {
                    baseline = Some(frame.pixels().to_vec());
                }
            }
        }
    }
}
