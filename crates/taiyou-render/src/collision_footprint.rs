//! Owner-approved ADR-012 geometric support. This is not raster pixel support.
use crate::geometry::DeviceMapping;
use taiyou_types::*;

fn finite(value: f64) -> Result<FiniteF64, RenderError> {
    FiniteF64::new(value).map_err(|_| {
        RenderError::new(
            ErrorCode::LimitExceeded,
            "collision footprint arithmetic exceeded",
        )
    })
}
fn q6(value: f64) -> Result<Q6, RenderError> {
    Q6::from_pixels(finite(value)?).map_err(|e| RenderError::new(e.code(), e.detail()))
}
fn include(result: &mut Option<ScriptBounds>, value: ScriptBounds) -> Result<(), RenderError> {
    *result = Some(match *result {
        Some(bounds) => bounds.union(value)?,
        None => value,
    });
    Ok(())
}

/// Device-space hull of a compatible group. A visible shadow uses the expanded
/// source even when outline paint is transparent. None means no visible support.
pub fn effect_bounds(
    ink: ScriptBounds,
    border: Option<Q6>,
    shadow: Option<(Q6, Q6)>,
    fill_visible: bool,
    border_visible: bool,
) -> Result<Option<ScriptBounds>, RenderError> {
    effect_bounds_axes(
        ink,
        border.map(|r| (r, r)),
        shadow,
        fill_visible,
        border_visible,
    )
}
pub fn effect_bounds_axes(
    ink: ScriptBounds,
    border: Option<(Q6, Q6)>,
    shadow: Option<(Q6, Q6)>,
    fill_visible: bool,
    border_visible: bool,
) -> Result<Option<ScriptBounds>, RenderError> {
    if ink.width() == 0.0 || ink.height() == 0.0 {
        return Ok(None);
    }
    let (rx, ry) = border.map_or((0.0, 0.0), |(x, y)| {
        (f64::from(x.ticks()) / 64.0, f64::from(y.ticks()) / 64.0)
    });
    if rx < 0.0 || ry < 0.0 {
        return Err(RenderError::new(
            ErrorCode::InvalidArgument,
            "negative collision border radius",
        ));
    }
    let expanded = ScriptBounds::new(
        ink.x0().get() - rx,
        ink.y0().get() - ry,
        ink.x1().get() + rx,
        ink.y1().get() + ry,
    )?;
    let mut result = None;
    if fill_visible {
        include(&mut result, ink)?;
    }
    if border.is_some() && border_visible {
        include(&mut result, expanded)?;
    }
    if let Some((x, y)) = shadow {
        include(
            &mut result,
            expanded.translated(f64::from(x.ticks()) / 64.0, f64::from(y.ticks()) / 64.0)?,
        )?;
    }
    Ok(result)
}

/// Separate placement artifact: never mutate the paint-independent LocalLayout.
pub fn collision_footprint(
    local: &LocalLayout,
    device: DeviceMapping,
    scaled_border_shadow: bool,
    _policy: UnsupportedPolicy,
) -> Result<ScriptBounds, RenderError> {
    if local.shaped().placement().origin().is_some() {
        return Err(RenderError::new(
            ErrorCode::UnsupportedFeature,
            "fixed explicit rotation origin requires a separate collision policy",
        ));
    }
    let sx = f64::from(device.content().width()) / f64::from(device.play_res().width());
    let sy = f64::from(device.content().height()) / f64::from(device.play_res().height());
    let [lx,ly]=device.layout_scale();
    let mut result = local.collision_bounds();
    for positioned in local.glyphs() {
        let run = &local.shaped().runs()[positioned.run_index()];
        let glyph_ink = if let Some(drawing) = run.drawing() {
            match drawing.outline.bounds() {
                Some(b) => GlyphInk::Bounds(ScriptBounds::new(
                    b.min().x().get(),
                    -b.max().y().get(),
                    b.max().x().get(),
                    -b.min().y().get(),
                )?),
                None => GlyphInk::Empty,
            }
        } else {
            run.glyphs()[positioned.glyph_index()].ink()
        };
        let ink = match glyph_ink {
            GlyphInk::Bounds(bounds) => bounds,
            GlyphInk::Empty => continue,
            GlyphInk::Unknown => {
                return Err(RenderError::new(
                    ErrorCode::UnsupportedFeature,
                    "collision glyph ink unavailable",
                ));
            }
        };
        let state = &local.shaped().evaluated().runs()[positioned.association_index()];
        let additional = local.local_scale();
        let ink = ink
            .scaled(
                state.geometry().scale_x().get() * additional.0.get(),
                state.geometry().scale_y().get() * additional.1.get(),
            )?
            .translated(
                positioned.position().x().get(),
                positioned.position().y().get(),
            )?;
        // Match GeometryMap before its separate Q6 point quantization. Angle
        // reduction and exact cardinals matter for equivalent large rotations.
        let angle = state.geometry().rotation_z().get().rem_euclid(360.0);
        let (sin, cos) = match angle {
            0.0 => (0.0, 1.0),
            90.0 => (1.0, 0.0),
            180.0 => (0.0, -1.0),
            270.0 => (-1.0, 0.0),
            _ => angle.to_radians().sin_cos(),
        };
        let mut hull = None;
        for x in [ink.x0().get(), ink.x1().get()] {
            for y in [ink.y0().get(), ink.y1().get()] {
                let (x, y) = (
                    x + state.geometry().shear_x().get() * y,
                    y + state.geometry().shear_y().get() * x,
                );
                let (x, y) = crate::geometry::project_3d(
                    cos * x*lx + sin * y*ly,
                    -sin * x*lx + cos * y*ly,
                    state.geometry().rotation_x().get(),
                    state.geometry().rotation_y().get(),
                )?;
                let dx = finite(x * sx/lx)?.get();
                let dy = finite(y * sy/ly)?.get();
                include(&mut hull, ScriptBounds::new(dx, dy, dx, dy)?)?;
            }
        }
        let border = state.effects().border_style() == BorderStyle::Outline
            && (state.effects().border_x().get() > 0.0 || state.effects().border_y().get() > 0.0);
        let radius = if border {
            Some((
                q6(state.effects().border_x().get() * if scaled_border_shadow { sx } else { sx/lx })?,
                q6(state.effects().border_y().get() * if scaled_border_shadow { sy } else { sy/ly })?,
            ))
        } else {
            None
        };
        let shadow = if state.paint().shadow().a() == 0
            || (state.effects().shadow_x().get() == 0.0 && state.effects().shadow_y().get() == 0.0)
        {
            None
        } else {
            Some((
                q6(state.effects().shadow_x().get() * if scaled_border_shadow { sx } else { sx/lx })?,
                q6(state.effects().shadow_y().get() * if scaled_border_shadow { sy } else { sy/ly })?,
            ))
        };
        if let Some(hull) = hull {
            if let Some(support) = effect_bounds_axes(
                hull,
                radius,
                shadow,
                state.paint().primary().a() != 0,
                state.paint().outline().a() != 0,
            )? {
                result = result.union(support.scaled(1.0 / sx, 1.0 / sy)?)?;
            }
        }
    }
    Ok(result)
}
