//! Iterative Core tokenization. Legacy malformed/nesting behavior remains R001.
use crate::{Context, decode, invalid, reserve};
use std::collections::BTreeMap;
use std::sync::Arc;
use taiyou_types::*;

struct Program {
    ops: Vec<TextOp>,
    spans: Vec<Option<SourceSpan>>,
    transient: u64,
}
fn push(program: &mut Program, op: TextOp, limit: u64) -> Result<(), ParseError> {
    push_at(program, op, limit, None)
}
fn push_at(
    program: &mut Program,
    op: TextOp,
    limit: u64,
    explicit: Option<SourceSpan>,
) -> Result<(), ParseError> {
    // At most one operation per input byte, also enforced for empty input's
    // single empty literal. This derives a token bound from the existing text
    // byte limit rather than silently introducing a new independent policy cap.
    if u64::try_from(program.ops.len()).map_err(|_| invalid("token count conversion overflow"))?
        >= limit
    {
        return Err(ParseError::new(
            ErrorCode::LimitExceeded,
            "text-derived token limit exceeded",
        ));
    }
    let cost = (program.ops.len() as u64 + 1)
        .checked_mul(
            3 * (std::mem::size_of::<TextOp>() + std::mem::size_of::<Option<SourceSpan>>()) as u64,
        )
        .and_then(|n| n.checked_add(256));
    if cost.is_none_or(|n| n > program.transient) {
        return Err(ParseError::new(
            ErrorCode::LimitExceeded,
            "token provenance transient limit exceeded",
        ));
    }
    let span = match &op {
        TextOp::Text { source, .. } | TextOp::Unsupported { source, .. } => Some(*source),
        TextOp::Override(op) => Some(op.source()),
        _ => explicit,
    };
    reserve(&mut program.ops)?;
    reserve(&mut program.spans)?;
    program.ops.push(op);
    program.spans.push(span);
    Ok(())
}

pub(crate) fn tokenize(
    source: &str,
    span: SourceSpan,
    event: EventId,
    styles: &BTreeMap<Arc<str>, usize>,
    ctx: &mut Context<'_>,
) -> Result<(Vec<TextOp>, Vec<Option<SourceSpan>>), ParseError> {
    let text = span
        .slice(source)
        .ok_or_else(|| invalid("invalid tokenizer source span"))?;
    let base =
        usize::try_from(span.start()).map_err(|_| invalid("text offset conversion overflow"))?;
    let mut program = Program {
        ops: Vec::new(),
        spans: Vec::new(),
        transient: ctx.options.limits().transient_bytes(),
    };
    let mut cursor = 0;
    let mut literal_start = 0;
    let limit = ctx.options.limits().text_bytes_per_event();
    let literal = |start, end| -> Result<TextOp, ParseError> {
        Ok(TextOp::Text {
            text: Arc::from(&text[start..end]),
            source: SourceSpan::new(source, base + start, base + end)?,
        })
    };
    while cursor < text.len() {
        let byte = text.as_bytes()[cursor];
        let escape = if byte == b'\\' {
            match text.as_bytes().get(cursor + 1) {
                Some(b'N') => Some(TextOp::HardBreak),
                Some(b'n') => Some(TextOp::SoftBreak),
                Some(b'h') => Some(TextOp::NoBreakSpace),
                _ => None,
            }
        } else {
            None
        };
        if byte != b'{' && escape.is_none() {
            cursor += 1;
            continue;
        }
        // Delimiters are ASCII, hence both endpoints are UTF-8 boundaries.
        if literal_start < cursor {
            push(&mut program, literal(literal_start, cursor)?, limit)?;
        }
        if let Some(op) = escape {
            push_at(
                &mut program,
                op,
                limit,
                Some(SourceSpan::new(source, base + cursor, base + cursor + 2)?),
            )?;
            cursor += 2;
        } else {
            let start = cursor;
            let end = text[cursor + 1..]
                .find('}')
                .map(|n| cursor + 1 + n)
                .unwrap_or(text.len());
            let block_span = SourceSpan::new(
                source,
                base + start,
                base + end + usize::from(end < text.len()),
            )?;
            if end == text.len() {
                ctx.recover(
                    invalid("unterminated Core override block consumes remaining text"),
                    Some(event),
                    block_span,
                )?;
            }
            block(
                source,
                &text[cursor + 1..end],
                base + cursor + 1,
                event,
                styles,
                ctx,
                &mut program,
            )?;
            cursor = end + usize::from(end < text.len());
        }
        literal_start = cursor;
    }
    if literal_start < text.len() || text.is_empty() {
        push(&mut program, literal(literal_start, text.len())?, limit)?;
    }
    Ok((program.ops, program.spans))
}

fn block(
    source: &str,
    text: &str,
    base: usize,
    event: EventId,
    styles: &BTreeMap<Arc<str>, usize>,
    ctx: &mut Context<'_>,
    program: &mut Program,
) -> Result<(), ParseError> {
    let mut cursor = 0;
    while cursor < text.len() {
        let Some(skip) = text[cursor..].find('\\') else {
            break;
        };
        cursor += skip;
        let start = cursor;
        cursor += 1;
        let remaining = &text[cursor..];
        // Named resets may contain literal parentheses. Only a final exact style
        // match gets that unstructured treatment; unsupported r-prefixed tags
        // retain nested parameters as one operation and cannot execute children.
        let reset = remaining
            .split('\\')
            .next()
            .and_then(|raw| raw.strip_prefix('r'));
        let unstructured = remaining.starts_with("fn")
            || reset.is_some_and(|name| {
                let name = decode::trim(name);
                name.is_empty() || styles.contains_key(name)
            });
        let mut depth = 0_u32;
        while cursor < text.len() {
            match text.as_bytes()[cursor] {
                b'\\' if depth == 0 => break,
                b'(' if !unstructured => {
                    depth += 1;
                    if depth > ctx.options.limits().tag_nesting() {
                        return Err(ParseError::new(
                            ErrorCode::LimitExceeded,
                            "override nesting limit exceeded",
                        ));
                    }
                }
                b')' if !unstructured && depth > 0 => depth -= 1,
                _ => (),
            }
            cursor += 1;
        }
        let span = SourceSpan::new(source, base + start, base + cursor)?;
        let raw = &text[start + 1..cursor];
        match parse(raw, span, styles) {
            Ok(Some(op)) => {
                op.validate()
                    .map_err(|_| invalid("invalid typed override"))?;
                push(
                    program,
                    TextOp::Override(op),
                    ctx.options.limits().text_bytes_per_event(),
                )?;
            }
            Ok(None) => {
                let name = unsupported_name(raw);
                push(
                    program,
                    TextOp::Unsupported {
                        name: Arc::from(name),
                        source: span,
                    },
                    ctx.options.limits().text_bytes_per_event(),
                )?;
                ctx.report(
                    DiagnosticCode::UnsupportedFeature,
                    Some(event),
                    span,
                    "unsupported override retained without execution",
                )?;
            }
            Err(error) => ctx.recover(error, Some(event), span)?,
        }
    }
    Ok(())
}

fn hex(value: &str, digits: usize) -> Result<u32, ParseError> {
    let value = decode::trim(value);
    let value = value
        .strip_prefix("&H")
        .or_else(|| value.strip_prefix("&h"))
        .ok_or_else(|| invalid("override color/alpha requires &H hex"))?;
    let value = value.strip_suffix('&').unwrap_or(value);
    if value.is_empty() || value.len() > digits || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid("override hex channel out of range"));
    }
    u32::from_str_radix(value, 16).map_err(|_| invalid("invalid override hex"))
}
fn channel(name: &str) -> PaintChannel {
    match name.as_bytes()[0] {
        b'2' => PaintChannel::Secondary,
        b'3' => PaintChannel::Outline,
        b'4' => PaintChannel::Shadow,
        _ => PaintChannel::Primary,
    }
}
fn unsupported_name(raw: &str) -> &str {
    // Preserve the tag identity for known, deliberately unsupported static
    // extensions whose parameters are adjacent to the name.
    for name in ["xbord", "ybord", "xshad", "yshad", "be"] {
        if raw.strip_prefix(name).is_some_and(|suffix| {
            suffix.is_empty()
                || suffix
                    .as_bytes()
                    .first()
                    .is_some_and(|b| b.is_ascii_digit() || matches!(*b, b'+' | b'-' | b'.'))
        }) {
            return &raw[..name.len()];
        }
    }
    let name_end = raw
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric())
        .count();
    &raw[..name_end]
}
fn parse(
    raw: &str,
    source: SourceSpan,
    styles: &BTreeMap<Arc<str>, usize>,
) -> Result<Option<OverrideOp>, ParseError> {
    // pinterf ParseSSATag takes these values from the command suffix,
    // not parenthesized arguments. wcstod consumes a numeric prefix.
    for (name, gaussian) in [("blur", true), ("be", false)] {
        if let Some(suffix) = raw.strip_prefix(name) {
            let suffix = suffix.split_once('(').map_or(suffix, |(value, _)| value);
            let suffix = decode::trim(suffix);
            let value = if suffix.is_empty() {
                None
            } else {
                let n = blur_number(suffix)?;
                Some(FiniteF64::new(if gaussian { n.max(0.0) } else { n })?)
            };
            return Ok(Some(if gaussian {
                OverrideOp::SetBlur { value, source }
            } else {
                OverrideOp::SetBe { value, source }
            }));
        }
    }
    // pinterf's q path uses a decimal integer prefix. No digits means zero;
    // empty/out-of-domain values restore Script Info's wrapping style.
    if let Some(value) = raw.strip_prefix('q') {
        // q consumes only its command suffix; parentheses never supply its value.
        let value = value.split_once('(').map_or(value, |(suffix, _)| suffix);
        let value = value.trim_start_matches([' ', '\t', '\r', '\n']);
        let negative = value.starts_with('-');
        let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
        let mut number = 0_u32;
        for digit in digits.bytes().take_while(u8::is_ascii_digit) {
            number = number
                .saturating_mul(10)
                .saturating_add(u32::from(digit - b'0'));
        }
        let mode = if value.is_empty() || number > 3 || (negative && number != 0) {
            None
        } else {
            Some(WrapMode::new(number as u8).map_err(|_| invalid("invalid q style"))?)
        };
        return Ok(Some(OverrideOp::SetWrapMode { mode, source }));
    }
    // Longest names first; fn and exact resolved r names consume literal suffixes.
    let names = [
        "warpmode", "fscx", "fscy", "alpha", "bord", "shad", "fsp", "pos", "1c", "2c", "3c", "4c",
        "1a", "2a", "3a", "4a", "fn", "fs", "an", "b", "i", "c", "r",
    ];
    let Some(name) = names.iter().copied().find(|name| raw.starts_with(name)) else {
        return Ok(None);
    };
    let value = decode::trim(&raw[name.len()..]);
    let nonfinite = ["nan", "inf", "infinity"]
        .iter()
        .any(|v| value.eq_ignore_ascii_case(v));
    if name != "fn"
        && name != "r"
        && !nonfinite
        && value
            .as_bytes()
            .first()
            .is_some_and(|b| b.is_ascii_alphabetic())
    {
        return Ok(None);
    }
    let op = match name {
        "warpmode" => OverrideOp::SetWarpMode {
            mode: match value {
                "0" => WarpMode::XyStyle,
                "1" => WarpMode::Unicode,
                _ => return Err(invalid("warpmode requires the single digit 0 or 1")),
            },
            source,
        },
        "fn" => {
            if value.is_empty() {
                return Err(invalid("empty font override"));
            }
            OverrideOp::SetFont {
                family: Arc::from(value),
                source,
            }
        }
        "r" => {
            if !value.is_empty() && !styles.contains_key(value) {
                return Ok(None);
            }
            OverrideOp::ResetStyle {
                name: if value.is_empty() {
                    None
                } else {
                    Some(Arc::from(value))
                },
                source,
            }
        }
        "fs" => {
            let size = decode::number(value)?;
            if size.get() <= 0.0 {
                return Err(invalid("font size must be positive"));
            }
            OverrideOp::SetSize { size, source }
        }
        "b" => OverrideOp::SetWeight {
            weight: if decode::boolean(value)
                .map_err(|_| invalid("Core bold requires 0, 1 or -1"))?
            {
                700
            } else {
                400
            },
            source,
        },
        "i" => OverrideOp::SetItalic {
            italic: decode::boolean(value)
                .map_err(|_| invalid("Core italic requires 0, 1 or -1"))?,
            source,
        },
        "fscx" | "fscy" => {
            let factor = decode::number(value)?.checked_div(FiniteF64::new(100.0)?)?;
            if factor.get() < 0.0 {
                return Err(invalid("scale must be nonnegative"));
            }
            OverrideOp::SetScale {
                axis: if name == "fscx" {
                    ScaleAxis::X
                } else {
                    ScaleAxis::Y
                },
                factor,
                source,
            }
        }
        "fsp" => OverrideOp::SetSpacing {
            spacing: decode::number(value)?,
            source,
        },
        "bord" => {
            let width = decode::number(value)?;
            if width.get() < 0.0 {
                return Err(invalid("border width must be nonnegative"));
            }
            OverrideOp::SetBorder { width, source }
        }
        "shad" => OverrideOp::SetShadow {
            offset: decode::number(value)?,
            source,
        },
        "an" => OverrideOp::SetAlignment {
            alignment: Alignment::new(
                u8::try_from(decode::unsigned(value)?)
                    .map_err(|_| invalid("alignment out of range"))?,
            )
            .map_err(|_| invalid("alignment must be 1..9"))?,
            source,
        },
        "pos" => {
            let value = value
                .strip_prefix('(')
                .and_then(|v| v.strip_suffix(')'))
                .ok_or_else(|| invalid("pos requires (x,y)"))?;
            let (x, y) = value
                .split_once(',')
                .ok_or_else(|| invalid("pos requires two coordinates"))?;
            OverrideOp::SetPosition {
                point: ScriptPoint::new(decode::number(x)?, decode::number(y)?),
                source,
            }
        }
        "c" | "1c" | "2c" | "3c" | "4c" => OverrideOp::SetPaint {
            channel: channel(name),
            color: Rgba8::from_ass(hex(value, 6)?),
            source,
        },
        "alpha" | "1a" | "2a" | "3a" | "4a" => OverrideOp::SetOpacity {
            channel: if name == "alpha" {
                None
            } else {
                Some(channel(name))
            },
            opacity: 255
                - u8::try_from(hex(value, 2)?).map_err(|_| invalid("alpha out of range"))?,
            source,
        },
        _ => return Ok(None),
    };
    Ok(Some(op))
}

fn blur_number(value: &str) -> Result<f64, ParseError> {
    let b = value.as_bytes();
    let mut end = usize::from(matches!(b.first(), Some(b'+' | b'-')));
    let mut digits = 0;
    while b.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
        digits += 1;
    }
    if b.get(end) == Some(&b'.') {
        end += 1;
        while b.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        if value
            .trim_start_matches(['+', '-'])
            .to_ascii_lowercase()
            .starts_with("inf")
            || value
                .trim_start_matches(['+', '-'])
                .to_ascii_lowercase()
                .starts_with("nan")
        {
            return Err(invalid("blur strength must be finite"));
        }
        return Ok(0.0);
    }
    if matches!(b.get(end), Some(b'e' | b'E')) {
        let exponent = end;
        end += 1;
        if matches!(b.get(end), Some(b'+' | b'-')) {
            end += 1;
        }
        let start = end;
        while b.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if start == end {
            end = exponent;
        }
    }
    let number = value[..end]
        .parse::<f64>()
        .map_err(|_| invalid("invalid blur value"))?;
    if !number.is_finite() {
        return Err(invalid("blur strength must be finite"));
    }
    Ok(number)
}
