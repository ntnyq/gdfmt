//! Lossless lexical and hierarchical expression syntax storage.

pub mod ast;
pub mod directives;
mod language;
pub use language::{LanguageFeature, LanguageVersion};

use gdfmt_text::Span;
use rowan::{GreenNode, GreenNodeBuilder, Language};
use serde::Serialize;

macro_rules! syntax_kinds {
    ($($kind:ident),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[repr(u16)]
        pub enum SyntaxKind { $($kind),+ }
        const KINDS: &[SyntaxKind] = &[$(SyntaxKind::$kind),+];
    };
}

syntax_kinds! {
    ExpressionRoot, LiteralExpr, NameExpr, SelfExpr, SuperExpr, UnaryExpr, BinaryExpr, TernaryExpr,
    GroupExpr, ArrayExpr, DictionaryExpr, DictionaryEntry, CallExpr, ArgumentList,
    MemberExpr, SubscriptExpr, NodeExpr, AwaitExpr, CastExpr, TypeTestExpr, TypeRef,
    LambdaExpr, Suite, ClassDecl, ExtendsDecl, ClassNameDecl, VariableDecl, ConstantDecl,
    EnumDecl, EnumEntry, SignalDecl, FunctionDecl, ParameterList, Parameter, AnnotationNode,
    PropertyBlock, Getter, Setter, IfStmt, ElifClause, ElseClause, ForStmt, WhileStmt,
    MatchStmt, MatchBranch, Pattern, ReturnStmt, AssertStmt, BreakStmt, ContinueStmt,
    PassStmt, BreakpointStmt, AssignmentStmt, ExpressionStmt,
    ErrorNode,
    SourceFile, Bom, Whitespace, PhysicalNewline, Comment, LineContinuation,
    Newline, Indent, Dedent, Eof, Error, Identifier, Keyword, Annotation,
    Number, String, StringName, NodePath, Underscore, QuestionMark, Backtick, VcsConflictMarker,
    LeftParen, RightParen, LeftBracket, RightBracket, LeftBrace, RightBrace,
    Comma, Semicolon, Dot, DotDot, Ellipsis, Colon, Dollar, Arrow,
    Plus, Minus, Star, Power, Slash, Percent, Equal, EqualEqual, NotEqual,
    Less, LessEqual, Greater, GreaterEqual, Ampersand, Pipe, Caret, Tilde,
    ShiftLeft, ShiftRight, AndAnd, OrOr, Bang,
    PlusEqual, MinusEqual, StarEqual, PowerEqual, SlashEqual, PercentEqual,
    ShiftLeftEqual, ShiftRightEqual, AmpersandEqual, PipeEqual, CaretEqual,
}

impl SyntaxKind {
    /// Godot Token::is_identifier: API-compatible keyword exceptions and
    /// builtin constants are permitted at identifier sites.
    pub fn is_identifier(self, text: &str) -> bool {
        self == Self::Identifier
            || (self == Self::Keyword
                && matches!(text, "match" | "when" | "PI" | "TAU" | "INF" | "NAN"))
    }

    /// Godot Token::is_node_name, shared by node paths and dotted attributes.
    pub fn is_node_name(self, text: &str) -> bool {
        matches!(self, Self::Identifier | Self::Underscore)
            || (self == Self::Keyword && !matches!(text, "true" | "false" | "null"))
    }

    pub fn is_virtual(self) -> bool {
        matches!(
            self,
            Self::Newline | Self::Indent | Self::Dedent | Self::Eof
        )
    }

    pub fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::Bom
                | Self::Whitespace
                | Self::PhysicalNewline
                | Self::Comment
                | Self::LineContinuation
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GdscriptLanguage {}

impl Language for GdscriptLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        KINDS
            .get(raw.0 as usize)
            .copied()
            .unwrap_or(SyntaxKind::Error)
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}

pub type SyntaxNode = rowan::SyntaxNode<GdscriptLanguage>;
pub type SyntaxToken = rowan::SyntaxToken<GdscriptLanguage>;
pub type SyntaxElement = rowan::SyntaxElement<GdscriptLanguage>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Token {
    pub kind: SyntaxKind,
    pub span: Span,
}

/// Green storage can cross worker threads; create red cursors within each worker.
#[derive(Debug, Clone)]
pub struct LosslessTree {
    green: GreenNode,
}

impl LosslessTree {
    /// Wrap a parser-built green root. Leaves must preserve the source token order.
    pub fn from_green(green: GreenNode) -> Self {
        Self { green }
    }

    /// Returns None if ranges do not partition the source, or a virtual token has text.
    pub fn from_tokens(source: &str, tokens: &[Token]) -> Option<Self> {
        let mut builder = GreenNodeBuilder::new();
        builder.start_node(GdscriptLanguage::kind_to_raw(SyntaxKind::SourceFile));
        let mut offset = 0;
        for token in tokens {
            if token.span.start() != offset || (token.kind.is_virtual() && !token.span.is_empty()) {
                return None;
            }
            let text = token.span.slice(source).ok()?;
            builder.token(GdscriptLanguage::kind_to_raw(token.kind), text);
            offset = token.span.end();
        }
        if offset as usize != source.len() {
            return None;
        }
        builder.finish_node();
        Some(Self {
            green: builder.finish(),
        })
    }

    pub fn root(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }
    pub fn text(&self) -> String {
        self.root().text().to_string()
    }
}
