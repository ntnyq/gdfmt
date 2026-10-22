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
