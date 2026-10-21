use gdfmt_config::{PartialConfig, parse_config, schema_json};

#[test]
fn merge_preserves_unset_fields_and_explicit_false_and_zero() {
    let mut parent =
        parse_config(r#"{"format":{"lineWidth":120,"finalNewline":true,"maxBlankLines":3}}"#)
            .unwrap();
    let child = parse_config(r#"{"format":{"finalNewline":false,"maxBlankLines":0}}"#).unwrap();
    parent.merge(&child);
    let resolved = parent.resolve().unwrap();
    assert_eq!(resolved.format.line_width, 120);
    assert!(!resolved.format.final_newline);
    assert_eq!(resolved.format.max_blank_lines, 0);
    assert_eq!(resolved.format.indent_width, 4);
    assert_eq!(
        PartialConfig::default()
            .resolve()
            .unwrap()
            .format
            .line_width,
        100
    );
}

#[test]
fn runtime_and_schema_agree_on_configuration_matrix() {
    let schema: serde_json::Value = serde_json::from_str(&schema_json()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let cases = [
        (r#"{"lint":{}}"#, false),
        (r#"{"rules":{}}"#, false),
        (r#"{"plugins":[]}"#, false),
        (r#"{}"#, true),
        (r#"{"extends":[],"ignore":[],"overrides":[]}"#, true),
        (
            r#"{"extends":["./base.json"],"ignore":["generated/","!keep.gd"],"overrides":[{"files":["**/*.gd"],"exclude":[],"format":{"maxBlankLines":0,"finalNewline":false},"language":{"godotVersion":"4.3"}}]}"#,
            true,
        ),
        (r#"{"overrides":[{"files":[]}] }"#, true),
        (r#"{"extends":null}"#, false),
        (r#"{"extends":"base.json"}"#, false),
        (r#"{"extends":[null]}"#, false),
        (r#"{"ignore":null}"#, false),
        (r#"{"ignore":[false]}"#, false),
        (r#"{"overrides":null}"#, false),
        (r#"{"overrides":[{}]}"#, false),
        (r#"{"overrides":[{"files":null}]}"#, false),
        (r#"{"overrides":[{"files":[],"exclude":null}]}"#, false),
        (
            r#"{"overrides":[{"files":[],"format":{"lineWidth":0}}]}"#,
            false,
        ),
        (r#"{"overrides":[{"files":[],"format":null}]}"#, false),
        (
            r#"{"overrides":[{"files":[],"language":{"godotVersion":"latest"}}]}"#,
            false,
        ),
        (r#"{"overrides":[{"files":[],"extends":[]}]}"#, false),
        (r#"{"overrides":[{"files":[],"ignore":[]}]}"#, false),
        (r#"{"overrides":[{"files":[],"unknown":true}]}"#, false),
        (r#"{"language":{}}"#, true),
        (r#"{"language":{"godotVersion":"4.3"}}"#, true),
        (r#"{"language":{"godotVersion":"4.4"}}"#, true),
        (r#"{"language":{"godotVersion":"4.5"}}"#, true),
        (r#"{"language":{"godotVersion":"4.6"}}"#, true),
        (r#"{"language":{"godotVersion":"4.7"}}"#, true),
        (r#"{"language":null}"#, false),
        (r#"{"language":{"godotVersion":null}}"#, false),
        (r#"{"language":{"godotVersion":4.5}}"#, false),
        (r#"{"language":{"godotVersion":"latest"}}"#, false),
        (r#"{"language":{"godotVersion":"4.8"}}"#, false),
        (r#"{"language":{"godotVersion":"4.7.2"}}"#, false),
        (r#"{"language":{"godotVersion":"4.2"}}"#, false),
        (r#"{"language":{"unknown":true}}"#, false),
        (r#"{"schemaVersion":1,"format":{}}"#, true),
        (
            r#"{"root":false,"format":{"lineWidth":40,"indentWidth":8,"maxBlankLines":0,"finalNewline":false,"lineEnding":"preserve","indentStyle":"space"}}"#,
            true,
        ),
        (r#"{"schemaVersion":2}"#, false),
        (r#"{"schemaVersion":null}"#, false),
        (r#"{"root":null}"#, false),
        (r#"{"format":null}"#, false),
        (r#"{"$schema":null}"#, false),
        (r#"{"format":{"lineWidth":null}}"#, false),
        (r#"{"format":{"lineWidth":39}}"#, false),
        (r#"{"format":{"lineWidth":321}}"#, false),
        (r#"{"format":{"indentWidth":0}}"#, false),
        (r#"{"format":{"indentWidth":9}}"#, false),
        (r#"{"format":{"maxBlankLines":6}}"#, false),
        (r#"{"format":{"lineEnding":"auto"}}"#, false),
        (r#"{"unknown":1}"#, false),
        (r#"{"format":{"unknown":true}}"#, false),
        (r#"{"format":{"lineWidth":"100"}}"#, false),
    ];
    for (source, valid) in cases {
        let value: serde_json::Value = serde_json::from_str(source).unwrap();
        assert_eq!(parse_config(source).is_ok(), valid, "runtime: {source}");
        assert_eq!(validator.is_valid(&value), valid, "schema: {source}");
    }
    assert!(parse_config("{} {}").is_err());
    assert!(
        parse_config(r#"{"format":{"indentStyle":null}}"#)
            .unwrap_err()
            .field
            .contains("indentStyle")
    );
}

#[test]
fn language_merging_preserves_inheritance_and_uses_shared_parser_profile() {
    use gdfmt_config::LanguageVersion;
    let mut parent = parse_config(r#"{"language":{"godotVersion":"4.3"}}"#).unwrap();
    parent.merge(&parse_config(r#"{"language":{}}"#).unwrap());
    assert_eq!(
        parent.resolve().unwrap().language.godot_version,
        LanguageVersion::Godot4_3
    );
    parent.merge(&parse_config(r#"{"language":{"godotVersion":"4.5"}}"#).unwrap());
    assert_eq!(
        parent.resolve().unwrap().language.godot_version,
        LanguageVersion::Godot4_5
    );
    assert_eq!(
        PartialConfig::default()
            .resolve()
            .unwrap()
            .language
            .godot_version,
        LanguageVersion::Godot4_7
    );
}

#[test]
fn ordinary_arrays_replace_but_overrides_append_even_with_empty_child_arrays() {
    let mut parent = parse_config(r#"{"extends":["base.json"],"ignore":["old"],"overrides":[{"files":["**/*.gd"],"format":{"lineWidth":120}}]}"#).unwrap();
    parent.merge(&parse_config(r#"{"extends":[],"ignore":[],"overrides":[]}"#).unwrap());
    assert_eq!(parent.extends, Some(vec![]));
    assert_eq!(parent.ignore, Some(vec![]));
    assert_eq!(parent.overrides.as_ref().unwrap().len(), 1);
    parent.merge(
        &parse_config(r#"{"overrides":[{"files":[],"format":{"finalNewline":false}}]}"#).unwrap(),
    );
    assert_eq!(parent.overrides.as_ref().unwrap().len(), 2);
}
