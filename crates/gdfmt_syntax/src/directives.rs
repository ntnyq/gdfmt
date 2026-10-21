//! Directives are recognized only in real, standalone comment tokens.
use crate::{LosslessTree, SyntaxKind, SyntaxToken};
use gdfmt_text::Span;

pub fn span(token: &SyntaxToken) -> Span {
    Span::new(
        token.text_range().start().into(),
        token.text_range().end().into(),
    )
    .expect("ordered token range")
}

pub fn comments(tree: &LosslessTree) -> Vec<(Span, String)> {
    let source = tree.text();
    tree.root()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| token.kind() == SyntaxKind::Comment)
        .filter(|token| {
            source[..u32::from(token.text_range().start()) as usize]
                .rsplit('\n')
                .next()
                .unwrap_or_default()
                .trim_start_matches('\u{feff}')
                .trim()
                .is_empty()
        })
        .filter_map(|token| {
            token
                .text()
                .strip_prefix('#')?
                .trim()
                .strip_prefix("gdfmt:")
                .map(|body| (span(&token), body.trim().to_owned()))
        })
        .collect()
}

pub fn line_span(source: &str, span: Span) -> Span {
    let start = source[..span.start() as usize]
        .rfind('\n')
        .map_or(0, |offset| offset + 1);
    let end = source[span.end() as usize..]
        .find('\n')
        .map_or(source.len(), |offset| span.end() as usize + offset + 1);
    // A BOM belongs to the file, rather than the first directive line.
    let start = if start == 0 && source.starts_with('\u{feff}') {
        3
    } else {
        start
    };
    Span::new(start as u32, end as u32).expect("ordered line range")
}

/// Includes both marker lines. Nesting and unpaired markers are errors.
pub fn formatter_regions(tree: &LosslessTree) -> Result<Vec<Span>, Vec<Span>> {
    let source = tree.text();
    let mut open = None;
    let mut regions = Vec::new();
    let mut errors = Vec::new();
    for (span, body) in comments(tree) {
        match body.as_str() {
            "formatter-off" if open.is_none() => open = Some(span),
            "formatter-on" if open.is_some() => {
                let begin = line_span(&source, open.take().unwrap());
                let end = line_span(&source, span);
                regions.push(Span::new(begin.start(), end.end()).unwrap());
            }
            "formatter-off" | "formatter-on" => errors.push(span),
            _ => {}
        }
    }
    if let Some(span) = open {
        errors.push(span);
    }
    if errors.is_empty() {
        Ok(regions)
    } else {
        Err(errors)
    }
}
