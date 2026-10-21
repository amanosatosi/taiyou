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
    let has_karaoke = event
        .program()
        .iter()
        .any(|op| matches!(op, TextOp::Override(OverrideOp::Karaoke { .. })));
    let per = std::mem::size_of::<EvaluatedRun>()
        .checked_mul(3)
        .and_then(|v| v.checked_add(3 * (std::mem::size_of::<TextSourcePiece>() + 32) + 128))
        .ok_or_else(|| error(ErrorCode::LimitExceeded, "evaluation size overflow"))?;
    let mut drawing_mode = 0;
    u64::try_from(count)
        .ok()
        .and_then(|n| n.checked_mul(u64::try_from(per).ok()?))
        .and_then(|n| n.checked_add(3 * std::mem::size_of::<EvaluatedEvent>() as u64 + 512))
        .and_then(|n| {
            event.program().iter().try_fold(n, |n, op| {
                let bytes = match op {
                    TextOp::Override(OverrideOp::SetDrawingMode { scale, .. }) => {
                        drawing_mode = *scale;
                        0
                    }
                    TextOp::Text { text, .. } if drawing_mode > 0 => {
                        (text.len() as u64).checked_mul(256)?
                    }
                    TextOp::Override(OverrideOp::ClipDrawing { path, .. }) => {
                        (path.len() as u64).checked_mul(256)?
                    }
                    TextOp::Text { text, .. } => {
                        (text.len() as u64).checked_mul(if has_karaoke { per as u64 } else { 1 })?
                    }
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
    // CMD_bord and CMD_shad interpolate each current axis independently.
    if matches!(
        property,
        TransformProperty::Border | TransformProperty::Shadow
    ) {
        let (x, y) = if property == TransformProperty::Border {
            (TransformProperty::BorderX, TransformProperty::BorderY)
        } else {
            (TransformProperty::ShadowX, TransformProperty::ShadowY)
        };
        let s = animate_number(state, base, x, value, relative, animation)?;
        let s = animate_number(s, base, y, value, relative, animation)?;
        if property == TransformProperty::Shadow {
            let positive = |n: FiniteF64| if n.get() < 0.0 { FiniteF64::ZERO } else { n };
            let (x, y) = (positive(s.shadow()), positive(s.shadow_y()));
            return Ok(s
                .with_axis_shadow(ScaleAxis::X, x)
                .with_axis_shadow(ScaleAxis::Y, y));
        }
        return Ok(s);
    }
    let (current, original) = match property {
        TransformProperty::RotationZ => (state.angle(), base.angle()),
        TransformProperty::RotationX => (state.rotation_x(), base.rotation_x()),
        TransformProperty::RotationY => (state.rotation_y(), base.rotation_y()),
        TransformProperty::ShearX => (state.shear_x(), base.shear_x()),
        TransformProperty::ShearY => (state.shear_y(), base.shear_y()),
        TransformProperty::BorderX => (state.border(), base.border()),
        TransformProperty::BorderY => (state.border_y(), base.border_y()),
        TransformProperty::ShadowX => (state.shadow(), base.shadow()),
        TransformProperty::ShadowY => (state.shadow_y(), base.shadow_y()),
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
                TransformProperty::ScaleX | TransformProperty::ScaleY => value.get(),
                _ => value.get(),
            };
            if matches!(
                property,
                TransformProperty::ScaleX | TransformProperty::ScaleY
            ) {
                FiniteF64::new(animation.calc(dst, current.get() * 100.0)?.get() / 100.0)
                    .map_err(convert)?
            } else {
                animation.calc(dst, current.get())?
            }
        }
    };
    let positive = if n.get() < 0.0 { FiniteF64::ZERO } else { n };
    match property {
        TransformProperty::RotationZ => Ok(state.with_angle(n)),
        TransformProperty::RotationX
        | TransformProperty::RotationY
        | TransformProperty::ShearX
        | TransformProperty::ShearY => Ok(state.with_projection(property, n)),
        TransformProperty::BorderX => state
            .with_axis_border(ScaleAxis::X, positive)
            .map_err(convert),
        TransformProperty::BorderY => state
            .with_axis_border(ScaleAxis::Y, positive)
            .map_err(convert),
        TransformProperty::ShadowX => Ok(state.with_axis_shadow(ScaleAxis::X, n)),
        TransformProperty::ShadowY => Ok(state.with_axis_shadow(ScaleAxis::Y, n)),
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
// RTS.cpp EF_MOVE: sort times, nonpositive pair means full duration, start
// wins at t1 (also equal times); interpolation is later quantized by MoveSample.
fn move_progress(
    times: Option<[i32; 2]>,
    now: i128,
    duration: i128,
) -> Result<FiniteF64, RenderError> {
    let [a, b] = times.unwrap_or([-1, -1]);
    let (mut a, mut b) = (i128::from(a), i128::from(b));
    if b < a {
        std::mem::swap(&mut a, &mut b);
    }
    if a <= 0 && b <= 0 {
        a = 0;
        b = duration;
    }
    let p = if now <= a {
        0.0
    } else if now < b {
        (now - a) as f64 / (b - a) as f64
    } else {
        1.0
    };
    FiniteF64::new(p).map_err(convert)
}
// Keep xy's ordered piecewise tests: overlapping fad intervals do not get
// sorted or multiplied, and empty intervals never divide by zero.
fn fade_alpha(alpha: [i32; 3], times: [i32; 4], now: i128, duration: i128) -> i32 {
    let [mut a, b, mut c, mut d] = times.map(i128::from);
    if a == -1 && d == -1 {
        a = 0;
        c = duration - c;
        d = duration;
    }
    let interpolate = |from: i32, to: i32, start: i128, end: i128| {
        let p = (now - start) as f64 / (end - start) as f64;
        (f64::from(from) * (1.0 - p) + f64::from(to) * p) as i32
    };
    if now < a {
        alpha[0]
    } else if now >= a && now < b {
        interpolate(alpha[0], alpha[1], a, b)
    } else if now >= b && now < c {
        alpha[1]
    } else if now >= c && now < d {
        interpolate(alpha[1], alpha[2], c, d)
    } else if now >= d {
        alpha[2]
    } else {
        0
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
    let mut motion = None;
    let mut origin = None;
    let mut fade = None;
    let mut karaoke: Option<Karaoke> = None;
    let mut karaoke_word = false;
    let mut karaoke_raw_end=None;
    let mut clip_rectangles = Vec::new();
    let mut clip_vector = None;
    let mut alignment_seen = false;
    let mut event_alignment = base.alignment();
    let mut changes = ChangeClass::default();
    let mut drawing_mode = 0;
    let mut drawing_baseline = 0;
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
    let mut consumed = 0;
    for (index, operation) in event.program().iter().enumerate() {
        if index < consumed {
            continue;
        }
        if matches!(operation,TextOp::Text {..}|TextOp::NoBreakSpace|TextOp::HardBreak|TextOp::SoftBreak) {
            let span=event.operation_sources().and_then(|spans|spans.get(index)).copied().flatten().or(match operation {TextOp::Text {source,..}=>Some(*source),_=>None});
            if let Some(span)=span {
                // An empty/ignored override block still ends ParseString's
                // current word. Escapes have contiguous original source spans.
                if karaoke_word && karaoke_raw_end.is_some_and(|end|end!=span.start()) {
                    if let Some(k)=&mut karaoke {k.start_ms=k.end_ms;}
                    karaoke_word=false;
                }
                karaoke_raw_end=Some(span.end());
            }
        }
        if drawing_mode > 0
            && matches!(
                operation,
                TextOp::Text { .. } | TextOp::HardBreak | TextOp::SoftBreak | TextOp::NoBreakSpace
            )
        {
            // ParsePolygon receives original drawing bytes, not ParseString's
            // decoded escapes. A source gap or override ends the polygon word.
            let map = source_map.as_ref().ok_or_else(|| {
                error(
                    ErrorCode::InvalidArgument,
                    "drawing word requires original source mapping",
                )
            })?;
            let mut raw_start = None;
            let mut raw_end = 0;
            let mut length = 0_u32;
            let mut next = index;
            while let Some(op) = event.program().get(next) {
                let bytes = match op {
                    TextOp::Text { text, .. } => text.len(),
                    TextOp::HardBreak | TextOp::SoftBreak => 1,
                    TextOp::NoBreakSpace => 2,
                    _ => break,
                };
                let span = match op {
                    TextOp::Text { source, .. } => Some(*source),
                    _ => event.operation_sources().and_then(|s| s[next]),
                }
                .ok_or_else(|| {
                    error(
                        ErrorCode::InvalidArgument,
                        "drawing escape lacks original source",
                    )
                })?;
                if raw_start.is_some() && span.start() != raw_end {
                    break;
                }
                raw_start.get_or_insert(span.start());
                raw_end = span.end();
                length = length
                    .checked_add(u32::try_from(bytes).map_err(|_| {
                        error(ErrorCode::LimitExceeded, "drawing decoded length overflow")
                    })?)
                    .ok_or_else(|| {
                        error(ErrorCode::LimitExceeded, "drawing decoded length overflow")
                    })?;
                next += 1;
            }
            let end = decoded_end.checked_add(length).ok_or_else(|| {
                error(ErrorCode::LimitExceeded, "drawing decoded length overflow")
            })?;
            let text = map
                .text()
                .get(decoded_end as usize..end as usize)
                .ok_or_else(|| {
                    error(ErrorCode::InvalidArgument, "drawing decoded range mismatch")
                })?;
            let span = SourceSpan::new(
                track.source(),
                raw_start
                    .ok_or_else(|| error(ErrorCode::InvalidArgument, "missing drawing source"))?
                    as usize,
                raw_end as usize,
            )
            .map_err(convert)?;
            let raw = span.slice(track.source()).ok_or_else(|| {
                error(ErrorCode::InvalidArgument, "drawing source range mismatch")
            })?;
            let drawing = crate::drawing::parse(raw, drawing_mode, drawing_baseline, limits)?;
            reserve(&mut runs)?;
            runs.push(
                mapped_run(Arc::from(text), span, &state, &source_map, &mut decoded_end)?
                    .with_drawing(drawing)
                    .with_karaoke(karaoke),
            );
            if let Some(k) = &mut karaoke {
                k.start_ms = k.end_ms;
            }
            karaoke_word = false;
            consumed = next;
            continue;
        }
        match operation {
            TextOp::Text { text, source } => {
                if karaoke.is_some() {
                    let mut start = 0;
                    while start < text.len() {
                        let tail = &text[start..];
                        let separator = matches!(tail.as_bytes()[0], b' ' | b'\n');
                        let end = if separator {
                            start + 1
                        } else {
                            start + tail.find([' ', '\n']).unwrap_or(tail.len())
                        };
                        if separator && karaoke_word {
                            if let Some(k) = &mut karaoke {
                                k.start_ms = k.end_ms;
                            }
                            karaoke_word = false;
                        }
                        if !karaoke_word {
                            if let Some(k) = &mut karaoke {
                                k.word = decoded_end;
                            }
                        }
                        let span = if source.slice(track.source()) == Some(text.as_ref()) {
                            SourceSpan::new(
                                track.source(),
                                source.start() as usize + start,
                                source.start() as usize + end,
                            )
                            .map_err(convert)?
                        } else {
                            *source
                        };
                        reserve(&mut runs)?;
                        runs.push(
                            mapped_run(
                                Arc::from(&text[start..end]),
                                span,
                                &state,
                                &source_map,
                                &mut decoded_end,
                            )?
                            .with_karaoke(karaoke),
                        );
                        karaoke_word = !separator;
                        if separator {
                            if let Some(k) = &mut karaoke {
                                k.start_ms = k.end_ms;
                            }
                        }
                        start = end;
                    }
                    continue;
                }
                reserve(&mut runs)?;
                let run = mapped_run(text.clone(), *source, &state, &source_map, &mut decoded_end)?;
                runs.push(run);
            }
            TextOp::HardBreak | TextOp::SoftBreak | TextOp::NoBreakSpace => {
                let text = match operation {
                    TextOp::NoBreakSpace => "\u{a0}",
                    TextOp::SoftBreak if wrap_mode.get() != 2 => " ",
                    _ => "\n",
                };
                let separator = text == " " || text == "\n";
                if separator && karaoke_word {
                    if let Some(k) = &mut karaoke {
                        k.start_ms = k.end_ms;
                    }
                    karaoke_word = false;
                }
                if !karaoke_word {
                    if let Some(k) = &mut karaoke {
                        k.word = decoded_end;
                    }
                }
                reserve(&mut runs)?;
                runs.push(
                    mapped_run(
                        Arc::from(text),
                        event.source(),
                        &state,
                        &source_map,
                        &mut decoded_end,
                    )?
                    .with_karaoke(karaoke),
                );
                karaoke_word = !separator;
                if separator {
                    if let Some(k) = &mut karaoke {
                        k.start_ms = k.end_ms;
                    }
                }
            }
            TextOp::Unsupported { source, .. } => unsupported(
                event,
                *source,
                policy,
                d,
                "unsupported operation retained without evaluation",
            )?,
            TextOp::Override(op) => {
                if karaoke_word {
                    if let Some(k) = &mut karaoke {
                        k.start_ms = k.end_ms;
                    }
                    karaoke_word = false;
                }
                let before = state.clone();
                match op {
                    OverrideOp::Karaoke { kind, value_ms, .. } => {
                        let mut k = karaoke.unwrap_or(Karaoke {
                            kind: KaraokeTag::Instant,
                            start_ms: 0,
                            end_ms: 0,
                            now_ms: i64::try_from(local_ms).map_err(|_| {
                                error(ErrorCode::LimitExceeded, "karaoke local time overflow")
                            })?,
                            word: decoded_end,
                        });
                        let value = if *kind == KaraokeTag::Absolute {
                            value_ms.map_or(0.0, FiniteF64::get)
                        } else {
                            f64::from(k.end_ms) + value_ms.map_or(1000.0, FiniteF64::get)
                        };
                        if value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
                            return Err(error(ErrorCode::LimitExceeded, "karaoke clock overflow"));
                        }
                        if *kind == KaraokeTag::Absolute {
                            k.start_ms = value as i32;
                            k.end_ms = k.start_ms;
                        } else {
                            k.kind = *kind;
                            k.start_ms = k.end_ms;
                            k.end_ms = value as i32;
                        }
                        karaoke = Some(k);
                        changes = changes.with_animation();
                    }
                    OverrideOp::ClipRect {
                        coords, inverse, ..
                    } => {
                        changes = changes.with_clip();
                        let mut target = [0; 4];
                        for (to, from) in target.iter_mut().zip(coords) {
                            let n = (from.get() + 0.5).trunc();
                            if n < f64::from(i32::MIN) || n > f64::from(i32::MAX) {
                                return Err(error(
                                    ErrorCode::LimitExceeded,
                                    "clip coordinate exceeds native range",
                                ));
                            }
                            *to = n as i32;
                        }
                        reserve(&mut clip_rectangles)?;
                        clip_rectangles.push(ClipRectStep {
                            coords: target,
                            progress: animation.calc(1.0, 0.0)?,
                            inverse: *inverse,
                        });
                    }
                    OverrideOp::ClipDrawing {
                        path,
                        scale,
                        inverse,
                        ..
                    } => {
                        changes = changes.with_clip();
                        if clip_vector.is_none() {
                            clip_vector =
                                Some((crate::drawing::parse(path, *scale, 0, limits)?, *inverse));
                        }
                    }
                    OverrideOp::SetDecoration {
                        strikeout, value, ..
                    } => {
                        let (u, s) = (state.underline(), state.strikeout());
                        state = if *strikeout {
                            state.with_decoration(u, value.unwrap_or(base.strikeout()))
                        } else {
                            state.with_decoration(value.unwrap_or(base.underline()), s)
                        };
                    }
                    OverrideOp::SetLegacyAlignment { value, .. } => {
                        if !alignment_seen {
                            // xy uses modulo-three columns and any value >=7
                            // as top alignment, including the old a4/a8 values.
                            let value = value
                                .and_then(|v| Alignment::new(if v > 9 { v - 3 } else { v }).ok())
                                .unwrap_or(base.alignment());
                            state = state.with_alignment(value);
                            event_alignment = value;
                            alignment_seen = true;
                        }
                    }
                    OverrideOp::RestoreOriginal { field, .. } => {
                        let original = |channel| match channel {
                            PaintChannel::Primary => base.primary(),
                            PaintChannel::Secondary => base.secondary(),
                            PaintChannel::Outline => base.outline(),
                            PaintChannel::Shadow => base.shadow_color(),
                        };
                        state = match field {
                            OriginalField::Font => {
                                let f = FontRequest::new(
                                    Arc::from(base.font().family()),
                                    state.font().weight(),
                                    state.font().italic(),
                                    base.font().vertical(),
                                );
                                state.with_font(f)
                            }
                            OriginalField::Weight => {
                                let f = state.font().clone().with_weight(base.font().weight());
                                state.with_font(f)
                            }
                            OriginalField::Italic => {
                                let f = state.font().clone().with_italic(base.font().italic());
                                state.with_font(f)
                            }
                            OriginalField::Paint(channel) => channel_color(state, *channel, |c| {
                                let b = original(*channel);
                                Rgba8::new(b.r(), b.g(), b.b(), c.a())
                            }),
                            OriginalField::Opacity(Some(channel)) => {
                                channel_color(state, *channel, |c| {
                                    Rgba8::new(c.r(), c.g(), c.b(), original(*channel).a())
                                })
                            }
                            OriginalField::Opacity(None) => {
                                for c in [
                                    PaintChannel::Primary,
                                    PaintChannel::Secondary,
                                    PaintChannel::Outline,
                                    PaintChannel::Shadow,
                                ] {
                                    state = channel_color(state, c, |old| {
                                        Rgba8::new(old.r(), old.g(), old.b(), original(c).a())
                                    });
                                }
                                state
                            }
                        };
                    }
                    OverrideOp::SetDrawingMode { scale, .. } => {
                        drawing_mode = *scale;
                    }
                    OverrideOp::SetDrawingBaseline { offset, .. } => {
                        drawing_baseline = *offset;
                    }
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
                        if animation.depth != 0 {
                            unsupported(
                                event,
                                *source,
                                policy,
                                d,
                                "warpmode remains a standalone Taiyou selector",
                            )?;
                        } else {
                            warp_mode = *mode;
                            warp_mode_source = Some(*source);
                        }
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
                        state = if animation.depth == 0 {
                            state.with_size(*size).map_err(convert)?
                        } else {
                            animate_number(
                                state,
                                base,
                                TransformProperty::Size,
                                Some(*size),
                                false,
                                &animation,
                            )?
                        };
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
                        if animation.depth > 0 {
                            let property = if *axis == ScaleAxis::X {
                                TransformProperty::ScaleX
                            } else {
                                TransformProperty::ScaleY
                            };
                            let value = FiniteF64::new(factor.get() * 100.0).map_err(convert)?;
                            state = animate_number(
                                state,
                                base,
                                property,
                                Some(value),
                                false,
                                &animation,
                            )?;
                            changes = changes.union(ChangeClass::between_styles(&before, &state));
                            continue;
                        }
                        let (x, y) = match axis {
                            ScaleAxis::X => (*factor, state.scale_y()),
                            ScaleAxis::Y => (state.scale_x(), *factor),
                        };
                        state = state.with_scale(x, y).map_err(convert)?;
                    }
                    OverrideOp::SetSpacing { spacing, .. } => {
                        let value = animation.calc(spacing.get(), state.spacing().get())?;
                        state = state.with_spacing(value);
                    }
                    OverrideOp::SetBorder { width, .. } => {
                        state = if animation.depth == 0 {
                            state.with_border(*width).map_err(convert)?
                        } else {
                            animate_number(
                                state,
                                base,
                                TransformProperty::Border,
                                Some(*width),
                                false,
                                &animation,
                            )?
                        };
                    }
                    OverrideOp::SetShadow { offset, .. } => {
                        state = animate_number(
                            state,
                            base,
                            TransformProperty::Shadow,
                            Some(*offset),
                            false,
                            &animation,
                        )?;
                    }
                    OverrideOp::SetAlignment { alignment, .. } => {
                        if !alignment_seen {
                            state = state.with_alignment(*alignment);
                            event_alignment = *alignment;
                        }
                        alignment_seen = true;
                    }
                    OverrideOp::SetPosition { point, .. } => {
                        if anchor != Anchor::Default {
                            continue;
                        }
                        if anchor != Anchor::Explicit(*point) {
                            changes = changes.with_placement();
                        }
                        anchor = Anchor::Explicit(*point);
                    }
                    OverrideOp::Move {
                        from,
                        to,
                        times_ms,
                        source,
                    } => {
                        if anchor == Anchor::Default {
                            let sample = MoveSample::new(
                                *from,
                                *to,
                                move_progress(*times_ms, local_ms, duration_ms)?,
                            )?;
                            anchor =
                                Anchor::Explicit(sample.script_position().map_err(|e| {
                                    e.with_context(Some(event.id()), Some(*source))
                                })?);
                            motion = Some(sample);
                            changes = changes.with_placement().with_animation();
                        }
                    }
                    OverrideOp::SetOrigin { point, .. } => {
                        if origin.is_none() {
                            origin = Some(*point);
                            changes = changes.with_rotation_origin();
                        }
                    }
                    OverrideOp::Fade {
                        alpha, times_ms, ..
                    } => {
                        if fade.is_none() {
                            fade = Some((*alpha, *times_ms));
                            changes = changes.with_animation().with_event_paint();
                        }
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
                    OverrideOp::SetAlpha { channel, value, .. } => {
                        let channels = [
                            PaintChannel::Primary,
                            PaintChannel::Secondary,
                            PaintChannel::Outline,
                            PaintChannel::Shadow,
                        ];
                        for c in channels {
                            if channel.is_some_and(|only| only != c) {
                                continue;
                            }
                            let color = match c {
                                PaintChannel::Primary => state.primary(),
                                PaintChannel::Secondary => state.secondary(),
                                PaintChannel::Outline => state.outline(),
                                PaintChannel::Shadow => state.shadow_color(),
                            };
                            let original = match c {
                                PaintChannel::Primary => base.primary(),
                                PaintChannel::Secondary => base.secondary(),
                                PaintChannel::Outline => base.outline(),
                                PaintChannel::Shadow => base.shadow_color(),
                            };
                            let alpha = match value {
                                Some(target) => animation
                                    .calc(f64::from(*target), f64::from(255 - color.a()))?
                                    .get(),
                                None => f64::from(255 - original.a()),
                            };
                            let opacity = 255 - alpha.trunc().rem_euclid(256.0) as u8;
                            state = channel_color(state, c, |color| {
                                Rgba8::new(color.r(), color.g(), color.b(), opacity)
                            });
                        }
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
                    OverrideOp::SetFont { .. }
                        | OverrideOp::ResetStyle { .. }
                        | OverrideOp::RestoreOriginal {
                            field: OriginalField::Font,
                            ..
                        }
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
    if let Some((alpha, times)) = fade {
        let alpha = fade_alpha(alpha, times, local_ms, duration_ms);
        for run in &mut runs {
            *run = run.clone().with_event_fade(alpha);
        }
    }
    let placement = Placement::new(
        &state.clone().with_alignment(event_alignment),
        event.margins(),
        anchor,
    )
    .with_origin(origin)
    .with_motion(motion);
    let evaluated = EvaluatedEvent::new(
        event.id(),
        EventOrder::new(event.layer(), event.id().ordinal()),
        runs.into(),
        placement,
        changes,
    )
    .with_terminal_style(state)
    .with_ass_clip(AssClip {
        rectangles: clip_rectangles.into(),
        vector: clip_vector,
    })
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
/// `one` starts at Default; SetPosition/Move share the first placement slot.
/// xy also excludes explicit origins from allocation. Style resets,
/// recovery literals and unsupported operations never write it. Full evaluation
/// still runs before rendering and enforces policy/errors independently.
#[cfg(any(test, feature = "native"))]
pub(crate) fn collision_eligible(event: &Event) -> bool {
    !event.program().iter().any(|operation| {
        matches!(
            operation,
            TextOp::Override(
                OverrideOp::SetPosition { .. }
                    | OverrideOp::Move { .. }
                    | OverrideOp::SetOrigin { .. }
            )
        )
    })
}

/// Parsed drawing mode controls whether any visible word needs font services.
/// p/pbo and reset behavior are immediate even inside transforms in xy.
pub fn requires_font(event: &Event) -> bool {
    let mut mode = 0;
    for op in event.program() {
        match op {
            TextOp::Override(OverrideOp::SetDrawingMode { scale, .. }) => mode = *scale,
            TextOp::Text { text, .. } if mode == 0 && !text.is_empty() => return true,
            TextOp::HardBreak | TextOp::SoftBreak | TextOp::NoBreakSpace if mode == 0 => {
                return true;
            }
            _ => (),
        }
    }
    false
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
            TextOp::Override(OverrideOp::Move {
                from: ScriptPoint::new(FiniteF64::ZERO, FiniteF64::ZERO),
                to: ScriptPoint::new(FiniteF64::ONE, FiniteF64::ONE),
                times_ms: None,
                source: operation_span,
            }),
            TextOp::Override(OverrideOp::SetOrigin {
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
