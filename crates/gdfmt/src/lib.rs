//! Pure text facade. Writable output is returned only after syntax, structure
//! and idempotence checks succeed with the same language profile and limits.
pub use gdfmt_formatter::{FormatOptions, LineEnding};
pub use gdfmt_parser::{LanguageVersion, ParseOptions};
use gdfmt_text::{Diagnostic, DiagnosticCategory, Severity, Span};

#[derive(Debug, Clone, Default)]
pub struct FormatRequest {
    pub parse: ParseOptions,
    pub format: FormatOptions,
}

#[derive(Debug, Clone)]
pub struct FormatResult {
    /// None for source errors or failed formatter invariants. Never write a
    /// recovery tree or a partially formatted file to disk.
    pub output: Option<String>,
    pub changed: bool,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn format(source: &str, request: &FormatRequest) -> FormatResult {
    let parsed = gdfmt_parser::parse(source, &request.parse);
    let mut result = FormatResult {
        output: None,
        changed: false,
        diagnostics: parsed.diagnostics.clone(),
    };
    if parsed.has_errors() {
        return result;
    }
    let Some(tree) = parsed.tree else {
        return result;
    };
    let regions = match gdfmt_syntax::directives::formatter_regions(&tree) {
        Ok(regions) => regions,
        Err(errors) => {
            for span in errors
                .into_iter()
                .take(request.parse.lex.max_diagnostics.max(1))
            {
                result.diagnostics.push(Diagnostic {
                    code: "FMT002".into(),
                    category: DiagnosticCategory::Configuration,
                    severity: Severity::Error,
                    message: "formatter-off/on markers must be paired and cannot nest".into(),
                    source_id: request.parse.lex.source_id,
                    span,
                });
            }
            return result;
        }
    };
    let protected: Vec<_> = regions
        .iter()
        .map(|span| span.slice(source).expect("valid region span"))
        .collect();
    let output = match gdfmt_formatter::format(&tree, &result.diagnostics, &request.format) {
        Ok(output) => output,
        Err(error) => {
            internal(&mut result, request, error.to_string());
            return result;
        }
    };
    let valid_output = |output: &str, conservative| {
        let reparsed = gdfmt_parser::parse(output, &request.parse);
        !reparsed.has_errors()
            && reparsed.tree.as_ref().is_some_and(|reparsed_tree| {
                gdfmt_formatter::structurally_equivalent(&tree, reparsed_tree)
                    && gdfmt_syntax::directives::formatter_regions(reparsed_tree).is_ok_and(
                        |regions| {
                            regions
                                .iter()
                                .map(|span| span.slice(output).expect("valid output region span"))
                                .eq(protected.iter().copied())
                        },
                    )
                    && gdfmt_formatter::format_with_regions(
                        reparsed_tree,
                        &reparsed.diagnostics,
                        &request.format,
                        conservative,
                    )
                    .is_ok_and(|second| second == output)
            })
    };
    let output = if valid_output(&output, false) {
        Some(output)
    } else if !regions.is_empty() {
        gdfmt_formatter::format_with_regions(&tree, &result.diagnostics, &request.format, true)
            .ok()
            .filter(|output| valid_output(output, true))
    } else {
        None
    };
    let Some(output) = output else {
        internal(
            &mut result,
            request,
            "formatter reparse, structural equivalence or idempotence check failed".into(),
        );
        return result;
    };
    result.changed = output != source;
    result.output = Some(output);
    result
}

fn internal(result: &mut FormatResult, request: &FormatRequest, message: String) {
    result.diagnostics.push(Diagnostic {
        code: "FMT001".into(),
        category: DiagnosticCategory::Internal,
        severity: Severity::Error,
        message,
        source_id: request.parse.lex.source_id,
        span: Span::empty(0),
    });
}
