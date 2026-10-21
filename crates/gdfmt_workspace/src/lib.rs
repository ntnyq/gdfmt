//! Filesystem/configuration adapters and guarded per-file replacement.

use gdfmt_config::ResolvedConfig;
use gdfmt_parser::{LexOptions, LexResult, lex};
use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};
use thiserror::Error;

mod configuration;
mod discovery;
mod writing;

pub use configuration::{CONFIG_FILENAME, ConfigResolution, resolve_config};
pub use discovery::{discover_files, discover_files_with_config};
pub use writing::write_source;
const MAX_CONFIG_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("{path}: {message}")]
    Configuration { path: PathBuf, message: String },
    #[error("{0}")]
    Input(String),
}

fn io_error(path: &Path, source: io::Error) -> WorkspaceError {
    WorkspaceError::Io {
        path: path.into(),
        source,
    }
}

fn absolute(path: &Path, cwd: &Path) -> PathBuf {
    if path.is_absolute() {
        path.into()
    } else {
        cwd.join(path)
    }
}

fn read_utf8(path: &Path, limit: u64) -> Result<String, WorkspaceError> {
    let file = File::open(path).map_err(|error| io_error(path, error))?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| io_error(path, error))?;
    if bytes.len() as u64 > limit {
        return Err(WorkspaceError::Input(format!(
            "{}: exceeds byte limit {limit}",
            path.display()
        )));
    }
    String::from_utf8(bytes).map_err(|error| {
        WorkspaceError::Input(format!("{}: invalid UTF-8: {error}", path.display()))
    })
}

#[derive(Debug)]
pub struct LexedFile {
    pub path: PathBuf,
    pub source: String,
    pub result: LexResult,
}

pub fn lex_file(path: &Path, options: &LexOptions) -> Result<LexedFile, WorkspaceError> {
    let source = read_utf8(path, options.max_source_bytes.min(u32::MAX as usize) as u64)?;
    let result = lex(&source, options);
    Ok(LexedFile {
        path: path.into(),
        source,
        result,
    })
}

/// Map resolved host configuration into the pure text formatter.
pub fn format_request(
    config: &ResolvedConfig,
    source_id: gdfmt_text::SourceId,
) -> gdfmt::FormatRequest {
    gdfmt::FormatRequest {
        parse: gdfmt::ParseOptions {
            language_version: config.language.godot_version,
            lex: LexOptions {
                source_id,
                ..Default::default()
            },
            ..Default::default()
        },
        format: gdfmt::FormatOptions {
            line_width: config.format.line_width as usize,
            indent: match config.format.indent_style {
                gdfmt_config::IndentStyle::Tab => "\t".into(),
                gdfmt_config::IndentStyle::Space => " ".repeat(config.format.indent_width as usize),
            },
            tab_width: config.format.indent_width as usize,
            line_ending: match config.format.line_ending {
                gdfmt_config::LineEnding::Lf => gdfmt::LineEnding::Lf,
                gdfmt_config::LineEnding::Crlf => gdfmt::LineEnding::Crlf,
                gdfmt_config::LineEnding::Preserve => gdfmt::LineEnding::Preserve,
            },
            max_blank_lines: config.format.max_blank_lines as usize,
            final_newline: config.format.final_newline,
        },
    }
}

pub fn read_source(path: &Path) -> Result<String, WorkspaceError> {
    read_utf8(path, LexOptions::default().max_source_bytes as u64)
}
