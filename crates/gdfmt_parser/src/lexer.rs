use crate::{
    LexOptions, LexResult,
    unicode::{is_identifier_continue, is_identifier_start},
};
use gdfmt_syntax::{LosslessTree, SyntaxKind as K, Token};
use gdfmt_text::{Diagnostic, Span};

pub fn lex(source: &str, options: &LexOptions) -> LexResult {
    match Lexer::new(source, options) {
        Ok(mut lexer) => {
            while !lexer.done {
                lexer.step();
            }
            let tree = LosslessTree::from_tokens(source, &lexer.tokens)
                .expect("lexer tokens partition the source");
            LexResult {
                tokens: lexer.tokens,
                tree: Some(tree),
                diagnostics: lexer.diagnostics,
            }
        }
        Err(result) => result,
    }
}

pub(crate) struct Lexer<'a> {
    source: &'a str,
    options: &'a LexOptions,
    offset: usize,
    pub(crate) tokens: Vec<Token>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    indents: Vec<usize>,
    indent_char: Option<u8>,
    brackets: Vec<(K, usize)>,
    overflow_depth: usize,
    at_line_start: bool,
    continuation: bool,
    line_has_code: bool,
    suite_floors: Vec<(usize, Option<Vec<usize>>)>,
    pub(crate) done: bool,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(source: &'a str, options: &'a LexOptions) -> Result<Self, LexResult> {
        if source.len() > options.max_source_bytes || source.len() > u32::MAX as usize {
            return Err(LexResult {
                tokens: vec![],
                tree: None,
                diagnostics: vec![Diagnostic::lexical(
                    "LEX001",
                    "source exceeds the configured byte limit",
                    options.source_id,
                    Span::empty(0),
                )],
            });
        }
        let mut lexer = Self {
            source,
            options,
            offset: 0,
            tokens: vec![],
            diagnostics: vec![],
            indents: vec![0],
            indent_char: None,
            brackets: vec![],
            overflow_depth: 0,
            at_line_start: true,
            continuation: false,
            line_has_code: false,
            suite_floors: vec![],
            done: false,
        };
        if source.starts_with('\u{feff}') {
            lexer.offset = 3;
            lexer.emit(K::Bom, 0);
        }
        Ok(lexer)
    }

    // A lambda suite temporarily ignores delimiters belonging to its enclosing
    // expression. Delimiters opened inside the body still suppress newlines.
    pub(crate) fn push_suite(&mut self) {
        let saved = self.in_brackets().then(|| self.indents.clone());
        self.suite_floors.push((self.brackets.len(), saved));
    }
    pub(crate) fn suite_is_enclosed(&self) -> bool {
        self.suite_floors
            .last()
            .is_some_and(|context| context.1.is_some())
    }
    pub(crate) fn pop_suite(&mut self) {
        if let Some((_, Some(saved))) = self.suite_floors.pop() {
            self.indents = saved;
        }
    }

    fn rest(&self) -> &str {
        &self.source[self.offset..]
    }
    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }
    fn bump(&mut self) {
        if let Some(ch) = self.peek() {
            self.offset += ch.len_utf8();
        }
    }
    fn emit(&mut self, kind: K, start: usize) {
        self.tokens.push(Token {
            kind,
            span: Span::new(start as u32, self.offset as u32).expect("ordered lexer offsets"),
        });
    }
    fn virtual_token(&mut self, kind: K) {
        self.emit(kind, self.offset);
    }
    fn error(&mut self, code: &str, message: &str, start: usize, end: usize) {
        let count = self.diagnostics.len();
        if count < self.options.max_diagnostics {
            self.diagnostics.push(Diagnostic::lexical(
                code,
                message,
                self.options.source_id,
                Span::new(start as u32, end as u32).expect("ordered diagnostic offsets"),
            ));
        } else if count == self.options.max_diagnostics {
            self.diagnostics.push(Diagnostic::lexical(
                "LEX002",
                "further lexical diagnostics suppressed",
                self.options.source_id,
                Span::empty(start as u32),
            ));
        }
    }
    fn in_brackets(&self) -> bool {
        self.brackets.len() > self.suite_floors.last().map_or(0, |context| context.0)
            || self.overflow_depth > 0
    }

    pub(crate) fn step(&mut self) {
        if self.done {
            return;
        }
        if self.at_line_start && self.offset < self.source.len() {
            self.indentation();
            if self
                .tokens
                .last()
                .is_some_and(|t| matches!(t.kind, K::Indent | K::Dedent))
            {
                return;
            }
        }
        if self.offset < self.source.len() {
            let start = self.offset;
            let ch = self.peek().expect("not at EOF");
            match ch {
                ' ' | '\t' => {
                    while matches!(self.peek(), Some(' ' | '\t')) {
                        self.bump();
                    }
                    self.emit(K::Whitespace, start);
                }
                '\r' | '\n' => self.newline(),
                '#' => {
                    while !matches!(self.peek(), None | Some('\r' | '\n')) {
                        self.bump();
                    }
                    self.emit(K::Comment, start);
                }
                '\\' if self.rest().starts_with("\\\n") || self.rest().starts_with("\\\r\n") => {
                    self.offset += if self.rest().starts_with("\\\r\n") {
                        3
                    } else {
                        2
                    };
                    self.emit(K::LineContinuation, start);
                    self.continuation = true;
                    self.at_line_start = true;
                    if self.offset == self.source.len() {
                        self.error(
                            "LEX009",
                            "expected a continued line after backslash",
                            start,
                            self.offset,
                        );
                    }
                }
                '"' | '\'' => {
                    self.line_has_code = true;
                    self.string(K::String, false, false);
                }
                'r' | '&' | '^' if matches!(self.rest().as_bytes().get(1), Some(b'\'' | b'"')) => {
                    self.line_has_code = true;
                    let kind = match ch {
                        '&' => K::StringName,
                        '^' => K::NodePath,
                        _ => K::String,
                    };
                    self.string(kind, ch == 'r', true);
                }
                '@' => {
                    self.line_has_code = true;
                    self.bump();
                    if self.peek().is_some_and(is_identifier_start) {
                        self.identifier_tail();
                    } else {
                        self.error(
                            "LEX010",
                            "expected an annotation name after @",
                            start,
                            self.offset,
                        );
                    }
                    self.emit(K::Annotation, start);
                }
                '+' | '-'
                    if self
                        .rest()
                        .as_bytes()
                        .get(1)
                        .is_some_and(u8::is_ascii_digit)
                        && !self.can_precede_binary() =>
                {
                    self.line_has_code = true;
                    self.number();
                }
                '0'..='9' => {
                    self.line_has_code = true;
                    self.number();
                }
                '.' if self
                    .rest()
                    .as_bytes()
                    .get(1)
                    .is_some_and(u8::is_ascii_digit) =>
                {
                    self.line_has_code = true;
                    self.number();
                }
                _ if is_identifier_start(ch) => {
                    self.line_has_code = true;
                    self.identifier_tail();
                    let text = &self.source[start..self.offset];
                    self.emit(
                        if text == "_" {
                            K::Underscore
                        } else if is_keyword(text) {
                            K::Keyword
                        } else {
                            K::Identifier
                        },
                        start,
                    );
                }
                _ => {
                    self.line_has_code = true;
                    if !self.operator() {
                        self.bump();
                        self.emit(K::Error, start);
                        self.error("LEX003", "unexpected character", start, self.offset);
                    }
                }
            }
        } else {
            if self.line_has_code {
                self.virtual_token(K::Newline);
            }
            while self.indents.len() > 1 {
                self.indents.pop();
                self.virtual_token(K::Dedent);
            }
            for (_, start) in self.brackets.clone() {
                self.error("LEX008", "unclosed delimiter", start, start + 1);
            }
            self.virtual_token(K::Eof);
            self.done = true;
        }
    }

    fn indentation(&mut self) {
        self.at_line_start = false;
        let continued = self.continuation || self.in_brackets();
        let start = self.offset;
        let mut width = 0;
        while let Some(ch @ (' ' | '\t')) = self.peek() {
            width += if ch == '\t' { 4 } else { 1 };
            self.bump();
        }
        if self.offset > start {
            self.emit(K::Whitespace, start);
        }
        if matches!(self.peek(), None | Some('#' | '\r' | '\n')) {
            return;
        }
        self.continuation = false;
        if continued {
            return;
        }
        let indent = &self.source.as_bytes()[start..self.offset];
        if let Some(first) = indent.first() {
            if indent.iter().any(|ch| ch != first) {
                self.error(
                    "LEX004",
                    "mixed tabs and spaces in indentation",
                    start,
                    self.offset,
                );
            }
            if self.indent_char.is_some_and(|ch| ch != *first) {
                self.error(
                    "LEX004",
                    "indentation character differs from earlier lines",
                    start,
                    self.offset,
                );
            }
            self.indent_char.get_or_insert(*first);
        }
        let current = *self.indents.last().expect("base indent exists");
        if width > current {
            if self.indents.len() > self.options.max_nesting {
                self.error(
                    "LEX002",
                    "indentation exceeds the configured nesting limit",
                    start,
                    self.offset,
                );
            } else {
                self.indents.push(width);
                self.virtual_token(K::Indent);
            }
        } else if width < current {
            while self.indents.last().is_some_and(|value| *value > width) {
                self.indents.pop();
                self.virtual_token(K::Dedent);
            }
            if self.indents.last().copied() != Some(width) {
                self.error(
                    "LEX005",
                    "dedent does not match an earlier indentation level",
                    start,
                    self.offset,
                );
            }
        }
    }

    fn newline(&mut self) {
        let start = self.offset;
        if self.rest().starts_with("\r\n") {
            self.offset += 2;
        } else {
            self.bump();
            if &self.source[start..self.offset] == "\r" {
                self.error(
                    "LEX003",
                    "carriage return must be followed by a newline",
                    start,
                    self.offset,
                );
            }
        }
        self.emit(K::PhysicalNewline, start);
        if self.line_has_code && !self.in_brackets() && !self.continuation {
            self.virtual_token(K::Newline);
            self.line_has_code = false;
        }
        self.at_line_start = true;
    }

    fn identifier_tail(&mut self) {
        self.bump();
        while self.peek().is_some_and(is_identifier_continue) {
            self.bump();
        }
    }

    fn can_precede_binary(&self) -> bool {
        let Some(token) = self
            .tokens
            .iter()
            .rev()
            .find(|token| !token.kind.is_trivia())
        else {
            return false;
        };
        matches!(
            token.kind,
            K::Identifier
                | K::Number
                | K::String
                | K::StringName
                | K::NodePath
                | K::RightParen
                | K::RightBracket
                | K::RightBrace
        ) || (token.kind == K::Keyword
            && matches!(
                token.span.slice(self.source).unwrap(),
                "self" | "PI" | "TAU" | "INF" | "NAN" | "true" | "false" | "null"
            ))
    }

    fn number(&mut self) {
        let start = self.offset;
        if matches!(self.peek(), Some('+' | '-')) {
            self.bump();
        }
        let base = if self.rest().starts_with("0x") || self.rest().starts_with("0X") {
            16
        } else if self.rest().starts_with("0b") || self.rest().starts_with("0B") {
            2
        } else {
            10
        };
        if base != 10 {
            self.offset += 2;
        }
        let digits = self.offset;
        self.digits(base);
        if base != 10 && (self.offset == digits || self.source.as_bytes()[digits] == b'_') {
            self.error(
                "LEX011",
                "expected digits after numeric base prefix",
                start,
                self.offset,
            );
        }
        if base == 10 {
            if self.peek() == Some('.') && !self.rest().starts_with("..") {
                self.bump();
                if self.peek() == Some('_') {
                    self.error(
                        "LEX011",
                        "unexpected underscore after decimal point",
                        start,
                        self.offset + 1,
                    );
                }
                self.digits(10);
            }
            if matches!(self.peek(), Some('e' | 'E')) {
                self.bump();
                if matches!(self.peek(), Some('+' | '-')) {
                    self.bump();
                }
                if !self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                    self.error("LEX011", "expected exponent digits", start, self.offset);
                }
                self.digits(10);
            }
        }
        if self.peek() == Some('.') && !self.rest().starts_with("..") {
            self.error(
                "LEX011",
                "unexpected decimal point in numeric literal",
                self.offset,
                self.offset + 1,
            );
        }
        if self.peek().is_some_and(is_identifier_continue) {
            self.error("LEX011", "invalid numeric suffix", start, self.offset);
        }
        self.emit(K::Number, start);
    }

    fn digits(&mut self, base: u32) {
        let mut underscore = false;
        while let Some(ch) = self.peek() {
            if ch == '_' {
                if underscore {
                    self.error(
                        "LEX011",
                        "adjacent underscores in number",
                        self.offset,
                        self.offset + 1,
                    );
                }
                underscore = true;
            } else if ch.is_ascii() && ch.is_digit(base) {
                underscore = false;
            } else {
                break;
            }
            self.bump();
        }
    }

    fn string(&mut self, kind: K, raw: bool, prefix: bool) {
        let start = self.offset;
        if prefix {
            self.bump();
        }
        let quote = self.peek().expect("string starts with quote");
        let delimiter = if quote == '"' { "\"\"\"" } else { "'''" };
        let multiline = self.rest().starts_with(delimiter);
        self.offset += if multiline { 3 } else { 1 };
        let mut closed = false;
        let mut lead_surrogate = None;
        while let Some(ch) = self.peek() {
            if matches!(ch, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
            {
                self.error(
                    "LEX012",
                    "escape invisible direction control characters in strings",
                    self.offset,
                    self.offset + ch.len_utf8(),
                );
            }
            if lead_surrogate.is_some() && ch != '\\' {
                let start = lead_surrogate.take().expect("pending surrogate");
                self.error("LEX012", "unpaired lead surrogate", start, self.offset);
            }
            if multiline && self.rest().starts_with(delimiter) {
                self.offset += 3;
                closed = true;
                break;
            }
            if !multiline && ch == quote {
                self.bump();
                closed = true;
                break;
            }
            if ch == '\\' {
                let escape_start = self.offset;
                self.bump();
                let Some(escape) = self.peek() else {
                    break;
                };
                if raw {
                    // Raw backslashes protect the following quote but remain verbatim in the token.
                    if escape == quote || escape == '\\' {
                        self.bump();
                    }
                    continue;
                }
                self.bump();
                if !matches!(escape, 'u' | 'U')
                    && let Some(start) = lead_surrogate.take()
                {
                    self.error("LEX012", "unpaired lead surrogate", start, escape_start);
                }
                match escape {
                    'a' | 'b' | 'f' | 'n' | 'r' | 't' | 'v' | '\'' | '"' | '\\' | '\n' => {}
                    '\r' if self.peek() == Some('\n') => self.bump(),
                    'u' | 'U' => {
                        let count = if escape == 'u' { 4 } else { 6 };
                        let mut value = 0;
                        let mut valid = true;
                        for _ in 0..count {
                            if let Some(digit) = self
                                .peek()
                                .and_then(|ch| ch.to_digit(16).filter(|_| ch.is_ascii()))
                            {
                                value = value * 16 + digit;
                                self.bump();
                            } else {
                                self.error(
                                    "LEX012",
                                    "invalid Unicode escape",
                                    escape_start,
                                    self.offset,
                                );
                                valid = false;
                                break;
                            }
                        }
                        if valid {
                            if let Some(start) = lead_surrogate.take() {
                                if (0xdc00..=0xdfff).contains(&value) {
                                    continue;
                                }
                                self.error(
                                    "LEX012",
                                    "unpaired lead surrogate",
                                    start,
                                    escape_start,
                                );
                            }
                            if (0xd800..=0xdbff).contains(&value) {
                                lead_surrogate = Some(escape_start);
                            } else if (0xdc00..=0xdfff).contains(&value) {
                                self.error(
                                    "LEX012",
                                    "unpaired trail surrogate",
                                    escape_start,
                                    self.offset,
                                );
                            } else if value > 0x10ffff {
                                self.error(
                                    "LEX012",
                                    "Unicode escape exceeds the scalar range",
                                    escape_start,
                                    self.offset,
                                );
                            }
                        }
                    }
                    _ => self.error("LEX012", "invalid string escape", escape_start, self.offset),
                }
            } else {
                self.bump();
            }
        }
        if let Some(start) = lead_surrogate {
            self.error("LEX012", "unpaired lead surrogate", start, self.offset);
        }
        self.emit(kind, start);
        if !closed {
            self.error("LEX006", "unterminated string literal", start, self.offset);
        }
    }

    fn operator(&mut self) -> bool {
        if let Some(ch @ ('<' | '>' | '=')) = self.peek() {
            let count = self
                .rest()
                .bytes()
                .take_while(|byte| *byte == ch as u8)
                .count();
            if count >= 7 {
                let start = self.offset;
                self.offset += count;
                self.emit(K::VcsConflictMarker, start);
                return true;
            }
        }
        const OPERATORS: &[(&str, K)] = &[
            ("?", K::QuestionMark),
            ("`", K::Backtick),
            ("**=", K::PowerEqual),
            ("<<=", K::ShiftLeftEqual),
            (">>=", K::ShiftRightEqual),
            ("...", K::Ellipsis),
            ("**", K::Power),
            ("<<", K::ShiftLeft),
            (">>", K::ShiftRight),
            ("==", K::EqualEqual),
            ("!=", K::NotEqual),
            ("<=", K::LessEqual),
            (">=", K::GreaterEqual),
            ("&&", K::AndAnd),
            ("||", K::OrOr),
            ("+=", K::PlusEqual),
            ("-=", K::MinusEqual),
            ("*=", K::StarEqual),
            ("/=", K::SlashEqual),
            ("%=", K::PercentEqual),
            ("&=", K::AmpersandEqual),
            ("|=", K::PipeEqual),
            ("^=", K::CaretEqual),
            ("->", K::Arrow),
            ("..", K::DotDot),
            ("(", K::LeftParen),
            (")", K::RightParen),
            ("[", K::LeftBracket),
            ("]", K::RightBracket),
            ("{", K::LeftBrace),
            ("}", K::RightBrace),
            (",", K::Comma),
            (";", K::Semicolon),
            (".", K::Dot),
            (":", K::Colon),
            ("$", K::Dollar),
            ("+", K::Plus),
            ("-", K::Minus),
            ("*", K::Star),
            ("/", K::Slash),
            ("%", K::Percent),
            ("=", K::Equal),
            ("<", K::Less),
            (">", K::Greater),
            ("&", K::Ampersand),
            ("|", K::Pipe),
            ("^", K::Caret),
            ("~", K::Tilde),
            ("!", K::Bang),
        ];
        let Some(&(text, kind)) = OPERATORS
            .iter()
            .find(|(text, _)| self.rest().starts_with(text))
        else {
            return false;
        };
        let start = self.offset;
        self.offset += text.len();
        self.emit(kind, start);
        match kind {
            K::LeftParen | K::LeftBracket | K::LeftBrace => {
                if self.brackets.len() >= self.options.max_nesting || self.overflow_depth > 0 {
                    self.overflow_depth += 1;
                    self.error(
                        "LEX002",
                        "delimiter nesting exceeds the configured limit",
                        start,
                        self.offset,
                    );
                } else {
                    self.brackets.push((kind, start));
                }
            }
            K::RightParen | K::RightBracket | K::RightBrace => {
                if self.overflow_depth > 0 {
                    self.overflow_depth -= 1;
                } else {
                    let expected = match kind {
                        K::RightParen => K::LeftParen,
                        K::RightBracket => K::LeftBracket,
                        _ => K::LeftBrace,
                    };
                    if self.brackets.pop().map(|(kind, _)| kind) != Some(expected) {
                        self.error(
                            "LEX007",
                            "unmatched or mismatched closing delimiter",
                            start,
                            self.offset,
                        );
                    }
                }
            }
            _ => {}
        }
        true
    }
}

fn is_keyword(text: &str) -> bool {
    matches!(
        text,
        "and"
            | "or"
            | "not"
            | "if"
            | "elif"
            | "else"
            | "for"
            | "while"
            | "break"
            | "continue"
            | "pass"
            | "return"
            | "match"
            | "when"
            | "as"
            | "assert"
            | "await"
            | "breakpoint"
            | "class"
            | "class_name"
            | "const"
            | "enum"
            | "extends"
            | "func"
            | "in"
            | "is"
            | "namespace"
            | "preload"
            | "self"
            | "signal"
            | "static"
            | "super"
            | "trait"
            | "var"
            | "void"
            | "yield"
            | "true"
            | "false"
            | "null"
            | "PI"
            | "TAU"
            | "INF"
            | "NAN"
    )
}
