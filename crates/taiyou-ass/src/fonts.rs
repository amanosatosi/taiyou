//! ASS's six-bit, offset-33 attachment encoding (not MIME/base64 or Unix uuencode).
use crate::{Context, invalid, reserve};
use std::sync::Arc;
use taiyou_types::*;

#[derive(Default)]
pub(crate) struct Fonts {
    name: Option<Arc<str>>,
    start: usize,
    end: usize,
    pending: [u8; 4],
    count: usize,
    bytes: Vec<u8>,
    invalid: bool,
    total: u64,
    records: Vec<EmbeddedFont>,
    names: u64,
}
impl Fonts {
    fn emit(&mut self, count: usize, ctx: &Context<'_>) -> Result<(), ParseError> {
        let size = self
            .total
            .checked_add(self.bytes.len() as u64)
            .and_then(|n| n.checked_add(count as u64))
            .ok_or_else(|| ParseError::new(ErrorCode::LimitExceeded, "attachment size overflow"))?;
        let reservation = size
            .checked_mul(3)
            .and_then(|n| {
                n.checked_add(
                    (self.records.len() as u64 + 1)
                        * 3
                        * std::mem::size_of::<EmbeddedFont>() as u64,
                )
            })
            .and_then(|n| self.names.checked_mul(2).and_then(|names| n.checked_add(names)))
            .ok_or_else(|| {
                ParseError::new(ErrorCode::LimitExceeded, "attachment reservation overflow")
            })?;
        if size > ctx.options.limits().font_bytes()
            || reservation > ctx.options.limits().transient_bytes()
        {
            return Err(ParseError::new(
                ErrorCode::LimitExceeded,
                "embedded font budget exceeded",
            ));
        }
        self.bytes.try_reserve(count).map_err(|_| {
            ParseError::new(ErrorCode::AllocationFailed, "attachment allocation failed")
        })?;
        let [a, b, c, d] = self.pending;
        let decoded = [(a << 2) | (b >> 4), (b << 4) | (c >> 2), (c << 6) | d];
        self.bytes.extend_from_slice(&decoded[..count]);
        Ok(())
    }
    pub fn finish(&mut self, source: &str, ctx: &mut Context<'_>) -> Result<(), ParseError> {
        let Some(name) = self.name.take() else {
            return Ok(());
        };
        let span = SourceSpan::new(source, self.start, self.end)?;
        if !self.invalid && (self.count == 1 || self.bytes.is_empty() && self.count == 0) {
            self.invalid = true;
            ctx.recover(
                invalid("empty or truncated embedded font encoding"),
                None,
                span,
            )?;
        }
        if !self.invalid {
            if self.count >= 2 {
                self.emit(self.count - 1, ctx)?;
            }
            self.total = self
                .total
                .checked_add(self.bytes.len() as u64)
                .ok_or_else(|| {
                    ParseError::new(ErrorCode::LimitExceeded, "attachment total overflow")
                })?;
            reserve(&mut self.records)?;
            self.records.push(EmbeddedFont::new(
                name,
                Arc::from(std::mem::take(&mut self.bytes)),
                span,
            ));
        }
        self.bytes.clear();
        self.count = 0;
        self.pending = [0; 4];
        self.invalid = false;
        Ok(())
    }
    pub fn line(
        &mut self,
        text: &str,
        source: &str,
        span: SourceSpan,
        ctx: &mut Context<'_>,
    ) -> Result<(), ParseError> {
        let text = text.trim_matches([' ', '\t']);
        if text.is_empty() {
            return self.finish(source, ctx);
        }
        if let Some((key, value)) = text.split_once(':') {
            if key.eq_ignore_ascii_case("fontname") {
                self.finish(source, ctx)?;
                let value = value.trim_matches([' ', '\t']);
                if value.is_empty() {
                    return ctx.recover(invalid("empty embedded font name"), None, span);
                }
                let names = self
                    .names
                    .checked_add(value.len() as u64)
                    .ok_or_else(|| {
                        ParseError::new(ErrorCode::LimitExceeded, "attachment name size overflow")
                    })?;
                if names
                    .checked_mul(2)
                    .and_then(|n| {
                        self.total
                            .checked_mul(3)
                            .and_then(|bytes| n.checked_add(bytes))
                    })
                    .is_none_or(|n| n > ctx.options.limits().transient_bytes())
                {
                    return Err(ParseError::new(
                        ErrorCode::LimitExceeded,
                        "attachment metadata budget exceeded",
                    ));
                }
                self.names = names;
                self.name = Some(Arc::from(value));
                self.start = span.start() as usize;
                self.end = span.end() as usize;
                return Ok(());
            }
        }
        if self.name.is_none() {
            return ctx.recover(invalid("embedded font payload requires fontname"), None, span);
        }
        self.end = span.end() as usize;
        if self.invalid {
            return Ok(());
        }
        for byte in text.bytes() {
            if !(33..=96).contains(&byte) {
                self.invalid = true;
                return ctx.recover(
                    invalid("invalid embedded font encoding character"),
                    None,
                    span,
                );
            }
            self.pending[self.count] = byte - 33;
            self.count += 1;
            if self.count == 4 {
                self.emit(3, ctx)?;
                self.count = 0;
                self.pending = [0; 4];
            }
        }
        Ok(())
    }
    pub fn into_records(self) -> Arc<[EmbeddedFont]> {
        self.records.into()
    }
}
