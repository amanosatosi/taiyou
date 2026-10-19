//! Stateless C4 Core placement in script space. Local boxes remain immutable.
use taiyou_types::*;

/// One active event's existing local collision box and natural anchor.
#[derive(Clone, Copy, Debug)]
pub struct CollisionInput {
    pub id: EventId,
    pub layer: i32,
    pub start: TimeUs,
    pub alignment: Alignment,
    pub anchor: ScriptPoint,
    pub bounds: ScriptBounds,
    pub eligible: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlacedEvent {
    pub id: EventId,
    pub anchor: ScriptPoint,
    pub bounds: ScriptBounds,
}

/// Deterministic work counters and conservative Taiyou-owned workspace charge.
#[derive(Clone, Copy, Debug, Default)]
pub struct CollisionStats {
    pub eligible: u64,
    pub obstacle_tests: u64,
    pub intervals: u64,
    pub sort_comparisons: u64,
    pub interval_visits: u64,
    pub reserved_bytes: u64,
    pub allocated_vector_bytes: u64,
}

fn limited() -> RenderError {
    RenderError::new(
        ErrorCode::LimitExceeded,
        "collision arithmetic/reservation exceeded",
    )
}
fn finite(value: f64) -> Result<FiniteF64, RenderError> {
    FiniteF64::new(value).map_err(|_| limited())
}
fn occupied(bounds: ScriptBounds) -> bool {
    bounds.width() > 0.0 && bounds.height() > 0.0
}
fn translated(bounds: ScriptBounds, anchor: ScriptPoint) -> Result<ScriptBounds, RenderError> {
    let output = bounds.translated(anchor.x().get(), anchor.y().get())?;
    if (bounds.width() > 0.0 && output.width() == 0.0)
        || (bounds.height() > 0.0 && output.height() == 0.0)
    {
        return Err(limited());
    }
    Ok(output)
}

/// Returns EventId-sorted placements; paint order is independent. The only
/// admitted policy is CoreV1. No viewport clamp, border inflation or history.
pub fn resolve_collisions(
    events: &[CollisionInput],
    profile: Profile,
    limits: &Limits,
) -> Result<Vec<PlacedEvent>, RenderError> {
    resolve_collisions_with_stats(events, profile, limits).map(|(placed, _)| placed)
}

pub fn resolve_collisions_with_stats(
    events: &[CollisionInput],
    profile: Profile,
    limits: &Limits,
) -> Result<(Vec<PlacedEvent>, CollisionStats), RenderError> {
    if profile != Profile::CoreV1 {
        return Err(RenderError::new(
            ErrorCode::UnsupportedFeature,
            "Legacy collision policy requires authenticated evidence and approval",
        ));
    }
    let count = events.iter().filter(|e| e.eligible).count();
    let reservation = (events.len() as u64)
        .checked_mul(3 * std::mem::size_of::<PlacedEvent>() as u64)
        .and_then(|n| {
            n.checked_add((count as u64).checked_mul(
                3 * (std::mem::size_of::<usize>() + std::mem::size_of::<(f64, f64)>()) as u64,
            )?)
        })
        .ok_or_else(limited)?;
    if events.len() as u64 > limits.events() || reservation > limits.transient_bytes() {
        return Err(limited());
    }
    let mut order = Vec::new();
    let mut result: Vec<PlacedEvent> = Vec::new();
    let mut intervals: Vec<(f64, f64)> = Vec::new();
    let mut stats = CollisionStats {
        eligible: count as u64,
        reserved_bytes: reservation,
        ..Default::default()
    };
    intervals.try_reserve_exact(count).map_err(|_| {
        RenderError::new(
            ErrorCode::AllocationFailed,
            "collision interval allocation failed",
        )
    })?;
    order.try_reserve_exact(count).map_err(|_| {
        RenderError::new(
            ErrorCode::AllocationFailed,
            "collision order allocation failed",
        )
    })?;
    result.try_reserve_exact(events.len()).map_err(|_| {
        RenderError::new(
            ErrorCode::AllocationFailed,
            "collision placement allocation failed",
        )
    })?;
    let order_bytes = (order.capacity() as u64)
        .checked_mul(std::mem::size_of::<usize>() as u64)
        .ok_or_else(limited)?;
    let interval_bytes = (intervals.capacity() as u64)
        .checked_mul(std::mem::size_of::<(f64, f64)>() as u64)
        .ok_or_else(limited)?;
    let result_bytes = (result.capacity() as u64)
        .checked_mul(std::mem::size_of::<PlacedEvent>() as u64)
        .ok_or_else(limited)?;
    stats.allocated_vector_bytes = order_bytes
        .checked_add(interval_bytes)
        .and_then(|n| n.checked_add(result_bytes))
        .ok_or_else(limited)?;
    // Ineligible inputs only receive natural output records. They never enter
    // placement ordering, obstacle collection or interval processing. The
    // renderer structurally excludes them before calling this compatibility API.
    for (i, input) in events.iter().enumerate() {
        if input.eligible {
            order.push(i);
        }
    }
    order.sort_unstable_by_key(|&i| (events[i].layer, events[i].start, events[i].id));
    let mut layer_start = 0;
    for (ordinal, &index) in order.iter().enumerate() {
        let input = &events[index];
        if ordinal > 0 && events[order[ordinal - 1]].layer != input.layer {
            layer_start = ordinal;
        }
        let natural = translated(input.bounds, input.anchor)?;
        let mut y = input.anchor.y().get();
        if occupied(natural) {
            let up = input.alignment.get() <= 3;
            intervals.clear();
            for prior in &result[layer_start..] {
                stats.obstacle_tests += 1;
                if !occupied(prior.bounds)
                    || natural.x0() >= prior.bounds.x1()
                    || natural.x1() <= prior.bounds.x0()
                {
                    continue;
                }
                // Open forbidden interval of anchor Y values. Testing in
                // anchor coordinates makes exact edge contact noncolliding
                // without an epsilon, pixel rounding or accumulated deltas.
                let low = finite(prior.bounds.y0().get() - input.bounds.y1().get())?.get();
                let high = finite(prior.bounds.y1().get() - input.bounds.y0().get())?.get();
                if low >= high {
                    return Err(limited());
                }
                intervals.push((low, high));
                stats.intervals += 1;
            }
            intervals.sort_unstable_by(|a, b| {
                stats.sort_comparisons += 1;
                a.0.total_cmp(&b.0).then_with(|| a.1.total_cmp(&b.1))
            });
            // Union OPEN intervals. Equal endpoints must remain separate: the
            // touching point is legal. A connected component supplies the
            // nearest legal anchor directly, with one scan and no restarts.
            let mut at = 0;
            while at < intervals.len() {
                let (low, mut high) = intervals[at];
                at += 1;
                stats.interval_visits += 1;
                while at < intervals.len() && intervals[at].0 < high {
                    high = high.max(intervals[at].1);
                    at += 1;
                    stats.interval_visits += 1;
                }
                if y > low && y < high {
                    y = if up { low } else { high };
                    break;
                }
            }
        }
        let anchor = ScriptPoint::new(input.anchor.x(), finite(y)?);
        result.push(PlacedEvent {
            id: input.id,
            anchor,
            bounds: translated(input.bounds, anchor)?,
        });
    }
    for input in events.iter().filter(|input| !input.eligible) {
        result.push(PlacedEvent {
            id: input.id,
            anchor: input.anchor,
            bounds: translated(input.bounds, input.anchor)?,
        });
    }
    // Identity publication is distinct from collision order, including for the
    // older public API accepting ineligible inputs. Renderer inputs are eligible only.
    result.sort_unstable_by_key(|event| event.id);
    if result.windows(2).any(|p| p[0].id == p[1].id) {
        return Err(RenderError::new(
            ErrorCode::InvalidArgument,
            "duplicate collision EventId",
        ));
    }
    Ok((result, stats))
}
