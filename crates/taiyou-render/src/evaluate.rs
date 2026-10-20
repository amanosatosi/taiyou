//! Absolute-time Core evaluation; no font, layout, raster or prior-frame state.
use std::sync::Arc;
use taiyou_types::*;

fn error(code: ErrorCode, detail: &str) -> RenderError {
    RenderError::new(code, detail)
}
fn convert(e: ParseError) -> RenderError {
    error(e.code(), e.detail()).with_resource_failure(e.resource_failure())
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
/// RTS.cpp CalcAnimation: ordered current-state interpolation, epsilon shortcut,
/// integer local milliseconds, and destination at/after end (even reversed).
struct Animation {
    start: i128,
    end: i128,
    accel: f64,
    depth: u32,
    now: i128,
}
impl Animation {
    fn calc(&self, dst: f64, src: f64) -> Result<FiniteF64, RenderError> {
        let value = if self.depth == 0 || (dst - src).abs() < 0.0001 {
            dst
        } else if self.now < self.start {
            src
        } else if self.now < self.end {
            let p =
                ((self.now - self.start) as f64 / (self.end - self.start) as f64).powf(self.accel);
            (1.0 - p) * src + p * dst
        } else {
            dst
        };
        FiniteF64::new(value)
            .map_err(|_| error(ErrorCode::InvalidScript, "nonfinite ASS transform result"))
    }
    fn opacity(&self, dst: u8, src: u8) -> Result<u8, RenderError> {
        // xy casts interpolated transparency, before opacity inversion.
        let value = self.calc(f64::from(255 - dst), f64::from(255 - src))?.get();
        // C++ BYTE conversion retains the low byte; extrapolating negative
        // accelerations can leave the ordinary channel domain.
        Ok(255 - (value.trunc().rem_euclid(256.0) as u8))
    }
}
fn animate_number(
    state: Style,
    base: &Style,
    property: TransformProperty,
    value: Option<FiniteF64>,
    relative: bool,
    animation: &Animation,
) -> Result<Style, RenderError> {
    let (current, original) = match property {
        TransformProperty::Size => (state.font_size(), base.font_size()),
        TransformProperty::ScaleX => (state.scale_x(), base.scale_x()),
        TransformProperty::ScaleY => (state.scale_y(), base.scale_y()),
        TransformProperty::Spacing => (state.spacing(), base.spacing()),
        TransformProperty::Border => (state.border(), base.border()),
        TransformProperty::Shadow => (state.shadow(), base.shadow()),
    };
    let n = match value {
        None => original,
        Some(value) => {
            let dst = match property {
                TransformProperty::Size if relative => {
                    current.get() + current.get() * value.get() / 10.0
                }
                TransformProperty::ScaleX | TransformProperty::ScaleY => value.get() / 100.0,
                _ => value.get(),
            };
            animation.calc(dst, current.get())?
        }
    };
    let positive = if n.get() < 0.0 { FiniteF64::ZERO } else { n };
    match property {
        TransformProperty::Size => state
            .with_size(if n.get() > 0.0 { n } else { original })
            .map_err(convert),
        TransformProperty::ScaleX => {
            let y = state.scale_y();
            state.with_scale(positive, y).map_err(convert)
        }
        TransformProperty::ScaleY => {
            let x = state.scale_x();
            state.with_scale(x, positive).map_err(convert)
        }
        TransformProperty::Spacing => Ok(state.with_spacing(n)),
        TransformProperty::Border => state.with_border(positive).map_err(convert),
        TransformProperty::Shadow => Ok(state.with_shadow(positive)),
    }
}
fn one(
    track: &Track,
    event: &Event,
    now: TimeUs,
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
    let local_ms = (i128::from(now.as_micros()) - i128::from(event.start().as_micros())) / 1000;
    let duration_ms =
        (i128::from(event.end().as_micros()) - i128::from(event.start().as_micros())) / 1000;
    let mut animation = Animation {
        start: 0,
        end: duration_ms,
        accel: 1.0,
        depth: 0,
        now: local_ms,
    };
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
                    OverrideOp::BeginTransform {
                        start_ms,
                        end_ms,
                        accel,
                        ..
                    } => {
                        animation.start = i128::from(*start_ms);
                        animation.end = if *end_ms == 0 {
                            duration_ms
                        } else {
                            i128::from(*end_ms)
                        };
                        animation.accel = accel.get();
                        animation.depth += 1;
                        changes = changes.with_animation();
                    }
                    OverrideOp::EndTransform { .. } => {
                        animation.depth -= 1;
                    }
                    OverrideOp::TransformNumber {
                        property,
                        value,
                        relative,
                        ..
                    } => {
                        state =
                            animate_number(state, base, *property, *value, *relative, &animation)?;
                    }
                    OverrideOp::SetBlur { value, .. } => {
                        // Preserve the raw target in the operation for future
                        // animation; xy clamps only after CalcAnimation.
                        let target = match value {
                            Some(target) => animation.calc(target.get(), state.blur().get())?,
                            None => base.blur(),
                        };
                        state = state
                            .with_blur(if target.get() < 0.0 {
                                FiniteF64::ZERO
                            } else {
                                target
                            })
                            .map_err(convert)?;
                    }
                    OverrideOp::SetBe { value, .. } => {
                        let target = match value {
                            Some(target) => animation.calc(target.get(), state.be().get())?,
                            None => base.be(),
                        };
                        state = state.with_be(target);
                    }
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
                        let current = match channel {
                            PaintChannel::Primary => state.primary(),
                            PaintChannel::Secondary => state.secondary(),
                            PaintChannel::Outline => state.outline(),
                            PaintChannel::Shadow => state.shadow_color(),
                        };
                        let c = |dst: u8, src: u8, shift: u32| -> Result<u8, RenderError> {
                            let v = animation
                                .calc(
                                    f64::from(u32::from(dst) << shift),
                                    f64::from(u32::from(src) << shift),
                                )?
                                .get();
                            if v < i32::MIN as f64 || v > i32::MAX as f64 {
                                return Err(error(
                                    ErrorCode::InvalidScript,
                                    "transformed color exceeds native integer range",
                                ));
                            }
                            Ok((((v as i32) >> shift) & 255) as u8)
                        };
                        let rgb = Rgba8::new(
                            c(color.r(), current.r(), 0)?,
                            c(color.g(), current.g(), 8)?,
                            c(color.b(), current.b(), 16)?,
                            255,
                        );
                        state = channel_color(state, *channel, |c| {
                            Rgba8::new(rgb.r(), rgb.g(), rgb.b(), c.a())
                        })
                    }
                    OverrideOp::SetOpacity {
                        channel: Some(channel),
                        opacity,
                        ..
                    } => {
                        let opacity = animation.opacity(
                            *opacity,
                            match channel {
                                PaintChannel::Primary => state.primary().a(),
                                PaintChannel::Secondary => state.secondary().a(),
                                PaintChannel::Outline => state.outline().a(),
                                PaintChannel::Shadow => state.shadow_color().a(),
                            },
                        )?;
                        state = channel_color(state, *channel, |c| {
                            Rgba8::new(c.r(), c.g(), c.b(), opacity)
                        })
                    }
                    OverrideOp::SetOpacity {
                        channel: None,
                        opacity,
                        ..
                    } => {
                        let apply = |c: Rgba8| -> Result<Rgba8, RenderError> {
                            Ok(Rgba8::new(
                                c.r(),
                                c.g(),
                                c.b(),
                                animation.opacity(*opacity, c.a())?,
                            ))
                        };
                        let colors = [
                            apply(state.primary())?,
                            apply(state.secondary())?,
                            apply(state.outline())?,
                            apply(state.shadow_color())?,
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
        return Err(error(
            ErrorCode::UnsupportedFeature,
            "warpmode1 automatic wrapping requires Unicode/native-work/C7 admission",
        )
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
                    TextOp::SoftBreak if wrap_mode.get() == 2 => {
                        TextSourcePiece::SoftBreakLine(span)
                    }
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
    .map_err(|e| error(e.code(), e.detail()).with_resource_failure(e.resource_failure()))
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
    evaluate_event_with_policy(track, event, now, profile, policy, limits)
}

/// Coordinator-only entry: the event reference comes from this immutable track.
/// Keep the public ID lookup at the public boundary, not in each rendering stage.
pub(crate) fn evaluate_event_with_policy(
    track: &Track,
    event: &Event,
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
    if !event.is_active(now) {
        return Err(error(
            ErrorCode::InvalidArgument,
            "event is inactive at requested absolute time",
        ));
    }
    if preflight(event)? > limits.transient_bytes() {
        return Err(RenderError::transient_limit(
            "evaluation transient budget exceeded",
        ));
    }
    let mut diagnostics = DiagnosticBuffer::new();
    let result = one(track, event, now, policy, &mut diagnostics, limits)?;
    Ok((result, diagnostics.finish().map_err(convert)?.into()))
}

/// Structural placement analysis of the parsed program, not subtitle text.
/// `one` starts at Default; only SetPosition writes the anchor. Style resets,
/// recovery literals and unsupported operations never write it. Full evaluation
/// still runs before rendering and enforces policy/errors independently.
#[cfg(any(test, feature = "native"))]
pub(crate) fn collision_eligible(event: &Event) -> bool {
    !event
        .program()
        .iter()
        .any(|operation| matches!(operation, TextOp::Override(OverrideOp::SetPosition { .. })))
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
        return Err(RenderError::transient_limit(
            "evaluation transient budget exceeded",
        ));
    }
    let mut diagnostics = DiagnosticBuffer::new();
    let mut events = Vec::new();
    for event in track.events().iter().filter(|e| e.is_active(now)) {
        reserve(&mut events)?;
        events.push(one(track, event, now, policy, &mut diagnostics, limits)?);
    }
    // Event IDs are unique; this total key preserves source order without the
    // scratch allocation used by a stable sort.
    events.sort_unstable_by_key(EvaluatedEvent::order);
    Ok(Evaluation::new(
        events.into(),
        diagnostics.finish().map_err(convert)?.into(),
    ))
}

#[cfg(test)]
mod placement_analysis_tests {
    use super::*;
    #[test]
    fn parsed_operations_match_full_evaluation_in_both_policies() {
        let source: Arc<str> = Arc::from("literal \\pos(1,2) recovery");
        let span = SourceSpan::new(&source, 0, source.len()).unwrap();
        let operation_span = SourceSpan::new(&source, 0, 0).unwrap();
        let operations = [
            TextOp::NoBreakSpace,
            TextOp::Override(OverrideOp::SetPosition {
                point: ScriptPoint::new(FiniteF64::ZERO, FiniteF64::ZERO),
                source: operation_span,
            }),
            TextOp::Override(OverrideOp::ResetStyle {
                name: None,
                source: operation_span,
            }),
            TextOp::Override(OverrideOp::ResetStyle {
                name: Some(Arc::from("Default")),
                source: operation_span,
            }),
            TextOp::Unsupported {
                name: Arc::from("malformed/recovered tag"),
                source: operation_span,
            },
            TextOp::Override(OverrideOp::SetAlignment {
                alignment: Alignment::new(7).unwrap(),
                source: operation_span,
            }),
        ];
        for a in &operations {
            for b in &operations {
                for c in &operations {
                    let event = Event::new(
                        EventId::new(0),
                        TimeUs::from_micros(0),
                        TimeUs::from_micros(2),
                        StyleId::new(0),
                        span,
                    )
                    .with_program(Arc::from([
                        a.clone(),
                        b.clone(),
                        c.clone(),
                        TextOp::Text {
                            text: source.clone(),
                            source: span,
                        },
                    ]));
                    let track = TrackIngestor::new()
                        .ingest(
                            source.clone(),
                            ScriptInfo::core_default(),
                            Arc::from([Style::core_default()]),
                            Arc::from([event]),
                            &Limits::default(),
                        )
                        .unwrap();
                    let event = &track.events()[0];
                    for policy in [UnsupportedPolicy::Strict, UnsupportedPolicy::Tolerant] {
                        if let Ok((state, _)) = evaluate_event_with_policy(
                            &track,
                            event,
                            TimeUs::from_micros(1),
                            Profile::CoreV1,
                            policy,
                            &Limits::default(),
                        ) {
                            assert_eq!(
                                collision_eligible(event),
                                state.placement().collision_eligible()
                            );
                        }
                    }
                }
            }
        }
    }
}
