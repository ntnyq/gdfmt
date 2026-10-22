//! Node boundary only: owned UTF-8 text/JSON crosses into libuv workers.
//! Syntax, configuration, validated formatting and CLI behavior stay in Rust.
use napi::{Env, Error, Result, Task, bindgen_prelude::AsyncTask};
use napi_derive::napi;
use serde::Serialize;
use std::{io, path::Path};

fn invalid(error: impl ToString) -> Error {
    Error::new(napi::Status::InvalidArg, error.to_string())
}

fn json(value: &impl Serialize) -> Result<String> {
    serde_json::to_string(value).map_err(|error| Error::from_reason(error.to_string()))
}

fn format_text(source: &str, options: &str) -> Result<String> {
    gdfmt_config::text::format_text_json(source, options).map_err(invalid)
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
        &mut io::stdin(),
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
