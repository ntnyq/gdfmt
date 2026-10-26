//! CST-based layout with token-anchored comments and validation helpers.
mod doc;

use gdfmt_syntax::{LosslessTree, SyntaxKind as K};
use gdfmt_text::{Diagnostic, Severity};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CollectionLayout {
    #[default]
    Auto,
    Multiline,
    Preserve,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DictionarySpacing {
    #[default]
    Compact,
    Spaced,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TrailingCommaLayout {
    #[default]
    Auto,
    Multiline,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LambdaBodyStyle {
    #[default]
    Auto,
    Multiline,
}

#[derive(Debug, Clone)]
pub struct FormatOptions {
    pub line_width: usize,
    pub indent: String,
    pub tab_width: usize,
    pub line_ending: LineEnding,
    pub max_blank_lines: usize,
    pub final_newline: bool,
    pub array_layout: CollectionLayout,
    pub dictionary_layout: CollectionLayout,
    pub enum_layout: CollectionLayout,
    pub dictionary_spacing: DictionarySpacing,
    pub trailing_comma_layout: TrailingCommaLayout,
    /// None preserves existing gaps; Some(0..=5) normalizes declaration boundaries.
    pub blank_lines_around_definitions: Option<u8>,
    pub lambda_body_style: LambdaBodyStyle,
    pub inline_comment_spacing: u8,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum LineEnding {
    Lf,
    Crlf,
    #[default]
    Preserve,
}
impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            line_width: 100,
            indent: "\t".into(),
            tab_width: 4,
            line_ending: LineEnding::Preserve,
            max_blank_lines: 2,
            final_newline: true,
            array_layout: CollectionLayout::default(),
            dictionary_layout: CollectionLayout::default(),
            enum_layout: CollectionLayout::default(),
            dictionary_spacing: DictionarySpacing::default(),
            trailing_comma_layout: TrailingCommaLayout::default(),
            blank_lines_around_definitions: None,
            lambda_body_style: LambdaBodyStyle::default(),
            inline_comment_spacing: 1,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FormatError {
    #[error("formatter-off/on markers must be paired and cannot nest")]
    InvalidDirective,
    #[error("formatting requires a valid parse without recovery or error diagnostics")]
    InvalidSyntax,
    #[error("invalid formatter layout options")]
    InvalidOptions,
}

/// The caller supplies the diagnostics from the same parse. Hosts should use
/// the `gdfmt` facade, which also validates the output before allowing writes.
pub fn format(
    tree: &LosslessTree,
    diagnostics: &[Diagnostic],
    options: &FormatOptions,
) -> Result<String, FormatError> {
    format_with_regions(tree, diagnostics, options, false)
}

/// Conservative fallback preserves complete top-level units touched by a region
/// when changing indentation outside that region would invalidate its contents.
pub fn format_with_regions(
    tree: &LosslessTree,
    diagnostics: &[Diagnostic],
    options: &FormatOptions,
    conservative: bool,
) -> Result<String, FormatError> {
    if options.line_width == 0
        || !(1..=8).contains(&options.tab_width)
        || options.indent.is_empty()
        || options.indent.len() > 8
        || options.max_blank_lines > 5
        || options
            .blank_lines_around_definitions
            .is_some_and(|value| value > 5)
        || !(1..=4).contains(&options.inline_comment_spacing)
        || !options
            .indent
            .chars()
            .all(|character| matches!(character, ' ' | '\t'))
    {
        return Err(FormatError::InvalidOptions);
    }
    let root = tree.root();
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
        || root
            .descendants_with_tokens()
            .any(|element| matches!(element.kind(), K::ErrorNode | K::Error))
    {
        return Err(FormatError::InvalidSyntax);
    }
    let source = tree.text();
    let preserved_newline = source.find('\n').map_or("\n", |offset| {
        if offset > 0 && source.as_bytes()[offset - 1] == b'\r' {
            "\r\n"
        } else {
            "\n"
        }
    });
    let newline = match options.line_ending {
        LineEnding::Lf => "\n",
        LineEnding::Crlf => "\r\n",
        LineEnding::Preserve => preserved_newline,
    };
    let printer = layout::Layout::new(&root, &source, options);
    let output = doc::print(&printer.node(&root), options, 0, "", newline);
    if root.kind() == K::SourceFile {
        let mut output = output.trim_end_matches(['\r', '\n']).to_owned();
        if options.final_newline && !output.is_empty() {
            output.push_str(newline);
        }
        preserve_regions(tree, &output, conservative)
    } else {
        Ok(output)
    }
}

fn preserve_regions(
    tree: &LosslessTree,
    output: &str,
    conservative: bool,
) -> Result<String, FormatError> {
    use gdfmt_syntax::directives::{formatter_regions, line_span, span};
    use gdfmt_text::{Span, TextEdit, apply_edits};
    let mut regions = formatter_regions(tree).map_err(|_| FormatError::InvalidDirective)?;
    if regions.is_empty() {
        return Ok(output.into());
    }
    let source = tree.text();
    let tokens: Vec<_> = tree
        .root()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| {
            !token.kind().is_virtual()
                && !matches!(
                    token.kind(),
                    K::Whitespace | K::PhysicalNewline | K::LineContinuation | K::Bom
                )
        })
        .collect();
    if conservative {
        for region in &mut regions {
            for node in tree.root().children() {
                let Some(first) = node
                    .descendants_with_tokens()
                    .filter_map(|element| element.into_token())
                    .find(|token| !token.kind().is_trivia() && !token.kind().is_virtual())
                else {
                    continue;
                };
                let Some(last) = node
                    .descendants_with_tokens()
                    .filter_map(|element| element.into_token())
                    .filter(|token| !token.kind().is_trivia() && !token.kind().is_virtual())
                    .last()
                else {
                    continue;
                };
                let unit = line_span(
                    &source,
                    Span::new(span(&first).start(), span(&last).end()).unwrap(),
                );
                if unit.start() < region.end() && region.start() < unit.end() {
                    *region = Span::new(
                        region.start().min(unit.start()),
                        region.end().max(unit.end()),
                    )
                    .unwrap();
                }
            }
        }
    }
    regions.sort_by_key(|region| region.start());
    let mut merged: Vec<Span> = Vec::new();
    for region in regions {
        if let Some(previous) = merged.last_mut()
            && region.start() <= previous.end()
        {
            *previous = Span::new(previous.start(), previous.end().max(region.end())).unwrap();
        } else {
            merged.push(region);
        }
    }
    // Match every token in order, including strings and comments, so a marker
    // spelling inside a literal cannot be mistaken for a directive.
    let mut cursor = 0;
    let mut mapped = Vec::new();
    for token in tokens {
        let offset = output[cursor..]
            .find(token.text())
            .ok_or(FormatError::InvalidSyntax)?
            + cursor;
        let end = offset + token.text().len();
        mapped.push((span(&token), Span::new(offset as u32, end as u32).unwrap()));
        cursor = end;
    }
    let mut edits = Vec::new();
    for region in merged {
        let inside: Vec<_> = mapped
            .iter()
            .filter(|(original, _)| {
                original.start() >= region.start() && original.end() <= region.end()
            })
            .collect();
        let (Some(first), Some(last)) = (inside.first(), inside.last()) else {
            continue;
        };
        let printed = line_span(output, Span::new(first.1.start(), last.1.end()).unwrap());
        let printed = if region.end() as usize == source.len() {
            Span::new(printed.start(), output.len() as u32).unwrap()
        } else {
            printed
        };
        edits.push(TextEdit {
            span: printed,
            replacement: region.slice(&source).unwrap().into(),
        });
    }
    apply_edits(output, &edits).map_err(|_| FormatError::InvalidSyntax)
}

mod layout;

fn same_shape(left: &rowan::GreenNodeData, right: &rowan::GreenNodeData) -> bool {
    use gdfmt_syntax::GdscriptLanguage;
    use rowan::{Language, NodeOrToken};
    let kind = GdscriptLanguage::kind_from_raw;
    if kind(left.kind()) != kind(right.kind()) {
        return false;
    }
    let significant =
        |element: &NodeOrToken<&rowan::GreenNodeData, &rowan::GreenTokenData>| match element {
            NodeOrToken::Node(_) => true,
            NodeOrToken::Token(token) => {
                let kind = kind(token.kind());
                !kind.is_trivia() && !kind.is_virtual()
            }
        };
    let mut right = right.children().filter(significant);
    // Compare borrowed spellings and node boundaries directly instead of
    // materializing two full shape vectors and a String per significant token.
    for left in left.children().filter(significant) {
        match (left, right.next()) {
            (NodeOrToken::Node(left), Some(NodeOrToken::Node(right)))
                if same_shape(left, right) => {}
            (NodeOrToken::Token(left), Some(NodeOrToken::Token(right)))
                if kind(left.kind()) == kind(right.kind()) && left.text() == right.text() => {}
            _ => return false,
        }
    }
    right.next().is_none()
}
/// Compare node topology, significant token spellings and comment attachments.
/// Trivia may move between CST nodes, but each comment retains adjacent tokens,
/// its own-line/suffix role and any dangling suite owner. Parentheses, commas,
/// semicolons and signed-number tokens remain significant.
pub fn structurally_equivalent(left: &LosslessTree, right: &LosslessTree) -> bool {
    if !same_shape(&left.root().green(), &right.root().green()) {
        return false;
    }
    let before = CommentAnchors::new(left);
    let after = CommentAnchors::new(right);
    before.iter().eq(after.iter())
}

struct CommentAnchors {
    source: String,
    anchors: Vec<CommentAnchor>,
}

struct CommentAnchor {
    preceding_tokens: usize,
    own_line: bool,
    owner: Option<usize>,
    span: gdfmt_text::Span,
}

impl CommentAnchors {
    fn new(tree: &LosslessTree) -> Self {
        let tokens: Vec<_> = tree
            .root()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .collect();
        let significant: Vec<_> = tokens
            .iter()
            .filter(|token| !token.kind().is_trivia() && !token.kind().is_virtual())
            .map(|token| u32::from(token.text_range().start()))
            .collect();
        let source = tree.text();
        let mut preceding_tokens = 0;
        let mut previous_owner = None;
        let anchors = tokens
            .iter()
            .filter_map(|token| {
                if !token.kind().is_trivia() && !token.kind().is_virtual() {
                    preceding_tokens += 1;
                    previous_owner = None;
                }
                if token.kind() != K::Comment {
                    return None;
                }
                let prefix = &source[..u32::from(token.text_range().start()) as usize];
                let own_line = prefix
                    .rsplit('\n')
                    .next()
                    .unwrap_or_default()
                    .trim_start_matches('\u{feff}')
                    .trim()
                    .is_empty();
                let owner = own_line
                    .then(|| {
                        layout::comment_owner(
                            token,
                            &source,
                            significant.get(preceding_tokens).copied(),
                        )
                    })
                    .flatten();
                let owner = if own_line {
                    layout::ordered_owner(owner, &mut previous_owner)
                } else {
                    None
                };
                let owner = owner.and_then(|offset| {
                    // Offsets are ordered; partition_point also retains the first
                    // match if a caller-built tree has zero-width significant tokens.
                    let index = significant.partition_point(|start| *start < offset);
                    (significant.get(index) == Some(&offset)).then_some(index)
                });
                Some(CommentAnchor {
                    preceding_tokens,
                    own_line,
                    owner,
                    span: gdfmt_syntax::directives::span(token),
                })
            })
            .collect();
        // Release token cursors before constructing the other tree's anchors;
        // only source bytes and compact comment metadata survive comparison.
        Self { source, anchors }
    }

    fn iter(&self) -> impl Iterator<Item = (usize, bool, Option<usize>, &str)> {
        self.anchors.iter().map(|anchor| {
            (
                anchor.preceding_tokens,
                anchor.own_line,
                anchor.owner,
                anchor.span.slice(&self.source).expect("comment token span"),
            )
        })
    }
}
