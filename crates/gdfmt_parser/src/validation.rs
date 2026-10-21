//! Context and feature checks over the same lossless CST used by consumers.
use crate::ParseOptions;
use gdfmt_syntax::{
    LanguageFeature as Feature, SyntaxKind as K, SyntaxNode, SyntaxToken, ast::Expression,
};
use gdfmt_text::{Diagnostic, DiagnosticCategory, Severity, Span};
use std::collections::HashSet;

pub(crate) fn validate(
    root: &SyntaxNode,
    options: &ParseOptions,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut validator = Validator {
        options,
        diagnostics,
    };
    if root.kind() == K::SourceFile {
        validator.class_body(root, false, false);
    }
    // Warning regions span nested classes and functions in source order, not
    // lexical scopes. Do this once, separately from recursive body validation.
    validator.warning_regions(root);
    for node in root.descendants() {
        match node.kind() {
            K::TypeRef => validator.type_ref(&node),
            K::ParameterList => validator.parameters(&node),
            K::EnumDecl => validator.unique(
                node.children().filter(|n| n.kind() == K::EnumEntry),
                "PAR015",
            ),
            K::MatchBranch => validator.patterns(&node),
            K::MatchStmt => validator.match_annotations(&node),
            K::Suite if !is_class_body(&node) => validator.statement_annotations(&node),
            K::SelfExpr | K::SuperExpr | K::NodeExpr => {
                let context = node.ancestors().find(|n| {
                    n.kind() == K::FunctionDecl
                        || (n.kind() == K::VariableDecl
                            && n.parent()
                                .is_some_and(|p| p.kind() == K::SourceFile || is_class_body(&p)))
                });
                if context.as_ref().is_some_and(is_static) {
                    validator.error(
                        &node,
                        "PAR016",
                        "instance access is not allowed in a static context",
                    );
                }
                if node.kind() == K::SuperExpr
                    && root.kind() == K::SourceFile
                    && !node.ancestors().any(|n| {
                        matches!(
                            n.kind(),
                            K::FunctionDecl | K::LambdaExpr | K::Getter | K::Setter
                        )
                    })
                {
                    validator.error(&node, "PAR016", "super requires a function body");
                }
            }
            K::ReturnStmt => validator.return_statement(&node),
            K::FunctionDecl | K::LambdaExpr | K::Getter | K::Setter
                if !node.ancestors().skip(1).any(|parent| {
                    matches!(
                        parent.kind(),
                        K::FunctionDecl | K::LambdaExpr | K::Getter | K::Setter
                    )
                }) =>
            {
                // Nested lambdas are visited in source order with the outer
                // locals visible at their declaration, not in isolation.
                validator.function_locals(&node, &mut HashSet::new());
            }
            _ => {}
        }
    }
}

struct Validator<'a> {
    options: &'a ParseOptions,
    diagnostics: &'a mut Vec<Diagnostic>,
}

fn tokens(node: &SyntaxNode) -> impl Iterator<Item = SyntaxToken> {
    node.children_with_tokens()
        .filter_map(|element| element.into_token())
        .filter(|token| !token.kind().is_trivia() && !token.kind().is_virtual())
}
fn first(node: &SyntaxNode) -> Option<SyntaxToken> {
    tokens(node).next()
}
fn name(node: &SyntaxNode) -> Option<String> {
    tokens(node)
        .find(|t| t.kind().is_identifier(t.text()))
        .map(|t| t.text().to_string())
}
fn has(node: &SyntaxNode, text: &str) -> bool {
    tokens(node).any(|t| t.text() == text)
}
fn is_static(node: &SyntaxNode) -> bool {
    has(node, "static")
}
fn is_class_body(node: &SyntaxNode) -> bool {
    node.parent()
        .is_some_and(|parent| parent.kind() == K::ClassDecl)
}
fn body(node: &SyntaxNode) -> Option<SyntaxNode> {
    node.children().find(|child| child.kind() == K::Suite)
}
fn annotation_name(node: &SyntaxNode) -> String {
    first(node)
        .map(|t| t.text().to_string())
        .unwrap_or_default()
}

impl Validator<'_> {
    fn error(&mut self, node: &SyntaxNode, code: &str, message: &str) {
        let range = node
            .descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .find(|t| !t.kind().is_trivia() && !t.kind().is_virtual())
            .map(|t| t.text_range())
            .unwrap_or(node.text_range());
        self.at(
            Span::new(range.start().into(), range.end().into()).unwrap(),
            code,
            message,
        );
    }
    fn at(&mut self, span: Span, code: &str, message: &str) {
        let count = self.diagnostics.len();
        let (code, message) = if count < self.options.lex.max_diagnostics {
            (code, message)
        } else if count == self.options.lex.max_diagnostics {
            ("PAR002", "further syntax diagnostics suppressed")
        } else {
            return;
        };
        self.diagnostics.push(Diagnostic {
            code: code.into(),
            category: DiagnosticCategory::Syntax,
            severity: Severity::Error,
            message: message.into(),
            source_id: self.options.lex.source_id,
            span,
        });
    }
    fn gate(&mut self, node: &SyntaxNode, feature: Feature) {
        if !self.options.language_version.supports(feature) {
            self.error(
                node,
                "PAR017",
                &format!(
                    "{feature:?} requires Godot {} or newer (target {})",
                    feature.since(),
                    self.options.language_version
                ),
            );
        }
    }
    fn warning_regions(&mut self, root: &SyntaxNode) {
        if !self
            .options
            .language_version
            .supports(Feature::WarningRegions)
        {
            return;
        }
        let mut ignored = HashSet::new();
        for node in root
            .descendants()
            .filter(|node| node.kind() == K::AnnotationNode)
        {
            let annotation = annotation_name(&node);
            let start = match annotation.as_str() {
                "@warning_ignore_start" => true,
                "@warning_ignore_restore" => false,
                _ => continue,
            };
            for argument in node.children().filter(|node| node.kind() == K::LiteralExpr) {
                let Some(key) = warning_key(&argument) else {
                    continue;
                };
                let valid = if start {
                    ignored.insert(key)
                } else {
                    ignored.remove(&key)
                };
                if !valid {
                    self.error(
                        &argument,
                        "PAR014",
                        if start {
                            "warning is already being ignored by @warning_ignore_start"
                        } else {
                            "warning is not being ignored by @warning_ignore_start"
                        },
                    );
                }
            }
        }
    }
    fn unique(&mut self, nodes: impl Iterator<Item = SyntaxNode>, code: &str) {
        let mut names = HashSet::new();
        for node in nodes {
            if let Some(name) = name(&node)
                && !names.insert(name)
            {
                self.error(&node, code, "duplicate name in this declaration");
            }
        }
    }
    fn type_ref(&mut self, node: &SyntaxNode) {
        if !has(node, "[") {
            return;
        }
        let count = node
            .children()
            .filter(|child| child.kind() == K::TypeRef)
            .count();
        match name(node).as_deref() {
            Some("Array") if count == 1 => {}
            Some("Dictionary") => {
                self.gate(node, Feature::TypedDictionary);
                if count != 2 {
                    self.error(
                        node,
                        "PAR008",
                        "Dictionary requires exactly two type arguments",
                    );
                }
            }
            _ => self.error(
                node,
                "PAR008",
                "only Array[T] and Dictionary[K, V] accept type arguments",
            ),
        }
    }
    fn parameters(&mut self, node: &SyntaxNode) {
        self.unique(
            node.children().filter(|n| n.kind() == K::Parameter),
            "PAR011",
        );
        for parameter in node
            .children()
            .filter(|n| n.kind() == K::Parameter && has(n, "..."))
        {
            self.gate(&parameter, Feature::VariadicFunction);
            if let Some(ty) = parameter.children().find(|n| n.kind() == K::TypeRef)
                && (name(&ty).as_deref() != Some("Array") || tokens(&ty).count() != 1)
            {
                self.error(&ty, "PAR011", "rest parameter type must be untyped Array");
            }
        }
    }
    fn patterns(&mut self, branch: &SyntaxNode) {
        let mut names = HashSet::new();
        for pattern in branch
            .children()
            .filter(|n| n.kind() == K::Pattern)
            .flat_map(|n| n.descendants())
        {
            if has(&pattern, "var")
                && let Some(name) = name(&pattern)
                && !names.insert(name)
            {
                self.error(&pattern, "PAR013", "duplicate binding in match pattern");
            }
        }
    }
    fn return_statement(&mut self, node: &SyntaxNode) {
        let Some(mut value) = node
            .children()
            .find(|n| Expression::cast(n.clone()).is_some())
        else {
            return;
        };
        // Godot discards grouping when deciding whether a void return is a
        // call. Calls are allowed for their effects; constructors still reject
        // every return value, including calls.
        while value.kind() == K::GroupExpr {
            let Some(inner) = value
                .children()
                .find(|n| Expression::cast(n.clone()).is_some())
            else {
                break;
            };
            value = inner;
        }
        let Some(function) = node.ancestors().find(|n| {
            matches!(
                n.kind(),
                K::FunctionDecl | K::LambdaExpr | K::Getter | K::Setter
            )
        }) else {
            return;
        };
        let constructor = function.kind() == K::FunctionDecl
            && matches!(name(&function).as_deref(), Some("_init" | "_static_init"));
        let void = function
            .children()
            .any(|n| n.kind() == K::TypeRef && has(&n, "void"));
        if constructor || (void && value.kind() != K::CallExpr) {
            self.error(node, "PAR016", "this function cannot return a value");
        }
    }
    fn function(&mut self, node: &SyntaxNode, abstract_method: bool, abstract_class: bool) {
        let name = name(node).unwrap_or_default();
        let has_body = body(node).is_some();
        if abstract_method {
            if has_body
                || is_static(node)
                || !abstract_class
                || matches!(name.as_str(), "_init" | "_static_init")
            {
                self.error(node, "PAR016", "abstract methods require an abstract class, cannot be static or constructors, and must have no body");
            }
        } else if !has_body {
            self.error(node, "PAR004", "non-abstract function requires a body");
        }
        if name == "_static_init" {
            let has_parameters = node
                .children()
                .find(|n| n.kind() == K::ParameterList)
                .is_some_and(|n| n.children().any(|n| n.kind() == K::Parameter));
            if !is_static(node) || has_parameters {
                self.error(
                    node,
                    "PAR016",
                    "_static_init must be static and cannot have parameters",
                );
            }
        }
        if (name == "_init" && is_static(node))
            || (matches!(name.as_str(), "_init" | "_static_init")
                && node
                    .children()
                    .any(|n| n.kind() == K::TypeRef && !has(&n, "void")))
        {
            self.error(
                node,
                "PAR016",
                "constructors must use their required instance/static context and return void",
            );
        }
    }

    fn class_body(&mut self, block: &SyntaxNode, mut abstract_class: bool, mut extends: bool) {
        let root = block.kind() == K::SourceFile;
        let mut header = true;
        let mut class_name = false;
        let mut pending = Vec::new();
        let mut names = HashSet::new();
        let mut script_annotations = HashSet::new();
        for node in block.children() {
            if node.kind() == K::AnnotationNode {
                let annotation = annotation_name(&node);
                self.annotation(&node);
                // After a root header, @abstract targets the next member;
                // inner-class extends must precede all member annotations.
                if !root || ((class_name || extends) && annotation == "@abstract") {
                    header = false;
                }
                if matches!(annotation.as_str(), "@tool" | "@icon" | "@static_unload") {
                    if !root
                        || !header
                        || class_name
                        || extends
                        || !script_annotations.insert(annotation)
                    {
                        self.error(
                            &node,
                            "PAR014",
                            "script annotation must appear once, before class_name and extends",
                        );
                    }
                    self.head_annotations(&mut pending, &mut abstract_class);
                } else if standalone(&annotation) {
                    if annotation.starts_with("@export_") {
                        header = false;
                    }
                    self.standalone(&node);
                } else {
                    if annotation != "@abstract" {
                        header = false;
                    }
                    pending.push(node);
                }
                continue;
            }
            if matches!(node.kind(), K::ExtendsDecl | K::ClassNameDecl) {
                if root && header {
                    self.head_annotations(&mut pending, &mut abstract_class);
                }
                if !header
                    || (node.kind() == K::ClassNameDecl && (!root || class_name))
                    || (node.kind() == K::ExtendsDecl && extends)
                {
                    self.error(&node, "PAR015", "class_name/extends must appear once at the start of the class; class_name is script-only");
                }
                if node.kind() == K::ClassNameDecl {
                    class_name = true;
                } else {
                    extends = true;
                }
                if node.children().any(|n| n.kind() == K::ExtendsDecl) {
                    if extends {
                        self.error(&node, "PAR015", "extends can only be used once");
                    }
                    extends = true;
                }
                continue;
            }
            // Class comment strings do not close the declaration header.
            if node.kind() == K::ExpressionStmt
                && node
                    .children()
                    .any(|n| n.kind() == K::LiteralExpr && has_string(&n))
            {
                if !root {
                    header = false;
                }
                continue;
            }
            header = false;
            // Class-body pass/comment strings leave annotations pending for
            // the next member. An annotation still pending at EOF is invalid.
            if node.kind() == K::PassStmt {
                continue;
            }
            let abstract_target = pending.iter().any(|n| annotation_name(n) == "@abstract");
            self.bind(&mut pending, &node, true);
            if matches!(
                node.kind(),
                K::ClassDecl
                    | K::VariableDecl
                    | K::ConstantDecl
                    | K::EnumDecl
                    | K::SignalDecl
                    | K::FunctionDecl
            ) && let Some(name) = name(&node)
                && !names.insert(name)
            {
                self.error(&node, "PAR015", "duplicate class member name");
            }
            if node.kind() == K::EnumDecl && name(&node).is_none() {
                for entry in node.children().filter(|n| n.kind() == K::EnumEntry) {
                    if let Some(name) = name(&entry)
                        && !names.insert(name)
                    {
                        self.error(&entry, "PAR015", "enum entry conflicts with a class member");
                    }
                }
            }
            match node.kind() {
                K::ClassDecl => {
                    if let Some(suite) = body(&node) {
                        self.class_body(
                            &suite,
                            abstract_target,
                            node.children().any(|n| n.kind() == K::TypeRef)
                                || has(&node, "extends"),
                        );
                    }
                }
                K::FunctionDecl => self.function(&node, abstract_target, abstract_class),
                _ => {}
            }
        }
        if root && header {
            self.head_annotations(&mut pending, &mut abstract_class);
        }
        self.unused(&pending);
    }
    fn head_annotations(&mut self, pending: &mut Vec<SyntaxNode>, abstract_class: &mut bool) {
        for node in pending.drain(..) {
            if annotation_name(&node) == "@abstract" {
                if *abstract_class {
                    self.error(&node, "PAR014", "duplicate @abstract annotation");
                }
                *abstract_class = true;
            } else {
                self.error(&node, "PAR014", "annotation cannot target the script");
            }
        }
    }
    fn statement_annotations(&mut self, block: &SyntaxNode) {
        let mut pending = Vec::new();
        for node in block.children() {
            if node.kind() == K::AnnotationNode {
                self.annotation(&node);
                if standalone(&annotation_name(&node)) {
                    self.standalone(&node);
                } else {
                    pending.push(node);
                }
            } else {
                self.bind(&mut pending, &node, false);
            }
        }
        self.unused(&pending);
    }
    fn match_annotations(&mut self, block: &SyntaxNode) {
        let mut pending = Vec::new();
        for node in block.children() {
            match node.kind() {
                K::AnnotationNode => {
                    self.annotation(&node);
                    if annotation_name(&node) == "@warning_ignore" {
                        pending.push(node);
                    } else {
                        self.error(
                            &node,
                            "PAR014",
                            "only warning_ignore may annotate a match branch",
                        );
                    }
                }
                K::MatchBranch => self.bind(&mut pending, &node, false),
                _ => {}
            }
        }
        self.unused(&pending);
    }
    fn function_locals(&mut self, function: &SyntaxNode, names: &mut HashSet<String>) {
        let mut introduced = Vec::new();
        if let Some(parameters) = function.children().find(|n| n.kind() == K::ParameterList) {
            for parameter in parameters.children().filter(|n| n.kind() == K::Parameter) {
                self.expression_lambdas(&parameter, names);
                // Parameters may shadow outer locals, but body declarations
                // may not. Only remove names newly introduced by this lambda.
                if let Some(name) = name(&parameter)
                    && names.insert(name.clone())
                {
                    introduced.push(name);
                }
            }
        }
        if function.kind() == K::Setter
            && let Some(parameter) = tokens(function)
                .filter(|t| t.kind().is_identifier(t.text()))
                .nth(1)
        {
            let name = parameter.text().to_string();
            if names.insert(name.clone()) {
                introduced.push(name);
            }
        }
        if let Some(suite) = body(function) {
            self.locals(&suite, names);
        }
        for name in introduced {
            names.remove(&name);
        }
    }
    fn expression_lambdas(&mut self, node: &SyntaxNode, names: &mut HashSet<String>) {
        if node.kind() == K::LambdaExpr {
            self.function_locals(node, names);
            return;
        }
        for child in node.children() {
            match child.kind() {
                K::Suite | K::MatchBranch => {}
                _ => self.expression_lambdas(&child, names),
            }
        }
    }
    fn locals(&mut self, suite: &SyntaxNode, names: &mut HashSet<String>) {
        // Undo additions on exit instead of cloning every visible local for
        // each branch. Wide scripts remain linear in their declarations.
        let mut introduced = Vec::new();
        for statement in suite.children() {
            // Initializers/iterables are parsed before registering their new
            // variable. Match guards need branch bindings, handled below.
            self.expression_lambdas(&statement, names);
            if matches!(statement.kind(), K::VariableDecl | K::ConstantDecl) {
                if let Some(name) = name(&statement) {
                    if names.insert(name.clone()) {
                        introduced.push(name);
                    } else {
                        self.error(
                            &statement,
                            "PAR015",
                            "local name is already declared in this scope",
                        );
                    }
                }
                continue;
            }
            if statement.kind() == K::ForStmt {
                let binding = name(&statement).filter(|name| {
                    if names.insert(name.clone()) {
                        true
                    } else {
                        self.error(
                            &statement,
                            "PAR015",
                            "loop variable is already declared in this scope",
                        );
                        false
                    }
                });
                if let Some(suite) = body(&statement) {
                    self.locals(&suite, names);
                }
                if let Some(binding) = binding {
                    names.remove(&binding);
                }
                continue;
            }
            if statement.kind() == K::MatchStmt {
                for branch in statement.children().filter(|n| n.kind() == K::MatchBranch) {
                    for pattern in branch.children().filter(|n| n.kind() == K::Pattern) {
                        self.expression_lambdas(&pattern, names);
                    }
                    let mut bindings = Vec::new();
                    for pattern in branch
                        .children()
                        .filter(|n| n.kind() == K::Pattern)
                        .flat_map(|n| n.descendants())
                    {
                        if has(&pattern, "var")
                            && let Some(name) = name(&pattern)
                        {
                            if names.insert(name.clone()) {
                                bindings.push(name);
                            } else {
                                self.error(
                                    &pattern,
                                    "PAR013",
                                    "pattern binding conflicts with an existing local",
                                );
                            }
                        }
                    }
                    for guard in branch
                        .children()
                        .filter(|node| Expression::cast(node.clone()).is_some())
                    {
                        self.expression_lambdas(&guard, names);
                    }
                    if let Some(suite) = body(&branch) {
                        self.locals(&suite, names);
                    }
                    for binding in bindings {
                        names.remove(&binding);
                    }
                }
                continue;
            }
            if matches!(statement.kind(), K::IfStmt | K::WhileStmt) {
                for child in statement.children() {
                    if child.kind() == K::Suite {
                        self.locals(&child, names);
                    } else if matches!(child.kind(), K::ElifClause | K::ElseClause)
                        && let Some(suite) = body(&child)
                    {
                        self.locals(&suite, names);
                    }
                }
            }
        }
        for name in introduced {
            names.remove(&name);
        }
    }
    fn unused(&mut self, pending: &[SyntaxNode]) {
        for annotation in pending {
            self.error(annotation, "PAR014", "annotation has no target");
        }
    }
    fn bind(&mut self, pending: &mut Vec<SyntaxNode>, target: &SyntaxNode, class_body: bool) {
        let mut seen = HashSet::new();
        let mut exported = false;
        for node in pending.drain(..) {
            let annotation = annotation_name(&node);
            let valid = match annotation.as_str() {
                "@abstract" => {
                    class_body && matches!(target.kind(), K::ClassDecl | K::FunctionDecl)
                }
                "@rpc" => class_body && target.kind() == K::FunctionDecl && !is_static(target),
                "@warning_ignore" => true,
                "@onready" => class_body && target.kind() == K::VariableDecl && !is_static(target),
                annotation if annotation.starts_with("@export") => {
                    class_body && target.kind() == K::VariableDecl && !is_static(target)
                }
                _ => false,
            };
            if !valid {
                self.error(&node, "PAR014", "annotation is not valid for this target");
            }
            if annotation.starts_with("@export") {
                if exported {
                    self.error(
                        &node,
                        "PAR014",
                        "a variable can only have one export annotation",
                    );
                }
                exported = true;
            }
            if annotation != "@warning_ignore" && !seen.insert(annotation) {
                self.error(&node, "PAR014", "duplicate annotation on this declaration");
            }
        }
    }
    fn standalone(&mut self, node: &SyntaxNode) {
        let newline = node.last_token().is_some_and(|t| t.kind() == K::Newline);
        if !newline {
            self.error(node, "PAR014", "standalone annotation requires a newline");
        }
        if annotation_name(node).starts_with("@export_")
            && node
                .parent()
                .is_some_and(|p| p.kind() != K::SourceFile && !is_class_body(&p))
        {
            self.error(node, "PAR014", "export groups require a class body");
        }
    }
    fn annotation(&mut self, node: &SyntaxNode) {
        let annotation = annotation_name(node);
        let Some((min, max)) = annotation_arity(&annotation) else {
            self.error(node, "PAR014", "unknown annotation");
            return;
        };
        let arguments: Vec<_> = node
            .children()
            .filter(|n| Expression::cast(n.clone()).is_some())
            .collect();
        if !(min..=max).contains(&arguments.len()) {
            self.error(node, "PAR014", "invalid annotation argument count");
        }
        match annotation.as_str() {
            "@icon" => {
                if arguments
                    .iter()
                    .any(|node| node.kind() != K::LiteralExpr || !has_string(node))
                {
                    self.error(node, "PAR014", "icon argument must be a string literal");
                }
                if arguments.iter().any(empty_string_literal) {
                    self.error(node, "PAR014", "icon path cannot be empty");
                }
            }
            "@abstract" => self.gate(node, Feature::Abstract),
            "@export_tool_button" => self.gate(node, Feature::ExportToolButton),
            "@export_file_path" => self.gate(node, Feature::ExportFilePath),
            "@export_multiline" if !arguments.is_empty() => {
                self.gate(node, Feature::MultilineExportHints)
            }
            "@warning_ignore_start" | "@warning_ignore_restore" => {
                self.gate(node, Feature::WarningRegions);
                if arguments
                    .iter()
                    .any(|n| n.kind() != K::LiteralExpr || !has_string(n))
                {
                    self.error(
                        node,
                        "PAR014",
                        "warning region arguments must be string literals",
                    );
                }
            }
            _ => {}
        }
    }
}

// Warning names are ASCII identifiers, case insensitive in the engine. Decode
// Unicode escapes and line continuations so equivalent spellings share state.
fn warning_key(node: &SyntaxNode) -> Option<String> {
    let token = tokens(node).find(|token| token.kind() == K::String)?;
    let text = token.text();
    let raw = text.starts_with('r');
    let text = if raw { &text[1..] } else { text };
    let quote = text.chars().next()?;
    let delimiter = quote
        .to_string()
        .repeat(if text.starts_with(&quote.to_string().repeat(3)) {
            3
        } else {
            1
        });
    let content = text.strip_prefix(&delimiter)?.strip_suffix(&delimiter)?;
    let mut chars = content.chars().peekable();
    let mut key = String::new();
    while let Some(mut character) = chars.next() {
        if character == '\\' && !raw {
            character = match chars.next()? {
                'u' => char::from_u32(
                    u32::from_str_radix(&chars.by_ref().take(4).collect::<String>(), 16).ok()?,
                )?,
                'U' => char::from_u32(
                    u32::from_str_radix(&chars.by_ref().take(6).collect::<String>(), 16).ok()?,
                )?,
                '\n' => continue,
                '\r' if chars.next() == Some('\n') => continue,
                _ => return None,
            };
        }
        if !(character.is_ascii_alphabetic() || character == '_') {
            return None;
        }
        key.push(character.to_ascii_uppercase());
    }
    (!key.is_empty()).then_some(key)
}

fn empty_string_literal(node: &SyntaxNode) -> bool {
    let Some(token) = tokens(node).find(|token| token.kind() == K::String) else {
        return false;
    };
    let text = token.text();
    let raw = text.starts_with('r');
    let text = if raw { &text[1..] } else { text };
    let Some(quote) = text.chars().next() else {
        return false;
    };
    let delimiter = quote
        .to_string()
        .repeat(if text.starts_with(&quote.to_string().repeat(3)) {
            3
        } else {
            1
        });
    let Some(content) = text
        .strip_prefix(&delimiter)
        .and_then(|text| text.strip_suffix(&delimiter))
    else {
        return false;
    };
    if raw {
        return content.is_empty();
    }
    // Only escaped physical newlines decode to zero characters. Other escapes
    // and ordinary characters necessarily make the path nonempty.
    let mut chars = content.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            return false;
        }
        match chars.next() {
            Some('\n') => {}
            Some('\r') if chars.next() == Some('\n') => {}
            _ => return false,
        }
    }
    true
}

fn has_string(node: &SyntaxNode) -> bool {
    tokens(node).any(|t| t.kind() == K::String)
}
fn standalone(annotation: &str) -> bool {
    matches!(
        annotation,
        "@export_category"
            | "@export_group"
            | "@export_subgroup"
            | "@warning_ignore_start"
            | "@warning_ignore_restore"
    )
}
fn annotation_arity(annotation: &str) -> Option<(usize, usize)> {
    Some(match annotation {
        "@tool"
        | "@static_unload"
        | "@abstract"
        | "@onready"
        | "@export"
        | "@export_dir"
        | "@export_global_dir"
        | "@export_color_no_alpha"
        | "@export_storage"
        | "@export_flags_2d_render"
        | "@export_flags_2d_physics"
        | "@export_flags_2d_navigation"
        | "@export_flags_3d_render"
        | "@export_flags_3d_physics"
        | "@export_flags_3d_navigation"
        | "@export_flags_avoidance" => (0, 0),
        "@icon" | "@export_placeholder" | "@export_category" => (1, 1),
        "@export_group" | "@export_subgroup" | "@export_tool_button" => (1, 2),
        "@export_file" | "@export_file_path" | "@export_global_file" | "@export_node_path" => {
            (0, usize::MAX)
        }
        "@export_multiline" | "@export_exp_easing" => (0, usize::MAX),
        "@export_enum"
        | "@export_flags"
        | "@warning_ignore"
        | "@warning_ignore_start"
        | "@warning_ignore_restore" => (1, usize::MAX),
        "@export_range" => (2, usize::MAX),
        "@export_custom" => (2, 3),
        "@rpc" => (0, 4),
        _ => return None,
    })
}
