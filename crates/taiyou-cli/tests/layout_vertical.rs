use taiyou_render::renderer::*;
fn parsed(header:&str,family:&str,body:&str)->ParsedTrack {
    let input=format!("[Script Info]\nPlayResX: 240\nPlayResY: 160\nWrapStyle: 2\n{header}\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow,Encoding\nStyle: Default,{family},40,0,0,1\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:05.00,{body}\n");
    taiyou_ass::parse_ass(input.as_bytes(),&ParseOptions::new(InputEncoding::Utf8,Profile::CoreV1,Limits::default(),SyntaxPolicy::Strict),&mut TrackIngestor::new()).unwrap()
}
#[test]
fn ignored_commands_and_prefix_recovery_preserve_karaoke_boundaries(){
    let p=parsed("","Source Sans 3",r"{\k118}{\board1\c&HFF9920&}cael{\i1}um{\mystery42}X");
    assert!(!p.track().events()[0].program().iter().any(|op|matches!(op,TextOp::Unsupported{..})));
    assert!(p.track().events()[0].program().iter().any(|op|matches!(op,TextOp::Override(OverrideOp::SetWeight{weight:400,source}) if source.slice(p.track().source())==Some(r"\board1"))));
    let e=taiyou_render::evaluate(p.track(),EventId::new(0),TimeUs::from_micros(500000),Profile::CoreV1).unwrap();
    assert_eq!(e.runs()[0].text(),"cael");
    assert_eq!(e.runs()[0].karaoke().unwrap().start_ms,0);
    assert_eq!(e.runs()[1].karaoke().unwrap().start_ms,1180);
    assert_eq!(e.runs()[2].karaoke().unwrap().start_ms,1180);
    for r in e.runs(){assert!(r.decoded().unwrap().belongs_to(e.source_map().unwrap()));}
}
#[cfg(feature="native")]
mod native {
    use super::*;
    use std::sync::Arc;
    fn fonts()->Arc<ValidatedFontCatalog>{
        let mut ctx=FontContext::new().unwrap();let mut b=FontCatalogBuilder::new(Limits::default());
        for name in ["latin.otf","cjk.otf","IPAexGothic.ttf"] {
            let p=std::path::PathBuf::from(std::env::var("RUNNER_TEMP").unwrap()).join("taiyou-shaping-fonts").join(name);
            b.register(&mut ctx,&std::fs::read(p).unwrap(),0).unwrap();
        }b.snapshot(&mut CatalogIngestor::new()).unwrap()
    }
    fn pixels(p:&ParsedTrack,c:Arc<ValidatedFontCatalog>,viewport:Viewport,ms:i64)->Vec<u8>{
        Renderer::new(Default::default()).unwrap().render_parsed(p,c,&RenderRequest{now:TimeUs::from_micros(ms*1000),viewport,unsupported:UnsupportedPolicy::Strict}).unwrap().pixels().to_vec()
    }
    #[test]
    fn vertical_family_metrics_variants_and_orientation_are_real(){
        let c=fonts();let mut ctx=FontContext::new().unwrap();
        let shape=|ctx:&mut FontContext,family:&str,text:&str|{
            let p=parsed("",family,text);
            let e=taiyou_render::evaluate(p.track(),EventId::new(0),TimeUs::from_micros(0),Profile::CoreV1).unwrap();
            taiyou_font::shape_event(ctx,&c,&e,&Default::default(),&Limits::default()).unwrap()
        };
        let h=shape(&mut ctx,"IPAexGothic","A日、");let v=shape(&mut ctx,"@IPAexGothic","A日、");
        let hg:Vec<_>=h.runs().iter().flat_map(|r|r.glyphs()).collect();let vg:Vec<_>=v.runs().iter().flat_map(|r|r.glyphs()).collect();
        assert_eq!(hg.len(),3);assert_eq!(vg.len(),3);
        assert_eq!(hg[0].font(),vg[0].font());assert_eq!(hg[0].glyph(),vg[0].glyph());assert_eq!(hg[0].position(),vg[0].position());assert_eq!(hg[0].ink(),vg[0].ink());
        assert_eq!(hg[1].font(),vg[1].font());assert_eq!(hg[1].glyph(),vg[1].glyph());
        assert_eq!(vg[1].vertical_offset(),Some(FontPoint::new(1802.0,-246.0).unwrap()));
        // Actual GDI TextOut/GetGlyphIndices probe, pinned IPA font, run 39682853492.
        assert_eq!(vg[2].glyph(),7473);assert_ne!(hg[2].glyph(),vg[2].glyph());
        let cff=shape(&mut ctx,"@Noto Sans CJK JP","A日");
        assert!(cff.runs().iter().flat_map(|r|r.glyphs()).all(|g|g.vertical_offset().is_some()));
        assert_eq!(vg[1].advance_y().get(),0.0);assert!(vg[1].advance_x().get()>0.0);
    }
    #[test]
    fn vertical_text_shares_placement_effects_clips_drawings_and_seek_pipeline(){
        let c=fonts();let vp=Viewport::full(Size::new(240,160)).unwrap();
        let plain=parsed("","IPAexGothic",r"{\pos(120,80)}日あA");
        let vertical=parsed("","@IPAexGothic",r"{\pos(120,80)}日あA");
        assert_ne!(pixels(&plain,c.clone(),vp,0),pixels(&vertical,c.clone(),vp,0));
        for tag in [r"\frz270",r"\org(0,0)\frz30",r"\frx30\fry20",r"\fax0.3\fay0.2",r"\bord2\shad3\blur1",r"\iclip(0,0,120,160)"] {
            let p=parsed("","@IPAexGothic",&format!(r"{{\pos(120,80){tag}}}日あA"));
            let raw=pixels(&p,c.clone(),vp,500);assert!(raw.chunks_exact(4).any(|p|p[3]>0),"{tag}");
        }
        let p=parsed("","@IPAexGothic",r"{\move(20,60,180,100)\frz270\1c&HFF&\2c&HFF0000&\kf100}日あ{\fnIPAexGothic}A{\r}日{\p1}m 0 0 l 10 0 10 10 0 10");
        let mut renderer=Renderer::new(Default::default()).unwrap();
        for ms in [0,500,1500,300,1000,4999,0] {
            let raw=renderer.render_parsed(&p,c.clone(),&RenderRequest{now:TimeUs::from_micros(ms*1000),viewport:vp,unsupported:UnsupportedPolicy::Strict}).unwrap();
            assert_eq!(raw.pixels(),pixels(&p,c.clone(),vp,ms));
        }
        assert_ne!(pixels(&p,c.clone(),vp,0),pixels(&p,c, vp,1500));
    }
    #[test]
    fn layout_storage_preserves_script_positions_but_changes_font_aspect_and_effect_scale(){
        let c=fonts();let vp=Viewport::full(Size::new(240,160)).unwrap();
        let base=parsed("","Source Sans 3",r"{\pos(120,80)}HH");
        let same=parsed("LayoutResX: 240\nLayoutResY: 160","Source Sans 3",r"{\pos(120,80)}HH");
        assert_eq!(pixels(&base,c.clone(),vp,0),pixels(&same,c.clone(),vp,0));
        let layout=parsed("LayoutResX: 480\nLayoutResY: 160","Source Sans 3",r"{\pos(120,80)}HH");
        let bounds=|raw:Vec<u8>|{
            let mut x0=240;let mut x1=0;let mut y0=160;let mut y1=0;
            for (i,p) in raw.chunks_exact(4).enumerate(){if p[3]>0 {x0=x0.min(i%240);x1=x1.max(i%240);y0=y0.min(i/240);y1=y1.max(i/240);}}
            (x0,y0,x1,y1)
        };
        let a=bounds(pixels(&same,c.clone(),vp,0));let b=bounds(pixels(&layout,c.clone(),vp,0));
        assert_eq!((a.1,a.3),(b.1,b.3));assert!(b.2-b.0<a.2-a.0);assert!((b.0+b.2).abs_diff(240)<=2);
        // LayoutRes supersedes external storage dimensions without replacing PlayRes.
        assert_eq!(pixels(&layout,c.clone(),vp.with_original_size(Size::new(999,333)).unwrap(),0),pixels(&layout,c.clone(),vp,0));
        for sbas in ["yes","no"] {
            let header=format!("LayoutResX: 480\nLayoutResY: 320\nScaledBorderAndShadow: {sbas}");
            let p=parsed(&header,"Source Sans 3",r"{\move(30,40,180,100)\org(120,80)\frz20\clip(10,10,230,150)\bord3\shad2\blur1}A{\p1}m 0 0 l 10 0 10 10 0 10");
            assert!(pixels(&p,c.clone(),vp,500).chunks_exact(4).any(|p|p[3]>0));
        }
    }
}
