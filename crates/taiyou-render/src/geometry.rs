//! C5 design-unit contours -> checked Q6 paths. No rasterization or clipping.
use std::sync::Arc;
use taiyou_types::*;
pub use taiyou_types::{
    DevicePoint, FiniteF64, FontPoint, Limits, PathOp, PixelRect, Q6, ScriptPoint, Size,
};

fn finite(v: f64) -> Result<FiniteF64, RenderError> {
    FiniteF64::new(v).map_err(|_| geometry_limit())
}
fn q6(v: f64) -> Result<Q6, RenderError> {
    Q6::from_pixels(finite(v)?).map_err(|_| geometry_limit())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceMapping {
    play_res: Size,
    content: PixelRect,
    layout_scale: [FiniteF64; 2],
    has_layout: bool,
}
impl DeviceMapping {
    pub fn new(play_res: Size, content: PixelRect) -> Result<Self, RenderError> {
        if play_res.is_empty() || content.width() == 0 || content.height() == 0 {
            return Err(RenderError::new(
                ErrorCode::InvalidArgument,
                "empty geometry mapping",
            ));
        }
        Ok(Self {
            play_res,
            content,
            layout_scale: [FiniteF64::ONE; 2],
            has_layout: false,
        })
    }
    pub fn with_layout_res(mut self, layout: Option<Size>) -> Result<Self, RenderError> {
        if let Some(size) = layout {
            self.has_layout = true;
            if size.is_empty() {
                return Err(RenderError::new(
                    ErrorCode::InvalidArgument,
                    "empty layout resolution",
                ));
            }
            self.layout_scale = [
                finite(f64::from(size.width()) / f64::from(self.play_res.width()))?,
                finite(f64::from(size.height()) / f64::from(self.play_res.height()))?,
            ];
        }
        Ok(self)
    }
    pub fn layout_scale(self) -> [f64; 2] {
        self.layout_scale.map(FiniteF64::get)
    }
    pub fn target_scale(self) -> [f64; 2] {
        if !self.has_layout {
            return [1.0, 1.0];
        }
        let [x, y] = self.layout_scale();
        [
            f64::from(self.content.width()) / f64::from(self.play_res.width()) / x,
            f64::from(self.content.height()) / f64::from(self.play_res.height()) / y,
        ]
    }
    pub const fn play_res(self) -> Size {
        self.play_res
    }
    pub const fn content(self) -> PixelRect {
        self.content
    }
}
/// A complete immutable map. `position` already includes HarfBuzz offsets;
/// `scale` converts design units to script units, including local ASS scale once.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeometryMap {
    scale: (FiniteF64, FiniteF64),
    position: ScriptPoint,
    anchor: ScriptPoint,
    origin: ScriptPoint,
    rotation: FiniteF64,
    projection: [FiniteF64; 4],
    shear_offset: FontPoint,
    device: DeviceMapping,
}
impl GeometryMap {
    pub fn new(
        scale: (FiniteF64, FiniteF64),
        position: ScriptPoint,
        anchor: ScriptPoint,
        origin: Option<ScriptPoint>,
        rotation: FiniteF64,
        device: DeviceMapping,
    ) -> Result<Self, RenderError> {
        if scale.0.get() < 0.0 || scale.1.get() < 0.0 {
            return Err(RenderError::new(
                ErrorCode::InvalidArgument,
                "negative outline scale",
            ));
        }
        Ok(Self {
            scale,
            position,
            anchor,
            origin: origin.unwrap_or(anchor),
            rotation,
            projection: [FiniteF64::ZERO; 4],
            shear_offset: FontPoint::ZERO,
            device,
        })
    }
    pub fn with_projection(mut self, style: &GeometryStyle) -> Self {
        self.projection = [
            style.rotation_x(),
            style.rotation_y(),
            style.shear_x(),
            style.shear_y(),
        ];
        self
    }
    pub fn with_shear_offset(mut self, offset: FontPoint) -> Self {
        self.shear_offset = offset;
        self
    }
    pub fn point(self, p: FontPoint) -> Result<DevicePoint, RenderError> {
        let (x, y) = self.precise_point(p)?;
        Ok(DevicePoint::new(q6(x)?, q6(y)?))
    }
    /// Continuous geometric support before the final raster Q6 conversion.
    pub fn precise_point(self, p: FontPoint) -> Result<(f64, f64), RenderError> {
        // Font y-up conversion occurs exactly once, before rotation and viewport.
        let [rx, ry, fx, fy] = self.projection.map(FiniteF64::get);
        // Shear acts on the word's native coordinates before scale/rotation;
        // changing the rotation origin must not change the shear itself.
        let x = (p.x().get() + fx * (-p.y().get() + self.shear_offset.y().get()))
            * self.scale.0.get()
            + self.position.x().get()
            + self.anchor.x().get()
            - self.origin.x().get();
        let y = (-p.y().get() + fy * (p.x().get() + self.shear_offset.x().get()))
            * self.scale.1.get()
            + self.position.y().get()
            + self.anchor.y().get()
            - self.origin.y().get();
        finite(x)?;
        finite(y)?;
        let angle = self.rotation.get().rem_euclid(360.0);
        // Exact cardinal rotations avoid artificial one-tick errors at right angles.
        let (sin, cos) = match angle {
            0.0 => (0.0, 1.0),
            90.0 => (1.0, 0.0),
            180.0 => (0.0, -1.0),
            270.0 => (-1.0, 0.0),
            _ => angle.to_radians().sin_cos(),
        };
        let [lx, ly] = self.device.layout_scale();
        let (x, y) = (x * lx, y * ly);
        let (x, y) = (cos * x + sin * y, -sin * x + cos * y);
        let (x, y) = project_3d(x, y, rx, ry)?;
        let (x, y) = (
            x / lx + self.origin.x().get(),
            y / ly + self.origin.y().get(),
        );
        let x = x
            * (f64::from(self.device.content.width()) / f64::from(self.device.play_res.width()))
            + f64::from(self.device.content.x0());
        let y = y
            * (f64::from(self.device.content.height()) / f64::from(self.device.play_res.height()))
            + f64::from(self.device.content.y0());
        finite(x)?;
        finite(y)?;
        Ok((x, y))
    }
}
/// RTS.cpp CWord::Transform: Z, X, Y, then perspective. Native paths use
/// 64 units/pixel: the source 20000/1000 constants become 312.5/15.625.
pub(crate) fn project_3d(x: f64, y: f64, rx: f64, ry: f64) -> Result<(f64, f64), RenderError> {
    if rx.rem_euclid(360.0) == 0.0 && ry.rem_euclid(360.0) == 0.0 {
        return Ok((x, y));
    }
    let trig = |a: f64| match a.rem_euclid(360.0) {
        0.0 => (0.0, 1.0),
        90.0 => (1.0, 0.0),
        180.0 => (0.0, -1.0),
        270.0 => (-1.0, 0.0),
        a => a.to_radians().sin_cos(),
    };
    let (sx, cx) = trig(rx);
    let (sy, cy) = trig(ry);
    let z = sx * y;
    let denom = (312.5 + sy * x - cy * z).max(15.625);
    let out = ((cy * x + sy * z) * 312.5 / denom, cx * y * 312.5 / denom);
    finite(out.0)?;
    finite(out.1)?;
    Ok(out)
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceBounds {
    min: DevicePoint,
    max: DevicePoint,
}
impl DeviceBounds {
    pub const fn min(self) -> DevicePoint {
        self.min
    }
    pub const fn max(self) -> DevicePoint {
        self.max
    }
}
/// Local Q6 commands plus whole-pixel offset reconstruct the absolute mapped path.
/// Bounds are conservative control bounds, distinct from layout ink/collision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometryInstance {
    outline: Arc<Outline>,
    mapping: GeometryMap,
    ops: Arc<[PathOp<DevicePoint>]>,
    offset: PixelOffset,
    phase: (u8, u8),
    bounds: Option<DeviceBounds>,
}
impl GeometryInstance {
    pub fn outline(&self) -> &Arc<Outline> {
        &self.outline
    }
    pub const fn mapping(&self) -> GeometryMap {
        self.mapping
    }
    pub fn ops(&self) -> &[PathOp<DevicePoint>] {
        &self.ops
    }
    pub const fn offset(&self) -> PixelOffset {
        self.offset
    }
    pub const fn phase(&self) -> (u8, u8) {
        self.phase
    }
    pub const fn bounds(&self) -> Option<DeviceBounds> {
        self.bounds
    }
}
pub fn map_outline(
    outline: Arc<Outline>,
    mapping: GeometryMap,
    limits: &Limits,
) -> Result<GeometryInstance, RenderError> {
    check_path_budget(outline.ops().len(), limits)?;
    let zero = mapping.point(FontPoint::ZERO)?;
    let (ox, px) = zero.x().split();
    let (oy, py) = zero.y().split();
    let mut ops = Vec::new();
    ops.try_reserve_exact(outline.ops().len()).map_err(|_| {
        RenderError::new(ErrorCode::AllocationFailed, "device path allocation failed")
    })?;
    let mut bounds: Option<DeviceBounds> = None;
    for &op in outline.ops() {
        ops.push(op.try_map(|p| {
            let p = mapping.point(p)?;
            bounds = Some(match bounds {
                None => DeviceBounds { min: p, max: p },
                Some(b) => DeviceBounds {
                    min: DevicePoint::new(p.x().min(b.min.x()), p.y().min(b.min.y())),
                    max: DevicePoint::new(p.x().max(b.max.x()), p.y().max(b.max.y())),
                },
            });
            let local = |q: Q6, offset: i32| {
                i32::try_from(i64::from(q.ticks()) - i64::from(offset) * 64)
                    .map(Q6::from_ticks)
                    .map_err(|_| geometry_limit())
            };
            Ok(DevicePoint::new(local(p.x(), ox)?, local(p.y(), oy)?))
        })?);
    }
    if let Some(b) = bounds {
        let cap = i64::from(limits.max_dimension()) * 64;
        if i64::from(b.max.x().ticks()) - i64::from(b.min.x().ticks()) > cap
            || i64::from(b.max.y().ticks()) - i64::from(b.min.y().ticks()) > cap
        {
            return Err(geometry_limit());
        }
    }
    Ok(GeometryInstance {
        outline,
        mapping,
        ops: ops.into(),
        offset: PixelOffset::new(ox, oy),
        phase: (px as u8, py as u8),
        bounds,
    })
}

/// Placement is explicit: layout is local and does not infer viewport margins.
/// An optional host/ASS origin is absolute script space. None uses the supplied
/// alignment anchor; parsed org uses the same map for text and drawing geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeometryPlacement {
    pub anchor: ScriptPoint,
    pub origin: Option<ScriptPoint>,
    pub device: DeviceMapping,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometryGlyph {
    pub outline: GlyphOutline,
    pub geometry: GeometryInstance,
    pub cluster: DecodedSpan,
    pub association: usize,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometryEvent {
    layout: Arc<LocalLayout>,
    glyphs: Arc<[GeometryGlyph]>,
    items: Arc<[GeometryItem]>,
}
/// Paint-ordered geometry, including drawings without a fabricated font face.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometryItem {
    pub geometry: GeometryInstance,
    pub cluster: DecodedSpan,
    pub association: usize,
}
impl GeometryEvent {
    pub fn layout(&self) -> &LocalLayout {
        &self.layout
    }
    pub fn glyphs(&self) -> &[GeometryGlyph] {
        &self.glyphs
    }
    pub fn items(&self) -> &[GeometryItem] {
        &self.items
    }
}
#[cfg(feature = "native")]
pub(crate) fn font_error(e: FontError) -> RenderError {
    let mut result = RenderError::new(e.code(), e.detail())
        .with_resource_failure(e.resource_failure())
        .with_context(e.event(), e.span());
    if let Some(code) = e.native_code() {
        result = result.with_native_code(code);
    }
    result
}
/// RTS.cpp CreateOpaqueBox: a polygon per compatible word segment on each
/// finalized line, using text advance/cell height, not glyph ink. Its copied
/// style applies scale/shear/rotation again, including the historical double
/// scale of already measured word dimensions. Drawings use the same path.
pub fn opaque_box_paths(
    layout: &LocalLayout,
    members: std::ops::Range<usize>,
    device: DeviceMapping,
    anchor: ScriptPoint,
    origin: Option<ScriptPoint>,
    scaled_bas: bool,
    limits: &Limits,
) -> Result<Vec<GeometryInstance>, RenderError> {
    let count = members
        .end
        .checked_sub(members.start)
        .ok_or_else(geometry_limit)?;
    let charge = (count as u64)
        .checked_mul(
            3 * (std::mem::size_of::<GeometryInstance>()
                + 5 * (std::mem::size_of::<PathOp<FontPoint>>()
                    + std::mem::size_of::<PathOp<DevicePoint>>())) as u64,
        )
        .ok_or_else(geometry_limit)?;
    if charge > limits.transient_bytes() {
        return Err(geometry_limit().with_resource_failure(ResourceFailure::TransientMemory));
    }
    let [lx, ly] = device.layout_scale();
    let (extra_x, extra_y) = layout.local_scale();
    let mut paths = Vec::new();
    paths.try_reserve_exact(count).map_err(|_| {
        RenderError::new(
            ErrorCode::AllocationFailed,
            "opaque box geometry allocation",
        )
    })?;
    let gs = layout.glyphs();
    let runs = layout.shaped().runs();
    let states = layout.shaped().evaluated().runs();
    let mut segments = 0usize;
    for line in layout.lines() {
        let mut start = line.glyph_range().start.max(members.start);
        let stop = line.glyph_range().end.min(members.end);
        while start < stop {
            let state = &states[gs[start].association_index()];
            let mut end = start + 1;
            while end < stop {
                let b = &states[gs[end].association_index()];
                if state.shaping() != b.shaping()
                    || state.geometry() != b.geometry()
                    || state.effects() != b.effects()
                    || state.paint() != b.paint()
                    || state.karaoke() != b.karaoke()
                    || gs[start].is_drawing() != gs[end].is_drawing()
                {
                    break;
                }
                end += 1;
            }
            if state.effects().border_style() != BorderStyle::OpaqueBox {
                start = end;
                continue;
            }
            let sx = state.geometry().scale_x().get() * extra_x.get();
            let sy = state.geometry().scale_y().get() * extra_y.get();
            let mut bounds = None;
            for g in &gs[start..end] {
                let run = &runs[g.run_index()];
                let (x, y, w, a, d) = if let Some(draw) = run.drawing() {
                    (
                        g.position().x().get(),
                        g.position().y().get(),
                        draw.width.get() * sx,
                        draw.ascent.get() * sy,
                        draw.descent.get() * sy,
                    )
                } else {
                    let glyph = &run.glyphs()[g.glyph_index()];
                    let m = run
                        .face_metrics()
                        .iter()
                        .find(|f| f.glyphs.contains(&g.glyph_index()))
                        .map_or(run.metrics(), |f| f.metrics);
                    (
                        g.position().x().get() - glyph.offset_x().get() * sx,
                        g.position().y().get() - glyph.offset_y().get() * sy,
                        (glyph.advance_x().get() + state.shaping().spacing().get()) * sx,
                        m.ascent().get() * sy,
                        m.descent().get() * sy,
                    )
                };
                let b = ScriptBounds::new(x.min(x + w), y - a, x.max(x + w), y + d)?;
                bounds = Some(if let Some(old) = bounds {
                    ScriptBounds::union(old, b)?
                } else {
                    b
                });
            }
            if let Some(b) = bounds {
                let round = |n: f64| -> Result<f64, RenderError> {
                    let n = (n + 0.5).trunc();
                    if !n.is_finite() || n < i32::MIN as f64 || n > i32::MAX as f64 {
                        return Err(geometry_limit());
                    }
                    Ok(n)
                };
                let w = round(b.width() * lx * 8.0)? / 8.0;
                let h = round(b.height() * ly * 8.0)? / 8.0;
                let px = round(
                    state.effects().border_x().get() * if scaled_bas { lx } else { 1.0 } * 8.0,
                )? / 8.0;
                let py = round(
                    state.effects().border_y().get() * if scaled_bas { ly } else { 1.0 } * 8.0,
                )? / 8.0;
                let outline = Arc::new(Outline::new(
                    vec![
                        PathOp::MoveTo(FontPoint::new(-px, py)?),
                        PathOp::LineTo(FontPoint::new(w + px, py)?),
                        PathOp::LineTo(FontPoint::new(w + px, -h - py)?),
                        PathOp::LineTo(FontPoint::new(-px, -h - py)?),
                        PathOp::Close,
                    ],
                    limits,
                )?);
                segments = segments.checked_add(5).ok_or_else(geometry_limit)?;
                check_path_budget(segments, limits)?;
                let mapping = GeometryMap::new(
                    (finite(sx / lx)?, finite(sy / ly)?),
                    ScriptPoint::new(b.x0(), b.y0()),
                    anchor,
                    origin,
                    state.geometry().rotation_z(),
                    device,
                )?
                .with_projection(state.geometry());
                paths.push(map_outline(outline, mapping, limits)?);
            }
            start = end;
        }
    }
    Ok(paths)
}
#[cfg(feature = "native")]
pub fn geometry(
    context: &mut taiyou_font::FontContext,
    catalog: &taiyou_font::ValidatedFontCatalog,
    layout: Arc<LocalLayout>,
    placement: GeometryPlacement,
    limits: &Limits,
) -> Result<GeometryEvent, RenderError> {
    let count = layout.glyphs().len();
    let mut charge = (count as u64)
        .checked_mul(
            3 * (std::mem::size_of::<GeometryGlyph>() + std::mem::size_of::<GeometryItem>()) as u64,
        )
        .ok_or_else(geometry_limit)?;
    if count as u64 > limits.glyphs_per_event() {
        return Err(geometry_limit());
    }
    if charge > limits.transient_bytes() {
        return Err(geometry_limit().with_resource_failure(ResourceFailure::TransientMemory));
    }
    let mut glyphs = Vec::new();
    let mut items = Vec::new();
    let mut word_key = None;
    let mut word_advance = 0.0;
    items.try_reserve_exact(count).map_err(|_| {
        RenderError::new(
            ErrorCode::AllocationFailed,
            "geometry item allocation failed",
        )
    })?;
    glyphs.try_reserve_exact(count).map_err(|_| {
        RenderError::new(
            ErrorCode::AllocationFailed,
            "geometry glyph allocation failed",
        )
    })?;
    let mut total = 0usize;
    for positioned in layout.glyphs() {
        let run = &layout.shaped().runs()[positioned.run_index()];
        let state = &layout.shaped().evaluated().runs()[positioned.association_index()];
        if let Some(drawing) = run.drawing() {
            total = total
                .checked_add(drawing.outline.ops().len())
                .ok_or_else(geometry_limit)?;
            check_path_budget(total, limits)?;
            charge = charge
                .checked_add(
                    (drawing.outline.ops().len() as u64)
                        .checked_mul(
                            4 * (std::mem::size_of::<PathOp<FontPoint>>()
                                + std::mem::size_of::<PathOp<DevicePoint>>())
                                as u64,
                        )
                        .ok_or_else(geometry_limit)?,
                )
                .ok_or_else(geometry_limit)?;
            if charge > limits.transient_bytes() {
                return Err(
                    geometry_limit().with_resource_failure(ResourceFailure::TransientMemory)
                );
            }
            let additional = layout.local_scale();
            let mapping = GeometryMap::new(
                (
                    finite(state.geometry().scale_x().get() * additional.0.get())?,
                    finite(state.geometry().scale_y().get() * additional.1.get())?,
                ),
                positioned.position(),
                placement.anchor,
                placement.origin,
                state.geometry().rotation_z(),
                placement.device,
            )?
            .with_projection(state.geometry());
            items.push(GeometryItem {
                geometry: map_outline(drawing.outline.clone(), mapping, limits)?,
                cluster: positioned.cluster().clone(),
                association: positioned.association_index(),
            });
            continue;
        }
        let glyph = &run.glyphs()[positioned.glyph_index()];
        let start = positioned.cluster().start() as usize;
        let word_start = layout.shaped().source_map().text()[..start]
            .rfind([' ', '\n'])
            .map_or(0, |i| i + 1);
        let key = (positioned.association_index(), word_start);
        if word_key != Some(key) {
            word_key = Some(key);
            word_advance = 0.0;
        }
        let blob = catalog
            .faces()
            .iter()
            .find(|f| f.blob().id() == glyph.font())
            .ok_or_else(|| {
                RenderError::new(
                    ErrorCode::FontUnavailable,
                    "outline font absent from validated catalog",
                )
            })?
            .blob();
        let remaining = limits
            .transient_bytes()
            .checked_sub(charge)
            .filter(|n| *n > 0)
            .ok_or_else(|| {
                geometry_limit().with_resource_failure(ResourceFailure::TransientMemory)
            })?;
        let remaining_segments = limits
            .path_segments_per_frame()
            .checked_sub(total as u64)
            .filter(|n| *n > 0)
            .ok_or_else(geometry_limit)?;
        let intake_limits = limits
            .clone()
            .with_transient_bytes(remaining)
            .and_then(|l| l.with_path_segments_per_frame(remaining_segments))
            .map_err(|_| geometry_limit())?;
        let outline = catalog
            .outline(context, blob.id(), glyph.glyph(), &intake_limits)
            .map_err(font_error)?;
        total = total
            .checked_add(outline.outline().ops().len())
            .ok_or_else(geometry_limit)?;
        check_path_budget(total, limits)?;
        charge = charge
            .checked_add(
                (outline.outline().ops().len() as u64)
                    .checked_mul(
                        4 * (std::mem::size_of::<PathOp<FontPoint>>()
                            + std::mem::size_of::<PathOp<DevicePoint>>())
                            as u64,
                    )
                    .ok_or_else(geometry_limit)?,
            )
            .ok_or_else(geometry_limit)?;
        if charge.checked_add(4096).ok_or_else(geometry_limit)? > limits.transient_bytes() {
            return Err(geometry_limit().with_resource_failure(ResourceFailure::TransientMemory));
        }
        let [lx, ly] = placement.device.layout_scale();
        let vertical =
            state.shaping().font().vertical() || state.shaping().font().family().starts_with('@');
        let size = catalog
            .ass_oriented_font_scale(
                blob.id(),
                finite(state.shaping().font_size().get() * ly)?,
                vertical,
            )
            .map_err(font_error)?
            .get();
        let additional = layout.local_scale();
        let mapping = GeometryMap::new(
            (
                finite(size / lx * state.geometry().scale_x().get() * additional.0.get())?,
                finite(size / ly * state.geometry().scale_y().get() * additional.1.get())?,
            ),
            positioned.position(),
            placement.anchor,
            placement.origin.or(layout.shaped().placement().origin()),
            state.geometry().rotation_z(),
            placement.device,
        )?
        .with_projection(state.geometry());
        let mut path = outline.outline().clone();
        if let Some(offset) = glyph.vertical_offset() {
            let mut ops = Vec::new();
            ops.try_reserve_exact(path.ops().len()).map_err(|_| {
                RenderError::new(ErrorCode::AllocationFailed, "vertical outline allocation")
            })?;
            for op in path.ops() {
                ops.push(op.try_map(|p| {
                    FontPoint::new(
                        offset.x().get() - p.y().get(),
                        p.x().get() + offset.y().get(),
                    )
                })?);
            }
            path = Arc::new(Outline::new(ops, limits)?);
        }
        if state.geometry().underline() || state.geometry().strikeout() {
            let metrics = catalog
                .decoration_metrics(context, blob.id())
                .map_err(font_error)?;
            let extra = 5
                * (usize::from(state.geometry().underline())
                    + usize::from(state.geometry().strikeout()));
            total = total.checked_add(extra).ok_or_else(geometry_limit)?;
            check_path_budget(total, limits)?;
            charge = charge
                .checked_add(
                    (extra as u64 + path.ops().len() as u64)
                        * 4
                        * (std::mem::size_of::<PathOp<FontPoint>>()
                            + std::mem::size_of::<PathOp<DevicePoint>>())
                            as u64,
                )
                .ok_or_else(geometry_limit)?;
            if charge > limits.transient_bytes() {
                return Err(
                    geometry_limit().with_resource_failure(ResourceFailure::TransientMemory)
                );
            }
            let mut decorated = Vec::new();
            decorated
                .try_reserve_exact(path.ops().len() + extra)
                .map_err(|_| {
                    RenderError::new(ErrorCode::AllocationFailed, "decoration path allocation")
                })?;
            decorated.extend_from_slice(path.ops());
            // GDI TextOut decorations use face metrics and text advance, not
            // glyph ink. Retain the exact same path/effects/composition route.
            let width = glyph.advance_x().get() * lx / size;
            for (enabled, (centre, thickness)) in
                [state.geometry().underline(), state.geometry().strikeout()]
                    .into_iter()
                    .zip(metrics)
            {
                if !enabled || thickness <= 0 {
                    continue;
                }
                let (y0, y1) = (
                    f64::from(centre) - f64::from(thickness) / 2.0,
                    f64::from(centre) + f64::from(thickness) / 2.0,
                );
                // A decoration is another filled contour, not an opposite-
                // winding hole where it crosses a TrueType glyph.
                let (left, right) = (width.min(0.0), width.max(0.0));
                let (y0, y1) = if outline.clockwise() {
                    (y1, y0)
                } else {
                    (y0, y1)
                };
                decorated.extend([
                    PathOp::MoveTo(FontPoint::new(left, y0)?),
                    PathOp::LineTo(FontPoint::new(right, y0)?),
                    PathOp::LineTo(FontPoint::new(right, y1)?),
                    PathOp::LineTo(FontPoint::new(left, y1)?),
                    PathOp::Close,
                ]);
            }
            path = Arc::new(Outline::new(decorated, limits)?);
        }
        let (_, win_ascent, _) = catalog
            .ass_oriented_cell_metrics(blob.id(), vertical)
            .map_err(font_error)?;
        let ascent = (f64::from(win_ascent) * size * 64.0 + 0.5).floor() / 64.0;
        let mapping = mapping.with_shear_offset(FontPoint::new(
            (word_advance + glyph.offset_x().get()) * lx / size,
            (ascent - glyph.offset_y().get() * ly) / size,
        )?);
        word_advance += glyph.advance_x().get();
        finite(word_advance)?;
        let mapped = map_outline(path, mapping, limits)?;
        items.push(GeometryItem {
            geometry: mapped.clone(),
            cluster: positioned.cluster().clone(),
            association: positioned.association_index(),
        });
        glyphs.push(GeometryGlyph {
            outline,
            geometry: mapped,
            cluster: positioned.cluster().clone(),
            association: positioned.association_index(),
        });
    }
    Ok(GeometryEvent {
        layout,
        glyphs: glyphs.into(),
        items: items.into(),
    })
}
