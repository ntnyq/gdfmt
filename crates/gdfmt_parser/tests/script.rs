use gdfmt_parser::{LexOptions, ParseOptions, parse, parse_expression};
use gdfmt_syntax::{SyntaxKind as K, SyntaxNode};
use gdfmt_text::{DiagnosticCategory, SourceId};

fn script(source: &str) -> SyntaxNode {
    let result = parse(source, &ParseOptions::default());
    assert!(!result.has_errors(), "{source:?}: {:?}", result.diagnostics);
    let tree = result.tree.unwrap();
    assert_eq!(tree.text(), source);
    assert_eq!(tree.root().kind(), K::SourceFile);
    tree.root()
}
fn count(root: &SyntaxNode, kind: K) -> usize {
    root.descendants()
        .filter(|node| node.kind() == kind)
        .count()
}
fn expression(source: &str) -> SyntaxNode {
    let result = parse_expression(source, &ParseOptions::default());
    assert!(!result.has_errors(), "{source:?}: {:?}", result.diagnostics);
    let tree = result.tree.unwrap();
    assert_eq!(tree.text(), source);
    tree.root()
}

#[test]
fn declarations_have_structured_lossless_nodes() {
    let root = script(
        "\u{feff}@tool\r\nclass_name Example\r\nextends 'res://base.gd'.Inner\r\n@export_range(0, 10) var count: int = 1\r\nconst MAX := 10\r\nsignal changed(value: int)\r\nenum State { IDLE, ACTIVE = 2, }\r\nstatic var total = 0\r\nclass Inner extends RefCounted:\r\n\tvar data: Dictionary[String, int] = {}\r\n\tfunc value() -> int: return 1\r\nstatic func compute(x: int, scale = 2, ...rest) -> int:\r\n\treturn x * scale\r\n",
    );
    for (kind, expected) in [
        (K::ClassDecl, 1),
        (K::VariableDecl, 3),
        (K::ConstantDecl, 1),
        (K::FunctionDecl, 2),
        (K::Parameter, 4),
        (K::EnumEntry, 2),
        (K::AnnotationNode, 2),
    ] {
        assert_eq!(count(&root, kind), expected, "{kind:?}");
    }
}

#[test]
fn statements_control_flow_assignments_and_inline_suites() {
    let root = script(
        "func run(values):\n    var result = 0; const STEP = 1\n    for item: int in values:\n        if item < 0:\n            continue\n        elif item == 0: break\n        else:\n            result += item\n    while result < 20:\n        result = result + STEP\n    assert(result >= 0, 'negative')\n    breakpoint\n    pass\n    return result\n",
    );
    for kind in [
        K::ForStmt,
        K::WhileStmt,
        K::IfStmt,
        K::ElifClause,
        K::ElseClause,
        K::ContinueStmt,
        K::BreakStmt,
        K::AssertStmt,
        K::BreakpointStmt,
        K::PassStmt,
    ] {
        assert_eq!(count(&root, kind), 1, "{kind:?}");
    }
    assert_eq!(count(&root, K::AssignmentStmt), 2);
    let root = script("func f(): pass; pass; return\nfunc g(): pass\n");
    assert_eq!(count(&root, K::FunctionDecl), 2);
    assert_eq!(count(&root, K::PassStmt), 3);
}

#[test]
fn lambda_context_restores_outer_delimiters_indentation_and_loops() {
    for source in [
        "func(): return 1",
        "func named(x: int = 1) -> int:\n    return x\n",
        "consume(func(x): return x, 2)",
        "[func(): return 1, func(): return 2]",
        "{'callback': func(): return 1}",
        "consume(\n    func(x):\n        # body\n\n        var list = [\n  1,\n  2,\n        ]\n        return list[x]\n)\n",
        "[func():\n    if true:\n        return 1\n    return 2\n, 3]\n",
        "func():\n    return func():\n        return 1\n",
        "consume(func():\n    return other(func():\n        return 1\n)\n)\n",
        "func(cb = func(): return 1): return cb.call() + 2",
        "(func(): return 1).call()",
    ] {
        assert!(count(&expression(source), K::LambdaExpr) >= 1);
    }
    let root = script(
        "func run():\n    while true:\n        consume(func():\n            return 1\n)\n        break\n    var next = [\n        1,\n        2\n    ]\n    return next\n",
    );
    assert_eq!(count(&root, K::BreakStmt), 1);
    assert_eq!(count(&root, K::VariableDecl), 1);
    assert_eq!(count(&root, K::ReturnStmt), 2);
}

#[test]
fn match_patterns_guards_and_property_accessors() {
    let root = script(
        "var value: int = 0:\n    get:\n        return value\n    set(next):\n        value = next\nvar other: int: get = get_other, set = set_other\nfunc run(value):\n    match value:\n        [var head, ..] when head > 0:\n            return head\n        {'key': var entry, ..}: return entry\n        1, 2: pass\n        _: return null\n",
    );
    for (kind, expected) in [
        (K::MatchStmt, 1),
        (K::MatchBranch, 4),
        (K::Getter, 2),
        (K::Setter, 2),
        (K::PropertyBlock, 2),
    ] {
        assert_eq!(count(&root, kind), expected, "{kind:?}");
    }
    script("var untyped:\n    get: return 1\nvar delegated: get = read_value\n");
    script(
        "pass\n'documentation'\nclass Empty: pass\nfunc match_rest(v):\n    match v:\n        [var x, ..,]: pass\n        {'key': _, ..,}: pass\n",
    );
}

#[test]
fn invalid_scripts_report_context_and_recover_following_declarations() {
    for (source, code) in [
        ("return 1\n", "PAR010"),
        ("func f():\n    break\n", "PAR010"),
        (
            "func f():\n    while true:\n        var callback = func(): break\n",
            "PAR010",
        ),
        ("func f():\n    class Inner: pass\n", "PAR010"),
        ("const X\n", "PAR004"),
        ("func f(a = 1, b): pass\n", "PAR011"),
        ("func f(...rest, x): pass\n", "PAR011"),
        ("signal changed(value = 1)\n", "PAR011"),
        ("func f():\n    1 = 2\n", "PAR010"),
        ("func f():\npass\n", "PAR004"),
        (
            "func f():\n    match 1:\n        [.., var x]: pass\n",
            "PAR013",
        ),
        (
            "var value: int:\n    get: return 1\n    get: return 2\n",
            "PAR012",
        ),
        ("var value: get: return 1\n", "PAR012"),
        ("var = ;\n", "PAR004"),
        (
            "func f(v):\n    match v:\n        var x, 1: pass\n",
            "PAR013",
        ),
    ] {
        let source = format!("{source}\nvar sentinel = 9\n");
        let result = parse(&source, &ParseOptions::default());
        assert!(
            result.diagnostics.iter().any(|d| d.code == code),
            "{source:?}: {:?}",
            result.diagnostics
        );
        let tree = result.tree.unwrap();
        assert_eq!(tree.text(), source);
        assert!(
            tree.root()
                .descendants()
                .any(|node| node.kind() == K::VariableDecl
                    && node.text().to_string().contains("sentinel")),
            "{source:?}"
        );
    }
}

#[test]
fn typed_script_views_share_nodes_and_expose_direct_bodies() {
    use gdfmt_syntax::ast::{SourceFile, Statement};
    let root = script("var x = 1\nfunc run():\n    x = 2\n    return x\n");
    let file = SourceFile::cast(root.clone()).unwrap();
    assert_eq!(file.syntax(), &root);
    let declarations: Vec<_> = file.declarations().collect();
    assert_eq!(declarations.len(), 2);
    assert!(declarations[0].body().is_none());
    let body = declarations[1].body().unwrap();
    let statements: Vec<_> = body.statements().collect();
    assert_eq!(statements.len(), 2);
    assert_eq!(statements[0].syntax().kind(), K::AssignmentStmt);
    assert_eq!(
        statements[0].syntax().first_child().unwrap().kind(),
        K::NameExpr
    );
    assert_eq!(statements[1].syntax().kind(), K::ReturnStmt);
    assert!(Statement::cast(root).is_none());
}

#[test]
fn every_truncation_and_token_offset_preserves_the_source() {
    let source = "@tool\nclass Worker:\n    var 值 = consume(func(x):\n        if x:\n            return [1, 2]\n        return null\n)\n    func run(v):\n        match v:\n            {'key': var found, ..} when found:\n                assert(found)\n                return found\n";
    for end in source.char_indices().map(|(i, _)| i).chain([source.len()]) {
        let source = &source[..end];
        let result = parse(source, &ParseOptions::default());
        let tree = result.tree.unwrap();
        assert_eq!(tree.text(), source);
        let mut offset = 0;
        for token in tree
            .root()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
        {
            assert_eq!(u32::from(token.text_range().start()), offset);
            if token.kind().is_virtual() {
                assert!(token.text().is_empty());
            }
            offset += token.text().len() as u32;
        }
        assert_eq!(offset as usize, source.len());
        for diagnostic in result.diagnostics {
            assert!(diagnostic.span.slice(source).is_ok());
        }
    }
}

#[test]
fn script_limits_and_shared_diagnostic_budget() {
    let options = ParseOptions {
        lex: LexOptions {
            source_id: SourceId(8),
            max_nesting: 4096,
            max_diagnostics: 3,
            ..Default::default()
        },
        max_expression_depth: 16,
        ..Default::default()
    };
    for source in [
        format!(
            "func f():\n{}pass\n",
            (1..200)
                .map(|n| format!("{}if true:\n", "    ".repeat(n)))
                .collect::<String>()
        ),
        "var x = func(): return ".repeat(1000),
        "?\nreturn 1\n".repeat(1000),
    ] {
        let result = parse(&source, &options);
        assert!(result.has_errors());
        assert!(result.diagnostics.len() <= 4, "{:?}", result.diagnostics);
        assert!(
            result
                .diagnostics
                .iter()
                .all(|d| d.source_id == SourceId(8))
        );
        assert_eq!(result.tree.unwrap().text(), source);
    }
    let result = parse(
        "func f(): pass",
        &ParseOptions {
            lex: LexOptions {
                max_source_bytes: 2,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert!(result.tree.is_none());
    assert_eq!(result.diagnostics[0].category, DiagnosticCategory::Lexical);
    for limit in [128, usize::MAX] {
        let source = format!("{}return 1", "var x = func(): ".repeat(1000));
        let result = parse(
            &source,
            &ParseOptions {
                max_expression_depth: limit,
                ..Default::default()
            },
        );
        assert!(result.diagnostics.iter().any(|d| d.code == "PAR005"));
        assert_eq!(
            std::thread::spawn(move || result.tree.unwrap().text())
                .join()
                .unwrap(),
            source
        );
    }
}

#[test]
fn arbitrary_script_tokens_always_advance_and_keep_source() {
    let alphabet = [
        "func",
        "var",
        "class",
        "if",
        "elif",
        "else",
        "while",
        "for",
        "in",
        "match",
        "when",
        "return",
        "pass",
        "break",
        "static",
        "signal",
        "enum",
        "assert",
        "@tool",
        "x",
        "1",
        ":",
        "=",
        ",",
        ";",
        "(",
        ")",
        "[",
        "]",
        "{",
        "}",
        " ",
        "\n",
        "\t",
        "# hi\n",
        "'中😀'",
        "?",
    ];
    let mut seed = 42_u64;
    for length in 0..100 {
        for _ in 0..10 {
            let source: String = (0..length)
                .map(|_| {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    alphabet[(seed >> 32) as usize % alphabet.len()]
                })
                .collect();
            let result = parse(&source, &ParseOptions::default());
            assert_eq!(result.tree.unwrap().text(), source);
        }
    }
}
