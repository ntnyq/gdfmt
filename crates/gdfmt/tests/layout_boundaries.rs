use gdfmt::{
    CollectionLayout, DictionarySpacing, FormatOptions, FormatRequest, LambdaBodyStyle, LineEnding,
    TrailingCommaLayout, format,
};

fn layout_cases() -> Vec<FormatOptions> {
    let policies = [
        CollectionLayout::Auto,
        CollectionLayout::Multiline,
        CollectionLayout::Preserve,
    ];
    let mut collections = Vec::new();
    for array_layout in policies {
        for dictionary_layout in policies {
            for enum_layout in policies {
                collections.push(FormatOptions {
                    array_layout,
                    dictionary_layout,
                    enum_layout,
                    ..Default::default()
                });
            }
        }
    }
    let mut cases = Vec::new();
    for collection in collections {
        for (lambda_body_style, trailing_comma_layout) in [
            (LambdaBodyStyle::Auto, TrailingCommaLayout::Auto),
            (LambdaBodyStyle::Auto, TrailingCommaLayout::Multiline),
            (LambdaBodyStyle::Multiline, TrailingCommaLayout::Auto),
            (LambdaBodyStyle::Multiline, TrailingCommaLayout::Multiline),
        ] {
            for (
                line_width,
                dictionary_spacing,
                blank_lines_around_definitions,
                inline_comment_spacing,
            ) in [
                (20, DictionarySpacing::Spaced, None, 1),
                (40, DictionarySpacing::Compact, Some(0), 2),
                (100, DictionarySpacing::Spaced, Some(2), 4),
            ] {
                cases.push(FormatOptions {
                    lambda_body_style,
                    trailing_comma_layout,
                    line_width,
                    dictionary_spacing,
                    blank_lines_around_definitions,
                    inline_comment_spacing,
                    max_blank_lines: 0,
                    ..collection.clone()
                });
            }
        }
    }
    cases
}

#[test]
fn mixed_layout_options_reparse_and_remain_idempotent() {
    let sources = [
        "var a = [func(): pass]\n",
        "var a = [{key = func(): pass}]\n",
        "func f():\n\tvar a = [{key = func(): pass}]\n\treturn a\n",
        "var a = [1, [2,3], {key=func(): pass}]\n",
        "var a = [func(): return [1,2], [3,4]]\n",
        "var a = [\"\"\"literal\n    bytes\"\"\"]\n",
        "var a = {key = func(): pass}\n",
        "var a = call(func(): pass)\n",
        "var a = [func(): return [1,2]]\n",
        "var a = {key = func(): return [1,2]}\n",
        "var a = func(): return [1,2]\n",
        "func f():\n\tvar a = [func(): pass]\n\tvar b = [2]\n",
        "var a = [\n\t1, # first\n\t# before second\n\t{'a': [2,3]},\n\t# at end\n] # end\n",
        "enum E {\n\tA, # entry\n\t# next\n\tB,\n\t# end\n}\n",
        "var a = 1\n\n## doc\n@warning_ignore(\"unused_parameter\")\n\nfunc f(x):\n\tpass # suffix\n\n\t# dangling\n\n## next\nfunc g(): pass\n",
        "class Inner:\n\tfunc f():\n\t\tpass\n\t\t# dangling\n\t# next\n\tfunc g(): pass\nfunc end(): pass\n",
    ];
    let cases = layout_cases();
    for source in sources {
        for options in cases.iter().cloned() {
            let request = FormatRequest {
                format: options,
                ..Default::default()
            };
            let result = format(source, &request);
            let output = result.output.unwrap_or_else(|| {
                panic!("{source}\n{:?}\n{:?}", request.format, result.diagnostics)
            });
            assert_eq!(
                format(&output, &request).output.as_deref(),
                Some(output.as_str())
            );
        }
    }
}

#[test]
fn protected_regions_keep_bytes_at_every_line_boundary() {
    let sources = [
        "var a=1\nfunc f():\n\tpass\n\t# tail\n## docs\nfunc g():\n\treturn [1,2,]\nvar b={key=1}\n",
        "class Inner:\n\tvar x=1\n\tfunc f():\n\t\tpass\n\t\t# tail\n\t## docs\n\tfunc g():\n\t\treturn [1,2,]\nvar b={key=1}\n",
        "var a = [\n\t1,\n\t{key=2},\n\t3,\n]\nvar b=2\n",
        "var a = [\n{key=func(): pass\n# before close\n},\n]\nvar b=2\n",
    ];
    let cases = layout_cases();
    for source in sources {
        let lines: Vec<_> = source.lines().collect();
        for begin in 0..lines.len() {
            for end in begin..lines.len() {
                let mut protected = Vec::new();
                for (index, line) in lines.iter().enumerate() {
                    if index == begin {
                        protected.push("# gdfmt: formatter-off");
                    }
                    protected.push(line);
                    if index == end {
                        protected.push("# gdfmt: formatter-on");
                    }
                }
                let source = format!("{}\n", protected.join("\n"));
                let region = format!(
                    "# gdfmt: formatter-off\n{}\n# gdfmt: formatter-on\n",
                    lines[begin..=end].join("\n")
                );
                for (index, options) in cases.iter().cloned().enumerate() {
                    if index % 9 != 0 {
                        continue;
                    }
                    let count = [None, Some(0), Some(2), Some(5)][(index / 9) % 4];
                    let request = FormatRequest {
                        format: FormatOptions {
                            line_ending: LineEnding::Crlf,
                            blank_lines_around_definitions: count,
                            ..options
                        },
                        ..Default::default()
                    };
                    let result = format(&source, &request);
                    let output = result.output.unwrap_or_else(|| {
                        panic!("{source}\n{:?}\n{:?}", request.format, result.diagnostics)
                    });
                    assert!(output.contains(&region), "{source} -> {output}");
                    assert_eq!(
                        format(&output, &request).output.as_deref(),
                        Some(output.as_str())
                    );
                }
            }
        }
    }
}

#[test]
fn comments_at_token_boundaries_keep_their_attachments() {
    let sources = [
        "var a = [1, {key=2}, func(x): return x+1,]\n",
        "func f(a,b,): return [a,b,]\n",
        "enum E {A,B,}\n",
    ];
    let cases = layout_cases();
    for source in sources {
        let parsed = gdfmt_parser::parse(source, &Default::default());
        let tree = parsed.tree.unwrap();
        for token in tree
            .root()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
        {
            if token.kind().is_trivia() || token.kind().is_virtual() {
                continue;
            }
            let offset = u32::from(token.text_range().end()) as usize;
            for comment in [" # tail\n", "\n# leading\n", "\n\n## doc\n"] {
                let candidate = format!("{}{comment}{}", &source[..offset], &source[offset..]);
                if gdfmt_parser::parse(&candidate, &Default::default()).has_errors() {
                    continue;
                }
                for (index, options) in cases.iter().cloned().enumerate() {
                    if index % 9 != 0 {
                        continue;
                    }
                    let request = FormatRequest {
                        format: options,
                        ..Default::default()
                    };
                    let result = format(&candidate, &request);
                    assert!(
                        result.output.is_some(),
                        "{candidate}\n{:?}\n{:?}",
                        request.format,
                        result.diagnostics
                    );
                }
            }
        }
    }
}

#[test]
fn inline_lambda_boundaries_work_across_profiles_indentation_and_newlines() {
    let sources = [
        "func f():\n\tvar a = [{key = func(): pass}]\n\treturn a\n",
        "var a = [func(): pass\n# leading\n,]\n",
    ];
    for language_version in gdfmt::LanguageVersion::ALL {
        for line_ending in [LineEnding::Lf, LineEnding::Crlf, LineEnding::Preserve] {
            for indent in ["\t", "  "] {
                for source in sources {
                    let request = FormatRequest {
                        parse: gdfmt::ParseOptions {
                            language_version,
                            ..Default::default()
                        },
                        format: FormatOptions {
                            line_ending,
                            indent: indent.into(),
                            dictionary_layout: CollectionLayout::Multiline,
                            inline_comment_spacing: 3,
                            ..Default::default()
                        },
                    };
                    let result = format(source, &request);
                    let output = result.output.unwrap_or_else(|| {
                        panic!(
                            "{language_version}/{line_ending:?}/{indent:?}: {:?}",
                            result.diagnostics
                        )
                    });
                    assert!(!format(&output, &request).changed);
                }
            }
        }
    }
}
