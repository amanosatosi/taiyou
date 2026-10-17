use crate::{invalid, unsupported};
use taiyou_types::{FiniteF64, ParseError, Rgba8, TimeUs};

pub(crate) fn trim(value: &str) -> &str {
    value.trim_matches([' ', '\t'])
}
pub(crate) fn utf8(bytes: &[u8]) -> Result<&str, ParseError> {
    std::str::from_utf8(bytes).map_err(|_| {
        taiyou_types::ParseError::new(
            taiyou_types::ErrorCode::InvalidEncoding,
            "subtitle input is not UTF-8",
        )
    })
}
pub(crate) fn integer(value: &str) -> Result<i32, ParseError> {
    let value = trim(value);
    let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid("expected signed decimal integer"));
    }
    value.parse().map_err(|_| invalid("integer out of range"))
}
pub(crate) fn unsigned(value: &str) -> Result<u32, ParseError> {
    let value = trim(value).strip_prefix('+').unwrap_or(trim(value));
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid("expected nonnegative decimal integer"));
    }
    value
        .parse()
        .map_err(|_| invalid("unsigned integer out of range"))
}
pub(crate) fn number(value: &str) -> Result<FiniteF64, ParseError> {
    let value = trim(value);
    let digits = value.strip_prefix(['+', '-']).unwrap_or(value);
    let mut parts = digits.split('.');
    let whole = parts.next().unwrap_or("");
    let fraction = parts.next();
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || parts.next().is_some()
        || fraction.is_some_and(|v| v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(invalid("expected Core decimal number without exponent"));
    }
    let value = value
        .parse::<f64>()
        .map_err(|_| invalid("invalid decimal number"))?;
    FiniteF64::new(value).map_err(|_| invalid("decimal number must be finite"))
}
pub(crate) fn boolean(value: &str) -> Result<bool, ParseError> {
    match integer(value)? {
        0 => Ok(false),
        1 | -1 => Ok(true),
        _ => Err(unsupported("boolean form outside Core subset")),
    }
}
pub(crate) fn color(value: &str) -> Result<Rgba8, ParseError> {
    let value = trim(value);
    let hex = value
        .strip_prefix("&H")
        .or_else(|| value.strip_prefix("&h"))
        .ok_or_else(|| unsupported("decimal style colors require the legacy decoding gate"))?;
    let hex = hex.strip_suffix('&').unwrap_or(hex);
    if hex.is_empty() || hex.len() > 8 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(invalid("style color requires 1..8 hex digits"));
    }
    u32::from_str_radix(hex, 16)
        .map(Rgba8::from_ass)
        .map_err(|_| invalid("invalid style color"))
}
pub(crate) fn timestamp(value: &str) -> Result<TimeUs, ParseError> {
    let value = trim(value);
    let negative = value.starts_with('-');
    let value = value.strip_prefix(['+', '-']).unwrap_or(value);
    let mut parts = value.split(':');
    let hours = parts.next().unwrap_or("");
    let minutes = parts
        .next()
        .ok_or_else(|| invalid("timestamp requires h:mm:ss.cc"))?;
    let seconds = parts
        .next()
        .ok_or_else(|| invalid("timestamp requires h:mm:ss.cc"))?;
    if parts.next().is_some()
        || hours.is_empty()
        || !hours.bytes().all(|b| b.is_ascii_digit())
        || minutes.len() != 2
        || !minutes.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(invalid("timestamp requires h:mm:ss.cc"));
    }
    let (seconds, centis) = seconds
        .split_once('.')
        .ok_or_else(|| invalid("timestamp requires centiseconds"))?;
    if seconds.len() != 2
        || centis.len() != 2
        || !seconds
            .bytes()
            .chain(centis.bytes())
            .all(|b| b.is_ascii_digit())
    {
        return Err(invalid(
            "timestamp seconds/centiseconds must have two digits",
        ));
    }
    let time = TimeUs::from_ass_components(
        hours
            .parse()
            .map_err(|_| invalid("timestamp hour overflow"))?,
        minutes
            .parse()
            .map_err(|_| invalid("invalid timestamp minutes"))?,
        seconds
            .parse()
            .map_err(|_| invalid("invalid timestamp seconds"))?,
        centis
            .parse()
            .map_err(|_| invalid("invalid centiseconds"))?,
    )
    .map_err(|_| invalid("timestamp component or arithmetic overflow"))?;
    if negative {
        TimeUs::from_micros(0)
            .checked_sub(time)
            .map_err(|_| invalid("timestamp negation overflow"))
    } else {
        Ok(time)
    }
}
