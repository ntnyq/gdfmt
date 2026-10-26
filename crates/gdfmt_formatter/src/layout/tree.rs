//! Borrowed, immutable layout views. Offsets include all trivia and zero-width
//! tokens; navigating them does not allocate Rowan's parent-aware red cursors.
use gdfmt_syntax::{GdscriptLanguage, SyntaxKind as K};
use rowan::{GreenNodeData, GreenTokenData, Language, NodeOrToken, TextRange};

#[derive(Clone, Copy)]
pub(super) struct Node<'a> {
    green: &'a GreenNodeData,
    offset: u32,
    parent: Option<Parent<'a>>,
}

#[derive(Clone, Copy)]
struct Parent<'a> {
    green: &'a GreenNodeData,
    offset: u32,
}

#[derive(Clone, Copy)]
pub(super) struct Token<'a> {
    green: &'a GreenTokenData,
    offset: u32,
}

#[derive(Clone, Copy)]
pub(super) enum Element<'a> {
    Node(Node<'a>),
    Token(Token<'a>),
}

impl<'a> Node<'a> {
    pub(super) fn new(green: &'a GreenNodeData) -> Self {
        Self {
            green,
            offset: 0,
            parent: None,
        }
    }

    pub(super) fn kind(&self) -> K {
        GdscriptLanguage::kind_from_raw(self.green.kind())
    }

    pub(super) fn children_with_tokens(&self) -> impl Iterator<Item = Element<'a>> {
        self.green.children().scan(self.offset, |offset, child| {
            let element = self.child(child, *offset);
            *offset += u32::from(child.text_len());
            Some(element)
        })
    }

    pub(super) fn children_reversed(&self) -> impl Iterator<Item = Element<'a>> {
        self.green.children().rev().scan(
            self.offset + u32::from(self.green.text_len()),
            |offset, child| {
                *offset -= u32::from(child.text_len());
                Some(self.child(child, *offset))
            },
        )
    }

    fn child(
        &self,
        child: NodeOrToken<&'a GreenNodeData, &'a GreenTokenData>,
        offset: u32,
    ) -> Element<'a> {
        match child {
            NodeOrToken::Node(green) => Element::Node(Node {
                green,
                offset,
                parent: Some(Parent {
                    green: self.green,
                    offset: self.offset,
                }),
            }),
            NodeOrToken::Token(green) => Element::Token(Token { green, offset }),
        }
    }

    pub(super) fn children(&self) -> impl Iterator<Item = Node<'a>> {
        self.children_with_tokens().filter_map(Element::into_node)
    }

    // Layout only needs the immediate parent's contents (inline lambda suites).
    // Deliberately do not expose a misleading complete ancestor chain.
    pub(super) fn parent(&self) -> Option<Node<'a>> {
        self.parent.map(|parent| Node {
            green: parent.green,
            offset: parent.offset,
            parent: None,
        })
    }

    pub(super) fn first_token(&self) -> Option<Token<'a>> {
        match self.children_with_tokens().next()? {
            Element::Node(node) => node.first_token(),
            Element::Token(token) => Some(token),
        }
    }

    pub(super) fn first_significant_token(&self) -> Option<Token<'a>> {
        self.children_with_tokens().find_map(|child| match child {
            Element::Node(node) => node.first_significant_token(),
            Element::Token(token) => token.significant().then_some(token),
        })
    }

    pub(super) fn contains_comment_or_lambda(&self) -> bool {
        matches!(self.kind(), K::Comment | K::LambdaExpr)
            || self.children_with_tokens().any(|child| match child {
                Element::Node(node) => node.contains_comment_or_lambda(),
                Element::Token(token) => matches!(token.kind(), K::Comment | K::LambdaExpr),
            })
    }

    #[cfg(test)]
    pub(super) fn descendants_with_tokens(&self) -> impl Iterator<Item = Element<'a>> {
        let mut pending = vec![Element::Node(*self)];
        std::iter::from_fn(move || {
            let element = pending.pop()?;
            if let Element::Node(node) = element {
                pending.extend(node.children_reversed());
            }
            Some(element)
        })
    }

    #[cfg(test)]
    pub(super) fn descendants(&self) -> impl Iterator<Item = Node<'a>> {
        self.descendants_with_tokens()
            .filter_map(Element::into_node)
    }
}

impl Token<'_> {
    pub(super) fn kind(&self) -> K {
        GdscriptLanguage::kind_from_raw(self.green.kind())
    }

    pub(super) fn significant(&self) -> bool {
        !self.kind().is_trivia() && !self.kind().is_virtual()
    }

    pub(super) fn start(&self) -> u32 {
        self.offset
    }

    pub(super) fn text_range(&self) -> TextRange {
        TextRange::at(self.offset.into(), self.green.text_len())
    }
}

impl<'a> Element<'a> {
    pub(super) fn kind(&self) -> K {
        match self {
            Self::Node(node) => node.kind(),
            Self::Token(token) => token.kind(),
        }
    }
    pub(super) fn as_node(&self) -> Option<&Node<'a>> {
        match self {
            Self::Node(node) => Some(node),
            Self::Token(_) => None,
        }
    }
    pub(super) fn as_token(&self) -> Option<&Token<'a>> {
        match self {
            Self::Token(token) => Some(token),
            Self::Node(_) => None,
        }
    }
    fn into_node(self) -> Option<Node<'a>> {
        match self {
            Self::Node(node) => Some(node),
            Self::Token(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gdfmt_parser::{ParseOptions, parse};
    use gdfmt_syntax::SyntaxElement;

    #[test]
    fn borrowed_traversal_matches_rowan_for_valid_and_recovery_trees() {
        let mut sources: Vec<String> = [
            "",
            "\u{feff}# 中文\r\nfunc run():\r\n\tvar a = [func(x): return x + 1, 2]\r\n",
            "class Nested:\n    func run():\n        pass\n    # end\n",
            "func broken(:\n\tvar a = [1,\n",
            "var names = [\"same\", \"same\", \"same\"]\n",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/compatibility/fixtures");
        for entry in std::fs::read_dir(fixtures).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|extension| extension == "gd") {
                sources.push(std::fs::read_to_string(path).unwrap());
            }
        }
        for source in sources {
            let parsed = parse(&source, &ParseOptions::default());
            let Some(tree) = parsed.tree else {
                continue;
            };
            let root = tree.root();
            let green = root.green();
            let view = Node::new(&green);
            let before: Vec<_> = root
                .descendants_with_tokens()
                .map(|element| {
                    (
                        element.kind(),
                        element.as_token().map(|token| token.text_range()),
                    )
                })
                .collect();
            let after: Vec<_> = view
                .descendants_with_tokens()
                .map(|element| (element.kind(), element.as_token().map(Token::text_range)))
                .collect();
            assert_eq!(before, after, "{source}");
            assert_eq!(
                crate::contains_errors(&green),
                root.descendants_with_tokens()
                    .any(|element| matches!(element.kind(), K::ErrorNode | K::Error)),
                "{source}"
            );
            for (red, borrowed) in root.descendants().zip(view.descendants()) {
                assert_eq!(
                    red.parent().map(|parent| parent.kind()),
                    borrowed.parent().map(|parent| parent.kind())
                );
                assert_eq!(
                    red.first_token().map(|token| token.text_range()),
                    borrowed.first_token().map(|token| token.text_range())
                );
                assert_eq!(
                    red.descendants_with_tokens()
                        .filter_map(SyntaxElement::into_token)
                        .find(|token| !token.kind().is_trivia() && !token.kind().is_virtual())
                        .map(|token| token.text_range()),
                    borrowed
                        .first_significant_token()
                        .map(|token| token.text_range())
                );
                let before: Vec<_> = red
                    .children_with_tokens()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .map(|element| {
                        (
                            element.kind(),
                            element.as_token().map(|token| token.text_range()),
                        )
                    })
                    .collect();
                let after: Vec<_> = borrowed
                    .children_reversed()
                    .map(|element| (element.kind(), element.as_token().map(Token::text_range)))
                    .collect();
                assert_eq!(before, after, "{source}");
            }
        }
    }
}
