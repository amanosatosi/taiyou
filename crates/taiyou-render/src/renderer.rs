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
std::thread_local! {
    static PREPARE_CALLS: Cell<usize> = const { Cell::new(0) };
    static RETAINED_CALLS: Cell<usize> = const { Cell::new(0) };
    static PREPARATION_ATTEMPTS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(feature = "native")]
fn limited() -> RenderError {
    RenderError::new(
        ErrorCode::LimitExceeded,
        "renderer frame/intermediate reservation exceeded",
    )
}
fn convert(e: ParseError) -> RenderError {
    RenderError::new(e.code(), e.detail())
        .with_resource_failure(e.resource_failure())
        .with_context(e.event(), e.span())
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
        .ok_or_else(|| {
            RenderError::transient_limit("renderer frame/intermediate reservation exceeded")
        })?;
    limits.clone().with_transient_bytes(n).map_err(convert)
}
#[cfg(feature = "native")]
struct GroupMasks {
    fill: Arc<Mask>,
    border: Option<Arc<Mask>>,
    shadow: Option<Arc<Mask>>,
    fill_paint: Rgba8,
    sweep: Option<(i32, Rgba8)>,
    border_paint: Rgba8,
    shadow_paint: Rgba8,
}
#[cfg(feature = "native")]
fn filter_mask(
    mask: Arc<Mask>,
    spec: &crate::effects::BlurSpec,
    held: &mut u64,
    event_charge: u64,
    limits: &Limits,
) -> Result<Arc<Mask>, RenderError> {
    let filtered =
        crate::effects::apply_effects(&mask, spec, &available(limits, add(*held, event_charge)?)?)?;
    if !Arc::ptr_eq(&mask, &filtered) {
        charge_mask(&filtered, held, event_charge, limits)?;
        let old = (mask.data().len() as u64)
            .checked_mul(3)
            .ok_or_else(limited)?;
        *held = held.checked_sub(old).ok_or_else(limited)?;
    }
    Ok(filtered)
}
#[cfg(feature = "native")]
fn q6_pixels(value: f64) -> Result<Q6, RenderError> {
    Q6::from_pixels(FiniteF64::new(value).map_err(convert)?).map_err(convert)
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
        let [lx, ly] = device.layout_scale();
        if x_axis {
            f64::from(content.width()) / f64::from(play.width()) / lx
        } else {
            f64::from(content.height()) / f64::from(play.height()) / ly
        }
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
#[allow(clippy::too_many_arguments)]
fn push_operation(
    operations: &mut Vec<DrawOp>,
    held: &mut u64,
    event_charge: u64,
    limits: &Limits,
    order: DrawOrder,
    mask: Arc<Mask>,
    paint: Rgba8,
    clip: Arc<ResolvedClip>,
) -> Result<(), RenderError> {
    let future = add(*held, add(bytes::<DrawOp>(1)?, bytes::<ResolvedClip>(3)?)?)?;
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
        clip,
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
fn device_mapping(track: &Track, request: &RenderRequest) -> Result<DeviceMapping, RenderError> {
    DeviceMapping::new(track.script().play_res(), request.viewport.content())?.with_layout_res(
        Some(
            track
                .script()
                .layout_res()
                .unwrap_or(request.viewport.original_size()),
        ),
    )
}
#[cfg(feature = "native")]
struct PreparedLocal {
    id: EventId,
    local: Arc<LocalLayout>,
    charge: u64,
    local_charge: u64,
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
    let shaped = taiyou_font::shape_event(
        ctx,
        fonts,
        &evaluated,
        &taiyou_font::ShapeOptions::default()
            .with_kerning(ass_kerning(track))
            .with_layout_scale(device_mapping(track, request)?.layout_scale())
            .map_err(crate::geometry::font_error)?,
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
    let mut constraints = LayoutConstraints::new(shaped.placement().alignment())
        .with_wrap(evaluated.wrap_mode())
        .with_metric_scale_x(
            FiniteF64::new(device_mapping(track, request)?.layout_scale()[0]).map_err(convert)?,
        )?
        .with_limits(available(limits, stage_held)?);
    if evaluated.wrap_mode().get() != 2 {
        let style_index = usize::try_from(event.style().index()).map_err(|_| limited())?;
        let base = track.styles().get(style_index).ok_or_else(limited)?;
        let margins = event.margins().inherit(base.margins());
        let width = f64::from(track.script().play_res().width())
            - f64::from(margins.left())
            - f64::from(margins.right());
        constraints = constraints.with_width(FiniteF64::new(width).map_err(convert)?)?;
    }
    let local = Arc::new(layout(&shaped, &constraints)?);
    let layout_charge = add(
        bytes::<PositionedGlyph>(local.glyphs().len())?,
        bytes::<Line>(local.lines().len())?,
    )?;
    let stage_held = add(stage_held, layout_charge)?;
    available(limits, stage_held)?;
    Ok((local, stage_held.checked_sub(held).ok_or_else(limited)?))
}
#[cfg(feature = "native")]
fn ass_kerning(track: &Track) -> bool {
    // Standard ASS measurements in pinterf use additive GDI extents; libass
    // likewise defaults Kerning to false. Its optional header is retained in
    // ScriptInfo metadata, including duplicate source order.
    track
        .script()
        .metadata()
        .iter()
        .rev()
        .find(|(key, _)| key.eq_ignore_ascii_case("Kerning"))
        .is_some_and(|(_, value)| {
            // libass ass.c parse_bool: yes prefix or positive decimal prefix.
            value
                .get(..3)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("yes"))
                || value
                    .strip_prefix('+')
                    .unwrap_or(value)
                    .bytes()
                    .take_while(u8::is_ascii_digit)
                    .any(|digit| digit != b'0')
        })
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

#[cfg(feature = "native")]
fn transition_placement(
    effect: crate::evaluate::TransitionEffect,
    local: &LocalLayout,
    event: &Event,
    now: TimeUs,
    device: DeviceMapping,
    anchor: ScriptPoint,
) -> Result<(ScriptPoint, PixelRect), RenderError> {
    use crate::evaluate::TransitionEffect;
    // pinterf ParseEffect/EF_BANNER/EF_SCROLL operate in layout eighth-pixels.
    // libass uses the same integer delay scaling but a continuous scroll shift.
    let native = |value: f64| -> Result<i32, RenderError> {
        if !value.is_finite() || value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
            return Err(limited());
        }
        Ok(value.trunc() as i32)
    };
    let elapsed = (i128::from(now.as_micros()) - i128::from(event.start().as_micros())) / 1000;
    let [sx, sy] = device.layout_scale();
    let bounds = local.logical_bounds();
    let content = device.content();
    let mut x = anchor.x().get();
    let mut y = anchor.y().get();
    let mut band = content;
    match effect {
        TransitionEffect::Banner {
            delay, rightward, ..
        } => {
            let delay = native((f64::from(delay) / sx).max(1.0))?;
            let shift = f64::from(native(elapsed as f64 * 8.0 / f64::from(delay))?) / (8.0 * sx);
            x = if rightward {
                shift - bounds.x1().get()
            } else {
                f64::from(device.play_res().width()) - shift - bounds.x0().get()
            };
        }
        TransitionEffect::Scroll {
            top,
            bottom,
            delay,
            downward,
            ..
        } => {
            let delay = native((f64::from(delay) / sy).max(1.0))?;
            let shift = native(elapsed as f64 * 8.0 / f64::from(delay))?;
            let top = native(f64::from(top) * sy * 8.0)?;
            let bottom = native(f64::from(bottom) * sy * 8.0)?;
            y = if downward {
                (f64::from(top) + f64::from(shift)) / (8.0 * sy) - bounds.y1().get()
            } else {
                (f64::from(bottom) - f64::from(shift)) / (8.0 * sy) - bounds.y0().get()
            };
            let target_y = f64::from(content.height()) / f64::from(device.play_res().height()) / sy;
            let clip_y = |value: i32| -> Result<i32, RenderError> {
                let pixel = (i64::from(value) + 4).div_euclid(8);
                native(pixel as f64 * target_y)?
                    .checked_add(content.y0())
                    .ok_or_else(limited)
            };
            let y0 = clip_y(top)?.clamp(content.y0(), content.y1());
            let y1 = clip_y(bottom)?.clamp(y0, content.y1());
            band = PixelRect::new(content.x0(), y0, content.x1(), y1).map_err(convert)?;
        }
    }
    Ok((
        ScriptPoint::new(
            FiniteF64::new(x).map_err(convert)?,
            FiniteF64::new(y).map_err(convert)?,
        ),
        band,
    ))
}

/// Move-only idle coordinator (Send, not Sync). No native handles or mutable
/// faces migrate with it. Completed frames retain the C7 origin-budget lease.
pub struct Renderer {
    config: RendererConfig,
    output: OutputBudget,
    #[cfg(feature = "native")]
    attachment_catalogs: CatalogIngestor,
    // Regression injection below ASS coordinate semantics, absent from normal
    // optimized builds. Always None for ordinary rendering.
    #[cfg(all(any(test, debug_assertions), feature = "native"))]
    continuous_test_anchors: Option<Vec<(EventId, ScriptPoint)>>,
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
            #[cfg(feature = "native")]
            attachment_catalogs: CatalogIngestor::new(),
            #[cfg(all(any(test, debug_assertions), feature = "native"))]
            continuous_test_anchors: None,
            _not_sync: PhantomData,
        })
    }
    pub fn config(&self) -> &RendererConfig {
        &self.config
    }
    pub fn retained_output_bytes(&self) -> u64 {
        self.output.retained_bytes()
    }

    /// Internal regression helper, not a supported host placement API. Accepts
    /// already-resolved continuous anchors below ASS pos/move quantization.
    /// Debug builds only; injection is cleared on both success and error.
    #[cfg(all(feature = "native", debug_assertions))]
    #[doc(hidden)]
    pub fn render_with_continuous_anchors_for_test(
        &mut self,
        track: Arc<Track>,
        fonts: Arc<ValidatedFontCatalog>,
        request: &RenderRequest,
        anchors: Vec<(EventId, ScriptPoint)>,
    ) -> Result<Frame, RenderError> {
        if self.continuous_test_anchors.is_some() {
            return Err(RenderError::new(
                ErrorCode::InvalidArgument,
                "nested test placement",
            ));
        }
        self.continuous_test_anchors = Some(anchors);
        let result = self.render(track, fonts, request);
        self.continuous_test_anchors = None;
        result
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
        // ASS attachment names are metadata. Only validated font bytes enter
        // the same explicit, ordered catalog as host-supplied faces.
        let fonts = if parsed.embedded_fonts().is_empty() {
            fonts
        } else {
            let mut context = FontContext::with_limits(self.config.limits.clone())
                .map_err(crate::geometry::font_error)?;
            let mut builder =
                FontCatalogBuilder::from_catalog(&mut context, &fonts, self.config.limits.clone())
                    .map_err(crate::geometry::font_error)?;
            for attachment in parsed.embedded_fonts() {
                let attachment_error = |e| {
                    crate::geometry::font_error(e).with_context(None, Some(attachment.source()))
                };
                let count = context
                    .face_count(attachment.shared_bytes())
                    .map_err(attachment_error)?;
                // Resource order, then ascending face index; register shares
                // one digest-backed byte blob and charges it only once.
                for index in 0..count {
                    builder
                        .register(&mut context, attachment.bytes(), index)
                        .map_err(attachment_error)?;
                }
            }
            builder
                .snapshot(&mut self.attachment_catalogs)
                .map_err(crate::geometry::font_error)?
        };
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
        let result = self.render_staged(
            track.clone(),
            fonts.clone(),
            request,
            true,
            &mut optional_used,
        );
        // Only transient memory pressure can be relieved by dropping optional
        // retention. Fixed count/extent/segment limits keep their original error
        // and context without repeating preparation. Nothing from a failed
        // attempt is published; its ownership is dropped before a memory retry.
        if optional_used
            && result
                .as_ref()
                .is_err_and(|e| e.resource_failure() == ResourceFailure::TransientMemory)
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
        #[cfg(test)]
        PREPARATION_ATTEMPTS.with(|n| n.set(n.get() + 1));
        let adjusted;
        let request = if let Some(layout) = track.script().layout_res() {
            adjusted = RenderRequest {
                viewport: request.viewport.with_original_size(layout)?,
                ..*request
            };
            &adjusted
        } else {
            request
        };
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
        // This API returns encoded RGB, not video-space YCbCr. Like libass's
        // image API and pinterf's default ARGB TransColor path, preserve ASS RGB.
        // ScriptInfo retains the matrix for a host which has the video matrix
        // and can apply the corresponding conversion. The header alone is not
        // an unsupported rendering feature and cannot determine that transform.
        let mut operations = Vec::new();
        let mut total_segments = 0_u64;
        if events.iter().any(|event| crate::requires_font(event)) && fonts.faces().is_empty() {
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
        let retain_cap = available(limits, held)?.transient_bytes() / 8;
        if retain && collision_count > 1 {
            *optional_used = true;
            prepared_charge = bytes::<(EventId, Option<PreparedLocal>)>(collision_count)?;
            held = add(held, prepared_charge)?;
            available(limits, held)?;
            prepared.try_reserve_exact(collision_count).map_err(|_| {
                RenderError::new(
                    ErrorCode::AllocationFailed,
                    "prepared layout allocation failed",
                )
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
                let mut result = prepare_local(&mut ctx, event, &preparation, held, &mut report);
                if result
                    .as_ref()
                    .is_err_and(|e| e.resource_failure() == ResourceFailure::TransientMemory)
                {
                    if let Some((_, Some(saved))) = prepared.pop() {
                        // The last admitted layout consumed the headroom needed
                        // by this preparation. Reclaim that one optional result,
                        // stop admission, and retry only this event. Its compact
                        // collision descriptor has already been published.
                        held = held.checked_sub(saved.charge).ok_or_else(limited)?;
                        retained_charge = retained_charge
                            .checked_sub(saved.charge)
                            .ok_or_else(limited)?;
                        drop(saved);
                        admitting = false;
                        #[cfg(test)]
                        RETAINED_CALLS.with(|n| n.set(n.get() - 1));
                        report = DiagnosticBuffer::new();
                        // If freeing optional state cannot help, preserve the
                        // original typed error for the existing minimal fallback.
                        result = prepare_local(&mut ctx, event, &preparation, held, &mut report)
                            .or(result);
                    }
                }
                let (local, local_charge) = result.map_err(|e| {
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
                    bounds: crate::collision_footprint::collision_footprint_with_limits(
                        &local,
                        device_mapping(&track, request)?,
                        track.script().scaled_border_shadow(),
                        request.unsupported,
                        &available(limits, add(held, local_charge)?)?,
                    )?,
                    eligible: true,
                });
                if admitting {
                    let report = report.finish().map_err(convert)?;
                    let report_charge =
                        report
                            .iter()
                            .try_fold(bytes::<Diagnostic>(report.len())?, |n, d| {
                                add(
                                    n,
                                    (d.detail().len() as u64)
                                        .checked_mul(3)
                                        .ok_or_else(limited)?,
                                )
                            })?;
                    let charge = add(local_charge, report_charge)?;
                    let next = add(retained_charge, charge)?;
                    if next <= retain_cap {
                        held = add(held, charge)?;
                        available(limits, held)?;
                        retained_charge = next;
                        #[cfg(test)]
                        RETAINED_CALLS.with(|n| n.set(n.get() + 1));
                        prepared.push((
                            event.id(),
                            Some(PreparedLocal {
                                id: event.id(),
                                local,
                                charge,
                                local_charge,
                                diagnostics: report,
                            }),
                        ));
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
        let mut prepared_remaining = prepared.len();
        // Ordinary paint order and full effects apply to explicit events too.
        for event in events {
            let result = (|| {
                let (local, local_charge) = if let Ok(index) =
                    prepared.binary_search_by_key(&event.id(), |p| p.0)
                {
                    let saved = prepared[index].1.take().ok_or_else(|| {
                        RenderError::new(
                            ErrorCode::InternalError,
                            "prepared event already consumed",
                        )
                    })?;
                    debug_assert_eq!(saved.id, event.id());
                    held = held.checked_sub(saved.charge).ok_or_else(limited)?;
                    prepared_remaining = prepared_remaining.checked_sub(1).ok_or_else(limited)?;
                    if prepared_remaining == 0 {
                        // The retained prefix has been consumed. Its empty
                        // lookup allocation must not crowd out later native
                        // preparation or provoke a full minimal retry.
                        drop(std::mem::take(&mut prepared));
                        held = held.checked_sub(prepared_charge).ok_or_else(limited)?;
                        prepared_charge = 0;
                    }
                    diagnostics
                        .extend_finished(&saved.diagnostics)
                        .map_err(convert)?;
                    // The saved report Vec drops here; the frame's bounded
                    // diagnostic reservation owns the merged records now.
                    (saved.local, saved.local_charge)
                } else {
                    prepare_local(&mut ctx, event, &preparation, held, &mut diagnostics)?
                };
                held = add(held, local_charge)?;
                let state = local.shaped().placement();
                let device = device_mapping(&track, request)?;
                // xy's integer effect coordinates belong to layout pixels,
                // before any separately declared raster-target oversampling.
                let target_scale = request.viewport.target_scale();
                let sx = f64::from(device.content().width())
                    / f64::from(device.play_res().width())
                    / target_scale[0];
                let sy = f64::from(device.content().height())
                    / f64::from(device.play_res().height())
                    / target_scale[1];
                let anchor = if collision_count > 1 && state.collision_eligible() {
                    placements
                        .binary_search_by_key(&event.id(), |p| p.id)
                        .ok()
                        .map(|i| placements[i].anchor)
                        .ok_or_else(|| {
                            RenderError::new(ErrorCode::InternalError, "missing eligible placement")
                        })?
                } else if let Some(point) = state.ass_anchor(sx, sy)? {
                    point
                } else {
                    alignment_anchor(state, track.script().play_res())?
                };
                let (anchor, transition_clip) = if let Some(effect) =
                    crate::evaluate::TransitionEffect::parse(event.effect())
                {
                    let (point, clip) =
                        transition_placement(effect, &local, event, request.now, device, anchor)?;
                    (point, Some(clip))
                } else {
                    (anchor, None)
                };
                #[cfg(any(test, debug_assertions))]
                let anchor = self
                    .continuous_test_anchors
                    .as_ref()
                    .and_then(|anchors| anchors.iter().find(|(id, _)| *id == event.id()))
                    .map_or(anchor, |(_, point)| *point);
                let remaining_segments = limits
                    .path_segments_per_frame()
                    .checked_sub(total_segments)
                    .filter(|n| *n > 0)
                    .ok_or_else(limited)?;
                let geom_limits = available(limits, held)?
                    .with_path_segments_per_frame(remaining_segments)
                    .map_err(convert)?;
                let origin = state.ass_origin(sx, sy)?;
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
                let mut geom_charge = bytes::<GeometryGlyph>(placed.glyphs().len())?
                    .checked_add(bytes::<crate::geometry::GeometryItem>(
                        placed.items().len(),
                    )?)
                    .ok_or_else(limited)?;
                geom_charge = add(
                    geom_charge,
                    bytes::<GeometryInstance>(placed.items().len())?,
                )?;
                for g in placed.items() {
                    total_segments = add(total_segments, g.geometry.ops().len() as u64)?;
                    geom_charge = add(
                        geom_charge,
                        add(
                            bytes::<PathOp<FontPoint>>(g.geometry.outline().ops().len())?,
                            bytes::<PathOp<DevicePoint>>(g.geometry.ops().len())?,
                        )?,
                    )?;
                }
                if total_segments > limits.path_segments_per_frame() {
                    return Err(limited());
                }
                geom_charge = add(geom_charge, bytes::<CoverageGroup>(placed.items().len())?)?;
                let mut event_charge = geom_charge;
                let (mut event_clip, clip_mask, clip_segments) = crate::ass_clip::resolve(
                    placed.layout().shaped().evaluated().ass_clip(),
                    device,
                    &available(limits, add(held, event_charge)?)?,
                )?;
                if let Some(bounds) = transition_clip {
                    event_charge = add(event_charge, bytes::<ResolvedClip>(2)?)?;
                    available(limits, add(held, event_charge)?)?;
                    event_clip = Arc::new(ResolvedClip::Intersection(
                        event_clip,
                        Arc::new(ResolvedClip::Rectangle {
                            bounds,
                            inverse: false,
                        }),
                    ));
                }
                total_segments = add(total_segments, clip_segments)?;
                if total_segments > limits.path_segments_per_frame() {
                    return Err(limited());
                }
                if let Some(mask) = clip_mask {
                    charge_mask(&mask, &mut held, event_charge, limits)?;
                }
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
                        [placed.items()[first].association];
                    let mut fill_context =
                        RasterContext::with_limits(available(limits, add(held, event_charge)?)?)?;
                    let mut fill = rasterize_xy(&mut fill_context, group)?;
                    drop(fill_context);
                    charge_mask(&fill, &mut held, event_charge, limits)?;

                    let border_enabled = state.effects().border_style() == BorderStyle::Outline;
                    let opaque = state.effects().border_style() == BorderStyle::OpaqueBox;
                    let border_value = state.effects().border_x().get();
                    let border_radius = if border_enabled {
                        let ratio = if track.script().scaled_border_shadow() {
                            f64::from(device.content().width())
                                / f64::from(device.play_res().width())
                        } else {
                            request.viewport.target_scale()[0]
                        };
                        q6_pixels(border_value * ratio)?
                    } else {
                        Q6::from_ticks(0)
                    };
                    let border_radius_y = if border_enabled {
                        let ratio = if track.script().scaled_border_shadow() {
                            f64::from(device.content().height())
                                / f64::from(device.play_res().height())
                        } else {
                            request.viewport.target_scale()[1]
                        };
                        q6_pixels(state.effects().border_y().get() * ratio)?
                    } else {
                        Q6::from_ticks(0)
                    };
                    let border_exists = border_enabled
                        && (border_value > 0.0 || state.effects().border_y().get() > 0.0);
                    let blur_spec = crate::effects::BlurSpec::new(
                        state.effects().blur(),
                        state.effects().be(),
                        request.viewport.target_scale(),
                        limits,
                    )?;
                    let br = if track.script().scaled_border_shadow() {
                        [sx * target_scale[0], sy * target_scale[1]]
                    } else {
                        target_scale
                    };
                    let box_outline = (state.effects().border_x().get() * br[0] * 8.0 + 0.5 >= 1.0
                        || state.effects().border_y().get() * br[1] * 8.0 + 0.5 >= 1.0)
                        && !state.karaoke().is_some_and(|k| {
                            k.kind == KaraokeTag::Outline && k.now_ms < i64::from(k.start_ms)
                        });
                    let mut box_charge = 0;
                    let box_group = if opaque
                        && ((box_outline && state.paint().outline().a() != 0)
                            || (state.paint().shadow().a() != 0
                                && (state.effects().shadow_x().get() != 0.0
                                    || state.effects().shadow_y().get() != 0.0)))
                    {
                        let instances = opaque_box_paths(
                            placed.layout(),
                            group.policy().members(),
                            device,
                            anchor,
                            origin,
                            track.script().scaled_border_shadow(),
                            &available(limits, add(held, event_charge)?)?,
                        )?;
                        let segments = instances
                            .iter()
                            .try_fold(0u64, |n, g| add(n, g.ops().len() as u64))?;
                        total_segments = add(total_segments, segments)?;
                        if total_segments > limits.path_segments_per_frame() {
                            return Err(limited());
                        }
                        box_charge = add(
                            bytes::<GeometryInstance>(instances.len())?,
                            segments
                                .checked_mul(
                                    4 * (std::mem::size_of::<PathOp<FontPoint>>()
                                        + std::mem::size_of::<PathOp<DevicePoint>>())
                                        as u64,
                                )
                                .ok_or_else(limited)?,
                        )?;
                        event_charge = add(event_charge, box_charge)?;
                        Some(CoverageGroup::new(
                            instances,
                            CoveragePass::Fill,
                            Profile::CoreV1,
                            &available(limits, add(held, event_charge)?)?,
                        )?)
                    } else {
                        None
                    };
                    if opaque || !border_exists {
                        fill = filter_mask(fill, &blur_spec, &mut held, event_charge, limits)?;
                    }
                    let mut expanded: Option<Arc<Mask>> = None;
                    let mut border = if let Some(boxes) = box_group.as_ref() {
                        let mut raster = RasterContext::with_limits(available(
                            limits,
                            add(held, event_charge)?,
                        )?)?;
                        let mask = rasterize_xy(&mut raster, boxes)?;
                        drop(raster);
                        charge_mask(&mask, &mut held, event_charge, limits)?;
                        let mask = filter_mask(mask, &blur_spec, &mut held, event_charge, limits)?;
                        expanded = Some(mask.clone());
                        if box_outline && state.paint().outline().a() != 0 {
                            Some(mask)
                        } else {
                            None
                        }
                    } else if border_exists {
                        let remaining = limits
                            .path_segments_per_frame()
                            .checked_sub(total_segments)
                            .ok_or_else(limited)?;
                        let mut stroke_context = RasterContext::with_limits(available(
                            limits,
                            add(held, event_charge)?,
                        )?)?;
                        let (mask, segments) = crate::stroke::rasterize_expanded_axes_xy(
                            &mut stroke_context,
                            group,
                            border_radius,
                            border_radius_y,
                            Q6::from_ticks(0),
                            Q6::from_ticks(0),
                            remaining,
                        )?;
                        drop(stroke_context);
                        total_segments = add(total_segments, segments)?;
                        charge_mask(&mask, &mut held, event_charge, limits)?;
                        let mask = filter_mask(mask, &blur_spec, &mut held, event_charge, limits)?;
                        // RTS.cpp's outline fBody flag and libass's
                        // FILTER_FILL_IN_BORDER retain E under opaque primary
                        // AND secondary paint. Subtracting F in that case
                        // punches alpha gaps into antialiased body edges.
                        let border_mask = if state.outline_has_fill() {
                            mask.clone()
                        } else {
                            let subtraction_limits = available(limits, add(held, event_charge)?)?;
                            let border_mask = crate::stroke::subtract_coverage(
                                &mask,
                                &fill,
                                &subtraction_limits,
                            )?;
                            charge_mask(&border_mask, &mut held, event_charge, limits)?;
                            border_mask
                        };
                        expanded = Some(mask);
                        Some(border_mask)
                    } else {
                        None
                    };

                    // RTS.cpp1243 tests the ASS depths before quantization.
                    // A zero-depth style has no shadow, even with opaque paint.
                    let shadow = if state.paint().shadow().a() == 0
                        || (state.effects().shadow_x().get() == 0.0
                            && state.effects().shadow_y().get() == 0.0)
                        || (!opaque && !border_exists && !state.shadow_has_fill())
                    {
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
                        let mask = if dx.ticks() == 0 && dy.ticks() == 0 {
                            Arc::clone(expanded.as_ref().unwrap_or(&fill))
                        } else if let Some(boxes) = box_group.as_ref() {
                            let mut raster = RasterContext::with_limits(available(
                                limits,
                                add(held, event_charge)?,
                            )?)?;
                            let mask =
                                crate::stroke::rasterize_translated_xy(&mut raster, boxes, dx, dy)?;
                            drop(raster);
                            charge_mask(&mask, &mut held, event_charge, limits)?;
                            filter_mask(mask, &blur_spec, &mut held, event_charge, limits)?
                        } else if border_exists {
                            let remaining = limits
                                .path_segments_per_frame()
                                .checked_sub(total_segments)
                                .ok_or_else(limited)?;
                            let mut stroke_context = RasterContext::with_limits(available(
                                limits,
                                add(held, event_charge)?,
                            )?)?;
                            let (mask, segments) = crate::stroke::rasterize_expanded_axes_xy(
                                &mut stroke_context,
                                group,
                                border_radius,
                                border_radius_y,
                                dx,
                                dy,
                                remaining,
                            )?;
                            drop(stroke_context);
                            total_segments = add(total_segments, segments)?;
                            charge_mask(&mask, &mut held, event_charge, limits)?;
                            filter_mask(mask, &blur_spec, &mut held, event_charge, limits)?
                        } else {
                            let mut shadow_context = RasterContext::with_limits(available(
                                limits,
                                add(held, event_charge)?,
                            )?)?;
                            let mask = crate::stroke::rasterize_translated_xy(
                                &mut shadow_context,
                                group,
                                dx,
                                dy,
                            )?;
                            drop(shadow_context);
                            charge_mask(&mask, &mut held, event_charge, limits)?;
                            filter_mask(mask, &blur_spec, &mut held, event_charge, limits)?
                        };
                        if !opaque && border_exists && !state.shadow_has_fill() {
                            // Both references subtract the unfiltered body
                            // from the filtered outline in the shadow overlay.
                            // Rasterize that body at the SAME shadow phase;
                            // subtracting the event's original fill is wrong.
                            let body = if dx.ticks() == 0 && dy.ticks() == 0 {
                                fill.clone()
                            } else {
                                let mut raster = RasterContext::with_limits(available(
                                    limits,
                                    add(held, event_charge)?,
                                )?)?;
                                let body = crate::stroke::rasterize_translated_xy(
                                    &mut raster,
                                    group,
                                    dx,
                                    dy,
                                )?;
                                drop(raster);
                                charge_mask(&body, &mut held, event_charge, limits)?;
                                body
                            };
                            let subtraction_limits = available(limits, add(held, event_charge)?)?;
                            let ring = crate::stroke::subtract_coverage(
                                &mask,
                                &body,
                                &subtraction_limits,
                            )?;
                            charge_mask(&ring, &mut held, event_charge, limits)?;
                            for old in [&mask, &body] {
                                if !Arc::ptr_eq(old, &fill)
                                    && !expanded.as_ref().is_some_and(|e| Arc::ptr_eq(old, e))
                                {
                                    let charge = (old.data().len() as u64)
                                        .checked_mul(3)
                                        .ok_or_else(limited)?;
                                    held = held.checked_sub(charge).ok_or_else(limited)?;
                                }
                            }
                            Some(ring)
                        } else {
                            Some(mask)
                        }
                    };
                    drop(box_group);
                    event_charge = event_charge.checked_sub(box_charge).ok_or_else(limited)?;
                    if (border_exists || (opaque && border.is_none()))
                        && border
                            .as_ref()
                            .zip(expanded.as_ref())
                            .is_none_or(|(border, expanded)| !Arc::ptr_eq(border, expanded))
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
                    let mut fill_paint = state.paint().primary();
                    let mut sweep = None;
                    if let Some(k) = state.karaoke() {
                        let mut progress = if k.now_ms < i64::from(k.start_ms) {
                            0.0
                        } else if k.kind == KaraokeTag::Sweep && k.now_ms < i64::from(k.end_ms) {
                            (k.now_ms - i64::from(k.start_ms)) as f64
                                / (i64::from(k.end_ms) - i64::from(k.start_ms)) as f64
                        } else {
                            1.0
                        };
                        if k.kind == KaraokeTag::Outline && k.now_ms < i64::from(k.start_ms) {
                            border = None;
                        }
                        if progress <= 0.0 {
                            fill_paint = state.paint().secondary();
                        } else if progress < 1.0 {
                            let angle = state.geometry().rotation_z().get() % 360.0;
                            let mut secondary = state.paint().secondary();
                            if angle > 90.0 && angle < 270.0 {
                                progress = 1.0 - progress;
                                std::mem::swap(&mut fill_paint, &mut secondary);
                            }
                            // CWord::Append adds independently rounded layout
                            // extents. Karaoke sweeps that stored m_width, not
                            // the continuous glyph cursor used by TextOut.
                            let local = placed.layout();
                            let members = group.policy().members();
                            let pen = |ordinal: usize| -> Result<f64, RenderError> {
                                let g = local.glyphs().get(ordinal).ok_or_else(limited)?;
                                let run = &local.shaped().runs()[g.run_index()];
                                let style =
                                    &local.shaped().evaluated().runs()[g.association_index()];
                                let offset = if run.drawing().is_some() {
                                    0.0
                                } else {
                                    run.glyphs()[g.glyph_index()].offset_x().get()
                                        * style.geometry().scale_x().get()
                                        * local.local_scale().0.get()
                                };
                                Ok(g.position().x().get() - offset)
                            };
                            let line = local
                                .lines()
                                .iter()
                                .find(|line| line.glyph_range().contains(&members.start))
                                .ok_or_else(limited)?;
                            let right = if members.end < line.glyph_range().end {
                                pen(members.end)?
                            } else {
                                line.logical_bounds().x1().get()
                            };
                            let width = (right - pen(members.start)?) * sx;
                            let native = ((state.effects().border_x().get() + progress * width)
                                * 8.0)
                                .trunc();
                            if !native.is_finite()
                                || native < f64::from(i32::MIN)
                                || native > f64::from(i32::MAX)
                            {
                                return Err(limited());
                            }
                            let cut = fill
                                .bounds()
                                .x0()
                                .checked_add((native as i32) >> 3)
                                .ok_or_else(limited)?;
                            sweep = Some((cut, secondary));
                        }
                    }
                    // Effects have already consumed the complete source masks.
                    // Retaining wholly offscreen results cannot affect compose,
                    // but can exhaust later shaping reservations in dense scenes.
                    // Do not cull geometry or source masks before filtering.
                    let size = request.viewport.size();
                    let frame_bounds = PixelRect::new(
                        0,
                        0,
                        i32::try_from(size.width()).map_err(|_| limited())?,
                        i32::try_from(size.height()).map_err(|_| limited())?,
                    )
                    .map_err(convert)?;
                    let masks = [Some(&fill), border.as_ref(), shadow.as_ref()];
                    if masks
                        .iter()
                        .flatten()
                        .all(|mask| !mask.bounds().intersects(frame_bounds))
                    {
                        for (i, mask) in masks.iter().enumerate() {
                            let Some(mask) = mask else {
                                continue;
                            };
                            // A coincident shadow or opaque box can share a
                            // charged mask; release that reservation only once.
                            if masks[..i]
                                .iter()
                                .flatten()
                                .any(|prior| Arc::ptr_eq(prior, mask))
                            {
                                continue;
                            }
                            let charge = (mask.data().len() as u64)
                                .checked_mul(3)
                                .ok_or_else(limited)?;
                            held = held.checked_sub(charge).ok_or_else(limited)?;
                        }
                        continue;
                    }
                    group_masks.push(GroupMasks {
                        fill,
                        border,
                        shadow,
                        fill_paint,
                        sweep,
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
                        if selector == 2 {
                            if let Some((cut, secondary)) = masks.sweep {
                                for (side, color) in [(0_u32, paint), (1, secondary)] {
                                    let rect = if side == 0 {
                                        PixelRect::new(i32::MIN, i32::MIN, cut, i32::MAX)
                                    } else {
                                        PixelRect::new(cut, i32::MIN, i32::MAX, i32::MAX)
                                    }
                                    .map_err(convert)?;
                                    let clip = Arc::new(ResolvedClip::Intersection(
                                        event_clip.clone(),
                                        Arc::new(ResolvedClip::Rectangle {
                                            bounds: rect,
                                            inverse: false,
                                        }),
                                    ));
                                    push_operation(
                                        &mut operations,
                                        &mut held,
                                        event_charge,
                                        limits,
                                        DrawOrder::new(
                                            event.layer(),
                                            event.id(),
                                            selector,
                                            u32::try_from(ordinal)
                                                .map_err(|_| limited())?
                                                .checked_mul(2)
                                                .and_then(|v| v.checked_add(side))
                                                .ok_or_else(limited)?,
                                        ),
                                        mask.clone(),
                                        color,
                                        clip,
                                    )?;
                                }
                                continue;
                            }
                        }
                        push_operation(
                            &mut operations,
                            &mut held,
                            event_charge,
                            limits,
                            DrawOrder::new(
                                event.layer(),
                                event.id(),
                                selector,
                                u32::try_from(ordinal)
                                    .map_err(|_| limited())?
                                    .checked_mul(2)
                                    .ok_or_else(limited)?,
                            ),
                            mask,
                            paint,
                            event_clip.clone(),
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
        *optional_used = false;
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

#[cfg(all(test, feature = "native"))]
mod staging_tests;

#[cfg(all(test, feature = "native"))]
mod f1_equivalence_tests;

#[cfg(all(test, feature = "native"))]
mod f1_fixture;
