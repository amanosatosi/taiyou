use taiyou_render::renderer::*;

fn parse(text: &str) -> ParsedTrack {
    let source = format!("[Script Info]\nPlayResX: 160\nPlayResY: 100\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow\nStyle: Default,Source Sans 3,30,0,0\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:02.00,{{\\pos(80,60)}}{text}\n");
    taiyou_ass::parse_ass(source.as_bytes(), &ParseOptions::new(InputEncoding::Utf8, Profile::CoreV1, Limits::default(), SyntaxPolicy::Strict), &mut TrackIngestor::new()).unwrap()
}
#[test]
fn drawing_commands_metrics_scales_baseline_and_malformed_tails() {
    use taiyou_render::drawing;
    let limits = Limits::default();
    let one = drawing::parse("m 0 0 l 100 0 100 100 0 100",1,0,&limits).unwrap();
    assert_eq!((one.width.get(),one.ascent.get(),one.descent.get()),(100.0,100.0,0.0));
    let half = drawing::parse("m 0 0 l 100 0 100 100 0 100",2,20,&limits).unwrap();
    assert_eq!((half.width.get(),half.ascent.get(),half.descent.get()),(50.0,40.0,10.0));
    for input in ["", "l 0 0 10 10", "n 0 0 l 10 10"] {
        assert!(drawing::parse(input,1,0,&limits).unwrap().outline.ops().is_empty());
    }
    let paths = drawing::parse("m 0 0 l 10 0 n 20 0 l 20 10 m 30 0 b 31 10 39 10 40 0 m 50 0 s 55 10 60 10 65 0 p 70 -10 c",1,0,&limits).unwrap();
    assert!(paths.outline.ops().iter().filter(|p| matches!(p,PathOp::CubicTo(..))).count() >= 4);
    assert!(drawing::parse("m 0 0 l 1 1 bad tail",1,0,&limits).is_ok());
    assert_eq!(drawing::parse("m 1e100 0",1,0,&limits).err().unwrap().code(),ErrorCode::LimitExceeded);
    let small = limits.clone().with_path_segments_per_frame(2).unwrap();
    assert_eq!(drawing::parse("m 0 0 l 10 0 10 10",1,0,&small).err().unwrap().code(),ErrorCode::LimitExceeded);
    let p = parse(r"{\p1}m 0 0 l 20 0 20 20 0 20{\p0}Ag");
    let e = taiyou_render::evaluate(p.track(),p.track().events()[0].id(),TimeUs::from_micros(0),Profile::CoreV1).unwrap();
    assert!(e.runs()[0].drawing().is_some());
    assert!(e.runs()[1].drawing().is_none());
    assert_eq!(e.runs()[0].decoded().unwrap().original_ranges(&limits).unwrap()[0].slice(p.track().source()),Some("m 0 0 l 20 0 20 20 0 20"));
}

#[test]
fn explicit_oversampling_budget_preserves_default_and_enforces_512() {
    use taiyou_render::effects::BlurSpec;
    let blur = FiniteF64::new(32.0).unwrap();
    let default = Limits::default();
    assert_eq!(default.filter_radius(),256);
    assert_eq!(BlurSpec::new(FiniteF64::new(200.0).unwrap(),FiniteF64::ZERO,[1.0,1.0],&default).err().unwrap().code(),ErrorCode::LimitExceeded);
    assert_eq!(BlurSpec::new(blur,FiniteF64::ZERO,[8.0,1.0],&default).err().unwrap().code(),ErrorCode::LimitExceeded);
    let adapter = default.clone().with_filter_radius(512);
    assert!(BlurSpec::new(blur,FiniteF64::ZERO,[8.0,1.0],&adapter).is_ok());
    assert_eq!(BlurSpec::new(FiniteF64::new(64.0).unwrap(),FiniteF64::ZERO,[8.0,1.0],&adapter).err().unwrap().code(),ErrorCode::LimitExceeded);
    assert_eq!(adapter.transient_bytes(),default.transient_bytes());
    let viewport = Viewport::full(Size::new(1280,100)).unwrap().with_original_size(Size::new(160,100)).unwrap();
    assert_eq!(viewport.target_scale(),[8.0,1.0]);
}

#[cfg(feature = "native")]
#[test]
fn drawings_use_normal_masks_effects_and_deterministic_seeking() {
    let mut context = FontContext::new().unwrap();
    let empty = FontCatalogBuilder::new(Limits::default()).snapshot(&mut CatalogIngestor::new()).unwrap();
    let mut builder = FontCatalogBuilder::new(Limits::default());
    builder.register(&mut context,&std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap()).unwrap(),0).unwrap();
    let fonts = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let request = |ms| RenderRequest { now:TimeUs::from_micros(ms*1000),viewport:Viewport::full(Size::new(160,100)).unwrap(),unsupported:UnsupportedPolicy::Strict };
    let input = r"{\p1}m 0 0 l 20 0 20 20 0 20";
    let baseline = Renderer::new(RendererConfig::default()).unwrap().render(parse(input).track().clone(),empty.clone(),&request(500)).unwrap();
    assert!(baseline.pixels().iter().any(|b| *b!=0));
    let half = Renderer::new(RendererConfig::default()).unwrap().render(parse(r"{\p2}m 0 0 l 40 0 40 40 0 40").track().clone(),empty.clone(),&request(500)).unwrap();
    assert_eq!(baseline.pixels(),half.pixels());
    for effects in [r"\blur2",r"\be2",r"\bord3",r"\shad3\4a&H00&",r"\bord2\shad3\blur2\be1"] {
        let p = parse(&format!("{{{effects}}}{input}"));
        let frame = Renderer::new(RendererConfig::default()).unwrap().render(p.track().clone(),empty.clone(),&request(500)).unwrap();
        assert_ne!(frame.pixels(),baseline.pixels(),"{effects}");
    }
    let mixed = parse(r"Ag{\p1\blur2}m 0 0 l 20 0 20 20 0 20{\p0}Ag");
    let mut renderer = Renderer::new(RendererConfig::default()).unwrap();
    let original = renderer.render(mixed.track().clone(),fonts.clone(),&request(500)).unwrap();
    for time in [1500,0,500,1000,500] {
        let actual = renderer.render(mixed.track().clone(),fonts.clone(),&request(time)).unwrap();
        assert_eq!(original.pixels(),actual.pixels());
    }
}
