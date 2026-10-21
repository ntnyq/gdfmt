//! Typed views share the lossless tree; no second AST is built.

use crate::{SyntaxKind as K, SyntaxNode};

macro_rules! syntax_view {
    ($name:ident, $($kind:ident)|+) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name { syntax: SyntaxNode }
        impl $name {
            pub fn cast(syntax: SyntaxNode) -> Option<Self> {
                matches!(syntax.kind(), $(K::$kind)|+).then_some(Self { syntax })
            }
            pub fn syntax(&self) -> &SyntaxNode { &self.syntax }
        }
    };
}

syntax_view!(SourceFile, SourceFile);
syntax_view!(
    Declaration,
    ClassDecl
        | ExtendsDecl
        | ClassNameDecl
        | VariableDecl
        | ConstantDecl
        | EnumDecl
        | SignalDecl
        | FunctionDecl
);
syntax_view!(
    Statement,
    VariableDecl
        | ConstantDecl
        | IfStmt
        | ForStmt
        | WhileStmt
        | MatchStmt
        | ReturnStmt
        | AssertStmt
        | BreakStmt
        | ContinueStmt
        | PassStmt
        | BreakpointStmt
        | AssignmentStmt
        | ExpressionStmt
);
syntax_view!(Suite, Suite);

impl SourceFile {
    pub fn declarations(&self) -> impl Iterator<Item = Declaration> {
        self.syntax.children().filter_map(Declaration::cast)
    }
}
impl Declaration {
    /// Direct body, if present. Property accessors have their own nested suites.
    pub fn body(&self) -> Option<Suite> {
        self.syntax.children().find_map(Suite::cast)
    }
}
impl Suite {
    pub fn statements(&self) -> impl Iterator<Item = Statement> {
        self.syntax.children().filter_map(Statement::cast)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    syntax: SyntaxNode,
}

impl Expression {
    pub fn cast(syntax: SyntaxNode) -> Option<Self> {
        matches!(
            syntax.kind(),
            K::LambdaExpr
                | K::LiteralExpr
                | K::NameExpr
                | K::SelfExpr
                | K::SuperExpr
                | K::UnaryExpr
                | K::BinaryExpr
                | K::TernaryExpr
                | K::GroupExpr
                | K::ArrayExpr
                | K::DictionaryExpr
                | K::CallExpr
                | K::MemberExpr
                | K::SubscriptExpr
                | K::NodeExpr
                | K::AwaitExpr
                | K::CastExpr
                | K::TypeTestExpr
        )
        .then_some(Self { syntax })
    }

    pub fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }

    /// Direct expression children. Arguments and dictionary entries have their
    /// own list/entry nodes and can be traversed through `syntax()`.
    pub fn operands(&self) -> impl Iterator<Item = Self> {
        self.syntax.children().filter_map(Self::cast)
    }
}
