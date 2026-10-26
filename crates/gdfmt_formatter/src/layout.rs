//! Layout follows CST boundaries; trivia ownership in Rowan is deliberately
//! independent of comment attachment. Comments are anchored to adjacent tokens.
use std::collections::{BTreeMap, BTreeSet};

mod tree;
use tree::{Element, Node, Token};

use gdfmt_syntax::{SyntaxElement, SyntaxKind as K, SyntaxNode, SyntaxToken};

use crate::{
    CollectionLayout, DictionarySpacing, FormatOptions, LambdaBodyStyle, TrailingCommaLayout,
    doc::Doc,
};

pub(super) struct Layout<'a> {
    options: &'a FormatOptions,
    source: &'a str,
    leading: BTreeMap<u32, Vec<SyntaxToken>>,
    trailing: BTreeMap<u32, String>,
    continuations: BTreeSet<u32>,
    dangling: BTreeMap<u32, Vec<SyntaxToken>>,
    end: Vec<SyntaxToken>,
    bom: bool,
    final_newline: bool,
    blanks: BTreeMap<u32, usize>,
}

fn significant(token: &SyntaxToken) -> bool {
    !token.kind().is_trivia() && !token.kind().is_virtual()
}
fn start(token: &SyntaxToken) -> u32 {
    token.text_range().start().into()
}

impl<'a> Layout<'a> {
    pub(super) fn new(root: &SyntaxNode, source: &'a str, options: &'a FormatOptions) -> Self {
        let tokens: Vec<_> = root
            .descendants_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .collect();
        let mut layout = Self {
            options,
            source,
            leading: BTreeMap::new(),
            trailing: BTreeMap::new(),
            continuations: BTreeSet::new(),
            dangling: BTreeMap::new(),
            end: Vec::new(),
            bom: source.starts_with('\u{feff}'),
            final_newline: source.ends_with('\n'),
            blanks: BTreeMap::new(),
        };
        let mut previous = None;
        let mut previous_owner = None;
        let mut previous_end = 0;
        for (index, token) in tokens.iter().enumerate() {
            if significant(token) || token.kind() == K::Comment {
                let gap = &source[previous_end..start(token) as usize];
                let blanks = gap
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count()
                    .saturating_sub(1)
                    .min(options.max_blank_lines);
                if blanks > 0 && previous_end > 0 {
                    layout.blanks.insert(start(token), blanks);
                }
                previous_end = u32::from(token.text_range().end()) as usize;
            }
            if significant(token) {
                previous = Some(token);
                previous_owner = None;
            } else if token.kind() == K::LineContinuation
                && tokens[index + 1..]
                    .iter()
                    .find(|token| significant(token) || token.kind() == K::Comment)
                    .is_some_and(|token| token.kind() == K::Comment)
                && let Some(previous) = previous
            {
                // An own-line comment forces a break that still needs the
                // original explicit continuation outside delimiters.
                layout.continuations.insert(start(previous));
            } else if token.kind() == K::Comment {
                let prefix = &source[..start(token) as usize];
                let own_line = prefix
                    .rsplit('\n')
                    .next()
                    .unwrap_or_default()
                    .trim_start_matches('\u{feff}')
                    .trim()
                    .is_empty();
                let next = tokens[index + 1..].iter().find(|token| significant(token));
                let owner = if own_line {
                    ordered_owner(
                        comment_owner(token, source, next.map(start)),
                        &mut previous_owner,
                    )
                } else {
                    None
                };
                if !own_line && let Some(previous) = previous {
                    layout.trailing.insert(start(previous), token.text().into());
                } else if let Some(owner) = owner {
                    layout
                        .dangling
                        .entry(owner)
                        .or_default()
                        .push(token.clone());
                } else if let Some(next) = next {
                    layout
                        .leading
                        .entry(start(next))
                        .or_default()
                        .push(token.clone());
                } else {
                    layout.end.push(token.clone());
                }
            }
        }
        if let Some(count) = options.blank_lines_around_definitions {
            layout.definition_spacing(root, usize::from(count));
        }
        layout
    }

    /// Normalize only declaration boundaries in script/class scopes. Annotations
    /// and leading comments travel with the declaration, never with its gap.
    fn definition_spacing(&mut self, root: &SyntaxNode, count: usize) {
        for scope in root.descendants().filter(|node| {
            node.kind() == K::SourceFile
                || (node.kind() == K::Suite
                    && node
                        .parent()
                        .is_some_and(|parent| parent.kind() == K::ClassDecl))
        }) {
            let mut previous = None;
            let mut annotations = Vec::new();
            for node in scope.children() {
                if node.kind() == K::AnnotationNode {
                    annotations.push(node);
                    continue;
                }
                let definition = matches!(node.kind(), K::FunctionDecl | K::ClassDecl);
                let declaration = matches!(
                    node.kind(),
                    K::FunctionDecl
                        | K::ClassDecl
                        | K::VariableDecl
                        | K::ConstantDecl
                        | K::SignalDecl
                        | K::EnumDecl
                        | K::ExtendsDecl
                        | K::ClassNameDecl
                );
                if declaration
                    && previous.is_some_and(|was_definition| definition || was_definition)
                {
                    let mut heads = annotations.clone();
                    heads.push(node.clone());
                    let mut boundary = None;
                    for head in heads {
                        if let Some(token) = head
                            .descendants_with_tokens()
                            .filter_map(SyntaxElement::into_token)
                            .find(significant)
                        {
                            let offset = start(&token);
                            let comments = self.leading.get(&offset).cloned().unwrap_or_default();
                            boundary.get_or_insert_with(|| comments.first().map_or(offset, start));
                            self.blanks.remove(&offset);
                            for comment in comments {
                                self.blanks.remove(&start(&comment));
                            }
                        }
                    }
                    if let Some(boundary) = boundary {
                        self.blanks.insert(boundary, count);
                    }
                }
                previous = declaration.then_some(definition);
                annotations.clear();
            }
        }
    }

    pub(super) fn document(&self, root: &SyntaxNode) -> Doc<'a> {
        self.node(&Node::new(&root.green()))
    }

    fn inline_lambda(&self, node: &Node) -> bool {
        self.options.lambda_body_style == LambdaBodyStyle::Auto && inline_lambda(node)
    }

    fn contains_block_lambda(&self, node: &Node<'_>) -> bool {
        (node.kind() == K::LambdaExpr && !self.inline_lambda(node))
            || node
                .children()
                .any(|child| self.contains_block_lambda(&child))
    }

    fn ends_in_inline_lambda(&self, element: &Element<'_>) -> bool {
        fn ending(layout: &Layout<'_>, node: &Node<'_>) -> Option<bool> {
            for child in node.children_reversed() {
                // Only the final significant child's ancestor path can end
                // here. Traverse it once, skipping trailing trivia/empty nodes.
                let ending = match child {
                    Element::Node(child) => ending(layout, &child),
                    Element::Token(token) => token.significant().then_some(false),
                };
                if let Some(ending) = ending {
                    return Some(
                        ending || (node.kind() == K::LambdaExpr && layout.inline_lambda(node)),
                    );
                }
            }
            None
        }
        element
            .as_node()
            .is_some_and(|node| ending(self, node) == Some(true))
    }

    fn source_text(&self, token: &Token) -> &'a str {
        let range = token.text_range();
        &self.source[u32::from(range.start()) as usize..u32::from(range.end()) as usize]
    }
    fn leading(&self, token: &Token) -> Doc<'a> {
        Doc::concat(
            self.leading
                .get(&token.start())
                .into_iter()
                .flatten()
                .map(|comment| self.comment(comment)),
        )
    }
    fn comment(&self, comment: &SyntaxToken) -> Doc<'a> {
        Doc::concat([
            Doc::EnsureLine,
            self.blank_lines(start(comment)),
            Doc::text(
                &self.source[u32::from(comment.text_range().start()) as usize
                    ..u32::from(comment.text_range().end()) as usize],
            ),
            Doc::HardLine,
        ])
    }
    fn blank_lines(&self, offset: u32) -> Doc<'a> {
        Doc::concat((0..self.blanks.get(&offset).copied().unwrap_or(0)).map(|_| Doc::HardLine))
    }
    fn token_text(&self, token: &Token) -> Doc<'a> {
        Doc::concat([Doc::text(self.source_text(token)), self.suffix(token)])
    }
    fn suffix(&self, token: &Token) -> Doc<'a> {
        let mut docs = Vec::new();
        if self.continuations.contains(&token.start()) {
            docs.push(Doc::LineSuffix(" \\".into()));
        }
        if let Some(comment) = self.trailing.get(&token.start()) {
            docs.push(Doc::LineSuffix(format!(
                "{}{comment}",
                " ".repeat(usize::from(self.options.inline_comment_spacing))
            )));
        }
        Doc::concat_vec(docs)
    }
    fn token(&self, token: &Token) -> Doc<'a> {
        Doc::concat([
            self.leading(token),
            self.blank_lines(token.start()),
            self.token_text(token),
        ])
    }
    fn parts<'tree>(&self, node: &'tree Node<'tree>) -> impl Iterator<Item = Element<'tree>> {
        node.children_with_tokens().filter(|element| match element {
            Element::Node(_) => true,
            Element::Token(token) => token.significant(),
        })
    }
    fn element(&self, element: &Element) -> Doc<'a> {
        match element {
            Element::Node(node) => self.node(node),
            Element::Token(token) => self.token(token),
        }
    }
    fn sequence(&self, parts: &[Element]) -> Doc<'a> {
        Doc::concat(
            parts
                .iter()
                .flat_map(|element| [Doc::EnsureLine, self.element(element)]),
        )
    }
    fn dangling(&self, node: &Node) -> Doc<'a> {
        let first = node.first_significant_token();
        Doc::concat(
            first
                .and_then(|token| self.dangling.get(&token.start()))
                .into_iter()
                .flatten()
                .map(|comment| self.comment(comment)),
        )
    }
    fn delimited(&self, parts: &[Element], kind: K) -> Doc<'a> {
        let (Some(open), Some(close)) = (parts.first(), parts.last()) else {
            return Doc::text("");
        };
        if parts.len() < 2 {
            return self.element(open);
        }
        if parts.len() == 2
            && close
                .as_token()
                .is_none_or(|token| !self.leading.contains_key(&token.start()))
        {
            return Doc::concat(parts.iter().map(|part| self.element(part)));
        }
        let policy = match kind {
            K::ArrayExpr => self.options.array_layout,
            K::DictionaryExpr => self.options.dictionary_layout,
            K::EnumDecl => self.options.enum_layout,
            _ => CollectionLayout::Auto,
        };
        let multiline_source = || {
            let first = open
                .as_token()
                .map(|token| token.start())
                .unwrap_or_default() as usize;
            let last = close
                .as_token()
                .map(|token| token.start())
                .unwrap_or(first as u32) as usize;
            self.source[first..last].contains('\n')
        };
        let force = parts.len() > 2
            && (policy == CollectionLayout::Multiline
                || (policy == CollectionLayout::Preserve && multiline_source())
                || (self.options.trailing_comma_layout == TrailingCommaLayout::Multiline
                    && parts[parts.len() - 2].kind() == K::Comma));
        let spacing = if kind == K::DictionaryExpr
            && self.options.dictionary_spacing == DictionarySpacing::Spaced
        {
            " "
        } else {
            ""
        };
        let boundary = || {
            if force {
                Doc::EnsureLine
            } else {
                Doc::Line(spacing)
            }
        };
        // A lambda suite's indentation belongs to its enclosing statement.
        // Continuation indentation is not a lexer indentation-stack entry, so
        // dedenting a block lambda to that column can be invalid GDScript.
        let block_lambda = parts
            .iter()
            .filter_map(Element::as_node)
            .any(|node| self.contains_block_lambda(node));
        let mut body = if block_lambda {
            Vec::new()
        } else {
            vec![boundary()]
        };
        for (index, element) in parts[1..parts.len() - 1].iter().enumerate() {
            let doc = self.element(element);
            body.push(
                if element.kind() == K::Comma
                    && index > 0
                    && self.ends_in_inline_lambda(&parts[index])
                {
                    doc.statement_indent()
                } else {
                    doc
                },
            );
            if element.kind() == K::Comma && index + 2 < parts.len() - 1 {
                body.push(if force {
                    Doc::EnsureLine
                } else {
                    Doc::Line(" ")
                });
            }
        }
        let lambda_end = self.ends_in_inline_lambda(&parts[parts.len() - 2]);
        let close_doc = if let Some(token) = close.as_token() {
            let leading = self.leading(token);
            body.push(if lambda_end {
                leading.statement_indent()
            } else {
                leading
            });
            Doc::text(self.source_text(token))
        } else {
            self.element(close)
        };
        let group = Doc::concat([
            self.element(open),
            if block_lambda {
                Doc::concat_vec(body)
            } else {
                Doc::concat_vec(body).continuation_indent()
            },
            boundary(),
            if lambda_end {
                close_doc.statement_indent()
            } else {
                close_doc
            },
        ])
        .group();
        Doc::concat([
            group,
            close
                .as_token()
                .map_or_else(|| Doc::text(""), |token| self.suffix(token)),
        ])
    }

    fn generic(&self, parts: &[Element]) -> Doc<'a> {
        let mut docs = Vec::new();
        for (index, element) in parts.iter().enumerate() {
            if index > 0
                && (space_between(&parts[index - 1], element)
                    || (element.kind() == K::Colon
                        && parts
                            .get(index + 1)
                            .is_some_and(|next| next.kind() == K::Equal)))
            {
                docs.push(Doc::text(" "));
            }
            docs.push(self.element(element));
        }
        Doc::concat_vec(docs)
    }

    fn with_delimiters(&self, parts: &[Element], kind: K) -> Doc<'a> {
        let open = parts
            .iter()
            .position(|part| matches!(part.kind(), K::LeftParen | K::LeftBracket | K::LeftBrace));
        let close = parts.iter().rposition(|part| {
            matches!(part.kind(), K::RightParen | K::RightBracket | K::RightBrace)
        });
        if let (Some(open), Some(close)) = (open, close) {
            Doc::concat([
                self.generic(&parts[..open]),
                self.delimited(&parts[open..=close], kind),
                self.generic(&parts[close + 1..]),
            ])
        } else {
            self.generic(parts)
        }
    }

    fn node(&self, node: &Node) -> Doc<'a> {
        let parts = self.parts(node);
        if matches!(
            node.kind(),
            K::CallExpr
                | K::MemberExpr
                | K::NodeExpr
                | K::LiteralExpr
                | K::NameExpr
                | K::SelfExpr
                | K::SuperExpr
        ) {
            // These nodes concatenate children without neighbor-dependent
            // spacing. Avoid a temporary child vector for every leaf/wrapper.
            return Doc::concat(parts.map(|part| self.element(&part)));
        }
        let parts: Vec<_> = parts.collect();
        match node.kind() {
            K::SourceFile | K::ExpressionRoot => {
                let mut docs = Vec::new();
                if self.bom {
                    docs.push(Doc::text("\u{feff}"));
                }
                docs.push(if node.kind() == K::SourceFile {
                    self.sequence(&parts)
                } else {
                    self.generic(&parts)
                });
                for comment in &self.end {
                    docs.push(self.comment(comment));
                }
                if self.final_newline {
                    docs.push(Doc::EnsureLine);
                }
                Doc::concat_vec(docs)
            }
            K::ArrayExpr
            | K::DictionaryExpr
            | K::GroupExpr
            | K::ArgumentList
            | K::ParameterList => self.delimited(&parts, node.kind()),
            // Godot requires a collection's first type on the same line as '['.
            // Keep type references indivisible rather than inventing a break.
            K::AnnotationNode | K::AssertStmt | K::Pattern | K::SubscriptExpr => {
                self.with_delimiters(&parts, node.kind())
            }
            K::Suite => {
                if node.parent().is_some_and(|parent| {
                    parent.kind() == K::LambdaExpr && self.inline_lambda(&parent)
                }) {
                    return Doc::concat([
                        self.element(&parts[0]),
                        Doc::text(" "),
                        self.generic(&parts[1..]),
                    ]);
                }
                let mut docs = Vec::new();
                if let Some(colon) = parts.first() {
                    docs.push(self.element(colon));
                }
                let mut body = vec![self.sequence(&parts[1..])];
                body.push(self.dangling(node));
                docs.push(Doc::concat_vec(body).indent());
                Doc::concat_vec(docs)
            }
            K::LambdaExpr if !self.inline_lambda(node) => {
                Doc::concat([self.generic(&parts), Doc::EnsureLine])
            }
            K::PropertyBlock => {
                let colon = usize::from(parts.first().is_some_and(|part| part.kind() == K::Colon));
                let mut accessors = Vec::new();
                for part in &parts[colon..] {
                    if part.kind() != K::Comma {
                        accessors.push(Doc::EnsureLine);
                    }
                    accessors.push(self.element(part));
                }
                Doc::concat([
                    self.generic(&parts[..colon]),
                    Doc::concat([Doc::concat_vec(accessors), self.dangling(node)]).indent(),
                ])
            }
            K::MatchStmt => {
                let split = parts
                    .iter()
                    .position(|part| {
                        matches!(
                            part.kind(),
                            K::MatchBranch | K::PassStmt | K::AnnotationNode
                        )
                    })
                    .unwrap_or(parts.len());
                Doc::concat([
                    self.generic(&parts[..split]),
                    Doc::concat([self.sequence(&parts[split..]), self.dangling(node)]).indent(),
                ])
            }
            K::IfStmt => {
                let split = parts
                    .iter()
                    .position(|part| matches!(part.kind(), K::ElifClause | K::ElseClause))
                    .unwrap_or(parts.len());
                Doc::concat([
                    self.generic(&parts[..split]),
                    self.sequence(&parts[split..]),
                ])
            }
            K::EnumDecl => {
                let open = parts
                    .iter()
                    .position(|part| part.kind() == K::LeftBrace)
                    .unwrap_or(parts.len());
                let close = parts
                    .iter()
                    .rposition(|part| part.kind() == K::RightBrace)
                    .unwrap_or(open);
                Doc::concat([
                    self.generic(&parts[..open]),
                    Doc::text(" "),
                    self.delimited(&parts[open..=close], K::EnumDecl),
                    self.generic(&parts[close + 1..]),
                ])
            }
            K::BinaryExpr | K::TernaryExpr | K::CastExpr | K::TypeTestExpr => {
                Doc::concat(parts.iter().enumerate().flat_map(|(index, part)| {
                    [
                        Doc::text(if index == 0 { "" } else { " " }),
                        self.element(part),
                    ]
                }))
            }
            K::UnaryExpr | K::AwaitExpr => {
                let mut docs = Vec::new();
                if let Some(first) = parts.first() {
                    docs.push(self.element(first));
                }
                if let Some(first) = parts.first().and_then(Element::as_token) {
                    let operand = node.children().next().and_then(|node| node.first_token());
                    if first.kind() == K::Keyword
                        || (matches!(first.kind(), K::Plus | K::Minus)
                            && operand.is_some_and(|token| {
                                matches!(token.kind(), K::Number | K::Plus | K::Minus)
                            }))
                    {
                        docs.push(Doc::text(" "));
                    }
                }
                docs.extend(parts.iter().skip(1).map(|part| self.element(part)));
                Doc::concat_vec(docs)
            }
            _ => self.generic(&parts),
        }
    }
}

/// Once a run of comments leaves a completed block, later comment-only
/// indentation cannot reopen that block. This also keeps irregularly indented
/// comment runs in source order when they span several dedents.
pub(super) fn ordered_owner(owner: Option<u32>, previous: &mut Option<Option<u32>>) -> Option<u32> {
    let owner = match *previous {
        Some(None) => None,
        Some(Some(previous)) if owner.is_some_and(|owner| owner > previous) => Some(previous),
        _ => owner,
    };
    *previous = Some(owner);
    owner
}

fn inline_lambda(node: &Node<'_>) -> bool {
    let Some(suite) = node.children().find(|node| node.kind() == K::Suite) else {
        return false;
    };
    let mut statements = suite.children();
    let Some(statement) = statements.next() else {
        return false;
    };
    statements.next().is_none()
        && !matches!(
            statement.kind(),
            K::IfStmt | K::ForStmt | K::WhileStmt | K::MatchStmt
        )
        && !suite.contains_comment_or_lambda()
}

/// Comment-only lines do not emit indentation tokens. At the end of a suite,
/// use their original indentation to distinguish dangling body comments from
/// leading comments for the next declaration (both can be stored in the suite).
pub(super) fn comment_owner(token: &SyntaxToken, source: &str, next: Option<u32>) -> Option<u32> {
    fn indentation(source: &str, offset: u32) -> usize {
        let prefix = &source[..offset as usize];
        let line = prefix.rsplit('\n').next().unwrap_or_default();
        let indent: String = line
            .chars()
            .take_while(|character| matches!(character, ' ' | '\t'))
            .collect();
        crate::doc::columns(&indent, 0, 4)
    }
    let comment_indent = indentation(source, start(token));
    token
        .parent_ancestors()
        .filter(|node| matches!(node.kind(), K::Suite | K::MatchStmt | K::PropertyBlock))
        .find_map(|suite| {
            let colon = suite
                .descendants_with_tokens()
                .filter_map(SyntaxElement::into_token)
                .find(significant)?;
            let end = u32::from(suite.text_range().end());
            let header = if suite.kind() == K::PropertyBlock {
                suite
                    .parent()
                    .and_then(|parent| parent.first_token())
                    .map_or(start(&colon), |token| start(&token))
            } else {
                start(&colon)
            };
            (next.is_none_or(|next| next >= end) && comment_indent > indentation(source, header))
                .then(|| start(&colon))
        })
}

fn space_between(left: &Element<'_>, right: &Element<'_>) -> bool {
    let a = left.kind();
    let b = right.kind();
    if matches!(
        b,
        K::Suite
            | K::PropertyBlock
            | K::Comma
            | K::Semicolon
            | K::Dot
            | K::Colon
            | K::RightParen
            | K::RightBracket
            | K::RightBrace
    ) || matches!(
        a,
        K::Dot | K::LeftParen | K::LeftBracket | K::LeftBrace | K::Ellipsis
    ) || matches!(b, K::LeftParen | K::LeftBracket | K::ParameterList)
    {
        return false;
    }
    if a == K::Colon {
        return b != K::Equal;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use gdfmt_parser::{ParseOptions, parse};

    fn reference_inline_lambda(node: &SyntaxNode) -> bool {
        let Some(suite) = node.children().find(|node| node.kind() == K::Suite) else {
            return false;
        };
        let statements: Vec<_> = suite.children().collect();
        statements.len() == 1
            && !matches!(
                statements[0].kind(),
                K::IfStmt | K::ForStmt | K::WhileStmt | K::MatchStmt
            )
            && !suite
                .descendants_with_tokens()
                .any(|element| matches!(element.kind(), K::Comment | K::LambdaExpr))
    }

    #[test]
    fn lambda_tail_path_matches_full_subtree_reference() {
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/compatibility/fixtures");
        let mut checked = 0;
        for entry in std::fs::read_dir(fixtures).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|extension| extension != "gd") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            let parsed = parse(&source, &ParseOptions::default());
            if parsed.has_errors() {
                continue;
            }
            let root = parsed.tree.unwrap().root();
            let green = root.green();
            for style in [LambdaBodyStyle::Auto, LambdaBodyStyle::Multiline] {
                let options = FormatOptions {
                    lambda_body_style: style,
                    ..Default::default()
                };
                let layout = Layout::new(&root, &source, &options);
                for (red, borrowed) in root.descendants().zip(Node::new(&green).descendants()) {
                    let last = |node: &SyntaxNode| {
                        node.descendants_with_tokens()
                            .filter_map(SyntaxElement::into_token)
                            .filter(significant)
                            .last()
                            .map(|token| token.text_range().end())
                    };
                    let end = last(&red);
                    let expected = style == LambdaBodyStyle::Auto
                        && red.descendants().any(|lambda| {
                            lambda.kind() == K::LambdaExpr
                                && reference_inline_lambda(&lambda)
                                && last(&lambda) == end
                        });
                    assert_eq!(
                        layout.ends_in_inline_lambda(&Element::Node(borrowed)),
                        expected,
                        "{}: {:?}",
                        path.display(),
                        red.text_range()
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 1000);
    }
}
