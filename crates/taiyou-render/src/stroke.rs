//! CoreV1 isotropic device-space stroke masks and shadow geometry placement.
#[cfg(feature = "native")]
use crate::raster::RasterContext;
#[cfg(feature = "native")]
use crate::raster::{CoverageGroup, CoveragePass, coverage_path};
use std::sync::Arc;
use taiyou_types::*;

fn limited() -> RenderError {
    RenderError::new(
        ErrorCode::LimitExceeded,
        "stroke path or coverage allocation limit",
    )
}

#[cfg(feature = "native")]
fn bounds_for_path(ops: &[PathOp<DevicePoint>]) -> Result<PixelRect, RenderError> {
    let mut bounds: Option<(i64, i64, i64, i64)> = None;
    let mut point = |p: DevicePoint| {
        let (x, y) = (i64::from(p.x().ticks()), i64::from(p.y().ticks()));
        bounds = Some(bounds.map_or((x, y, x, y), |(x0, y0, x1, y1)| {
            (x0.min(x), y0.min(y), x1.max(x), y1.max(y))
        }));
    };
    for &op in ops {
        match op {
            PathOp::MoveTo(p) | PathOp::LineTo(p) => point(p),
            PathOp::QuadTo(a, b) => {
                point(a);
                point(b);
            }
            PathOp::CubicTo(a, b, c) => {
                point(a);
                point(b);
                point(c);
            }
            PathOp::Close => (),
        }
    }
    let Some((x0, y0, x1, y1)) = bounds else {
        return PixelRect::new(0, 0, 0, 0).map_err(|_| limited());
    };
    let floor = |n: i64| i32::try_from(n.div_euclid(64)).map_err(|_| limited());
    let ceil = |n: i64| i32::try_from(-(-n).div_euclid(64)).map_err(|_| limited());
    PixelRect::new(floor(x0)?, floor(y0)?, ceil(x1)?, ceil(y1)?).map_err(|_| limited())
}

#[cfg(feature = "native")]
fn check_group(group: &CoverageGroup) -> Result<(), RenderError> {
    if group.pass() != CoveragePass::Fill || group.policy().profile() != Profile::CoreV1 {
        return Err(RenderError::new(
            ErrorCode::UnsupportedFeature,
            "CoreV1 stroke masks require a nonzero fill geometry group",
        ));
    }
    Ok(())
}

/// Rasterize translated geometry before antialiasing. Fractional offsets change
/// Q6 phase and are never applied to a previously rasterized mask.
#[cfg(feature = "native")]
pub fn rasterize_translated(
    context: &mut RasterContext,
    group: &CoverageGroup,
    translate_x: Q6,
    translate_y: Q6,
) -> Result<Arc<Mask>, RenderError> {
    check_group(group)?;
    let ops = coverage_path(group, translate_x, translate_y, context.limits())?;
    let bounds = bounds_for_path(&ops)?;
    if bounds.is_empty() {
        return Ok(Arc::new(Mask::new(bounds, 0, Arc::from([]))?));
    }
    let stride = usize::try_from(bounds.width()).map_err(|_| limited())?;
    context.rasterize_path(&ops, bounds, stride)
}

/// Expand the combined group's transformed contours with the worker-local
/// FreeType stroker. Returns E and the number of generated native path points
/// for deterministic frame-wide segment accounting.
#[cfg(feature = "native")]
pub fn rasterize_expanded(
    context: &mut RasterContext,
    group: &CoverageGroup,
    radius: Q6,
    translate_x: Q6,
    translate_y: Q6,
    remaining_segments: u64,
) -> Result<(Arc<Mask>, u64), RenderError> {
    check_group(group)?;
    let ops = coverage_path(group, translate_x, translate_y, context.limits())?;
    if ops.is_empty() {
        return Ok((
            Arc::new(Mask::new(
                PixelRect::new(0, 0, 0, 0).map_err(|_| limited())?,
                0,
                Arc::from([]),
            )?),
            0,
        ));
    }
    context.stroke_rasterize_path(&ops, radius, remaining_segments)
}

/// Core outline coverage B=max(E-F,0), using saturating A8 subtraction.
/// The result retains E's full support bounds and initialized row padding.
pub fn subtract_coverage(
    expanded: &Mask,
    fill: &Mask,
    limits: &Limits,
) -> Result<Arc<Mask>, RenderError> {
    let bounds = expanded.bounds();
    if bounds.is_empty() {
        return Ok(Arc::new(Mask::new(bounds, 0, Arc::from([]))?));
    }
    let (width, height) = (
        usize::try_from(bounds.width()).map_err(|_| limited())?,
        usize::try_from(bounds.height()).map_err(|_| limited())?,
    );
    if bounds.width() > limits.max_dimension() || bounds.height() > limits.max_dimension() {
        return Err(limited());
    }
    let length = width.checked_mul(height).ok_or_else(limited)?;
    let pixels = u64::try_from(length).map_err(|_| limited())?;
    let transient = pixels.checked_mul(3).ok_or_else(limited)?;
    if pixels > limits.output_pixels() || pixels > limits.output_bytes() {
        return Err(limited());
    }
    if transient > limits.transient_bytes() {
        return Err(limited().with_resource_failure(ResourceFailure::TransientMemory));
    }
    let mut data = Vec::new();
    data.try_reserve_exact(length).map_err(|_| {
        RenderError::new(
            ErrorCode::AllocationFailed,
            "border subtraction allocation failed",
        )
    })?;
    for y in i64::from(bounds.y0())..i64::from(bounds.y1()) {
        for x in i64::from(bounds.x0())..i64::from(bounds.x1()) {
            data.push(
                expanded
                    .coverage_at(x, y, PixelOffset::default())
                    .saturating_sub(fill.coverage_at(x, y, PixelOffset::default())),
            );
        }
    }
    if data.iter().all(|&coverage| coverage == 0) {
        return Ok(Arc::new(Mask::new(
            PixelRect::new(0, 0, 0, 0).map_err(|_| limited())?,
            0,
            Arc::from([]),
        )?));
    }
    Ok(Arc::new(Mask::new(bounds, width, data.into())?))
}
