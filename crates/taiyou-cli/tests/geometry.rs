#![cfg(feature = "native")]
use std::sync::Arc;
use taiyou_render::{
    geometry::*,
    layout::{LayoutConstraints, layout},
    shaping::*,
};
fn f(v: f64) -> FiniteF64 {
    FiniteF64::new(v).unwrap()
}
#[test]
fn geometry_real_parser_shape_layout_handoff_and_lifetime() {
    let output = {
        let input = "[Script Info]\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Alignment,Angle\nStyle: Default,Source Sans 3,20,7,90\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:01.00,Of{\\c&HFF0000&}f\\N{\\fscx150\\fscy200}é\\h";
        let options = Default::default();
        let parsed =
            taiyou_ass::parse_ass(input.as_bytes(), &options, &mut Default::default()).unwrap();
        let event = &parsed.track().events()[0];
        let evaluated =
            taiyou_render::evaluate(parsed.track(), event.id(), event.start(), options.profile())
                .unwrap();
        let mut context = FontContext::new().unwrap();
        let mut builder = FontCatalogBuilder::new(Limits::default());
        builder
            .register(
                &mut context,
                &std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap()).unwrap(),
                0,
            )
            .unwrap();
        let catalog = builder.snapshot(&mut Default::default()).unwrap();
        let shaped = shape_event(
            &mut context,
            &catalog,
            &evaluated,
            &ShapeOptions::default(),
            &Limits::default(),
        )
        .unwrap();
        let local = Arc::new(
            layout(
                &shaped,
                &LayoutConstraints::new(shaped.placement().alignment())
                    .with_scale(f(2.0), f(0.5))
                    .unwrap(),
            )
            .unwrap(),
        );
        let placement = GeometryPlacement {
            anchor: ScriptPoint::new(f(20.0), f(30.0)),
            origin: None,
            device: DeviceMapping::new(
                Size::new(100, 100),
                PixelRect::new(0, 0, 200, 300).unwrap(),
            )
            .unwrap(),
        };
        let output = geometry(
            &mut context,
            &catalog,
            local.clone(),
            placement,
            &Limits::default(),
        )
        .unwrap();
        let mut fresh = FontContext::new().unwrap();
        assert_eq!(
            output,
            geometry(&mut fresh, &catalog, local, placement, &Limits::default()).unwrap()
        );
        output
    };
    assert_eq!(output.glyphs().len(), output.layout().glyphs().len());
    for (g, positioned) in output.glyphs().iter().zip(output.layout().glyphs()) {
        assert_eq!(&g.cluster, positioned.cluster());
        let run = &output.layout().shaped().evaluated().runs()[g.association];
        // This fixture's Source Sans 3 OS/2 cell is1000+326 at UPEM1000.
        let native_em = ((run.shaping().font_size().get() * 64.0 + 0.5).floor() * 1000.0 / 1326.0
            + 0.5)
            .floor();
        let size = native_em / 64000.0;
        let point = g.outline.outline().ops().iter().find_map(|op| match op {
            PathOp::MoveTo(p) => Some(*p),
            _ => None,
        });
        if let Some(p) = point {
            // Independent cardinal matrix: local scale (2,.5), then ASS scale,
            // origin once (already includes HB offsets), rotation, device (2,3).
            let x = p.x().get() * size * run.geometry().scale_x().get() * 2.0
                + positioned.position().x().get();
            let y = -p.y().get() * size * run.geometry().scale_y().get() * 0.5
                + positioned.position().y().get();
            let (x, y) = if run.geometry().rotation_z().get() == 90.0 {
                (y, -x)
            } else {
                (x, y)
            };
            let expected = DevicePoint::new(
                Q6::from_pixels(f((x + 20.0) * 2.0)).unwrap(),
                Q6::from_pixels(f((y + 30.0) * 3.0)).unwrap(),
            );
            assert_eq!(g.geometry.mapping().point(p).unwrap(), expected);
        }
    }
    assert!(output.glyphs().iter().any(|g| g.geometry.ops().is_empty()));
    let ligature = output
        .glyphs()
        .iter()
        .find(|g| g.cluster.text() == "ff")
        .unwrap();
    assert_eq!(
        ligature
            .cluster
            .original_ranges(&Limits::default())
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn geometry_real_multiscript_fallback_font_identities_and_event_limits() {
    let limits = Limits::default()
        .with_transient_bytes(512 * 1024 * 1024)
        .unwrap();
    let input = "[Script Info]\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Alignment\nStyle: Default,Source Sans 3,20,7\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:01.00,Aبक語";
    let options = Default::default();
    let parsed =
        taiyou_ass::parse_ass(input.as_bytes(), &options, &mut Default::default()).unwrap();
    let event = &parsed.track().events()[0];
    let evaluated =
        taiyou_render::evaluate(parsed.track(), event.id(), event.start(), options.profile())
            .unwrap();
    let mut context = FontContext::with_limits(limits.clone()).unwrap();
    let mut builder = FontCatalogBuilder::new(limits.clone());
    for (id, suffix) in [
        ("latin", "otf"),
        ("arabic", "ttf"),
        ("indic", "ttf"),
        ("cjk", "otf"),
    ] {
        let path = std::path::PathBuf::from(std::env::var("RUNNER_TEMP").unwrap())
            .join("taiyou-shaping-fonts")
            .join(format!("{id}.{suffix}"));
        builder
            .register(&mut context, &std::fs::read(path).unwrap(), 0)
            .unwrap();
    }
    let catalog = builder.snapshot(&mut Default::default()).unwrap();
    let shaped = shape_event(
        &mut context,
        &catalog,
        &evaluated,
        &ShapeOptions::default(),
        &limits,
    )
    .unwrap();
    let local = Arc::new(
        layout(
            &shaped,
            &LayoutConstraints::new(shaped.placement().alignment()).with_limits(limits.clone()),
        )
        .unwrap(),
    );
    let placement = GeometryPlacement {
        anchor: ScriptPoint::new(f(0.0), f(0.0)),
        origin: None,
        device: DeviceMapping::new(Size::new(100, 100), PixelRect::new(0, 0, 100, 100).unwrap())
            .unwrap(),
    };
    let output = geometry(&mut context, &catalog, local.clone(), placement, &limits).unwrap();
    let ids: std::collections::BTreeSet<_> =
        output.glyphs().iter().map(|g| g.outline.font()).collect();
    assert_eq!(ids.len(), 4);
    assert!(
        output
            .glyphs()
            .iter()
            .all(|g| g.outline.glyph() != 0 && !g.geometry.ops().is_empty())
    );
    assert!(
        geometry(
            &mut context,
            &catalog,
            local.clone(),
            placement,
            &limits.clone().with_path_segments_per_frame(2).unwrap()
        )
        .is_err()
    );
    assert!(
        geometry(
            &mut context,
            &catalog,
            local,
            placement,
            &limits.with_transient_bytes(1).unwrap()
        )
        .is_err()
    );
}
