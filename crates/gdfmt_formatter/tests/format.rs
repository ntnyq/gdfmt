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
fn comments_in_explicit_continuations_keep_valid_line_breaks() {
    let source =
        "func run():\n\tif first() \\\n\t\t# condition\n\t\tand second():\n\t\t\treturn true\n";
    assert_eq!(
        script(source, &FormatOptions::default()),
        "func run():\n\tif first() \\\n\t# condition\n\tand second():\n\t\treturn true\n"
    );
    let source =
        include_str!("../../../tests/compatibility/fixtures/formatter-continuation-comments.gd");
    for width in [20, 40, 80, 100, 120] {
        for indent in ["\t", "  ", "    "] {
            for line_ending in [
                gdfmt_formatter::LineEnding::Lf,
                gdfmt_formatter::LineEnding::Crlf,
                gdfmt_formatter::LineEnding::Preserve,
            ] {
                for final_newline in [false, true] {
                    let options = FormatOptions {
                        line_width: width,
                        indent: indent.into(),
                        line_ending,
                        final_newline,
                        ..Default::default()
                    };
                    script(source, &options);
                    script(&source.replace('\n', "\r\n"), &options);
                }
            }
        }
    }
}

#[test]
fn explicit_continuation_comments_at_token_boundaries_remain_stable() {
    let source = "func run(value):\n\treturn value.method(1, 2) + 3 if value is Node else -4\n";
    let parsed = parse(source, &ParseOptions::default());
    assert!(!parsed.has_errors());
    let mut checked = 0;
    for token in parsed
        .tree
        .unwrap()
        .root()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
    {
        if token.kind().is_trivia() || token.kind().is_virtual() {
            continue;
        }
        let offset = u32::from(token.text_range().end()) as usize;
        for comment in [
            " \\\n\t# continuation\n\t",
            " \\\n\t# first\n\t# second\n\t",
        ] {
            let candidate = format!("{}{comment}{}", &source[..offset], &source[offset..]);
            if parse(&candidate, &ParseOptions::default()).has_errors() {
                continue;
            }
            for width in [20, 40, 100] {
                script(
                    &candidate,
                    &FormatOptions {
                        line_width: width,
                        ..Default::default()
                    },
                );
                checked += 1;
            }
        }
    }
    assert!(checked > 100);
}

#[test]
fn caller_built_error_nodes_and_tokens_are_rejected_without_diagnostics() {
    use gdfmt_syntax::{GdscriptLanguage, LosslessTree, SyntaxKind as K};
    use rowan::{GreenNodeBuilder, Language};
    for error_kind in [K::ErrorNode, K::Error] {
        for as_node in [false, true] {
            let mut builder = GreenNodeBuilder::new();
            builder.start_node(GdscriptLanguage::kind_to_raw(K::SourceFile));
            builder.start_node(GdscriptLanguage::kind_to_raw(K::CallExpr));
            if as_node {
                builder.start_node(GdscriptLanguage::kind_to_raw(error_kind));
                builder.token(GdscriptLanguage::kind_to_raw(K::Number), "1");
                builder.finish_node();
            } else {
                builder.token(GdscriptLanguage::kind_to_raw(error_kind), "bad");
            }
            builder.finish_node();
            builder.finish_node();
            let tree = LosslessTree::from_green(builder.finish());
            assert_eq!(
                format(&tree, &[], &FormatOptions::default()),
                Err(FormatError::InvalidSyntax)
            );
        }
    }
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

#[test]
fn structural_validation_checks_topology_even_when_all_source_tokens_match() {
    use gdfmt_syntax::{LosslessTree, Token};
    let source = "func run():\n\treturn [1, 2]\n";
    let parsed = parse(source, &ParseOptions::default());
    assert!(!parsed.has_errors());
    let nested = parsed.tree.unwrap();
    let tokens: Vec<_> = nested
        .root()
        .descendants_with_tokens()
        .filter_map(|element| element.into_token())
        .map(|token| Token {
            kind: token.kind(),
            span: gdfmt_syntax::directives::span(&token),
        })
        .collect();
    let flat = LosslessTree::from_tokens(source, &tokens).unwrap();
    assert_eq!(nested.text(), flat.text());
    assert!(!structurally_equivalent(&nested, &flat));
    assert!(!structurally_equivalent(&flat, &nested));
}

#[test]
fn structural_validation_retains_literal_spelling_and_comment_ownership_checks() {
    let sources = [
        "var value = [1, 2] # note\n",
        "var  value=[1,2] # note\n",
        "var value = [1, 3] # note\n",
        "var value = [1, 2] # changed\n",
        "var value = [1, # note\n2]\n",
        "var value = [1,\n# note\n2]\n",
    ];
    let trees: Vec<_> = sources
        .iter()
        .map(|source| {
            let parsed = parse(source, &ParseOptions::default());
            assert!(!parsed.has_errors());
            parsed.tree.unwrap()
        })
        .collect();
    for (left, before) in trees.iter().enumerate() {
        for (right, after) in trees.iter().enumerate() {
            assert_eq!(
                structurally_equivalent(before, after),
                left == right || (left < 2 && right < 2),
                "{left}/{right}"
            );
        }
    }
}

#[test]
fn long_comment_runs_keep_suite_anchors_across_line_endings_and_dedents() {
    let comments: String = (0..128)
        .map(|index| format!("\t# 中文 marker {index}\n"))
        .collect();
    let source = format!(
        "func first():\n\tvar label = \"中文\"\n\tpass\n{comments}\nfunc second():\n\tpass\n\t# eof\n"
    );
    let before = parse(&source, &ParseOptions::default());
    assert!(!before.has_errors());
    let before = before.tree.unwrap();
    for equivalent in [source.replace('\n', "\r\n"), source.replace('\t', "    ")] {
        let parsed = parse(&equivalent, &ParseOptions::default());
        assert!(!parsed.has_errors());
        assert!(structurally_equivalent(&before, &parsed.tree.unwrap()));
    }
    for marker in ["\t# 中文 marker 64", "\t# eof"] {
        let changed = source.replacen(marker, marker.trim_start(), 1);
        let parsed = parse(&changed, &ParseOptions::default());
        assert!(!parsed.has_errors());
        assert!(!structurally_equivalent(&before, &parsed.tree.unwrap()));
    }
}

#[test]
fn collection_layouts_are_independent_and_empty_lists_stay_compact() {
    use gdfmt_formatter::{CollectionLayout, DictionarySpacing};
    let source = "var a = [1,2]\nvar d = {name=1, 'other'=2}\nenum E {A,B,}\nvar empty = [[],{}]\n";
    let options = FormatOptions {
        array_layout: CollectionLayout::Multiline,
        dictionary_layout: CollectionLayout::Multiline,
        enum_layout: CollectionLayout::Multiline,
        dictionary_spacing: DictionarySpacing::Spaced,
        ..Default::default()
    };
    assert_eq!(
        script(source, &options),
        "var a = [\n\t1,\n\t2\n]\nvar d = {\n\tname = 1,\n\t'other' = 2\n}\nenum E {\n\tA,\n\tB,\n}\nvar empty = [\n\t[],\n\t{}\n]\n"
    );
    let options = FormatOptions {
        dictionary_spacing: DictionarySpacing::Spaced,
        ..Default::default()
    };
    assert_eq!(
        script(source, &options),
        "var a = [1, 2]\nvar d = { name = 1, 'other' = 2 }\nenum E {A, B,}\nvar empty = [[], {}]\n"
    );
    for policy in [CollectionLayout::Auto, CollectionLayout::Preserve] {
        let options = FormatOptions {
            array_layout: policy,
            dictionary_layout: policy,
            enum_layout: policy,
            ..Default::default()
        };
        let output = script(
            "var a = [\n  1,2\n]\nvar d = {\n  'k': 1\n}\nenum E {\n A,B\n}\n",
            &options,
        );
        assert_eq!(output.contains("[\n"), policy == CollectionLayout::Preserve);
        assert_eq!(output.contains("{\n"), policy == CollectionLayout::Preserve);
        assert_eq!(script("var a = [1,2]\n", &options), "var a = [1, 2]\n");
    }
}

#[test]
fn existing_trailing_commas_expand_lists_without_inserting_tokens() {
    use gdfmt_formatter::TrailingCommaLayout;
    let options = FormatOptions {
        trailing_comma_layout: TrailingCommaLayout::Multiline,
        ..Default::default()
    };
    let source = "func f(a,b,): return call([1,2,], {'k': 3,},)\n";
    assert_eq!(
        script(source, &options),
        "func f(\n\ta,\n\tb,\n):\n\treturn call(\n\t\t[\n\t\t\t1,\n\t\t\t2,\n\t\t],\n\t\t{\n\t\t\t'k': 3,\n\t\t},\n\t)\n"
    );
    assert_eq!(
        script("var a = [1,2]\nvar b = []\n", &options),
        "var a = [1, 2]\nvar b = []\n"
    );
}

#[test]
fn definition_spacing_keeps_annotation_comment_groups_and_class_boundaries() {
    let source = "var x=1\n## Documentation\n@warning_ignore(\"unused_parameter\")\nfunc f(value): pass\nclass Inner:\n    var y=2\n    func run(): pass\n    var z=3\nvar end=4\n";
    for count in [0, 1, 2, 5] {
        for max_blank_lines in [0, 2, 5] {
            let options = FormatOptions {
                blank_lines_around_definitions: Some(count),
                max_blank_lines,
                ..Default::default()
            };
            let gap = "\n".repeat(usize::from(count) + 1);
            let output = script(source, &options);
            assert_eq!(
                output,
                format!(
                    "var x = 1{gap}## Documentation\n@warning_ignore(\"unused_parameter\")\nfunc f(value):\n\tpass{gap}class Inner:\n\tvar y = 2{gap}\tfunc run():\n\t\tpass{gap}\tvar z = 3{gap}var end = 4\n"
                )
            );
        }
    }
}

#[test]
fn multiline_lambdas_comments_and_typed_collections_keep_valid_layout() {
    use gdfmt_formatter::{
        CollectionLayout, DictionarySpacing, LambdaBodyStyle, TrailingCommaLayout,
    };
    let sources = [
        "var callback = func(x): return x+1\n",
        "var callbacks = [func(x): return x+1, func(): pass,]\n",
        "var callbacks = {name = func(x): return x+1,}\n",
        "func f():\n    call(func(x): return x+1, 2,)\n",
        "var values: Dictionary[String, int] = {'a': 1,} # tail\n",
        "var values = [1, # first\n    # leading\n    2,\n    # dangling\n] # after\n",
        "func f():\n    pass # body\n    # dangling\n## next\nfunc g(): pass\n",
    ];
    for width in [20, 40, 100] {
        for policy in [
            CollectionLayout::Auto,
            CollectionLayout::Multiline,
            CollectionLayout::Preserve,
        ] {
            for source in sources {
                let options = FormatOptions {
                    line_width: width,
                    array_layout: policy,
                    dictionary_layout: policy,
                    enum_layout: policy,
                    dictionary_spacing: DictionarySpacing::Spaced,
                    trailing_comma_layout: TrailingCommaLayout::Multiline,
                    lambda_body_style: LambdaBodyStyle::Multiline,
                    blank_lines_around_definitions: Some(2),
                    inline_comment_spacing: 4,
                    ..Default::default()
                };
                let output = script(source, &options);
                if source.contains("# tail") {
                    assert!(output.contains("    # tail"));
                }
                if source.contains("var callback =") {
                    assert!(output.contains(":\n\treturn x + 1"));
                }
            }
        }
    }
}

#[test]
fn layout_option_bounds_are_checked_for_direct_rust_callers() {
    let parsed = parse("var a = 1\n", &ParseOptions::default());
    let tree = parsed.tree.unwrap();
    for options in [
        FormatOptions {
            blank_lines_around_definitions: Some(6),
            ..Default::default()
        },
        FormatOptions {
            inline_comment_spacing: 0,
            ..Default::default()
        },
        FormatOptions {
            inline_comment_spacing: 5,
            ..Default::default()
        },
    ] {
        assert_eq!(
            format(&tree, &parsed.diagnostics, &options),
            Err(FormatError::InvalidOptions)
        );
    }
}

#[test]
fn preserve_layout_still_wraps_wide_input_and_definitions_do_not_change_local_gaps() {
    use gdfmt_formatter::CollectionLayout;
    let options = FormatOptions {
        line_width: 20,
        array_layout: CollectionLayout::Preserve,
        dictionary_layout: CollectionLayout::Auto,
        enum_layout: CollectionLayout::Auto,
        blank_lines_around_definitions: Some(5),
        max_blank_lines: 1,
        ..Default::default()
    };
    assert_eq!(
        script("var a = [alpha,beta,gamma]\n", &options),
        "var a = [\n\talpha,\n\tbeta,\n\tgamma\n]\n"
    );
    assert_eq!(
        script("func f():\n    var a=1\n\n\n\n    var b=2\n", &options),
        "func f():\n\tvar a = 1\n\n\tvar b = 2\n"
    );
    assert_eq!(
        script("var d = {'a':1}\nenum E {A,B}\n", &options),
        "var d = {'a': 1}\nenum E {A, B}\n"
    );
}

#[test]
fn nested_inline_lambda_endings_keep_statement_indentation() {
    use gdfmt_formatter::CollectionLayout;
    let options = FormatOptions {
        dictionary_layout: CollectionLayout::Multiline,
        ..Default::default()
    };
    assert_eq!(
        script("var a = [{k = func(): pass}]\n", &options),
        "var a = [\n\t{\n\t\tk = func(): pass\n}\n]\n"
    );
}

#[test]
fn comments_before_inline_lambda_separators_keep_statement_indentation() {
    assert_eq!(
        script(
            "var a = [func(): pass\n# leading\n,]\n",
            &FormatOptions::default()
        ),
        "var a = [\n\tfunc(): pass\n# leading\n,\n]\n"
    );
}
