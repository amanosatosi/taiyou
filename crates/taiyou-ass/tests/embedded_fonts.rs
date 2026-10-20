use taiyou_ass::parse_ass;
use taiyou_types::*;

fn parse(source: &str, syntax: SyntaxPolicy, limits: Limits) -> Result<ParsedTrack, ParseError> {
    parse_ass(
        source.as_bytes(),
        &ParseOptions::new(InputEncoding::Utf8, Profile::CoreV1, limits, syntax),
        &mut TrackIngestor::new(),
    )
}
#[test]
fn embedded_encoding_quartets_tails_multiple_sections_and_metadata_paths() {
    // Independent encoding vectors: [0], [0,255], [0,255,128].
    let input = "[Fonts]\nfontname: ../../outside.ttf\n!!\nfontname: C:\\outside.ttf\n!0]\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:01.00,A\n[Fonts]\nfontname: third\n!0_!\n";
    let parsed = parse(input, SyntaxPolicy::Strict, Limits::default()).unwrap();
    assert_eq!(parsed.embedded_fonts().len(), 3);
    assert_eq!(parsed.embedded_fonts()[0].name(), "../../outside.ttf");
    assert_eq!(parsed.embedded_fonts()[0].bytes(), &[0]);
    assert_eq!(parsed.embedded_fonts()[1].name(), "C:\\outside.ttf");
    assert_eq!(parsed.embedded_fonts()[1].bytes(), &[0, 255]);
    assert_eq!(parsed.embedded_fonts()[2].bytes(), &[0, 255, 128]);
    assert_eq!(parsed.track().events().len(), 1);
    for font in parsed.embedded_fonts() {
        assert!(font.source().slice(input).unwrap().starts_with("fontname:"));
    }
    // '[' is valid attachment data, not an arbitrary section declaration.
    assert!(
        parse(
            "[Fonts]\nfontname: x\n[!!!\n",
            SyntaxPolicy::Strict,
            Limits::default()
        )
        .is_ok()
    );
}
#[test]
fn malformed_attachments_recover_as_units_and_resource_failures_are_fatal() {
    for bad in ["", "!", "abc", "\u{80}"] {
        let input = format!("[Fonts]\nfontname: bad\n{bad}\nfontname: good\n!!!!\n");
        assert_eq!(
            parse(&input, SyntaxPolicy::Strict, Limits::default())
                .unwrap_err()
                .code(),
            ErrorCode::InvalidScript
        );
        let recovered = parse(&input, SyntaxPolicy::Recover, Limits::default()).unwrap();
        assert_eq!(recovered.embedded_fonts().len(), 1);
        assert_eq!(recovered.embedded_fonts()[0].name(), "good");
        assert!(
            recovered
                .diagnostics()
                .iter()
                .any(|d| d.code() == DiagnosticCode::MalformedRecord)
        );
    }
    let input = "[Fonts]\nfontname: a\n!!!!\nfontname: b\n!!!!\n";
    for policy in [SyntaxPolicy::Strict, SyntaxPolicy::Recover] {
        assert_eq!(
            parse(input, policy, Limits::default().with_font_bytes(5).unwrap())
                .unwrap_err()
                .code(),
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            parse(
                input,
                policy,
                Limits::default().with_transient_bytes(1).unwrap()
            )
            .unwrap_err()
            .code(),
            ErrorCode::LimitExceeded
        );
    }
}
#[test]
fn unsupported_ssa_styles_never_masquerade_as_missing_ass_fonts() {
    for policy in [SyntaxPolicy::Strict, SyntaxPolicy::Recover] {
        let error = parse(
            "[V4 Styles]\nFormat: Name, Fontname\nStyle: Default,Unknown",
            policy,
            Limits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code(), ErrorCode::UnsupportedFeature);
        assert!(error.span().is_some());
    }
}
