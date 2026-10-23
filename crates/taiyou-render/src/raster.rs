//! CoreV1 fill groups and pinned FreeType grayscale coverage. No paint execution.
use crate::geometry::{GeometryEvent, GeometryInstance};
use std::{ops::Range, sync::Arc};
#[cfg(feature = "native")]
pub use taiyou_font::RasterContext;
pub use taiyou_types::Mask;
use taiyou_types::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoveragePass {
    Shadow,
    Border(u16),
    Fill,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoveragePolicy {
    profile: Profile,
    revision: u32,
    members: Range<usize>,
}
impl CoveragePolicy {
    pub const fn profile(&self) -> Profile {
        self.profile
    }
    pub const fn revision(&self) -> u32 {
        self.revision
    }
    /// Ordinals scoped to this group's retained event or explicit instance list.
    /// Policy alone is not a cache key; full ordered geometry is also required.
    pub fn members(&self) -> Range<usize> {
        self.members.clone()
    }
}
/// Instances are in layout visual order. The retained event owns source/paint
/// associations; neither paint values nor RGBA pixels enter the resulting mask.
#[derive(Clone, Debug)]
pub struct CoverageGroup {
    instances: Arc<[GeometryInstance]>,
    pass: CoveragePass,
    policy: CoveragePolicy,
    event: Option<Arc<GeometryEvent>>,
}
impl CoverageGroup {
    /// Explicit host/synthetic compatible group. Production uses coverage_groups
    /// to check compatibility against the retained evaluated identities.
    pub fn new(
        instances: Vec<GeometryInstance>,
        pass: CoveragePass,
        profile: Profile,
        limits: &Limits,
    ) -> Result<Self, RenderError> {
        if profile != Profile::CoreV1 || pass != CoveragePass::Fill {
            return Err(RenderError::new(
                ErrorCode::UnsupportedFeature,
                "only CoreV1 nonzero fill rasterization is implemented",
            ));
        }
        let n = instances.len();
        group_budget(n, limits)?;
        Ok(Self {
            instances: instances.into(),
            pass,
            policy: CoveragePolicy {
                profile,
                revision: 1,
                members: 0..n,
            },
            event: None,
        })
    }
    pub fn instances(&self) -> &[GeometryInstance] {
        &self.instances
    }
    pub const fn pass(&self) -> CoveragePass {
        self.pass
    }
    pub fn policy(&self) -> &CoveragePolicy {
        &self.policy
    }
    pub fn event(&self) -> Option<&GeometryEvent> {
        self.event.as_deref()
    }
}
fn limited() -> RenderError {
    RenderError::new(
        ErrorCode::LimitExceeded,
        "raster grouping/coordinate/allocation limit",
    )
}
fn group_budget(n: usize, limits: &Limits) -> Result<(), RenderError> {
    let bytes = (n as u64)
        .checked_mul(
            3 * (std::mem::size_of::<GeometryInstance>() + std::mem::size_of::<CoverageGroup>())
                as u64,
        )
        .ok_or_else(limited)?;
    if n as u64 > limits.glyphs_per_event() {
        return Err(limited());
    }
    if bytes > limits.transient_bytes() {
        return Err(limited().with_resource_failure(ResourceFailure::TransientMemory));
    }
    Ok(())
}
pub fn coverage_groups(
    event: Arc<GeometryEvent>,
    limits: &Limits,
) -> Result<Vec<CoverageGroup>, RenderError> {
    group_budget(event.items().len(), limits)?;
    let mut groups = Vec::new();
    groups.try_reserve_exact(event.items().len()).map_err(|_| {
        RenderError::new(
            ErrorCode::AllocationFailed,
            "coverage groups allocation failed",
        )
    })?;
    let state =
        |i: usize| &event.layout().shaped().evaluated().runs()[event.items()[i].association];
    let mut start = 0;
    let mut total = 0usize;
    let mut line_index = 0;
    while start < event.items().len() {
        while event
            .layout()
            .lines()
            .get(line_index)
            .is_some_and(|l| l.glyph_range().end <= start)
        {
            line_index += 1;
        }
        let opaque = state(start).effects().border_style() == BorderStyle::OpaqueBox;
        let line_end = if opaque {
            event
                .layout()
                .lines()
                .get(line_index)
                .map_or(event.items().len(), |l| l.glyph_range().end)
        } else {
            event.items().len()
        };
        let mut end = start + 1;
        while end < line_end {
            let (a, b) = (state(start), state(end));
            // C3 classifies shape changes as geometry changes as well. Fallback
            // faces under the same request stay compatible; per-glyph position
            // and source ranges are membership, not style-identity boundaries.
            if a.shaping() != b.shaping()
                || a.geometry() != b.geometry()
                || a.effects() != b.effects()
                || a.paint() != b.paint()
                || a.karaoke() != b.karaoke()
                // Drawing words are independent overlays, even with identical
                // style. Joining their contours erases fractional-pixel seams
                // and changes translucent overlaps. pinterf CPolygon::Append
                // refuses every append; libass also breaks bitmap runs here.
                || a.drawing().is_some()
                || b.drawing().is_some()
            {
                break;
            }
            end += 1;
        }
        let mut instances = Vec::new();
        instances.try_reserve_exact(end - start).map_err(|_| {
            RenderError::new(
                ErrorCode::AllocationFailed,
                "coverage membership allocation failed",
            )
        })?;
        for glyph in &event.items()[start..end] {
            total = total
                .checked_add(glyph.geometry.ops().len())
                .ok_or_else(limited)?;
            check_path_budget(total, limits)?;
            instances.push(glyph.geometry.clone());
        }
        groups.push(CoverageGroup {
            instances: instances.into(),
            pass: CoveragePass::Fill,
            policy: CoveragePolicy {
                profile: Profile::CoreV1,
                revision: 1,
                members: start..end,
            },
            event: Some(event.clone()),
        });
        start = end;
    }
    Ok(groups)
}
/// Conservative full source support, independent of any output viewport.
pub fn coverage_bounds(group: &CoverageGroup) -> Result<PixelRect, RenderError> {
    let mut bounds: Option<(i32, i32, i32, i32)> = None;
    for i in group.instances() {
        if let Some(b) = i.bounds() {
            let (x0, y0, x1, y1) = (
                b.min().x().ticks(),
                b.min().y().ticks(),
                b.max().x().ticks(),
                b.max().y().ticks(),
            );
            bounds = Some(bounds.map_or((x0, y0, x1, y1), |a| {
                (a.0.min(x0), a.1.min(y0), a.2.max(x1), a.3.max(y1))
            }));
        }
    }
    let Some((x0, y0, x1, y1)) = bounds else {
        return empty_bounds();
    };
    if x0 == x1 || y0 == y1 {
        return empty_bounds();
    }
    let floor = |n: i32| n.div_euclid(64);
    let ceil = |n: i32| i32::try_from(-(-i64::from(n)).div_euclid(64)).map_err(|_| limited());
    PixelRect::new(floor(x0), floor(y0), ceil(x1)?, ceil(y1)?).map_err(|_| limited())
}
fn empty_bounds() -> Result<PixelRect, RenderError> {
    PixelRect::new(0, 0, 0, 0).map_err(|_| limited())
}
#[cfg(feature = "native")]
pub(crate) fn coverage_path(
    group: &CoverageGroup,
    translate_x: Q6,
    translate_y: Q6,
    limits: &Limits,
) -> Result<Vec<PathOp<DevicePoint>>, RenderError> {
    let count = group.instances().iter().try_fold(0usize, |n, i| {
        n.checked_add(i.ops().len()).ok_or_else(limited)
    })?;
    check_path_budget(count, limits)?;
    let bytes = u64::try_from(count)
        .ok()
        .and_then(|n| n.checked_mul(3 * std::mem::size_of::<PathOp<DevicePoint>>() as u64))
        .ok_or_else(limited)?;
    if bytes > limits.transient_bytes() {
        return Err(limited().with_resource_failure(ResourceFailure::TransientMemory));
    }
    let mut ops = Vec::new();
    ops.try_reserve_exact(count).map_err(|_| {
        RenderError::new(
            ErrorCode::AllocationFailed,
            "combined contour allocation failed",
        )
    })?;
    for instance in group.instances() {
        for &op in instance.ops() {
            ops.push(op.try_map(|p| {
                let add = |q: Q6, offset: i32, translate: Q6| {
                    i32::try_from(
                        i64::from(q.ticks())
                            + i64::from(offset) * 64
                            + i64::from(translate.ticks()),
                    )
                    .map(Q6::from_ticks)
                    .map_err(|_| limited())
                };
                Ok(DevicePoint::new(
                    add(p.x(), instance.offset().x(), translate_x)?,
                    add(p.y(), instance.offset().y(), translate_y)?,
                ))
            })?);
        }
    }
    Ok(ops)
}
#[cfg(feature = "native")]
pub fn rasterize(ctx: &mut RasterContext, group: &CoverageGroup) -> Result<Arc<Mask>, RenderError> {
    if group.pass != CoveragePass::Fill || group.policy.profile != Profile::CoreV1 {
        return Err(RenderError::new(
            ErrorCode::UnsupportedFeature,
            "unsupported raster policy/pass",
        ));
    }
    let bounds = coverage_bounds(group)?;
    if bounds.is_empty() {
        return Ok(Arc::new(Mask::new(bounds, 0, Arc::from([]))?));
    }
    let ops = coverage_path(group, Q6::from_ticks(0), Q6::from_ticks(0), ctx.limits())?;
    ctx.rasterize_path(
        &ops,
        bounds,
        usize::try_from(bounds.width()).map_err(|_| limited())?,
    )
}
/// Source-mask comparison entry point using exactly the same supplied path.
/// FreeType rasterize above remains available as an independent native control.
#[cfg(feature = "native")]
pub fn rasterize_xy(
    ctx: &mut RasterContext,
    group: &CoverageGroup,
) -> Result<Arc<Mask>, RenderError> {
    let ops = coverage_path(group, Q6::from_ticks(0), Q6::from_ticks(0), ctx.limits())?;
    let bytes = (ops.len() as u64)
        .checked_mul(3 * std::mem::size_of::<PathOp<DevicePoint>>() as u64)
        .ok_or_else(limited)?;
    let available = ctx
        .limits()
        .transient_bytes()
        .checked_sub(bytes)
        .filter(|&n| n > 0)
        .ok_or_else(|| limited().with_resource_failure(ResourceFailure::TransientMemory))?;
    let limits = ctx
        .limits()
        .clone()
        .with_transient_bytes(available)
        .map_err(|_| limited())?;
    crate::axis_widen::rasterize_body(&ops, &limits)
}
