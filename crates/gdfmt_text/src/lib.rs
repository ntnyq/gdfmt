//! Source locations use half-open UTF-8 byte ranges. Display positions are zero-based.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SpanRepr")]
pub struct Span {
    start: u32,
    end: u32,
}

#[derive(Deserialize)]
struct SpanRepr {
    start: u32,
    end: u32,
}

impl TryFrom<SpanRepr> for Span {
    type Error = TextError;

    fn try_from(value: SpanRepr) -> Result<Self, Self::Error> {
        Self::new(value.start, value.end)
    }
}

impl Span {
    pub fn new(start: u32, end: u32) -> Result<Self, TextError> {
        if start > end {
            return Err(TextError::ReversedSpan);
        }
        Ok(Self { start, end })
    }

    pub const fn empty(offset: u32) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    pub const fn start(self) -> u32 {
        self.start
    }
    pub const fn end(self) -> u32 {
        self.end
    }
    pub const fn len(self) -> u32 {
        self.end - self.start
    }
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }

    pub fn slice(self, source: &str) -> Result<&str, TextError> {
        source
            .get(self.start as usize..self.end as usize)
            .ok_or(TextError::InvalidBoundary)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TextError {
    #[error("source exceeds the 32-bit byte offset limit")]
    SourceTooLarge,
    #[error("span start exceeds span end")]
    ReversedSpan,
    #[error("offset is outside the source or splits a UTF-8 character")]
    InvalidBoundary,
    #[error("text edits overlap or insert at the same byte offset")]
    OverlappingEdits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// Owns the indexed source so positions cannot accidentally use a different document.
#[derive(Debug, Clone)]
pub struct LineIndex {
    source: Box<str>,
    starts: Vec<u32>,
}

impl LineIndex {
    pub fn new(source: &str) -> Result<Self, TextError> {
        u32::try_from(source.len()).map_err(|_| TextError::SourceTooLarge)?;
        let mut starts = vec![0];
        for (offset, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                starts.push((offset + 1) as u32);
            }
        }
        Ok(Self {
            source: source.into(),
            starts,
        })
    }

    fn prefix(&self, offset: u32) -> Result<(u32, &str), TextError> {
        if offset as usize > self.source.len() || !self.source.is_char_boundary(offset as usize) {
            return Err(TextError::InvalidBoundary);
        }
        let line = self.starts.partition_point(|start| *start <= offset) - 1;
        Ok((
            line as u32,
            &self.source[self.starts[line] as usize..offset as usize],
        ))
    }

    pub fn byte_position(&self, offset: u32) -> Result<Position, TextError> {
        let (line, prefix) = self.prefix(offset)?;
        Ok(Position {
            line,
            column: prefix.len() as u32,
        })
    }

    pub fn utf16_position(&self, offset: u32) -> Result<Position, TextError> {
        let (line, prefix) = self.prefix(offset)?;
        Ok(Position {
            line,
            column: prefix.encode_utf16().count() as u32,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticCategory {
    Lexical,
    Syntax,
    Configuration,
    Io,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub code: String,
    pub category: DiagnosticCategory,
    pub severity: Severity,
    pub message: String,
    pub source_id: SourceId,
    pub span: Span,
}

impl Diagnostic {
    pub fn lexical(
        code: &str,
        message: impl Into<String>,
        source_id: SourceId,
        span: Span,
    ) -> Self {
        Self {
            code: code.into(),
            category: DiagnosticCategory::Lexical,
            severity: Severity::Error,
            message: message.into(),
            source_id,
            span,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEdit {
    pub span: Span,
    pub replacement: String,
}

/// Validates the complete batch before changing any bytes. Input order is irrelevant.
pub fn apply_edits(source: &str, edits: &[TextEdit]) -> Result<String, TextError> {
    let mut sorted: Vec<_> = edits.iter().collect();
    sorted.sort_by_key(|edit| (edit.span.start, edit.span.end));
    let mut previous: Option<Span> = None;
    for edit in &sorted {
        edit.span.slice(source)?;
        if let Some(span) = previous
            && (edit.span.start < span.end || edit.span.start == span.start)
        {
            return Err(TextError::OverlappingEdits);
        }
        previous = Some(edit.span);
    }
    let mut output = String::new();
    let mut cursor = 0;
    for edit in sorted {
        output.push_str(&source[cursor..edit.span.start as usize]);
        output.push_str(&edit.replacement);
        cursor = edit.span.end as usize;
    }
    output.push_str(&source[cursor..]);
    Ok(output)
}
