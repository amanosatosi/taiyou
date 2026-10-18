//! Uncached inline CoreV1 coordinator. Native contexts exist only during a call.
use crate::{compose, evaluate_with_policy, geometry::*, layout, raster::*};
use std::{cell::Cell, marker::PhantomData, sync::Arc};
pub use taiyou_types::*;
#[cfg(feature = "native")]
pub use taiyou_font::{FontCatalogBuilder, FontContext, ValidatedFontCatalog};

fn limited() -> RenderError { RenderError::new(ErrorCode::LimitExceeded, "renderer frame/intermediate reservation exceeded") }
fn convert(e: ParseError) -> RenderError { RenderError::new(e.code(), e.detail()).with_context(e.event(),e.span()) }
fn add(a: u64, b: u64) -> Result<u64, RenderError> { a.checked_add(b).ok_or_else(limited) }
fn bytes<T>(count: usize) -> Result<u64, RenderError> {
    (count as u64).checked_mul(3 * std::mem::size_of::<T>() as u64).ok_or_else(limited)
}
fn available(limits: &Limits, held: u64) -> Result<Limits, RenderError> {
    let n = limits.transient_bytes().checked_sub(held).filter(|n| *n > 0).ok_or_else(limited)?;
    limits.clone().with_transient_bytes(n).map_err(convert)
}
/// Natural script-space alignment anchor. Margins define the horizontal usable
/// rectangle; middle vertical alignment is the script midpoint. Explicit pos
/// bypasses margins. Device mapping is applied only by T011.
pub fn alignment_anchor(placement: &Placement, play_res: Size) -> Result<ScriptPoint, RenderError> {
    if let Anchor::Explicit(point) = placement.anchor() { return Ok(point); }
    let m = placement.margins();
    let a = placement.alignment().get() - 1;
    let left = f64::from(m.left());
    let right = f64::from(play_res.width()) - f64::from(m.right());
    let x = match a % 3 { 0 => left, 1 => (left + right) / 2.0, _ => right };
    let y = match a / 3 { 0 => f64::from(play_res.height()) - f64::from(m.vertical()),
        1 => f64::from(play_res.height()) / 2.0, _ => f64::from(m.vertical()) };
    Ok(ScriptPoint::new(FiniteF64::new(x).map_err(convert)?,FiniteF64::new(y).map_err(convert)?))
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
            return Err(RenderError::new(ErrorCode::UnsupportedFeature, "T014 requires CoreV1 and exactly one worker"));
        }
        let output = OutputBudget::new(&config.limits);
        Ok(Self {config, output, _not_sync: PhantomData})
    }
    pub fn config(&self) -> &RendererConfig { &self.config }
    pub fn retained_output_bytes(&self) -> u64 { self.output.retained_bytes() }

    #[cfg(feature = "native")]
    pub fn render(&mut self, track: Arc<Track>, fonts: Arc<ValidatedFontCatalog>, request: &RenderRequest) -> Result<Frame, RenderError> {
        let limits = &self.config.limits;
        let (_, length) = limits.check_output_size(request.viewport.size()).map_err(convert)?;
        if add(self.output.retained_bytes(), length as u64)? > self.output.limit() { return Err(limited()); }
        limits.check_input_bytes(track.source().len()).map_err(convert)?;
        if track.events().len() as u64 > limits.events() { return Err(limited()); }
        // Reserve composition publication, bounded diagnostics and context
        // overhead before creating any native object or collecting active work.
        let diagnostic_peak = 3 * (MAX_DIAGNOSTICS as u64 + 1)
            * (MAX_DETAIL_BYTES as u64 + std::mem::size_of::<Diagnostic>() as u64);
        let mut held = add(length as u64, add(diagnostic_peak, 2 * 1024 * 1024)?)?;
        let count = track.events().iter().filter(|e| e.is_active(request.now)).count();
        held = add(held, bytes::<&Event>(count)?)?;
        available(limits, held)?;
        let mut events = Vec::new();
        events.try_reserve_exact(count).map_err(|_| RenderError::new(ErrorCode::AllocationFailed,"active-event allocation failed"))?;
        events.extend(track.events().iter().filter(|e| e.is_active(request.now)));
        events.sort_unstable_by_key(|e| (e.layer(),e.id()));
        let mut diagnostics = DiagnosticBuffer::new();
        let mut operations = Vec::new();
        let mut total_segments = 0_u64;
        if !events.is_empty() && fonts.faces().is_empty() {
            return Err(RenderError::new(ErrorCode::FontUnavailable,"an explicit validated font is required"));
        }
        let mut ctx = FontContext::with_limits(available(limits, held)?)
            .map_err(|e| RenderError::new(e.code(),e.detail()))?;
        for event in events {
            let result = (|| {
                let eval_charge = crate::evaluate::preflight(event)?;
                let (evaluated, report) = evaluate_with_policy(&track,event.id(),request.now,self.config.profile,
                    request.unsupported,&available(limits,held)?)?;
                diagnostics.extend_finished(&report).map_err(convert)?;
                let stage_held = add(held,eval_charge)?;
                // Existing evaluator records unsupported tags; style effects
                // and decorations need an explicit renderer capability check.
                for state in evaluated.runs() {
                    if state.effects().border_x().get() != 0.0 || state.effects().shadow_x().get() != 0.0
                        || state.geometry().underline() || state.geometry().strikeout() {
                        let detail = "T014 fill-only: borders, shadows and decorations are unsupported; tolerant mode omits these effects";
                        if request.unsupported == UnsupportedPolicy::Strict {
                            return Err(RenderError::new(ErrorCode::UnsupportedFeature,detail).with_context(Some(event.id()),Some(state.source())));
                        }
                        diagnostics.push(Diagnostic::new(DiagnosticCode::UnsupportedFeature,Some(event.id()),Some(state.source()),detail)).map_err(convert)?;
                    }
                }
                let shaped = taiyou_font::shape_event(&mut ctx,&fonts,&evaluated,&Default::default(),&available(limits,stage_held)?)
                    .map_err(|e| { let mut r=RenderError::new(e.code(),e.detail()); if let Some(n)=e.native_code(){r=r.with_native_code(n);} r })?;
                diagnostics.extend_finished(shaped.diagnostics()).map_err(convert)?;
                let glyphs = shaped.runs().iter().try_fold(0usize, |n,r| n.checked_add(r.glyphs().len()).ok_or_else(limited))?;
                let shape_charge = add(bytes::<ShapedGlyph>(glyphs)?, add(bytes::<ShapedRun>(shaped.runs().len())?,
                    (shaped.source_map().text().len() as u64).checked_mul(256).ok_or_else(limited)?)?)?;
                let stage_held = add(stage_held,shape_charge)?;
                let local = Arc::new(layout(&shaped,&LayoutConstraints::new(shaped.placement().alignment())
                    .with_limits(available(limits,stage_held)?))?);
                let layout_charge = add(bytes::<PositionedGlyph>(local.glyphs().len())?, bytes::<Line>(local.lines().len())?)?;
                let stage_held = add(stage_held,layout_charge)?;
                let remaining_segments = limits.path_segments_per_frame().checked_sub(total_segments).filter(|n| *n > 0).ok_or_else(limited)?;
                let geom_limits = available(limits,stage_held)?.with_path_segments_per_frame(remaining_segments).map_err(convert)?;
                let placed = Arc::new(geometry(&mut ctx,&fonts,local,GeometryPlacement {
                    anchor: alignment_anchor(evaluated.placement(),track.script().play_res())?, origin: evaluated.placement().origin(),
                    device: DeviceMapping::new(track.script().play_res(),request.viewport.content())?,
                },&geom_limits)?);
                let mut geom_charge = bytes::<GeometryGlyph>(placed.glyphs().len())?;
                geom_charge = add(geom_charge,bytes::<GeometryInstance>(placed.glyphs().len())?)?;
                for g in placed.glyphs() {
                    total_segments = add(total_segments,g.geometry.ops().len() as u64)?;
                    geom_charge = add(geom_charge,add(bytes::<PathOp<FontPoint>>(g.outline.outline().ops().len())?,bytes::<PathOp<DevicePoint>>(g.geometry.ops().len())?)?)?;
                }
                if total_segments > limits.path_segments_per_frame() { return Err(limited()); }
                geom_charge=add(geom_charge,bytes::<CoverageGroup>(placed.glyphs().len())?)?;
                let event_charge = add(stage_held,geom_charge)?.checked_sub(held).ok_or_else(limited)?;
                let groups = coverage_groups(placed.clone(),&available(limits,add(held,event_charge)?)?)?;
                for (ordinal,group) in groups.iter().enumerate() {
                    let first = group.policy().members().start;
                    let state = &placed.layout().shaped().evaluated().runs()[placed.glyphs()[first].association];
                    let mut raster = RasterContext::with_limits(available(limits,add(held,event_charge)?)?)?;
                    let mask = rasterize(&mut raster,group)?;
                    let charge=add((mask.data().len() as u64).checked_mul(3).ok_or_else(limited)?,bytes::<DrawOp>(1)?)?;
                    // Current-event scratch remains live, and previous event
                    // masks/DrawOps stay charged until final composition.
                    available(limits,add(add(held,event_charge)?,charge)?)?;
                    held=add(held,charge)?;
                    operations.try_reserve_exact(1).map_err(|_| RenderError::new(ErrorCode::AllocationFailed,"draw list allocation failed"))?;
                    operations.push(DrawOp::new(DrawOrder::new(event.layer(),event.id(),2,u32::try_from(ordinal).map_err(|_| limited())?),
                        mask,PixelOffset::default(),Paint::Solid(state.paint().primary()),PaintSpace::Device,PaintMap::Identity,Arc::new(ResolvedClip::None)));
                }
                Ok(())
            })();
            result.map_err(|e: RenderError| { let id=e.event().or(Some(event.id())); let span=e.span().or(Some(event.source())); e.with_context(id,span) })?;
        }
        compose(request.viewport.size(),&operations,limits,&mut self.output)?.with_diagnostics(diagnostics)
    }
}
