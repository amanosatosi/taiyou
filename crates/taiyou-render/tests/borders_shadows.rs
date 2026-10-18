#![cfg(feature = "native")]
use std::sync::Arc;
use taiyou_render::{compose, geometry::*, raster::*, stroke};
use taiyou_types::*;

fn f(value: f64) -> FiniteF64 {
    FiniteF64::new(value).unwrap()
}
fn p(x: f64, y: f64) -> FontPoint {
    FontPoint::new(x, -y).unwrap()
}
fn rectangle(x: f64, y: f64, w: f64, h: f64, reverse: bool) -> Vec<PathOp<FontPoint>> {
    let corners = if reverse {
        [(x, y), (x, y + h), (x + w, y + h), (x + w, y)]
    } else {
        [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
    };
    vec![
        PathOp::MoveTo(p(corners[0].0, corners[0].1)),
        PathOp::LineTo(p(corners[1].0, corners[1].1)),
        PathOp::LineTo(p(corners[2].0, corners[2].1)),
        PathOp::LineTo(p(corners[3].0, corners[3].1)),
        PathOp::Close,
    ]
}
fn mapped(
    ops: Vec<PathOp<FontPoint>>,
    position: (f64, f64),
    scale: (f64, f64),
    rotation: f64,
) -> GeometryInstance {
    let limits = Limits::default();
    let outline = Arc::new(Outline::new(ops, &limits).unwrap());
    let device =
        DeviceMapping::new(Size::new(32, 32), PixelRect::new(0, 0, 32, 32).unwrap()).unwrap();
    let mapping = GeometryMap::new(
        (f(scale.0), f(scale.1)),
        ScriptPoint::new(f(0.0), f(0.0)),
        ScriptPoint::new(f(position.0), f(position.1)),
        None,
        f(rotation),
        device,
    )
    .unwrap();
    map_outline(outline, mapping, &limits).unwrap()
}
fn group(instances: Vec<GeometryInstance>) -> CoverageGroup {
    CoverageGroup::new(
        instances,
        CoveragePass::Fill,
        Profile::CoreV1,
        &Limits::default(),
    )
    .unwrap()
}
fn rectangle_group(x: f64, y: f64, w: f64, h: f64) -> CoverageGroup {
    group(vec![mapped(
        rectangle(x, y, w, h, false),
        (0.0, 0.0),
        (1.0, 1.0),
        0.0,
    )])
}
fn q6(ticks: i32) -> Q6 {
    Q6::from_ticks(ticks)
}
fn mask_limits() -> Limits {
    Limits::default()
        .with_transient_bytes(512 * 1024 * 1024)
        .unwrap()
}

#[test]
fn border_zero_has_no_outline_coverage() {
    let group = rectangle_group(2.0, 2.0, 2.0, 2.0);
    let fill = rasterize(&mut RasterContext::new().unwrap(), &group).unwrap();
    let (expanded, segments) = stroke::rasterize_expanded(
        &mut RasterContext::new().unwrap(),
        &group,
        q6(0),
        q6(0),
        q6(0),
        Limits::default().path_segments_per_frame(),
    )
    .unwrap();
    let border = stroke::subtract_coverage(&expanded, &fill, &Limits::default()).unwrap();
    assert_eq!(segments, 0);
    assert_eq!(expanded.data(), fill.data());
    assert!(border.bounds().is_empty());
    assert!(border.data().is_empty());
}

#[test]
fn border_hole_expands_the_nonzero_winding_shape_and_subtracts_safely() {
    let mut contours = rectangle(0.0, 0.0, 6.0, 6.0, false);
    contours.extend(rectangle(2.0, 2.0, 2.0, 2.0, true));
    let group = group(vec![mapped(contours, (0.0, 0.0), (1.0, 1.0), 0.0)]);
    let fill = rasterize(&mut RasterContext::new().unwrap(), &group).unwrap();
    let (expanded, _) = stroke::rasterize_expanded(
        &mut RasterContext::new().unwrap(),
        &group,
        q6(64),
        q6(0),
        q6(0),
        Limits::default().path_segments_per_frame(),
    )
    .unwrap();
    let border = stroke::subtract_coverage(&expanded, &fill, &Limits::default()).unwrap();
    assert_eq!(fill.bounds(), PixelRect::new(0, 0, 6, 6).unwrap());
    assert_eq!(expanded.bounds(), PixelRect::new(-1, -1, 7, 7).unwrap());
    assert_eq!(fill.coverage_at(3, 3, PixelOffset::default()), 0);
    assert_eq!(expanded.coverage_at(3, 3, PixelOffset::default()), 255);
    assert_eq!(border.coverage_at(3, 3, PixelOffset::default()), 255);
    assert_eq!(border.coverage_at(1, 1, PixelOffset::default()), 0);
}

#[test]
fn border_coverage_subtraction_saturates_instead_of_wrapping() {
    let limits = Limits::default();
    let expanded = Mask::new(
        PixelRect::new(-1, 0, 1, 1).unwrap(),
        2,
        Arc::from([16, 200]),
    )
    .unwrap();
    let fill = Mask::new(
        PixelRect::new(-1, 0, 1, 1).unwrap(),
        2,
        Arc::from([32, 100]),
    )
    .unwrap();
    let border = stroke::subtract_coverage(&expanded, &fill, &limits).unwrap();
    assert_eq!(border.data(), [0, 100]);
    assert_eq!(border.bounds(), expanded.bounds());
}

#[test]
fn shadow_phase_rasterizes_fractional_geometry_before_coverage() {
    let group = rectangle_group(0.0, 0.0, 1.0, 1.0);
    let shadow =
        stroke::rasterize_translated(&mut RasterContext::new().unwrap(), &group, q6(32), q6(-16))
            .unwrap();
    assert_eq!(shadow.bounds(), PixelRect::new(0, -1, 2, 1).unwrap());
    assert_eq!(shadow.data(), [32, 32, 96, 96]);
}

#[test]
fn fractional_border_width_keeps_q6_support_and_edge_coverage() {
    let group = rectangle_group(0.0, 0.0, 2.0, 2.0);
    let (expanded, _) = stroke::rasterize_expanded(
        &mut RasterContext::new().unwrap(),
        &group,
        q6(32),
        q6(0),
        q6(0),
        Limits::default().path_segments_per_frame(),
    )
    .unwrap();
    assert_eq!(expanded.bounds(), PixelRect::new(-1, -1, 3, 3).unwrap());
    assert_eq!(expanded.coverage_at(-1, 1, PixelOffset::default()), 128);
    assert_eq!(expanded.coverage_at(0, 1, PixelOffset::default()), 255);
}

#[test]
fn border_and_shadow_masks_stay_separate_for_transparent_fill() {
    let group = rectangle_group(1.0, 1.0, 3.0, 3.0);
    let fill = rasterize(&mut RasterContext::new().unwrap(), &group).unwrap();
    let (expanded, _) = stroke::rasterize_expanded(
        &mut RasterContext::new().unwrap(),
        &group,
        q6(64),
        q6(0),
        q6(0),
        Limits::default().path_segments_per_frame(),
    )
    .unwrap();
    let border = stroke::subtract_coverage(&expanded, &fill, &Limits::default()).unwrap();
    assert_eq!(fill.coverage_at(2, 2, PixelOffset::default()), 255);
    assert_eq!(border.coverage_at(2, 2, PixelOffset::default()), 0);
    let frame = compose(
        Size::new(5, 5),
        &[
            DrawOp::new(
                DrawOrder::new(0, EventId::new(0), 1, 0),
                border,
                PixelOffset::default(),
                Paint::Solid(Rgba8::new(255, 0, 0, 255)),
                PaintSpace::Device,
                PaintMap::Identity,
                Arc::new(ResolvedClip::None),
            ),
            DrawOp::new(
                DrawOrder::new(0, EventId::new(0), 2, 0),
                fill,
                PixelOffset::default(),
                Paint::Solid(Rgba8::new(0, 255, 0, 0)),
                PaintSpace::Device,
                PaintMap::Identity,
                Arc::new(ResolvedClip::None),
            ),
        ],
        &Limits::default(),
        &mut OutputBudget::new(&Limits::default()),
    )
    .unwrap();
    let index = (2 * 5 + 2) * 4;
    assert_eq!(&frame.pixels()[index..index + 4], [0, 0, 0, 0]);
    assert!(
        frame
            .pixels()
            .chunks_exact(4)
            .any(|p| p[0] == p[3] && p[3] > 0)
    );
}

#[test]
fn shadow_without_border_uses_translated_fill_geometry() {
    let group = rectangle_group(0.0, 0.0, 1.0, 1.0);
    let shadow =
        stroke::rasterize_translated(&mut RasterContext::new().unwrap(), &group, q6(64), q6(0))
            .unwrap();
    assert_eq!(shadow.bounds(), PixelRect::new(1, 0, 2, 1).unwrap());
    assert_eq!(shadow.data(), [255]);
}

#[test]
fn coincident_shadow_remains_visible_when_fill_paint_is_transparent() {
    let group = rectangle_group(0.0, 0.0, 1.0, 1.0);
    let fill = rasterize(&mut RasterContext::new().unwrap(), &group).unwrap();
    let limits = Limits::default();
    let frame = compose(
        Size::new(1, 1),
        &[
            DrawOp::new(
                DrawOrder::new(0, EventId::new(0), 0, 0),
                fill.clone(),
                PixelOffset::default(),
                Paint::Solid(Rgba8::new(0, 0, 255, 255)),
                PaintSpace::Device,
                PaintMap::Identity,
                Arc::new(ResolvedClip::None),
            ),
            DrawOp::new(
                DrawOrder::new(0, EventId::new(0), 2, 0),
                fill,
                PixelOffset::default(),
                Paint::Solid(Rgba8::new(255, 255, 255, 0)),
                PaintSpace::Device,
                PaintMap::Identity,
                Arc::new(ResolvedClip::None),
            ),
        ],
        &limits,
        &mut OutputBudget::new(&limits),
    )
    .unwrap();
    assert_eq!(frame.pixels(), [0, 0, 255, 255]);
}

#[test]
fn overlapping_glyphs_group_before_translucent_composition() {
    let first = rectangle_group(0.0, 0.0, 2.0, 1.0);
    let second = rectangle_group(1.0, 0.0, 2.0, 1.0);
    let combined = group(vec![
        mapped(
            rectangle(0.0, 0.0, 2.0, 1.0, false),
            (0.0, 0.0),
            (1.0, 1.0),
            0.0,
        ),
        mapped(
            rectangle(0.0, 0.0, 2.0, 1.0, false),
            (1.0, 0.0),
            (1.0, 1.0),
            0.0,
        ),
    ]);
    let combined_mask = rasterize(&mut RasterContext::new().unwrap(), &combined).unwrap();
    let first_mask = rasterize(&mut RasterContext::new().unwrap(), &first).unwrap();
    let second_mask = rasterize(&mut RasterContext::new().unwrap(), &second).unwrap();
    assert_eq!(combined_mask.coverage_at(1, 0, PixelOffset::default()), 255);
    let limits = Limits::default();
    let paint = Paint::Solid(Rgba8::new(255, 0, 0, 128));
    let clip = || Arc::new(ResolvedClip::None);
    let grouped = compose(
        Size::new(3, 1),
        &[DrawOp::new(
            DrawOrder::new(0, EventId::new(0), 2, 0),
            combined_mask.clone(),
            PixelOffset::default(),
            paint,
            PaintSpace::Device,
            PaintMap::Identity,
            clip(),
        )],
        &limits,
        &mut OutputBudget::new(&limits),
    )
    .unwrap();
    let separate = compose(
        Size::new(3, 1),
        &[
            DrawOp::new(
                DrawOrder::new(0, EventId::new(0), 2, 0),
                first_mask.clone(),
                PixelOffset::default(),
                paint,
                PaintSpace::Device,
                PaintMap::Identity,
                clip(),
            ),
            DrawOp::new(
                DrawOrder::new(0, EventId::new(0), 2, 1),
                second_mask.clone(),
                PixelOffset::default(),
                paint,
                PaintSpace::Device,
                PaintMap::Identity,
                clip(),
            ),
        ],
        &limits,
        &mut OutputBudget::new(&limits),
    )
    .unwrap();
    assert_eq!(grouped.pixels()[7], 128);
    assert_eq!(separate.pixels()[7], 192);

    let opaque = Paint::Solid(Rgba8::new(255, 0, 0, 255));
    let grouped_opaque = compose(
        Size::new(3, 1),
        &[DrawOp::new(
            DrawOrder::new(0, EventId::new(0), 2, 0),
            combined_mask,
            PixelOffset::default(),
            opaque,
            PaintSpace::Device,
            PaintMap::Identity,
            clip(),
        )],
        &limits,
        &mut OutputBudget::new(&limits),
    )
    .unwrap();
    let separate_opaque = compose(
        Size::new(3, 1),
        &[
            DrawOp::new(
                DrawOrder::new(0, EventId::new(0), 2, 0),
                first_mask,
                PixelOffset::default(),
                opaque,
                PaintSpace::Device,
                PaintMap::Identity,
                clip(),
            ),
            DrawOp::new(
                DrawOrder::new(0, EventId::new(0), 2, 1),
                second_mask,
                PixelOffset::default(),
                opaque,
                PaintSpace::Device,
                PaintMap::Identity,
                clip(),
            ),
        ],
        &limits,
        &mut OutputBudget::new(&limits),
    )
    .unwrap();
    assert_eq!(grouped_opaque.pixels(), separate_opaque.pixels());
}

#[test]
fn grouped_strokes_do_not_seam_translucent_overlapping_glyphs() {
    let combined = group(vec![
        mapped(
            rectangle(0.0, 0.0, 2.0, 1.0, false),
            (0.0, 0.0),
            (1.0, 1.0),
            0.0,
        ),
        mapped(
            rectangle(0.0, 0.0, 2.0, 1.0, false),
            (1.0, 0.0),
            (1.0, 1.0),
            0.0,
        ),
    ]);
    let fill = rasterize(&mut RasterContext::new().unwrap(), &combined).unwrap();
    let (expanded, _) = stroke::rasterize_expanded(
        &mut RasterContext::new().unwrap(),
        &combined,
        q6(64),
        q6(0),
        q6(0),
        Limits::default().path_segments_per_frame(),
    )
    .unwrap();
    let grouped_border = stroke::subtract_coverage(&expanded, &fill, &Limits::default()).unwrap();
    assert_eq!(grouped_border.coverage_at(2, 0, PixelOffset::default()), 0);

    let mut separate_masks = Vec::new();
    for x in [0.0, 1.0] {
        let one = group(vec![mapped(
            rectangle(0.0, 0.0, 2.0, 1.0, false),
            (x, 0.0),
            (1.0, 1.0),
            0.0,
        )]);
        let one_fill = rasterize(&mut RasterContext::new().unwrap(), &one).unwrap();
        let (one_expanded, _) = stroke::rasterize_expanded(
            &mut RasterContext::new().unwrap(),
            &one,
            q6(64),
            q6(0),
            q6(0),
            Limits::default().path_segments_per_frame(),
        )
        .unwrap();
        separate_masks
            .push(stroke::subtract_coverage(&one_expanded, &one_fill, &Limits::default()).unwrap());
    }
    let limits = Limits::default();
    let clip = || Arc::new(ResolvedClip::None);
    let paint = Paint::Solid(Rgba8::new(255, 0, 0, 128));
    let separate = compose(
        Size::new(3, 1),
        &separate_masks
            .into_iter()
            .enumerate()
            .map(|(ordinal, mask)| {
                DrawOp::new(
                    DrawOrder::new(0, EventId::new(0), 1, ordinal as u32),
                    mask,
                    PixelOffset::default(),
                    paint,
                    PaintSpace::Device,
                    PaintMap::Identity,
                    clip(),
                )
            })
            .collect::<Vec<_>>(),
        &limits,
        &mut OutputBudget::new(&limits),
    )
    .unwrap();
    assert_eq!(separate.pixels()[11], 128);
}

#[test]
fn overlap_pass_order_keeps_all_shadows_before_borders_and_fills() {
    let mask =
        Arc::new(Mask::new(PixelRect::new(0, 0, 1, 1).unwrap(), 1, Arc::from([255])).unwrap());
    let colors = [
        [
            Rgba8::new(255, 0, 0, 128),
            Rgba8::new(0, 255, 0, 128),
            Rgba8::new(0, 0, 255, 128),
        ],
        [
            Rgba8::new(255, 255, 0, 128),
            Rgba8::new(0, 255, 255, 128),
            Rgba8::new(255, 0, 255, 128),
        ],
    ];
    let mut operations = Vec::new();
    for pass in 0..3 {
        for group in 0..2 {
            operations.push(DrawOp::new(
                DrawOrder::new(0, EventId::new(0), pass, group),
                mask.clone(),
                PixelOffset::default(),
                Paint::Solid(colors[group as usize][pass as usize]),
                PaintSpace::Device,
                PaintMap::Identity,
                Arc::new(ResolvedClip::None),
            ));
        }
    }
    let limits = Limits::default();
    let frame = compose(
        Size::new(1, 1),
        &operations,
        &limits,
        &mut OutputBudget::new(&limits),
    )
    .unwrap();
    assert_eq!(frame.pixels(), [140, 56, 224, 252]);
}

#[test]
fn transform_border_is_expanded_after_rotation_and_scale() {
    let group = group(vec![mapped(
        rectangle(0.0, 0.0, 4.0, 2.0, false),
        (10.0, 10.0),
        (2.0, 0.5),
        90.0,
    )]);
    let (expanded, _) = stroke::rasterize_expanded(
        &mut RasterContext::new().unwrap(),
        &group,
        q6(64),
        q6(0),
        q6(0),
        Limits::default().path_segments_per_frame(),
    )
    .unwrap();
    assert_eq!(expanded.bounds(), PixelRect::new(9, 1, 12, 11).unwrap());
}

#[test]
fn negative_shadow_and_partially_offscreen_strokes_keep_full_origins() {
    let group = rectangle_group(-2.0, -1.0, 2.0, 2.0);
    let (expanded, _) = stroke::rasterize_expanded(
        &mut RasterContext::new().unwrap(),
        &group,
        q6(64),
        q6(-32),
        q6(16),
        Limits::default().path_segments_per_frame(),
    )
    .unwrap();
    assert_eq!(expanded.bounds(), PixelRect::new(-4, -2, 1, 3).unwrap());
}

#[test]
fn stroke_limits_reject_tiny_budgets_and_excessive_growth() {
    let group = rectangle_group(1.0, 1.0, 2.0, 2.0);
    let tiny = Limits::default().with_transient_bytes(1).unwrap();
    let error = stroke::rasterize_expanded(
        &mut RasterContext::with_limits(tiny).unwrap(),
        &group,
        q6(64),
        q6(0),
        q6(0),
        Limits::default().path_segments_per_frame(),
    )
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::LimitExceeded);
    let mut context = RasterContext::with_limits(mask_limits()).unwrap();
    let error = stroke::rasterize_expanded(
        &mut context,
        &group,
        q6(2_000_000),
        q6(0),
        q6(0),
        Limits::default().path_segments_per_frame(),
    )
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::LimitExceeded);
}

fn acute_round_join_group() -> CoverageGroup {
    group(vec![mapped(
        vec![
            PathOp::MoveTo(p(-2.0, 8.0)),
            PathOp::LineTo(p(2.0, 0.0)),
            PathOp::LineTo(p(2.25, 8.0)),
            PathOp::Close,
        ],
        (0.0, 0.0),
        (1.0, 1.0),
        0.0,
    )])
}

#[test]
fn round_join_exported_control_hull_is_not_a_configured_limit() {
    let group = acute_round_join_group();
    let mut ctx = RasterContext::new().unwrap();
    let (a, points) =
        stroke::rasterize_expanded(&mut ctx, &group, q6(64), q6(0), q6(0), 4096).unwrap();
    let (b, again) =
        stroke::rasterize_expanded(&mut ctx, &group, q6(64), q6(0), q6(0), 4096).unwrap();
    assert_eq!(a.bounds(), b.bounds());
    assert_eq!(a.data(), b.data());
    assert_eq!(points, again);
    assert!(points > 0 && !a.bounds().is_empty());
    // Nominal source hull +/-1 is [-3,-1,4,9]. Round Bezier controls
    // exceed that box. Preserve them rather than reject or clip the stroke.
    assert!(a.bounds().x0() < -3 || a.bounds().y0() < -1);
    assert!(a.data().iter().any(|&v| v != 0));
    let error =
        stroke::rasterize_expanded(&mut ctx, &group, q6(64), q6(0), q6(0), points - 1)
            .unwrap_err();
    assert_eq!(error.code(), ErrorCode::LimitExceeded);
    assert!(error.detail().contains("check=frame-segments"));
}

#[test]
fn exported_hull_growth_rechecks_real_extent_and_reservation_limits() {
    let group = acute_round_join_group();
    let cap = Limits::default().with_max_dimension(10).unwrap();
    let error = stroke::rasterize_expanded(
        &mut RasterContext::with_limits(cap).unwrap(),
        &group,
        q6(64),
        q6(0),
        q6(0),
        4096,
    )
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::LimitExceeded);
    assert!(error.detail().contains("check=exported-mask-extent"));
    let (mut low, mut high) = (1, 4 * 1024 * 1024);
    while low < high {
        let mid = (low + high) / 2;
        let limits = Limits::default().with_transient_bytes(mid).unwrap();
        let result = stroke::rasterize_expanded(
            &mut RasterContext::with_limits(limits).unwrap(),
            &group,
            q6(64),
            q6(0),
            q6(0),
            4096,
        );
        match result {
            Ok(_) => high = mid,
            Err(e) => {
                assert_eq!(e.code(), ErrorCode::LimitExceeded);
                low = mid + 1;
            }
        }
    }
    let cap = Limits::default().with_transient_bytes(high - 1).unwrap();
    let error = stroke::rasterize_expanded(
        &mut RasterContext::with_limits(cap).unwrap(),
        &group,
        q6(64),
        q6(0),
        q6(0),
        4096,
    )
    .unwrap_err();
    assert_eq!(error.code(), ErrorCode::LimitExceeded);
    assert!(error.detail().contains("check=exported-mask-reservation"));
}
