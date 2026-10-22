//! Thin browser binding; validation and formatting use the same contract as NAPI.
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn format_text(source: &str, options: &str) -> Result<String, JsError> {
    gdfmt_config::text::format_text_json(source, options)
        .map_err(|error| JsError::new(&error.to_string()))
}

#[wasm_bindgen]
pub fn parse_config(source: &str) -> Result<String, JsError> {
    gdfmt_config::text::parse_text_config_json(source)
        .map_err(|error| JsError::new(&error.to_string()))
}

#[wasm_bindgen]
pub fn resolve_text_options(layers: &str) -> Result<String, JsError> {
    gdfmt_config::text::resolve_text_options_json(layers)
        .map_err(|error| JsError::new(&error.to_string()))
}
