//! Shared JSON contract for JavaScript hosts. No file discovery or I/O.
use crate::{ConfigError, PartialConfig, PartialFormatOptions, PartialLanguageOptions};
use gdfmt_text::{Diagnostic, SourceId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TextOptions {
    #[serde(default)]
    pub language: PartialLanguageOptions,
    #[serde(default)]
    pub format: PartialFormatOptions,
    #[serde(default)]
    pub source_id: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextResult {
    pub schema_version: u32,
    pub span_encoding: &'static str,
    pub output: Option<String>,
    pub changed: bool,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, thiserror::Error)]
pub enum TextOptionsError {
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Configuration(#[from] ConfigError),
}

pub fn format_text(source: &str, options: TextOptions) -> Result<TextResult, ConfigError> {
    let config = PartialConfig {
        language: Some(options.language),
        format: Some(options.format),
        ..Default::default()
    }
    .resolve()?;
    let result = gdfmt::format(
        source,
        &crate::format_request(&config, SourceId(options.source_id)),
    );
    Ok(TextResult {
        schema_version: 1,
        span_encoding: "utf8Bytes",
        output: result.output,
        changed: result.changed,
        diagnostics: result.diagnostics,
    })
}

/// Invalid arguments are errors; invalid source is a successful result with no output.
pub fn format_text_json(source: &str, options: &str) -> Result<String, TextOptionsError> {
    let options = serde_json::from_str(options)?;
    Ok(serde_json::to_string(&format_text(source, options)?)?)
}

fn text_config(config: &PartialConfig) -> Result<(), ConfigError> {
    for (field, present) in [
        ("overrides", config.overrides.is_some()),
        ("ignore", config.ignore.is_some()),
    ] {
        if present {
            return Err(ConfigError { field: field.into(), message: "not supported by the browser text configuration adapter; use the desktop formatter".into() });
        }
    }
    Ok(())
}

/// Parses one JSON/JSONC layer. The host loads extends using its filesystem API.
pub fn parse_text_config_json(source: &str) -> Result<String, TextOptionsError> {
    if source.len() > 1024 * 1024 {
        return Err(ConfigError {
            field: "$".into(),
            message: "configuration exceeds 1 MiB".into(),
        }
        .into());
    }
    let config = crate::parse_config(source)?;
    text_config(&config)?;
    Ok(serde_json::to_string(&config)?)
}

/// Resolves already flattened language/format layers with the shared defaults.
pub fn resolve_text_options_json(layers: &str) -> Result<String, TextOptionsError> {
    let layers: Vec<PartialConfig> = serde_json::from_str(layers)?;
    if layers.len() > 128 {
        return Err(ConfigError {
            field: "$".into(),
            message: "configuration layer limit exceeded".into(),
        }
        .into());
    }
    let mut merged = PartialConfig::default();
    for layer in layers {
        text_config(&layer)?;
        if layer.extends.is_some() {
            return Err(ConfigError {
                field: "extends".into(),
                message: "load and flatten inherited files before resolving text options".into(),
            }
            .into());
        }
        layer.resolve()?;
        merged.merge(&layer);
    }
    let config = merged.resolve()?;
    Ok(serde_json::to_string(
        &serde_json::json!({ "language": config.language, "format": config.format }),
    )?)
}
