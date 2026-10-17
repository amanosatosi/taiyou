#![cfg(feature = "native")]
use std::{sync::Arc, path::PathBuf};
use taiyou_font::*;
use taiyou_types::*;

fn limits() -> Limits { Limits::default().with_transient_bytes(512 * 1024 * 1024).unwrap() }
fn fixture(id: &str) -> Vec<u8> {
    let suffix = if matches!(id, "arabic" | "indic") { ".ttf" } else { ".otf" };
    std::fs::read(PathBuf::from(std::env::var("RUNNER_TEMP").unwrap()).join("taiyou-shaping-fonts").join(format!("{id}{suffix}"))).unwrap()
}
fn catalog() -> (FontContext, Arc<ValidatedFontCatalog>, Vec<FontId>) {
    let mut ctx = FontContext::with_limits(limits()).unwrap(); let mut builder = FontCatalogBuilder::new(limits()); let mut ids = Vec::new();
    for id in ["latin", "arabic", "indic", "cjk"] { ids.push(builder.register(&mut ctx, &fixture(id), 0).unwrap()); }
    let catalog = builder.snapshot(&mut CatalogIngestor::new()).unwrap(); (ctx, catalog, ids)
}
fn style(family: &str) -> Style {
    Style::core_default().with_font(FontRequest::new(Arc::from(family), 400, false, false)).with_size(FiniteF64::new(1000.0).unwrap()).unwrap()
}
fn evaluated(map: Arc<SourceMappedText>, spans: &[(u32,u32,Style)]) -> EvaluatedEvent {
    let source = SourceSpan::new(map.original_source(), 0, map.original_source().len()).unwrap();
    let runs: Vec<_> = spans.iter().map(|(a,b,s)| EvaluatedRun::new(Arc::from(&map.text()[*a as usize..*b as usize]), source, s).with_decoded(map.decoded_span(*a..*b).unwrap()).unwrap()).collect();
    EvaluatedEvent::new(EventId::new(0), EventOrder::new(0,0), runs.into(), Placement::new(&spans[0].2, Margins::new(0,0,0), Anchor::Default), ChangeClass::default()).with_source_map(map).unwrap()
}
fn plain(text: &str, style: &Style) -> EvaluatedEvent {
    let source: Arc<str> = Arc::from(text); let span = SourceSpan::new(&source, 0, source.len()).unwrap();
    let map = Arc::new(SourceMappedText::from_pieces(source, &[TextSourcePiece::Literal(span)], WrapMode::new(2).unwrap(), &limits()).unwrap());
    evaluated(map, &[(0,text.len() as u32,style.clone())])
}
struct Expected { upem: f64, metrics: [i32;3], glyphs: Vec<[i64;6]> }
fn reference(id: &str) -> Expected {
    let contents = std::fs::read_to_string(std::env::var("TAIYOU_SHAPING_REFERENCES").unwrap()).unwrap();
    let mut lines = contents.lines();
    while let Some(line) = lines.next() {
        let header: Vec<_> = line.split_whitespace().collect();
        if header.first() == Some(&"case") && header[1] == id {
            let mut glyphs = Vec::new();
            for line in lines.by_ref() { if line == "end" { break; } let values: Vec<i64> = line.split_whitespace().skip(1).map(|n| n.parse().unwrap()).collect(); glyphs.push(values.try_into().unwrap()); }
            return Expected { upem: header[2].parse().unwrap(), metrics: [header[3].parse().unwrap(),header[4].parse().unwrap(),header[5].parse().unwrap()], glyphs };
        }
    }
    panic!("missing independently produced provisional reference {id}")
}
fn assert_reference(run: &ShapedRun, id: &str, size: f64) {
    let expected = reference(id); let scale = size / expected.upem;
    assert_eq!(run.glyphs().len(), expected.glyphs.len(), "{id}");
    let mut starts: Vec<_> = expected.glyphs.iter().map(|g| g[1] as u32).collect(); starts.push(run.decoded().end()); starts.sort_unstable(); starts.dedup();
    for (glyph, raw) in run.glyphs().iter().zip(&expected.glyphs) {
        assert_eq!(glyph.glyph(), raw[0] as u32, "{id}"); assert_eq!(glyph.cluster().start(), raw[1] as u32, "{id}");
        assert_eq!(glyph.cluster().end(), starts[starts.partition_point(|&p| p <= raw[1] as u32)], "{id}");
        assert_eq!([glyph.advance_x().get(), glyph.advance_y().get(), glyph.offset_x().get(), glyph.offset_y().get()], [raw[2] as f64*scale,-(raw[3] as f64)*scale,raw[4] as f64*scale,-(raw[5] as f64)*scale], "{id}");
    }
    assert_eq!(run.metrics().ascent().get(), f64::from(expected.metrics[0])*scale);
    assert_eq!(run.metrics().descent().get(), -f64::from(expected.metrics[1])*scale);
    assert_eq!(run.metrics().line_gap().get(), f64::from(expected.metrics[2])*scale);
}
#[test]
fn shaping_exact_provisional_native_sequences_metrics_and_advances() {
    let (mut ctx, catalog, _) = catalog();
    for (id,text,family,language) in [("fi","fi","Source Sans 3","en"),("office","office AV","Source Sans 3","en"),("combining","a\u{301}","Source Sans 3","en"),("arabic","السَّلَامُ","Noto Naskh Arabic","ar"),("indic","किरण क्षि","Noto Sans Devanagari","hi"),("cjk","日本語","Noto Sans CJK JP","ja")] {
        let event = plain(text,&style(family)); let options = ShapeOptions::default().with_language(language).unwrap();
        let result = shape_event(&mut ctx,&catalog,&event,&options,&limits()).unwrap();
        assert!(result.diagnostics().is_empty(),"{id}"); assert_eq!(result.runs().len(),1,"{id}"); assert_reference(&result.runs()[0],id,1000.0);
    }
}
#[test]
fn shaping_kerning_ligatures_spacing_and_design_unit_scaling() {
    let (mut ctx,catalog,_) = catalog();
    let s = style("Source Sans 3");
    let shaped = |ctx: &mut FontContext, text: &str, s: &Style| shape_event(ctx,&catalog,&plain(text,s),&ShapeOptions::default().with_language("en").unwrap(),&limits()).unwrap();
    let fi=shaped(&mut ctx,"fi",&s); let no=shaped(&mut ctx,"fi",&s.clone().with_spacing(FiniteF64::new(1.0).unwrap()));
    assert_reference(&fi.runs()[0],"fi",1000.0); assert_reference(&no.runs()[0],"spacing",1000.0); assert!(fi.runs()[0].glyphs().len()<no.runs()[0].glyphs().len());
    let av=shaped(&mut ctx,"AV",&s); let a=shaped(&mut ctx,"A",&s); let v=shaped(&mut ctx,"V",&s);
    assert_reference(&av.runs()[0],"kern",1000.0); assert_reference(&a.runs()[0],"A",1000.0); assert_reference(&v.runs()[0],"V",1000.0);
    let width=|r: &ShapedEvent| r.runs().iter().flat_map(|r|r.glyphs()).map(|g|g.advance_x().get()).sum::<f64>(); assert!(width(&av)<width(&a)+width(&v));
    assert_reference(&shaped(&mut ctx,"fi",&s.with_size(FiniteF64::new(20.0).unwrap()).unwrap()).runs()[0],"fi",20.0);
}
#[test]
fn shaping_noncontiguous_ligature_and_combining_paint_provenance() {
    let (mut ctx,catalog,_) = catalog();
    for (source, right, id) in [("f{\\c&HFF0000&}i","i","fi"),("a{\\c&HFF0000&}\u{301}","\u{301}","combining")] {
        let a=SourceSpan::new(source,0,1).unwrap(); let b=SourceSpan::new(source,source.find(right).unwrap(),source.len()).unwrap();
        let map=Arc::new(SourceMappedText::from_pieces(Arc::from(source),&[TextSourcePiece::Literal(a),TextSourcePiece::Literal(b)],WrapMode::new(2).unwrap(),&limits()).unwrap());
        let first=style("Source Sans 3"); let second=first.clone().with_colors(Rgba8::new(255,0,0,255),first.secondary(),first.outline(),first.shadow_color());
        let event=evaluated(map.clone(),&[(0,1,first.clone()),(1,map.text().len() as u32,second)]);
        let result=shape_event(&mut ctx,&catalog,&event,&ShapeOptions::default().with_language("en").unwrap(),&limits()).unwrap();
        assert_eq!(result.runs().len(),1); assert_reference(&result.runs()[0],id,1000.0);
        let glyph=&result.runs()[0].glyphs()[0]; assert_eq!(glyph.cluster().original_ranges(&limits()).unwrap(),[a,b]); assert_eq!(result.paint_for(glyph.cluster()).unwrap(),&PaintSet::from_style(&first));
    }
}
#[test]
fn shaping_bidi_logical_order_and_arabic_fallback_with_context() {
    let (mut ctx,catalog,ids)=catalog(); let event=plain("abc سلام",&style("Source Sans 3"));
    let result=shape_event(&mut ctx,&catalog,&event,&ShapeOptions::default(),&limits()).unwrap();
    assert_eq!(result.runs().len(),2); assert_eq!(result.runs()[0].bidi_level(),0); assert_eq!(result.runs()[1].bidi_level(),1);
    assert_reference(&result.runs()[0],"mixed-latin",1000.0); assert_reference(&result.runs()[1],"mixed-arabic",1000.0);
    assert!(result.runs()[0].glyphs().iter().all(|g|g.font()==ids[0])); assert!(result.runs()[1].glyphs().iter().all(|g|g.font()==ids[1]));
    assert!(result.runs()[1].glyphs().windows(2).all(|g|g[0].cluster().start()>=g[1].cluster().start()));
}
#[test]
fn shaping_whole_cluster_fallback_and_explicit_notdef() {
    let (mut ctx,catalog,ids)=catalog();
    let result=shape_event(&mut ctx,&catalog,&plain("क्षि",&style("Source Sans 3")),&ShapeOptions::default(),&limits()).unwrap();
    assert!(result.runs().iter().flat_map(|r|r.glyphs()).all(|g|g.font()==ids[2]&&g.glyph()!=0)); assert!(result.missing().is_empty());
    let absent=shape_event(&mut ctx,&catalog,&plain("\u{10ffff}",&style("Source Sans 3")),&ShapeOptions::default(),&limits()).unwrap();
    assert!(absent.runs().iter().flat_map(|r|r.glyphs()).any(|g|g.glyph()==0)); assert!(!absent.missing().is_empty()); assert!(absent.diagnostics().iter().any(|d|d.code()==DiagnosticCode::MissingGlyph));
    assert_eq!(absent.missing()[0].decoded.original_ranges(&limits()).unwrap()[0].slice(absent.source_map().original_source()),Some("\u{10ffff}"));
}
#[test]
fn shaping_escapes_hard_breaks_and_owned_context_lifetime() {
    let result={
        let (mut ctx,catalog,_)=catalog(); let source="\u{feff}header\r\nf\\Nfi\\nA\\hV";
        let span=|s: &str| {let i=source.find(s).unwrap(); SourceSpan::new(source,i,i+s.len()).unwrap()};
        let pieces=[TextSourcePiece::Literal(span("f")),TextSourcePiece::HardBreak(span("\\N")),TextSourcePiece::Literal(span("fi")),TextSourcePiece::SoftBreak(span("\\n")),TextSourcePiece::Literal(span("A")),TextSourcePiece::NoBreakSpace(span("\\h")),TextSourcePiece::Literal(span("V"))];
        let map=Arc::new(SourceMappedText::from_pieces(Arc::from(source),&pieces,WrapMode::new(2).unwrap(),&limits()).unwrap()); let event=evaluated(map.clone(),&[(0,map.text().len() as u32,style("Source Sans 3"))]);
        shape_event(&mut ctx,&catalog,&event,&ShapeOptions::default(),&limits()).unwrap()
    };
    assert_eq!(result.breaks().len(),2); assert_eq!(result.breaks()[0].original_ranges(&limits()).unwrap()[0].slice(result.source_map().original_source()),Some("\\N")); assert_eq!(result.breaks()[1].original_ranges(&limits()).unwrap()[0].slice(result.source_map().original_source()),Some("\\n"));
    assert!(result.runs().iter().flat_map(|r|r.glyphs()).any(|g|g.cluster().original_ranges(&limits()).unwrap().iter().any(|s|s.slice(result.source_map().original_source())==Some("\\h"))));
}
#[test]
fn shaping_limits_unmappable_inputs_repeatability_and_foreign_maps() {
    let (mut ctx,catalog,_)=catalog(); let event=plain("office AV",&style("Source Sans 3")); let options=ShapeOptions::default();
    let a=shape_event(&mut ctx,&catalog,&event,&options,&limits()).unwrap(); let b=shape_event(&mut ctx,&catalog,&event,&options,&limits()).unwrap(); assert_eq!(a,b);
    for bound in [limits().with_transient_bytes(1).unwrap(),limits().with_text_bytes_per_event(1).unwrap(),limits().with_glyphs_per_event(1).unwrap()] { assert_eq!(shape_event(&mut ctx,&catalog,&event,&options,&bound).unwrap_err().code(),ErrorCode::LimitExceeded); }
    let foreign=plain("office AV",&style("Source Sans 3")); assert!(a.paint_for(foreign.runs()[0].decoded().unwrap()).is_err());
    let span=foreign.runs()[0].decoded().unwrap().clone(); let glyph=ShapedGlyph::new(catalog.faces()[0].blob().id(),1,span,GlyphPosition::new(FiniteF64::new(0.0).unwrap(),FiniteF64::new(0.0).unwrap(),FiniteF64::new(0.0).unwrap(),FiniteF64::new(0.0).unwrap()));
    assert!(ShapedRun::new(Arc::from([glyph]),a.runs()[0].metrics(),event.runs()[0].decoded().unwrap().clone(),0,u32::from_be_bytes(*b"Latn"),Arc::from("und")).is_err());
    assert!(ShapeOptions::default().with_language("en\0bad").is_err());
    let unmapped=EvaluatedEvent::new(event.id(),event.order(),Arc::from(event.runs()),event.placement().clone(),event.animation()); assert_eq!(shape_event(&mut ctx,&catalog,&unmapped,&options,&limits()).unwrap_err().code(),ErrorCode::InvalidArgument);
}
#[test]
fn shaping_collection_face_identity_uses_requested_weight() {
    let mut ctx=FontContext::with_limits(limits()).unwrap(); let mut builder=FontCatalogBuilder::new(limits());
    let bytes=std::fs::read(std::env::var("TAIYOU_TEST_COLLECTION").unwrap()).unwrap(); let a=builder.register(&mut ctx,&bytes,0).unwrap(); let b=builder.register(&mut ctx,&bytes,1).unwrap();
    let catalog=builder.snapshot(&mut CatalogIngestor::new()).unwrap(); let s=style("Source Sans 3").with_font(FontRequest::new(Arc::from("Source Sans 3"),700,false,false));
    let result=shape_event(&mut ctx,&catalog,&plain("fi",&s),&ShapeOptions::default().with_language("en").unwrap(),&limits()).unwrap(); assert_ne!(a,b); assert!(result.runs()[0].glyphs().iter().all(|g|g.font()==b)); assert_reference(&result.runs()[0],"collection",1000.0);
}
