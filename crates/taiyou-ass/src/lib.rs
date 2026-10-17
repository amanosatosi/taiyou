//! Bounded UTF-8 CoreV1 ASS record ingestion. No tag evaluation or rendering.

mod decode;
mod events;
mod sections;
mod styles;

use std::sync::Arc;
use std::collections::BTreeMap;
use taiyou_types::*;

fn invalid(detail: &str) -> ParseError { ParseError::new(ErrorCode::InvalidScript, detail) }
fn unsupported(detail: &str) -> ParseError { ParseError::new(ErrorCode::UnsupportedFeature, detail) }
fn reserve<T>(values: &mut Vec<T>) -> Result<(), ParseError> {
    values.try_reserve(1).map_err(|_| ParseError::new(ErrorCode::AllocationFailed, "parser record allocation failed"))
}

struct Context<'a> { options: &'a ParseOptions, diagnostics: DiagnosticBuffer }
impl Context<'_> {
    fn report(&mut self, code: DiagnosticCode, event: Option<EventId>, span: SourceSpan, detail: &str) -> Result<(), ParseError> {
        self.diagnostics.push(Diagnostic::new(code, event, Some(span), detail))
    }
    fn recover(&mut self, error: ParseError, event: Option<EventId>, span: SourceSpan) -> Result<(), ParseError> {
        if self.options.syntax() == SyntaxPolicy::Strict || matches!(error.code(), ErrorCode::LimitExceeded | ErrorCode::AllocationFailed | ErrorCode::InvalidEncoding) {
            return Err(error.with_context(event, Some(span)));
        }
        let code = if error.code() == ErrorCode::UnsupportedFeature { DiagnosticCode::UnsupportedFeature } else { DiagnosticCode::MalformedRecord };
        self.report(code, event, span, error.detail())
    }
}

/// Parses the CoreV1 record subset using the caller's serialized session owner.
/// Malformed known records are skipped with bounded diagnostics in Recover mode;
/// Strict rejects them. Encoding and resource failures are always fatal.
/// Source text is retained verbatim, including BOM/line endings; spans are bytes.
pub fn parse_ass(bytes: &[u8], options: &ParseOptions, ingestion: &mut TrackIngestor) -> Result<ParsedTrack, ParseError> {
    options.limits().check_input_bytes(bytes.len())?;
    if options.profile() != Profile::CoreV1 { return Err(unsupported("LegacyXyV1 parsing requires its research gate")); }
    let source = decode::utf8(bytes)?;
    let mut ctx = Context { options, diagnostics: DiagnosticBuffer::new() };
    let mut header = sections::Header::default();
    let mut section = sections::Section::Ignored;
    let mut style_format = None;
    let mut event_format = None;
    let mut definitions = Vec::<Style>::new();
    let mut style_indices = BTreeMap::<Arc<str>, usize>::new();
    let mut pending = Vec::new();
    let mut ordinal = 0_u64;
    for line in sections::lines(source) {
        let text = line.text.trim_start_matches([' ', '\t']);
        if text.is_empty() || text.starts_with(';') { continue; }
        let span = SourceSpan::new(source, line.start, line.start + line.text.len())?;
        if text.starts_with('[') {
            match sections::section(text) {
                Ok(value) => {
                    section = value;
                    if section == sections::Section::Ignored { ctx.report(DiagnosticCode::UnknownSection, None, span, "unknown or unsupported section ignored")?; }
                }
                Err(error) => { section = sections::Section::Ignored; ctx.recover(error, None, span)?; }
            }
            continue;
        }
        let Some((key, value)) = text.split_once(':') else {
            if section != sections::Section::Ignored { ctx.recover(invalid("record has no colon"), None, span)?; }
            continue;
        };
        let key = decode::trim(key);
        match section {
            sections::Section::Script => header.record(key, value, &mut ctx, span)?,
            sections::Section::Styles if key.eq_ignore_ascii_case("format") => {
                style_format = None;
                match sections::Format::parse(value, false) {
                    Ok(format) => style_format = Some(format),
                    Err(error) => ctx.recover(error, None, span)?,
                }
            }
            sections::Section::Events if key.eq_ignore_ascii_case("format") => {
                event_format = None;
                match sections::Format::parse(value, true) {
                    Ok(format) => event_format = Some(format),
                    Err(error) => ctx.recover(error, None, span)?,
                }
            }
            sections::Section::Styles if key.eq_ignore_ascii_case("style") => {
                let result = style_format.as_ref().ok_or_else(|| invalid("Style requires a valid Format declaration")).and_then(|format| styles::parse(value, format));
                match result {
                    Ok(style) => {
                        if style_indices.insert(Arc::from(style.name()), definitions.len()).is_some() { ctx.report(DiagnosticCode::DuplicateStyle, None, span, "duplicate style: last definition wins")?; }
                        reserve(&mut definitions)?; definitions.push(style);
                    }
                    Err(error) => ctx.recover(error, None, span)?,
                }
            }
            sections::Section::Events if key.eq_ignore_ascii_case("dialogue") => {
                if ordinal >= options.limits().events() { return Err(ParseError::new(ErrorCode::LimitExceeded, "event record limit exceeded").with_context(None, Some(span))); }
                let id = EventId::new(u32::try_from(ordinal).map_err(|_| ParseError::new(ErrorCode::LimitExceeded, "event ordinal space exhausted"))?);
                ordinal += 1;
                let offset = line.start + line.text.len() - value.len();
                let result = event_format.as_ref().ok_or_else(|| invalid("Dialogue requires a valid Format declaration")).and_then(|format| events::parse(value, offset, source, format, id, span, options.limits()));
                match result {
                    Ok(event) => { reserve(&mut pending)?; pending.push(event); }
                    Err(error) => ctx.recover(error, Some(id), span)?,
                }
            }
            sections::Section::Ignored => (),
            _ if key.eq_ignore_ascii_case("comment") => (),
            _ => ctx.report(DiagnosticCode::UnsupportedFeature, None, span, "unknown record ignored")?,
        }
    }
    let script = header.finish()?;
    if !style_indices.contains_key("Default") {
        ctx.report(DiagnosticCode::MissingDefaultStyle, None, SourceSpan::new(source, 0, 0)?, "missing Default style synthesized from Core defaults")?;
        style_indices.insert(Arc::from("Default"), definitions.len());
        reserve(&mut definitions)?; definitions.push(Style::core_default());
    }
    let mut events = Vec::new();
    for event in pending {
        let resolved = style_indices.get(event.style).copied();
        let index = match resolved {
            Some(index) => index,
            None => {
                ctx.report(DiagnosticCode::UnknownStyle, Some(event.id), event.source, "unknown event style resolved to Default")?;
                *style_indices.get("Default").ok_or_else(|| invalid("Default style missing after normalization"))?
            }
        };
        let index = u32::try_from(index).map_err(|_| ParseError::new(ErrorCode::LimitExceeded, "style index overflow"))?;
        reserve(&mut events)?; events.push(event.publish(StyleId::new(index)));
    }
    let track = ingestion.ingest(Arc::from(source), script, definitions.into(), events.into(), options.limits())?;
    ParsedTrack::new(track, ctx.diagnostics)
}
