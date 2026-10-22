//! Opt-in frozen-corpus controls; none of these alternatives changes production.
#![cfg(feature = "native")]
use std::{path::PathBuf, sync::Arc};
use taiyou_render::{
    compose,
    effects::{BlurSpec, apply_effects},
    geometry::*,
    raster::*,
    renderer::*,
    shaping::shape_event,
};
fn f(v: f64) -> FiniteF64 {
    FiniteF64::new(v).unwrap()
}
fn tight(mask: &Arc<Mask>) -> Arc<Mask> {
    let b = mask.bounds();
    let mut hull: Option<(i32, i32, i32, i32)> = None;
    for y in b.y0()..b.y1() {
        for x in b.x0()..b.x1() {
            if mask.coverage_at(i64::from(x), i64::from(y), PixelOffset::default()) != 0 {
                hull = Some(hull.map_or((x, y, x + 1, y + 1), |(a, b, c, d)| {
                    (a.min(x), b.min(y), c.max(x + 1), d.max(y + 1))
                }));
            }
        }
    }
    let Some((a, b, c, d)) = hull else {
        return mask.clone();
    };
    let bounds = PixelRect::new(a, b, c, d).unwrap();
    let mut data = Vec::new();
    for y in b..d {
        for x in a..c {
            data.push(mask.coverage_at(i64::from(x), i64::from(y), PixelOffset::default()));
        }
    }
    Arc::new(
        mask.with_data(bounds, bounds.width() as usize, data.into())
            .unwrap(),
    )
}
#[test]
fn gaussian_source_units_bounds_and_boundary_shapes_are_independent_controls() {
    let Ok(corpus) = std::env::var("TAIYOU_BE_AUDIT_CORPUS") else {
        return;
    };
    let corpus = PathBuf::from(corpus);
    let root = PathBuf::from(std::env::var("TAIYOU_BE_AUDIT_OUTPUT").unwrap());
    std::fs::create_dir_all(&root).unwrap();
    let source = std::fs::read(corpus.join("regression/blurs/blur+t.ass")).unwrap();
    let parsed =
        taiyou_ass::parse_ass(&source, &ParseOptions::default(), &mut TrackIngestor::new())
            .unwrap();
    let track = parsed.track();
    let limits = Limits::default();
    let mut context = FontContext::new().unwrap();
    let mut builder = FontCatalogBuilder::new(limits.clone());
    builder
        .register(
            &mut context,
            &std::fs::read(corpus.join("regression/blurs/Aileron-Regular.otf")).unwrap(),
            0,
        )
        .unwrap();
    let catalog = builder.snapshot(&mut CatalogIngestor::new()).unwrap();
    let mut metadata = String::new();
    for ms in (1000..=1900).step_by(100) {
        let now = TimeUs::from_micros(ms * 1000);
        let event = track.events().iter().find(|e| e.is_active(now)).unwrap();
        let e = taiyou_render::evaluate(track, event.id(), now, Profile::CoreV1).unwrap();
        let shaped = shape_event(&mut context, &catalog, &e, &Default::default(), &limits).unwrap();
        let local = Arc::new(
            taiyou_render::layout(
                &shaped,
                &LayoutConstraints::new(e.placement().alignment())
                    .with_wrap(e.wrap_mode())
                    .with_width(f(740.0))
                    .unwrap(),
            )
            .unwrap(),
        );
        let geometry = Arc::new(
            taiyou_render::geometry::geometry(
                &mut context,
                &catalog,
                local,
                GeometryPlacement {
                    anchor: ScriptPoint::new(f(400.0), f(300.0)),
                    origin: None,
                    device: DeviceMapping::new(
                        Size::new(800, 600),
                        PixelRect::new(0, 0, 800, 600).unwrap(),
                    )
                    .unwrap(),
                },
                &limits,
            )
            .unwrap(),
        );
        let groups = coverage_groups(geometry.clone(), &limits).unwrap();
        let mut variants: [Vec<DrawOp>; 4] = std::array::from_fn(|_| Vec::new());
        for (i, group) in groups.iter().enumerate() {
            let member = group.policy().members().start;
            let state = &geometry.layout().shaped().evaluated().runs()
                [geometry.items()[member].association];
            assert_eq!(state.effects().border_x().get(), 0.0);
            assert_eq!(state.effects().shadow_x().get(), 0.0);
            assert_eq!(state.effects().be().get(), 0.0);
            let mut ctx = RasterContext::with_limits(limits.clone()).unwrap();
            let ft = rasterize(&mut ctx, group).unwrap();
            let xy = rasterize_xy(&mut ctx, group).unwrap();
            let b = xy.bounds();
            let mut bytes = Vec::new();
            for y in b.y0()..b.y1() {
                for x in b.x0()..b.x1() {
                    bytes.push(xy.coverage_a8_at(
                        i64::from(x),
                        i64::from(y),
                        PixelOffset::default(),
                    ));
                }
            }
            // Diagnostic alternative only: this conversion is NOT what the
            // inspected pinterf byte-to-float Gaussian input does.
            let converted = Arc::new(Mask::new(b, b.width() as usize, bytes.into()).unwrap());
            let spec = BlurSpec::new(
                state.effects().blur(),
                state.effects().be(),
                [1.0, 1.0],
                &limits,
            )
            .unwrap();
            metadata.push_str(&format!(
                "{ms}\t{i}\t{}\t{:?}\t{:?}\n",
                state.effects().blur().get(),
                ft.bounds(),
                xy.bounds()
            ));
            for (v, mask) in [ft, xy.clone(), converted, tight(&xy)]
                .into_iter()
                .enumerate()
            {
                let mask = apply_effects(&mask, &spec, &limits).unwrap();
                variants[v].push(DrawOp::new(
                    DrawOrder::new(0, event.id(), 2, i as u32),
                    mask,
                    PixelOffset::default(),
                    Paint::Solid(state.paint().primary()),
                    PaintSpace::GlyphLocal,
                    PaintMap::Identity,
                    Arc::new(ResolvedClip::None),
                ));
            }
        }
        let frames: Vec<_> = variants
            .iter()
            .map(|ops| {
                compose(
                    Size::new(800, 600),
                    ops,
                    &limits,
                    &mut OutputBudget::new(&limits),
                )
                .unwrap()
            })
            .collect();
        let actual = Renderer::new(Default::default())
            .unwrap()
            .render_parsed(
                &parsed,
                catalog.clone(),
                &RenderRequest {
                    now,
                    viewport: Viewport::full(Size::new(800, 600)).unwrap(),
                    unsupported: UnsupportedPolicy::Strict,
                },
            )
            .unwrap();
        assert_eq!(
            frames[1].pixels(),
            actual.pixels(),
            "actual source-mask pipeline at {ms}"
        );
        assert_eq!(
            frames[1].pixels(),
            frames[3].pixels(),
            "zero allocation padding cannot change scalar Gaussian output at {ms}"
        );
        for (frame, label) in
            frames
                .iter()
                .zip(["ft-a8", "xy64", "xy-converted-before-gaussian", "xy-tight"])
        {
            std::fs::write(
                root.join(format!("gaussian-{ms}-{label}.rgba")),
                frame.pixels(),
            )
            .unwrap();
        }
    }
    std::fs::write(root.join("gaussian-groups.tsv"), metadata).unwrap();
}
