use gdfmt::{FormatRequest, LanguageVersion, format};
use gdfmt_parser::{parse, parse_expression};

#[test]
fn complete_valid_corpus_reparses_preserves_structure_and_is_idempotent() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/compatibility/manifest.json")).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/compatibility");
    let mut formatted_cases = 0;
    for fixture in manifest["fixtures"].as_array().unwrap() {
        if fixture["stage"] != "script" {
            continue;
        }
        let file = fixture["file"].as_str().unwrap();
        let source = std::fs::read_to_string(root.join(file)).unwrap();
        for version in LanguageVersion::ALL {
            let mut request = FormatRequest::default();
            request.parse.language_version = version;
            for (width, indent, ending, blanks, final_newline) in [
                (40, "\t", gdfmt::LineEnding::Preserve, 2, true),
                (100, "\t", gdfmt::LineEnding::Preserve, 2, true),
                (40, "  ", gdfmt::LineEnding::Crlf, 0, false),
                (100, "    ", gdfmt::LineEnding::Lf, 1, true),
            ] {
                request.format.line_width = width;
                request.format.indent = indent.into();
                request.format.line_ending = ending;
                request.format.max_blank_lines = blanks;
                request.format.final_newline = final_newline;
                let parsed = parse(&source, &request.parse);
                let result = format(&source, &request);
                if parsed.has_errors() {
                    assert!(result.output.is_none(), "{file}/{version}");
                    continue;
                }
                let output = result.output.unwrap_or_else(|| {
                    panic!("{file}/{version}/{width}: {:?}", result.diagnostics)
                });
                assert!(gdfmt_formatter::structurally_equivalent(
                    &parsed.tree.unwrap(),
                    &parse(&output, &request.parse).tree.unwrap()
                ));
                assert_eq!(
                    format(&output, &request).output.unwrap(),
                    output,
                    "{file}/{version}/{width}"
                );
                formatted_cases += 1;
            }
        }
    }
    assert!(formatted_cases > 2000);
}

#[test]
fn all_valid_standalone_expressions_keep_structure_and_literals() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/compatibility/manifest.json")).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/compatibility");
    for fixture in manifest["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|fixture| fixture["stage"] == "expression")
    {
        let source = std::fs::read_to_string(root.join(fixture["file"].as_str().unwrap())).unwrap();
        for version in LanguageVersion::ALL {
            let mut request = FormatRequest::default();
            request.parse.language_version = version;
            let parsed = parse_expression(&source, &request.parse);
            if parsed.has_errors() {
                continue;
            }
            let before = parsed.tree.unwrap();
            let output =
                gdfmt_formatter::format(&before, &parsed.diagnostics, &request.format).unwrap();
            let after = parse_expression(&output, &request.parse);
            assert!(!after.has_errors(), "{source} -> {output}");
            let tree = after.tree.unwrap();
            assert!(
                gdfmt_formatter::structurally_equivalent(&before, &tree),
                "{source} -> {output}"
            );
            assert_eq!(
                gdfmt_formatter::format(&tree, &after.diagnostics, &request.format).unwrap(),
                output
            );
        }
    }
}

#[test]
fn failed_output_reparse_returns_no_writable_text() {
    let source = "var a = [1,2]\n";
    let mut request = FormatRequest::default();
    request.parse.lex.max_source_bytes = source.len();
    request.parse.lex.source_id = gdfmt_text::SourceId(17);
    assert!(!parse(source, &request.parse).has_errors());
    // Adding the canonical comma space crosses the caller's parser limit.
    let result = format(source, &request);
    assert!(result.output.is_none());
    assert!(!result.changed);
    assert_eq!(result.diagnostics[0].code, "FMT001");
    assert_eq!(result.diagnostics[0].source_id, gdfmt_text::SourceId(17));
}

#[test]
fn layout_options_preserve_valid_corpus_across_profiles() {
    use gdfmt::{CollectionLayout, DictionarySpacing, LambdaBodyStyle, TrailingCommaLayout};
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/compatibility/manifest.json")).unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/compatibility");
    for fixture in manifest["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|fixture| fixture["stage"] == "script")
    {
        let file = fixture["file"].as_str().unwrap();
        let source = std::fs::read_to_string(root.join(file)).unwrap();
        for version in LanguageVersion::ALL {
            for policy in [
                CollectionLayout::Auto,
                CollectionLayout::Multiline,
                CollectionLayout::Preserve,
            ] {
                let mut request = FormatRequest::default();
                request.parse.language_version = version;
                request.format.array_layout = policy;
                request.format.dictionary_layout = policy;
                request.format.enum_layout = policy;
                request.format.dictionary_spacing = DictionarySpacing::Spaced;
                request.format.trailing_comma_layout = TrailingCommaLayout::Multiline;
                request.format.blank_lines_around_definitions = Some(2);
                request.format.max_blank_lines = 0;
                request.format.lambda_body_style = LambdaBodyStyle::Multiline;
                request.format.inline_comment_spacing = 3;
                if parse(&source, &request.parse).has_errors() {
                    continue;
                }
                let result = format(&source, &request);
                assert!(
                    result.output.is_some(),
                    "{file}/{version}/{policy:?}: {:?}",
                    result.diagnostics
                );
            }
        }
    }
}
