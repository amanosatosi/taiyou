use crate::{decode, invalid, sections::Format};
use std::sync::Arc;
use taiyou_types::*;

pub(crate) struct PendingEvent<'a> {
    pub id: EventId,
    pub source: SourceSpan,
    pub style: &'a str,
    layer: i32,
    start: TimeUs,
    end: TimeUs,
    margins: Margins,
    actor: &'a str,
    effect: &'a str,
    pub text_span: SourceSpan,
}
impl PendingEvent<'_> {
    pub fn publish(
        self,
        style: StyleId,
        program: Arc<[TextOp]>,
        spans: Arc<[Option<SourceSpan>]>,
    ) -> Event {
        Event::new(self.id, self.start, self.end, style, self.source)
            .with_layer(self.layer)
            .with_margins(self.margins)
            .with_actor(Arc::from(self.actor))
            .with_effect(Arc::from(self.effect))
            .with_program(program)
            .with_text_source(self.text_span)
            .with_operation_sources(spans)
    }
}
pub(crate) fn parse<'a>(
    value: &'a str,
    offset: usize,
    source: &str,
    format: &Format,
    id: EventId,
    span: SourceSpan,
    limits: &Limits,
) -> Result<PendingEvent<'a>, ParseError> {
    format.validate_record(value)?;
    let required = |name| {
        format
            .field(value, name)
            .ok_or_else(|| invalid("Dialogue requires Start, End and Text fields"))
    };
    let text = required("text")?;
    if u64::try_from(text.0.len()).map_err(|_| invalid("text length overflow"))?
        > limits.text_bytes_per_event()
    {
        return Err(ParseError::new(
            ErrorCode::LimitExceeded,
            "event text byte limit exceeded",
        ));
    }
    let get = |name, default| {
        format
            .field(value, name)
            .map(|v| decode::trim(v.0))
            .unwrap_or(default)
    };
    Ok(PendingEvent {
        id,
        source: span,
        style: get("style", "Default"),
        layer: decode::integer(get("layer", "0"))?,
        start: decode::timestamp(required("start")?.0)?,
        end: decode::timestamp(required("end")?.0)?,
        margins: Margins::new(
            decode::unsigned(get("marginl", "0"))?,
            decode::unsigned(get("marginr", "0"))?,
            decode::unsigned(get("marginv", "0"))?,
        ),
        actor: get("name", ""),
        effect: get("effect", ""),
        text_span: SourceSpan::new(source, offset + text.1, offset + text.1 + text.0.len())?,
    })
}
