//! Declarations and statement suites share the expression parser's token cursor.
use super::{K, Parser};

impl Parser<'_> {
    pub(super) fn items(&mut self, class_body: bool) {
        while !matches!(self.current(), K::Eof | K::Dedent) {
            if self.enclosed_lambda_end() {
                break;
            }
            if self.eat(K::Newline) {
                continue;
            }
            if self.current() == K::Semicolon {
                self.error("PAR009", "semicolon requires a preceding statement");
                self.start(K::ErrorNode);
                self.bump();
                self.finish();
                continue;
            }
            let before = self.cursor;
            self.item(class_body);
            if self.cursor == before {
                self.error("PAR009", "unexpected token in block");
                self.start(K::ErrorNode);
                self.bump();
                self.finish();
            }
        }
    }

    fn enclosed_lambda_end(&self) -> bool {
        self.lambda_depth > 0
            && self.lexer.borrow().suite_is_enclosed()
            && matches!(
                self.current(),
                K::Comma | K::RightParen | K::RightBracket | K::RightBrace
            )
    }

    fn identifier(&mut self) {
        if self.name() {
            self.bump();
        } else {
            self.error("PAR004", "expected identifier");
        }
    }
    fn word(&mut self, word: &str) {
        if self.keyword(word) {
            self.bump();
        } else {
            self.error("PAR004", &format!("expected {word}"));
        }
    }
    fn statement_end(&self) -> bool {
        matches!(
            self.current(),
            K::Newline | K::Semicolon | K::Eof | K::Dedent
        ) || (self.lambda_depth > 0
            && matches!(
                self.current(),
                K::Comma | K::RightParen | K::RightBracket | K::RightBrace
            ))
    }
    fn end_statement(&mut self) {
        if std::mem::take(&mut self.lambda_ended) {
            return;
        }
        if self.eat(K::Semicolon) || self.eat(K::Newline) {
            // Godot consumes a run of separators after a complete statement.
            // A semicolon at the start of a block still has no such statement.
            while self.eat(K::Semicolon) || self.eat(K::Newline) {}
            return;
        }
        if self.statement_end() {
            return;
        }
        self.error("PAR009", "expected end of statement");
        self.start(K::ErrorNode);
        while !self.statement_end() {
            self.bump();
        }
        self.finish();
        self.eat(K::Newline);
    }
    fn invalid_context(&mut self, valid: bool, message: &str) {
        if !valid {
            self.error("PAR010", message);
        }
    }
    fn item(&mut self, class_body: bool) {
        if self.depth >= self.limit() {
            self.error("PAR005", "block exceeds the configured depth limit");
            self.start(K::ErrorNode);
            // At a depth limit consume the remaining block iteratively, including
            // nested indentation, without creating a recursively deep error tree.
            let mut nesting = 0;
            while self.current() != K::Eof {
                if self.current() == K::Dedent {
                    if nesting == 0 {
                        break;
                    }
                    nesting -= 1;
                } else if self.current() == K::Indent {
                    nesting += 1;
                }
                self.bump();
            }
            self.finish();
            return;
        }
        self.depth += 1;
        self.trivia();
        if self.current() == K::Annotation {
            self.annotation();
            self.depth -= 1;
            return;
        }
        let checkpoint = self.builder.checkpoint();
        let is_static = self.keyword("static");
        if is_static {
            self.invalid_context(class_body, "static declarations require a class body");
            self.bump();
            self.invalid_context(
                self.keyword("func") || self.keyword("var"),
                "static requires a function or variable declaration",
            );
        }
        let keyword = if self.current() == K::Keyword {
            self.text()
        } else {
            ""
        };
        let kind = match keyword {
            "class" => K::ClassDecl,
            "extends" => K::ExtendsDecl,
            "class_name" => K::ClassNameDecl,
            "var" => K::VariableDecl,
            "const" => K::ConstantDecl,
            "enum" => K::EnumDecl,
            "signal" => K::SignalDecl,
            "func" if class_body => K::FunctionDecl,
            "if" => K::IfStmt,
            "for" => K::ForStmt,
            "while" => K::WhileStmt,
            "match" => K::MatchStmt,
            "return" => K::ReturnStmt,
            "assert" => K::AssertStmt,
            "break" => K::BreakStmt,
            "continue" => K::ContinueStmt,
            "pass" => K::PassStmt,
            "breakpoint" => K::BreakpointStmt,
            _ => K::ExpressionStmt,
        };
        if kind != K::ExpressionStmt {
            self.wrap(checkpoint, kind);
        }
        match kind {
            K::ClassDecl
            | K::ExtendsDecl
            | K::ClassNameDecl
            | K::EnumDecl
            | K::SignalDecl
            | K::FunctionDecl => {
                self.invalid_context(class_body, "declaration requires a class body");
            }
            K::VariableDecl | K::ConstantDecl | K::PassStmt => {}
            _ => self.invalid_context(
                !class_body || self.current() == K::String,
                "statement requires a function body",
            ),
        }
        let compound = match kind {
            K::ClassDecl => {
                self.bump();
                self.identifier();
                if self.keyword("extends") {
                    self.bump();
                    self.base_type();
                }
                self.suite(true);
                true
            }
            K::ExtendsDecl => {
                self.bump();
                self.base_type();
                false
            }
            K::ClassNameDecl => {
                self.bump();
                self.identifier();
                if self.keyword("extends") {
                    self.start(K::ExtendsDecl);
                    self.bump();
                    self.base_type();
                    self.finish();
                }
                false
            }
            K::VariableDecl | K::ConstantDecl => self.variable(kind == K::ConstantDecl, class_body),
            K::EnumDecl => {
                self.enumeration();
                false
            }
            K::SignalDecl => {
                self.bump();
                self.identifier();
                if self.current() == K::LeftParen {
                    self.parameters(false);
                }
                false
            }
            K::FunctionDecl => {
                self.function(false);
                true
            }
            K::IfStmt => {
                self.if_statement();
                true
            }
            K::ForStmt | K::WhileStmt => {
                self.bump();
                if kind == K::ForStmt {
                    self.identifier();
                    if self.eat(K::Colon) {
                        self.type_ref(false);
                    }
                    self.word("in");
                }
                self.expression(0);
                self.loop_depth += 1;
                self.suite(false);
                self.loop_depth -= 1;
                true
            }
            K::MatchStmt => {
                self.match_statement();
                true
            }
            K::ReturnStmt => {
                self.invalid_context(self.function_depth > 0, "return requires a function body");
                self.bump();
                if !self.statement_end() {
                    self.expression(0);
                }
                false
            }
            K::AssertStmt => {
                self.bump();
                self.expect(K::LeftParen, "expected parenthesis after assert");
                self.expression(0);
                if self.eat(K::Comma) && self.current() != K::RightParen {
                    self.expression(0);
                    self.eat(K::Comma);
                }
                self.expect(K::RightParen, "expected closing parenthesis after assert");
                false
            }
            K::BreakStmt | K::ContinueStmt => {
                self.invalid_context(
                    self.loop_depth > 0,
                    "break/continue requires an enclosing loop in this function",
                );
                self.bump();
                false
            }
            K::PassStmt | K::BreakpointStmt => {
                self.bump();
                false
            }
            _ => {
                let left = self.expression(0);
                if class_body && !(left.kind == K::LiteralExpr && left.is_string) {
                    self.error(
                        "PAR010",
                        "only comment strings may appear as expressions in a class body",
                    );
                }
                if left.kind == K::LambdaExpr {
                    self.error("PAR010", "standalone lambdas must be assigned or called");
                }
                if self.assignment() {
                    self.invalid_context(
                        matches!(left.kind, K::NameExpr | K::MemberExpr | K::SubscriptExpr),
                        "invalid assignment target",
                    );
                    self.wrap(checkpoint, K::AssignmentStmt);
                    self.bump();
                    self.expression(0);
                } else {
                    self.wrap(checkpoint, K::ExpressionStmt);
                }
                false
            }
        };
        if class_body && kind == K::ExpressionStmt {
            // Class comment strings require a newline, unlike expression
            // statements inside functions, which may end with a semicolon.
            self.expect(K::Newline, "expected newline after class comment string");
        } else if !compound {
            self.end_statement();
        }
        self.finish();
        self.depth -= 1;
    }

    fn annotation(&mut self) {
        self.start(K::AnnotationNode);
        self.bump();
        if self.eat(K::LeftParen) {
            self.list(K::RightParen);
        }
        self.eat(K::Newline);
        self.finish();
    }
    fn base_type(&mut self) {
        if self.current() == K::String {
            self.bump();
            while self.eat(K::Dot) {
                self.identifier();
            }
        } else {
            // A superclass is a dotted name, not a collection type annotation.
            self.start(K::TypeRef);
            self.identifier();
            while self.eat(K::Dot) {
                self.identifier();
            }
            self.finish();
        }
    }
    fn variable(&mut self, constant: bool, class_body: bool) -> bool {
        self.bump();
        self.identifier();
        let typed = self.eat(K::Colon);
        if typed
            && !constant
            && class_body
            && (self.current() == K::Newline || matches!(self.text(), "get" | "set"))
        {
            self.property(true);
            return true;
        }
        if typed && self.current() != K::Equal {
            self.type_ref(false);
        }
        let initialized = self.eat(K::Equal);
        if initialized {
            self.expression(0);
        }
        if constant && !initialized {
            self.error("PAR004", "constant requires an initializer");
        }
        if !constant && class_body && self.current() == K::Colon {
            self.property(false);
            true
        } else {
            false
        }
    }
    fn enumeration(&mut self) {
        self.bump();
        if self.name() {
            self.bump();
        }
        self.expect(K::LeftBrace, "expected brace after enum name");
        while !matches!(self.current(), K::RightBrace | K::Eof) && !self.statement_end() {
            let before = self.cursor;
            self.start(K::EnumEntry);
            self.identifier();
            if self.eat(K::Equal) {
                self.expression(0);
            }
            self.finish();
            if !self.eat(K::Comma) || self.cursor == before {
                break;
            }
        }
        self.expect(K::RightBrace, "expected closing brace after enum entries");
    }
    fn parameters(&mut self, defaults: bool) {
        self.start(K::ParameterList);
        self.expect(
            K::LeftParen,
            "expected opening parenthesis before parameters",
        );
        let mut optional = false;
        let mut rest_seen = false;
        while !matches!(self.current(), K::RightParen | K::Eof) && !self.statement_end() {
            let before = self.cursor;
            self.start(K::Parameter);
            let rest = self.eat(K::Ellipsis);
            if rest_seen {
                self.error("PAR011", "rest parameter must be last");
            }
            self.identifier();
            if self.eat(K::Colon) && self.current() != K::Equal {
                self.type_ref(false);
            }
            if self.eat(K::Equal) {
                if !defaults || rest {
                    self.error("PAR011", "default value is not allowed for this parameter");
                }
                optional = true;
                self.expression(0);
                self.lambda_ended = false;
            } else if optional && !rest {
                self.error("PAR011", "required parameter follows an optional parameter");
            }
            if rest && !defaults {
                self.error("PAR011", "rest parameter is not allowed here");
            }
            rest_seen |= rest;
            self.finish();
            if !self.eat(K::Comma) || self.cursor == before {
                break;
            }
        }
        self.expect(
            K::RightParen,
            "expected closing parenthesis after parameters",
        );
        self.finish();
    }
    pub(super) fn function(&mut self, lambda: bool) {
        self.bump(); // func
        if !lambda || self.name() {
            self.identifier();
        }
        self.parameters(true);
        if self.eat(K::Arrow) {
            if self.keyword("void") {
                self.start(K::TypeRef);
                self.bump();
                self.finish();
            } else {
                self.type_ref(false);
            }
        }
        // Bodyless named methods are checked against their annotations after
        // building the CST, so annotation binding does not affect recovery.
        if !lambda && self.current() != K::Colon {
            self.end_statement();
            return;
        }
        let previous_loop = self.loop_depth;
        let previous_lambda = self.lambda_depth;
        self.loop_depth = 0;
        self.function_depth += 1;
        if lambda {
            self.lambda_depth += 1;
            // No body token has been requested yet: the colon is still current.
            self.lexer.borrow_mut().push_suite();
        } else {
            self.lambda_depth = 0;
        }
        self.suite(false);
        if lambda {
            // Indentation scanning can queue several dedents at once. In an
            // enclosing expression they belong to the saved lambda context.
            let enclosed = self.lexer.borrow().suite_is_enclosed();
            if enclosed {
                while self.cursor < self.lexer.borrow().tokens.len()
                    && matches!(
                        self.token_at(self.cursor).kind,
                        K::Dedent | K::Indent | K::Newline
                    )
                {
                    self.raw_bump();
                }
            }
            self.lexer.borrow_mut().pop_suite();
        }
        self.loop_depth = previous_loop;
        self.lambda_depth = previous_lambda;
        self.function_depth -= 1;
        self.lambda_ended = lambda;
    }
    fn suite(&mut self, class_body: bool) {
        self.start(K::Suite);
        self.expect(K::Colon, "expected colon before block");
        if self.eat(K::Newline) {
            if self.eat(K::Indent) {
                let before = self.cursor;
                self.items(class_body);
                if self.cursor == before {
                    self.error("PAR004", "expected statement in block");
                }
                // An enclosing delimiter may terminate a block lambda on its
                // last statement's physical line, before any dedent is lexed.
                if !self.enclosed_lambda_end() {
                    self.expect(K::Dedent, "expected dedent after block");
                }
            } else {
                self.error("PAR004", "expected indented block");
            }
        } else if !matches!(self.current(), K::Eof | K::Dedent) {
            if class_body && self.keyword("extends") {
                self.item(true);
            }
            loop {
                if matches!(self.current(), K::Newline | K::Eof | K::Dedent) {
                    break;
                }
                let before = self.cursor;
                self.item(class_body);
                if class_body
                    || self.cursor == before
                    || self.lambda_ended
                    || !self.previous_semicolon()
                {
                    break;
                }
                if matches!(self.current(), K::Newline | K::Eof | K::Dedent) {
                    break;
                }
            }
        } else {
            self.error("PAR004", "expected statement in block");
        }
        self.finish();
    }
    fn previous_semicolon(&self) -> bool {
        (0..self.cursor)
            .rev()
            .find(|&i| !self.token_at(i).kind.is_trivia())
            .is_some_and(|i| self.token_at(i).kind == K::Semicolon)
    }
    fn if_statement(&mut self) {
        self.bump();
        self.expression(0);
        self.suite(false);
        while self.keyword("elif") {
            self.start(K::ElifClause);
            self.bump();
            self.expression(0);
            self.suite(false);
            self.finish();
        }
        if self.keyword("else") {
            self.start(K::ElseClause);
            self.bump();
            self.suite(false);
            self.finish();
        }
    }
    fn assignment(&self) -> bool {
        matches!(
            self.current(),
            K::Equal
                | K::PlusEqual
                | K::MinusEqual
                | K::StarEqual
                | K::PowerEqual
                | K::SlashEqual
                | K::PercentEqual
                | K::ShiftLeftEqual
                | K::ShiftRightEqual
                | K::AmpersandEqual
                | K::PipeEqual
                | K::CaretEqual
        )
    }
    fn property(&mut self, colon_consumed: bool) {
        self.start(K::PropertyBlock);
        if !colon_consumed {
            self.bump();
        }
        let multiline = self.eat(K::Newline);
        if multiline {
            self.expect(K::Indent, "expected indented property accessors");
        }
        let mut getter = false;
        let mut setter = false;
        let mut delegated = None;
        loop {
            let set = self.text() == "set";
            if !set && self.text() != "get" {
                self.error("PAR004", "expected get or set accessor");
                if !multiline || matches!(self.current(), K::Dedent | K::Eof) {
                    break;
                }
                // Keep malformed accessors in this property, including any
                // indented body, then retry at the next sibling accessor.
                self.start(K::ErrorNode);
                while !matches!(self.current(), K::Newline | K::Dedent | K::Eof) {
                    self.bump();
                }
                self.eat(K::Newline);
                if self.eat(K::Indent) {
                    let mut nesting = 1;
                    while nesting > 0 && self.current() != K::Eof {
                        match self.current() {
                            K::Indent => nesting += 1,
                            K::Dedent => nesting -= 1,
                            _ => {}
                        }
                        self.bump();
                    }
                }
                self.finish();
                if matches!(self.current(), K::Dedent | K::Eof) {
                    break;
                }
                continue;
            }
            self.start(if set { K::Setter } else { K::Getter });
            if (set && setter) || (!set && getter) {
                self.error("PAR012", "duplicate property accessor");
            }
            if set {
                setter = true;
            } else {
                getter = true;
            }
            self.bump();
            let pointer = self.current() == K::Equal;
            if delegated.is_some_and(|previous| previous != pointer) {
                self.error(
                    "PAR012",
                    "cannot mix delegated accessors and accessor bodies",
                );
            }
            delegated.get_or_insert(pointer);
            if self.eat(K::Equal) {
                self.identifier();
            } else {
                if !multiline {
                    self.error(
                        "PAR012",
                        "accessor bodies require an indented property block",
                    );
                }
                if set {
                    self.expect(K::LeftParen, "expected setter parameter");
                    self.identifier();
                    self.expect(
                        K::RightParen,
                        "expected closing parenthesis after setter parameter",
                    );
                }
                if !set && self.eat(K::LeftParen) {
                    self.expect(K::RightParen, "getter cannot have parameters");
                }
                self.function_depth += 1;
                self.suite(false);
                self.function_depth -= 1;
            }
            self.finish();
            if multiline {
                self.eat(K::Newline);
            }
            if self.eat(K::Comma) {
                if !pointer {
                    self.error("PAR012", "accessor bodies cannot be separated by commas");
                }
                if multiline {
                    self.eat(K::Newline);
                }
                continue;
            }
            if pointer && multiline && !matches!(self.current(), K::Dedent | K::Eof) {
                self.error("PAR012", "delegated accessors require a comma separator");
            }
            if !multiline || matches!(self.current(), K::Dedent | K::Eof) {
                break;
            }
        }
        if multiline {
            self.expect(K::Dedent, "expected dedent after property accessors");
        } else {
            self.end_statement();
        }
        self.finish();
    }
    fn match_statement(&mut self) {
        self.bump();
        self.expression(0);
        self.expect(K::Colon, "expected colon after match value");
        self.expect(K::Newline, "expected newline before match branches");
        if !self.eat(K::Indent) {
            self.error("PAR004", "expected indented match branches");
            return;
        }
        let mut branches = 0;
        while !matches!(self.current(), K::Dedent | K::Eof) {
            if self.keyword("pass") {
                self.start(K::PassStmt);
                self.bump();
                self.expect(K::Newline, "expected newline after match pass");
                self.finish();
                branches += 1;
                continue;
            }
            if self.current() == K::Annotation {
                self.annotation();
                continue;
            }
            let before = self.cursor;
            self.start(K::MatchBranch);
            let mut patterns = 0;
            let mut has_bind = false;
            loop {
                has_bind |= self.pattern();
                patterns += 1;
                if !self.eat(K::Comma) {
                    break;
                }
            }
            if patterns > 1 && has_bind {
                self.error(
                    "PAR013",
                    "variable binds cannot be combined with alternative patterns",
                );
            }
            if self.keyword("when") {
                self.bump();
                self.expression(0);
            }
            self.suite(false);
            self.finish();
            branches += 1;
            if self.cursor == before {
                self.bump();
            }
        }
        if branches == 0 {
            self.error("PAR004", "expected match branch");
        }
        self.expect(K::Dedent, "expected dedent after match branches");
    }
    fn pattern(&mut self) -> bool {
        self.start(K::Pattern);
        let mut has_bind = false;
        if self.depth >= self.limit() {
            self.error("PAR005", "pattern exceeds the configured depth limit");
            self.recover();
        } else {
            self.depth += 1;
            match self.current() {
                K::Underscore => self.bump(),
                K::Keyword if self.keyword("var") => {
                    self.bump();
                    self.identifier();
                    has_bind = true;
                }
                K::LeftBracket | K::LeftBrace => {
                    let dictionary = self.current() == K::LeftBrace;
                    let close = if dictionary {
                        K::RightBrace
                    } else {
                        K::RightBracket
                    };
                    self.bump();
                    let mut rest_seen = false;
                    while self.current() != close && self.current() != K::Eof {
                        let before = self.cursor;
                        if rest_seen && (!dictionary || self.current() == K::DotDot) {
                            self.error("PAR013", "rest pattern must be last");
                        }
                        if self.eat(K::DotDot) {
                            rest_seen = true;
                        } else if dictionary {
                            self.expression(0);
                            if self.eat(K::Colon) {
                                // The pinned parsers allow key-only entries after
                                // rest, but reject subsequent value patterns.
                                if rest_seen {
                                    self.error(
                                        "PAR013",
                                        "rest pattern must precede no value patterns",
                                    );
                                }
                                has_bind |= self.pattern();
                            }
                        } else {
                            has_bind |= self.pattern();
                        }
                        if !self.eat(K::Comma) || self.cursor == before {
                            break;
                        }
                    }
                    self.expect(close, "expected closing delimiter after pattern");
                }
                _ => {
                    self.expression(0);
                }
            }
            self.depth -= 1;
        }
        self.finish();
        has_bind
    }
}
