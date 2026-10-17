use taiyou_ass::parse_ass;
use taiyou_types::*;

fn script(text: &str) -> String {
    format!("[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:01.00,{text}")
}
fn parse(text: &str) -> ParsedTrack {
    parse_ass(
        script(text).as_bytes(),
        &ParseOptions::default(),
        &mut TrackIngestor::new(),
    )
    .unwrap()
}

#[test]
fn overrides_color_reset_and_text_retain_exact_order_and_spans() {
    let parsed = parse(r"{\c&H0000FF&}A{\r}B");
    let track = parsed.track();
    let ops = track.events()[0].program();
    assert_eq!(ops.len(), 4);
    assert!(
        matches!(ops[0], TextOp::Override(OverrideOp::SetPaint { channel: PaintChannel::Primary, color, .. }) if color.channels() == [255,0,0,255])
    );
    assert!(
        matches!(&ops[1], TextOp::Text { text, source } if text.as_ref() == "A" && source.slice(track.source()) == Some("A"))
    );
    assert!(matches!(
        ops[2],
        TextOp::Override(OverrideOp::ResetStyle { name: None, .. })
    ));
    assert!(matches!(&ops[3], TextOp::Text { text, .. } if text.as_ref() == "B"));
    let TextOp::Override(op) = &ops[0] else {
        panic!("missing color")
    };
    assert_eq!(op.source().slice(track.source()), Some(r"\c&H0000FF&"));
}

#[test]
fn overrides_all_assigned_tags_have_typed_parameters() {
    // ADR-009 requires a declared style for a legitimate named reset. Put its
    // definition after Events to also exercise final, forward name resolution.
    let text = r"{\fnSome Font\fs24\b-1\i1\fscx150\fscy50\fsp-2\c&H112233&\1c&HFF&\2c&H00FF00&\3c&H123&\4c&H0&\alpha&H80&\1a&HFF&\2a&H0&\3a&H01&\4a&H7F&\an9\pos(-2.5, 3)\rNamed}";
    let input = format!(
        "{}\n[V4+ Styles]\nFormat: Name,Fontsize\nStyle: Named,24",
        script(text)
    );
    let parsed = parse_ass(
        input.as_bytes(),
        &ParseOptions::default(),
        &mut TrackIngestor::new(),
    )
    .unwrap();
    let ops = parsed.track().events()[0].program();
    assert_eq!(ops.len(), 20);
    assert!(
        matches!(&ops[0], TextOp::Override(OverrideOp::SetFont { family, .. }) if family.as_ref() == "Some Font")
    );
    assert!(
        matches!(ops[1], TextOp::Override(OverrideOp::SetSize { size, .. }) if size.get() == 24.0)
    );
    assert!(matches!(
        ops[2],
        TextOp::Override(OverrideOp::SetWeight { weight: 700, .. })
    ));
    assert!(matches!(
        ops[3],
        TextOp::Override(OverrideOp::SetItalic { italic: true, .. })
    ));
    assert!(
        matches!(ops[4], TextOp::Override(OverrideOp::SetScale { axis: ScaleAxis::X, factor, .. }) if factor.get() == 1.5)
    );
    assert!(
        matches!(ops[5], TextOp::Override(OverrideOp::SetScale { axis: ScaleAxis::Y, factor, .. }) if factor.get() == 0.5)
    );
    assert!(
        matches!(ops[6], TextOp::Override(OverrideOp::SetSpacing { spacing, .. }) if spacing.get() == -2.0)
    );
    assert!(matches!(
        ops[12],
        TextOp::Override(OverrideOp::SetOpacity {
            channel: None,
            opacity: 127,
            ..
        })
    ));
    assert!(matches!(
        ops[13],
        TextOp::Override(OverrideOp::SetOpacity {
            channel: Some(PaintChannel::Primary),
            opacity: 0,
            ..
        })
    ));
    assert!(matches!(
        ops[14],
        TextOp::Override(OverrideOp::SetOpacity {
            channel: Some(PaintChannel::Secondary),
            opacity: 255,
            ..
        })
    ));
    assert!(
        matches!(ops[17], TextOp::Override(OverrideOp::SetAlignment { alignment, .. }) if alignment.get() == 9)
    );
    assert!(
        matches!(ops[18], TextOp::Override(OverrideOp::SetPosition { point, .. }) if point.x().get() == -2.5 && point.y().get() == 3.0)
    );
    assert!(
        matches!(&ops[19], TextOp::Override(OverrideOp::ResetStyle { name: Some(name), .. }) if name.as_ref() == "Named")
    );
    for op in ops {
        let TextOp::Override(op) = op else {
            panic!("unexpected operation")
        };
        assert!(
            op.source()
                .slice(parsed.track().source())
                .unwrap()
                .starts_with('\\')
        );
        op.validate().unwrap();
    }
}

#[test]
fn overrides_escapes_empty_blocks_and_literal_whitespace_are_preserved() {
    let parsed = parse(r" A\NB\nC\hD{} E ");
    let ops = parsed.track().events()[0].program();
    assert_eq!(ops.len(), 8);
    assert!(matches!(&ops[0], TextOp::Text { text, .. } if text.as_ref() == " A"));
    assert_eq!(ops[1], TextOp::HardBreak);
    assert_eq!(ops[3], TextOp::SoftBreak);
    assert_eq!(ops[5], TextOp::NoBreakSpace);
    assert!(matches!(&ops[7], TextOp::Text { text, .. } if text.as_ref() == " E "));
}

#[test]
fn overrides_unknown_drawing_and_animated_tags_remain_ordered_unsupported() {
    let parsed = parse(r"{\mystery42(1,2)\t(0,100,\fs40)\p1}A");
    let ops = parsed.track().events()[0].program();
    assert_eq!(ops.len(), 4);
    assert!(
        matches!(&ops[0], TextOp::Unsupported { name, source } if name.as_ref() == "mystery42" && source.slice(parsed.track().source()) == Some(r"\mystery42(1,2)"))
    );
    assert!(
        matches!(&ops[1], TextOp::Unsupported { name, source } if name.as_ref() == "t" && source.slice(parsed.track().source()) == Some(r"\t(0,100,\fs40)"))
    );
    assert!(matches!(&ops[2], TextOp::Unsupported { name, .. } if name.as_ref() == "p1"));
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|d| d.code() == DiagnosticCode::UnsupportedFeature)
            .count(),
        3
    );
}

#[test]
fn overrides_invalid_parameters_diagnose_without_setting_state() {
    for tag in [
        r"\fs0",
        r"\fsNaN",
        r"\fs1e2",
        r"\fscx-1",
        r"\an0",
        r"\pos(1,2,3)",
        r"\c&H1234567&",
        r"\alpha&H123&",
        r"\fn",
        r"\b2",
        r"\i2",
        r"\pos(1e2,0)",
        r"\fs",
        r"\i",
    ] {
        let input = script(&format!("{{{tag}}}A"));
        let parsed = parse_ass(
            input.as_bytes(),
            &ParseOptions::default(),
            &mut TrackIngestor::new(),
        )
        .unwrap();
        assert_eq!(parsed.track().events()[0].program().len(), 1, "{tag}");
        assert!(
            parsed
                .diagnostics()
                .iter()
                .any(|d| d.code() == DiagnosticCode::MalformedRecord),
            "{tag}"
        );
        let strict = ParseOptions::new(
            InputEncoding::Utf8,
            Profile::CoreV1,
            Limits::default(),
            SyntaxPolicy::Strict,
        );
        assert_eq!(
            parse_ass(input.as_bytes(), &strict, &mut TrackIngestor::new())
                .unwrap_err()
                .code(),
            ErrorCode::InvalidScript,
            "{tag}"
        );
    }
}

#[test]
fn overrides_unterminated_core_block_consumes_remainder_and_nesting_is_bounded() {
    let parsed = parse(r"A{\fs20 trailing");
    assert_eq!(parsed.track().events()[0].program().len(), 1);
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|d| d.detail().contains("unterminated Core override"))
    );
    let input = script(r"{\t(((((((((1)))))))))}A");
    assert_eq!(
        parse_ass(
            input.as_bytes(),
            &ParseOptions::default(),
            &mut TrackIngestor::new()
        )
        .unwrap_err()
        .code(),
        ErrorCode::LimitExceeded
    );
    let limits = Limits::default().with_text_bytes_per_event(3).unwrap();
    let options = ParseOptions::new(
        InputEncoding::Utf8,
        Profile::CoreV1,
        limits,
        SyntaxPolicy::Recover,
    );
    assert_eq!(
        parse_ass(
            script(r"A\NB").as_bytes(),
            &options,
            &mut TrackIngestor::new()
        )
        .unwrap_err()
        .code(),
        ErrorCode::LimitExceeded
    );
}

#[test]
fn overrides_every_byte_truncation_and_seeded_malformed_input_never_panics() {
    let input = script(r"日{\pos(-2,3)\fs20\unknown(\fs99)}é\N{\alpha&H80&}");
    for length in 0..=input.len() {
        let _ = parse_ass(
            &input.as_bytes()[..length],
            &ParseOptions::default(),
            &mut TrackIngestor::new(),
        );
    }
    let alphabet = ['{', '}', '\\', '(', ')', ',', 'N', 'h', '日', '0', '-', '&'];
    let mut seed = 1_u32;
    for _ in 0..256 {
        let mut text = String::new();
        for _ in 0..64 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let index = usize::try_from(seed % 12).unwrap();
            text.push(alphabet[index]);
        }
        let _ = parse_ass(
            script(&text).as_bytes(),
            &ParseOptions::default(),
            &mut TrackIngestor::new(),
        );
    }
}

#[test]
fn overrides_preserve_declared_named_resets_including_r_prefix_collisions() {
    for name in ["Named", "nd", "ndx", "nd42", "nd(1)", "日本語 Style"] {
        let input = format!(
            "[V4+ Styles]\nFormat: Name,Fontsize\nStyle: Default,20\nStyle: {name},24\n{}",
            script(&format!("{{\\r{name}\\r}}A"))
        );
        let parsed = parse_ass(
            input.as_bytes(),
            &ParseOptions::default(),
            &mut TrackIngestor::new(),
        )
        .unwrap();
        let ops = parsed.track().events()[0].program();
        assert!(
            matches!(&ops[0], TextOp::Override(OverrideOp::ResetStyle { name: Some(actual), .. }) if actual.as_ref() == name)
        );
        assert!(matches!(
            &ops[1],
            TextOp::Override(OverrideOp::ResetStyle { name: None, .. })
        ));
        let TextOp::Override(op) = &ops[0] else {
            panic!("missing named reset")
        };
        assert_eq!(
            op.source().slice(parsed.track().source()),
            Some(format!("\\r{name}").as_str())
        );
    }
}

#[test]
fn overrides_escape_provenance_is_original_event_source_not_decoded_text() {
    let input = format!("\u{feff}{}", script("é\\N日\\nX\\hY")).replace('\n', "\r\n");
    let parsed = parse_ass(
        input.as_bytes(),
        &ParseOptions::default(),
        &mut TrackIngestor::new(),
    )
    .unwrap();
    let track = parsed.track();
    let event = &track.events()[0];
    let ops = event.program();
    assert_eq!(track.source(), input);
    assert_eq!(ops[1], TextOp::HardBreak);
    assert_eq!(ops[3], TextOp::SoftBreak);
    assert_eq!(ops[5], TextOp::NoBreakSpace);
    assert!(
        event
            .source()
            .slice(track.source())
            .unwrap()
            .ends_with("é\\N日\\nX\\hY")
    );
    for operation in ops {
        if let TextOp::Text { text, source } = operation {
            assert_eq!(source.slice(track.source()), Some(text.as_ref()));
            assert!(!text.contains(['\n', '\r']));
        }
    }
}

#[test]
fn overrides_unresolved_r_prefixes_remain_unsupported_without_executing_children() {
    for syntax in [SyntaxPolicy::Recover, SyntaxPolicy::Strict] {
        let options = ParseOptions::new(
            InputEncoding::Utf8,
            Profile::CoreV1,
            Limits::default(),
            syntax,
        );
        let input =
            script(r"{\rnd\rnd30\rndx1\rndy2\rndz3\rndsABC\rMissing\rnd(1,\fs99\i1)}A{\rDefault}B");
        let parsed = parse_ass(input.as_bytes(), &options, &mut TrackIngestor::new()).unwrap();
        let track = parsed.track();
        let ops = track.events()[0].program();
        let tags = [
            r"\rnd",
            r"\rnd30",
            r"\rndx1",
            r"\rndy2",
            r"\rndz3",
            r"\rndsABC",
            r"\rMissing",
            r"\rnd(1,\fs99\i1)",
        ];
        assert_eq!(ops.len(), tags.len() + 3);
        for (operation, raw) in ops.iter().zip(tags) {
            assert!(
                matches!(operation, TextOp::Unsupported { source, .. } if source.slice(track.source()) == Some(raw))
            );
        }
        assert!(matches!(&ops[8], TextOp::Text { text, .. } if text.as_ref() == "A"));
        assert!(
            matches!(&ops[9], TextOp::Override(OverrideOp::ResetStyle { name: Some(name), .. }) if name.as_ref() == "Default")
        );
        assert!(matches!(&ops[10], TextOp::Text { text, .. } if text.as_ref() == "B"));
        assert_eq!(
            parsed
                .diagnostics()
                .iter()
                .filter(|d| d.code() == DiagnosticCode::UnsupportedFeature)
                .count(),
            8
        );
        assert!(ops.iter().all(|op| !matches!(
            op,
            TextOp::Override(OverrideOp::SetSize { .. } | OverrideOp::SetItalic { .. })
        )));
    }
}

#[test]
fn overrides_reset_matching_uses_final_exact_style_names_and_retains_unknown_names() {
    let input = format!(
        "{}\n[V4+ Styles]\nFormat: Name,Fontsize\nStyle: nd30,20\nStyle: nd30,40\nStyle: 日本語 Style,24",
        script(r"{\rnd30\rND30\r日本語 Style\rDefault\r}X")
    );
    let parsed = parse_ass(
        input.as_bytes(),
        &ParseOptions::default(),
        &mut TrackIngestor::new(),
    )
    .unwrap();
    let track = parsed.track();
    let ops = track.events()[0].program();
    assert!(
        matches!(&ops[0], TextOp::Override(OverrideOp::ResetStyle { name: Some(name), .. }) if name.as_ref() == "nd30")
    );
    assert!(
        matches!(&ops[1], TextOp::Unsupported { name, source } if name.as_ref() == "rND30" && source.slice(track.source()) == Some(r"\rND30"))
    );
    assert!(
        matches!(&ops[2], TextOp::Override(OverrideOp::ResetStyle { name: Some(name), .. }) if name.as_ref() == "日本語 Style")
    );
    assert_eq!(
        track
            .styles()
            .iter()
            .filter(|s| s.name() == "nd30")
            .map(|s| s.font_size().get())
            .collect::<Vec<_>>(),
        [20.0, 40.0]
    );
    assert!(
        parsed
            .diagnostics()
            .iter()
            .any(|d| d.code() == DiagnosticCode::DuplicateStyle)
    );
}
