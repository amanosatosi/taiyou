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
        "[Script Info]\nWrapStyle: 2\n[V4+ Styles]\nFormat: Name,Fontname,Fontsize,Alignment,Outline\nStyle: Default,Source Sans 3,20,7,2\nStyle: Border,Source Sans 3,20,7,3\n[Events]\nFormat: Start,End,Text\nDialogue: 0:00:00.00,0:00:01.00,{text}"
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
fn compacted_text_keeps_continuous_ink_and_separate_layout_extents() {
    let mut rounded_layout_differs = false;
    let mut native_differs = false;
    for size in [17.37, 20.0, 23.51] {
        let event = geometry_for(&format!("{{\\fs{size}}}A A"));
        let local = event.layout();
        let advance: f64 = local.glyphs()[..2]
            .iter()
            .map(|g| {
                local.shaped().runs()[g.run_index()].glyphs()[g.glyph_index()]
                    .advance_x()
                    .get()
            })
            .sum();
        let native_advance: f64 = local.glyphs()[..2]
            .iter()
            .map(|g| {
                let raw = local.shaped().runs()[g.run_index()].glyphs()[g.glyph_index()]
                    .advance_x()
                    .get();
                let fixed = (raw * 1024.0 + 0.5).floor();
                ((fixed + 8.0) / 16.0).floor() / 64.0
            })
            .sum();
        native_differs |= native_advance != advance;
        let expected = Q6::from_pixels(f(native_advance)).unwrap().ticks();
        let origin = |index: usize| {
            event.glyphs()[index]
                .geometry
                .mapping()
                .point(FontPoint::ZERO)
                .unwrap()
                .x()
                .ticks()
        };
        assert_eq!(origin(2) - origin(0), expected);
        let layout_delta =
            local.glyphs()[2].position().x().get() - local.glyphs()[0].position().x().get();
        rounded_layout_differs |= Q6::from_pixels(f(layout_delta)).unwrap().ticks() != expected;
    }
    assert!(
        rounded_layout_differs,
        "fixture must expose independent word extent rounding"
    );
    assert!(
        native_differs,
        "fixture must expose native glyph measurement"
    );
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
        g.outline
            .outline()
            .ops()
            .iter()
            .any(|p| matches!(p, PathOp::QuadTo(..)))
            && g.geometry
                .ops()
                .iter()
                .any(|p| matches!(p, PathOp::CubicTo(..)))
    }));
    let groups = coverage_groups(event, &limits()).unwrap();
    assert_eq!(groups.len(), 1);
    let mask = {
        let mut ctx = RasterContext::with_limits(limits()).unwrap();
        rasterize(&mut ctx, &groups[0]).unwrap()
    };
    assert!(mask.data().contains(&255));
    assert!(mask.data().iter().any(|&v| v > 0 && v < 255));
    let fresh = rasterize(
        &mut RasterContext::with_limits(limits()).unwrap(),
        &groups[0],
    )
    .unwrap();
    assert_eq!(mask.bounds(), fresh.bounds());
    assert_eq!(mask.stride(), fresh.stride());
    assert_eq!(mask.data(), fresh.data());
    let dir = PathBuf::from(std::env::var_os("TAIYOU_RASTER_SAMPLE_DIR").unwrap());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("multiscript.a8"), mask.data()).unwrap();
    std::fs::write(dir.join("multiscript.q6"), command_dump(&groups[0])).unwrap();
    std::fs::write(dir.join("multiscript.ass"), input("Oé ب क 語")).unwrap();
    std::fs::write(
        dir.join("multiscript-info.txt"),
        format!(
            "{} {} {} {} {}",
            mask.bounds().width(),
            mask.bounds().height(),
            mask.stride(),
            mask.bounds().x0(),
            mask.bounds().y0()
        ),
    )
    .unwrap();
    let identities: Vec<String> = ids
        .iter()
        .map(|font| font.sha256().iter().map(|b| format!("{b:02x}")).collect())
        .collect();
    std::fs::write(dir.join("multiscript-fonts.txt"), identities.join("\n")).unwrap();
    drop(groups);
    assert!(mask.data().iter().any(|&v| v > 0));
}
#[test]
fn raster_grouping_obeys_retained_evaluated_identities_and_source_clusters() {
    for (text, count) in [
        ("AV", 1),
        ("A{\\c&HFF0000&}V", 2),
        ("A{\\rBorder}V", 2),
        ("A{\\fscx200}V", 2),
        ("A{\\fs30}V", 2),
        ("f{\\c&HFF0000&}f", 1),
        ("{\\kf100}A{\\kt0}V", 1),
        ("{\\kf100}A{\\kt500}V", 1),
        ("{\\kf100}A{\\kf100}V", 2),
        ("{\\kf100}A{\\kt0\\c&HFF0000&}V", 2),
        ("{\\kf100}A\\N{\\kt0}V", 2),
        ("A\\NV", 2),
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
    std::fs::write(dir.join("commands.q6"), command_dump(&groups[0])).unwrap();
    let event = groups[0].event().unwrap();
    let font = event.glyphs()[0].outline.font();
    assert!(event.glyphs().iter().all(|g| g.outline.font() == font));
    let digest: String = font.sha256().iter().map(|b| format!("{b:02x}")).collect();
    std::fs::write(dir.join("sample-info.txt"),format!("width={}\nheight={}\nstride={}\nx0={}\ny0={}\nbackend={}\nfont_sha256={}\nface_index={}\n",mask.bounds().width(),mask.bounds().height(),mask.stride(),mask.bounds().x0(),mask.bounds().y0(),ctx.backend(),digest,font.face_index())).unwrap();
}
fn command_dump(group: &CoverageGroup) -> String {
    let mut commands = String::new();
    for instance in group.instances() {
        for &op in instance.ops() {
            let (name, points) = match op {
                PathOp::MoveTo(p) => ("M", vec![p]),
                PathOp::LineTo(p) => ("L", vec![p]),
                PathOp::QuadTo(a, b) => ("Q", vec![a, b]),
                PathOp::CubicTo(a, b, c) => ("C", vec![a, b, c]),
                PathOp::Close => ("Z", vec![]),
            };
            commands.push_str(name);
            for p in points {
                commands.push_str(&format!(
                    " {} {}",
                    i64::from(p.x().ticks()) + i64::from(instance.offset().x()) * 64,
                    i64::from(p.y().ticks()) + i64::from(instance.offset().y()) * 64
                ));
            }
            commands.push('\n');
        }
    }
    commands
}
