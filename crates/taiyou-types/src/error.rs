//! Stable machine codes, bounded human details and deterministic diagnostics (C7).

use std::fmt;
use std::sync::Arc;

use crate::{EventId, SourceSpan};

/// C7 maximum ordinary diagnostic records; one suppression summary may follow.
pub const MAX_DIAGNOSTICS: usize = 1_024;
/// Implementation bound for human details, truncated only at UTF-8 boundaries.
/// Machine codes and native numeric codes are never truncated.
pub const MAX_DETAIL_BYTES: usize = 4_096;

fn bounded_detail(detail: &str) -> Arc<str> {
    let mut end = detail.len().min(MAX_DETAIL_BYTES);
    while !detail.is_char_boundary(end) {
        end -= 1;
    }
    Arc::from(&detail[..end])
}

/// Stable program-flow codes from C7. Display text is not an API for matching.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ErrorCode {
    InvalidArgument,
    InvalidEncoding,
    InvalidScript,
    UnsupportedFeature,
    FontUnavailable,
    NativeFailure,
    LimitExceeded,
    AllocationFailed,
    InternalError,
}

impl ErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidArgument => "InvalidArgument",
            Self::InvalidEncoding => "InvalidEncoding",
            Self::InvalidScript => "InvalidScript",
            Self::UnsupportedFeature => "UnsupportedFeature",
            Self::FontUnavailable => "FontUnavailable",
            Self::NativeFailure => "NativeFailure",
            Self::LimitExceeded => "LimitExceeded",
            Self::AllocationFailed => "AllocationFailed",
            Self::InternalError => "InternalError",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ErrorData {
    code: ErrorCode,
    event: Option<EventId>,
    span: Option<SourceSpan>,
    native_code: Option<i64>,
    detail: Arc<str>,
}

macro_rules! error_wrapper {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name(ErrorData);

        impl $name {
            pub fn new(code: ErrorCode, detail: &str) -> Self {
                Self(ErrorData {
                    code,
                    event: None,
                    span: None,
                    native_code: None,
                    detail: bounded_detail(detail),
                })
            }

            pub fn with_context(
                mut self,
                event: Option<EventId>,
                span: Option<SourceSpan>,
            ) -> Self {
                self.0.event = event;
                self.0.span = span;
                self
            }

            pub fn with_native_code(mut self, code: i64) -> Self {
                self.0.native_code = Some(code);
                self
            }

            pub fn code(&self) -> ErrorCode {
                self.0.code
            }
            pub fn event(&self) -> Option<EventId> {
                self.0.event
            }
            pub fn span(&self) -> Option<SourceSpan> {
                self.0.span
            }
            pub fn native_code(&self) -> Option<i64> {
                self.0.native_code
            }
            pub fn detail(&self) -> &str {
                &self.0.detail
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}: {}", self.code().as_str(), self.detail())?;
                if let Some(code) = self.native_code() {
                    write!(f, " (native code {code})")?;
                }
                Ok(())
            }
        }

        impl std::error::Error for $name {}
    };
}

error_wrapper!(ParseError, "Owned bounded parse/ingestion error.");
error_wrapper!(
    RenderError,
    "Owned bounded render-stage error; no renderer is implemented here."
);
error_wrapper!(
    FontError,
    "Owned bounded font-stage error; no native adapter is implemented here."
);

/// Stable diagnostic codes for the static record subset and publication limits.
/// Declaration order is the canonical code ordering used by C7 publication.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum DiagnosticCode {
    MalformedRecord,
    UnknownSection,
    DuplicateStyle,
    MissingDefaultStyle,
    UnknownStyle,
    InactiveEvent,
    UnsupportedFeature,
    DiagnosticsSuppressed,
    MissingGlyph,
}

impl DiagnosticCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MalformedRecord => "MalformedRecord",
            Self::UnknownSection => "UnknownSection",
            Self::DuplicateStyle => "DuplicateStyle",
            Self::MissingDefaultStyle => "MissingDefaultStyle",
            Self::UnknownStyle => "UnknownStyle",
            Self::InactiveEvent => "InactiveEvent",
            Self::UnsupportedFeature => "UnsupportedFeature",
            Self::DiagnosticsSuppressed => "DiagnosticsSuppressed",
            Self::MissingGlyph => "MissingGlyph",
        }
    }
}

/// Immutable diagnostic value. No shared mutable logger or callback is retained.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    code: DiagnosticCode,
    event: Option<EventId>,
    span: Option<SourceSpan>,
    detail: Arc<str>,
}

impl Diagnostic {
    pub fn new(
        code: DiagnosticCode,
        event: Option<EventId>,
        span: Option<SourceSpan>,
        detail: &str,
    ) -> Self {
        Self {
            code,
            event,
            span,
            detail: bounded_detail(detail),
        }
    }
    pub fn code(&self) -> DiagnosticCode {
        self.code
    }
    pub fn event(&self) -> Option<EventId> {
        self.event
    }
    pub fn span(&self) -> Option<SourceSpan> {
        self.span
    }
    pub fn detail(&self) -> &str {
        &self.detail
    }

    fn sort_key(&self) -> (Option<EventId>, Option<u32>, DiagnosticCode) {
        (self.event, self.span.map(SourceSpan::start), self.code)
    }
}

/// Coordinator-owned bounded builder. Retains the earliest 1,024 canonical keys
/// even when supplied out of order, then appends one suppression-count record.
/// Equal keys retain input order; producers must use stable source order for ties.
#[derive(Debug, Default)]
pub struct DiagnosticBuffer {
    records: Vec<Diagnostic>,
    suppressed: u64,
}

impl DiagnosticBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diagnostic: Diagnostic) -> Result<(), ParseError> {
        if diagnostic.code == DiagnosticCode::DiagnosticsSuppressed {
            return Err(ParseError::new(
                ErrorCode::InvalidArgument,
                "suppression records are builder-owned",
            ));
        }
        let index = self
            .records
            .partition_point(|d| d.sort_key() <= diagnostic.sort_key());
        if self.records.len() == MAX_DIAGNOSTICS {
            self.suppressed = self.suppressed.checked_add(1).ok_or_else(|| {
                ParseError::new(ErrorCode::LimitExceeded, "diagnostic counter overflow")
            })?;
            if index == MAX_DIAGNOSTICS {
                return Ok(());
            }
            self.records.pop();
        } else {
            self.records.try_reserve(1).map_err(|_| {
                ParseError::new(ErrorCode::AllocationFailed, "diagnostic allocation failed")
            })?;
        }
        self.records.insert(index, diagnostic);
        Ok(())
    }

    pub fn suppressed_count(&self) -> u64 {
        self.suppressed
    }

    pub fn finish(mut self) -> Result<Vec<Diagnostic>, ParseError> {
        if self.suppressed > 0 {
            self.records.try_reserve(1).map_err(|_| {
                ParseError::new(
                    ErrorCode::AllocationFailed,
                    "diagnostic summary allocation failed",
                )
            })?;
            self.records.push(Diagnostic::new(
                DiagnosticCode::DiagnosticsSuppressed,
                None,
                None,
                &format!("{} diagnostics suppressed", self.suppressed),
            ));
        }
        Ok(self.records)
    }
}
