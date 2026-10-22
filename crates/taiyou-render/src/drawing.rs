//! ASS polygon words: bounded coordinates and uniform cubic B-splines.
//! Source behavior: pinterf RTS.cpp CPolygon::ParseStr / ParsePolygon and
//! Rasterizer.cpp ScanConvert. No font substitution or independent effects path.
use std::sync::Arc;
use taiyou_types::*;

fn push<T>(out: &mut Vec<T>, item: T, limits: &Limits) -> Result<(), RenderError> {
    check_path_budget(out.len().checked_add(1).ok_or_else(geometry_limit)?, limits)?;
    out.try_reserve(1).map_err(|_| {
        RenderError::new(ErrorCode::AllocationFailed, "ASS drawing allocation failed")
    })?;
    out.push(item);
    Ok(())
}
fn scalar(text: &str) -> Option<(f64, usize)> {
    let bytes = text.as_bytes();
    let mut end = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let begin = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    let mut digits = end - begin;
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        let start = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        digits += end - start;
    }
    if digits == 0 {
        return None;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let saved = end;
        end += 1;
        if matches!(bytes.get(end), Some(b'+' | b'-')) {
            end += 1;
        }
        let start = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if start == end {
            end = saved;
        }
    }
    Some((text[..end].parse().ok()?, end))
}
#[derive(Clone, Copy)]
struct Point(f64, f64);
fn point(text: &mut &str, factor: f64) -> Result<Option<Point>, RenderError> {
    let mut values = [0.0; 2];
    for value in &mut values {
        *text = text.trim_start_matches(char::is_whitespace);
        let Some((n, end)) = scalar(text) else {
            return Ok(None);
        };
        let q = n * 64.0;
        if !q.is_finite() || q < i32::MIN as f64 || q > i32::MAX as f64 {
            return Err(geometry_limit());
        }
        // CPolygon truncates into native Q6 before and after drawing scale.
        *value = (q.trunc() * factor).trunc() / 64.0;
        *text = &text[end..];
    }
    Ok(Some(Point(values[0], values[1])))
}
/// Invalid command tails are ignored like the historical polygon reader. A word
/// without an initial m produces an empty polygon, never rendered font letters.
pub fn parse(
    text: &str,
    scale: u32,
    baseline: i32,
    limits: &Limits,
) -> Result<Arc<AssDrawing>, RenderError> {
    if scale == 0 || scale > 31 {
        return Err(RenderError::new(
            ErrorCode::LimitExceeded,
            "ASS drawing scale outside defined native shift domain",
        ));
    }
    let factor = 1.0 / f64::from(1_u32 << (scale - 1));
    let mut raw: Vec<(u8, Point)> = Vec::new();
    let mut command = None;
    let mut cursor = 0;
    let mut spline_start: Option<usize> = None;
    while cursor < text.len() {
        let Some((at, ch)) = text[cursor..]
            .char_indices()
            .find(|(_, c)| matches!(c, 'm' | 'n' | 'l' | 'b' | 's' | 'p' | 'c'))
        else {
            break;
        };
        cursor += at + ch.len_utf8();
        let end = text[cursor..]
            .char_indices()
            .find(|(_, c)| matches!(c, 'm' | 'n' | 'l' | 'b' | 's' | 'p' | 'c'))
            .map_or(text.len(), |(n, _)| cursor + n);
        let mut parameters = &text[cursor..end];
        cursor = end;
        if ch == 'c' {
            if let Some(start) = spline_start.take() {
                let seed: [Point; 3] = [raw[start - 1].1, raw[start].1, raw[start + 1].1];
                for p in seed {
                    push(&mut raw, (b'p', p), limits)?;
                }
            }
            continue;
        }
        let start = raw.len();
        if raw.is_empty() && !matches!(ch, 'm' | 'n') {
            continue;
        }
        if ch == 'p' && raw.len() < 3 {
            continue;
        }
        while let Some(p) = point(&mut parameters, factor)? {
            push(&mut raw, (ch as u8, p), limits)?;
        }
        if ch == 's' {
            if raw.len() - start < 3 {
                raw.truncate(start);
                spline_start = None;
            } else {
                spline_start = Some(start);
                for item in &mut raw[start + 3..] {
                    item.0 = b'p';
                }
            }
        }
        if raw.len() > start {
            command = Some(ch);
        }
    }
    let valid = raw.first().is_some_and(|(c, _)| *c == b'm') && command.is_some();
    let mut paths = Vec::new();
    let mut bounds: Option<(f64, f64, f64, f64)> = None;
    if valid {
        for &(_, p) in &raw {
            bounds = Some(bounds.map_or((p.0, p.1, p.0, p.1), |(a, b, c, d)| {
                (a.min(p.0), b.min(p.1), c.max(p.0), d.max(p.1))
            }));
        }
    }
    let (width, height) = bounds.map_or((0.0, 0.0), |(x, y, a, b)| {
        ((a - x).max(0.0), (b - y).max(0.0))
    });
    let descent = if valid {
        (64.0 * factor * f64::from(baseline)).trunc() / 64.0
    } else {
        0.0
    };
    let ascent = height - descent;
    let font = |p: Point| FontPoint::new(p.0, ascent - p.1);
    let mut open = false;
    let mut current = None;
    let mut pending = None;
    let mut i = 0;
    while valid && i < raw.len() {
        let (cmd, p) = raw[i];
        match cmd {
            b'm' => {
                if open {
                    push(&mut paths, PathOp::Close, limits)?;
                }
                push(&mut paths, PathOp::MoveTo(font(p)?), limits)?;
                current = Some(p);
                pending = None;
                open = true;
            }
            b'n' => {
                pending = Some(p);
            }
            b'l' => {
                if let Some(p) = pending.take() {
                    push(&mut paths, PathOp::LineTo(font(p)?), limits)?;
                }
                push(&mut paths, PathOp::LineTo(font(p)?), limits)?;
                current = Some(p);
            }
            b'b' if i + 2 < raw.len() && raw[i..i + 3].iter().all(|(c, _)| *c == b'b') => {
                if let Some(p) = pending.take() {
                    push(&mut paths, PathOp::LineTo(font(p)?), limits)?;
                }
                push(
                    &mut paths,
                    PathOp::CubicTo(font(p)?, font(raw[i + 1].1)?, font(raw[i + 2].1)?),
                    limits,
                )?;
                current = Some(raw[i + 2].1);
                i += 2;
            }
            b's' | b'p' => {
                let start = if cmd == b's' {
                    i.checked_sub(1)
                } else {
                    i.checked_sub(3)
                };
                let stop = if cmd == b's' { i + 2 } else { i };
                if let Some(start) = start.filter(|s| *s + 3 < raw.len() && stop < raw.len()) {
                    let q = [
                        raw[start].1,
                        raw[start + 1].1,
                        raw[start + 2].1,
                        raw[start + 3].1,
                    ];
                    let blend = |a: Point, wa: f64, b: Point, wb: f64, c: Point, wc: f64| {
                        Point(
                            (a.0 * wa + b.0 * wb + c.0 * wc) / 6.0,
                            (a.1 * wa + b.1 * wb + c.1 * wc) / 6.0,
                        )
                    };
                    let a = blend(q[0], 1.0, q[1], 4.0, q[2], 1.0);
                    let b = blend(q[1], 4.0, q[2], 2.0, q[0], 0.0);
                    let c = blend(q[1], 2.0, q[2], 4.0, q[0], 0.0);
                    let d = blend(q[1], 1.0, q[2], 4.0, q[3], 1.0);
                    if current.is_none_or(|p: Point| p.0 != a.0 || p.1 != a.1) {
                        push(&mut paths, PathOp::LineTo(font(a)?), limits)?;
                    }
                    push(
                        &mut paths,
                        PathOp::CubicTo(font(b)?, font(c)?, font(d)?),
                        limits,
                    )?;
                    current = Some(d);
                    pending = None;
                    if cmd == b's' {
                        i += 2;
                    }
                }
            }
            _ => (),
        }
        i += 1;
    }
    if open {
        push(&mut paths, PathOp::Close, limits)?;
    }
    Ok(Arc::new(AssDrawing {
        outline: Arc::new(Outline::new(paths, limits)?),
        width: FiniteF64::new(width).map_err(|_| geometry_limit())?,
        ascent: FiniteF64::new(ascent).map_err(|_| geometry_limit())?,
        descent: FiniteF64::new(descent).map_err(|_| geometry_limit())?,
    }))
}
