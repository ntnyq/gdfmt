#[path = "script.rs"]
mod script;
use crate::{LexOptions, lexer::Lexer};
use gdfmt_syntax::{GdscriptLanguage, LosslessTree, SyntaxKind as K, Token};
use gdfmt_text::{Diagnostic, DiagnosticCategory, Severity};
use rowan::{Checkpoint, GreenNodeBuilder, Language};
use std::cell::RefCell;

/// Shared options for standalone expressions and complete scripts.
#[derive(Debug, Clone)]
pub struct ParseOptions {
    pub lex: LexOptions,
    pub language_version: crate::LanguageVersion,
    /// Maximum tree height and recursion depth, capped at 128 for stack safety.
    pub max_expression_depth: usize,
}
impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            lex: LexOptions::default(),
            language_version: crate::LanguageVersion::default(),
            max_expression_depth: 128,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExpressionResult {
    /// Absent only when lexing rejects the source size.
    pub tree: Option<LosslessTree>,
    pub diagnostics: Vec<Diagnostic>,
}
impl ExpressionResult {
    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }
}

/// Parse exactly one expression, including lambda suites. Assignments and
/// declarations outside lambdas require the script entry point.
/// Invalid input still produces a lossless tree with error nodes.
pub fn parse_expression(source: &str, options: &ParseOptions) -> ExpressionResult {
    parse_root(source, options, false)
}

pub(crate) fn parse_root(source: &str, options: &ParseOptions, script: bool) -> ExpressionResult {
    let lexer = match Lexer::new(source, &options.lex) {
        Ok(lexer) => lexer,
        Err(result) => {
            return ExpressionResult {
                tree: None,
                diagnostics: result.diagnostics,
            };
        }
    };
    let mut parser = Parser {
        source,
        lexer: RefCell::new(lexer),
        options,
        cursor: 0,
        depth: 0,
        builder: GreenNodeBuilder::new(),
        suppressed: false,
        function_depth: 0,
        loop_depth: 0,
        lambda_depth: 0,
        lambda_ended: false,
    };
    if script {
        parser.start(K::SourceFile);
        while parser.current() != K::Eof {
            parser.items(true);
            if parser.current() == K::Dedent {
                parser.error("PAR009", "unexpected dedent at script scope");
                parser.bump();
            }
        }
    } else {
        parser.start(K::ExpressionRoot);
        parser.trivia();
        parser.expression(0);
        parser.eat(K::Newline);
        if parser.current() != K::Eof {
            parser.error("PAR003", "unexpected token after expression");
            parser.start(K::ErrorNode);
            while parser.current() != K::Eof {
                parser.bump();
            }
            parser.finish();
        }
    }
    while parser.cursor < parser.lexer.borrow().tokens.len() {
        parser.raw_bump();
    }
    parser.finish();
    let tree = LosslessTree::from_green(parser.builder.finish());
    let mut diagnostics = parser.lexer.into_inner().diagnostics;
    crate::validation::validate(&tree.root(), options, &mut diagnostics);
    ExpressionResult {
        tree: Some(tree),
        diagnostics,
    }
}

#[derive(Clone, Copy)]
struct Operand {
    kind: K,
    height: usize,
    is_string: bool,
}
pub(crate) struct Parser<'a> {
    source: &'a str,
    lexer: RefCell<Lexer<'a>>,
    options: &'a ParseOptions,
    cursor: usize,
    depth: usize,
    builder: GreenNodeBuilder<'static>,
    suppressed: bool,
    function_depth: usize,
    loop_depth: usize,
    lambda_depth: usize,
    lambda_ended: bool,
}
// Binding powers follow the pinned Godot Precedence enum. Power is left
// associative; casts bind below ternaries; type tests bind above power.
const CAST: u8 = 2;
const TERNARY: u8 = 3;
const LOGIC_NOT: u8 = 6;
const SIGN: u8 = 15;
const BIT_NOT: u8 = 16;
const AWAIT: u8 = 19;
const POSTFIX: u8 = 20;

impl Parser<'_> {
    fn token_at(&self, index: usize) -> Token {
        let mut lexer = self.lexer.borrow_mut();
        while index >= lexer.tokens.len() && !lexer.done {
            lexer.step();
        }
        lexer.tokens[index.min(lexer.tokens.len() - 1)]
    }
    fn index(&self) -> usize {
        let mut index = self.cursor;
        while self.token_at(index).kind.is_trivia() {
            index += 1;
        }
        index
    }
    fn current(&self) -> K {
        self.token_at(self.index()).kind
    }
    fn text(&self) -> &str {
        self.token_at(self.index()).span.slice(self.source).unwrap()
    }
    fn keyword(&self, text: &str) -> bool {
        self.current() == K::Keyword && self.text() == text
    }
    fn start(&mut self, kind: K) {
        self.builder.start_node(GdscriptLanguage::kind_to_raw(kind));
    }
    fn finish(&mut self) {
        self.builder.finish_node();
    }
    fn raw_bump(&mut self) {
        let token = self.token_at(self.cursor);
        self.builder.token(
            GdscriptLanguage::kind_to_raw(token.kind),
            token.span.slice(self.source).unwrap(),
        );
        self.cursor += 1;
    }
    fn trivia(&mut self) {
        while self.token_at(self.cursor).kind.is_trivia() {
            self.raw_bump();
        }
    }
    fn bump(&mut self) {
        self.trivia();
        if self.cursor < self.lexer.borrow().tokens.len() {
            self.raw_bump();
        }
    }
    fn eat(&mut self, kind: K) -> bool {
        if self.current() == kind {
            self.bump();
            if matches!(
                kind,
                K::Comma | K::RightParen | K::RightBracket | K::RightBrace
            ) {
                self.lambda_ended = false;
            }
            true
        } else {
            false
        }
    }
    fn expect(&mut self, kind: K, message: &str) {
        if !self.eat(kind) {
            self.error("PAR004", message);
        }
    }
    fn error(&mut self, code: &str, message: &str) {
        let span = self.token_at(self.index()).span;
        let count = self.lexer.borrow().diagnostics.len();
        let (code, message) = if count < self.options.lex.max_diagnostics {
            (code, message)
        } else if !self.suppressed && count == self.options.lex.max_diagnostics {
            self.suppressed = true;
            ("PAR002", "further syntax diagnostics suppressed")
        } else {
            return;
        };
        self.lexer.borrow_mut().diagnostics.push(Diagnostic {
            code: code.into(),
            category: DiagnosticCategory::Syntax,
            severity: Severity::Error,
            message: message.into(),
            source_id: self.options.lex.source_id,
            span,
        });
    }
    fn limit(&self) -> usize {
        self.options.max_expression_depth.clamp(1, 128)
    }
    fn boundary(&self) -> bool {
        matches!(
            self.current(),
            K::Eof
                | K::Newline
                | K::Dedent
                | K::Comma
                | K::RightParen
                | K::RightBracket
                | K::RightBrace
                | K::Colon
                | K::Semicolon
        ) || self.keyword("else")
    }
    fn recover(&mut self) {
        self.start(K::ErrorNode);
        while !self.boundary() {
            self.bump();
        }
        self.finish();
    }
    fn name(&self) -> bool {
        self.current().is_identifier(self.text())
    }
    fn expression(&mut self, minimum: u8) -> Operand {
        self.trivia();
        let checkpoint = self.builder.checkpoint();
        if self.depth >= self.limit() {
            self.error("PAR005", "expression exceeds the configured depth limit");
            self.recover();
            return Operand {
                kind: K::ErrorNode,
                height: 1,
                is_string: false,
            };
        }
        self.depth += 1;
        let mut left = self.prefix();
        while !self.lambda_ended {
            let Some((power, kind)) = self.infix() else {
                break;
            };
            if power < minimum {
                break;
            }
            if left.height + self.depth >= self.limit() {
                self.error("PAR005", "expression exceeds the configured depth limit");
                self.recover();
                break;
            }
            self.wrap(checkpoint, kind);
            let mut height = left.height;
            match kind {
                K::CallExpr => {
                    if !matches!(left.kind, K::NameExpr | K::SuperExpr | K::MemberExpr) {
                        self.error(
                            "PAR006",
                            "call requires a name or member; use .call() for a Callable",
                        );
                    }
                    self.start(K::ArgumentList);
                    self.bump();
                    height = height.max(1 + self.list(K::RightParen));
                    self.finish();
                }
                K::MemberExpr => {
                    self.bump();
                    if self.current().is_node_name(self.text()) {
                        self.bump();
                    } else {
                        self.error("PAR004", "expected member name after dot");
                    }
                    if left.kind == K::SuperExpr && self.current() != K::LeftParen {
                        self.error("PAR004", "expected call after super method name");
                    }
                }
                K::SubscriptExpr => {
                    self.bump();
                    height = height.max(self.expression(0).height);
                    self.expect(K::RightBracket, "expected closing bracket after subscript");
                }
                K::CastExpr | K::TypeTestExpr => {
                    self.bump();
                    if kind == K::TypeTestExpr && self.keyword("not") {
                        self.bump();
                    }
                    height = height.max(self.type_ref(false));
                }
                K::TernaryExpr => {
                    self.bump();
                    height = height.max(self.expression(TERNARY).height);
                    if self.keyword("else") {
                        self.bump();
                    } else {
                        self.error("PAR004", "expected else after ternary condition");
                    }
                    height = height.max(self.expression(TERNARY).height);
                }
                _ => {
                    let not_in = self.keyword("not");
                    self.bump();
                    if not_in {
                        if self.keyword("in") {
                            self.bump();
                        } else {
                            self.error("PAR004", "expected in after not in membership test");
                        }
                    }
                    height = height.max(self.expression(power + 1).height);
                }
            }
            self.finish();
            left = Operand {
                kind,
                height: height + 1,
                is_string: false,
            };
        }
        self.depth -= 1;
        left
    }
    fn wrap(&mut self, checkpoint: Checkpoint, kind: K) {
        self.builder
            .start_node_at(checkpoint, GdscriptLanguage::kind_to_raw(kind));
    }
    fn prefix(&mut self) -> Operand {
        // Brackets in a value expression are always subscripts. Collection
        // type arguments are only parsed at explicit type sites (as/is and
        // declarations), matching the pinned engine's Pratt rules.
        let (kind, binding) = match self.current() {
            K::Plus | K::Minus => (K::UnaryExpr, Some(SIGN)),
            K::Tilde => (K::UnaryExpr, Some(BIT_NOT)),
            K::Bang => (K::UnaryExpr, Some(LOGIC_NOT)),
            K::Keyword if self.keyword("not") => (K::UnaryExpr, Some(LOGIC_NOT)),
            K::Keyword if self.keyword("await") => (K::AwaitExpr, Some(AWAIT)),
            K::Number | K::String | K::StringName | K::NodePath => (K::LiteralExpr, None),
            K::Keyword if matches!(self.text(), "true" | "false" | "null") => {
                (K::LiteralExpr, None)
            }
            K::Keyword if self.keyword("self") => (K::SelfExpr, None),
            K::Keyword if self.keyword("super") => (K::SuperExpr, None),
            _ if self.name() => (K::NameExpr, None),
            K::LeftParen => (K::GroupExpr, None),
            K::LeftBracket => (K::ArrayExpr, None),
            K::LeftBrace => (K::DictionaryExpr, None),
            K::Dollar | K::Percent => (K::NodeExpr, None),
            K::Keyword if self.keyword("preload") => (K::CallExpr, None),
            K::Keyword if self.keyword("func") => (K::LambdaExpr, None),
            _ => (K::ErrorNode, None),
        };
        self.start(kind);
        let mut height = 1;
        let mut operand_kind = kind;
        let mut is_string = self.current() == K::String;
        if let Some(binding) = binding {
            self.bump();
            height += self.expression(binding).height;
        } else {
            match kind {
                K::GroupExpr => {
                    self.bump();
                    let inner = self.expression(0);
                    height += inner.height;
                    operand_kind = inner.kind;
                    is_string = inner.is_string;
                    self.expect(K::RightParen, "expected closing parenthesis after group");
                }
                K::ArrayExpr => {
                    self.bump();
                    height += self.list(K::RightBracket);
                }
                K::DictionaryExpr => {
                    self.bump();
                    height += self.dictionary();
                }
                K::NodeExpr => self.node_path(),
                K::LambdaExpr => {
                    self.function(true);
                }
                K::CallExpr => {
                    self.bump();
                    self.expect(K::LeftParen, "expected opening parenthesis after preload");
                    height += self.expression(0).height;
                    self.eat(K::Comma);
                    self.expect(
                        K::RightParen,
                        "expected closing parenthesis after preload path",
                    );
                }
                K::ErrorNode => {
                    self.error("PAR001", "expected expression");
                    if !self.boundary() {
                        self.bump();
                    }
                }
                _ => {
                    let is_super = self.keyword("super");
                    self.bump();
                    if is_super && !matches!(self.current(), K::LeftParen | K::Dot) {
                        self.error("PAR004", "expected parenthesis or dot after super");
                    }
                }
            }
        }
        self.finish();
        Operand {
            kind: operand_kind,
            height,
            is_string,
        }
    }
    fn infix(&self) -> Option<(u8, K)> {
        let power = match self.current() {
            K::LeftParen => return Some((POSTFIX, K::CallExpr)),
            K::Dot => return Some((POSTFIX + 1, K::MemberExpr)),
            K::LeftBracket => return Some((POSTFIX + 2, K::SubscriptExpr)),
            K::OrOr => 4,
            K::AndAnd => 5,
            K::EqualEqual | K::NotEqual | K::Less | K::LessEqual | K::Greater | K::GreaterEqual => {
                8
            }
            K::Pipe => 9,
            K::Caret => 10,
            K::Ampersand => 11,
            K::ShiftLeft | K::ShiftRight => 12,
            K::Plus | K::Minus => 13,
            K::Star | K::Slash | K::Percent => 14,
            K::Power => 17,
            K::Keyword => match self.text() {
                "as" => return Some((CAST, K::CastExpr)),
                "is" => return Some((18, K::TypeTestExpr)),
                "if" => return Some((TERNARY, K::TernaryExpr)),
                "or" => 4,
                "and" => 5,
                "in" | "not" => 7,
                _ => return None,
            },
            _ => return None,
        };
        Some((power, K::BinaryExpr))
    }
    fn list(&mut self, close: K) -> usize {
        let mut height = 0;
        while self.current() != close && self.current() != K::Eof {
            height = height.max(self.expression(0).height);
            self.lambda_ended = false;
            if self.eat(K::Comma) {
                continue;
            }
            if self.current() == close || self.boundary() {
                break;
            }
            self.error("PAR004", "expected comma between elements");
            self.recover();
            if !self.eat(K::Comma) {
                break;
            }
        }
        self.expect(close, "expected closing delimiter after elements");
        height
    }
    fn dictionary(&mut self) -> usize {
        let mut style = None;
        let mut height = 0;
        while !matches!(self.current(), K::RightBrace | K::Eof) {
            self.start(K::DictionaryEntry);
            let key = self.expression(0);
            self.lambda_ended = false;
            height = height.max(key.height + 1);
            let separator = self.current();
            if matches!(separator, K::Colon | K::Equal) {
                if style.is_some_and(|style| style != separator) {
                    self.error("PAR007", "cannot mix dictionary separator styles");
                }
                style.get_or_insert(separator);
                if separator == K::Equal && !(key.kind == K::NameExpr || key.is_string) {
                    self.error(
                        "PAR007",
                        "expected name or string as dictionary key with equals",
                    );
                }
                self.bump();
            } else {
                self.error("PAR004", "expected colon or equals after dictionary key");
            }
            height = height.max(self.expression(0).height + 1);
            self.finish();
            self.lambda_ended = false;
            if self.eat(K::Comma) {
                continue;
            }
            if self.current() != K::RightBrace && !self.boundary() {
                self.error("PAR004", "expected comma between dictionary entries");
                self.recover();
                if self.eat(K::Comma) {
                    continue;
                }
            }
            break;
        }
        self.expect(K::RightBrace, "expected closing brace after dictionary");
        height
    }
    fn type_ref(&mut self, nested: bool) -> usize {
        self.start(K::TypeRef);
        let mut height = 1;
        if self.depth >= self.limit() {
            self.error("PAR005", "type exceeds the configured depth limit");
            self.recover();
        } else if self.name() {
            self.depth += 1;
            self.bump();
            if self.eat(K::LeftBracket) {
                if nested {
                    self.error(
                        "PAR008",
                        "nested typed collections are not supported by the baseline",
                    );
                }
                loop {
                    height = height.max(1 + self.type_ref(true));
                    if !self.eat(K::Comma) {
                        break;
                    }
                }
                self.expect(
                    K::RightBracket,
                    "expected closing bracket after type arguments",
                );
            } else {
                while self.eat(K::Dot) {
                    if self.name() {
                        self.bump();
                    } else {
                        self.error("PAR004", "expected type name after dot");
                        break;
                    }
                }
            }
            self.depth -= 1;
        } else {
            self.error("PAR004", "expected type name");
        }
        self.finish();
        height
    }
    fn node_path(&mut self) {
        let unique = self.current() == K::Percent;
        self.bump();
        if !unique {
            self.eat(K::Slash);
            self.eat(K::Percent);
        }
        loop {
            let is_name = self.current().is_node_name(self.text());
            if is_name || self.current() == K::String {
                self.bump();
            } else {
                self.error("PAR004", "expected node name or string in node path");
                break;
            }
            if !self.eat(K::Slash) {
                if self.current() == K::Percent {
                    self.error("PAR004", "unique node marker must start a path segment");
                }
                break;
            }
            self.eat(K::Percent);
        }
    }
}
