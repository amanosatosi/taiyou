use crate::{Context, decode, invalid, reserve, unsupported};
use std::sync::Arc;
use taiyou_types::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Section {
    Script,
    Styles,
    Events,
    Ignored,
}
pub(crate) fn section(text: &str) -> Result<Section, ParseError> {
    let text = decode::trim(text);
    let name = text
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .ok_or_else(|| invalid("malformed section declaration"))?;
    Ok(if name.eq_ignore_ascii_case("Script Info") {
        Section::Script
    } else if name.eq_ignore_ascii_case("V4+ Styles") {
        Section::Styles
    } else if name.eq_ignore_ascii_case("Events") {
        Section::Events
    } else {
        Section::Ignored
    })
}

pub(crate) struct Line<'a> {
    pub text: &'a str,
    pub start: usize,
}
pub(crate) fn lines(source: &str) -> impl Iterator<Item = Line<'_>> {
    let mut cursor = if source.starts_with('\u{feff}') { 3 } else { 0 };
    std::iter::from_fn(move || {
        if cursor >= source.len() {
            return None;
        }
        let start = cursor;
        let rest = &source[start..];
        let length = rest
            .bytes()
            .position(|b| b == b'\r' || b == b'\n')
            .unwrap_or(rest.len());
        cursor += length;
        let text = &source[start..cursor];
        if source.as_bytes().get(cursor) == Some(&b'\r') {
            cursor += 1;
        }
        if source.as_bytes().get(cursor) == Some(&b'\n') {
            cursor += 1;
        }
        Some(Line { text, start })
    })
}

const STYLE_FIELDS: &[&str] = &[
    "name",
    "fontname",
    "fontsize",
    "primarycolour",
    "secondarycolour",
    "outlinecolour",
    "backcolour",
    "bold",
    "italic",
    "underline",
    "strikeout",
    "scalex",
    "scaley",
    "spacing",
    "angle",
    "borderstyle",
    "outline",
    "shadow",
    "alignment",
    "marginl",
    "marginr",
    "marginv",
    "encoding",
];
const EVENT_FIELDS: &[&str] = &[
    "layer", "start", "end", "style", "name", "marginl", "marginr", "marginv", "effect", "text",
];

/// Stores indices only for known fields; unknown Format columns cannot cause a
/// per-column allocation. Field extraction scans the bounded input record.
pub(crate) struct Format {
    count: usize,
    slots: Vec<(&'static str, usize)>,
    events: bool,
}
impl Format {
    pub fn parse(value: &str, events: bool) -> Result<Self, ParseError> {
        let known = if events { EVENT_FIELDS } else { STYLE_FIELDS };
        let mut result = Self {
            count: 0,
            slots: Vec::new(),
            events,
        };
        for (index, name) in value.split(',').enumerate() {
            let name = decode::trim(name);
            if name.is_empty() {
                return Err(invalid("empty Format field"));
            }
            if let Some(&field) = known.iter().find(|field| field.eq_ignore_ascii_case(name)) {
                if result.slots.iter().any(|(key, _)| *key == field) {
                    return Err(invalid("duplicate known Format field"));
                }
                reserve(&mut result.slots)?;
                result.slots.push((field, index));
            }
            result.count += 1;
        }
        for name in if events {
            &["start", "end", "text"][..]
        } else {
            &["name"][..]
        } {
            if result.index(name).is_none() {
                return Err(invalid("Format lacks required field"));
            }
        }
        if events && result.index("text") != result.count.checked_sub(1) {
            return Err(unsupported(
                "Core requires Text to be the last Format field",
            ));
        }
        Ok(result)
    }
    fn index(&self, name: &str) -> Option<usize> {
        self.slots.iter().find(|(key, _)| *key == name).map(|v| v.1)
    }
    pub fn validate_record(&self, value: &str) -> Result<(), ParseError> {
        let count = if self.events {
            value.splitn(self.count, ',').count()
        } else {
            value.split(',').count()
        };
        if count != self.count {
            return Err(invalid("record field count differs from Format"));
        }
        Ok(())
    }
    pub fn field<'a>(&self, value: &'a str, name: &str) -> Option<(&'a str, usize)> {
        let index = self.index(name)?;
        let mut offset = 0;
        for (ordinal, field) in value.splitn(self.count, ',').enumerate() {
            if ordinal == index {
                return Some((field, offset));
            }
            offset += field.len() + 1;
        }
        None
    }
}

#[derive(Default)]
pub(crate) struct Header {
    play_x: Option<u32>,
    play_y: Option<u32>,
    layout_x: Option<u32>,
    layout_y: Option<u32>,
    wrap: Option<WrapMode>,
    scaled: bool,
    matrix: Option<Arc<str>>,
    collisions: Option<Arc<str>>,
    metadata: Vec<(Arc<str>, Arc<str>)>,
}
impl Header {
    pub fn record(
        &mut self,
        key: &str,
        value: &str,
        ctx: &mut Context<'_>,
        span: SourceSpan,
    ) -> Result<(), ParseError> {
        let value = decode::trim(value);
        if ["playresx", "playresy", "layoutresx", "layoutresy"]
            .iter()
            .any(|name| key.eq_ignore_ascii_case(name))
        {
            let dimension =
                decode::unsigned(value).map_err(|e| e.with_context(None, Some(span)))?;
            if dimension == 0 {
                return Err(invalid("specified script resolution must be positive")
                    .with_context(None, Some(span)));
            }
            if key.eq_ignore_ascii_case("playresx") {
                self.play_x = Some(dimension);
            } else if key.eq_ignore_ascii_case("playresy") {
                self.play_y = Some(dimension);
            } else if key.eq_ignore_ascii_case("layoutresx") {
                self.layout_x = Some(dimension);
            } else {
                self.layout_y = Some(dimension);
            }
        } else if key.eq_ignore_ascii_case("wrapstyle") {
            let result = decode::unsigned(value)
                .and_then(|v| u8::try_from(v).map_err(|_| invalid("WrapStyle out of range")))
                .and_then(|v| WrapMode::new(v).map_err(|_| invalid("WrapStyle must be 0..3")));
            match result {
                Ok(value) => self.wrap = Some(value),
                Err(error) => ctx.recover(error, None, span)?,
            }
        } else if key.eq_ignore_ascii_case("scaledborderandshadow") {
            if value.eq_ignore_ascii_case("yes") {
                self.scaled = true;
            } else if value.eq_ignore_ascii_case("no") {
                self.scaled = false;
            } else {
                ctx.recover(invalid("ScaledBorderAndShadow requires yes/no"), None, span)?;
            }
        } else if key.eq_ignore_ascii_case("ycbcr matrix") {
            self.matrix = Some(Arc::from(value));
        } else if key.eq_ignore_ascii_case("collisions") {
            self.collisions = Some(Arc::from(value));
        } else {
            reserve(&mut self.metadata)?;
            self.metadata.push((Arc::from(key), Arc::from(value)));
        }
        Ok(())
    }
    pub fn finish(self) -> Result<ScriptInfo, ParseError> {
        let pair = |x, y| match (x, y) {
            (None, None) => Ok(None),
            (Some(x), Some(y)) => Ok(Some(Size::new(x, y))),
            _ => Err(unsupported(
                "partial script resolution has no Core inference rule",
            )),
        };
        let play = pair(self.play_x, self.play_y)?.unwrap_or(Size::new(384, 288));
        let layout = pair(self.layout_x, self.layout_y)?;
        Ok(ScriptInfo::core_default()
            .with_play_res(play)?
            .with_layout_res(layout)?
            .with_wrap(self.wrap.unwrap_or(WrapMode::new(0)?))
            .with_scaled_border_shadow(self.scaled)
            .with_ycbcr_matrix(self.matrix.unwrap_or_else(|| Arc::from("")))
            .with_collision_header(self.collisions)
            .with_metadata(self.metadata.into()))
    }
}
