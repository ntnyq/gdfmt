use gdfmt_config::text::format_text_json;
use serde_json::{Value, json};

#[test]
fn browser_configuration_parsing_and_merging_use_shared_validation() {
    use gdfmt_config::text::{parse_text_config_json, resolve_text_options_json};
    let config: Value = serde_json::from_str(&parse_text_config_json("{ // comment\n \"extends\":[\"base.json\"], \"root\":true, \"format\":{\"finalNewline\":false,},}").unwrap()).unwrap();
    assert_eq!(config["extends"], json!(["base.json"]));
    assert_eq!(config["root"], true);
    let resolved: Value = serde_json::from_str(&resolve_text_options_json(r#"[{"format":{"indentWidth":2,"finalNewline":true}},{"format":{"finalNewline":false,"maxBlankLines":0}}]"#).unwrap()).unwrap();
    assert_eq!(resolved["format"]["indentWidth"], 2);
    assert_eq!(resolved["format"]["finalNewline"], false);
    assert_eq!(resolved["format"]["maxBlankLines"], 0);
    for source in [
        r#"{"overrides":[]}"#,
        r#"{"ignore":[]}"#,
        r#"{"format":{"lineWidth":1}}"#,
        r#"{"root":null}"#,
        r#"{"unknown":true}"#,
    ] {
        assert!(parse_text_config_json(source).is_err());
    }
    assert!(resolve_text_options_json(r#"[{"extends":["base.json"]}]"#).is_err());
}

#[test]
fn shared_host_contract_validates_options_and_preserves_diagnostics() {
    let result: Value =
        serde_json::from_str(&format_text_json("var 名称=\"😀\"\n", r#"{"sourceId":7}"#).unwrap())
            .unwrap();
    assert_eq!(result["output"], "var 名称 = \"😀\"\n");
    assert_eq!(result["schemaVersion"], 1);
    assert_eq!(result["spanEncoding"], "utf8Bytes");
    for options in [
        r#"{"format":{"lineWidth":1}}"#,
        r#"{"cwd":"/tmp"}"#,
        r#"{"format":null}"#,
    ] {
        assert!(format_text_json("", options).is_err());
    }
    let result: Value =
        serde_json::from_str(&format_text_json("var =\n", r#"{"sourceId":7}"#).unwrap()).unwrap();
    assert_eq!(result["output"], Value::Null);
    assert_eq!(result["changed"], false);
    assert_eq!(result["diagnostics"][0]["sourceId"], json!(7));
}

#[test]
fn integer_spellings_work_in_text_formatting_and_browser_configuration_layers() {
    use gdfmt_config::text::{parse_text_config_json, resolve_text_options_json};
    let source = "{/* numbers */\"schemaVersion\":1e0,\"format\":{\"lineWidth\":1e2,\"indentWidth\":4.0,\"maxBlankLines\":2e0,\"blankLinesAroundDefinitions\":2.0,\"inlineCommentSpacing\":1.0,},}";
    let parsed = parse_text_config_json(source).unwrap();
    let config: Value = serde_json::from_str(&parsed).unwrap();
    assert_eq!(config["schemaVersion"].as_u64(), Some(1));
    assert_eq!(config["format"]["lineWidth"].as_u64(), Some(100));
    assert_eq!(
        config["format"]["blankLinesAroundDefinitions"].as_u64(),
        Some(2)
    );
    let layers = format!(
        r#"[{parsed},{{"format":{{"maxBlankLines":0.0,"blankLinesAroundDefinitions":0e0,"finalNewline":false}}}}]"#
    );
    let resolved: Value =
        serde_json::from_str(&resolve_text_options_json(&layers).unwrap()).unwrap();
    assert_eq!(resolved["format"]["maxBlankLines"], 0);
    assert_eq!(resolved["format"]["blankLinesAroundDefinitions"], 0);
    assert_eq!(resolved["format"]["finalNewline"], false);
    let options = r#"{"format":{"lineWidth":1e2,"indentWidth":4.0,"maxBlankLines":2.0,"inlineCommentSpacing":1.0}}"#;
    assert_eq!(
        format_text_json("func f(): pass\n", options).unwrap(),
        format_text_json("func f(): pass\n", "{}").unwrap()
    );
    for source in [
        r#"{"format":{"lineWidth":100.5}}"#,
        r#"{"format":{"indentWidth":9.0}}"#,
        r#"{"format":{"blankLinesAroundDefinitions":2.5}}"#,
        r#"{"format":{"maxBlankLines":null}}"#,
        r#"{"format":{"inlineCommentSpacing":"1"}}"#,
        r#"{"format":{"lineWidth":1e2,"lineWidth":100.0}}"#,
    ] {
        assert!(parse_text_config_json(source).is_err(), "{source}");
        assert!(
            resolve_text_options_json(&format!("[{source}]")).is_err(),
            "{source}"
        );
        assert!(format_text_json("var x=1\n", source).is_err(), "{source}");
    }
    assert!(
        resolve_text_options_json(
            r#"[{"format":{"lineWidth":100.5}},{"format":{"lineWidth":100.0}}]"#
        )
        .is_err()
    );
}
