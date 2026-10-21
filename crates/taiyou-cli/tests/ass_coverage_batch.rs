use taiyou_render::renderer::*;

fn parsed(tags: &str) -> ParsedTrack {
    let ass = format!(
        "[Script Info]\nPlayResX: 120\nPlayResY: 120\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow\nStyle: Default,Source Sans 3,30,0,0\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:05.00,{{{tags}}}A\n"
    );
    taiyou_ass::parse_ass(
        ass.as_bytes(),
        &ParseOptions::new(
            InputEncoding::Utf8,
            Profile::CoreV1,
            Limits::default(),
            SyntaxPolicy::Strict,
        ),
        &mut TrackIngestor::new(),
    )
    .unwrap()
}
fn at(tags: &str, ms: i64) -> EvaluatedEvent {
    let p = parsed(tags);
    taiyou_render::evaluate(
        p.track(),
        EventId::new(0),
        TimeUs::from_micros(ms * 1000),
        Profile::CoreV1,
    )
    .unwrap()
}
#[test]
fn bare_extended_alpha_narrows_after_animation() {
    for (ms, opacity) in [(2040, 47), (2560, 250), (4960, 5)] {
        assert_eq!(
            at(r"\1a0\t(\1a1FF)", ms).runs()[0].paint().primary().a(),
            opacity
        );
    }
    assert_eq!(at(r"\1a&H100&", 0).runs()[0].paint().primary().a(), 255);
    assert_eq!(at(r"\1a-1", 0).runs()[0].paint().primary().a(), 0);
    let p = parsed(r"\1a0\t(\1a1FF)");
    assert!(p.track().events()[0].program().iter().any(|op| matches!(op,TextOp::Override(OverrideOp::SetAlpha{value:Some(511),source,..}) if source.slice(p.track().source())==Some(r"\1a1FF"))));
}
