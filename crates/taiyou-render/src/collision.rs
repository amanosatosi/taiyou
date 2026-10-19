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

fn limited() -> RenderError {
    RenderError::new(ErrorCode::LimitExceeded, "collision arithmetic/reservation exceeded")
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
    if profile != Profile::CoreV1 {
        return Err(RenderError::new(
            ErrorCode::UnsupportedFeature,
            "Legacy collision policy requires authenticated evidence and approval",
        ));
    }
    let reservation = (events.len() as u64)
        .checked_mul(3 * (std::mem::size_of::<usize>() + std::mem::size_of::<PlacedEvent>()) as u64)
        .ok_or_else(limited)?;
    if events.len() as u64 > limits.events() || reservation > limits.transient_bytes() {
        return Err(limited());
    }
    let mut order = Vec::new();
    let mut result: Vec<PlacedEvent> = Vec::new();
    order.try_reserve_exact(events.len()).map_err(|_| {
        RenderError::new(ErrorCode::AllocationFailed, "collision order allocation failed")
    })?;
    result.try_reserve_exact(events.len()).map_err(|_| {
        RenderError::new(ErrorCode::AllocationFailed, "collision placement allocation failed")
    })?;
    order.extend(0..events.len());
    // Validate identity independently of start/layer, before placement.
    order.sort_unstable_by_key(|&i| events[i].id);
    if order.windows(2).any(|p| events[p[0]].id == events[p[1]].id) {
        return Err(RenderError::new(ErrorCode::InvalidArgument, "duplicate collision EventId"));
    }
    order.sort_unstable_by_key(|&i| (events[i].layer, events[i].start, events[i].id));
    for &index in &order {
        let input = &events[index];
        let natural = translated(input.bounds, input.anchor)?;
        let mut y = input.anchor.y().get();
        if input.eligible && occupied(natural) {
            let up = input.alignment.get() <= 3;
            let mut cleared = false;
            // Each jump leaves one fixed forbidden interval permanently, in
            // the chosen direction. At most N prior obstacles can be left;
            // the final scan proves clearance. This is a derived bound, not
            // an arbitrary iteration/displacement cap.
            for _ in 0..=result.len() {
                let mut next = None;
                for (&prior_index, prior) in order.iter().zip(&result) {
                    let obstacle = &events[prior_index];
                    if obstacle.layer != input.layer || !obstacle.eligible || !occupied(prior.bounds)
                        || natural.x0() >= prior.bounds.x1() || natural.x1() <= prior.bounds.x0()
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
                    if y > low && y < high {
                        next = Some(if up { low } else { high });
                        break;
                    }
                }
                match next {
                    Some(value) => y = value,
                    None => {
                        cleared = true;
                        break;
                    }
                }
            }
            if !cleared {
                return Err(RenderError::new(ErrorCode::InternalError, "collision search failed its derived bound"));
            }
        }
        let anchor = ScriptPoint::new(input.anchor.x(), finite(y)?);
        result.push(PlacedEvent {
            id: input.id,
            anchor,
            bounds: translated(input.bounds, anchor)?,
        });
    }
    result.sort_unstable_by_key(|event| event.id);
    Ok(result)
}
