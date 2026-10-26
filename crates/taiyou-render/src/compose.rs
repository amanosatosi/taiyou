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
        let mut x0 = (i64::from(bounds.x0()) + i64::from(offset.x())).max(0);
        let mut y0 = (i64::from(bounds.y0()) + i64::from(offset.y())).max(first_row as i64);
        let mut x1 = (i64::from(bounds.x1()) + i64::from(offset.x())).min(i64::from(size.width()));
        let mut y1 = (i64::from(bounds.y1()) + i64::from(offset.y())).min(last_row as i64);
        let unclipped = match operation.clip() {
            ResolvedClip::None => true,
            ResolvedClip::Rectangle { bounds, inverse: false } => {
                x0 = x0.max(i64::from(bounds.x0()));
                y0 = y0.max(i64::from(bounds.y0()));
                x1 = x1.min(i64::from(bounds.x1()));
                y1 = y1.min(i64::from(bounds.y1()));
                true
            }
            _ => false,
        };
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        let Paint::Solid(paint) = operation.paint();
        if unclipped {
            let region = [x0, y0, x1, y1];
            if operation.mask().is_xy_coverage() {
                paint_mask_rows::<true>(operation, region, stride, first_row, pixels, paint)?;
            } else {
                paint_mask_rows::<false>(operation, region, stride, first_row, pixels, paint)?;
            }
            continue;
        }
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

// The operation/mask intersection proves each source/destination row extent.
// Check the slice once per row, then traverse coverage and RGBA in lockstep.
// Keep complex/inverse clips on the original coordinate-sampling path above.
fn paint_mask_rows<const XY: bool>(
    operation: &DrawOp,
    [x0, y0, x1, y1]: [i64; 4],
    stride: usize,
    first_row: usize,
    pixels: &mut [u8],
    paint: Rgba8,
) -> Result<(), RenderError> {
    let invalid = || RenderError::new(ErrorCode::InternalError, "validated composition row unavailable");
    let mask = operation.mask();
    let offset = operation.offset();
    let source_x = usize::try_from(x0 - i64::from(offset.x()) - i64::from(mask.bounds().x0()))
        .map_err(|_| invalid())?;
    let count = usize::try_from(x1 - x0).map_err(|_| invalid())?;
    let destination_x = usize::try_from(x0).map_err(|_| invalid())?;
    let row_bytes = count.checked_mul(4).ok_or_else(invalid)?;
    for y in y0..y1 {
        let source_y = usize::try_from(y - i64::from(offset.y()) - i64::from(mask.bounds().y0()))
            .map_err(|_| invalid())?;
        let source_start = source_y.checked_mul(mask.stride())
            .and_then(|n| n.checked_add(source_x)).ok_or_else(invalid)?;
        let source_end = source_start.checked_add(count).ok_or_else(invalid)?;
        let source = mask.data().get(source_start..source_end).ok_or_else(invalid)?;
        let destination_y = usize::try_from(y - first_row as i64).map_err(|_| invalid())?;
        let destination_start = destination_y.checked_mul(stride)
            .and_then(|n| destination_x.checked_mul(4).and_then(|x| n.checked_add(x)))
            .ok_or_else(invalid)?;
        let destination_end = destination_start.checked_add(row_bytes).ok_or_else(invalid)?;
        let destination = pixels.get_mut(destination_start..destination_end).ok_or_else(invalid)?;
        for (pixel, &coverage) in destination.chunks_exact_mut(4).zip(source) {
            let pixel: &mut [u8; 4] = pixel.try_into().map_err(|_| invalid())?;
            if XY {
                crate::paint::blend_xy_pixel(pixel, paint, coverage, 255);
            } else {
                blend_pixel(pixel, paint, coverage, 255);
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
    fn row_traversal_matches_original_coordinate_sampling() {
        let limits = Limits::default();
        let size = Size::new(37, 29);
        for seed in 0..20_u32 {
            let mut operations = Vec::new();
            for n in 0..9_u32 {
                let bounds = PixelRect::new(-5, -3, 30, 25).unwrap();
                let stride = bounds.width() as usize + 7;
                let mut data = vec![0; stride * bounds.height() as usize];
                for (y, row) in data.chunks_exact_mut(stride).enumerate() {
                    for (x, coverage) in row[..bounds.width() as usize].iter_mut().enumerate() {
                        let value = ((x + y * 7 + seed as usize * 13 + n as usize * 19) % 65) as u8;
                        *coverage = if n % 2 == 0 { value } else { value.saturating_mul(4) };
                    }
                }
                let mask = Arc::new(if n % 2 == 0 {
                    Mask::new_xy(bounds, stride, data.into())
                } else {
                    Mask::new(bounds, stride, data.into())
                }.unwrap());
                let rectangle = PixelRect::new(3, 5, 26, 23).unwrap();
                let clip = match (seed + n) % 6 {
                    0 => ResolvedClip::None,
                    1 => ResolvedClip::Rectangle { bounds: rectangle, inverse: false },
                    2 => ResolvedClip::Rectangle { bounds: rectangle, inverse: true },
                    3 => ResolvedClip::Rectangle { bounds: PixelRect::new(0, 0, 0, 0).unwrap(), inverse: false },
                    4 => ResolvedClip::MaskCoverage { mask: mask.clone(), offset: PixelOffset::new(7, -3), inverse: seed % 2 == 0 },
                    _ => ResolvedClip::Intersection(Arc::new(ResolvedClip::None), Arc::new(ResolvedClip::Rectangle { bounds: rectangle, inverse: false })),
                };
                let offset = if n == 8 {
                    PixelOffset::new(i32::MAX, i32::MIN)
                } else {
                    PixelOffset::new(n as i32 * 3 - 9, 9 - n as i32 * 2)
                };
                operations.push(DrawOp::new(
                    DrawOrder::new(0, EventId::new(0), 2, n), mask, offset,
                    Paint::Solid(Rgba8::new((seed * 11) as u8, 127, (n * 29) as u8, ((seed + n) * 37) as u8)),
                    PaintSpace::Device, PaintMap::Identity, Arc::new(clip),
                ));
            }
            // Independent pre-optimization coordinate sampler: do not derive
            // expected pixels through the row traversal being tested.
            let mut expected = vec![0; size.width() as usize * size.height() as usize * 4];
            for operation in &operations {
                let Paint::Solid(paint) = operation.paint();
                for y in 0..i64::from(size.height()) {
                    for x in 0..i64::from(size.width()) {
                        let index = (y as usize * size.width() as usize + x as usize) * 4;
                        let pixel = (&mut expected[index..index + 4]).try_into().unwrap();
                        let blend = if operation.mask().is_xy_coverage() { crate::paint::blend_xy_pixel } else { blend_pixel };
                        blend(pixel, paint, operation.mask().coverage_at(x, y, operation.offset()), operation.clip().coverage_at(x, y));
                    }
                }
            }
            let actual = compose(size, &operations, &limits, &mut OutputBudget::new(&limits)).unwrap();
            assert_eq!(actual.pixels(), expected, "seed={seed}");
        }
    }

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
