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

fn line_border_y(local: &LocalLayout, line: &Line) -> Result<f64, RenderError> {
    let span = line.decoded();
    // GetNextLine resets border reservation for a genuinely empty hard line,
    // but not for a whitespace word removed by Compact.
    if span.start() == span.end() {
        return Ok(0.0);
    }
    let end = local.shaped().breaks().iter()
        .find(|b| b.start() == span.end()).map_or(span.end(), |b| b.end());
    let runs = local.shaped().evaluated().runs();
    let start = runs.partition_point(|run| run.decoded().is_some_and(|s| s.end() <= span.start()));
    let mut border: f64 = 0.0;
    for run in &runs[start..] {
        let decoded = run.decoded().ok_or_else(|| RenderError::new(
            ErrorCode::InvalidArgument, "collision line lacks decoded run"))?;
        if decoded.start() >= end { break; }
        border = border.max(run.effects().border_y().get());
    }
    Ok(border)
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
    policy: UnsupportedPolicy,
) -> Result<ScriptBounds, RenderError> {
    collision_footprint_with_limits(
        local,
        device,
        scaled_border_shadow,
        policy,
        &Limits::default(),
    )
}
pub fn collision_footprint_with_limits(
    local: &LocalLayout,
    device: DeviceMapping,
    scaled_border_shadow: bool,
    _policy: UnsupportedPolicy,
    limits: &Limits,
) -> Result<ScriptBounds, RenderError> {
    if local.shaped().placement().origin().is_some() {
        return Err(RenderError::new(
            ErrorCode::UnsupportedFeature,
            "fixed explicit rotation origin requires a separate collision policy",
        ));
    }
    let sx = f64::from(device.content().width()) / f64::from(device.play_res().width());
    let sy = f64::from(device.content().height()) / f64::from(device.play_res().height());
    let [lx, ly] = device.layout_scale();
    let target = device.target_scale();
    let mut result = local.collision_bounds();
    // pinterf RTS.cpp GetNextLine/MakeLines/AllocRect: reserve the first
    // line's top border and final line's bottom border regardless of paint
    // alpha. Reserve logical line metrics, not the visible glyph bitmap.
    // This supplements geometric effect support without altering local layout.
    if let (Some(first), Some(last)) = (local.lines().first(), local.lines().last()) {
        let scale = if scaled_border_shadow { sy } else { target[1] };
        let reserve = |line| -> Result<f64, RenderError> {
            let native = finite(line_border_y(local, line)? * scale * 8.0 + 0.5)?.get();
            Ok(finite(native.floor() / (8.0 * sy))?.get())
        };
        let logical = local.logical_bounds();
        result = result.union(ScriptBounds::new(
            logical.x0().get(), logical.y0().get() - reserve(first)?,
            logical.x1().get(), logical.y1().get() + reserve(last)?,
        )?)?;
    }
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
                    cos * x * lx + sin * y * ly,
                    -sin * x * lx + cos * y * ly,
                    state.geometry().rotation_x().get(),
                    state.geometry().rotation_y().get(),
                )?;
                let dx = finite(x * sx / lx)?.get();
                let dy = finite(y * sy / ly)?.get();
                include(&mut hull, ScriptBounds::new(dx, dy, dx, dy)?)?;
            }
        }
        let border = state.effects().border_style() == BorderStyle::Outline
            && (state.effects().border_x().get() > 0.0 || state.effects().border_y().get() > 0.0);
        let radius = if border {
            Some((
                q6(state.effects().border_x().get()
                    * if scaled_border_shadow { sx } else { target[0] })?,
                q6(state.effects().border_y().get()
                    * if scaled_border_shadow { sy } else { target[1] })?,
            ))
        } else {
            None
        };
        let shadow = if state.paint().shadow().a() == 0
            || state.effects().border_style() == BorderStyle::OpaqueBox
            || (state.effects().shadow_x().get() == 0.0 && state.effects().shadow_y().get() == 0.0)
        {
            None
        } else {
            Some((
                q6(state.effects().shadow_x().get()
                    * if scaled_border_shadow { sx } else { target[0] })?,
                q6(state.effects().shadow_y().get()
                    * if scaled_border_shadow { sy } else { target[1] })?,
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
    let mut start = 0;
    while start < local.glyphs().len() {
        let state = &local.shaped().evaluated().runs()[local.glyphs()[start].association_index()];
        let mut end = start + 1;
        while end < local.glyphs().len() {
            let b = &local.shaped().evaluated().runs()[local.glyphs()[end].association_index()];
            if state.shaping() != b.shaping()
                || state.geometry() != b.geometry()
                || state.effects() != b.effects()
                || state.paint() != b.paint()
                || state.karaoke() != b.karaoke()
            {
                break;
            }
            end += 1;
        }
        if state.effects().border_style() == BorderStyle::OpaqueBox {
            let zero = ScriptPoint::new(FiniteF64::ZERO, FiniteF64::ZERO);
            let boxes = crate::geometry::opaque_box_paths(
                local,
                start..end,
                device,
                zero,
                None,
                scaled_border_shadow,
                limits,
            )?;
            for path in boxes {
                let mut precise = None;
                for op in path.outline().ops() {
                    let _: PathOp<FontPoint> = op.try_map(|p| {
                        let (x, y) = path.mapping().precise_point(p)?;
                        include(&mut precise, ScriptBounds::new(x, y, x, y)?)?;
                        Ok::<_, RenderError>(p)
                    })?;
                }
                if let Some(b) = precise {
                    let bound = b.translated(
                        -f64::from(device.content().x0()),
                        -f64::from(device.content().y0()),
                    )?;
                    let visible = (state.effects().border_x().get() > 0.0
                        || state.effects().border_y().get() > 0.0)
                        && state.paint().outline().a() != 0;
                    if visible {
                        result = result.union(bound.scaled(1.0 / sx, 1.0 / sy)?)?;
                    }
                    if state.paint().shadow().a() != 0
                        && (state.effects().shadow_x().get() != 0.0
                            || state.effects().shadow_y().get() != 0.0)
                    {
                        let dx = q6(state.effects().shadow_x().get()
                            * if scaled_border_shadow { sx } else { target[0] })?;
                        let dy = q6(state.effects().shadow_y().get()
                            * if scaled_border_shadow { sy } else { target[1] })?;
                        result = result.union(
                            bound
                                .translated(
                                    f64::from(dx.ticks()) / 64.0,
                                    f64::from(dy.ticks()) / 64.0,
                                )?
                                .scaled(1.0 / sx, 1.0 / sy)?,
                        )?;
                    }
                }
            }
        }
        start = end;
    }
    Ok(result)
}
