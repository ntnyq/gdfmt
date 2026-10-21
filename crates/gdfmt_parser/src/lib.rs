//! Lossless lexer, expressions, declarations, and statement suites.

mod expression;
mod lexer;
mod unicode;
mod validation;

pub use expression::{ExpressionResult, ParseOptions, parse_expression};
pub use gdfmt_syntax::{LanguageFeature, LanguageVersion};
use gdfmt_syntax::{LosslessTree, Token};
use gdfmt_text::{Diagnostic, SourceId};
pub use lexer::lex;

#[derive(Debug, Clone)]
pub struct LexOptions {
    pub source_id: SourceId,
    pub max_source_bytes: usize,
    pub max_nesting: usize,
    pub max_diagnostics: usize,
}

impl Default for LexOptions {
    fn default() -> Self {
        Self {
            source_id: SourceId(0),
            max_source_bytes: 16 * 1024 * 1024,
            max_nesting: 256,
            max_diagnostics: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct LexResult {
    pub tokens: Vec<Token>,
    /// Absent only when the source exceeds the configured size or byte offset limit.
    pub tree: Option<LosslessTree>,
    pub diagnostics: Vec<Diagnostic>,
}

impl LexResult {
    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }
}

/// Script and expression results share storage; script roots are `SourceFile`.
pub type ParseResult = ExpressionResult;

/// Parse a complete script, preserving all source text even after syntax errors.
pub fn parse(source: &str, options: &ParseOptions) -> ParseResult {
    expression::parse_root(source, options, true)
}
