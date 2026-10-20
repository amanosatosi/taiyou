//! Additive, opt-in CI diagnostics using the frozen be.ass and its actual font.
#![cfg(feature = "native")]
use std::{path::Path, sync::Arc};
use taiyou_render::{
    compose, effects::{BlurSpec, apply_effects}, geometry::*, layout::layout,
    raster::{coverage_groups, rasterize}, renderer::*, shaping::shape_event,
};

fn save(root: &Path, name: &str, mask: &Mask, metadata: &mut String) {
    std::fs::write(root.join(format!("{name}.a8")), mask.data()).unwrap();
    let b = mask.bounds();
    metadata.push_str(&format!("{{\"name\":\"{name}\",\"x\":{},\"y\":{},\"width\":{},\"height\":{},\"stride\":{}}}\n", b.x0(), b.y0(), b.width(), b.height(), mask.stride()));
}
fn draw(id: EventId, ordinal: usize, pass: u16, mask: Arc<Mask>) -> DrawOp {
    DrawOp::new(DrawOrder::new(0, id, pass, ordinal as u32), mask,
        PixelOffset::default(), Paint::Solid(Rgba8::new(0,0,0,255)),
        PaintSpace::GlyphLocal, PaintMap::Identity, Arc::new(ResolvedClip::None))
}

// Independent separable integer equation: retain horizontal sums as integers,
// then divide the vertical sum only once. Preserve all four boundary edges.
fn integer_oracle(source: &Mask, passes: u32) -> Vec<u8> {
    let pad = (f64::from(passes).sqrt() * 0.5 + 0.5).floor() as usize;
    let w = source.bounds().width() as usize + 2 * pad;
    let h = source.bounds().height() as usize + 2 * pad;
    let mut a = vec![0; w*h];
    for y in 0..source.bounds().height() as usize {
        let n = source.bounds().width() as usize;
        a[(y+pad)*w+pad..(y+pad)*w+pad+n].copy_from_slice(&source.data()[y*source.stride()..y*source.stride()+n]);
    }
    for _ in 0..passes {
        if w < 3 || h < 3 { continue; }
        let mut horizontal = vec![0_u32; w*h];
        for y in 0..h {
            for x in 1..w-1 {
                horizontal[y*w+x] = u32::from(a[y*w+x-1]) + 2*u32::from(a[y*w+x]) + u32::from(a[y*w+x+1]);
            }
        }
        let mut next = a.clone();
        for y in 1..h-1 {
            for x in 1..w-1 {
                next[y*w+x] = ((horizontal[(y-1)*w+x] + 2*horizontal[y*w+x] + horizontal[(y+1)*w+x]) / 16) as u8;
            }
        }
        a = next;
    }
    a
}

#[test]
fn frozen_be_ass_stages_and_independent_integer_equation() {
    let Ok(corpus) = std::env::var("TAIYOU_BE_AUDIT_CORPUS") else {
        eprintln!("BE_AUDIT_NOT_REQUESTED: corpus diagnostics run in the supplemental audit job");
        return;
    };
    let root = std::path::PathBuf::from(std::env::var("TAIYOU_BE_AUDIT_OUTPUT").unwrap());
    std::fs::create_dir_all(&root).unwrap();
    let source = std::fs::read(Path::new(&corpus).join("regression/blurs/be.ass")).unwrap();
    let parsed = taiyou_ass::parse_ass(&source, &ParseOptions::default(), &mut TrackIngestor::new()).unwrap();
    let track = parsed.track().clone();
    let limits = Limits::default();
    let mut context = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(limits.clone());
    builder.register(&mut context, &std::fs::read(Path::new(&corpus).join("regression/blurs/Aileron-Regular.otf")).unwrap(), 0).unwrap();
    let catalog = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let mut metadata = String::new();
    let mut line_info = String::new();
    let mut group_info = String::new();
    for ms in [500,1500,2500,3500] {
        let now = TimeUs::from_micros(ms*1000);
        let event = track.events().iter().find(|e| e.is_active(now)).unwrap();
        let evaluated = taiyou_render::evaluate(&track, event.id(), now, Profile::CoreV1).unwrap();
        assert_eq!(evaluated.placement().alignment().get(), 5);
        let shaped = shape_event(&mut context, &catalog, &evaluated, &Default::default(), &limits).unwrap();
        let local = Arc::new(layout(&shaped, &LayoutConstraints::new(evaluated.placement().alignment()).with_wrap(evaluated.wrap_mode()).with_width(FiniteF64::new(780.0).unwrap()).unwrap()).unwrap());
        for (i,line) in local.lines().iter().enumerate() {
            line_info.push_str(&format!("{ms}\t{i}\t{:?}\t{}\t{}\t{}\n",line.baseline(),line.ascent().get(),line.descent().get(),line.advance().get()));
        }
        let geometry = Arc::new(geometry(&mut context, &catalog, local, GeometryPlacement {
            anchor: ScriptPoint::new(FiniteF64::new(400.0).unwrap(), FiniteF64::new(300.0).unwrap()), origin: None,
            device: DeviceMapping::new(track.script().play_res(),PixelRect::new(0,0,800,600).unwrap()).unwrap(),
        },&limits).unwrap());
        let groups = coverage_groups(geometry.clone(), &limits).unwrap();
        let mut fills = Vec::new();
        let mut doubles = Vec::new();
        for (i,group) in groups.iter().enumerate() {
            let member = group.policy().members().start;
            let state = &geometry.layout().shaped().evaluated().runs()[geometry.glyphs()[member].association];
            let mut decoded_hex = String::new();
            for glyph in &geometry.glyphs()[group.policy().members()] {
                let text = &geometry.layout().shaped().source_map().text()[glyph.cluster.start() as usize..glyph.cluster.end() as usize];
                for b in text.as_bytes() { decoded_hex.push_str(&format!("{b:02x}")); }
            }
            group_info.push_str(&format!("{{\"ms\":{ms},\"group\":{i},\"be\":{},\"decoded_utf8_hex\":\"{decoded_hex}\"}}\n",state.effects().be().get()));
            assert_eq!(state.effects().border_x().get(), 0.0);
            assert_eq!(state.effects().shadow_x().get(), 0.0);
            let raw = rasterize(&mut taiyou_render::raster::RasterContext::with_limits(limits.clone()).unwrap(),group).unwrap();
            save(&root,&format!("{ms}-g{i}-before"),&raw,&mut metadata);
            for pass in [1,2,4,8,16] {
                let filtered = apply_effects(&raw,&BlurSpec::new(FiniteF64::ZERO,FiniteF64::new(f64::from(pass)).unwrap(),[1.0,1.0],&limits).unwrap(),&limits).unwrap();
                if !raw.bounds().is_empty() { assert_eq!(filtered.data(),integer_oracle(&raw,pass)); }
                save(&root,&format!("{ms}-g{i}-be{pass}"),&filtered,&mut metadata);
            }
            let filtered = apply_effects(&raw,&BlurSpec::new(FiniteF64::ZERO,state.effects().be(),[1.0,1.0],&limits).unwrap(),&limits).unwrap();
            fills.push(draw(event.id(),i,2,filtered.clone()));
            doubles.push(draw(event.id(),i,0,filtered));
        }
        doubles.extend(fills.iter().cloned());
        let one = compose(Size::new(800,600),&fills,&limits,&mut OutputBudget::new(&limits)).unwrap();
        let two = compose(Size::new(800,600),&doubles,&limits,&mut OutputBudget::new(&limits)).unwrap();
        let actual = Renderer::new(RendererConfig::default()).unwrap().render(track.clone(),catalog.clone(),&RenderRequest {now,viewport:Viewport::full(Size::new(800,600)).unwrap(),unsupported:UnsupportedPolicy::Strict}).unwrap();
        assert_eq!(two.pixels(),actual.pixels(),"diagnostic reconstruction must match the real renderer");
        assert_ne!(one.pixels(),two.pixels(),"coincident shadow has an observable contribution");
        std::fs::write(root.join(format!("{ms}-single-fill.rgba")),one.pixels()).unwrap();
        std::fs::write(root.join(format!("{ms}-actual.rgba")),actual.pixels()).unwrap();
    }
    std::fs::write(root.join("masks.jsonl"),metadata).unwrap();
    std::fs::write(root.join("lines.tsv"),line_info).unwrap();
    std::fs::write(root.join("groups.jsonl"),group_info).unwrap();
}
