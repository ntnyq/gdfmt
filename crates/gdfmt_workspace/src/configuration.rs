use crate::{
    MAX_CONFIG_BYTES, WorkspaceError, absolute, discovery::is_ignored, io_error, read_utf8,
};
use ec4rs::PropertiesSource;
use gdfmt_config::{PartialConfig, PartialFormatOptions, ResolvedConfig, parse_config};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

pub const CONFIG_FILENAME: &str = ".gdfmtrc.json";
/// Configuration discovery order within each directory; the first existing file wins.
pub const CONFIG_FILENAMES: [&str; 2] = [CONFIG_FILENAME, ".gdfmtrc.jsonc"];
const MAX_EXTENDS_DEPTH: usize = 32;
const MAX_CONFIG_LAYERS: usize = 128;
const MAX_TOTAL_CONFIG_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigResolution {
    pub config: ResolvedConfig,
    /// JSON/JSONC files in application order, including inherited files (replays are retained).
    pub files: Vec<PathBuf>,
    pub editorconfig_files: Vec<PathBuf>,
    pub sources: BTreeMap<String, String>,
    pub ignored: bool,
}

pub(super) fn configuration_error(path: &Path, message: impl ToString) -> WorkspaceError {
    WorkspaceError::Configuration {
        path: path.into(),
        message: message.to_string(),
    }
}

/// Canonicalize the existing prefix while retaining a virtual file's missing suffix.
/// Lexical normalization makes the same virtual path work on every host platform.
pub(super) fn input_path(path: &Path, cwd: &Path) -> Result<PathBuf, WorkspaceError> {
    let mut normalized = PathBuf::new();
    for part in absolute(path, cwd).components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(part.as_os_str()),
        }
    }
    let mut existing = normalized.as_path();
    let mut suffix = Vec::new();
    loop {
        match fs::canonicalize(existing) {
            Ok(mut path) => {
                for part in suffix.into_iter().rev() {
                    path.push(part);
                }
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                suffix.push(
                    existing
                        .file_name()
                        .ok_or_else(|| io_error(existing, error))?
                        .to_owned(),
                );
                existing = existing
                    .parent()
                    .expect("a missing named path has a parent");
            }
            Err(error) => return Err(io_error(existing, error)),
        }
    }
}

#[derive(Default)]
struct ConfigLoader {
    stack: Vec<PathBuf>,
    layers: Vec<(PathBuf, PartialConfig)>,
    bytes: usize,
}

impl ConfigLoader {
    fn load(&mut self, path: &Path) -> Result<(), WorkspaceError> {
        let path = fs::canonicalize(path).map_err(|error| io_error(path, error))?;
        if self.stack.contains(&path) {
            return Err(configuration_error(&path, "extends cycle detected"));
        }
        if self.stack.len() >= MAX_EXTENDS_DEPTH || self.layers.len() >= MAX_CONFIG_LAYERS {
            return Err(configuration_error(
                &path,
                "configuration inheritance limit exceeded",
            ));
        }
        let source = read_utf8(&path, MAX_CONFIG_BYTES)?;
        self.bytes += source.len();
        if self.bytes > MAX_TOTAL_CONFIG_BYTES {
            return Err(configuration_error(
                &path,
                "total configuration byte limit exceeded",
            ));
        }
        let config = parse_config(&source).map_err(|error| configuration_error(&path, error))?;
        self.stack.push(path.clone());
        if let Some(parents) = &config.extends {
            for parent in parents {
                if parent.is_empty() || parent.contains("://") {
                    return Err(configuration_error(
                        &path,
                        "extends requires local JSON/JSONC file paths",
                    ));
                }
                self.load(&absolute(
                    Path::new(parent),
                    path.parent().expect("config has a parent"),
                ))?;
            }
        }
        self.stack.pop();
        if self.layers.len() >= MAX_CONFIG_LAYERS {
            return Err(configuration_error(
                &path,
                "configuration inheritance limit exceeded",
            ));
        }
        self.layers.push((path, config));
        Ok(())
    }
}

pub fn resolve_config(
    path: &Path,
    cwd: &Path,
    explicit: Option<&Path>,
) -> Result<ConfigResolution, WorkspaceError> {
    let target = input_path(path, cwd)?;
    let directory = if target.is_dir() {
        target.as_path()
    } else {
        target.parent().expect("input has a parent")
    };
    let mut discovered = Vec::new();
    if let Some(config) = explicit {
        discovered.push(absolute(config, cwd));
    } else {
        'ancestors: for ancestor in directory.ancestors() {
            for filename in CONFIG_FILENAMES {
                let path = ancestor.join(filename);
                if let Some(source) = optional_text(&path)? {
                    let config =
                        parse_config(&source).map_err(|error| configuration_error(&path, error))?;
                    let stop = config.root == Some(true);
                    discovered.push(path);
                    if stop {
                        break 'ancestors;
                    }
                    break;
                }
            }
        }
        discovered.reverse();
    }
    let mut loader = ConfigLoader::default();
    for path in discovered {
        loader.load(&path)?;
    }
    let mut sources: BTreeMap<_, _> = [
        "schemaVersion",
        "language.godotVersion",
        "format.lineWidth",
        "format.indentStyle",
        "format.indentWidth",
        "format.lineEnding",
        "format.maxBlankLines",
        "format.finalNewline",
    ]
    .into_iter()
    .map(|key| (key.to_owned(), "default".to_owned()))
    .collect();
    let (mut merged, editorconfig_files) = editorconfig(&target, directory, &mut sources)?;
    let mut ignore = None;
    for (path, config) in &loader.layers {
        apply_layer(
            &mut merged,
            &mut sources,
            config,
            &path.display().to_string(),
        );
        if let Some(patterns) = &config.ignore {
            super::discovery::matches_ignore(path, patterns, &target, target.is_dir())?;
            ignore = Some((path, patterns));
            sources.insert("ignore".into(), path.display().to_string());
        }
    }
    // All base fields first, then appended overrides in declaration/application order.
    for (path, config) in &loader.layers {
        if let Some(overrides) = &config.overrides {
            for (index, entry) in overrides.iter().enumerate() {
                let include = matches_globs(path, &entry.files, &target)?;
                let exclude = matches_globs(path, &entry.exclude, &target)?;
                if include && !exclude {
                    let layer = PartialConfig {
                        language: entry.language.clone(),
                        format: entry.format.clone(),
                        ..Default::default()
                    };
                    apply_layer(
                        &mut merged,
                        &mut sources,
                        &layer,
                        &format!("{}#overrides[{index}]", path.display()),
                    );
                }
            }
        }
    }
    let config_ignored = match ignore {
        Some((path, patterns)) => {
            super::discovery::matches_ignore(path, patterns, &target, target.is_dir())?
        }
        None => false,
    };
    let ignored = is_ignored(&target)? || config_ignored;
    let config = merged
        .resolve()
        .map_err(|error| configuration_error(&target, error))?;
    Ok(ConfigResolution {
        config,
        files: loader.layers.into_iter().map(|(path, _)| path).collect(),
        editorconfig_files,
        sources,
        ignored,
    })
}

pub(super) fn optional_text(path: &Path) -> Result<Option<String>, WorkspaceError> {
    match fs::metadata(path) {
        Ok(_) => read_utf8(path, MAX_CONFIG_BYTES).map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(path, error)),
    }
}

fn apply_layer(
    merged: &mut PartialConfig,
    sources: &mut BTreeMap<String, String>,
    config: &PartialConfig,
    origin: &str,
) {
    if config.schema_version.is_some() {
        sources.insert("schemaVersion".into(), origin.into());
    }
    if config
        .language
        .as_ref()
        .is_some_and(|v| v.godot_version.is_some())
    {
        sources.insert("language.godotVersion".into(), origin.into());
    }
    if let Some(format) = &config.format {
        for (field, present) in [
            ("lineWidth", format.line_width.is_some()),
            ("indentStyle", format.indent_style.is_some()),
            ("indentWidth", format.indent_width.is_some()),
            ("lineEnding", format.line_ending.is_some()),
            ("maxBlankLines", format.max_blank_lines.is_some()),
            ("finalNewline", format.final_newline.is_some()),
        ] {
            if present {
                sources.insert(format!("format.{field}"), origin.into());
            }
        }
    }
    // Host arrays are resolved separately, retaining their declaring directories.
    let layer = PartialConfig {
        schema_version: config.schema_version,
        language: config.language.clone(),
        format: config.format.clone(),
        ..Default::default()
    };
    merged.merge(&layer);
}

fn matches_globs(
    config: &Path,
    patterns: &[String],
    target: &Path,
) -> Result<bool, WorkspaceError> {
    let relative = target
        .strip_prefix(config.parent().expect("config has parent"))
        .ok();
    let mut matched = false;
    for pattern in patterns {
        if pattern.is_empty()
            || pattern.starts_with('/')
            || pattern.contains('\\')
            || pattern.contains(':')
            || pattern.split('/').any(|part| part == "..")
        {
            return Err(configuration_error(
                config,
                "override globs must be nonempty relative paths using /",
            ));
        }
        let glob = globset::GlobBuilder::new(pattern)
            .literal_separator(true)
            .backslash_escape(false)
            .build()
            .map_err(|error| configuration_error(config, error))?;
        matched |= relative.is_some_and(|path| glob.compile_matcher().is_match(path));
    }
    Ok(matched)
}

fn editorconfig(
    target: &Path,
    directory: &Path,
    sources: &mut BTreeMap<String, String>,
) -> Result<(PartialConfig, Vec<PathBuf>), WorkspaceError> {
    let mut layers = Vec::new();
    for ancestor in directory.ancestors() {
        let path = ancestor.join(".editorconfig");
        if let Some(source) = optional_text(&path)? {
            let parser = ec4rs::ConfigParser::new_with_path(
                std::io::Cursor::new(source),
                Some(path.clone()),
            )
            .map_err(|error| configuration_error(&path, error))?;
            let stop = parser.is_root;
            let sections = parser
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| configuration_error(&path, error))?;
            layers.push((path, sections));
            if stop {
                break;
            }
        }
    }
    layers.reverse();
    let mut properties = ec4rs::Properties::new();
    for (path, sections) in &layers {
        let relative = target
            .strip_prefix(path.parent().expect("editorconfig has parent"))
            .expect("ancestor");
        for section in sections {
            section
                .apply_to(&mut properties, relative)
                .map_err(|error| configuration_error(path, error))?;
        }
    }
    let raw = |key: &str| properties.get_raw_for_key(key).filter_unset();
    let value = |key: &str| raw(key).into_option().map(str::to_ascii_lowercase);
    let mut format = PartialFormatOptions::default();
    let mut record = |field: &str, key: &str| {
        if let Some((path, line)) = raw(key).source() {
            sources.insert(
                format!("format.{field}"),
                format!("{}:{line} ({key})", path.display()),
            );
        }
    };
    format.indent_style = match value("indent_style").as_deref() {
        Some("tab") => Some(gdfmt_config::IndentStyle::Tab),
        Some("space") => Some(gdfmt_config::IndentStyle::Space),
        _ => None,
    };
    if format.indent_style.is_some() {
        record("indentStyle", "indent_style");
    }
    let size = value("indent_size");
    let use_tab = format.indent_style == Some(gdfmt_config::IndentStyle::Tab)
        || size.as_deref() == Some("tab");
    let keys = if use_tab {
        ["tab_width", "indent_size"]
    } else {
        ["indent_size", "tab_width"]
    };
    for key in keys {
        if let Some(width) = value(key)
            .and_then(|v| v.parse::<u8>().ok())
            .filter(|v| (1..=8).contains(v))
        {
            format.indent_width = Some(width);
            record("indentWidth", key);
            break;
        }
    }
    format.line_ending = match value("end_of_line").as_deref() {
        Some("lf") => Some(gdfmt_config::LineEnding::Lf),
        Some("crlf") => Some(gdfmt_config::LineEnding::Crlf),
        _ => None,
    };
    if format.line_ending.is_some() {
        record("lineEnding", "end_of_line");
    }
    format.final_newline = value("insert_final_newline").and_then(|v| v.parse::<bool>().ok());
    if format.final_newline.is_some() {
        record("finalNewline", "insert_final_newline");
    }
    format.line_width = value("max_line_length")
        .and_then(|v| v.parse::<u16>().ok())
        .filter(|v| (40..=320).contains(v));
    if format.line_width.is_some() {
        record("lineWidth", "max_line_length");
    }
    Ok((
        PartialConfig {
            format: Some(format),
            ..Default::default()
        },
        layers.into_iter().map(|(path, _)| path).collect(),
    ))
}
