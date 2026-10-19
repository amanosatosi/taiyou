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
    for operation in operations {
        let bounds = operation.mask().bounds();
        let offset = operation.offset();
        let x0 = (i64::from(bounds.x0()) + i64::from(offset.x())).max(0);
        let y0 = (i64::from(bounds.y0()) + i64::from(offset.y())).max(0);
        let x1 = (i64::from(bounds.x1()) + i64::from(offset.x())).min(i64::from(size.width()));
        let y1 = (i64::from(bounds.y1()) + i64::from(offset.y())).min(i64::from(size.height()));
        if x0 >= x1 || y0 >= y1 {
            continue;
        }
        let Paint::Solid(paint) = operation.paint();
        for y in y0..y1 {
            for x in x0..x1 {
                let row = usize::try_from(y)
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
                blend_pixel(
                    destination,
                    paint,
                    operation.mask().coverage_at(x, y, offset),
                    operation.clip().coverage_at(x, y),
                );
            }
        }
    }
    Frame::from_owned_pixels(size, pixels, limits, reservation)
}
