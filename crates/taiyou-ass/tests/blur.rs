use taiyou_types::*;
fn parsed(tags: &str) -> ParsedTrack {
    let input =
        format!("[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:02.00,{{{tags}}}A\n");
    taiyou_ass::parse_ass(
        input.as_bytes(),
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
#[test]
fn typed_real_values_prefixes_parentheses_and_resets_keep_source_order() {
    let parsed = parsed(r"\blur1.5tail\be2.25\blur-5\blur\be\blur(2)\blur2(foo)\be-0.5\blur1e1");
    let ops = parsed.track().events()[0].program();
    let blur = |i: usize, expected: Option<f64>| {
        let TextOp::Override(OverrideOp::SetBlur { value, source }) = &ops[i] else {
            panic!("not blur")
        };
        assert_eq!(value.map(FiniteF64::get), expected);
        assert!(
            source
                .slice(parsed.track().source())
                .unwrap()
                .starts_with(r"\blur")
        );
    };
    blur(0, Some(1.5));
    blur(2, Some(-5.0));
    blur(3, None);
    blur(5, None);
    blur(6, Some(2.0));
    blur(8, Some(10.0));
    assert!(
        matches!(&ops[1], TextOp::Override(OverrideOp::SetBe { value: Some(value), .. }) if value.get() == 2.25)
    );
    assert!(matches!(
        &ops[4],
        TextOp::Override(OverrideOp::SetBe { value: None, .. })
    ));
    assert!(
        matches!(&ops[7], TextOp::Override(OverrideOp::SetBe { value: Some(value), .. }) if value.get() == -0.5)
    );
}
#[test]
fn animated_blur_forms_keep_outer_and_embedded_provenance() {
    for (tags, start, end, acceleration) in [
        (r"\t(\blur4\be2)", 0, 0, 1.0),
        (r"\t(2,\blur4\be2)", 0, 0, 2.0),
        (r"\t(10,100,\blur4\be2)", 10, 100, 1.0),
        (r"\t(10,100,2,\blur4\be2)", 10, 100, 2.0),
    ] {
        let track = parsed(tags);
        let ops = track.track().events()[0].program();
        assert_eq!(ops.len(), 5);
        let TextOp::Override(OverrideOp::BeginTransform { start_ms, end_ms, accel, source }) = &ops[0] else { panic!("missing typed transform") };
        assert_eq!((*start_ms, *end_ms, accel.get()), (start, end, acceleration));
        assert_eq!(source.slice(track.track().source()), Some(tags.split_once(r"\blur").unwrap().0));
        for (i, expected) in [(1, r"\blur4"), (2, r"\be2")] {
            let TextOp::Override(op) = &ops[i] else { panic!("missing child") };
            assert_eq!(op.source().slice(track.track().source()), Some(expected));
        }
        assert!(matches!(&ops[3], TextOp::Override(OverrideOp::EndTransform { source }) if source.slice(track.track().source()) == Some(")")));
        assert!(track.diagnostics().is_empty());
    }
}
