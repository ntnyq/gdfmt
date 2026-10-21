//! Layout follows CST boundaries; trivia ownership in Rowan is deliberately
//! independent of comment attachment. Comments are anchored to adjacent tokens.
use std::collections::BTreeMap;

use gdfmt_syntax::{SyntaxElement, SyntaxKind as K, SyntaxNode, SyntaxToken};

use crate::{FormatOptions, doc::Doc};

pub(super) struct Layout {
    leading: BTreeMap<u32, Vec<SyntaxToken>>,
    trailing: BTreeMap<u32, String>,
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

impl Layout {
    pub(super) fn new(root: &SyntaxNode, source: &str, options: &FormatOptions) -> Self {
        let tokens: Vec<_> = root
            .descendants_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .collect();
        let mut layout = Self {
            leading: BTreeMap::new(),
            trailing: BTreeMap::new(),
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
        layout
    }

    fn leading(&self, token: &SyntaxToken) -> Doc {
        Doc::concat(
            self.leading
                .get(&start(token))
                .into_iter()
                .flatten()
                .map(|comment| self.comment(comment)),
        )
    }
    fn comment(&self, comment: &SyntaxToken) -> Doc {
        Doc::concat([
            Doc::EnsureLine,
            self.blank_lines(comment),
            Doc::text(comment.text()),
            Doc::HardLine,
        ])
    }
    fn blank_lines(&self, token: &SyntaxToken) -> Doc {
        Doc::concat(
            (0..self.blanks.get(&start(token)).copied().unwrap_or(0)).map(|_| Doc::HardLine),
        )
    }
    fn token_text(&self, token: &SyntaxToken) -> Doc {
        Doc::concat([Doc::text(token.text()), self.suffix(token)])
    }
    fn suffix(&self, token: &SyntaxToken) -> Doc {
        let mut docs = Vec::new();
        if let Some(comment) = self.trailing.get(&start(token)) {
            docs.push(Doc::LineSuffix(format!(" {comment}")));
        }
        Doc::concat(docs)
    }
    fn token(&self, token: &SyntaxToken) -> Doc {
        Doc::concat([
            self.leading(token),
            self.blank_lines(token),
            self.token_text(token),
        ])
    }
    fn parts(&self, node: &SyntaxNode) -> Vec<SyntaxElement> {
        node.children_with_tokens()
            .filter(|element| match element {
                SyntaxElement::Node(_) => true,
                SyntaxElement::Token(token) => significant(token),
            })
            .collect()
    }
    fn element(&self, element: &SyntaxElement) -> Doc {
        match element {
            SyntaxElement::Node(node) => self.node(node),
            SyntaxElement::Token(token) => self.token(token),
        }
    }
    fn sequence(&self, parts: &[SyntaxElement]) -> Doc {
        Doc::concat(
            parts
                .iter()
                .flat_map(|element| [Doc::EnsureLine, self.element(element)]),
        )
    }
    fn dangling(&self, node: &SyntaxNode) -> Doc {
        let first = node
            .descendants_with_tokens()
            .filter_map(SyntaxElement::into_token)
            .find(significant);
        Doc::concat(
            first
                .and_then(|token| self.dangling.get(&start(&token)))
                .into_iter()
                .flatten()
                .map(|comment| self.comment(comment)),
        )
    }
    fn delimited(&self, parts: &[SyntaxElement]) -> Doc {
        let (Some(open), Some(close)) = (parts.first(), parts.last()) else {
            return Doc::text("");
        };
        if parts.len() < 2 {
            return self.element(open);
        }
        if parts.len() == 2
            && close
                .as_token()
                .is_none_or(|token| !self.leading.contains_key(&start(token)))
        {
            return Doc::concat(parts.iter().map(|part| self.element(part)));
        }
        // A lambda suite's indentation belongs to its enclosing statement.
        // Continuation indentation is not a lexer indentation-stack entry, so
        // dedenting a block lambda to that column can be invalid GDScript.
        let block_lambda = parts.iter().filter_map(SyntaxElement::as_node).any(|node| {
            node.descendants()
                .any(|node| node.kind() == K::LambdaExpr && !inline_lambda(&node))
        });
        let mut body = if block_lambda {
            Vec::new()
        } else {
            vec![Doc::Line("")]
        };
        for (index, element) in parts[1..parts.len() - 1].iter().enumerate() {
            body.push(self.element(element));
            if element.kind() == K::Comma && index + 2 < parts.len() - 1 {
                body.push(Doc::Line(" "));
            }
        }
        let close_doc = if let Some(token) = close.as_token() {
            body.push(self.leading(token));
            Doc::text(token.text())
        } else {
            self.element(close)
        };
        let group = Doc::concat([
            self.element(open),
            if block_lambda {
                Doc::concat(body)
            } else {
                Doc::concat(body).indent()
            },
            Doc::Line(""),
            close_doc,
        ])
        .group();
        Doc::concat([
            group,
            close
                .as_token()
                .map_or_else(|| Doc::text(""), |token| self.suffix(token)),
        ])
    }

    fn generic(&self, parts: &[SyntaxElement]) -> Doc {
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
        Doc::concat(docs)
    }

    fn with_delimiters(&self, parts: &[SyntaxElement]) -> Doc {
        let open = parts
            .iter()
            .position(|part| matches!(part.kind(), K::LeftParen | K::LeftBracket | K::LeftBrace));
        let close = parts.iter().rposition(|part| {
            matches!(part.kind(), K::RightParen | K::RightBracket | K::RightBrace)
        });
        if let (Some(open), Some(close)) = (open, close) {
            Doc::concat([
                self.generic(&parts[..open]),
                self.delimited(&parts[open..=close]),
                self.generic(&parts[close + 1..]),
            ])
        } else {
            self.generic(parts)
        }
    }

    pub(super) fn node(&self, node: &SyntaxNode) -> Doc {
        let parts = self.parts(node);
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
                Doc::concat(docs)
            }
            K::ArrayExpr
            | K::DictionaryExpr
            | K::GroupExpr
            | K::ArgumentList
            | K::ParameterList => self.delimited(&parts),
            // Godot requires a collection's first type on the same line as '['.
            // Keep type references indivisible rather than inventing a break.
            K::AnnotationNode | K::AssertStmt | K::Pattern | K::SubscriptExpr => {
                self.with_delimiters(&parts)
            }
            K::Suite => {
                if node
                    .parent()
                    .is_some_and(|parent| parent.kind() == K::LambdaExpr && inline_lambda(&parent))
                {
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
                docs.push(Doc::concat(body).indent());
                Doc::concat(docs)
            }
            K::LambdaExpr if !inline_lambda(node) => {
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
                    Doc::concat([Doc::concat(accessors), self.dangling(node)]).indent(),
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
                    self.delimited(&parts[open..=close]),
                    self.generic(&parts[close + 1..]),
                ])
            }
            K::BinaryExpr | K::TernaryExpr | K::CastExpr | K::TypeTestExpr => {
                Doc::concat(parts.iter().enumerate().flat_map(|(index, part)| {
                    let mut docs = Vec::new();
                    if index > 0 {
                        docs.push(Doc::text(" "));
                    }
                    docs.push(self.element(part));
                    docs
                }))
            }
            K::UnaryExpr | K::AwaitExpr => {
                let mut docs = Vec::new();
                if let Some(first) = parts.first() {
                    docs.push(self.element(first));
                }
                if let Some(first) = parts.first().and_then(SyntaxElement::as_token) {
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
                Doc::concat(docs)
            }
            K::CallExpr
            | K::MemberExpr
            | K::NodeExpr
            | K::LiteralExpr
            | K::NameExpr
            | K::SelfExpr
            | K::SuperExpr => Doc::concat(parts.iter().map(|part| self.element(part))),
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

fn inline_lambda(node: &SyntaxNode) -> bool {
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

fn space_between(left: &SyntaxElement, right: &SyntaxElement) -> bool {
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
