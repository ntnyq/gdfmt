use gdfmt_formatter::{FormatError, FormatOptions, format, structurally_equivalent};
use gdfmt_parser::{ParseOptions, parse, parse_expression};

fn expression(source: &str, width: usize) -> String {
    let parsed = parse_expression(source, &ParseOptions::default());
    assert!(!parsed.has_errors(), "{source}: {:?}", parsed.diagnostics);
    let tree = parsed.tree.unwrap();
    let options = FormatOptions {
        line_width: width,
        indent: "  ".into(),
        ..Default::default()
    };
    let output = format(&tree, &parsed.diagnostics, &options).unwrap();
    let reparsed = parse_expression(&output, &ParseOptions::default());
    assert!(
        !reparsed.has_errors(),
        "{source} -> {output}: {:?}",
        reparsed.diagnostics
    );
    let after = reparsed.tree.unwrap();
    assert!(
        structurally_equivalent(&tree, &after),
        "{source} -> {output}"
    );
    assert_eq!(
        format(&after, &reparsed.diagnostics, &options).unwrap(),
        output
    );
    output
}

#[test]
fn expressions_calls_and_both_dictionary_styles_have_stable_layout() {
    for (source, expected) in [
        ("a+b* c", "a + b * c"),
        ("a not   in b", "a not in b"),
        (
            "x is not Dictionary[String,int]",
            "x is not Dictionary[String, int]",
        ),
        ("( a+b ) *c", "(a + b) * c"),
        (
            "obj . method( 1,[ 2,3, ],) [ 0 ]",
            "obj.method(1, [2, 3,],)[0]",
        ),
        ("{'a' :1, 'b': [2,3]}", "{'a': 1, 'b': [2, 3]}"),
        ("{name= 1, 'other' =2,}", "{name = 1, 'other' = 2,}"),
        ("await   load_data( 1,2)", "await load_data(1, 2)"),
        ("- 2 **2", "- 2 ** 2"),
        ("-2 **2", "-2 ** 2"),
        ("- - 2", "- - 2"),
        ("$ Root / Child . position", "$Root/Child.position"),
        ("a if b else c as Node", "a if b else c as Node"),
    ] {
        assert_eq!(expression(source, 100), expected);
    }
}

#[test]
fn groups_wrap_lists_with_existing_comma_choices_and_indent() {
    assert_eq!(
        expression("call(alpha, beta, gamma)", 15),
        "call(\n  alpha,\n  beta,\n  gamma\n)"
    );
    assert_eq!(expression("[alpha,beta,]", 8), "[\n  alpha,\n  beta,\n]");
    assert_eq!(expression("[]", 1), "[]");
    assert_eq!(
        expression("'a very long unbreakable string'", 5),
        "'a very long unbreakable string'"
    );
    let source = "func run():\r\n    return call(alpha,beta,gamma)\r\n";
    let parsed = parse(source, &ParseOptions::default());
    let output = format(
        &parsed.tree.unwrap(),
        &parsed.diagnostics,
        &FormatOptions {
            line_width: 25,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        output,
        "func run():\r\n\treturn call(\r\n\t\talpha,\r\n\t\tbeta,\r\n\t\tgamma\r\n\t)\r\n"
    );
}

#[test]
fn comments_lambdas_and_literal_tokens_have_stable_layout() {
    for (source, expected) in [
        ("[1, # target\n   2 ]", "[\n  1, # target\n  2\n]"),
        (
            "[func(x): return x+1,2]",
            "[\n  func(\n    x\n  ): return x + 1,\n  2\n]",
        ),
        (
            "'''body\r\n    literal text'''",
            "'''body\r\n    literal text'''",
        ),
        ("r'''a\n  b'''", "r'''a\n  b'''"),
        ("[1,\n # dangling\n]", "[\n  1,\n  # dangling\n]"),
    ] {
        assert_eq!(expression(source, 5), expected);
    }
    let source = "\u{feff}## header\r\n@tool\r\nvar values  =  [ 1,2 ] # tail\r\n";
    assert_eq!(
        script(source, &FormatOptions::default()),
        "\u{feff}## header\r\n@tool\r\nvar values = [1, 2] # tail\r\n"
    );
}

fn script(source: &str, options: &FormatOptions) -> String {
    let parsed = parse(source, &ParseOptions::default());
    assert!(!parsed.has_errors(), "{source}: {:?}", parsed.diagnostics);
    let before = parsed.tree.unwrap();
    let output = format(&before, &parsed.diagnostics, options).unwrap();
    let parsed = parse(&output, &ParseOptions::default());
    assert!(
        !parsed.has_errors(),
        "{source} -> {output}: {:?}",
        parsed.diagnostics
    );
    let after = parsed.tree.unwrap();
    assert!(
        structurally_equivalent(&before, &after),
        "{source} -> {output}"
    );
    assert_eq!(
        format(&after, &parsed.diagnostics, options).unwrap(),
        output
    );
    output
}

#[test]
fn whole_script_layout_matches_reviewed_snapshots() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for name in [
        "comments",
        "lambdas",
        "declarations",
        "suites",
        "multiline-literals",
    ] {
        let source = std::fs::read_to_string(root.join(format!(
            "../../tests/compatibility/fixtures/formatter-{name}.gd"
        )))
        .unwrap();
        for width in [40, 100] {
            let expected = std::fs::read_to_string(
                root.join(format!("tests/snapshots/formatter-{name}-{width}.gd")),
            )
            .unwrap();
            let options = FormatOptions {
                line_width: width,
                ..Default::default()
            };
            assert_eq!(script(&source, &options), expected, "{name}/{width}");
        }
    }
}

#[test]
fn file_options_preserve_literal_bytes_bom_and_comment_spelling() {
    use gdfmt_formatter::LineEnding;
    assert_eq!(
        script("\n\n\nvar a=1\n\n\n", &Default::default()),
        "var a = 1\n"
    );
    let source =
        "\u{feff}## header  \n\n\nvar text = \"\"\"a\r\n  b\"\"\"\n\n\nfunc run(): return text";
    let options = FormatOptions {
        indent: "  ".into(),
        line_ending: LineEnding::Crlf,
        max_blank_lines: 0,
        final_newline: false,
        ..Default::default()
    };
    assert_eq!(
        script(source, &options),
        "\u{feff}## header  \r\nvar text = \"\"\"a\r\n  b\"\"\"\r\nfunc run():\r\n  return text"
    );
    for ending in [LineEnding::Lf, LineEnding::Crlf, LineEnding::Preserve] {
        for final_newline in [false, true] {
            for max_blank_lines in [0, 1, 2, 5] {
                let options = FormatOptions {
                    line_ending: ending,
                    final_newline,
                    max_blank_lines,
                    ..Default::default()
                };
                let output = script(
                    "# header\r\n\r\n\r\n\r\nfunc run():\r\n    pass\r\n    # body end\r\n\r\n# eof\r\n",
                    &options,
                );
                assert_eq!(output.ends_with('\n'), final_newline);
            }
        }
    }
}

#[test]
fn comment_anchors_detect_reassociation_and_suite_ownership_changes() {
    let equivalent = |left: &str, right: &str| {
        let before = parse(left, &ParseOptions::default());
        let after = parse(right, &ParseOptions::default());
        assert!(!before.has_errors() && !after.has_errors());
        structurally_equivalent(&before.tree.unwrap(), &after.tree.unwrap())
    };
    assert!(!equivalent(
        "var a = [1, # first\n2]\n",
        "var a = [1, 2] # first\n"
    ));
    assert!(!equivalent(
        "func run():\n    pass\n    # body end\nvar a = 1\n",
        "func run():\n    pass\n# body end\nvar a = 1\n"
    ));
    assert_eq!(
        script(
            "func run():\n  pass\n  # body end\n# next\nvar a=1\n",
            &Default::default()
        ),
        "func run():\n\tpass\n\t# body end\n# next\nvar a = 1\n"
    );
    let source = "var value: int:\n  get: return value\n  # property end\nfunc run(value):\n  match value:\n    _: pass\n    # match end\n  return value\n";
    assert_eq!(
        script(source, &Default::default()),
        "var value: int:\n\tget:\n\t\treturn value\n\t# property end\nfunc run(value):\n\tmatch value:\n\t\t_:\n\t\t\tpass\n\t\t# match end\n\treturn value\n"
    );
    let source = "func run():\n  if true:\n    pass\n    # inner\n  # outer\n    # irregular indentation\n# next\n  # still next\nvar value = 1\n";
    assert_eq!(
        script(source, &Default::default()),
        "func run():\n\tif true:\n\t\tpass\n\t\t# inner\n\t# outer\n\t# irregular indentation\n# next\n# still next\nvar value = 1\n"
    );
}

#[test]
fn typed_collections_keep_the_first_type_on_the_opening_line() {
    let source =
        "class Inner extends RefCounted:\n    var data : Dictionary [ String, int ] = {}\n";
    let output = script(
        source,
        &FormatOptions {
            line_width: 20,
            ..Default::default()
        },
    );
    assert_eq!(
        output,
        "class Inner extends RefCounted:\n\tvar data: Dictionary[String, int] = {}\n"
    );
}

#[test]
fn comments_at_expression_token_boundaries_keep_their_anchors() {
    let source = "call([1,2], {'first':3,'other':4}, func(value): return value+1)";
    let parsed = parse_expression(source, &ParseOptions::default());
    let tree = parsed.tree.unwrap();
    let mut checked = 0;
    for token in tree
        .root()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
    {
        if token.kind().is_trivia() || token.kind().is_virtual() {
            continue;
        }
        let offset = u32::from(token.text_range().end()) as usize;
        for comment in [" # suffix\n", "\n# own line\n"] {
            let candidate = format!("{}{comment}{}", &source[..offset], &source[offset..]);
            if parse_expression(&candidate, &ParseOptions::default()).has_errors() {
                continue;
            }
            for width in [20, 40, 100] {
                expression(&candidate, width);
                checked += 1;
            }
        }
    }
    assert!(checked > 100);
}

#[test]
fn invalid_sources_and_recovery_trees_never_produce_output() {
    for source in [
        "var x = [1, )\n",
        "func run(): break\n",
        "@unknown\nvar x = 1\n",
    ] {
        let parsed = parse(source, &ParseOptions::default());
        assert!(parsed.has_errors());
        assert_eq!(
            format(
                &parsed.tree.unwrap(),
                &parsed.diagnostics,
                &FormatOptions::default()
            ),
            Err(FormatError::InvalidSyntax)
        );
    }
    let parsed = parse("var x = [1, )\n", &ParseOptions::default());
    assert_eq!(
        format(&parsed.tree.unwrap(), &[], &FormatOptions::default()),
        Err(FormatError::InvalidSyntax)
    );
    let left = parse_expression("-2 ** 2", &ParseOptions::default())
        .tree
        .unwrap();
    let right = parse_expression("- 2 ** 2", &ParseOptions::default())
        .tree
        .unwrap();
    assert!(!structurally_equivalent(&left, &right));
    let inline = parse_expression("[1, # target\n 2]", &ParseOptions::default())
        .tree
        .unwrap();
    let own_line = parse_expression("[1,\n # target\n 2]", &ParseOptions::default())
        .tree
        .unwrap();
    assert!(!structurally_equivalent(&inline, &own_line));
}
