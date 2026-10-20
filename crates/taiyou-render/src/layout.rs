//! Core local layout: explicit breaks or source-derived xy word wrapping.
use std::sync::Arc;
use taiyou_types::*;
pub use taiyou_types::{LayoutConstraints, LocalLayout};

fn error(code: ErrorCode, text: &str) -> RenderError {
    RenderError::new(code, text)
}
fn limited() -> RenderError {
    error(
        ErrorCode::LimitExceeded,
        "layout geometry/reservation exceeded",
    )
}
fn finite(v: f64) -> Result<FiniteF64, RenderError> {
    FiniteF64::new(v).map_err(|_| limited())
}
fn reserve<T>(v: &mut Vec<T>, count: usize) -> Result<(), RenderError> {
    v.try_reserve_exact(count)
        .map_err(|_| error(ErrorCode::AllocationFailed, "layout allocation failed"))
}
fn check(b: ScriptBounds, limits: &Limits) -> Result<(), RenderError> {
    let cap = f64::from(limits.max_dimension());
    if [
        b.x0().get(),
        b.y0().get(),
        b.x1().get(),
        b.y1().get(),
        b.width(),
        b.height(),
    ]
    .iter()
    .any(|v| !v.is_finite() || v.abs() > cap)
    {
        return Err(limited());
    }
    Ok(())
}
fn include(target: &mut Option<ScriptBounds>, b: ScriptBounds) -> Result<(), RenderError> {
    if b.width() == 0.0 || b.height() == 0.0 {
        return Ok(());
    }
    *target = Some(match *target {
        Some(a) => a.union(b)?,
        None => b,
    });
    Ok(())
}
fn association(input: &ShapedEvent, at: u32) -> Result<usize, RenderError> {
    let runs = input.evaluated().runs();
    let i = runs.partition_point(|r| r.decoded().is_some_and(|s| s.end() <= at));
    if runs
        .get(i)
        .and_then(|r| r.decoded())
        .is_none_or(|s| at < s.start() || at >= s.end())
    {
        return Err(error(
            ErrorCode::InvalidArgument,
            "missing evaluated glyph association",
        ));
    }
    Ok(i)
}
fn visual_order(order: &mut [usize], input: &ShapedEvent) -> Result<(), RenderError> {
    let level = |i: usize| input.runs()[i].bidi_level();
    let max = order.iter().map(|&i| level(i)).max().unwrap_or(0);
    if max > 126 {
        return Err(error(
            ErrorCode::InvalidArgument,
            "invalid bidi embedding level",
        ));
    }
    let Some(min) = order.iter().map(|&i| level(i)).filter(|l| l % 2 == 1).min() else {
        return Ok(());
    };
    // UAX #9 L2 applies to run groups after L1 and hard-line delimitation.
    // Each group's glyph array already has HarfBuzz native order.
    for threshold in (min..=max).rev() {
        let mut i = 0;
        while i < order.len() {
            if level(order[i]) < threshold {
                i += 1;
                continue;
            }
            let start = i;
            while i < order.len() && level(order[i]) >= threshold {
                i += 1;
            }
            order[start..i].reverse();
        }
    }
    Ok(())
}
struct DraftGlyph {
    run: usize,
    glyph: usize,
    association: usize,
    x: f64,
    y: f64,
    ink: Option<ScriptBounds>,
}
struct DraftLine {
    decoded: DecodedSpan,
    ascent: f64,
    descent: f64,
    min: f64,
    max: f64,
    glyphs: std::ops::Range<usize>,
}

fn xy_plan(
    input: &ShapedEvent,
    constraints: &LayoutConstraints,
) -> Result<Vec<crate::xy_wrap::Span>, RenderError> {
    use crate::xy_wrap::{Word, width_units};
    let text = input.source_map().text();
    let maximum = text
        .len()
        .checked_mul(2)
        .and_then(|n| n.checked_add(input.evaluated().runs().len().checked_mul(2)?))
        .and_then(|n| n.checked_add(2))
        .ok_or_else(limited)?;
    let charge = maximum
        .checked_mul(
            3 * (std::mem::size_of::<Word>()
                + 2 * std::mem::size_of::<f64>()
                + std::mem::size_of::<crate::xy_wrap::Span>()),
        )
        .ok_or_else(limited)?;
    if charge as u64 > constraints.limits().transient_bytes() {
        return Err(limited().with_resource_failure(ResourceFailure::TransientMemory));
    }
    let mut points = Vec::new();
    reserve(&mut points, maximum)?;
    points.push(0_u32);
    points.push(text.len() as u32);
    for (at, ch) in text.char_indices() {
        if ch == ' ' {
            points.push(at as u32);
            points.push(at as u32 + 1);
        }
    }
    for run in input.evaluated().runs() {
        if let Some(span) = run.decoded() {
            points.extend([span.start(), span.end()]);
        }
    }
    for span in input.breaks() {
        points.extend([span.start(), span.end()]);
    }
    points.sort_unstable();
    points.dedup();
    // CRLF and other retained hard-break ranges remain one separator.
    points.retain(|&at| {
        let i = input.breaks().partition_point(|b| b.end() <= at);
        input.breaks().get(i).is_none_or(|b| at <= b.start())
    });
    let mut widths = Vec::new();
    reserve(&mut widths, points.len())?;
    widths.resize(points.len(), 0.0_f64);
    for run in input.runs() {
        for (g, glyph) in run.glyphs().iter().enumerate() {
            let i = points
                .partition_point(|&p| p <= glyph.cluster().start())
                .checked_sub(1)
                .ok_or_else(limited)?;
            let state = &input.evaluated().runs()[association(input, glyph.cluster().start())?];
            let scale = state.geometry().scale_x().get() * constraints.scale_x().get();
            widths[i] += glyph.advance_x().get() * scale;
            if run
                .glyphs()
                .get(g + 1)
                .is_none_or(|next| next.cluster() != glyph.cluster())
            {
                widths[i] += state.shaping().spacing().get() * scale;
            }
            finite(widths[i])?;
        }
    }
    let mut words = Vec::new();
    reserve(&mut words, points.len())?;
    for (i, p) in points.windows(2).enumerate() {
        let raw = &text[p[0] as usize..p[1] as usize];
        let hard = input
            .breaks()
            .binary_search_by_key(&p[0], |b| b.start())
            .is_ok();
        words.push(Word {
            start: p[0],
            end: p[1],
            width: width_units(widths[i])?,
            space: raw == " ",
            hard,
        });
    }
    let available = constraints.available_width().get() * 8.0;
    if !available.is_finite() || available.abs() >= i64::MAX as f64 {
        return Err(limited());
    }
    crate::xy_wrap::lines(
        &words,
        text.len() as u32,
        constraints.wrap().get(),
        available.trunc() as i64,
    )
}

pub fn layout(
    input: &ShapedEvent,
    constraints: &LayoutConstraints,
) -> Result<LocalLayout, RenderError> {
    if constraints.profile() != Profile::CoreV1
        || constraints.wrap() != input.evaluated().wrap_mode()
    {
        return Err(error(
            ErrorCode::UnsupportedFeature,
            "layout profile or evaluated wrapping policy mismatch",
        ));
    }
    let limits = constraints.limits();
    let automatic = constraints.wrap().get() != 2;
    if automatic && input.evaluated().warp_mode() == WarpMode::Unicode {
        return Err(error(
            ErrorCode::UnsupportedFeature,
            "warpmode1 automatic wrapping is not admitted",
        ));
    }
    let plan = if automatic {
        Some(xy_plan(input, constraints)?)
    } else {
        None
    };
    let count = input.runs().iter().try_fold(0_usize, |n, r| {
        n.checked_add(r.glyphs().len()).ok_or_else(limited)
    })?;
    let lines = plan.as_ref().map_or_else(
        || {
            let terminal_break = input.breaks().last().is_some_and(|b| {
                b.end() as usize == input.source_map().text().len()
            });
            input.breaks().len().checked_add(usize::from(!terminal_break)).ok_or_else(limited)
        },
        |p| Ok(p.len()),
    )?;
    let bytes = count
        .checked_mul(
            3 * (std::mem::size_of::<DraftGlyph>() + std::mem::size_of::<PositionedGlyph>()),
        )
        .and_then(|n| {
            n.checked_add(lines.checked_mul(
                3 * (std::mem::size_of::<DraftLine>()
                    + std::mem::size_of::<Line>()
                    + std::mem::size_of::<crate::xy_wrap::Span>()),
            )?)
        })
        .and_then(|n| {
            n.checked_add(
                input
                    .runs()
                    .len()
                    .checked_mul(3 * std::mem::size_of::<usize>())?,
            )
        })
        .and_then(|n| {
            n.checked_add(
                input
                    .evaluated()
                    .runs()
                    .len()
                    .checked_mul(3 * std::mem::size_of::<u32>())?,
            )
        })
        .and_then(|n| n.checked_add(4096))
        .ok_or_else(limited)?;
    if count as u64 > limits.glyphs_per_event()
        || input.source_map().text().len() as u64 > limits.text_bytes_per_event()
        || constraints.available_width().get() > f64::from(limits.max_dimension())
    {
        return Err(limited());
    }
    if bytes as u64 > limits.transient_bytes() {
        return Err(limited().with_resource_failure(ResourceFailure::TransientMemory));
    }
    let mut drafts = Vec::new();
    reserve(&mut drafts, count)?;
    let mut line_drafts = Vec::new();
    reserve(&mut line_drafts, lines)?;
    let mut order = Vec::new();
    reserve(&mut order, input.runs().len())?;
    let mut changes = Vec::new();
    reserve(&mut changes, input.evaluated().runs().len())?;
    for pair in input.evaluated().runs().windows(2) {
        if pair[0].geometry().scale_x() != pair[1].geometry().scale_x()
            || pair[0].geometry().scale_y() != pair[1].geometry().scale_y()
        {
            changes.push(
                pair[1]
                    .decoded()
                    .ok_or_else(|| error(ErrorCode::InvalidArgument, "missing geometry span"))?
                    .start(),
            );
        }
    }
    let mut run_cursor = 0;
    let mut start = 0;
    let mut total_height = 0.0;
    for i in 0..lines {
        let proposed = plan.as_ref().map(|p| p[i]);
        if let Some(p) = proposed {
            start = p.start;
        }
        let end = proposed.map_or_else(
            || {
                input
                    .breaks()
                    .get(i)
                    .map_or(input.source_map().text().len() as u32, |s| s.start())
            },
            |p| p.end,
        );
        let visible_start = proposed.map_or(start, |p| p.visible_start);
        let visible_end = proposed.map_or(end, |p| p.visible_end);
        let decoded = input
            .source_map()
            .decoded_span(start..end)
            .map_err(|e| error(e.code(), e.detail()))?;
        order.clear();
        while run_cursor < input.runs().len() && input.runs()[run_cursor].decoded().start() < end {
            let run = &input.runs()[run_cursor];
            if automatic
                && (run.decoded().end() <= visible_start || run.decoded().start() >= visible_end)
            {
                run_cursor += 1;
                continue;
            }
            if run.decoded().start() < start || run.decoded().end() > end {
                return Err(error(
                    ErrorCode::InvalidArgument,
                    "shaped run crosses a hard break",
                ));
            }
            order.push(run_cursor);
            run_cursor += 1;
        }
        visual_order(&mut order, input)?;
        let (mut ascent, mut descent, mut pen_x, mut pen_y, mut min, mut max) =
            (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
        let begin = drafts.len();
        if visible_start == visible_end {
            let metrics = input
                .empty_lines()
                .binary_search_by_key(&start, |m| m.decoded.start())
                .ok()
                .map(|i| &input.empty_lines()[i]);
            if let Some(metrics) = metrics {
                let sy = metrics.scale_y.get() * constraints.scale_y().get();
                ascent = metrics.metrics.ascent().get() * sy;
                descent = metrics.metrics.descent().get() * sy;
            } else if automatic {
                let run = input
                    .runs()
                    .iter()
                    .find(|r| r.decoded().start() >= start && r.decoded().start() < end)
                    .ok_or_else(limited)?;
                let state = &input.evaluated().runs()[association(input, run.decoded().start())?];
                let sy = state.geometry().scale_y().get() * constraints.scale_y().get();
                ascent = run.metrics().ascent().get() * sy;
                descent = run.metrics().descent().get() * sy;
            } else {
                return Err(error(
                    ErrorCode::InvalidArgument,
                    "empty line lacks actual font metrics",
                ));
            }
        }
        for &r in &order {
            let run = &input.runs()[r];
            let mut group = 0;
            if run.glyphs().is_empty() {
                let a = association(input, run.decoded().start())?;
                let sy = input.evaluated().runs()[a].geometry().scale_y().get()
                    * constraints.scale_y().get();
                ascent = ascent.max(run.metrics().ascent().get() * sy);
                descent = descent.max(run.metrics().descent().get() * sy);
            }
            for (g, glyph) in run.glyphs().iter().enumerate() {
                let a = association(input, glyph.cluster().start())?;
                let state = &input.evaluated().runs()[a];
                // A cluster cannot express two different scale transforms. Report
                // that seam explicitly rather than inventing a character offset.
                if changes
                    .get(changes.partition_point(|&p| p <= glyph.cluster().start()))
                    .is_some_and(|&p| p < glyph.cluster().end())
                {
                    return Err(error(
                        ErrorCode::UnsupportedFeature,
                        "scale boundary inside shaped cluster",
                    ));
                }
                let sx = state.geometry().scale_x().get() * constraints.scale_x().get();
                let sy = state.geometry().scale_y().get() * constraints.scale_y().get();
                while run
                    .face_metrics()
                    .get(group)
                    .is_some_and(|m| m.glyphs.end <= g)
                {
                    group += 1;
                }
                let metrics = run
                    .face_metrics()
                    .get(group)
                    .map_or(run.metrics(), |m| m.metrics);
                ascent = ascent.max(metrics.ascent().get() * sy - pen_y);
                descent = descent.max(metrics.descent().get() * sy + pen_y);
                let x = pen_x + glyph.offset_x().get() * sx;
                let y = pen_y + glyph.offset_y().get() * sy;
                finite(x)?;
                finite(y)?;
                let ink = match glyph.ink() {
                    GlyphInk::Unknown => {
                        return Err(error(
                            ErrorCode::UnsupportedFeature,
                            "glyph ink extents unavailable",
                        ));
                    }
                    GlyphInk::Empty => None,
                    GlyphInk::Bounds(b) => Some(b.scaled(sx, sy)?.translated(x, y)?),
                };
                if let Some(b) = ink {
                    check(b, limits)?;
                }
                drafts.push(DraftGlyph {
                    run: r,
                    glyph: g,
                    association: a,
                    x,
                    y,
                    ink,
                });
                pen_x += glyph.advance_x().get() * sx;
                pen_y += glyph.advance_y().get() * sy;
                // Core spacing is one contribution per cluster advance, including
                // the final cluster. Marks in a multi-glyph cluster add no spacing.
                if run
                    .glyphs()
                    .get(g + 1)
                    .is_none_or(|next| next.cluster() != glyph.cluster())
                {
                    pen_x += state.shaping().spacing().get() * sx;
                }
                finite(pen_x)?;
                finite(pen_y)?;
                min = min.min(pen_x);
                max = max.max(pen_x);
            }
        }
        // GetNextLine halves the integer eighth-pixel metrics only when
        // the next word is itself a hard break. Compacted spaces are not an
        // empty line. MakeLines creates no extra line after a terminal break.
        if start == end && input.breaks().binary_search_by_key(&end, |b| b.start()).is_ok() {
            let half = |v: f64| finite(((v * 8.0).trunc() * 0.5).floor() / 8.0);
            ascent = half(ascent)?.get();
            descent = half(descent)?.get();
        }
        finite(ascent)?;
        finite(descent)?;
        total_height += ascent + descent;
        finite(total_height)?;
        line_drafts.push(DraftLine {
            decoded,
            ascent,
            descent,
            min,
            max,
            glyphs: begin..drafts.len(),
        });
        if let Some(b) = input.breaks().get(i).filter(|_| !automatic) {
            if !matches!(b.text(), "\n" | "\r" | "\r\n" | "\u{2029}") {
                return Err(error(
                    ErrorCode::UnsupportedFeature,
                    "unsupported hard-line separator",
                ));
            }
            start = b.end();
        }
    }
    if run_cursor != input.runs().len() {
        return Err(error(ErrorCode::InvalidArgument, "unconsumed shaped runs"));
    }
    let alignment = constraints.alignment().get() - 1;
    let horizontal = f64::from(alignment % 3) * 0.5;
    let vertical = 1.0 - f64::from(alignment / 3) * 0.5;
    let mut top = -total_height * vertical;
    let mut positioned = Vec::new();
    reserve(&mut positioned, count)?;
    let mut output_lines = Vec::new();
    reserve(&mut output_lines, lines)?;
    let mut logical: Option<ScriptBounds> = None;
    let mut ink = None;
    for line in line_drafts {
        let width = line.max - line.min;
        let origin = -line.min - width * horizontal;
        let baseline = top + line.ascent;
        let bounds = ScriptBounds::new(
            -width * horizontal,
            top,
            width * (1.0 - horizontal),
            top + line.ascent + line.descent,
        )?;
        check(bounds, limits)?;
        let mut line_ink = None;
        for g in &drafts[line.glyphs.clone()] {
            let p = ScriptPoint::new(finite(g.x + origin)?, finite(g.y + baseline)?);
            check(
                ScriptBounds::new(p.x().get(), p.y().get(), p.x().get(), p.y().get())?,
                limits,
            )?;
            if let Some(b) = g.ink {
                let b = b.translated(origin, baseline)?;
                check(b, limits)?;
                include(&mut line_ink, b)?;
            }
            let glyph = &input.runs()[g.run].glyphs()[g.glyph];
            positioned.push(PositionedGlyph::new(
                g.run,
                g.glyph,
                p,
                glyph.cluster().clone(),
                g.association,
            ));
        }
        if let Some(b) = line_ink {
            include(&mut ink, b)?;
        }
        logical = Some(match logical {
            Some(b) => b.union(bounds)?,
            None => bounds,
        });
        output_lines.push(Line::new(
            line.decoded,
            ScriptPoint::new(finite(origin)?, finite(baseline)?),
            (finite(line.ascent)?, finite(line.descent)?, finite(width)?),
            bounds,
            line_ink,
            line.glyphs,
        )?);
        top += line.ascent + line.descent;
    }
    let logical = logical.ok_or_else(|| error(ErrorCode::InvalidArgument, "missing local line"))?;
    let collision = match ink {
        Some(b) => logical.union(b)?,
        None => logical,
    };
    check(collision, limits)?;
    LocalLayout::new_with_scale(
        Arc::new(input.clone()),
        output_lines.into(),
        positioned.into(),
        logical,
        ink,
        collision,
        (constraints.scale_x(), constraints.scale_y()),
    )
}
