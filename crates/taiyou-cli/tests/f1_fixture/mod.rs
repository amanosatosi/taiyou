// Test-only typed snapshot of the existing fixed ASS workload. The CLI test
// compares it to actual parser output; renderer unit tests need no parser edge.
use std::sync::Arc;
use taiyou_types::*;

pub fn track() -> Arc<Track> {
    let source = include_str!(
        "../../../../experiments/t017/results/ae467c9/optimized-initial/adaptive-256-multiglyph.ass"
    );
    let text = "HHHHHHHHHHHHHHHHHHHHHHHHHHHHHHHH";
    assert!(source.contains("Style: Default,Source Sans 3,20,&H00FFFFFF,&HFF000000,0,0,7,13,21,9"));
    let style = Style::core_default()
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
        .with_colors(
            Rgba8::new(255, 255, 255, 255),
            Rgba8::new(255, 255, 0, 255),
            Rgba8::new(0, 0, 0, 255),
            Rgba8::new(0, 0, 0, 0),
        )
        .with_alignment(Alignment::new(7).unwrap())
        .with_margins(Margins::new(13, 21, 9));
    let events: Vec<_> = source.match_indices("Dialogue:").enumerate().map(|(id,(offset,_))| {
        let line = source[offset..].split('\n').next().unwrap().trim_end_matches('\r');
        assert_eq!(line, format!("Dialogue: 0,0:00:00.00,0:00:02.00,Default,{{\\fs5\\bord0.5\\shad1.25\\4a&H00&}}{text}"));
        let span = |tag: &str| { let start=offset+line.find(tag).unwrap(); SourceSpan::new(source,start,start+tag.len()).unwrap() };
        let text_span=span(text);
        let program = Arc::from([
            TextOp::Override(OverrideOp::SetSize { size: FiniteF64::new(5.0).unwrap(), source: span(r"\fs5") }),
            TextOp::Override(OverrideOp::SetBorder { width: FiniteF64::new(0.5).unwrap(), source: span(r"\bord0.5") }),
            TextOp::Override(OverrideOp::SetShadow { offset: FiniteF64::new(1.25).unwrap(), source: span(r"\shad1.25") }),
            TextOp::Override(OverrideOp::SetOpacity { channel: Some(PaintChannel::Shadow), opacity: 255, source: span(r"\4a&H00&") }),
            TextOp::Text { text: Arc::from(text), source: text_span },
        ]);
        Event::new(EventId::new(id as u32), TimeUs::from_micros(0), TimeUs::from_micros(2_000_000), StyleId::new(0), SourceSpan::new(source,offset,offset+line.len()).unwrap())
            .with_text_source(SourceSpan::new(source, offset+line.find('{').unwrap(), offset+line.len()).unwrap())
            .with_program(program)
    }).collect();
    assert_eq!(events.len(), 256);
    TrackIngestor::new()
        .ingest(
            Arc::from(source),
            ScriptInfo::core_default()
                .with_play_res(Size::new(160, 100))
                .unwrap()
                .with_wrap(WrapMode::new(2).unwrap()),
            Arc::from([style]),
            events.into(),
            &Limits::default(),
        )
        .unwrap()
}
