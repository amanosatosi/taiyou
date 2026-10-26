use crate::blend_pixel;
use taiyou_types::*;

/// Composes caller-supplied operations in strictly increasing C6 DrawOrder.
/// Returns a complete clear/painted owned frame or an error, never partial output.
/// No subtitle renderer, rasterizer or implicit ordering policy is introduced.
pub fn compose(
    size: Size,
    operations: &[DrawOp],
    limits: &Limits,
    output: &mut OutputBudget,
) -> Result<Frame, RenderError> {
    compose_inner(size, operations, limits, output, None, 64)
}

pub(crate) fn compose_inner(
    size: Size,
    operations: &[DrawOp],
    limits: &Limits,
    output: &mut OutputBudget,
    pool: Option<&rayon::ThreadPool>,
    band_rows: usize,
) -> Result<Frame, RenderError> {
    let (stride, length) = limits
        .check_output_size(size)
        .map_err(|e| RenderError::new(e.code(), e.detail()))?;
    if output.limit() != limits.output_bytes() {
        return Err(RenderError::new(
            ErrorCode::InvalidArgument,
            "output origin and request limits differ",
        ));
    }
    for pair in operations.windows(2) {
        if pair[0].order() == pair[1].order() {
            return Err(RenderError::new(
                ErrorCode::InternalError,
                "duplicate composition key",
            ));
        }
        if pair[0].order() > pair[1].order() {
            return Err(RenderError::new(
                ErrorCode::InvalidArgument,
                "draw operations must be in canonical order",
            ));
        }
    }
    let bytes = u64::try_from(length)
        .map_err(|_| RenderError::new(ErrorCode::LimitExceeded, "output length overflow"))?;
    // Vec-to-Arc publication may temporarily retain the Vec while allocating the
    // final Arc. Charge that temporary pixel buffer against the transient cap.
    if bytes > limits.transient_bytes() {
        return Err(RenderError::transient_limit(
            "composition transient byte limit exceeded",
        ));
    }
    let reservation = output.reserve(bytes)?;
    let mut pixels = Vec::new();
    pixels
        .try_reserve_exact(length)
        .map_err(|_| RenderError::new(ErrorCode::AllocationFailed, "frame allocation failed"))?;
    pixels.resize(length, 0);
    if let Some(pool) = pool {
        use rayon::prelude::*;
        // Safe, disjoint output slices. Each pixel sees the entire canonical
        // operation order; complete masks/effect halos are immutable inputs.
        let chunk = stride
            .checked_mul(band_rows)
            .unwrap_or(length)
            .min(length)
            .max(1);
        pool.install(|| {
            pixels
                .par_chunks_mut(chunk)
                .enumerate()
                .try_for_each(|(band, pixels)| {
                    paint_rows(size, operations, stride, band * (chunk / stride), pixels)
                })
        })?;
    } else {
        paint_rows(size, operations, stride, 0, &mut pixels)?;
    }
    Frame::from_owned_pixels(size, pixels, limits, reservation)
}

fn paint_rows(
    size: Size,
    operations: &[DrawOp],
    stride: usize,
    first_row: usize,
    pixels: &mut [u8],
) -> Result<(), RenderError> {
    let last_row = first_row + pixels.len() / stride;
    for operation in operations {
        let bounds = operation.mask().bounds();
        let offset = operation.offset();
        let x0 = (i64::from(bounds.x0()) + i64::from(offset.x())).max(0);
        let y0 = (i64::from(bounds.y0()) + i64::from(offset.y())).max(first_row as i64);
        let x1 = (i64::from(bounds.x1()) + i64::from(offset.x())).min(i64::from(size.width()));
        let y1 = (i64::from(bounds.y1()) + i64::from(offset.y())).min(last_row as i64);
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        let Paint::Solid(paint) = operation.paint();
        for y in y0..y1 {
            for x in x0..x1 {
                let row = usize::try_from(y - first_row as i64)
                    .ok()
                    .and_then(|row| row.checked_mul(stride));
                let index = row.and_then(|row| {
                    usize::try_from(x)
                        .ok()
                        .and_then(|col| col.checked_mul(4))
                        .and_then(|col| row.checked_add(col))
                });
                let range = index.and_then(|index| index.checked_add(4).map(|end| index..end));
                let destination =
                    range
                        .and_then(|range| pixels.get_mut(range))
                        .ok_or_else(|| {
                            RenderError::new(
                                ErrorCode::InternalError,
                                "validated destination range unavailable",
                            )
                        })?;
                let destination: &mut [u8; 4] = destination.try_into().map_err(|_| {
                    RenderError::new(ErrorCode::InternalError, "invalid pixel extent")
                })?;
                let blend = if operation.mask().is_xy_coverage() {
                    crate::paint::blend_xy_pixel
                } else {
                    blend_pixel
                };
                blend(
                    destination,
                    paint,
                    operation.mask().coverage_at(x, y, offset),
                    operation.clip().coverage_at(x, y),
                );
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod parallel_tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn disjoint_bands_match_scalar_order_clips_units_and_budget_failures() {
        let limits = Limits::default();
        let size = Size::new(137, 131);
        for workers in [2, 4, 8] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(workers)
                .build()
                .unwrap();
            assert_eq!(pool.current_num_threads(), workers);
            for seed in 0..20_u32 {
                let mut operations = Vec::new();
                for n in 0..9_u32 {
                    let bounds = PixelRect::new(
                        -7 + n as i32,
                        -5 + n as i32,
                        113 + n as i32,
                        120 + n as i32,
                    )
                    .unwrap();
                    let data: Arc<[u8]> = (0..bounds.width() * bounds.height())
                        .map(|i| ((i * 13 + n * 19 + seed * 23) % 65) as u8)
                        .collect::<Vec<_>>()
                        .into();
                    let mask = if n % 2 == 0 {
                        Mask::new_xy(bounds, bounds.width() as usize, data)
                    } else {
                        Mask::new(bounds, bounds.width() as usize, data)
                    }
                    .unwrap();
                    let mask = Arc::new(mask);
                    let clip = if n % 3 == 0 {
                        ResolvedClip::MaskCoverage {
                            mask: mask.clone(),
                            offset: PixelOffset::new(7, -3),
                            inverse: n % 2 == 0,
                        }
                    } else {
                        ResolvedClip::None
                    };
                    operations.push(DrawOp::new(
                        DrawOrder::new(0, EventId::new(0), 2, n),
                        mask,
                        PixelOffset::new(n as i32 - 3, 5 - n as i32),
                        Paint::Solid(Rgba8::new((seed * 11) as u8, 70, (n * 23) as u8, 128)),
                        PaintSpace::Device,
                        PaintMap::Identity,
                        Arc::new(clip),
                    ));
                }
                let expected =
                    compose(size, &operations, &limits, &mut OutputBudget::new(&limits)).unwrap();
                for rows in [1, 16, 32, 64, 128, usize::MAX] {
                    let actual = compose_inner(
                        size,
                        &operations,
                        &limits,
                        &mut OutputBudget::new(&limits),
                        Some(&pool),
                        rows,
                    )
                    .unwrap();
                    assert_eq!(
                        expected.pixels(),
                        actual.pixels(),
                        "workers={workers}, rows={rows}, seed={seed}"
                    );
                }
                let small = limits.clone().with_transient_bytes(1).unwrap();
                let a =
                    compose(size, &operations, &small, &mut OutputBudget::new(&small)).unwrap_err();
                let b = compose_inner(
                    size,
                    &operations,
                    &small,
                    &mut OutputBudget::new(&small),
                    Some(&pool),
                    16,
                )
                .unwrap_err();
                assert_eq!(a.code(), b.code());
                assert_eq!(a.resource_failure(), b.resource_failure());
            }
        }
    }
}
