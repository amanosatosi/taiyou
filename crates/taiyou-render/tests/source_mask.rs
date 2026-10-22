//! Identical geometry controls: no font matching, shaping or blur parameter tuning.
#![cfg(feature = "native")]
use std::sync::Arc;
use taiyou_render::{
    effects::{BlurSpec, apply_effects},
    geometry::*,
    raster::*,
};
use taiyou_types::*;
fn f(x: f64) -> FiniteF64 {
    FiniteF64::new(x).unwrap()
}
fn p(x: f64, y: f64) -> FontPoint {
    FontPoint::new(x, -y).unwrap()
}
fn rectangle(x: f64, y: f64, w: f64, h: f64) -> Vec<PathOp<FontPoint>> {
    vec![
        PathOp::MoveTo(p(x, y)),
        PathOp::LineTo(p(x + w, y)),
        PathOp::LineTo(p(x + w, y + h)),
        PathOp::LineTo(p(x, y + h)),
        PathOp::Close,
    ]
}
fn group(ops: Vec<PathOp<FontPoint>>) -> CoverageGroup {
    let limits = Limits::default();
    let mapping = GeometryMap::new(
        (f(1.0), f(1.0)),
        ScriptPoint::new(f(0.0), f(0.0)),
        ScriptPoint::new(f(0.0), f(0.0)),
        None,
        f(0.0),
        DeviceMapping::new(Size::new(1, 1), PixelRect::new(0, 0, 1, 1).unwrap()).unwrap(),
    )
    .unwrap();
    let instance = map_outline(
        Arc::new(Outline::new(ops, &limits).unwrap()),
        mapping,
        &limits,
    )
    .unwrap();
    CoverageGroup::new(vec![instance], CoveragePass::Fill, Profile::CoreV1, &limits).unwrap()
}
fn scan(ops: Vec<PathOp<FontPoint>>) -> Arc<Mask> {
    rasterize_xy(&mut RasterContext::new().unwrap(), &group(ops)).unwrap()
}
#[test]
fn identical_fractional_rectangle_proves_raster_difference_before_fonts_or_filters() {
    let g = group(rectangle(1.0 / 16.0, 0.0, 1.0, 1.0));
    let mut context = RasterContext::new().unwrap();
    let ft = rasterize(&mut context, &g).unwrap();
    let xy = rasterize_xy(&mut context, &g).unwrap();
    // ScanLineData samples eighth rows, rounds crossings to eighth columns.
    // A half-eighth left edge rounds up: seven covered cells plus one next pixel.
    assert_eq!(xy.bounds(), PixelRect::new(0, 0, 3, 3).unwrap());
    assert_eq!(xy.data(), [56, 8, 0, 0, 0, 0, 0, 0, 0]);
    assert!(xy.is_xy_coverage());
    assert_eq!(ft.data(), [240, 16]);
    assert_eq!(xy.coverage_a8_at(0, 0, PixelOffset::default()), 223);
    assert_ne!(
        ft.coverage_at(0, 0, PixelOffset::default()),
        xy.coverage_a8_at(0, 0, PixelOffset::default())
    );
    println!(
        "IDENTICAL_PATH_CONTROL ft={:?} xy64={:?} xy_bounds={:?}",
        ft.data(),
        xy.data(),
        xy.bounds()
    );
}
#[test]
fn native_overlay_bounds_nonzero_winding_edge_inclusion_and_translation() {
    let a = scan(rectangle(0.0, 0.0, 1.0, 1.0));
    assert_eq!(a.bounds(), PixelRect::new(0, 0, 3, 3).unwrap());
    assert_eq!(a.data(), [64, 0, 0, 0, 0, 0, 0, 0, 0]);
    let mut union = rectangle(0.0, 0.0, 1.0, 1.0);
    union.extend(rectangle(0.0, 0.0, 1.0, 1.0));
    assert_eq!(scan(union).data(), a.data());
    let mut hole = rectangle(0.0, 0.0, 3.0, 3.0);
    hole.extend([
        PathOp::MoveTo(p(1.0, 1.0)),
        PathOp::LineTo(p(1.0, 2.0)),
        PathOp::LineTo(p(2.0, 2.0)),
        PathOp::LineTo(p(2.0, 1.0)),
        PathOp::Close,
    ]);
    let mask = scan(hole);
    assert_eq!(mask.coverage_at(1, 1, PixelOffset::default()), 0);
    assert_eq!(mask.coverage_at(0, 1, PixelOffset::default()), 64);
    let triangle = scan(vec![
        PathOp::MoveTo(p(0.0, 0.0)),
        PathOp::LineTo(p(1.0, 0.0)),
        PathOp::LineTo(p(0.0, 1.0)),
        PathOp::Close,
    ]);
    // Diagonal crossings at half-columns round up. 8+7+...+1 eighth cells.
    assert_eq!(triangle.data()[0], 36);
    for (x, y) in [(0.125, 0.0), (0.0, 0.125), (0.5, 0.5), (-3.875, -2.5)] {
        let one = scan(rectangle(x, y, 1.0, 1.0));
        let two = scan(rectangle(x - 7.0, y + 3.0, 1.0, 1.0));
        assert_eq!(one.data(), two.data());
        assert_eq!(one.data(), scan(rectangle(x, y, 1.0, 1.0)).data());
        assert_eq!(one.data().iter().map(|&v| u32::from(v)).sum::<u32>(), 64);
    }
}
#[test]
fn be_uses_native_scan_units_until_paint_conversion() {
    let source = scan(rectangle(0.0, 0.0, 1.0, 1.0));
    for strength in [1.0, 2.0, 4.0, 8.0, 16.0] {
        let spec = BlurSpec::new(f(0.0), f(strength), [1.0, 1.0], &Limits::default()).unwrap();
        let filtered = apply_effects(&source, &spec, &Limits::default()).unwrap();
        assert!(filtered.is_xy_coverage());
        assert!(filtered.data().iter().all(|&v| v <= 64));
        assert_eq!(source.data()[0], 64);
        assert_eq!(
            filtered.data(),
            apply_effects(&source, &spec, &Limits::default())
                .unwrap()
                .data()
        );
        if strength == 1.0 {
            assert_eq!(filtered.coverage_at(0, 0, PixelOffset::default()), 16);
            assert_eq!(filtered.coverage_a8_at(0, 0, PixelOffset::default()), 64);
        }
        println!(
            "NATIVE_BE strength={strength} bounds={:?} sum={}",
            filtered.bounds(),
            filtered.data().iter().map(|&v| u64::from(v)).sum::<u64>()
        );
    }
    assert_eq!(
        Mask::new_xy(PixelRect::new(0, 0, 1, 1).unwrap(), 1, Arc::from([65]))
            .unwrap_err()
            .code(),
        ErrorCode::InvalidArgument
    );
}
#[test]
fn scan_preflights_bounds_segments_and_transient_publication() {
    let g = group(rectangle(0.0, 0.0, 2.0, 2.0));
    for limits in [
        Limits::default().with_transient_bytes(1).unwrap(),
        Limits::default().with_max_dimension(1).unwrap(),
        Limits::default().with_output_pixels(1).unwrap(),
        Limits::default().with_path_segments_per_frame(1).unwrap(),
    ] {
        let error = rasterize_xy(&mut RasterContext::with_limits(limits).unwrap(), &g).unwrap_err();
        assert_eq!(error.code(), ErrorCode::LimitExceeded);
    }
    let bad = Mask::new_xy(PixelRect::new(0, 0, 1, 1).unwrap(), 2, Arc::from([64, 1])).unwrap_err();
    assert_eq!(bad.code(), ErrorCode::InvalidArgument);
    assert!(scan(vec![]).bounds().is_empty());
}
#[test]
fn expanded_contours_shadow_phase_and_subtraction_use_the_same_scan_units() {
    let limits = Limits::default();
    let g = group(rectangle(0.0, 0.0, 4.0, 4.0));
    let mut context = RasterContext::new().unwrap();
    let fill = rasterize_xy(&mut context, &g).unwrap();
    let (expanded, points) = taiyou_render::stroke::rasterize_expanded_axes_xy(
        &mut context,
        &g,
        Q6::from_ticks(64),
        Q6::from_ticks(64),
        Q6::from_ticks(0),
        Q6::from_ticks(0),
        1_000_000,
    )
    .unwrap();
    assert!(points > 0);
    assert!(expanded.is_xy_coverage());
    assert_eq!(expanded.coverage_at(2, 2, PixelOffset::default()), 64);
    assert_eq!(expanded.coverage_at(-1, 2, PixelOffset::default()), 64);
    let border = taiyou_render::stroke::subtract_coverage(&expanded, &fill, &limits).unwrap();
    assert!(border.is_xy_coverage());
    assert_eq!(border.coverage_at(2, 2, PixelOffset::default()), 0);
    assert_eq!(border.coverage_at(-1, 2, PixelOffset::default()), 64);
    let shadow = taiyou_render::stroke::rasterize_translated_xy(
        &mut context,
        &g,
        Q6::from_ticks(8),
        Q6::from_ticks(0),
    )
    .unwrap();
    assert_eq!(shadow.coverage_at(0, 2, PixelOffset::default()), 56);
    assert_eq!(shadow.coverage_at(4, 2, PixelOffset::default()), 8);
    assert_eq!(
        shadow.data().iter().map(|&v| u32::from(v)).sum::<u32>(),
        4 * 4 * 64
    );
    let error = taiyou_render::stroke::rasterize_expanded_axes_xy(
        &mut context,
        &g,
        Q6::from_ticks(64),
        Q6::from_ticks(64),
        Q6::from_ticks(0),
        Q6::from_ticks(0),
        1,
    )
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::LimitExceeded);
}
#[test]
fn supplied_gdi_and_freetype_paths_separate_geometry_from_scan_coverage() {
    let Ok(root) = std::env::var("TAIYOU_BE_AUDIT_OUTPUT") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let path = root.join("supplied-paths.tsv");
    if !path.exists() {
        return;
    }
    let input = std::fs::read_to_string(path).unwrap();
    let mut groups = Vec::new();
    let mut report = String::new();
    for row in input.lines() {
        let (name, values) = row.split_once('\t').unwrap();
        let points: Vec<Vec<i32>> = values
            .split(';')
            .map(|s| s.split(',').map(|v| v.parse().unwrap()).collect())
            .collect();
        for phase in [0.0, 0.125, 0.5] {
            let point = |i: usize| {
                p(
                    f64::from(points[i][1]) / 64.0 + phase,
                    f64::from(points[i][2]) / 64.0,
                )
            };
            let mut ops = Vec::new();
            let mut i = 0;
            while i < points.len() {
                match points[i][0] {
                    6 => ops.push(PathOp::MoveTo(point(i))),
                    2 => ops.push(PathOp::LineTo(point(i))),
                    4 => {
                        assert_eq!(points[i + 1][0], 4);
                        assert_eq!(points[i + 2][0], 4);
                        ops.push(PathOp::CubicTo(point(i), point(i + 1), point(i + 2)));
                        i += 2;
                    }
                    _ => panic!("unexpected authenticated path command"),
                }
                i += 1;
            }
            ops.push(PathOp::Close);
            let g = group(ops);
            let mut context = RasterContext::new().unwrap();
            let ft = rasterize(&mut context, &g).unwrap();
            let xy = rasterize_xy(&mut context, &g).unwrap();
            let compare = |a: &Mask, b: &Mask, normalize: bool| {
                let (a0, b0) = (a.bounds(), b.bounds());
                let mut changed = 0;
                let mut total = 0_u64;
                let mut max = 0;
                for y in a0.y0().min(b0.y0())..a0.y1().max(b0.y1()) {
                    for x in a0.x0().min(b0.x0())..a0.x1().max(b0.x1()) {
                        let value = |m: &Mask| {
                            if normalize {
                                m.coverage_a8_at(i64::from(x), i64::from(y), PixelOffset::default())
                            } else {
                                m.coverage_at(i64::from(x), i64::from(y), PixelOffset::default())
                            }
                        };
                        let d = value(a).abs_diff(value(b));
                        changed += u64::from(d != 0);
                        total += u64::from(d);
                        max = max.max(d);
                    }
                }
                (changed, total, max)
            };
            let (changed, total, max) = compare(&ft, &xy, true);
            report.push_str(&format!("{{\"path\":\"{name}\",\"phase\":{phase},\"comparison\":\"same-geometry-coverage\",\"changed\":{changed},\"sum_delta\":{total},\"max_delta\":{max}}}\n"));
            if name.ends_with("gdi") {
                groups.push((phase, xy));
            } else {
                // Find by phase rather than depending on font/text ordering.
                let reference = groups
                    .iter()
                    .rev()
                    .find(|(p, _)| *p == phase)
                    .map(|(_, m)| m)
                    .unwrap();
                for be in [0.0, 1.0, 2.0, 4.0, 8.0, 16.0] {
                    let spec =
                        BlurSpec::new(f(0.0), f(be), [1.0, 1.0], &Limits::default()).unwrap();
                    let a = apply_effects(reference, &spec, &Limits::default()).unwrap();
                    let b = apply_effects(&xy, &spec, &Limits::default()).unwrap();
                    let (changed, total, max) = compare(&a, &b, false);
                    report.push_str(&format!("{{\"path\":\"{name}\",\"phase\":{phase},\"be\":{be},\"comparison\":\"same-raster-different-outline\",\"changed\":{changed},\"sum_delta\":{total},\"max_delta\":{max}}}\n"));
                }
            }
        }
    }
    assert_eq!(groups.len(), 9);
    std::fs::write(root.join("supplied-path-comparisons.jsonl"), &report).unwrap();
    println!("SUPPLIED_PATH_COMPARISONS\n{report}");
}
