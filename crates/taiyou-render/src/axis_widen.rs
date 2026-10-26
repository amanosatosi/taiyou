//! Union nonzero-winding eighth-pixel spans with a discrete ellipse before
//! coverage accumulation. No A8 maximum filter or vector contour offset.
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
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Span {
    y: i64,
    left: i64,
    right: i64,
}

// Partition by integer scan row, then sort only intervals that can intersect.
// The resulting order is exactly (y,left,right), as in the original full sort.
// Scratch is optional and fits inside the producer's existing peak witness;
// allocation/range failures retain the original in-place sort and its limits.
fn sort_by_scan_row(
    spans: &mut Vec<Span>,
    producer_bytes: u64,
    limits: &Limits,
    mut timings: Option<&mut crate::timing::StageTimings>,
) -> bool {
    if spans.len() < 2 {
        return false;
    }
    let first = spans.iter().map(|s| s.y).min().unwrap_or(0);
    let last = spans.iter().map(|s| s.y).max().unwrap_or(0);
    let Some(rows) = last
        .checked_sub(first)
        .and_then(|n| n.checked_add(1))
        .and_then(|n| usize::try_from(n).ok())
        .filter(|&n| n <= spans.len())
    else {
        return false;
    };
    let Some(peak) = (spans.len() as u64)
        .checked_mul(3 * std::mem::size_of::<Span>() as u64)
        .and_then(|n| n.checked_add(producer_bytes))
    else {
        return false;
    };
    let Some(live) = (spans.capacity() as u64)
        .checked_mul(std::mem::size_of::<Span>() as u64)
        .and_then(|n| n.checked_add(producer_bytes))
    else {
        return false;
    };
    let Some(scratch) = (rows as u64).checked_mul(2 * std::mem::size_of::<usize>() as u64) else {
        return false;
    };
    let allowed = peak.min(limits.transient_bytes());
    if live.checked_add(scratch).is_none_or(|n| n > allowed) {
        return false;
    }
    let mut ends = Vec::<usize>::new();
    let mut next = Vec::<usize>::new();
    if reserve_observed(&mut ends, rows, true, timings.as_deref_mut()).is_err()
        || reserve_observed(&mut next, rows, true, timings.as_deref_mut()).is_err()
    {
        return false;
    }
    let Some(actual) = ends
        .capacity()
        .checked_add(next.capacity())
        .and_then(|n| n.checked_mul(std::mem::size_of::<usize>()))
        .and_then(|n| live.checked_add(n as u64))
    else {
        return false;
    };
    if actual > allowed {
        return false;
    }
    ends.resize(rows, 0);
    next.resize(rows, 0);
    // min/max and the checked row count prove these indices fit. Counts and
    // cumulative ends are bounded by the input slice length.
    for span in spans.iter() {
        ends[(span.y - first) as usize] += 1;
    }
    let mut end = 0;
    for (cursor, count) in next.iter_mut().zip(ends.iter_mut()) {
        *cursor = end;
        end += *count;
        *count = end;
    }
    for row in 0..rows {
        while next[row] < ends[row] {
            let index = next[row];
            let destination_row = (spans[index].y - first) as usize;
            if destination_row == row {
                next[row] += 1;
            } else {
                let destination = next[destination_row];
                spans.swap(index, destination);
                next[destination_row] += 1;
            }
        }
    }
    let mut start = 0;
    let timer = timings.as_ref().and_then(|t| t.start());
    for end in ends {
        spans[start..end].sort_unstable_by_key(|s| (s.left, s.right));
        start = end;
    }
    if let Some(t) = timings.as_mut() {
        t.span_sorting += crate::timing::StageTimings::elapsed(timer);
    }
    true
}
#[derive(Clone, Copy)]
struct VerticalBand {
    first_y: i64,
    last_y: i64,
    left: i64,
    right: i64,
}
/// Equal intervals on consecutive scan rows have nested ellipse expansions.
/// Their union at an output row needs only the nearest source row. Retain the
/// original producer when this proof or the optional scratch reservation fails.
fn vertical_bands(
    spans: &mut [Span],
    arc: &[i64],
    source_bytes: u64,
    arc_bytes: u64,
    limits: &Limits,
    mut timings: Option<&mut crate::timing::StageTimings>,
) -> Option<(Vec<VerticalBand>, u64)> {
    if spans.len() < 2 || arc.len() < 2 || !arc.windows(2).all(|w| w[0] >= w[1]) {
        return None;
    }
    let planned = (spans.len() as u64)
        .checked_mul(3 * std::mem::size_of::<VerticalBand>() as u64)?
        .checked_add(source_bytes)?;
    if planned > limits.transient_bytes() {
        return None;
    }
    let mut bands: Vec<VerticalBand> = Vec::new();
    reserve_observed(&mut bands, spans.len(), true, timings.as_deref_mut()).ok()?;
    let band_bytes =
        (bands.capacity() as u64).checked_mul(std::mem::size_of::<VerticalBand>() as u64)?;
    let build_peak = band_bytes.checked_mul(3)?.checked_add(source_bytes)?;
    if build_peak > limits.transient_bytes() {
        return None;
    }
    let sort_timer = timings.as_ref().and_then(|t| t.start());
    spans.sort_unstable_by_key(|s| (s.left, s.right, s.y));
    if let Some(t) = timings.as_mut() {
        t.span_sorting += crate::timing::StageTimings::elapsed(sort_timer);
    }
    for s in spans.iter() {
        if let Some(last) = bands
            .last_mut()
            .filter(|b| b.left == s.left && b.right == s.right && s.y <= b.last_y + 1)
        {
            last.last_y = last.last_y.max(s.y);
        } else {
            bands.push(VerticalBand {
                first_y: s.y,
                last_y: s.y,
                left: s.left,
                right: s.right,
            });
        }
    }
    if bands.len() == spans.len() {
        return None;
    }
    let ry = (arc.len() - 1) as u64;
    let generated = bands.iter().try_fold(0_u64, |n, b| {
        n.checked_add((b.last_y - b.first_y + 1) as u64)?
            .checked_add(2 * ry)
    })?;
    let original = (spans.len() as u64).checked_mul(2 * ry + 1)?;
    // Never make an originally successful widening fail by choosing larger
    // optional scratch. Final mask size is unchanged and wide capacity shrinks.
    let new_peak = generated
        .checked_mul(3 * std::mem::size_of::<Span>() as u64)?
        .checked_add(band_bytes)?
        .checked_add(arc_bytes)?;
    let old_peak = original
        .checked_mul(3 * std::mem::size_of::<Span>() as u64)?
        .checked_add(source_bytes)?;
    if new_peak > old_peak {
        return None;
    }
    Some((bands, build_peak))
}
fn limit() -> RenderError {
    RenderError::new(ErrorCode::LimitExceeded, "single-axis span widening budget")
}

// The tiled slice must retain the old deterministic rejection and cache witness,
// even though it no longer needs the legacy Edge/Span allocations. Keep this
// witness equivalent to the retained producer, as checked by focused native tests.
pub(crate) struct FillWitness {
    pub peak: u64,
    pub required_work: u64,
}
fn legacy_capacity<T>(length: u64) -> Option<u64> {
    if length == 0 {
        return Some(0);
    }
    let length = usize::try_from(length).ok()?;
    let minimum = if std::mem::size_of::<T>() == 1 { 8 } else { 4 };
    let capacity = length.max(minimum).checked_next_power_of_two()?;
    let bytes = capacity.checked_mul(std::mem::size_of::<T>())?;
    if bytes > isize::MAX as usize {
        return None;
    }
    Some(bytes as u64)
}
fn legacy_push_check<T>(length: u64, used: u64, limits: &Limits) -> Result<(), RenderError> {
    if length == 0 {
        return Ok(());
    }
    let each = 3 * std::mem::size_of::<T>() as u64;
    let overflow = (u64::MAX - used) / each;
    let memory = limits.transient_bytes().saturating_sub(used) / each;
    let work = limits.path_segments_per_frame();
    if length <= overflow.min(memory).min(work) {
        return Ok(());
    }
    // The original push checks arithmetic, then transient bytes, then work.
    // Preserve the first failing push, not merely the final aggregate failure.
    if overflow <= memory && overflow <= work && length > overflow {
        return Err(limit());
    }
    if memory <= work && length > memory {
        return Err(limit().with_resource_failure(ResourceFailure::TransientMemory));
    }
    Err(limit())
}
pub(crate) fn legacy_fill_witness(
    ops: usize,
    count: u64,
    edges: u64,
    intervals: u64,
    union: u64,
    bounds: PixelRect,
    limits: &Limits,
) -> Result<Option<FillWitness>, RenderError> {
    check_path_budget(ops, limits)?;
    let mut peak = (ops as u64)
        .saturating_mul(
            4 * (std::mem::size_of::<PathOp<FontPoint>>()
                + std::mem::size_of::<PathOp<DevicePoint>>()) as u64,
        )
        .saturating_add(4096);
    let mut required = (ops as u64).max(count);
    legacy_push_check::<Edge>(edges, 0, limits)?;
    if count > limits.path_segments_per_frame() {
        return Err(limit());
    }
    let Some(edge_bytes) = legacy_capacity::<Edge>(edges) else {
        return Ok(None);
    };
    let Some(span_bytes) = legacy_capacity::<Span>(intervals) else {
        return Ok(None);
    };
    peak = peak.max(
        edges
            .checked_mul(3 * std::mem::size_of::<Edge>() as u64)
            .ok_or_else(limit)?,
    );
    legacy_push_check::<Span>(intervals, edge_bytes, limits)?;
    peak = peak.max(
        edge_bytes
            .checked_add(
                intervals
                    .checked_mul(3 * std::mem::size_of::<Span>() as u64)
                    .ok_or_else(limit)?,
            )
            .ok_or_else(limit)?,
    );
    legacy_push_check::<i64>(1, span_bytes, limits)?;
    peak = peak.max(
        span_bytes
            .checked_add(3 * std::mem::size_of::<i64>() as u64)
            .ok_or_else(limit)?,
    );
    let producer = span_bytes
        .checked_add(legacy_capacity::<i64>(1).ok_or_else(limit)?)
        .ok_or_else(limit)?;
    legacy_push_check::<Span>(intervals, producer, limits)?;
    peak = peak.max(
        producer
            .checked_add(
                intervals
                    .checked_mul(3 * std::mem::size_of::<Span>() as u64)
                    .ok_or_else(limit)?,
            )
            .ok_or_else(limit)?,
    );
    required = required.max(intervals);
    if intervals == 0 {
        return Ok(Some(FillWitness {
            peak,
            required_work: required,
        }));
    }
    if bounds.width() > limits.max_dimension() || bounds.height() > limits.max_dimension() {
        return Err(limit());
    }
    let length = (bounds.width() as usize)
        .checked_mul(bounds.height() as usize)
        .ok_or_else(limit)? as u64;
    if length > limits.output_pixels() || length > limits.output_bytes() {
        return Err(limit());
    }
    let charge = length
        .checked_mul(3)
        .and_then(|n| n.checked_add(span_bytes))
        .ok_or_else(limit)?;
    if charge > limits.transient_bytes() {
        return Err(limit().with_resource_failure(ResourceFailure::TransientMemory));
    }
    let work = count.checked_add(union).ok_or_else(limit)?;
    peak = peak.max(charge);
    required = required.max(work);
    if work > limits.path_segments_per_frame() {
        return Err(limit());
    }
    Ok(Some(FillWitness {
        peak,
        required_work: required,
    }))
}
fn reserve_observed<T>(
    v: &mut Vec<T>,
    additional: usize,
    exact: bool,
    mut timings: Option<&mut crate::timing::StageTimings>,
) -> Result<(), std::collections::TryReserveError> {
    let capacity = v.capacity();
    let growth = v.capacity() - v.len() < additional;
    let timer = if growth {
        timings.as_ref().and_then(|t| t.start())
    } else {
        None
    };
    let result = if exact {
        v.try_reserve_exact(additional)
    } else {
        v.try_reserve(additional)
    };
    if let Some(t) = timings.as_mut().filter(|t| t.enabled && growth) {
        t.raster_allocation += crate::timing::StageTimings::elapsed(timer);
        t.raster_allocations = t.raster_allocations.saturating_add(1);
        t.raster_reallocations = t
            .raster_reallocations
            .saturating_add(u64::from(capacity != 0));
        t.raster_capacity_growth_bytes = t.raster_capacity_growth_bytes.saturating_add(
            (v.capacity().saturating_sub(capacity) as u64)
                .saturating_mul(std::mem::size_of::<T>() as u64),
        );
    }
    result
}
fn push<T>(
    v: &mut Vec<T>,
    item: T,
    used: u64,
    limits: &Limits,
    timings: Option<&mut crate::timing::StageTimings>,
) -> Result<(), RenderError> {
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
    reserve_observed(v, 1, false, timings)
        .map_err(|_| RenderError::new(ErrorCode::AllocationFailed, "span widening allocation"))?;
    v.push(item);
    Ok(())
}
fn line(
    a: Point,
    b: Point,
    edges: &mut Vec<Edge>,
    limits: &Limits,
    mut timings: Option<&mut crate::timing::StageTimings>,
) -> Result<(), RenderError> {
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
        push(edges, Edge { y, x, w }, 0, limits, timings.as_deref_mut())?;
    }
    Ok(())
}
#[cfg(test)]
fn curve(
    p: [Point; 4],
    edges: &mut Vec<Edge>,
    limits: &Limits,
    count: &mut u64,
    mut timings: Option<&mut crate::timing::StageTimings>,
) -> Result<(), RenderError> {
    walk_curve(p, limits, count, &mut |a, b| {
        line(a, b, edges, limits, timings.as_deref_mut())
    })
}
fn walk_curve(
    p: [Point; 4], limits: &Limits, count: &mut u64,
    emit: &mut impl FnMut(Point, Point) -> Result<(), RenderError>,
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
        emit(prev, next)?;
        prev = next;
        let next_t = t + step;
        if next_t <= t {
            return Err(limit());
        }
        t = next_t;
    }
    // Converted quadratic controls contain thirds. Evaluating the polynomial
    // at 1 can truncate one Q6 tick short of its exact endpoint. The caller
    // continues from p[3], so that lost tick leaves an open scanline winding
    // and can paint a span across the gap to another glyph. Use the supplied
    // endpoint, as for every other path segment. pinterf's _EvaluateLine also
    // reconnects its last emitted point to the next segment's exact start.
    emit(prev, p[3])
}

fn walk_outline(
    ops: &[PathOp<DevicePoint>], origin: (i64, i64), remaining: u64, limits: &Limits,
    emit: &mut impl FnMut(Point, Point) -> Result<(), RenderError>,
) -> Result<u64, RenderError> {
    let point = |p: DevicePoint| ((i64::from(p.x().ticks()) - origin.0) as f64,
        (i64::from(p.y().ticks()) - origin.1) as f64);
    let mut prev = (0.0, 0.0);
    let mut first = prev;
    let mut count = 0_u64;
    for &op in ops {
        match op {
            PathOp::MoveTo(p) => { prev = point(p); first = prev; }
            PathOp::LineTo(p) => {
                emit(prev, point(p))?; prev = point(p);
                count = count.checked_add(1).ok_or_else(limit)?;
            }
            PathOp::Close => { emit(prev, first)?; prev = first; }
            PathOp::QuadTo(a, b) => {
                let (a, b) = (point(a), point(b));
                let p1 = (prev.0 + (a.0 - prev.0) * 2.0 / 3.0,
                    prev.1 + (a.1 - prev.1) * 2.0 / 3.0);
                let p2 = (b.0 + (a.0 - b.0) * 2.0 / 3.0,
                    b.1 + (a.1 - b.1) * 2.0 / 3.0);
                walk_curve([prev, p1, p2, b], limits, &mut count, emit)?; prev = b;
            }
            PathOp::CubicTo(a, b, c) => {
                let c = point(c);
                walk_curve([prev, point(a), point(b), c], limits, &mut count, emit)?; prev = c;
            }
        }
        if count > remaining { return Err(limit()); }
    }
    Ok(count)
}

fn try_tiled_body(
    ops: &[PathOp<DevicePoint>], hull: (i64, i64, i64, i64), origin: (i64, i64),
    limits: &Limits, timings: &mut Option<&mut crate::timing::StageTimings>,
) -> Result<Option<(Arc<Mask>, u64)>, RenderError> {
    // Never reinterpret open or implicit contours as closed filled geometry.
    let mut open = false;
    for op in ops {
        match op {
            PathOp::MoveTo(_) if open => return Ok(None),
            PathOp::MoveTo(_) => open = true,
            PathOp::Close => open = false,
            _ if !open => return Ok(None),
            _ => (),
        }
    }
    if open || ops.is_empty() { return Ok(None); }
    let end = |n: i64| {
        let eighth = -(-n).div_euclid(8);
        -(-(eighth + 1)).div_euclid(8) + 1
    };
    let candidate_bounds = || PixelRect::new(
        i32::try_from(hull.0.div_euclid(64)).ok()?, i32::try_from(hull.1.div_euclid(64)).ok()?,
        i32::try_from(end(hull.2)).ok()?, i32::try_from(end(hull.3)).ok()?).ok();
    let Some(bounds) = candidate_bounds() else { return Ok(None); };
    let length = u64::from(bounds.width()).checked_mul(u64::from(bounds.height()));
    if bounds.width() > limits.max_dimension() || bounds.height() > limits.max_dimension()
        || length.is_none_or(|n| n > limits.output_pixels() || n > limits.output_bytes()) { return Ok(None); }
    let mut segments = Vec::new();
    let mut edges = 0_u64;
    let mut fallback = false;
    let count = walk_outline(ops, origin, limits.path_segments_per_frame(), limits, &mut |a, b| {
        if fallback { return Ok(()); }
        let global = |p: Point| Some([(p.0 as i64).checked_add(origin.0)?, (p.1 as i64).checked_add(origin.1)?]);
        let (Some(a), Some(b)) = (global(a), global(b)) else { fallback = true; return Ok(()); };
        let Some(segment) = crate::tile_fill::Segment::new(a, b) else { return Ok(()); };
        let Some(next) = edges.checked_add(segment.rows()) else { fallback = true; return Ok(()); };
        if legacy_capacity::<Edge>(next).is_none() { fallback = true; return Ok(()); }
        legacy_push_check::<Edge>(next, 0, limits)?;
        edges = next;
        if segments.len() == segments.capacity() {
            let additional = segments.capacity().max(4);
            let planned = segments.capacity().checked_add(additional)
                .and_then(|n| n.checked_mul(3 * std::mem::size_of::<crate::tile_fill::Segment>()))
                .and_then(|n| (n as u64).checked_add(4096));
            if planned.is_none_or(|n| n > limits.transient_bytes()) || segments.try_reserve_exact(additional).is_err() {
                fallback = true; return Ok(());
            }
            if (segments.capacity() as u64).saturating_mul(3 * std::mem::size_of::<crate::tile_fill::Segment>() as u64)
                .saturating_add(4096) > limits.transient_bytes() { fallback = true; return Ok(()); }
        }
        segments.push(segment);
        Ok(())
    });
    if fallback { return Ok(None); }
    let count = count?;
    let held = (segments.capacity() as u64).checked_mul(std::mem::size_of::<crate::tile_fill::Segment>() as u64).ok_or_else(limit)?;
    let Some(image) = crate::tile_fill::rasterize(&segments, bounds, limits.transient_bytes(), held) else { return Ok(None); };
    let intervals = image.source_intervals;
    let union = image.union_intervals;
    let stats = image.stats;
    drop(segments);
    let Some(witness) = legacy_fill_witness(ops.len(), count, edges, intervals, union, bounds, limits)? else { return Ok(None); };
    let (mask, work) = if intervals == 0 {
        (Arc::new(Mask::new(PixelRect::new(0, 0, 0, 0).map_err(|_| limit())?, 0, Arc::from([]))?), count)
    } else {
        let Some(pixels) = image.into_pixels(limits.transient_bytes()) else { return Ok(None); };
        (Arc::new(Mask::new_xy(bounds, bounds.width() as usize, pixels.into())?), count.checked_add(union).ok_or_else(limit)?)
    };
    if let Some(t) = timings.as_mut() {
        t.raster_peak(witness.peak, witness.required_work);
        t.span_rows = t.span_rows.saturating_add(intervals);
        t.expanded_candidates_without_reuse = t.expanded_candidates_without_reuse.saturating_add(intervals);
        t.tiled_fills = t.tiled_fills.saturating_add(1);
        t.tile_empty_regions = t.tile_empty_regions.saturating_add(stats.empty_regions);
        t.tile_solid_regions = t.tile_solid_regions.saturating_add(stats.solid_regions);
        t.tile_halfplanes = t.tile_halfplanes.saturating_add(stats.halfplane_tiles);
        t.tile_complex = t.tile_complex.saturating_add(stats.complex_tiles);
        t.tile_scratch_peak_bytes = t.tile_scratch_peak_bytes.max(stats.scratch_peak);
    }
    Ok(Some((mask, work)))
}

// A grid ellipse includes the nearest boundary cells in both dominant-axis
// directions. Solve the two half-cell squared-distance inequalities directly,
// rather than accumulating a floating-point angle or translating a native
// contour-walking implementation. The quarter-cell term resolves the diagonal
// transition; simply rounding sqrt(a*a*(1-y*y/(b*b))) loses those cells.
fn ellipse_width(rx: i64, ry: i64, y: i64) -> i64 {
    if ry == 0 || rx == 0 {
        return rx;
    }
    let (a, b, y) = (i128::from(rx), i128::from(ry), i128::from(y));
    let (aa, bb) = (a * a, b * b);
    let largest = |vertical: bool| {
        let fits = |x: i128| {
            if vertical {
                4 * x * x * bb + (4 * y * y - 4 * y + 2) * aa <= 4 * aa * bb
            } else {
                ((2 * x - 1).pow(2) + 1) * bb + 4 * y * y * aa <= 4 * aa * bb
            }
        };
        let (mut lo, mut hi) = (0, a + 1);
        while lo + 1 < hi {
            let mid = (lo + hi) / 2;
            if fits(mid) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        lo as i64
    };
    largest(false).max(largest(true))
}
pub(crate) fn rasterize(
    ops: &[PathOp<DevicePoint>],
    rx: Q6,
    ry: Q6,
    remaining: u64,
    limits: &Limits,
) -> Result<(Arc<Mask>, u64), RenderError> {
    rasterize_impl(ops, rx, ry, remaining, limits, false, None, None)
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
        None,
        None,
    )
    .map(|(mask, _)| mask)
}
pub(crate) fn rasterize_body_timed(
    ops: &[PathOp<DevicePoint>],
    limits: &Limits,
    timings: &mut crate::timing::StageTimings,
) -> Result<Arc<Mask>, RenderError> {
    rasterize_selected(
        ops,
        Q6::from_ticks(0),
        Q6::from_ticks(0),
        limits.path_segments_per_frame(),
        limits,
        true,
        None,
        Some(timings),
        true,
    )
    .map(|(mask, _)| mask)
}
#[cfg(test)]
pub(crate) fn rasterize_legacy_body_timed(
    ops: &[PathOp<DevicePoint>], limits: &Limits, timings: &mut crate::timing::StageTimings,
) -> Result<Arc<Mask>, RenderError> {
    rasterize_impl(ops, Q6::from_ticks(0), Q6::from_ticks(0), limits.path_segments_per_frame(), limits, true, None, Some(timings))
        .map(|(mask, _)| mask)
}
pub(crate) fn rasterize_axis_xy_timed(
    ops: &[PathOp<DevicePoint>],
    rx: Q6,
    ry: Q6,
    remaining: u64,
    limits: &Limits,
    timings: &mut crate::timing::StageTimings,
) -> Result<(Arc<Mask>, u64), RenderError> {
    rasterize_impl(ops, rx, ry, remaining, limits, true, None, Some(timings))
}
type Projection<'a> = dyn Fn(DevicePoint) -> Result<DevicePoint, RenderError> + 'a;
// A single-zero-axis border is an exact segment dilation, not an epsilon ellipse.
// Keep its existing spans until expansion is complete, project their closed
// strips, then rasterize once. This fallback creates no intermediate mask.
pub(crate) fn rasterize_projected_axis(
    ops: &[PathOp<DevicePoint>],
    rx: Q6,
    ry: Q6,
    remaining: u64,
    limits: &Limits,
    project: &Projection<'_>,
) -> Result<(Arc<Mask>, u64), RenderError> {
    rasterize_impl(ops, rx, ry, remaining, limits, true, Some(project), None)
}
#[allow(clippy::too_many_arguments)]
fn rasterize_impl(
    ops: &[PathOp<DevicePoint>],
    rx: Q6,
    ry: Q6,
    remaining: u64,
    limits: &Limits,
    xy: bool,
    projection: Option<&Projection<'_>>,
    timings: Option<&mut crate::timing::StageTimings>,
) -> Result<(Arc<Mask>, u64), RenderError> {
    rasterize_selected(ops, rx, ry, remaining, limits, xy, projection, timings, false)
}
#[allow(clippy::too_many_arguments)]
fn rasterize_selected(
    ops: &[PathOp<DevicePoint>], rx: Q6, ry: Q6, remaining: u64, limits: &Limits,
    xy: bool, projection: Option<&Projection<'_>>, mut timings: Option<&mut crate::timing::StageTimings>,
    allow_tiles: bool,
) -> Result<(Arc<Mask>, u64), RenderError> {
    let timer = timings.as_ref().and_then(|t| t.start());
    check_path_budget(ops.len(), limits)?;
    if let Some(t) = timings.as_mut() {
        let bytes = (ops.len() as u64)
            .saturating_mul(
                4 * (std::mem::size_of::<PathOp<FontPoint>>()
                    + std::mem::size_of::<PathOp<DevicePoint>>()) as u64,
            )
            .saturating_add(4096);
        t.raster_peak(bytes, ops.len() as u64);
    }
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
    // PathData::AlignLeftTop runs before xy flattens curves. Evaluating the
    // same polynomial in absolute viewport coordinates can truncate a sample
    // one Q6 tick differently after an otherwise integral translation.
    let (ox, oy) = if xy {
        hull.map_or((0, 0), |(x, y, _, _)| {
            (x.div_euclid(64) * 64, y.div_euclid(64) * 64)
        })
    } else {
        (0, 0)
    };
    if allow_tiles && xy && rx.ticks() == 0 && ry.ticks() == 0 && projection.is_none() {
        if let Some(hull) = hull {
            let tile_timer = timings.as_ref().and_then(|t| t.start());
            let result = try_tiled_body(ops, hull, (ox, oy), limits, &mut timings);
            if let Some(t) = timings.as_mut() {
                t.tiled_fill += crate::timing::StageTimings::elapsed(tile_timer);
                if matches!(&result, Ok(None)) { t.tile_fallbacks = t.tile_fallbacks.saturating_add(1); }
            }
            if let Some(result) = result? { return Ok(result); }
        }
    }
    let count = walk_outline(ops, (ox, oy), remaining, limits, &mut |a, b| {
        line(a, b, &mut edges, limits, timings.as_deref_mut())
    })?;
    for edge in &mut edges {
        edge.x += ox / 8;
        edge.y += oy / 8;
    }
    if let Some(t) = timings.as_mut() {
        t.edge_generation += crate::timing::StageTimings::elapsed(timer);
    }
    let sort_timer = timings.as_ref().and_then(|t| t.start());
    edges.sort_unstable_by_key(|e| (e.y, e.x, e.w));
    if let Some(t) = timings.as_mut() {
        t.edge_sorting += crate::timing::StageTimings::elapsed(sort_timer);
    }
    let edge_bytes = edges.capacity() as u64 * std::mem::size_of::<Edge>() as u64;
    if let Some(t) = timings.as_mut() {
        t.raster_peak(
            (edges.len() as u64).saturating_mul(3 * std::mem::size_of::<Edge>() as u64),
            count,
        );
    }
    let mut spans = Vec::new();
    let mut row = None;
    let mut winding = 0_i64;
    let mut left = 0;
    let generation_timer = timings.as_ref().and_then(|t| t.start());
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
                timings.as_deref_mut(),
            )?;
        }
    }
    if let Some(t) = timings.as_mut() {
        t.span_generation += crate::timing::StageTimings::elapsed(generation_timer);
        t.raster_peak(
            edge_bytes.saturating_add(
                (spans.len() as u64).saturating_mul(3 * std::mem::size_of::<Span>() as u64),
            ),
            count,
        );
    }
    drop(edges);
    if let Some(t) = timings.as_mut() {
        t.scan_conversion += crate::timing::StageTimings::elapsed(timer);
    }
    let timer = timings.as_ref().and_then(|t| t.start());
    let rx = (i64::from(rx.ticks()) + 4).div_euclid(8);
    let ry = (i64::from(ry.ticks()) + 4).div_euclid(8);
    let mut arc = Vec::new();
    let span_bytes = spans.capacity() as u64 * std::mem::size_of::<Span>() as u64;
    for y in 0..=ry {
        push(
            &mut arc,
            ellipse_width(rx, ry, y),
            span_bytes,
            limits,
            timings.as_deref_mut(),
        )?;
    }
    if let Some(t) = timings.as_mut() {
        t.raster_peak(
            span_bytes.saturating_add(
                (arc.len() as u64).saturating_mul(3 * std::mem::size_of::<i64>() as u64),
            ),
            count,
        );
    }
    let mut wide = Vec::new();
    let span_bytes = span_bytes
        .checked_add(arc.capacity() as u64 * std::mem::size_of::<i64>() as u64)
        .ok_or_else(limit)?;
    if let Some(t) = timings.as_mut() {
        t.span_rows = t.span_rows.saturating_add(spans.len() as u64);
        t.expanded_candidates_without_reuse = t
            .expanded_candidates_without_reuse
            .saturating_add((spans.len() as u64).saturating_mul((2 * ry + 1) as u64));
    }
    let bands = vertical_bands(
        &mut spans,
        &arc,
        span_bytes,
        arc.capacity() as u64 * std::mem::size_of::<i64>() as u64,
        limits,
        timings.as_deref_mut(),
    );
    let mut producer_bytes = span_bytes;
    if let Some((bands, build_peak)) = bands {
        if let Some(t) = timings.as_mut() {
            t.raster_peak(build_peak, count);
        }
        producer_bytes = bands.capacity() as u64 * std::mem::size_of::<VerticalBand>() as u64
            + arc.capacity() as u64 * std::mem::size_of::<i64>() as u64;
        drop(spans);
        for band in &bands {
            for y in band.first_y - ry..=band.last_y + ry {
                if wide.len() as u64 >= remaining {
                    return Err(limit());
                }
                let distance = if y < band.first_y {
                    band.first_y - y
                } else {
                    (y - band.last_y).max(0)
                };
                let width = arc[distance as usize];
                push(
                    &mut wide,
                    Span {
                        y,
                        left: band.left - width,
                        right: band.right + width,
                    },
                    producer_bytes,
                    limits,
                    timings.as_deref_mut(),
                )?;
            }
        }
    } else {
        for s in &spans {
            for y in s.y - ry..=s.y + ry {
                if wide.len() as u64 >= remaining {
                    return Err(limit());
                }
                push(
                    &mut wide,
                    Span {
                        y,
                        left: s.left - arc[(y - s.y).unsigned_abs() as usize],
                        right: s.right + arc[(y - s.y).unsigned_abs() as usize],
                    },
                    span_bytes,
                    limits,
                    timings.as_deref_mut(),
                )?;
            }
        }
        drop(spans);
    }
    if let Some(t) = timings.as_mut() {
        t.expanded_candidates = t.expanded_candidates.saturating_add(wide.len() as u64);
        t.raster_peak(
            producer_bytes.saturating_add(
                (wide.len() as u64).saturating_mul(3 * std::mem::size_of::<Span>() as u64),
            ),
            wide.len() as u64,
        );
    }
    drop(arc);
    if let Some(t) = timings.as_mut() {
        t.border_expansion += crate::timing::StageTimings::elapsed(timer);
    }
    let timer = timings.as_ref().and_then(|t| t.start());
    let sorting_before = timings
        .as_ref()
        .map_or(std::time::Duration::ZERO, |t| t.span_sorting);
    let bucket_timer = timings.as_ref().and_then(|t| t.start());
    let bucketed = sort_by_scan_row(&mut wide, producer_bytes, limits, timings.as_deref_mut());
    if let Some(t) = timings.as_mut() {
        t.row_bucketing += crate::timing::StageTimings::elapsed(bucket_timer)
            .saturating_sub(t.span_sorting.saturating_sub(sorting_before));
    }
    if !bucketed {
        let sort_timer = timings.as_ref().and_then(|t| t.start());
        wide.sort_unstable_by_key(|s| (s.y, s.left, s.right));
        if let Some(t) = timings.as_mut() {
            t.span_sorting += crate::timing::StageTimings::elapsed(sort_timer);
        }
    }
    let merge_timer = timings.as_ref().and_then(|t| t.start());
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
    if let Some(t) = timings.as_mut() {
        t.interval_merging += crate::timing::StageTimings::elapsed(merge_timer);
        t.span_union += crate::timing::StageTimings::elapsed(timer);
    }
    let timer = timings.as_ref().and_then(|t| t.start());
    if let Some(project) = projection {
        let segments = (wide.len() as u64).checked_mul(5).ok_or_else(limit)?;
        let work = count.checked_add(segments).ok_or_else(limit)?;
        if work > remaining {
            return Err(limit());
        }
        let live = wide.capacity() as u64 * std::mem::size_of::<Span>() as u64;
        let mut path = Vec::new();
        let point = |x: i64, y: i64| {
            let q = |n| i32::try_from(n).map(Q6::from_ticks).map_err(|_| limit());
            project(DevicePoint::new(q(x * 8 + ox)?, q(y * 8 + oy)?))
        };
        for span in &wide {
            for op in [
                PathOp::MoveTo(point(span.left, span.y)?),
                PathOp::LineTo(point(span.right, span.y)?),
                PathOp::LineTo(point(span.right, span.y + 1)?),
                PathOp::LineTo(point(span.left, span.y + 1)?),
                PathOp::Close,
            ] {
                push(&mut path, op, live, limits, timings.as_deref_mut())?;
            }
        }
        let charge = (path.capacity() as u64)
            .checked_mul(3 * std::mem::size_of::<PathOp<DevicePoint>>() as u64)
            .ok_or_else(limit)?;
        drop(wide);
        let available = limits
            .transient_bytes()
            .checked_sub(charge)
            .filter(|n| *n > 0)
            .ok_or_else(limit)?;
        let limits = limits
            .clone()
            .with_transient_bytes(available)
            .map_err(|_| limit())?;
        let (mask, rendered) = rasterize_impl(
            &path,
            Q6::from_ticks(0),
            Q6::from_ticks(0),
            remaining - work,
            &limits,
            true,
            None,
            timings,
        )?;
        return Ok((mask, work.checked_add(rendered).ok_or_else(limit)?));
    }
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
    if let Some(t) = timings.as_mut() {
        t.raster_peak(charge, count.saturating_add(wide.len() as u64));
    }
    let mut data = Vec::new();
    reserve_observed(&mut data, length, true, timings.as_deref_mut())
        .map_err(|_| RenderError::new(ErrorCode::AllocationFailed, "axis coverage allocation"))?;
    let clear_timer = timings.as_ref().and_then(|t| t.start());
    data.resize(length, 0_u8);
    if let Some(t) = timings.as_mut() {
        t.bitmap_clear += crate::timing::StageTimings::elapsed(clear_timer);
    }
    let coverage_timer = timings.as_ref().and_then(|t| t.start());
    for s in &wide {
        let y = (s.y.div_euclid(8) - y0) as usize;
        if let Some(t) = timings.as_mut().filter(|t| t.enabled) {
            t.coverage_write_samples = t
                .coverage_write_samples
                .saturating_add((-(-s.right).div_euclid(8) - s.left.div_euclid(8)) as u64);
        }
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
    if let Some(t) = timings.as_mut() {
        t.coverage_writes += crate::timing::StageTimings::elapsed(coverage_timer);
    }
    let work = count.checked_add(wide.len() as u64).ok_or_else(limit)?;
    if work > remaining {
        return Err(limit());
    }
    let publication_timer = timings.as_ref().and_then(|t| t.start());
    let mask = if xy {
        Mask::new_xy(bounds, stride, data.into())
    } else {
        Mask::new(bounds, stride, data.into())
    }?;
    if let Some(t) = timings.as_mut() {
        t.bitmap_publication += crate::timing::StageTimings::elapsed(publication_timer);
        t.mask_generation += crate::timing::StageTimings::elapsed(timer);
    }
    Ok((Arc::new(mask), work))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_capacity_witness_matches_actual_growth_for_all_retained_element_types() {
        fn prove<T: Copy>(value: T) {
            let mut actual = Vec::new();
            for length in 0..8193 {
                assert_eq!(
                    legacy_capacity::<T>(length),
                    Some((actual.capacity() * std::mem::size_of::<T>()) as u64)
                );
                actual.try_reserve(1).unwrap();
                actual.push(value);
            }
        }
        prove(Edge { y: 0, x: 0, w: 0 });
        prove(Span {
            y: 0,
            left: 0,
            right: 0,
        });
        prove(0_i64);
    }
    #[test]
    fn row_partition_matches_full_lexicographic_sort_and_declines_extra_budget() {
        let limits = Limits::default();
        for seed in 0..100_i64 {
            for rows in [1, 3, 17, 127] {
                let mut input = Vec::new();
                for n in 0..2048_i64 {
                    let left = (n * 119 + seed * 37) % 479 - 239;
                    input.push(Span {
                        y: (n * 17 + seed * 19 + n / 7) % rows - 80,
                        left,
                        right: left + (n * 13 + seed) % 37,
                    });
                }
                let mut expected = input.clone();
                expected.sort_unstable_by_key(|s| (s.y, s.left, s.right));
                assert!(sort_by_scan_row(&mut input, 0, &limits, None));
                assert_eq!(input, expected, "seed={seed}, rows={rows}");
                let before = input.clone();
                let tiny = limits.clone().with_transient_bytes(1).unwrap();
                assert!(!sort_by_scan_row(&mut input, 0, &tiny, None));
                assert_eq!(input, before);
            }
        }
        for extremes in [(i64::MIN, i64::MAX), (-1_000_000, 1_000_000)] {
            let mut input = vec![
                Span {
                    y: extremes.1,
                    left: 0,
                    right: 3,
                },
                Span {
                    y: extremes.0,
                    left: -4,
                    right: 0,
                },
            ];
            let before = input.clone();
            assert!(!sort_by_scan_row(&mut input, 0, &limits, None));
            assert_eq!(input, before);
        }
    }
    #[test]
    fn repeated_row_reuse_preserves_original_discrete_union() {
        use std::collections::BTreeSet;
        fn cells(spans: &[Span]) -> BTreeSet<(i64, i64)> {
            spans
                .iter()
                .flat_map(|s| (s.left..s.right).map(move |x| (s.y, x)))
                .collect()
        }
        let mut reused = 0;
        for seed in 0..8_i64 {
            for (rx, ry) in [(0, 0), (0, 4), (4, 0), (1, 1), (3, 4), (8, 4), (4, 8)] {
                let mut source = Vec::new();
                for y in -24_i64..32 {
                    for part in 0..3 {
                        let left = part * 12 + (y + seed).div_euclid(1 + seed % 5).rem_euclid(3);
                        source.push(Span {
                            y,
                            left,
                            right: left + 5 + y.div_euclid(10).rem_euclid(2),
                        });
                    }
                }
                let arc: Vec<_> = (0..=ry).map(|y| ellipse_width(rx, ry, y)).collect();
                // Original producer: one ellipse expansion for every input row.
                let mut reference = Vec::new();
                for s in &source {
                    for y in s.y - ry..=s.y + ry {
                        let w = arc[(y - s.y).unsigned_abs() as usize];
                        reference.push(Span {
                            y,
                            left: s.left - w,
                            right: s.right + w,
                        });
                    }
                }
                let source_bytes = source.capacity() as u64 * std::mem::size_of::<Span>() as u64
                    + arc.capacity() as u64 * 8;
                let bands = vertical_bands(
                    &mut source,
                    &arc,
                    source_bytes,
                    arc.capacity() as u64 * 8,
                    &Limits::default(),
                    None,
                );
                if let Some((bands, _)) = bands {
                    reused += 1;
                    let mut actual = Vec::new();
                    for b in bands {
                        for y in b.first_y - ry..=b.last_y + ry {
                            let distance = if y < b.first_y {
                                b.first_y - y
                            } else if y > b.last_y {
                                y - b.last_y
                            } else {
                                0
                            };
                            let w = arc[distance as usize];
                            actual.push(Span {
                                y,
                                left: b.left - w,
                                right: b.right + w,
                            });
                        }
                    }
                    assert!(actual.len() < reference.len());
                    assert_eq!(
                        cells(&reference),
                        cells(&actual),
                        "seed={seed}, radius={rx}/{ry}"
                    );
                }
            }
        }
        assert!(reused > 0);
    }
    #[test]
    fn optional_band_scratch_and_nonmonotone_kernel_bypass() {
        let mut spans = vec![
            Span {
                y: 0,
                left: 0,
                right: 3,
            },
            Span {
                y: 1,
                left: 0,
                right: 3,
            },
        ];
        assert!(
            vertical_bands(
                &mut spans,
                &[3, 2, 1],
                128,
                24,
                &Limits::default().with_transient_bytes(1).unwrap(),
                None,
            )
            .is_none()
        );
        assert!(
            vertical_bands(&mut spans, &[3, 1, 2], 128, 24, &Limits::default(), None).is_none()
        );
    }
    #[test]
    fn discrete_ellipse_retains_dominant_axis_boundary_cells() {
        assert_eq!(
            (0..=2).map(|y| ellipse_width(2, 2, y)).collect::<Vec<_>>(),
            [2, 2, 1]
        );
        assert_eq!(
            (0..=4).map(|y| ellipse_width(4, 4, y)).collect::<Vec<_>>(),
            [4, 4, 3, 3, 1]
        );
        for rx in 0..=32 {
            for ry in 0..=32 {
                let widths: Vec<_> = (0..=ry).map(|y| ellipse_width(rx, ry, y)).collect();
                assert!(widths.iter().all(|&x| (0..=rx).contains(&x)));
                assert!(widths.windows(2).all(|pair| pair[0] >= pair[1]));
            }
        }
    }
    #[test]
    fn xy_cubic_flattening_is_independent_of_integral_viewport_translation() {
        let limits = Limits::default();
        let p = |x, y| DevicePoint::new(Q6::from_ticks(x * 64), Q6::from_ticks(y * 64));
        let source = [
            PathOp::MoveTo(p(0, 0)),
            PathOp::CubicTo(p(32, -8), p(32, 24), p(0, 16)),
            PathOp::CubicTo(p(-8, 16), p(-8, 0), p(0, 0)),
            PathOp::Close,
        ];
        let original = rasterize_body(&source, &limits).unwrap();
        for (x, y) in [(150, 35), (-100, -200), (1700, 900)] {
            let moved: Vec<_> = source
                .iter()
                .map(|op| {
                    op.try_map(|point| {
                        Ok(DevicePoint::new(
                            Q6::from_ticks(point.x().ticks() + x * 64),
                            Q6::from_ticks(point.y().ticks() + y * 64),
                        ))
                    })
                    .unwrap()
                })
                .collect();
            let output = rasterize_body(&moved, &limits).unwrap();
            assert_eq!(output.data(), original.data());
            assert_eq!(output.bounds().x0(), original.bounds().x0() + x);
            assert_eq!(output.bounds().y0(), original.bounds().y0() + y);
            assert_eq!(output.bounds().width(), original.bounds().width());
            assert_eq!(output.bounds().height(), original.bounds().height());
        }
    }
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
    fn closed_quadratic_contours_have_balanced_scanline_winding() {
        // A closed contour cannot leave any nonzero winding at infinity. Sweep
        // fractional cubic conversions at eighth-grid endpoints, on both sides
        // of zero: no image baseline or renderer implementation is the oracle.
        for dy in -16..=16 {
            for bend in 1..=96 {
                let a = (0.0, f64::from(dy));
                let b = (64.0, f64::from(dy + 4));
                let control = (f64::from(bend), f64::from(dy + bend));
                let p1 = (
                    a.0 + (control.0 - a.0) * 2.0 / 3.0,
                    a.1 + (control.1 - a.1) * 2.0 / 3.0,
                );
                let p2 = (
                    b.0 + (control.0 - b.0) * 2.0 / 3.0,
                    b.1 + (control.1 - b.1) * 2.0 / 3.0,
                );
                let mut edges = Vec::new();
                curve([a, p1, p2, b], &mut edges, &Limits::default(), &mut 0, None).unwrap();
                line(b, a, &mut edges, &Limits::default(), None).unwrap();
                let mut winding = std::collections::BTreeMap::new();
                for edge in edges {
                    *winding.entry(edge.y).or_insert(0) += edge.w;
                }
                assert!(
                    winding.values().all(|&value| value == 0),
                    "dy={dy} bend={bend}: {winding:?}"
                );
            }
        }
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
