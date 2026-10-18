#![cfg(feature = "native")]
use std::{path::PathBuf, sync::Arc};
use taiyou_render::{
    geometry::*,
    layout::{LayoutConstraints, layout},
    raster::*,
    shaping::*,
};
fn f(n: f64) -> FiniteF64 {
    FiniteF64::new(n).unwrap()
}
fn limits() -> Limits {
    Limits::default()
        .with_transient_bytes(512 * 1024 * 1024)
        .unwrap()
}
fn input(text: &str) -> String {
    format!(
        "[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Alignment\nStyle: Default,Source Sans 3,20,7\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:01.00,{text}"
    )
}
fn geometry_for(text: &str) -> Arc<GeometryEvent> {
    let source = input(text);
    let options = Default::default();
    let parsed =
        taiyou_ass::parse_ass(source.as_bytes(), &options, &mut Default::default()).unwrap();
    let event = &parsed.track().events()[0];
    let evaluated =
        taiyou_render::evaluate(parsed.track(), event.id(), event.start(), options.profile())
            .unwrap();
    let mut ctx = FontContext::with_limits(limits()).unwrap();
    let mut builder = FontCatalogBuilder::new(limits());
    for (name, extension) in [
        ("latin", "otf"),
        ("arabic", "ttf"),
        ("indic", "ttf"),
        ("cjk", "otf"),
    ] {
        let path = PathBuf::from(std::env::var("RUNNER_TEMP").unwrap())
            .join("taiyou-shaping-fonts")
            .join(format!("{name}.{extension}"));
        builder
            .register(&mut ctx, &std::fs::read(path).unwrap(), 0)
            .unwrap();
    }
    let catalog = builder.snapshot(&mut Default::default()).unwrap();
    let shaped = shape_event(
        &mut ctx,
        &catalog,
        &evaluated,
        &ShapeOptions::default(),
        &limits(),
    )
    .unwrap();
    let local = Arc::new(
        layout(
            &shaped,
            &LayoutConstraints::new(shaped.placement().alignment()).with_limits(limits()),
        )
        .unwrap(),
    );
    Arc::new(
        geometry(
            &mut ctx,
            &catalog,
            local,
            GeometryPlacement {
                anchor: ScriptPoint::new(f(-1.0 / 64.0), f(-0.5)),
                origin: None,
                device: DeviceMapping::new(
                    Size::new(100, 100),
                    PixelRect::new(0, 0, 100, 100).unwrap(),
                )
                .unwrap(),
            },
            &limits(),
        )
        .unwrap(),
    )
}
#[test]
fn raster_fixed_fonts_owned_fallback_curves_and_repeatability() {
    let event = geometry_for("Oé ب क 語");
    let ids: std::collections::BTreeSet<_> =
        event.glyphs().iter().map(|g| g.outline.font()).collect();
    assert_eq!(ids.len(), 4);
    assert!(event.glyphs().iter().any(|g| {
        g.geometry
            .ops()
            .iter()
            .any(|p| matches!(p, PathOp::CubicTo(..)))
    }));
    assert!(event.glyphs().iter().any(|g| {
        g.geometry
            .ops()
            .iter()
            .any(|p| matches!(p, PathOp::QuadTo(..)))
    }));
    let groups = coverage_groups(event, &limits()).unwrap();
    assert_eq!(groups.len(), 1);
    let mask = {
        let mut ctx = RasterContext::with_limits(limits()).unwrap();
        rasterize(&mut ctx, &groups[0]).unwrap()
    };
    assert!(mask.data().iter().any(|&v| v == 255));
    assert!(mask.data().iter().any(|&v| v > 0 && v < 255));
    let fresh = rasterize(
        &mut RasterContext::with_limits(limits()).unwrap(),
        &groups[0],
    )
    .unwrap();
    assert_eq!(mask.bounds(), fresh.bounds());
    assert_eq!(mask.stride(), fresh.stride());
    assert_eq!(mask.data(), fresh.data());
    drop(groups);
    assert!(mask.data().iter().any(|&v| v > 0));
}
#[test]
fn raster_grouping_obeys_retained_evaluated_identities_and_source_clusters() {
    for (text, count) in [
        ("AV", 1),
        ("A{\\c&HFF0000&}V", 2),
        ("A{\\bord3}V", 2),
        ("A{\\fscx200}V", 2),
        ("A{\\fs30}V", 2),
        ("f{\\c&HFF0000&}f", 1),
    ] {
        let groups = coverage_groups(geometry_for(text), &limits()).unwrap();
        assert_eq!(groups.len(), count, "{text}");
        if text.starts_with('f') {
            let event = groups[0].event().unwrap();
            assert_eq!(
                event.glyphs()[0]
                    .cluster
                    .original_ranges(&limits())
                    .unwrap()
                    .len(),
                2
            );
        }
        for g in &groups {
            assert_eq!(g.policy().members().len(), g.instances().len());
        }
    }
}
#[test]
fn raster_first_genuine_a8_sample() {
    let Some(directory) = std::env::var_os("TAIYOU_RASTER_SAMPLE_DIR") else {
        panic!("CI sample directory is required");
    };
    let groups = coverage_groups(geometry_for("Taiyou"), &limits()).unwrap();
    assert_eq!(groups.len(), 1);
    let mut ctx = RasterContext::with_limits(limits()).unwrap();
    let mask = rasterize(&mut ctx, &groups[0]).unwrap();
    assert!(mask.data().iter().any(|&v| v > 0));
    let dir = PathBuf::from(directory);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("sample.a8"), mask.data()).unwrap();
    std::fs::write(dir.join("input.ass"), input("Taiyou")).unwrap();
    let event = groups[0].event().unwrap();
    let font = event.glyphs()[0].outline.font();
    assert!(event.glyphs().iter().all(|g| g.outline.font() == font));
    let digest: String = font.sha256().iter().map(|b| format!("{b:02x}")).collect();
    std::fs::write(dir.join("sample-info.txt"),format!("width={}\nheight={}\nstride={}\nx0={}\ny0={}\nbackend={}\nfont_sha256={}\nface_index={}\n",mask.bounds().width(),mask.bounds().height(),mask.stride(),mask.bounds().x0(),mask.bounds().y0(),ctx.backend(),digest,font.face_index())).unwrap();
}
