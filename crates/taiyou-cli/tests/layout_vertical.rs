use taiyou_render::renderer::*;
fn parsed(header:&str,family:&str,body:&str)->ParsedTrack {
    parsed_style(header,family,body,1)
}
fn parsed_style(header:&str,family:&str,body:&str,border_style:u8)->ParsedTrack {
    let input=format!("[Script Info]\nPlayResX: 240\nPlayResY: 160\nWrapStyle: 2\n{header}\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Outline,Shadow,Encoding,BorderStyle\nStyle: Default,{family},40,0,0,1,{border_style}\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:05.00,{body}\n");
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
#[test]
fn layout_pair_is_active_only_when_both_axes_are_positive(){
    for header in ["LayoutResX: 480","LayoutResY: 320","LayoutResX: 0\nLayoutResY: 320","LayoutResX: -1\nLayoutResY: 320"]{
        assert_eq!(parsed(header,"Source Sans 3","A").track().script().layout_res(),None);
    }
    assert_eq!(parsed("LayoutResX: 480\nLayoutResY: 320","Source Sans 3","A").track().script().layout_res(),Some(Size::new(480,320)));
}
#[cfg(feature="native")]
mod native {
    use super::*;
    use std::sync::Arc;
    fn fonts()->Arc<ValidatedFontCatalog>{
        let limits=Limits::default().with_transient_bytes(512*1024*1024).unwrap();
        let mut ctx=FontContext::with_limits(limits.clone()).unwrap();let mut b=FontCatalogBuilder::new(limits);
        for name in ["latin.otf","cjk.otf","IPAexGothic.ttf"] {
            let p=std::path::PathBuf::from(std::env::var("RUNNER_TEMP").unwrap()).join("taiyou-shaping-fonts").join(name);
            b.register(&mut ctx,&std::fs::read(p).unwrap(),0).unwrap();
        }b.snapshot(&mut CatalogIngestor::new()).unwrap()
    }
    fn pixels(p:&ParsedTrack,c:Arc<ValidatedFontCatalog>,viewport:Viewport,ms:i64)->Vec<u8>{
        Renderer::new(Default::default()).unwrap().render_parsed(p,c,&RenderRequest{now:TimeUs::from_micros(ms*1000),viewport,unsupported:UnsupportedPolicy::Strict}).unwrap().pixels().to_vec()
    }
    #[test]
    fn opaque_boxes_use_outline_paint_cell_rectangles_and_separate_shadow(){
        let c=fonts();let vp=Viewport::full(Size::new(240,160)).unwrap();
        let draw=|tags:&str|pixels(&parsed_style("","Source Sans 3",&format!(r"{{\pos(120,80)\1a&HFF&\3c&H00FF00&{tags}}}H"),3),c.clone(),vp,0);
        assert!(draw(r"\bord0").iter().all(|b|*b==0));
        let raw=draw(r"\bord4");assert!(raw.chunks_exact(4).any(|p|p==[0,255,0,255]));
        assert!(raw.chunks_exact(4).all(|p|p[0]==0 && p[2]==0));
        // Background contains the cell interior, rather than an outline ring.
        assert_eq!(&raw[(80*240+120)*4..(80*240+120)*4+4],&[0,255,0,255]);
        assert!(draw(r"\bord0\shad3").chunks_exact(4).any(|p|p[3]>0));
        assert!(draw(r"\bord4\3a&HFF&").iter().all(|b|*b==0));
        assert_ne!(draw(r"\bord4"),draw(r"\bord4\blur2"));
        assert_ne!(draw(r"\bord4"),draw(r"\xbord8\ybord2"));
    }
    #[test]
    fn opaque_boxes_finalize_per_line_and_follow_normal_transforms_and_limits(){
        let c=fonts();let vp=Viewport::full(Size::new(240,160)).unwrap();
        for (text,count) in [(r"H\NH",2),(r"H\N\NH",2),(r"   ",0),(r"\h",1)]{
            let p=parsed_style("","Source Sans 3",&format!(r"{{\bord3\pos(120,80)}}{text}"),3);
            let e=taiyou_render::evaluate(p.track(),EventId::new(0),TimeUs::from_micros(0),Profile::CoreV1).unwrap();
            let mut ctx=FontContext::new().unwrap();let shaped=taiyou_render::shaping::shape_event(&mut ctx,&c,&e,&Default::default(),&Limits::default()).unwrap();
            let local=taiyou_render::layout(&shaped,&LayoutConstraints::new(e.placement().alignment())).unwrap();
            let device=taiyou_render::geometry::DeviceMapping::new(Size::new(240,160),vp.content()).unwrap();
            let paths=taiyou_render::geometry::opaque_box_paths(&local,0..local.glyphs().len(),device,ScriptPoint::new(FiniteF64::new(120.0).unwrap(),FiniteF64::new(80.0).unwrap()),None,false,&Limits::default()).unwrap();
            assert_eq!(paths.len(),count,"{text}");
        }
        for tags in [r"\frz20",r"\frx20\fry30",r"\fax0.3",r"\fscx200\fscy50",r"\org(0,0)\frz20",r"\clip(0,0,130,160)",r"\blur1\be1",r"\fad(1000,1000)"] {
            let p=parsed_style("LayoutResX: 480\nLayoutResY: 320\nScaledBorderAndShadow: yes","Source Sans 3",&format!(r"{{\pos(120,80)\bord3\shad2{tags}}}H{{\p1}}m 0 0 l 10 0 10 10 0 10"),3);
            assert!(pixels(&p,c.clone(),vp,500).chunks_exact(4).any(|p|p[3]>0),"{tags}");
        }
        let p=parsed_style("","Source Sans 3",r"{\bord999999999}H",3);
        let error=Renderer::new(Default::default()).unwrap().render_parsed(&p,c,&RenderRequest{now:TimeUs::from_micros(0),viewport:vp,unsupported:UnsupportedPolicy::Strict}).unwrap_err();
        assert_eq!(error.code(),ErrorCode::LimitExceeded);
    }
    #[test]
    fn vertical_family_metrics_variants_and_orientation_are_real(){
        let c=fonts();let limits=Limits::default().with_transient_bytes(512*1024*1024).unwrap();let mut ctx=FontContext::with_limits(limits.clone()).unwrap();
        let shape=|ctx:&mut FontContext,family:&str,text:&str|{
            let p=parsed("",family,text);
            let e=taiyou_render::evaluate(p.track(),EventId::new(0),TimeUs::from_micros(0),Profile::CoreV1).unwrap();
            taiyou_render::shaping::shape_event(ctx,&c,&e,&Default::default(),&limits).unwrap()
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
