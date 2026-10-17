use taiyou_ass::parse_ass;
use taiyou_types::*;

const EVENTS: &str = "[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n";
fn parse(text: &str) -> ParsedTrack {
    parse_ass(text.as_bytes(), &ParseOptions::default(), &mut TrackIngestor::new()).unwrap()
}
fn strict() -> ParseOptions {
    ParseOptions::new(InputEncoding::Utf8, Profile::CoreV1, Limits::default(), SyntaxPolicy::Strict)
}
fn has(parsed: &ParsedTrack, code: DiagnosticCode) -> bool {
    parsed.diagnostics().iter().any(|d| d.code() == code)
}
fn literal(event: &Event) -> (&str, SourceSpan) {
    let TextOp::Text { text, source } = &event.program()[0];
    (text, *source)
}

#[test]
fn records_preserve_commas_whitespace_unicode_bom_and_line_endings() {
    for ending in ["\n", "\r\n", "\r"] {
        let input = format!("\u{feff}{}Dialogue: 4,1:02:03.45,1:02:04.00,Default, actor ,0,20,0, fx ,  héllo, 世界 {{\\b1}} ", EVENTS).replace('\n', ending);
        let parsed = parse(&input);
        let track = parsed.track();
        assert_eq!(track.source(), input);
        assert_eq!(track.events().len(), 1);
        let event = &track.events()[0];
        assert_eq!(event.id(), EventId::new(0));
        assert_eq!(event.layer(), 4);
        assert_eq!(event.start().as_micros(), 3_723_450_000);
        assert_eq!(event.actor(), "actor");
        assert_eq!(event.effect(), "fx");
        assert_eq!(event.margins(), Margins::new(0, 20, 0));
        let (text, span) = literal(event);
        assert_eq!(text, "  héllo, 世界 {\\b1} ");
        assert_eq!(span.slice(track.source()), Some(text));
        assert_eq!(usize::try_from(span.start()).unwrap(), input.find("  héllo").unwrap());
        assert!(event.source().slice(track.source()).unwrap().starts_with("Dialogue:"));
    }
}

#[test]
fn formats_are_dynamic_case_insensitive_and_updated_between_records() {
    let parsed = parse("[eVeNtS]\nfOrMaT: End, Unknown, Style, Start, Text\nDialogue: 0:00:02.00,ignored,Default,0:00:01.00,one,two\nFormat: Start, End, Text\nDialogue: 0:00:03.00,0:00:04.00,\n");
    assert_eq!(parsed.track().events().len(), 2);
    assert_eq!(literal(&parsed.track().events()[0]).0, "one,two");
    assert_eq!(literal(&parsed.track().events()[1]).0, "");
    assert_eq!(parsed.track().events()[0].start().as_micros(), 1_000_000);
}

#[test]
fn style_fields_and_exact_core_defaults_are_normalized() {
    let parsed = parse("[Script Info]\nPlayResX: 640\nPlayResY: 360\nLayoutResX: 1280\nLayoutResY: 720\nWrapStyle: 2\nScaledBorderAndShadow: yes\nYCbCr Matrix: TV.709\nCollisions: Reverse\nCustom: retained\n[V4+ Styles]\nFormat: Name, Fontname, Fontsize, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding, PrimaryColour\nStyle: Default,Some Font,30,-1,1,-1,0,150,50,-2,45,1,3,-4,9,0,11,12,1,&H800000FF\n");
    let track = parsed.track();
    let script = track.script();
    assert_eq!(script.play_res(), Size::new(640, 360));
    assert_eq!(script.layout_res(), Some(Size::new(1280, 720)));
    assert!(script.scaled_border_shadow());
    assert_eq!(script.ycbcr_matrix(), "TV.709");
    assert_eq!(script.collision_header(), Some("Reverse"));
    assert!(script.metadata().iter().any(|(k,v)| k.as_ref() == "Custom" && v.as_ref() == "retained"));
    let style = &track.styles()[0];
    assert_eq!(style.font().family(), "Some Font");
    assert_eq!(style.font().weight(), 700);
    assert!(style.font().italic());
    assert!(style.underline());
    assert!(!style.strikeout());
    assert_eq!(style.font_size().get(), 30.0);
    assert_eq!(style.scale_x().get(), 1.5);
    assert_eq!(style.scale_y().get(), 0.5);
    assert_eq!(style.spacing().get(), -2.0);
    assert_eq!(style.shadow().get(), -4.0);
    assert_eq!(style.primary().channels(), [255,0,0,127]);
    assert_eq!(style.secondary().channels(), [255,255,0,255]);
    assert_eq!(style.margins(), Margins::new(0,11,12));
    assert!(!has(&parsed, DiagnosticCode::MissingDefaultStyle));
}

#[test]
fn all_style_definitions_are_retained_and_last_definition_resolves_after_events() {
    let input = format!("{}Dialogue: 0,0:00:00.00,0:00:01.00,Later,,0,0,0,,A\nDialogue: 0,0:00:00.00,0:00:01.00,Missing,,0,0,0,,B\n[V4+ Styles]\nFormat: Fontsize, Name\nStyle: 10,Later\nStyle: 40,Later\n", EVENTS);
    let parsed = parse(&input);
    let track = parsed.track();
    assert_eq!(track.styles().len(), 3);
    assert_eq!(track.styles()[0].font_size().get(), 10.0);
    assert_eq!(track.events()[0].style(), StyleId::new(1));
    assert_eq!(track.styles()[1].font_size().get(), 40.0);
    assert_eq!(track.events()[1].style(), StyleId::new(2));
    assert!(has(&parsed, DiagnosticCode::DuplicateStyle));
    assert!(has(&parsed, DiagnosticCode::UnknownStyle));
    assert!(has(&parsed, DiagnosticCode::MissingDefaultStyle));
}

#[test]
fn event_source_order_survives_layers_times_and_malformed_records() {
    let input = format!("{}Dialogue: 9,0:00:10.00,0:00:11.00,Default,,0,0,0,,first\nDialogue: broken\nDialogue: -1,-0:00:01.00,0:00:00.00,Default,,0,0,0,,second\nDialogue: 0,0:00:03.00,0:00:02.00,Default,,0,0,0,,inactive\n", EVENTS);
    let parsed = parse(&input);
    let events = parsed.track().events();
    assert_eq!(events.iter().map(|e| e.id().ordinal()).collect::<Vec<_>>(), [0,2,3]);
    assert_eq!(events[0].layer(), 9);
    assert_eq!(events[1].layer(), -1);
    assert_eq!(events[1].start().as_micros(), -1_000_000);
    assert!(!events[2].is_active(TimeUs::from_micros(2_000_000)));
    assert!(has(&parsed, DiagnosticCode::MalformedRecord));
    assert!(has(&parsed, DiagnosticCode::InactiveEvent));
}

#[test]
fn invalid_formats_records_and_numbers_recover_at_record_boundaries() {
    for format in ["Start,End,Text,Style", "Start,Start,End,Text", "End,Text", "Start,,End,Text"] {
        let input = format!("[Events]\nFormat: {format}\nDialogue: 0:00:00.00,0:00:01.00,bad\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:01.00,good\n");
        let parsed = parse(&input);
        assert_eq!(parsed.track().events().len(), 1);
        assert_eq!(literal(&parsed.track().events()[0]).0, "good");
        assert!(parse_ass(input.as_bytes(), &strict(), &mut TrackIngestor::new()).is_err());
    }
    for bad in ["0:60:00.00", "0:00:60.00", "0:00:01.0", "1e2:00:00.00", "9999999999999999999999:00:00.00"] {
        let input = format!("[Events]\nFormat: Start,End,Text\nDialogue: {bad},0:00:01.00,bad\nDialogue: 0:00:00.00,0:00:01.00,good");
        let parsed = parse(&input);
        assert_eq!(parsed.track().events().len(), 1);
    }
    for bad in ["NaN", "Inf", "1e2", "1,5", "0", "-1", "20.", ".5"] {
        let input = format!("[V4+ Styles]\nFormat: Name,Fontsize\nStyle: Broken,{bad}\nStyle: Default,20");
        let parsed = parse(&input);
        assert_eq!(parsed.track().styles().len(), 1);
        assert_eq!(parsed.track().styles()[0].name(), "Default");
        assert!(parse_ass(input.as_bytes(), &strict(), &mut TrackIngestor::new()).is_err());
    }
}

#[test]
fn encoding_resolution_and_resource_errors_are_always_fatal() {
    assert_eq!(parse_ass(b"\xff", &ParseOptions::default(), &mut TrackIngestor::new()).unwrap_err().code(), ErrorCode::InvalidEncoding);
    for input in ["[Script Info]\nPlayResX: 640", "[Script Info]\nLayoutResY: 720"] {
        assert_eq!(parse_ass(input.as_bytes(), &ParseOptions::default(), &mut TrackIngestor::new()).unwrap_err().code(), ErrorCode::UnsupportedFeature);
    }
    for input in ["[Script Info]\nPlayResX: 0\nPlayResY: 100", "[Script Info]\nPlayResX: -1\nPlayResY: 100"] {
        assert_eq!(parse_ass(input.as_bytes(), &ParseOptions::default(), &mut TrackIngestor::new()).unwrap_err().code(), ErrorCode::InvalidScript);
    }
    let input = "[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:01.00,abc\nDialogue: 0:00:00.00,0:00:01.00,abc";
    for limits in [Limits::default().with_input_bytes(1).unwrap(), Limits::default().with_events(1).unwrap(), Limits::default().with_text_bytes_per_event(2).unwrap()] {
        let options = ParseOptions::new(InputEncoding::Utf8, Profile::CoreV1, limits, SyntaxPolicy::Recover);
        assert_eq!(parse_ass(input.as_bytes(), &options, &mut TrackIngestor::new()).unwrap_err().code(), ErrorCode::LimitExceeded);
    }
}

#[test]
fn unknown_sections_and_diagnostics_are_bounded_without_decoding_attachments() {
    let input = format!("{}[Fonts]\nfontname: ../file\nnot-a-real-attachment\n{}Dialogue: 0,0:00:00.00,0:00:01.00,Default,,0,0,0,,ok", "[Unknown]\n".repeat(1100), EVENTS);
    let parsed = parse(&input);
    assert_eq!(parsed.track().events().len(), 1);
    assert_eq!(parsed.diagnostics().len(), MAX_DIAGNOSTICS + 1);
    assert_eq!(parsed.diagnostics().last().unwrap().code(), DiagnosticCode::DiagnosticsSuppressed);
}

#[test]
fn parser_uses_the_supplied_identity_owner_and_failed_input_does_not_recycle() {
    let mut owner = TrackIngestor::new();
    let options = ParseOptions::default();
    let a = parse_ass(b"", &options, &mut owner).unwrap();
    assert!(parse_ass(b"\xff", &options, &mut owner).is_err());
    let b = parse_ass(b"", &options, &mut owner).unwrap();
    assert_eq!(a.track().revision().get(), 0);
    assert_eq!(b.track().revision().get(), 1);
    let legacy = ParseOptions::new(InputEncoding::Utf8, Profile::LegacyXyV1, Limits::default(), SyntaxPolicy::Recover);
    assert_eq!(parse_ass(b"", &legacy, &mut owner).unwrap_err().code(), ErrorCode::UnsupportedFeature);
}

#[test]
fn every_byte_truncation_is_bounded_and_never_panics() {
    let sample = format!("{}Dialogue: 0,0:00:00.00,0:00:01.00,Default,,0,0,0,,日,é{{\\pos(1,2)}}", EVENTS);
    for length in 0..=sample.len() {
        let _ = parse_ass(&sample.as_bytes()[..length], &ParseOptions::default(), &mut TrackIngestor::new());
    }
}
