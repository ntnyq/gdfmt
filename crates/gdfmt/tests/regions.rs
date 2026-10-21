use gdfmt::{FormatRequest, LanguageVersion, format};

fn verified(source: &str, request: &FormatRequest) -> String {
    let result = format(source, request);
    let output = result
        .output
        .unwrap_or_else(|| panic!("{:?}\n{source}", result.diagnostics));
    assert!(!gdfmt_parser::parse(&output, &request.parse).has_errors());
    assert_eq!(
        format(&output, request).output.as_deref(),
        Some(output.as_str())
    );
    output
}

#[test]
fn regions_preserve_exact_bytes_across_profiles_and_layout_options() {
    let raw = "# gdfmt: formatter-off\r\nvar 文本=[  1,2  ] # 😀\r\n\r\n# gdfmt: formatter-on\r\n";
    let source = format!("\u{feff}var before=1\r\n{raw}var after=2");
    for version in LanguageVersion::ALL {
        for ending in [
            gdfmt::LineEnding::Lf,
            gdfmt::LineEnding::Crlf,
            gdfmt::LineEnding::Preserve,
        ] {
            let mut request = FormatRequest::default();
            request.parse.language_version = version;
            request.format.line_ending = ending;
            request.format.max_blank_lines = 0;
            request.format.final_newline = false;
            let output = verified(&source, &request);
            assert!(output.starts_with("\u{feff}var before = 1"));
            assert!(output.contains(raw));
            assert!(output.ends_with("var after = 2"));
        }
    }
}

#[test]
fn directives_inside_literals_and_suffix_comments_are_ordinary_text() {
    let source = "var text = '''# gdfmt: formatter-off\nvar fake=1\n# gdfmt: formatter-on'''\nvar value=2 # gdfmt: formatter-off\n";
    let output = verified(source, &FormatRequest::default());
    assert!(output.contains("var value = 2 # gdfmt: formatter-off"));
    assert!(output.contains("var fake=1"));
}

#[test]
fn multiple_regions_and_eof_override_newline_policy() {
    let source = "# gdfmt: formatter-off\nvar a=1\n# gdfmt: formatter-on\nvar b=2\n# gdfmt: formatter-off\nvar c=3\n# gdfmt: formatter-on";
    let output = verified(source, &FormatRequest::default());
    assert!(output.contains("var a=1"));
    assert!(output.contains("var b = 2"));
    assert!(output.ends_with("var c=3\n# gdfmt: formatter-on"));
}

#[test]
fn nested_suite_regions_keep_surrounding_formatting_when_indentation_agrees() {
    let source = "func run():\n\tvar a=1\n\t# gdfmt: formatter-off\n\tvar b=[ 1,2 ]\n\t# gdfmt: formatter-on\n\tvar c=3\n";
    let output = verified(source, &FormatRequest::default());
    assert!(output.contains("\tvar a = 1\n"));
    assert!(output.contains("\tvar b=[ 1,2 ]\n"));
    assert!(output.contains("\tvar c = 3\n"));
}

#[test]
fn incompatible_indent_changes_preserve_enclosing_unit_conservatively() {
    let function = "func run():\n    var a=1\n    # gdfmt: formatter-off\n    var b=[ 1,2 ]\n    # gdfmt: formatter-on\n    var c=3\n";
    let source = format!("{function}var after=1\n");
    let output = verified(&source, &FormatRequest::default());
    assert!(output.starts_with(function));
    assert!(output.ends_with("var after = 1\n"));
}

#[test]
fn regions_can_cross_declarations_or_expression_lines() {
    for source in [
        "var a=[\n# gdfmt: formatter-off\n  1,    2,\n# gdfmt: formatter-on\n3]\nvar b=2\n",
        "func first():\n\t# gdfmt: formatter-off\n\tpass\n\nfunc second():\n\tpass\n\t# gdfmt: formatter-on\nvar after=2\n",
        "# gdfmt: formatter-off\n# no code here\n# gdfmt: formatter-on\nvar a=1\n",
        "# gdfmt: formatter-off\n# gdfmt: formatter-on\n",
    ] {
        verified(source, &FormatRequest::default());
    }
}

#[test]
fn lambda_property_and_match_regions_retain_their_comment_owners() {
    for source in [
        "func run():\n\tvar callback = func():\n\t\t# gdfmt: formatter-off\n\t\tpass\n\t\t# gdfmt: formatter-on\n\tvar after=1\n",
        "var value: int:\n\tget:\n\t\t# gdfmt: formatter-off\n\t\treturn  1\n\t\t# gdfmt: formatter-on\n",
        "func run(value):\n\tmatch value:\n\t\t1:\n\t\t\t# gdfmt: formatter-off\n\t\t\tpass\n\t\t\t# gdfmt: formatter-on\n\t\t_:\n\t\t\tpass\n",
    ] {
        let original = gdfmt_parser::parse(source, &Default::default())
            .tree
            .unwrap();
        let regions = gdfmt_syntax::directives::formatter_regions(&original).unwrap();
        let output = verified(source, &FormatRequest::default());
        assert!(output.contains(regions[0].slice(source).unwrap()));
    }
}

#[test]
fn invalid_directives_and_invalid_source_never_produce_output() {
    for source in [
        "# gdfmt: formatter-off\nvar a=1\n",
        "# gdfmt: formatter-on\nvar a=1\n",
        "# gdfmt: formatter-off\n# gdfmt: formatter-off\n# gdfmt: formatter-on\n",
    ] {
        let result = format(source, &FormatRequest::default());
        assert!(result.output.is_none());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "FMT002")
        );
        assert!(!result.changed);
    }
    let result = format(
        "# gdfmt: formatter-off\nvar =\n# gdfmt: formatter-on\n",
        &FormatRequest::default(),
    );
    assert!(result.output.is_none());
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.starts_with("PAR"))
    );
}

#[test]
fn directive_errors_use_source_byte_spans_and_a_bounded_diagnostic_count() {
    let mut request = FormatRequest::default();
    request.parse.lex.source_id = gdfmt_text::SourceId(42);
    request.parse.lex.max_diagnostics = 1;
    let source = "var 文本 = '😀'\n# gdfmt: formatter-on\n# gdfmt: formatter-on\n";
    let result = format(source, &request);
    assert!(result.output.is_none());
    assert_eq!(result.diagnostics.len(), 1);
    let diagnostic = &result.diagnostics[0];
    assert_eq!(diagnostic.source_id, gdfmt_text::SourceId(42));
    assert_eq!(
        diagnostic.span.slice(source).unwrap(),
        "# gdfmt: formatter-on"
    );
}
