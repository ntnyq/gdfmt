//! Node boundary only: owned UTF-8 text/JSON crosses into libuv workers.
//! Syntax, configuration, validated formatting and CLI behavior stay in Rust.
use gdfmt_config::{PartialConfig, PartialFormatOptions, PartialLanguageOptions};
use gdfmt_text::{Diagnostic, SourceId};
use napi::{Env, Error, Result, Status, Task, bindgen_prelude::AsyncTask};
use napi_derive::napi;
use serde::{Deserialize, Serialize};
use std::{io, path::Path};

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TextOptions {
    #[serde(default)]
    language: PartialLanguageOptions,
    #[serde(default)]
    format: PartialFormatOptions,
    #[serde(default)]
    source_id: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TextResult {
    schema_version: u32,
    span_encoding: &'static str,
    output: Option<String>,
    changed: bool,
    diagnostics: Vec<Diagnostic>,
}

fn invalid(error: impl ToString) -> Error {
    Error::new(Status::InvalidArg, error.to_string())
}

fn json(value: &impl Serialize) -> Result<String> {
    serde_json::to_string(value).map_err(|error| Error::from_reason(error.to_string()))
}

fn format_text(source: &str, options: &str) -> Result<String> {
    let options: TextOptions = serde_json::from_str(options).map_err(invalid)?;
    let config = PartialConfig {
        language: Some(options.language),
        format: Some(options.format),
        ..Default::default()
    }
    .resolve()
    .map_err(invalid)?;
    let request = gdfmt_workspace::format_request(&config, SourceId(options.source_id));
    let result = gdfmt::format(source, &request);
    json(&TextResult {
        schema_version: 1,
        span_encoding: "utf8Bytes",
        output: result.output,
        changed: result.changed,
        diagnostics: result.diagnostics,
    })
}

#[napi]
pub fn format_sync(source: String, options: String) -> Result<String> {
    format_text(&source, &options)
}

enum Operation {
    Format {
        source: String,
        options: String,
    },
    Resolve {
        path: String,
        cwd: String,
        config: Option<String>,
    },
}

pub struct HostTask(Operation);

impl Task for HostTask {
    type Output = String;
    type JsValue = String;

    fn compute(&mut self) -> Result<String> {
        match &self.0 {
            Operation::Format { source, options } => format_text(source, options),
            Operation::Resolve { path, cwd, config } => resolve(path, cwd, config.as_deref()),
        }
    }

    fn resolve(&mut self, _: Env, output: String) -> Result<String> {
        Ok(output)
    }
}

#[napi]
pub fn format_async(source: String, options: String) -> AsyncTask<HostTask> {
    AsyncTask::new(HostTask(Operation::Format { source, options }))
}

fn resolve(path: &str, cwd: &str, config: Option<&str>) -> Result<String> {
    if !Path::new(cwd).is_absolute() {
        return Err(invalid("cwd must be absolute"));
    }
    let result =
        gdfmt_workspace::resolve_config(Path::new(path), Path::new(cwd), config.map(Path::new))
            .map_err(|error| Error::from_reason(error.to_string()))?;
    json(&result)
}

#[napi]
pub fn resolve_config_sync(path: String, cwd: String, config: Option<String>) -> Result<String> {
    resolve(&path, &cwd, config.as_deref())
}

#[napi]
pub fn resolve_config_async(
    path: String,
    cwd: String,
    config: Option<String>,
) -> AsyncTask<HostTask> {
    AsyncTask::new(HostTask(Operation::Resolve { path, cwd, config }))
}

/// Dedicated CLI process only; forwards OS streams directly, without buffering
/// reports in JavaScript or duplicating argument/configuration/write behavior.
#[napi]
pub fn run_cli(arguments: Vec<String>) -> i32 {
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            eprintln!("gdfmt: cannot read current directory: {error}");
            return 2;
        }
    };
    gdfmt_cli::run(
        std::iter::once("gdfmt".to_string()).chain(arguments),
        &cwd,
        &mut io::stdin().lock(),
        &mut io::stdout().lock(),
        &mut io::stderr().lock(),
    )
}

#[napi]
pub fn package_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[napi]
pub fn native_target() -> &'static str {
    env!("GDFMT_NATIVE_TARGET")
}
