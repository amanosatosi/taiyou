//! Degenerate xy ellipse: union nonzero-winding eighth-pixel spans along one
//! axis, before coverage accumulation. No A8 maximum filter or epsilon radius.
//! Behavioural reference: Rasterizer.cpp ScanLineData and CreateWidenedRegion;
//! xy_widen_region.cpp gen_left_arc's rx==0 branch.
use std::sync::Arc;
use taiyou_types::*;
type Point = (f64, f64);
#[derive(Clone, Copy)]
struct Edge {
    y: i64,
    x: i64,
    w: i32,
}
#[derive(Clone, Copy)]
struct Span {
    y: i64,
    left: i64,
    right: i64,
}
fn limit() -> RenderError {
    RenderError::new(ErrorCode::LimitExceeded, "single-axis span widening budget")
}
fn push<T>(v: &mut Vec<T>, item: T, used: u64, limits: &Limits) -> Result<(), RenderError> {
    let bytes = (v.len() as u64 + 1)
        .checked_mul(3 * std::mem::size_of::<T>() as u64)
        .and_then(|n| n.checked_add(used))
        .ok_or_else(limit)?;
    if bytes > limits.transient_bytes() {
        return Err(limit().with_resource_failure(ResourceFailure::TransientMemory));
    }
    if v.len() as u64 >= limits.path_segments_per_frame() {
        return Err(limit());
    }
    v.try_reserve(1)
        .map_err(|_| RenderError::new(ErrorCode::AllocationFailed, "span widening allocation"))?;
    v.push(item);
    Ok(())
}
fn line(a: Point, b: Point, edges: &mut Vec<Edge>, limits: &Limits) -> Result<(), RenderError> {
    let (mut a, mut b) = ((a.0 as i64, a.1 as i64), (b.0 as i64, b.1 as i64));
    if a.1 == b.1 {
        return Ok(());
    }
    let w = if a.1 < b.1 { 1 } else { -1 };
    if a.1 > b.1 {
        std::mem::swap(&mut a, &mut b)
    }
    // Sample at the centre of each eighth-pixel row. The source uses a
    // truncated 16.16 slope and rounds crossings to eighth-pixel columns.
    let first = (a.1 + 3).div_euclid(8);
    let last = (b.1 - 5).div_euclid(8);
    let slope = (i128::from(b.0 - a.0) * 65536) / i128::from(b.1 - a.1);
    for y in first..=last {
        let sample = y * 8 + 4;
        let x16 = i128::from(a.0) * 8192 + (slope * i128::from(sample - a.1)).div_euclid(8);
        let x = i64::try_from((x16 + 32768).div_euclid(65536)).map_err(|_| limit())?;
        push(edges, Edge { y, x, w }, 0, limits)?;
    }
    Ok(())
}
fn curve(
    p: [Point; 4],
    edges: &mut Vec<Edge>,
    limits: &Limits,
    count: &mut u64,
) -> Result<(), RenderError> {
    let coefficients = |axis: usize| {
        let [a, b, c, d] = p.map(|p| if axis == 0 { p.0 } else { p.1 });
        [
            a,
            3.0 * (b - a),
            3.0 * (a - 2.0 * b + c),
            -a + 3.0 * b - 3.0 * c + d,
        ]
    };
    let x = coefficients(0);
    let y = coefficients(1);
    let accel = (2.0 * x[2]).abs() + (6.0 * x[3]).abs();
    let accel = accel.max((2.0 * y[2]).abs() + (6.0 * y[3]).abs());
    let step = if accel > 8.0 {
        (8.0 / accel).sqrt()
    } else {
        1.0
    };
    let sample = |t: f64| {
        (
            (x[0] + t * (x[1] + t * (x[2] + t * x[3]))).trunc(),
            (y[0] + t * (y[1] + t * (y[2] + t * y[3]))).trunc(),
        )
    };
    let mut t = 0.0;
    let mut prev = p[0];
    while t < 1.0 {
        let next = sample(t);
        *count = count.checked_add(1).ok_or_else(limit)?;
        if *count > limits.path_segments_per_frame() {
            return Err(limit());
        }
        line(prev, next, edges, limits)?;
        prev = next;
        let next_t = t + step;
        if next_t <= t {
            return Err(limit());
        }
        t = next_t;
    }
    line(prev, sample(1.0), edges, limits)
}
pub(crate) fn rasterize(
    ops: &[PathOp<DevicePoint>],
    rx: Q6,
    ry: Q6,
    remaining: u64,
    limits: &Limits,
) -> Result<(Arc<Mask>, u64), RenderError> {
    rasterize_impl(ops, rx, ry, remaining, limits, false)
}
/// Same supplied geometry, historical scan conversion and native overlay bounds.
/// This is not an A8-to-64 conversion of an already rasterized mask.
pub(crate) fn rasterize_body(
    ops: &[PathOp<DevicePoint>],
    limits: &Limits,
) -> Result<Arc<Mask>, RenderError> {
    rasterize_impl(
        ops,
        Q6::from_ticks(0),
        Q6::from_ticks(0),
        limits.path_segments_per_frame(),
        limits,
        true,
    )
    .map(|(mask, _)| mask)
}
pub(crate) fn rasterize_axis_xy(
    ops: &[PathOp<DevicePoint>],
    rx: Q6,
    ry: Q6,
    remaining: u64,
    limits: &Limits,
) -> Result<(Arc<Mask>, u64), RenderError> {
    rasterize_impl(ops, rx, ry, remaining, limits, true)
}
fn rasterize_impl(
    ops: &[PathOp<DevicePoint>],
    rx: Q6,
    ry: Q6,
    remaining: u64,
    limits: &Limits,
    xy: bool,
) -> Result<(Arc<Mask>, u64), RenderError> {
    check_path_budget(ops.len(), limits)?;
    if rx.ticks() < 0 || ry.ticks() < 0 {
        return Err(RenderError::new(
            ErrorCode::InvalidArgument,
            "negative span widening",
        ));
    }
    let mut hull: Option<(i64, i64, i64, i64)> = None;
    for &op in ops {
        op.try_map(|p| {
            let (x, y) = (i64::from(p.x().ticks()), i64::from(p.y().ticks()));
            hull = Some(hull.map_or((x, y, x, y), |(a, b, c, d)| {
                (a.min(x), b.min(y), c.max(x), d.max(y))
            }));
            Ok(p)
        })?;
    }
    if hull.is_some_and(|(a, b, c, d)| {
        c - a > i64::from(limits.max_dimension()) * 64
            || d - b > i64::from(limits.max_dimension()) * 64
    }) {
        return Err(limit());
    }
    let mut edges = Vec::new();
    let mut prev = (0.0, 0.0);
    let mut first = prev;
    let mut count = 0_u64;
    let point = |p: DevicePoint| (f64::from(p.x().ticks()), f64::from(p.y().ticks()));
    for &op in ops {
        match op {
            PathOp::MoveTo(p) => {
                prev = point(p);
                first = prev;
            }
            PathOp::LineTo(p) => {
                line(prev, point(p), &mut edges, limits)?;
                prev = point(p);
                count = count.checked_add(1).ok_or_else(limit)?;
            }
            PathOp::Close => {
                line(prev, first, &mut edges, limits)?;
                prev = first;
            }
            PathOp::QuadTo(a, b) => {
                let (a, b) = (point(a), point(b));
                let p1 = (
                    prev.0 + (a.0 - prev.0) * 2.0 / 3.0,
                    prev.1 + (a.1 - prev.1) * 2.0 / 3.0,
                );
                let p2 = (b.0 + (a.0 - b.0) * 2.0 / 3.0, b.1 + (a.1 - b.1) * 2.0 / 3.0);
                curve([prev, p1, p2, b], &mut edges, limits, &mut count)?;
                prev = b;
            }
            PathOp::CubicTo(a, b, c) => {
                let c = point(c);
                curve(
                    [prev, point(a), point(b), c],
                    &mut edges,
                    limits,
                    &mut count,
                )?;
                prev = c;
            }
        }
        if count > remaining {
            return Err(limit());
        }
    }
    edges.sort_unstable_by_key(|e| (e.y, e.x, e.w));
    let edge_bytes = edges.capacity() as u64 * std::mem::size_of::<Edge>() as u64;
    let mut spans = Vec::new();
    let mut row = None;
    let mut winding = 0_i64;
    let mut left = 0;
    for e in &edges {
        if row != Some(e.y) {
            row = Some(e.y);
            winding = 0;
        }
        if winding == 0 {
            left = e.x;
        }
        winding += i64::from(e.w);
        if winding == 0 && left < e.x {
            push(
                &mut spans,
                Span {
                    y: e.y,
                    left,
                    right: e.x,
                },
                edge_bytes,
                limits,
            )?;
        }
    }
    drop(edges);
    let rx = (i64::from(rx.ticks()) + 4).div_euclid(8);
    let ry = (i64::from(ry.ticks()) + 4).div_euclid(8);
    let mut wide = Vec::new();
    let span_bytes = spans.capacity() as u64 * std::mem::size_of::<Span>() as u64;
    for s in &spans {
        for y in s.y - ry..=s.y + ry {
            if wide.len() as u64 >= remaining {
                return Err(limit());
            }
            push(
                &mut wide,
                Span {
                    y,
                    left: s.left - rx,
                    right: s.right + rx,
                },
                span_bytes,
                limits,
            )?;
        }
    }
    drop(spans);
    wide.sort_unstable_by_key(|s| (s.y, s.left, s.right));
    let mut out = 0;
    for i in 0..wide.len() {
        let s = wide[i];
        if out > 0 && wide[out - 1].y == s.y && wide[out - 1].right >= s.left {
            wide[out - 1].right = wide[out - 1].right.max(s.right);
        } else {
            wide[out] = s;
            out += 1;
        }
    }
    wide.truncate(out);
    if wide.is_empty() {
        return Ok((
            Arc::new(Mask::new(
                PixelRect::new(0, 0, 0, 0).map_err(|_| limit())?,
                0,
                Arc::from([]),
            )?),
            count,
        ));
    }
    let mut x0 = wide
        .iter()
        .map(|s| s.left)
        .min()
        .ok_or_else(limit)?
        .div_euclid(8);
    let mut y0 = wide[0].y.div_euclid(8);
    let mut x1 = -(-wide.iter().map(|s| s.right).max().ok_or_else(limit)?).div_euclid(8);
    let mut y1 = (wide.last().ok_or_else(limit)?.y).div_euclid(8) + 1;
    if xy {
        // AlignLeftTop rounds the control hull out in eighth units and includes
        // one extra eighth; Rasterize then includes its extra right/bottom pixel.
        let (a, b, c, d) = hull.ok_or_else(limit)?;
        let pad = (rx.max(ry) + 7).div_euclid(8);
        x0 = a.div_euclid(64) - pad;
        y0 = b.div_euclid(64) - pad;
        let end = |n: i64| {
            let eighth = -(-n).div_euclid(8);
            -(-(eighth + 1)).div_euclid(8) + 1
        };
        x1 = end(c) + pad;
        y1 = end(d) + pad;
    }
    let bounds = PixelRect::new(
        i32::try_from(x0).map_err(|_| limit())?,
        i32::try_from(y0).map_err(|_| limit())?,
        i32::try_from(x1).map_err(|_| limit())?,
        i32::try_from(y1).map_err(|_| limit())?,
    )
    .map_err(|_| limit())?;
    if bounds.width() > limits.max_dimension() || bounds.height() > limits.max_dimension() {
        return Err(limit());
    }
    let stride = bounds.width() as usize;
    let length = stride
        .checked_mul(bounds.height() as usize)
        .ok_or_else(limit)?;
    if length as u64 > limits.output_pixels() || length as u64 > limits.output_bytes() {
        return Err(limit());
    }
    let charge = (length as u64)
        .checked_mul(3)
        .and_then(|n| n.checked_add(wide.capacity() as u64 * std::mem::size_of::<Span>() as u64))
        .ok_or_else(limit)?;
    if charge > limits.transient_bytes() {
        return Err(limit().with_resource_failure(ResourceFailure::TransientMemory));
    }
    let mut data = Vec::new();
    data.try_reserve_exact(length)
        .map_err(|_| RenderError::new(ErrorCode::AllocationFailed, "axis coverage allocation"))?;
    data.resize(length, 0_u8);
    for s in &wide {
        let y = (s.y.div_euclid(8) - y0) as usize;
        for x in s.left.div_euclid(8)..=-(-(s.right)).div_euclid(8) - 1 {
            let n = (s.right.min((x + 1) * 8) - s.left.max(x * 8)) as u8;
            let index = y * stride + (x - x0) as usize;
            data[index] = data[index]
                .checked_add(n)
                .filter(|n| *n <= 64)
                .ok_or_else(limit)?;
        }
    }
    if !xy {
        for v in &mut data {
            *v = ((u16::from(*v) * 255 + 32) / 64) as u8;
        }
    }
    let work = count.checked_add(wide.len() as u64).ok_or_else(limit)?;
    if work > remaining {
        return Err(limit());
    }
    let mask = if xy {
        Mask::new_xy(bounds, stride, data.into())
    } else {
        Mask::new(bounds, stride, data.into())
    }?;
    Ok((Arc::new(mask), work))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rectangle(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<PathOp<DevicePoint>> {
        let p = |x, y| DevicePoint::new(Q6::from_ticks(x * 64), Q6::from_ticks(y * 64));
        vec![
            PathOp::MoveTo(p(x0, y0)),
            PathOp::LineTo(p(x1, y0)),
            PathOp::LineTo(p(x1, y1)),
            PathOp::LineTo(p(x0, y1)),
            PathOp::Close,
        ]
    }
    #[test]
    fn zero_axis_expansion_occurs_before_coverage() {
        let ops = rectangle(-2, -1, 2, 1);
        let limits = Limits::default();
        let (x, _) = rasterize(
            &ops,
            Q6::from_ticks(64),
            Q6::from_ticks(0),
            1_000_000,
            &limits,
        )
        .unwrap();
        assert_eq!(x.bounds(), PixelRect::new(-3, -1, 3, 1).unwrap());
        assert!(x.data().iter().all(|&c| c == 255));
        let (y, _) = rasterize(
            &ops,
            Q6::from_ticks(0),
            Q6::from_ticks(64),
            1_000_000,
            &limits,
        )
        .unwrap();
        assert_eq!(y.bounds(), PixelRect::new(-2, -2, 2, 2).unwrap());
        assert!(y.data().iter().all(|&c| c == 255));
        assert_eq!(
            rasterize(&ops, Q6::from_ticks(64), Q6::from_ticks(0), 1, &limits)
                .unwrap_err()
                .code(),
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            rasterize(
                &ops,
                Q6::from_ticks(64),
                Q6::from_ticks(0),
                1_000_000,
                &limits.with_transient_bytes(1).unwrap()
            )
            .unwrap_err()
            .code(),
            ErrorCode::LimitExceeded
        );
    }
}
