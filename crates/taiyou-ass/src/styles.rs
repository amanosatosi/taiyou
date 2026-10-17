use crate::{decode, invalid, sections::Format};
use std::sync::Arc;
use taiyou_types::*;

pub(crate) fn parse(value: &str, format: &Format) -> Result<Style, ParseError> {
    format.validate_record(value)?;
    let name = format
        .field(value, "name")
        .ok_or_else(|| invalid("Style requires Name field"))?
        .0;
    let mut style = Style::core_default().with_name(Arc::from(decode::trim(name)));
    let get = |name| format.field(value, name).map(|v| decode::trim(v.0));
    let mut family: Arc<str> = Arc::from(style.font().family());
    let mut weight = 400;
    let mut italic = false;
    if let Some(v) = get("fontname") {
        family = Arc::from(v);
    }
    if let Some(v) = get("bold") {
        weight = if decode::boolean(v)? { 700 } else { 400 };
    }
    if let Some(v) = get("italic") {
        italic = decode::boolean(v)?;
    }
    style = style.with_font(FontRequest::new(family, weight, italic, false));
    if let Some(v) = get("fontsize") {
        style = style
            .with_size(decode::number(v)?)
            .map_err(|_| invalid("style font size must be positive"))?;
    }
    let mut colors = [
        style.primary(),
        style.secondary(),
        style.outline(),
        style.shadow_color(),
    ];
    for (index, name) in [
        "primarycolour",
        "secondarycolour",
        "outlinecolour",
        "backcolour",
    ]
    .iter()
    .enumerate()
    {
        if let Some(v) = get(name) {
            colors[index] = decode::color(v)?;
        }
    }
    style = style.with_colors(colors[0], colors[1], colors[2], colors[3]);
    let underline = get("underline")
        .map(decode::boolean)
        .transpose()?
        .unwrap_or(false);
    let strikeout = get("strikeout")
        .map(decode::boolean)
        .transpose()?
        .unwrap_or(false);
    style = style.with_decoration(underline, strikeout);
    let scale = |name| {
        get(name)
            .map(|v| {
                decode::number(v)?.checked_div(
                    FiniteF64::new(100.0).map_err(|_| invalid("invalid scale divisor"))?,
                )
            })
            .transpose()
    };
    style = style
        .with_scale(
            scale("scalex")?.unwrap_or(FiniteF64::ONE),
            scale("scaley")?.unwrap_or(FiniteF64::ONE),
        )
        .map_err(|_| invalid("style scales must be nonnegative"))?;
    if let Some(v) = get("spacing") {
        style = style.with_spacing(decode::number(v)?);
    }
    if let Some(v) = get("angle") {
        style = style.with_angle(decode::number(v)?);
    }
    if let Some(v) = get("outline") {
        style = style
            .with_border(decode::number(v)?)
            .map_err(|_| invalid("style border must be nonnegative"))?;
    }
    if let Some(v) = get("shadow") {
        style = style.with_shadow(decode::number(v)?);
    }
    if let Some(v) = get("borderstyle") {
        style = style.with_border_style(match decode::integer(v)? {
            1 => BorderStyle::Outline,
            3 => BorderStyle::OpaqueBox,
            _ => return Err(crate::unsupported("unsupported BorderStyle value")),
        });
    }
    if let Some(v) = get("alignment") {
        style = style.with_alignment(
            Alignment::new(
                u8::try_from(decode::unsigned(v)?)
                    .map_err(|_| invalid("alignment out of range"))?,
            )
            .map_err(|_| invalid("alignment must be 1..9"))?,
        );
    }
    let margin = |name| {
        get(name)
            .map(decode::unsigned)
            .transpose()
            .map(|v| v.unwrap_or(10))
    };
    style = style.with_margins(Margins::new(
        margin("marginl")?,
        margin("marginr")?,
        margin("marginv")?,
    ));
    if let Some(v) = get("encoding") {
        style = style.with_encoding(decode::integer(v)?);
    }
    Ok(style)
}
