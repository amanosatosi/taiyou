#![cfg(feature="native")]
use std::{path::PathBuf,sync::Arc};
use taiyou_font::*;
use taiyou_render::layout;
use taiyou_types::*;
fn f(v:f64)->FiniteF64 { FiniteF64::new(v).unwrap() }
fn limits()->Limits { Limits::default().with_transient_bytes(512*1024*1024).unwrap() }
fn catalog()->(FontContext,Arc<ValidatedFontCatalog>) {
    let mut context=FontContext::with_limits(limits()).unwrap(); let mut builder=FontCatalogBuilder::new(limits());
    for id in ["latin","arabic","indic","cjk"] {
        let suffix=if matches!(id,"arabic"|"indic"){ ".ttf" }else{".otf"};
        let path=PathBuf::from(std::env::var("RUNNER_TEMP").unwrap()).join("taiyou-shaping-fonts").join(format!("{id}{suffix}"));
        builder.register(&mut context,&std::fs::read(path).unwrap(),0).unwrap();
    }
    (context,builder.snapshot(&mut CatalogIngestor::new()).unwrap())
}
fn style()->Style { Style::core_default().with_font(FontRequest::new(Arc::from("Source Sans 3"),400,false,false)).with_size(f(20.0)).unwrap() }
fn event(text:&str,style:&Style)->EvaluatedEvent {
    let source:Arc<str>=Arc::from(text); let span=SourceSpan::new(&source,0,source.len()).unwrap();
    let map=Arc::new(SourceMappedText::from_pieces(source,&[TextSourcePiece::Literal(span)],WrapMode::new(2).unwrap(),&limits()).unwrap());
    EvaluatedEvent::new(EventId::new(0),EventOrder::new(0,0),Arc::from([EvaluatedRun::new(Arc::from(text),span,style).with_decoded(map.decoded_span(0..text.len() as u32).unwrap()).unwrap()]),Placement::new(style,Margins::new(0,0,0),Anchor::Default),ChangeClass::default()).with_terminal_style(style.clone()).with_source_map(map).unwrap()
}
fn shaped(ctx:&mut FontContext,catalog:&ValidatedFontCatalog,text:&str)->ShapedEvent { shape_event(ctx,catalog,&event(text,&style()),&ShapeOptions::default(),&limits()).unwrap() }
fn constraints()->LayoutConstraints { LayoutConstraints::new(Alignment::new(7).unwrap()).with_limits(limits()) }
#[test]
fn layout_fixed_font_real_empty_lines_and_owned_lifetime() {
    let layout={
        let (mut context,catalog)=catalog();
        let s=shaped(&mut context,&catalog,"\nA\n\nB\n");
        layout(&s,&constraints()).unwrap()
    };
    assert_eq!(layout.lines().len(),5);
    let descent=326.0*(20.0/1000.0); let height=20.0+descent;
    for (i,line) in layout.lines().iter().enumerate() {
        assert_eq!(line.ascent().get(),20.0); assert_eq!(line.descent().get(),descent);
        assert!((line.baseline().y().get()-(20.0+i as f64*height)).abs()<1e-12);
    }
    assert!((layout.logical_bounds().height()-5.0*height).abs()<1e-12);
    for i in [0,2,4] { assert_eq!(layout.lines()[i].ink_bounds(),None); assert!(layout.lines()[i].glyph_range().is_empty()); }
    assert!(layout.ink_bounds().is_some());
    for glyph in layout.glyphs() { assert!(!glyph.cluster().original_ranges(&limits()).unwrap().is_empty()); }
}
#[test]
fn layout_fixed_font_space_advance_and_real_ink_are_separate() {
    let (mut ctx,catalog)=catalog();
    let s=shaped(&mut ctx,&catalog,"   \u{a0}");
    let l=layout(&s,&constraints()).unwrap();
    let advance=s.runs().iter().flat_map(|r|r.glyphs()).map(|g|g.advance_x().get()).sum::<f64>();
    assert_eq!(l.lines()[0].advance().get(),advance); assert!(advance>0.0);
    assert_eq!(l.ink_bounds(),None);
    let s=shaped(&mut ctx,&catalog,"Aj\u{301}"); let l=layout(&s,&constraints()).unwrap();
    let mut bounds:Option<ScriptBounds>=None;
    for position in l.glyphs() {
        let glyph=&s.runs()[position.run_index()].glyphs()[position.glyph_index()];
        if let GlyphInk::Bounds(ink)=glyph.ink() {
            let b=ink.translated(position.position().x().get(),position.position().y().get()).unwrap();
            bounds=Some(bounds.map_or(b,|a|a.union(b).unwrap()));
        } else { assert_eq!(glyph.ink(),GlyphInk::Empty); }
    }
    assert_eq!(l.ink_bounds(),bounds); assert!(bounds.is_some());
    assert_ne!(l.ink_bounds(),Some(l.logical_bounds()));
}
#[test]
fn layout_fixed_font_mixed_bidi_and_trailing_whitespace_l1() {
    let (mut ctx,catalog)=catalog();
    for text in ["سلام abc ","abc سلام "] {
        let s=shaped(&mut ctx,&catalog,text); let l=layout(&s,&constraints()).unwrap();
        if text.starts_with('س') {
            assert_eq!(l.glyphs()[0].cluster().start(),text.len() as u32-1);
            assert_eq!(l.glyphs()[1].cluster().text(),"a");
        } else { assert_eq!(l.glyphs().last().unwrap().cluster().start(),text.len() as u32-1); }
        for r in 0..s.runs().len() {
            let native:Vec<_>=l.glyphs().iter().filter(|g|g.run_index()==r).map(|g|g.glyph_index()).collect();
            assert_eq!(native,(0..s.runs()[r].glyphs().len()).collect::<Vec<_>>());
        }
        assert_eq!(l,layout(&s,&constraints()).unwrap());
    }
}
#[test]
fn layout_fixed_font_maximal_metrics_include_all_fallback_faces() {
    let (mut ctx,catalog)=catalog(); let s=shaped(&mut ctx,&catalog,"A日本語कि");
    let l=layout(&s,&constraints()).unwrap();
    assert!(s.missing().is_empty());
    assert_eq!(l.lines()[0].ascent().get(),1160.0*(20.0/1000.0));
    assert_eq!(l.lines()[0].descent().get(),408.0*(20.0/1000.0));
    assert!(l.glyphs().iter().any(|g|s.runs()[g.run_index()].glyphs()[g.glyph_index()].font()==catalog.faces()[3].blob().id()));
}
#[test]
fn layout_fixed_font_paint_does_not_change_geometry() {
    let (mut ctx,catalog)=catalog(); let a=style();
    let b=a.clone().with_colors(Rgba8::new(9,8,7,255),a.secondary(),a.outline(),a.shadow_color());
    let s=shape_event(&mut ctx,&catalog,&event("office AV",&a),&ShapeOptions::default(),&limits()).unwrap();
    let t=shape_event(&mut ctx,&catalog,&event("office AV",&b),&ShapeOptions::default(),&limits()).unwrap();
    assert!(layout(&s,&constraints()).unwrap().geometry_eq(&layout(&t,&constraints()).unwrap()));
}
