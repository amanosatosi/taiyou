//! Absolute-time Core evaluation; no font, layout, raster or prior-frame state.
use std::sync::Arc;
use taiyou_types::*;

fn error(code: ErrorCode, detail: &str) -> RenderError {
    RenderError::new(code, detail)
}
fn convert(e: ParseError) -> RenderError {
    error(e.code(), e.detail())
}
fn reserve<T>(v: &mut Vec<T>) -> Result<(), RenderError> {
    v.try_reserve(1)
        .map_err(|_| error(ErrorCode::AllocationFailed, "evaluation allocation failed"))
}
fn channel_color(state: Style, channel: PaintChannel, apply: impl FnOnce(Rgba8) -> Rgba8) -> Style {
    let mut colors = [
        state.primary(),
        state.secondary(),
        state.outline(),
        state.shadow_color(),
    ];
    let index = match channel {
        PaintChannel::Primary => 0,
        PaintChannel::Secondary => 1,
        PaintChannel::Outline => 2,
        PaintChannel::Shadow => 3,
    };
    colors[index] = apply(colors[index]);
    state.with_colors(colors[0], colors[1], colors[2], colors[3])
}
fn unsupported(
    event: &Event,
    span: SourceSpan,
    policy: UnsupportedPolicy,
    diagnostics: &mut DiagnosticBuffer,
    detail: &str,
) -> Result<(), RenderError> {
    if policy == UnsupportedPolicy::Strict {
        return Err(
            error(ErrorCode::UnsupportedFeature, detail).with_context(Some(event.id()), Some(span))
        );
    }
    diagnostics
        .push(Diagnostic::new(
            DiagnosticCode::UnsupportedFeature,
            Some(event.id()),
            Some(span),
            detail,
        ))
        .map_err(convert)
}
pub(crate) fn preflight(event: &Event) -> Result<u64, RenderError> {
    // Include override-only diagnostics and empty-event storage as well as runs.
    // The multiplier covers Vec growth and Vec-to-Arc publication overlap.
    let count = event.program().len();
    let per = std::mem::size_of::<EvaluatedRun>()
        .checked_mul(3)
        .and_then(|v| v.checked_add(3 * (std::mem::size_of::<TextSourcePiece>() + 32) + 128))
        .ok_or_else(|| error(ErrorCode::LimitExceeded, "evaluation size overflow"))?;
    u64::try_from(count)
        .ok()
        .and_then(|n| n.checked_mul(u64::try_from(per).ok()?))
        .and_then(|n| n.checked_add(3 * std::mem::size_of::<EvaluatedEvent>() as u64 + 512))
        .and_then(|n| {
            event.program().iter().try_fold(n, |n, op| {
                let bytes = match op {
                    TextOp::Text { text, .. } => text.len() as u64,
                    TextOp::NoBreakSpace => 2,
                    TextOp::HardBreak | TextOp::SoftBreak => 1,
                    _ => 0,
                };
                n.checked_add(bytes.checked_mul(3)?)
            })
        })
        .ok_or_else(|| error(ErrorCode::LimitExceeded, "evaluation size overflow"))
}
fn validate_style(
    style: &Style,
    event: &Event,
    span: SourceSpan,
    policy: UnsupportedPolicy,
    d: &mut DiagnosticBuffer,
) -> Result<(), RenderError> {
    if style.font().vertical()
        || style.font().family().starts_with('@')
        || style.encoding() != 1
        || style.border_style() == BorderStyle::OpaqueBox
    {
        unsupported(
            event,
            span,
            policy,
            d,
            "vertical fonts, legacy encoding or opaque boxes require later gates",
        )?;
    }
    Ok(())
}
fn one(
    track: &Track,
    event: &Event,
    policy: UnsupportedPolicy,
    d: &mut DiagnosticBuffer,
    limits: &Limits,
) -> Result<EvaluatedEvent, RenderError> {
    let base = track
        .styles()
        .get(
            usize::try_from(event.style().index())
                .map_err(|_| error(ErrorCode::InvalidScript, "style index overflow"))?,
        )
        .ok_or_else(|| error(ErrorCode::InvalidScript, "missing event style"))?;
    let mut state = base.clone();
    let mut anchor = Anchor::Default;
    let mut changes = ChangeClass::default();
    let mut warp_mode = WarpMode::XyStyle;
    let mut warp_mode_source = None;
    let mut wrap_mode = track.script().wrap();
    let mut runs = Vec::new();
    let source_map = event_map(track, event, limits)?;
    let mut decoded_end = 0_u32;
    validate_style(&state, event, event.source(), policy, d)?;
    if !event.effect().is_empty() {
        unsupported(
            event,
            event.source(),
            policy,
            d,
            "event effects or non-hard-break wrapping remain unsupported",
        )?;
    }
    for operation in event.program() {
        match operation {
            TextOp::Text { text, source } => {
                reserve(&mut runs)?;
                runs.push(mapped_run(
                    text.clone(),
                    *source,
                    &state,
                    &source_map,
                    &mut decoded_end,
                )?);
            }
            TextOp::HardBreak | TextOp::SoftBreak | TextOp::NoBreakSpace => {
                let text = match operation {
                    TextOp::NoBreakSpace => "\u{a0}",
                    TextOp::SoftBreak if wrap_mode.get() != 2 => " ",
                    _ => "\n",
                };
                reserve(&mut runs)?;
                runs.push(mapped_run(
                    Arc::from(text),
                    event.source(),
                    &state,
                    &source_map,
                    &mut decoded_end,
                )?);
            }
            TextOp::Unsupported { source, .. } => unsupported(
                event,
                *source,
                policy,
                d,
                "unsupported operation retained without evaluation",
            )?,
            TextOp::Override(op) => {
                let before = state.clone();
                match op {
                    OverrideOp::SetWrapMode { mode, .. } => {
                        wrap_mode = mode.unwrap_or(track.script().wrap());
                    }
                    OverrideOp::SetWarpMode { mode, source } => {
                        warp_mode = *mode;
                        warp_mode_source = Some(*source);
                    }
                    OverrideOp::SetFont { family, .. } => {
                        let font = FontRequest::new(
                            family.clone(),
                            state.font().weight(),
                            state.font().italic(),
                            family.starts_with('@'),
                        );
                        state = state.with_font(font);
                    }
                    OverrideOp::SetSize { size, .. } => {
                        state = state.with_size(*size).map_err(convert)?;
                    }
                    OverrideOp::SetWeight { weight, .. } => {
                        let font = state.font().clone().with_weight(*weight);
                        state = state.with_font(font);
                    }
                    OverrideOp::SetItalic { italic, .. } => {
                        let font = state.font().clone().with_italic(*italic);
                        state = state.with_font(font);
                    }
                    OverrideOp::SetScale { axis, factor, .. } => {
                        let (x, y) = match axis {
                            ScaleAxis::X => (*factor, state.scale_y()),
                            ScaleAxis::Y => (state.scale_x(), *factor),
                        };
                        state = state.with_scale(x, y).map_err(convert)?;
                    }
                    OverrideOp::SetSpacing { spacing, .. } => state = state.with_spacing(*spacing),
                    OverrideOp::SetBorder { width, .. } => {
                        state = state.with_border(*width).map_err(convert)?;
                    }
                    OverrideOp::SetShadow { offset, .. } => {
                        state = state.with_shadow(*offset);
                    }
                    OverrideOp::SetAlignment { alignment, .. } => {
                        state = state.with_alignment(*alignment)
                    }
                    OverrideOp::SetPosition { point, .. } => {
                        if anchor != Anchor::Explicit(*point) {
                            changes = changes.with_placement();
                        }
                        anchor = Anchor::Explicit(*point);
                    }
                    OverrideOp::ResetStyle { name, .. } => {
                        if let Some(name) = name {
                            if let Some(style) = track
                                .styles()
                                .iter()
                                .rev()
                                .find(|s| s.name() == name.as_ref())
                            {
                                state = style.clone();
                            } else {
                                unsupported(
                                    event,
                                    op.source(),
                                    policy,
                                    d,
                                    "unresolved host-created named reset leaves state unchanged",
                                )?;
                            }
                        } else {
                            state = base.clone();
                        }
                    }
                    OverrideOp::SetPaint { channel, color, .. } => {
                        state = channel_color(state, *channel, |c| {
                            Rgba8::new(color.r(), color.g(), color.b(), c.a())
                        })
                    }
                    OverrideOp::SetOpacity {
                        channel: Some(channel),
                        opacity,
                        ..
                    } => {
                        state = channel_color(state, *channel, |c| {
                            Rgba8::new(c.r(), c.g(), c.b(), *opacity)
                        })
                    }
                    OverrideOp::SetOpacity {
                        channel: None,
                        opacity,
                        ..
                    } => {
                        let apply = |c: Rgba8| Rgba8::new(c.r(), c.g(), c.b(), *opacity);
                        let colors = [
                            apply(state.primary()),
                            apply(state.secondary()),
                            apply(state.outline()),
                            apply(state.shadow_color()),
                        ];
                        state = state.with_colors(colors[0], colors[1], colors[2], colors[3]);
                    }
                }
                changes = changes.union(ChangeClass::between_styles(&before, &state));
                if matches!(
                    op,
                    OverrideOp::SetFont { .. } | OverrideOp::ResetStyle { .. }
                ) {
                    validate_style(&state, event, op.source(), policy, d)?;
                }
            }
        }
    }
    if warp_mode == WarpMode::Unicode && wrap_mode.get() != 2 {
        return Err(error(ErrorCode::UnsupportedFeature,
            "warpmode1 automatic wrapping requires Unicode/native-work/C7 admission")
            .with_context(Some(event.id()), warp_mode_source.or(Some(event.source()))));
    }
    let placement = Placement::new(&state, event.margins(), anchor);
    let evaluated = EvaluatedEvent::new(
        event.id(),
        EventOrder::new(event.layer(), event.id().ordinal()),
        runs.into(),
        placement,
        changes,
    )
    .with_terminal_style(state)
    .with_wrap_mode(wrap_mode)
    .with_warp_mode(warp_mode, warp_mode_source);
    match source_map {
        Some(map) => evaluated.with_source_map(map),
        None => Ok(evaluated),
    }
}
fn event_map(
    track: &Track,
    event: &Event,
    limits: &Limits,
) -> Result<Option<Arc<SourceMappedText>>, RenderError> {
    let mut pieces = Vec::new();
    let mut wrap_mode = track.script().wrap();
    pieces
        .try_reserve_exact(event.program().len())
        .map_err(|_| error(ErrorCode::AllocationFailed, "event map allocation failed"))?;
    for (i, operation) in event.program().iter().enumerate() {
        let piece = match operation {
            TextOp::Override(OverrideOp::SetWrapMode { mode, .. }) => {
                wrap_mode = mode.unwrap_or(track.script().wrap());
                continue;
            }
            TextOp::Text { source, .. } => TextSourcePiece::Literal(*source),
            TextOp::HardBreak | TextOp::SoftBreak | TextOp::NoBreakSpace => {
                let Some(span) = event.operation_sources().and_then(|s| s[i]) else {
                    return Ok(None);
                };
                match operation {
                    TextOp::HardBreak => TextSourcePiece::HardBreak(span),
                    TextOp::SoftBreak if wrap_mode.get() == 2 => TextSourcePiece::SoftBreakLine(span),
                    TextOp::SoftBreak => TextSourcePiece::SoftBreak(span),
                    _ => TextSourcePiece::NoBreakSpace(span),
                }
            }
            _ => continue,
        };
        pieces.push(piece);
    }
    SourceMappedText::from_pieces(
        track.source_storage().clone(),
        &pieces,
        WrapMode::new(0).map_err(convert)?,
        limits,
    )
    .map(|map| Some(Arc::new(map)))
    .map_err(|e| error(e.code(), e.detail()))
}
fn mapped_run(
    text: Arc<str>,
    source: SourceSpan,
    style: &Style,
    map: &Option<Arc<SourceMappedText>>,
    end: &mut u32,
) -> Result<EvaluatedRun, RenderError> {
    let run = EvaluatedRun::new(text, source, style);
    if let Some(map) = map {
        let next = end
            .checked_add(
                u32::try_from(run.text().len())
                    .map_err(|_| error(ErrorCode::LimitExceeded, "decoded run extent overflow"))?,
            )
            .ok_or_else(|| error(ErrorCode::LimitExceeded, "decoded run extent overflow"))?;
        let span = map
            .decoded_span(*end..next)
            .map_err(|e| error(e.code(), e.detail()))?;
        *end = next;
        run.with_decoded(span)
    } else {
        Ok(run)
    }
}
/// Contract C3 entry point: strict Core evaluation of one active event.
pub fn evaluate(
    track: &Track,
    id: EventId,
    now: TimeUs,
    profile: Profile,
) -> Result<EvaluatedEvent, RenderError> {
    let (event, _) = evaluate_with_policy(
        track,
        id,
        now,
        profile,
        UnsupportedPolicy::Strict,
        &Limits::default(),
    )?;
    Ok(event)
}
/// Provisional coordinator adapter: policy/limits are explicit, diagnostics owned.
pub fn evaluate_with_policy(
    track: &Track,
    id: EventId,
    now: TimeUs,
    profile: Profile,
    policy: UnsupportedPolicy,
    limits: &Limits,
) -> Result<(EvaluatedEvent, Arc<[Diagnostic]>), RenderError> {
    if profile != Profile::CoreV1 {
        return Err(error(
            ErrorCode::UnsupportedFeature,
            "Legacy evaluation requires research gates",
        ));
    }
    let event = track
        .events()
        .iter()
        .find(|e| e.id() == id)
        .ok_or_else(|| error(ErrorCode::InvalidArgument, "event ID not found"))?;
    if !event.is_active(now) {
        return Err(error(
            ErrorCode::InvalidArgument,
            "event is inactive at requested absolute time",
        ));
    }
    if preflight(event)? > limits.transient_bytes() {
        return Err(error(
            ErrorCode::LimitExceeded,
            "evaluation transient budget exceeded",
        ));
    }
    let mut diagnostics = DiagnosticBuffer::new();
    let result = one(track, event, policy, &mut diagnostics, limits)?;
    Ok((result, diagnostics.finish().map_err(convert)?.into()))
}
pub fn evaluate_active(
    track: &Track,
    now: TimeUs,
    profile: Profile,
    policy: UnsupportedPolicy,
    limits: &Limits,
) -> Result<Evaluation, RenderError> {
    if profile != Profile::CoreV1 {
        return Err(error(
            ErrorCode::UnsupportedFeature,
            "Legacy evaluation requires research gates",
        ));
    }
    let mut cost = 0_u64;
    for event in track.events().iter().filter(|e| e.is_active(now)) {
        cost = cost
            .checked_add(preflight(event)?)
            .ok_or_else(|| error(ErrorCode::LimitExceeded, "evaluation budget overflow"))?;
    }
    if cost > limits.transient_bytes() {
        return Err(error(
            ErrorCode::LimitExceeded,
            "evaluation transient budget exceeded",
        ));
    }
    let mut diagnostics = DiagnosticBuffer::new();
    let mut events = Vec::new();
    for event in track.events().iter().filter(|e| e.is_active(now)) {
        reserve(&mut events)?;
        events.push(one(track, event, policy, &mut diagnostics, limits)?);
    }
    // Event IDs are unique; this total key preserves source order without the
    // scratch allocation used by a stable sort.
    events.sort_unstable_by_key(EvaluatedEvent::order);
    Ok(Evaluation::new(
        events.into(),
        diagnostics.finish().map_err(convert)?.into(),
    ))
}
