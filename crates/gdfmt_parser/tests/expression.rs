use gdfmt_parser::{LexOptions, ParseOptions, lex, parse_expression};
use gdfmt_syntax::{SyntaxKind as K, SyntaxNode};
use gdfmt_text::{DiagnosticCategory, SourceId};

fn parse(source: &str) -> SyntaxNode {
    let result = parse_expression(source, &ParseOptions::default());
    assert!(!result.has_errors(), "{source:?}: {:?}", result.diagnostics);
    let tree = result.tree.unwrap();
    assert_eq!(tree.text(), source);
    assert_eq!(tree.root().kind(), K::ExpressionRoot);
    tree.root().first_child().unwrap()
}

// Small structural oracle: no trivia, but every operator and operand is retained.
fn shape(node: &SyntaxNode) -> String {
    let children: Vec<_> = node
        .children_with_tokens()
        .filter_map(|child| match child {
            rowan::NodeOrToken::Node(node) => Some(shape(&node)),
            rowan::NodeOrToken::Token(token)
                if !token.kind().is_trivia() && !token.kind().is_virtual() =>
            {
                Some(token.text().to_owned())
            }
            _ => None,
        })
        .collect();
    match node.kind() {
        K::NameExpr | K::LiteralExpr => children.join(""),
        _ => format!("({})", children.join(" ")),
    }
}

#[test]
fn arithmetic_associativity_and_signed_literal_binding() {
    for (source, expected) in [
        ("a + b * c", "(a + (b * c))"),
        ("a - b - c", "((a - b) - c)"),
        ("a ** b ** c", "((a ** b) ** c)"),
        ("-2 ** 2", "(-2 ** 2)"),
        ("- 2 ** 2", "(- (2 ** 2))"),
        ("~a ** b", "(~ (a ** b))"),
        ("a ** - b", "(a ** (- b))"),
        ("a << b + c * d", "(a << (b + (c * d)))"),
        ("a | b ^ c & d", "(a | (b ^ (c & d)))"),
        ("a < b == c", "((a < b) == c)"),
    ] {
        assert_eq!(shape(&parse(source)), expected, "{source}");
    }
}

#[test]
fn logical_membership_type_tests_casts_and_ternaries() {
    for (source, expected) in [
        ("not a in b", "(not (a in b))"),
        ("!a == b", "(! (a == b))"),
        ("a not in b", "(a not in b)"),
        ("a or b and c", "(a or (b and c))"),
        ("a || b && c", "(a || (b && c))"),
        ("a ** b is int", "(a ** (b is (int)))"),
        ("x is not Node", "(x is not (Node))"),
        ("a if b else c if d else e", "(a if b else (c if d else e))"),
        ("a if b if c else d else e", "(a if (b if c else d) else e)"),
        ("a if b else c as Node", "((a if b else c) as (Node))"),
        ("a as Node if b else c", "((a as (Node)) if b else c)"),
        ("a as Node as Object", "((a as (Node)) as (Object))"),
    ] {
        assert_eq!(shape(&parse(source)), expected, "{source}");
    }
}

#[test]
fn postfix_containers_node_paths_and_special_forms() {
    for source in [
        "obj.method(1, [2, 3,],)[index].member",
        "(method)(1)",
        "{(name) = 1, ('other') = 2}",
        "await obj.ready",
        "await do_work() + other",
        "super(1)",
        "super.method()",
        "preload('res://thing.gd',)",
        "[1, {key = [true, false, null]},]",
        "{name = 1, 'other' = 2,}",
        "{[1]: 2, 'other': 3,}",
        "$Root/Child.position",
        "$'/root/Child'",
        "$/root/Child",
        "$Root/%Unique",
        "%Unique",
        "%Unique/Child",
        "$if/var/_",
        "&'hello'",
        "^'../Child'",
        "'hello'.match('h*')",
        "when + PI",
        "x as Dictionary[String, int]",
        "x is Outer.Inner",
        "[]",
        "{}",
        "(a + b) * c",
    ] {
        parse(source);
    }
    assert_eq!(
        parse("await f() + x").first_child().unwrap().kind(),
        K::AwaitExpr
    );
    assert_eq!(parse("a[0].b()").kind(), K::CallExpr);
}

#[test]
fn trivia_tokens_and_byte_spans_are_preserved_in_order() {
    let source = "\u{feff}# leading\r\n[变量, # comment\r\n  2 + \\\r\n  3,] # trailing\r\n";
    let result = parse_expression(source, &ParseOptions::default());
    assert!(!result.has_errors(), "{:?}", result.diagnostics);
    let tree = result.tree.unwrap();
    assert_eq!(tree.text(), source);
    let actual: Vec<_> = tree
        .root()
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .map(|t| {
            (
                t.kind(),
                u32::from(t.text_range().start()),
                t.text().to_owned(),
            )
        })
        .collect();
    let expected: Vec<_> = lex(source, &LexOptions::default())
        .tokens
        .into_iter()
        .map(|t| {
            (
                t.kind,
                t.span.start(),
                t.span.slice(source).unwrap().to_owned(),
            )
        })
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn invalid_expressions_report_syntax_and_keep_all_source() {
    let options = ParseOptions {
        lex: LexOptions {
            source_id: SourceId(42),
            ..Default::default()
        },
        ..Default::default()
    };
    for source in [
        "",
        " ",
        "+",
        "a +",
        "a if b",
        "a if else c",
        "f(, 1)",
        "f(1 2)",
        "a[]",
        "a.",
        "a not b",
        "a is",
        "a as",
        "a as 1",
        "[1, )",
        "{a:}",
        "{a = 1, b: 2}",
        "{1 = 2}",
        "a = b",
        "a += b",
        "a; b",
        "a\nb",
        "f()()",
        "a[0]()",
        "super",
        "super.method",
        "$Node%Unique",
        "self()",
        "$",
        "%true",
        "preload()",
        "_",
        "x as Dictionary[String, Array[int]]",
        "x ? y : z",
        "<<<<<<< HEAD",
    ] {
        let result = parse_expression(source, &options);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.category == DiagnosticCategory::Syntax),
            "{source:?}: {:?}",
            result.diagnostics
        );
        for diagnostic in &result.diagnostics {
            assert_eq!(diagnostic.source_id, SourceId(42));
            assert!(diagnostic.span.slice(source).is_ok());
        }
        assert_eq!(result.tree.unwrap().text(), source);
    }
}

#[test]
fn list_recovery_keeps_later_elements() {
    for source in ["[1 +, 2, 3]", "f(1 2, 3)", "{a: , b: 2}"] {
        let result = parse_expression(source, &ParseOptions::default());
        assert!(result.has_errors());
        let tree = result.tree.unwrap();
        assert_eq!(tree.text(), source);
        assert!(tree.root().descendants().any(|n| n.kind() == K::LiteralExpr
            && n.text() == if source.starts_with('{') { "2" } else { "3" }));
    }
}

#[test]
fn limits_bound_recursion_left_chains_types_and_diagnostics() {
    let options = ParseOptions {
        max_expression_depth: 16,
        language_version: Default::default(),
        lex: LexOptions {
            max_diagnostics: 3,
            max_nesting: 4096,
            ..Default::default()
        },
    };
    for source in [
        format!("{}x{}", "(".repeat(1000), ")".repeat(1000)),
        format!("{}x", "not ".repeat(1000)),
        format!("x{}", " + x".repeat(1000)),
        format!("x as {}int{}", "Array[".repeat(1000), "]".repeat(1000)),
        format!("x{}", ".member".repeat(1000)),
        "[, , , , , , ,]".into(),
    ] {
        let result = parse_expression(&source, &options);
        assert!(result.has_errors());
        assert!(result.diagnostics.len() <= 4);
        assert_eq!(result.tree.unwrap().text(), source);
    }
    let result = parse_expression(
        "abc",
        &ParseOptions {
            lex: LexOptions {
                max_source_bytes: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert!(result.tree.is_none());
    let result = parse_expression(
        "+",
        &ParseOptions {
            lex: LexOptions {
                max_diagnostics: 0,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "PAR002");
}

#[test]
fn arbitrary_inputs_terminate_without_losing_text() {
    let alphabet: Vec<char> = "a_01 +-*~!&|^=<>.,:;?'\"\\()[]{}@%$#\r\n\t中😀\u{301}\0"
        .chars()
        .collect();
    let mut seed = 0x5eed_u64;
    for length in 0..128 {
        for _ in 0..12 {
            let source: String = (0..length)
                .map(|_| {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                    alphabet[(seed >> 32) as usize % alphabet.len()]
                })
                .collect();
            let result = parse_expression(&source, &ParseOptions::default());
            assert_eq!(result.tree.unwrap().text(), source);
        }
    }
}

#[test]
fn typed_expression_views_share_the_lossless_tree() {
    use gdfmt_syntax::ast::Expression;
    let node = parse("a + b * c");
    let expression = Expression::cast(node.clone()).unwrap();
    assert_eq!(expression.syntax(), &node);
    let operands: Vec<_> = expression.operands().collect();
    assert_eq!(operands.len(), 2);
    assert_eq!(operands[0].syntax().kind(), K::NameExpr);
    assert_eq!(operands[1].syntax().kind(), K::BinaryExpr);
    assert!(Expression::cast(node.parent().unwrap()).is_none());
    let result = parse_expression("1 +", &ParseOptions::default());
    let root = result.tree.unwrap().root();
    let error = root
        .descendants()
        .find(|node| node.kind() == K::ErrorNode)
        .unwrap();
    assert!(Expression::cast(error).is_none());
}

#[test]
fn every_binary_precedence_pair_matches_the_pinned_language_order() {
    // Low to high, transcribed from Godot's grammar precedence groups.
    let groups: &[&[&str]] = &[
        &["or", "||"],
        &["and", "&&"],
        &["in", "not in"],
        &["==", "!=", "<", "<=", ">", ">="],
        &["|"],
        &["^"],
        &["&"],
        &["<<", ">>"],
        &["+", "-"],
        &["*", "/", "%"],
        &["**"],
    ];
    for (first_rank, first_group) in groups.iter().enumerate() {
        for first in *first_group {
            for (second_rank, second_group) in groups.iter().enumerate() {
                for second in *second_group {
                    let source = format!("a {first} b {second} c");
                    let expected = if first_rank < second_rank {
                        format!("(a {first} (b {second} c))")
                    } else {
                        format!("((a {first} b) {second} c)")
                    };
                    assert_eq!(shape(&parse(&source)), expected, "{source}");
                }
            }
        }
    }
}

#[test]
fn syntax_limits_apply_without_lexical_overflow_and_green_trees_cross_threads() {
    let options = ParseOptions {
        max_expression_depth: 16,
        language_version: Default::default(),
        lex: LexOptions {
            max_nesting: 4096,
            ..Default::default()
        },
    };
    for source in [
        format!("{}a{}", "(".repeat(1000), ")".repeat(1000)),
        format!("{}a", "not ".repeat(1000)),
        format!("a{}", " + a".repeat(1000)),
        format!("a{}", ".b".repeat(1000)),
    ] {
        let result = parse_expression(&source, &options);
        assert!(
            result.diagnostics.iter().any(|d| d.code == "PAR005"),
            "{:?}",
            result.diagnostics
        );
        assert!(
            result
                .diagnostics
                .iter()
                .all(|d| d.category == DiagnosticCategory::Syntax)
        );
        let tree = result.tree.unwrap();
        assert_eq!(
            std::thread::spawn(move || tree.text()).join().unwrap(),
            source
        );
    }
    // Default and oversized configured limits also terminate with bounded stacks.
    for limit in [128, usize::MAX] {
        let source = format!("{}a", "not ".repeat(10000));
        let result = parse_expression(
            &source,
            &ParseOptions {
                max_expression_depth: limit,
                ..Default::default()
            },
        );
        assert!(result.diagnostics.iter().any(|d| d.code == "PAR005"));
        assert_eq!(result.tree.unwrap().text(), source);
    }
}
