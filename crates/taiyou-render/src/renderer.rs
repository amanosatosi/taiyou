//! Uncached inline CoreV1 coordinator. Native contexts exist only during a call.
#[cfg(feature = "native")]
use crate::collision::{CollisionInput, PlacedEvent, resolve_collisions};
#[cfg(feature = "native")]
use crate::{compose, geometry::*, layout, raster::*};
#[cfg(feature = "native")]
use std::sync::Arc;
use std::{cell::Cell, marker::PhantomData};
#[cfg(feature = "native")]
pub use taiyou_font::{FontCatalogBuilder, FontContext, ValidatedFontCatalog};
pub use taiyou_types::*;

#[cfg(all(test, feature = "native"))]
mod staging_tests;
#[cfg(all(test, feature = "native"))]
std::thread_local! {
    static PREPARE_CALLS: Cell<usize> = const { Cell::new(0) };
    static RETAINED_CALLS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(feature = "native")]
fn limited() -> RenderError {
    RenderError::new(
        ErrorCode::LimitExceeded,
        "renderer frame/intermediate reservation exceeded",
    )
}
fn convert(e: ParseError) -> RenderError {
    RenderError::new(e.code(), e.detail()).with_context(e.event(), e.span())
}
#[cfg(feature = "native")]
fn add(a: u64, b: u64) -> Result<u64, RenderError> {
    a.checked_add(b).ok_or_else(limited)
}
#[cfg(feature = "native")]
fn bytes<T>(count: usize) -> Result<u64, RenderError> {
    (count as u64)
        .checked_mul(3 * std::mem::size_of::<T>() as u64)
        .ok_or_else(limited)
}
#[cfg(feature = "native")]
fn available(limits: &Limits, held: u64) -> Result<Limits, RenderError> {
    let n = limits
        .transient_bytes()
        .checked_sub(held)
        .filter(|n| *n > 0)
        .ok_or_else(limited)?;
    limits.clone().with_transient_bytes(n).map_err(convert)
}
#[cfg(feature = "native")]
struct GroupMasks {
    fill: Arc<Mask>,
    border: Option<Arc<Mask>>,
    shadow: Option<Arc<Mask>>,
    fill_paint: Rgba8,
    border_paint: Rgba8,
    shadow_paint: Rgba8,
}
#[cfg(feature = "native")]
fn q6_pixels(value: f64) -> Result<Q6, RenderError> {
    Q6::from_pixels(FiniteF64::new(value).map_err(convert)?).map_err(convert)
}
#[cfg(feature = "native")]
fn equal_device_scale(device: DeviceMapping) -> bool {
    let content = device.content();
    let play = device.play_res();
    u64::from(content.width()).checked_mul(u64::from(play.height()))
        == u64::from(content.height()).checked_mul(u64::from(play.width()))
}
#[cfg(feature = "native")]
fn script_offset_q6(
    value: FiniteF64,
    device: DeviceMapping,
    x_axis: bool,
    scaled_border_shadow: bool,
) -> Result<Q6, RenderError> {
    let content = device.content();
    let play = device.play_res();
    let ratio = if !scaled_border_shadow {
        1.0
    } else if x_axis {
        f64::from(content.width()) / f64::from(play.width())
    } else {
        f64::from(content.height()) / f64::from(play.height())
    };
    q6_pixels(value.get() * ratio)
}
#[cfg(feature = "native")]
fn charge_mask(
    mask: &Mask,
    held: &mut u64,
    event_charge: u64,
    limits: &Limits,
) -> Result<(), RenderError> {
    let charge = u64::try_from(mask.data().len())
        .ok()
        .and_then(|n| n.checked_mul(3))
        .ok_or_else(limited)?;
    let future = add(*held, charge)?;
    available(limits, add(future, event_charge)?)?;
    *held = future;
    Ok(())
}
#[cfg(feature = "native")]
fn push_operation(
    operations: &mut Vec<DrawOp>,
    held: &mut u64,
    event_charge: u64,
    limits: &Limits,
    order: DrawOrder,
    mask: Arc<Mask>,
    paint: Rgba8,
) -> Result<(), RenderError> {
    let future = add(*held, bytes::<DrawOp>(1)?)?;
    available(limits, add(future, event_charge)?)?;
    operations.try_reserve_exact(1).map_err(|_| {
        RenderError::new(ErrorCode::AllocationFailed, "draw list allocation failed")
    })?;
    operations.push(DrawOp::new(
        order,
        mask,
        PixelOffset::default(),
        Paint::Solid(paint),
        PaintSpace::Device,
        PaintMap::Identity,
        Arc::new(ResolvedClip::None),
    ));
    *held = future;
    Ok(())
}
#[cfg(feature = "native")]
struct Preparation<'a> {
    fonts: &'a ValidatedFontCatalog,
    track: &'a Track,
    request: &'a RenderRequest,
    config: &'a RendererConfig,
}
#[cfg(feature = "native")]
struct PreparedLocal {
    id: EventId,
    local: Arc<LocalLayout>,
    charge: u64,
    diagnostics: Vec<Diagnostic>,
}
#[cfg(feature = "native")]
fn prepare_local(
    ctx: &mut FontContext,
    event: &Event,
    preparation: &Preparation<'_>,
    held: u64,
    diagnostics: &mut DiagnosticBuffer,
) -> Result<(Arc<LocalLayout>, u64), RenderError> {
    #[cfg(test)]
    PREPARE_CALLS.with(|n| n.set(n.get() + 1));
    let Preparation {
        fonts,
        track,
        request,
        config,
    } = *preparation;
    let limits = &config.limits;
    let profile = config.profile;
    let eval_charge = crate::evaluate::preflight(event)?;
    let (evaluated, report) = crate::evaluate::evaluate_event_with_policy(
        track,
        event,
        request.now,
        profile,
        request.unsupported,
        &available(limits, held)?,
    )?;
    diagnostics.extend_finished(&report).map_err(convert)?;
    let stage_held = add(held, eval_charge)?;
    // Static Core borders/shadows are rendered below. Decorations
    // remain explicitly unsupported and are omitted in tolerant mode.
    for state in evaluated.runs() {
        if state.geometry().underline() || state.geometry().strikeout() {
            let detail = "underline and strikeout decorations require a later Core stage";
            if request.unsupported == UnsupportedPolicy::Strict {
                return Err(RenderError::new(ErrorCode::UnsupportedFeature, detail)
                    .with_context(Some(event.id()), Some(state.source())));
            }
            diagnostics
                .push(Diagnostic::new(
                    DiagnosticCode::UnsupportedFeature,
                    Some(event.id()),
                    Some(state.source()),
                    detail,
                ))
                .map_err(convert)?;
        }
    }
    let shaped = taiyou_font::shape_event(
        ctx,
        fonts,
        &evaluated,
        &Default::default(),
        &available(limits, stage_held)?,
    )
    .map_err(crate::geometry::font_error)?;
    diagnostics
        .extend_finished(shaped.diagnostics())
        .map_err(convert)?;
    let glyphs = shaped.runs().iter().try_fold(0usize, |n, r| {
        n.checked_add(r.glyphs().len()).ok_or_else(limited)
    })?;
    let shape_charge = add(
        bytes::<ShapedGlyph>(glyphs)?,
        add(
            bytes::<ShapedRun>(shaped.runs().len())?,
            (shaped.source_map().text().len() as u64)
                .checked_mul(256)
                .ok_or_else(limited)?,
        )?,
    )?;
    let stage_held = add(stage_held, shape_charge)?;
    let local = Arc::new(layout(
        &shaped,
        &LayoutConstraints::new(shaped.placement().alignment())
            .with_limits(available(limits, stage_held)?),
    )?);
    let layout_charge = add(
        bytes::<PositionedGlyph>(local.glyphs().len())?,
        bytes::<Line>(local.lines().len())?,
    )?;
    let stage_held = add(stage_held, layout_charge)?;
    available(limits, stage_held)?;
    Ok((local, stage_held.checked_sub(held).ok_or_else(limited)?))
}
/// Natural script-space alignment anchor. Margins define the horizontal usable
/// rectangle; middle vertical alignment is the script midpoint. Explicit pos
/// bypasses margins. Device mapping is applied only by T011.
pub fn alignment_anchor(placement: &Placement, play_res: Size) -> Result<ScriptPoint, RenderError> {
    if let Anchor::Explicit(point) = placement.anchor() {
        return Ok(point);
    }
    let m = placement.margins();
    let a = placement.alignment().get() - 1;
    let left = f64::from(m.left());
    let right = f64::from(play_res.width()) - f64::from(m.right());
    let x = match a % 3 {
        0 => left,
        1 => (left + right) / 2.0,
        _ => right,
    };
    let y = match a / 3 {
        0 => f64::from(play_res.height()) - f64::from(m.vertical()),
        1 => f64::from(play_res.height()) / 2.0,
        _ => f64::from(m.vertical()),
    };
    Ok(ScriptPoint::new(
        FiniteF64::new(x).map_err(convert)?,
        FiniteF64::new(y).map_err(convert)?,
    ))
}

/// Move-only idle coordinator (Send, not Sync). No native handles or mutable
/// faces migrate with it. Completed frames retain the C7 origin-budget lease.
pub struct Renderer {
    config: RendererConfig,
    output: OutputBudget,
    _not_sync: PhantomData<Cell<()>>,
}
impl Renderer {
    pub fn new(config: RendererConfig) -> Result<Self, RenderError> {
        if config.workers.get() != 1 || config.profile != Profile::CoreV1 {
            return Err(RenderError::new(
                ErrorCode::UnsupportedFeature,
                "T014 requires CoreV1 and exactly one worker",
            ));
        }
        let output = OutputBudget::new(&config.limits);
        Ok(Self {
            config,
            output,
            _not_sync: PhantomData,
        })
    }
    pub fn config(&self) -> &RendererConfig {
        &self.config
    }
    pub fn retained_output_bytes(&self) -> u64 {
        self.output.retained_bytes()
    }

    /// Parsed-input convenience boundary: retain ingestion diagnostics in the
    /// completed frame, and enforce strict failures even for recover-mode input.
    #[cfg(feature = "native")]
    pub fn render_parsed(
        &mut self,
        parsed: &ParsedTrack,
        fonts: Arc<ValidatedFontCatalog>,
        request: &RenderRequest,
    ) -> Result<Frame, RenderError> {
        if request.unsupported == UnsupportedPolicy::Strict {
            for d in parsed.diagnostics() {
                let code = match d.code() {
                    DiagnosticCode::UnsupportedFeature | DiagnosticCode::UnknownSection => {
                        Some(ErrorCode::UnsupportedFeature)
                    }
                    DiagnosticCode::MalformedRecord => Some(ErrorCode::InvalidScript),
                    _ => None,
                };
                if let Some(code) = code {
                    return Err(
                        RenderError::new(code, d.detail()).with_context(d.event(), d.span())
                    );
                }
            }
        }
        let frame = self.render(parsed.track().clone(), fonts, request)?;
        let mut report = DiagnosticBuffer::new();
        report
            .extend_finished(parsed.diagnostics())
            .map_err(convert)?;
        report
            .extend_finished(frame.diagnostics())
            .map_err(convert)?;
        frame.with_diagnostics(report)
    }

    #[cfg(feature = "native")]
    pub fn render(
        &mut self,
        track: Arc<Track>,
        fonts: Arc<ValidatedFontCatalog>,
        request: &RenderRequest,
    ) -> Result<Frame, RenderError> {
        let mut optional_used = false;
        let result = self.render_staged(track.clone(), fonts.clone(), request, true, &mut optional_used);
        // Optional retention must never turn a successful minimal render into a
        // limit failure. The failed attempt owns no published frame and is fully
        // dropped (including native contexts, diagnostics and masks) before retry.
        if optional_used
            && result.as_ref().is_err_and(|e| matches!(e.code(), ErrorCode::LimitExceeded | ErrorCode::AllocationFailed))
        {
            return self.render_staged(track, fonts, request, false, &mut false);
        }
        result
    }

    #[cfg(feature = "native")]
    fn render_staged(
        &mut self,
        track: Arc<Track>,
        fonts: Arc<ValidatedFontCatalog>,
        request: &RenderRequest,
        retain: bool,
        optional_used: &mut bool,
    ) -> Result<Frame, RenderError> {
        let limits = &self.config.limits;
        let (_, length) = limits
            .check_output_size(request.viewport.size())
            .map_err(convert)?;
        if add(self.output.retained_bytes(), length as u64)? > self.output.limit() {
            return Err(limited());
        }
        limits
            .check_input_bytes(track.source().len())
            .map_err(convert)?;
        if track.events().len() as u64 > limits.events() {
            return Err(limited());
        }
        // Reserve composition publication, bounded diagnostics and context
        // overhead before creating any native object or collecting active work.
        let diagnostic_peak = 3
            * (MAX_DIAGNOSTICS as u64 + 1)
            * (MAX_DETAIL_BYTES as u64 + std::mem::size_of::<Diagnostic>() as u64);
        let mut held = add(length as u64, add(diagnostic_peak, 2 * 1024 * 1024)?)?;
        let count = track
            .events()
            .iter()
            .filter(|e| e.is_active(request.now))
            .count();
        held = add(held, bytes::<&Event>(count)?)?;
        available(limits, held)?;
        let mut events = Vec::new();
        events.try_reserve_exact(count).map_err(|_| {
            RenderError::new(
                ErrorCode::AllocationFailed,
                "active-event allocation failed",
            )
        })?;
        events.extend(track.events().iter().filter(|e| e.is_active(request.now)));
        events.sort_unstable_by_key(|e| (e.layer(), e.id()));
        let mut diagnostics = DiagnosticBuffer::new();
        if count > 0
            && (track.script().layout_res().is_some()
                || (!track.script().ycbcr_matrix().is_empty()
                    && !track.script().ycbcr_matrix().eq_ignore_ascii_case("None")))
        {
            let detail = "T014 uses PlayRes mapping and encoded RGB; LayoutRes and legacy YCbCr emulation are unsupported";
            if request.unsupported == UnsupportedPolicy::Strict {
                return Err(RenderError::new(ErrorCode::UnsupportedFeature, detail));
            }
            diagnostics
                .push(Diagnostic::new(
                    DiagnosticCode::UnsupportedFeature,
                    None,
                    None,
                    detail,
                ))
                .map_err(convert)?;
        }
        let mut operations = Vec::new();
        let mut total_segments = 0_u64;
        if !events.is_empty() && fonts.faces().is_empty() {
            return Err(RenderError::new(
                ErrorCode::FontUnavailable,
                "an explicit validated font is required",
            ));
        }
        let mut ctx = FontContext::with_limits(available(limits, held)?)
            .map_err(crate::geometry::font_error)?;
        let preparation = Preparation {
            fonts: &fonts,
            track: &track,
            request,
            config: &self.config,
        };
        // Parsed operations determine eligibility without allocating evaluated
        // text or resolving fonts. Explicit events never reach collision inputs.
        let flag_charge = bytes::<bool>(count)?;
        held = add(held, flag_charge)?;
        available(limits, held)?;
        let mut eligible = Vec::new();
        eligible.try_reserve_exact(count).map_err(|_| {
            RenderError::new(
                ErrorCode::AllocationFailed,
                "placement classification allocation failed",
            )
        })?;
        for event in &events {
            eligible.push(crate::evaluate::collision_eligible(event));
        }
        let collision_count = eligible.iter().filter(|&&value| value).count();
        let mut prepared = Vec::<(EventId, Option<PreparedLocal>)>::new();
        let mut prepared_charge = 0;
        // Admission is a deterministic fraction of the caller's remaining
        // budget, not host RAM. Keep a stable prefix of eligible paint order;
        // no eviction policy or frame-history cache is involved.
        let retain_cap = available(limits, held)?.transient_bytes() / 4;
        if retain && collision_count > 1 {
            *optional_used = true;
            prepared_charge = bytes::<(EventId, Option<PreparedLocal>)>(collision_count)?;
            held = add(held, prepared_charge)?;
            available(limits, held)?;
            prepared.try_reserve_exact(collision_count).map_err(|_| {
                RenderError::new(ErrorCode::AllocationFailed, "prepared layout allocation failed")
            })?;
        }
        let mut retained_charge = 0_u64;
        let mut admitting = retain;
        // Zero/single-eligible scenes need neither a native prepass nor a solver.
        let placements = if collision_count <= 1 {
            Vec::new()
        } else {
            let input_charge = bytes::<CollisionInput>(collision_count)?;
            held = add(held, input_charge)?;
            available(limits, held)?;
            let mut inputs = Vec::new();
            inputs.try_reserve_exact(collision_count).map_err(|_| {
                RenderError::new(
                    ErrorCode::AllocationFailed,
                    "collision descriptor allocation failed",
                )
            })?;
            for (event, &collides) in events.iter().zip(&eligible) {
                if !collides {
                    continue;
                }
                // Publish compact descriptors; optionally retain the exact
                // immutable local result and its diagnostics for the paint pass.
                let mut report = DiagnosticBuffer::new();
                let (local, charge) = prepare_local(
                    &mut ctx,
                    event,
                    &preparation,
                    held,
                    &mut report,
                )
                .map_err(|e| {
                    let id = e.event().or(Some(event.id()));
                    let span = e.span().or(Some(event.source()));
                    e.with_context(id, span)
                })?;
                let placement = local.shaped().placement();
                inputs.push(CollisionInput {
                    id: event.id(),
                    layer: event.layer(),
                    start: event.start(),
                    alignment: placement.alignment(),
                    anchor: alignment_anchor(placement, track.script().play_res())?,
                    bounds: crate::collision_footprint::collision_footprint(
                        &local,
                        DeviceMapping::new(track.script().play_res(), request.viewport.content())?,
                        track.script().scaled_border_shadow(),
                        request.unsupported,
                    )?,
                    eligible: true,
                });
                if admitting {
                    let report = report.finish().map_err(convert)?;
                    let report_charge = report.iter().try_fold(bytes::<Diagnostic>(report.len())?, |n, d| add(n, (d.detail().len() as u64).checked_mul(3).ok_or_else(limited)?))?;
                    let charge = add(charge, report_charge)?;
                    let next = add(retained_charge, charge)?;
                    if next <= retain_cap {
                        held = add(held, charge)?;
                        available(limits, held)?;
                        retained_charge = next;
                        #[cfg(test)]
                        RETAINED_CALLS.with(|n| n.set(n.get() + 1));
                        prepared.push((event.id(), Some(PreparedLocal { id: event.id(), local, charge, diagnostics: report })));
                    } else {
                        admitting = false;
                    }
                }
            }
            let placed =
                resolve_collisions(&inputs, self.config.profile, &available(limits, held)?)?;
            held = add(held, bytes::<PlacedEvent>(collision_count)?)?;
            drop(inputs);
            held = held.checked_sub(input_charge).ok_or_else(limited)?;
            placed
        };
        drop(eligible);
        held = held.checked_sub(flag_charge).ok_or_else(limited)?;
        // IDs are unique. Sorted slots permit O(log N) lookup and O(1) take
        // without moving other entries, independently of layer/paint ordering.
        prepared.sort_unstable_by_key(|p| p.0);
        // Ordinary paint order and full effects apply to explicit events too.
        for event in events {
            let result = (|| {
                let (local, local_charge) = if let Ok(index) = prepared.binary_search_by_key(&event.id(), |p| p.0) {
                    let saved = prepared[index].1.take().ok_or_else(|| RenderError::new(ErrorCode::InternalError, "prepared event already consumed"))?;
                    debug_assert_eq!(saved.id, event.id());
                    held = held.checked_sub(saved.charge).ok_or_else(limited)?;
                    diagnostics.extend_finished(&saved.diagnostics).map_err(convert)?;
                    (saved.local, saved.charge)
                } else {
                    prepare_local(&mut ctx, event, &preparation, held, &mut diagnostics)?
                };
                held = add(held, local_charge)?;
                let state = local.shaped().placement();
                let anchor = if collision_count > 1 && state.collision_eligible() {
                    placements
                        .binary_search_by_key(&event.id(), |p| p.id)
                        .ok()
                        .map(|i| placements[i].anchor)
                        .ok_or_else(|| {
                            RenderError::new(ErrorCode::InternalError, "missing eligible placement")
                        })?
                } else {
                    alignment_anchor(state, track.script().play_res())?
                };
                let remaining_segments = limits
                    .path_segments_per_frame()
                    .checked_sub(total_segments)
                    .filter(|n| *n > 0)
                    .ok_or_else(limited)?;
                let geom_limits = available(limits, held)?
                    .with_path_segments_per_frame(remaining_segments)
                    .map_err(convert)?;
                let device =
                    DeviceMapping::new(track.script().play_res(), request.viewport.content())?;
                let origin = local.shaped().placement().origin();
                let placed = Arc::new(geometry(
                    &mut ctx,
                    &fonts,
                    local,
                    GeometryPlacement {
                        anchor,
                        origin,
                        device,
                    },
                    &geom_limits,
                )?);
                let mut geom_charge = bytes::<GeometryGlyph>(placed.glyphs().len())?;
                geom_charge = add(
                    geom_charge,
                    bytes::<GeometryInstance>(placed.glyphs().len())?,
                )?;
                for g in placed.glyphs() {
                    total_segments = add(total_segments, g.geometry.ops().len() as u64)?;
                    geom_charge = add(
                        geom_charge,
                        add(
                            bytes::<PathOp<FontPoint>>(g.outline.outline().ops().len())?,
                            bytes::<PathOp<DevicePoint>>(g.geometry.ops().len())?,
                        )?,
                    )?;
                }
                if total_segments > limits.path_segments_per_frame() {
                    return Err(limited());
                }
                geom_charge = add(geom_charge, bytes::<CoverageGroup>(placed.glyphs().len())?)?;
                let mut event_charge = geom_charge;
                let groups = coverage_groups(
                    placed.clone(),
                    &available(limits, add(held, event_charge)?)?,
                )?;
                event_charge = add(event_charge, bytes::<GroupMasks>(groups.len())?)?;
                available(limits, add(held, event_charge)?)?;
                let mut group_masks = Vec::new();
                group_masks.try_reserve_exact(groups.len()).map_err(|_| {
                    RenderError::new(
                        ErrorCode::AllocationFailed,
                        "coverage pass group allocation failed",
                    )
                })?;
                for group in &groups {
                    let first = group.policy().members().start;
                    let state = &placed.layout().shaped().evaluated().runs()
                        [placed.glyphs()[first].association];
                    let mut fill_context =
                        RasterContext::with_limits(available(limits, add(held, event_charge)?)?)?;
                    let fill = rasterize(&mut fill_context, group)?;
                    drop(fill_context);
                    charge_mask(&fill, &mut held, event_charge, limits)?;

                    let mut border_enabled = state.effects().border_style() == BorderStyle::Outline;
                    let border_value = state.effects().border_x().get();
                    if border_enabled
                        && border_value > 0.0
                        && track.script().scaled_border_shadow()
                        && !equal_device_scale(device)
                    {
                        let detail = "Core isotropic border scaling requires equal device/script x and y ratios";
                        if request.unsupported == UnsupportedPolicy::Strict {
                            return Err(RenderError::new(ErrorCode::UnsupportedFeature, detail)
                                .with_context(Some(event.id()), Some(state.source())));
                        }
                        diagnostics
                            .push(Diagnostic::new(
                                DiagnosticCode::UnsupportedFeature,
                                Some(event.id()),
                                Some(state.source()),
                                detail,
                            ))
                            .map_err(convert)?;
                        border_enabled = false;
                    }
                    let border_radius = if border_enabled {
                        let ratio = if track.script().scaled_border_shadow() {
                            f64::from(device.content().width())
                                / f64::from(device.play_res().width())
                        } else {
                            1.0
                        };
                        q6_pixels(border_value * ratio)?
                    } else {
                        Q6::from_ticks(0)
                    };
                    let border_exists = border_enabled && border_value > 0.0;
                    let mut expanded: Option<Arc<Mask>> = None;
                    let border = if border_exists {
                        let remaining = limits
                            .path_segments_per_frame()
                            .checked_sub(total_segments)
                            .ok_or_else(limited)?;
                        let mut stroke_context = RasterContext::with_limits(available(
                            limits,
                            add(held, event_charge)?,
                        )?)?;
                        let (mask, segments) = crate::stroke::rasterize_expanded(
                            &mut stroke_context,
                            group,
                            border_radius,
                            Q6::from_ticks(0),
                            Q6::from_ticks(0),
                            remaining,
                        )?;
                        drop(stroke_context);
                        total_segments = add(total_segments, segments)?;
                        charge_mask(&mask, &mut held, event_charge, limits)?;
                        let subtraction_limits = available(limits, add(held, event_charge)?)?;
                        let border_mask =
                            crate::stroke::subtract_coverage(&mask, &fill, &subtraction_limits)?;
                        charge_mask(&border_mask, &mut held, event_charge, limits)?;
                        expanded = Some(mask);
                        Some(border_mask)
                    } else {
                        None
                    };

                    // Alpha zero proves this pass cannot affect any pixel. Do
                    // not quantize offsets, stroke, rasterize or reserve masks.
                    // Visible zero-offset shadows still use the coincident E/F.
                    let shadow = if state.paint().shadow().a() == 0 {
                        None
                    } else {
                        let dx = script_offset_q6(
                            state.effects().shadow_x(),
                            device,
                            true,
                            track.script().scaled_border_shadow(),
                        )?;
                        let dy = script_offset_q6(
                            state.effects().shadow_y(),
                            device,
                            false,
                            track.script().scaled_border_shadow(),
                        )?;
                        if dx.ticks() == 0 && dy.ticks() == 0 {
                            Some(Arc::clone(expanded.as_ref().unwrap_or(&fill)))
                        } else if border_exists {
                            let remaining = limits
                                .path_segments_per_frame()
                                .checked_sub(total_segments)
                                .ok_or_else(limited)?;
                            let mut stroke_context = RasterContext::with_limits(available(
                                limits,
                                add(held, event_charge)?,
                            )?)?;
                            let (mask, segments) = crate::stroke::rasterize_expanded(
                                &mut stroke_context,
                                group,
                                border_radius,
                                dx,
                                dy,
                                remaining,
                            )?;
                            drop(stroke_context);
                            total_segments = add(total_segments, segments)?;
                            charge_mask(&mask, &mut held, event_charge, limits)?;
                            Some(mask)
                        } else {
                            let mut shadow_context = RasterContext::with_limits(available(
                                limits,
                                add(held, event_charge)?,
                            )?)?;
                            let mask = crate::stroke::rasterize_translated(
                                &mut shadow_context,
                                group,
                                dx,
                                dy,
                            )?;
                            drop(shadow_context);
                            charge_mask(&mask, &mut held, event_charge, limits)?;
                            Some(mask)
                        }
                    };
                    if border_exists
                        && shadow
                            .as_ref()
                            .zip(expanded.as_ref())
                            .is_none_or(|(shadow, expanded)| !Arc::ptr_eq(shadow, expanded))
                    {
                        if let Some(expanded) = expanded.take() {
                            let charge = u64::try_from(expanded.data().len())
                                .ok()
                                .and_then(|n| n.checked_mul(3))
                                .ok_or_else(limited)?;
                            held = held.checked_sub(charge).ok_or_else(limited)?;
                        }
                    }
                    group_masks.push(GroupMasks {
                        fill,
                        border,
                        shadow,
                        fill_paint: state.paint().primary(),
                        border_paint: state.paint().outline(),
                        shadow_paint: state.paint().shadow(),
                    });
                }
                // C5 keeps pass order across every compatible group in this
                // event; per-group shadow/border/fill triplets are forbidden.
                for selector in [0_u16, 1, 2] {
                    for (ordinal, masks) in group_masks.iter().enumerate() {
                        let selected = match selector {
                            0 => masks
                                .shadow
                                .as_ref()
                                .map(|m| (m.clone(), masks.shadow_paint)),
                            1 => masks
                                .border
                                .as_ref()
                                .map(|m| (m.clone(), masks.border_paint)),
                            _ => Some((masks.fill.clone(), masks.fill_paint)),
                        };
                        let Some((mask, paint)) = selected else {
                            continue;
                        };
                        push_operation(
                            &mut operations,
                            &mut held,
                            event_charge,
                            limits,
                            DrawOrder::new(
                                event.layer(),
                                event.id(),
                                selector,
                                u32::try_from(ordinal).map_err(|_| limited())?,
                            ),
                            mask,
                            paint,
                        )?;
                    }
                }
                held = held.checked_sub(local_charge).ok_or_else(limited)?;
                Ok(())
            })();
            result.map_err(|e: RenderError| {
                let id = e.event().or(Some(event.id()));
                let span = e.span().or(Some(event.source()));
                e.with_context(id, span)
            })?;
        }
        drop(prepared);
        held = held.checked_sub(prepared_charge).ok_or_else(limited)?;
        available(limits, held)?;
        compose(
            request.viewport.size(),
            &operations,
            limits,
            &mut self.output,
        )?
        .with_diagnostics(diagnostics)
    }
}
