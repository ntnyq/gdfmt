use gdfmt_config::{PartialConfig, parse_config, schema_json};

#[test]
fn jsonc_accepts_comments_and_trailing_commas_without_changing_strings() {
    let source = r#"// 项目配置
{
    "$schema": "https://example.test/schema.json#/*schema*/",
    /* Keep escaped quotes, slashes and comment markers inside strings. */
    "extends": ["base.jsonc", /* inherited */],
    "ignore": ["路径//文件", "/*literal*/", "quote\"//literal", "backslash\\",],
    "format": {"lineWidth": 120, "finalNewline": false, /* trailing */},
    "overrides": [{"files": ["**/*.gd",], "format": {"maxBlankLines": 0,},},],
} // no final newline"#;
    let parsed = parse_config(source).unwrap();
    assert_eq!(
        parsed.schema.as_deref(),
        Some("https://example.test/schema.json#/*schema*/")
    );
    assert_eq!(parsed.extends.unwrap(), ["base.jsonc"]);
    assert_eq!(
        parsed.ignore.unwrap(),
        [
            "路径//文件",
            "/*literal*/",
            "quote\"//literal",
            "backslash\\"
        ]
    );
    let format = parsed.format.unwrap();
    assert_eq!(format.line_width, Some(120));
    assert_eq!(format.final_newline, Some(false));
    assert_eq!(
        parsed.overrides.unwrap()[0]
            .format
            .as_ref()
            .unwrap()
            .max_blank_lines,
        Some(0)
    );
    for source in [
        "/* empty */ {} /* end */",
        "{\r\n// comment\r\n}",
        "{/**/}",
        "{\"ignore\": [/* empty */],}",
    ] {
        assert!(parse_config(source).is_ok(), "{source}");
    }
}

#[test]
fn jsonc_rejects_malformed_syntax_and_preserves_field_validation() {
    for source in [
        "{} /* unterminated",
        "/* unterminated",
        "{} /",
        "{} // comment\n {}",
        "// only a comment",
        "{,}",
        r#"{"ignore":[,]}"#,
        r#"{"ignore":["a",,]}"#,
        r#"{"format":{"lineWidth":120,,}}"#,
        r#"{"root":tr/* comment */ue}"#,
        r#"{"format":{"lineWidth":1/* comment */00}}"#,
        r#"{'root':true}"#,
        "{root:true}",
        "{# shell comment\n}",
        r#"{"root":true,"root":false,}"#,
        r#"{"format":{"lineWidth":120,"lineWidth":100,}}"#,
    ] {
        assert!(parse_config(source).is_err(), "{source}");
    }
    for (source, field) in [
        ("{/* comment */\"unknown\":true,}", "unknown"),
        (r#"{"format":{"lineWidth":null,},}"#, "format.lineWidth"),
        (r#"{"format":{"lineWidth":"100",},}"#, "format.lineWidth"),
        (r#"{"format":{"lineWidth":39,},}"#, "format.lineWidth"),
        (r#"{"schemaVersion":2,}"#, "schemaVersion"),
        (
            r#"{"language":{"godotVersion":"latest",},}"#,
            "language.godotVersion",
        ),
    ] {
        assert_eq!(parse_config(source).unwrap_err().field, field, "{source}");
    }
}

#[test]
fn jsonc_diagnostics_keep_original_lines_and_columns_after_comments() {
    let source = "{\r\n/* 多行\r\n注释 */\r\n\"format\": {\"lineWidth\": \"bad\"},\r\n}";
    let error = parse_config(source).unwrap_err();
    assert_eq!(error.field, "format.lineWidth");
    assert!(error.message.ends_with("at line 4 column 29"), "{error}");
}

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
fn integer_number_spellings_match_schema_in_base_fields_and_overrides() {
    let schema: serde_json::Value = serde_json::from_str(&schema_json()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let check = |field: &str, value: &str, expected: Option<u64>| {
        for in_override in [false, true] {
            let source = if in_override {
                format!(
                    r#"{{"overrides":[{{"files":["**/*.gd"],"format":{{"{field}":{value}}}}}]}}"#
                )
            } else {
                format!(r#"{{"format":{{"{field}":{value}}}}}"#)
            };
            let valid = expected.is_some();
            assert_eq!(
                validator.is_valid(&serde_json::from_str(&source).unwrap()),
                valid,
                "schema: {source}"
            );
            let parsed = parse_config(&source);
            assert_eq!(parsed.is_ok(), valid, "runtime: {source}");
            if let Some(expected) = expected {
                let serialized = serde_json::to_value(parsed.unwrap()).unwrap();
                let format = if in_override {
                    &serialized["overrides"][0]["format"]
                } else {
                    &serialized["format"]
                };
                assert_eq!(format[field].as_u64(), Some(expected), "{source}");
            } else {
                let prefix = if in_override {
                    "overrides[0].format"
                } else {
                    "format"
                };
                assert_eq!(
                    parsed.unwrap_err().field,
                    format!("{prefix}.{field}"),
                    "{source}"
                );
            }
        }
    };
    for (field, min, max, default) in [
        ("lineWidth", 40_u64, 320, 100),
        ("indentWidth", 1, 8, 4),
        ("maxBlankLines", 0, 5, 2),
        ("inlineCommentSpacing", 1, 4, 1),
        ("blankLinesAroundDefinitions", 0, 5, 2),
    ] {
        for number in [min, max, default] {
            for value in [
                number.to_string(),
                format!("{number}.0"),
                format!("{number}e0"),
                format!("{}e-1", number * 10),
            ] {
                check(field, &value, Some(number));
            }
        }
        for value in [
            format!("{default}.5"),
            format!("{}e0", max + 1),
            "-1.0".into(),
            "1e99".into(),
            "1e-99".into(),
            "65536.0".into(),
            "18446744073709551615".into(),
            "18446744073709551616.0".into(),
            "null".into(),
            "true".into(),
            "\"100\"".into(),
            "[]".into(),
            "{}".into(),
        ] {
            check(field, &value, None);
        }
        if min > 0 {
            check(field, &format!("{}.0", min - 1), None);
        } else {
            check(field, "-0.0", Some(0));
        }
    }
    for (value, valid) in [
        ("1", true),
        ("1.0", true),
        ("1e0", true),
        ("10e-1", true),
        ("0.0", false),
        ("2e0", false),
        ("1.5", false),
        ("1e99", false),
        ("null", false),
        ("\"1\"", false),
    ] {
        let source = format!(r#"{{"schemaVersion":{value}}}"#);
        assert_eq!(
            validator.is_valid(&serde_json::from_str(&source).unwrap()),
            valid,
            "schema: {source}"
        );
        let parsed = parse_config(&source);
        assert_eq!(parsed.is_ok(), valid, "runtime: {source}");
        if valid {
            assert_eq!(parsed.unwrap().resolve().unwrap().schema_version, 1);
        } else {
            assert_eq!(parsed.unwrap_err().field, "schemaVersion", "{source}");
        }
    }
}

#[test]
fn integer_spellings_preserve_defaults_merging_and_duplicate_key_rejection() {
    let mut config = parse_config(r#"{"schemaVersion":1.0,"format":{"lineWidth":1e2,"indentWidth":4.0,"maxBlankLines":2.0,"inlineCommentSpacing":1e0}}"#).unwrap();
    assert_eq!(
        config.resolve().unwrap(),
        PartialConfig::default().resolve().unwrap()
    );
    config.merge(&parse_config(r#"{"format":{"maxBlankLines":0.0,"blankLinesAroundDefinitions":0e0,"finalNewline":false}}"#).unwrap());
    let resolved = config.resolve().unwrap();
    assert_eq!(resolved.format.line_width, 100);
    assert_eq!(resolved.format.max_blank_lines, 0);
    assert_eq!(
        resolved.format.blank_lines_around_definitions,
        gdfmt_config::BlankLinesAroundDefinitions::Count(0)
    );
    assert!(!resolved.format.final_newline);
    for source in [
        r#"{"schemaVersion":1.0,"schemaVersion":1}"#,
        r#"{"format":{"lineWidth":100.0,"lineWidth":1e2}}"#,
        r#"{"overrides":[{"files":[],"format":{"indentWidth":2.0,"indentWidth":4.0}}]}"#,
    ] {
        assert!(parse_config(source).is_err(), "{source}");
    }
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

#[test]
fn layout_options_validate_merge_and_match_schema() {
    let schema: serde_json::Value = serde_json::from_str(&schema_json()).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let fields = [
        (
            "arrayLayout",
            vec!["\"auto\"", "\"multiline\"", "\"preserve\""],
        ),
        (
            "dictionaryLayout",
            vec!["\"auto\"", "\"multiline\"", "\"preserve\""],
        ),
        (
            "enumLayout",
            vec!["\"auto\"", "\"multiline\"", "\"preserve\""],
        ),
        ("dictionarySpacing", vec!["\"compact\"", "\"spaced\""]),
        ("trailingCommaLayout", vec!["\"auto\"", "\"multiline\""]),
        (
            "blankLinesAroundDefinitions",
            vec!["\"preserve\"", "0", "1", "5"],
        ),
        ("lambdaBodyStyle", vec!["\"auto\"", "\"multiline\""]),
        ("inlineCommentSpacing", vec!["1", "2", "4"]),
    ];
    for (field, values) in fields {
        for value in values.iter().copied().chain([
            "null",
            "true",
            "-1",
            "0.5",
            "6",
            "\"unknown\"",
            "{}",
            "[]",
        ]) {
            let valid = values.contains(&value);
            let source = format!(r#"{{"format":{{"{field}":{value}}}}}"#);
            let parsed = parse_config(&source);
            assert_eq!(parsed.is_ok(), valid, "{source}");
            assert_eq!(
                validator.is_valid(&serde_json::from_str(&source).unwrap()),
                valid,
                "{source}"
            );
            if !valid {
                assert_eq!(parsed.unwrap_err().field, format!("format.{field}"));
            }
            let nested =
                format!(r#"{{"overrides":[{{"files":[],"format":{{"{field}":{value}}}}}]}}"#);
            assert_eq!(parse_config(&nested).is_ok(), valid, "{nested}");
            assert_eq!(
                validator.is_valid(&serde_json::from_str(&nested).unwrap()),
                valid,
                "{nested}"
            );
        }
    }
    let mut parent = parse_config(r#"{"format":{"arrayLayout":"multiline","dictionaryLayout":"preserve","enumLayout":"multiline","dictionarySpacing":"spaced","trailingCommaLayout":"multiline","blankLinesAroundDefinitions":5,"lambdaBodyStyle":"multiline","inlineCommentSpacing":4}}"#).unwrap();
    parent.merge(&parse_config(r#"{"format":{}}"#).unwrap());
    let resolved = serde_json::to_value(parent.resolve().unwrap()).unwrap();
    assert_eq!(resolved["format"]["dictionaryLayout"], "preserve");
    parent.merge(&parse_config(r#"{"format":{"arrayLayout":"auto","dictionaryLayout":"auto","enumLayout":"auto","dictionarySpacing":"compact","trailingCommaLayout":"auto","blankLinesAroundDefinitions":0,"lambdaBodyStyle":"auto","inlineCommentSpacing":1}}"#).unwrap());
    let mut expected = serde_json::to_value(PartialConfig::default().resolve().unwrap()).unwrap();
    expected["format"]["blankLinesAroundDefinitions"] = 0.into();
    assert_eq!(
        serde_json::to_value(parent.resolve().unwrap()).unwrap(),
        expected
    );
    parent
        .merge(&parse_config(r#"{"format":{"blankLinesAroundDefinitions":"preserve"}}"#).unwrap());
    assert_eq!(
        parent.resolve().unwrap(),
        PartialConfig::default().resolve().unwrap()
    );
}
