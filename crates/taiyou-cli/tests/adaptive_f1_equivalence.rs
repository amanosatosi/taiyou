#![cfg(feature = "native")]
use taiyou_render::renderer::*;

#[path = "../../taiyou-render/src/renderer/f1_fixture.rs"]
mod fixture;

#[test]
fn internal_f1_snapshot_is_identical_to_the_original_ass_parser_output() {
    let expected = fixture::track();
    let parsed = taiyou_ass::parse_ass(
        expected.source().as_bytes(),
        &ParseOptions::new(
            InputEncoding::Utf8,
            Profile::CoreV1,
            Limits::default(),
            SyntaxPolicy::Strict,
        ),
        &mut TrackIngestor::new(),
    )
    .unwrap();
    let actual = parsed.track();
    assert_eq!(actual.script(), expected.script());
    assert_eq!(actual.styles(), expected.styles());
    assert_eq!(actual.events().len(), expected.events().len());
    for (a, b) in actual.events().iter().zip(expected.events()) {
        assert_eq!(a.id(), b.id());
        assert_eq!(a.start(), b.start());
        assert_eq!(a.end(), b.end());
        assert_eq!(a.layer(), b.layer());
        assert_eq!(a.style(), b.style());
        assert_eq!(a.source(), b.source());
        assert_eq!(a.text_source(), b.text_source());
        assert_eq!(a.program(), b.program());
    }
}
