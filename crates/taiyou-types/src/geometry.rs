//! C5 owned nonzero paths. Bounds are conservative control-point bounds, not ink.
use crate::*;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FontPoint {
    x: FiniteF64,
    y: FiniteF64,
}
impl FontPoint {
    pub const ZERO: Self = Self {
        x: FiniteF64::ZERO,
        y: FiniteF64::ZERO,
    };
    pub fn new(x: f64, y: f64) -> Result<Self, RenderError> {
        Ok(Self {
            x: FiniteF64::new(x).map_err(|_| geometry_limit())?,
            y: FiniteF64::new(y).map_err(|_| geometry_limit())?,
        })
    }
    pub const fn x(self) -> FiniteF64 {
        self.x
    }
    pub const fn y(self) -> FiniteF64 {
        self.y
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FontBounds {
    min: FontPoint,
    max: FontPoint,
}
impl FontBounds {
    pub const fn min(self) -> FontPoint {
        self.min
    }
    pub const fn max(self) -> FontPoint {
        self.max
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FillRule {
    Nonzero,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathOp<P> {
    MoveTo(P),
    LineTo(P),
    QuadTo(P, P),
    CubicTo(P, P, P),
    Close,
}
impl<P: Copy> PathOp<P> {
    pub fn try_map<Q>(
        self,
        mut f: impl FnMut(P) -> Result<Q, RenderError>,
    ) -> Result<PathOp<Q>, RenderError> {
        Ok(match self {
            Self::MoveTo(p) => PathOp::MoveTo(f(p)?),
            Self::LineTo(p) => PathOp::LineTo(f(p)?),
            Self::QuadTo(a, b) => PathOp::QuadTo(f(a)?, f(b)?),
            Self::CubicTo(a, b, c) => PathOp::CubicTo(f(a)?, f(b)?, f(c)?),
            Self::Close => PathOp::Close,
        })
    }
}
pub fn geometry_limit() -> RenderError {
    RenderError::new(
        ErrorCode::LimitExceeded,
        "geometry coordinate/count/reservation limit",
    )
}
/// Immutable y-up design-unit outline; no placement, paint or native pointers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Outline {
    ops: Arc<[PathOp<FontPoint>]>,
    bounds: Option<FontBounds>,
}
impl Outline {
    pub fn new(ops: Vec<PathOp<FontPoint>>, limits: &Limits) -> Result<Self, RenderError> {
        check_path_budget(ops.len(), limits)?;
        let mut open = false;
        let mut bounds: Option<FontBounds> = None;
        for &op in &ops {
            match op {
                PathOp::MoveTo(_) if !open => open = true,
                PathOp::Close if open => open = false,
                PathOp::LineTo(_) | PathOp::QuadTo(..) | PathOp::CubicTo(..) if open => {}
                _ => {
                    return Err(RenderError::new(
                        ErrorCode::InvalidArgument,
                        "unclosed or malformed contour",
                    ));
                }
            }
            op.try_map(|p| {
                bounds = Some(match bounds {
                    None => FontBounds { min: p, max: p },
                    Some(b) => FontBounds {
                        min: FontPoint::new(
                            p.x.get().min(b.min.x.get()),
                            p.y.get().min(b.min.y.get()),
                        )?,
                        max: FontPoint::new(
                            p.x.get().max(b.max.x.get()),
                            p.y.get().max(b.max.y.get()),
                        )?,
                    },
                });
                Ok(p)
            })?;
        }
        if open {
            return Err(RenderError::new(ErrorCode::InvalidArgument, "open contour"));
        }
        if let Some(b) = bounds {
            if !(b.max.x.get() - b.min.x.get()).is_finite()
                || !(b.max.y.get() - b.min.y.get()).is_finite()
            {
                return Err(geometry_limit());
            }
        }
        Ok(Self {
            ops: ops.into(),
            bounds,
        })
    }
    pub fn ops(&self) -> &[PathOp<FontPoint>] {
        &self.ops
    }
    pub const fn bounds(&self) -> Option<FontBounds> {
        self.bounds
    }
    pub const fn fill_rule(&self) -> FillRule {
        FillRule::Nonzero
    }
}
/// Includes both owned design and device commands plus temporary publication copies.
pub fn check_path_budget(count: usize, limits: &Limits) -> Result<(), RenderError> {
    let cost = (count as u64)
        .checked_mul(
            4 * (std::mem::size_of::<PathOp<FontPoint>>()
                + std::mem::size_of::<PathOp<DevicePoint>>()) as u64,
        )
        .and_then(|n| n.checked_add(4096))
        .ok_or_else(geometry_limit)?;
    if count as u64 > limits.path_segments_per_frame() {
        return Err(geometry_limit());
    }
    if cost > limits.transient_bytes() {
        return Err(geometry_limit().with_resource_failure(crate::ResourceFailure::TransientMemory));
    }
    Ok(())
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GlyphOutline {
    font: FontId,
    glyph: u32,
    upem: u16,
    outline: Arc<Outline>,
}
impl GlyphOutline {
    pub fn new(
        font: FontId,
        glyph: u32,
        upem: u16,
        outline: Arc<Outline>,
    ) -> Result<Self, FontError> {
        if upem == 0 {
            return Err(FontError::new(
                ErrorCode::InvalidArgument,
                "zero outline units per em",
            ));
        }
        Ok(Self {
            font,
            glyph,
            upem,
            outline,
        })
    }
    pub const fn font(&self) -> FontId {
        self.font
    }
    pub const fn glyph(&self) -> u32 {
        self.glyph
    }
    pub const fn units_per_em(&self) -> u16 {
        self.upem
    }
    pub fn outline(&self) -> &Arc<Outline> {
        &self.outline
    }
    /// FT_NO_SCALE | NO_HINTING | NO_BITMAP | IGNORE_TRANSFORM; adapter revision 1.
    pub const fn extraction_revision(&self) -> u32 {
        1
    }
}
