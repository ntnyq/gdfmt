use crate::{
    WorkspaceError, absolute,
    configuration::{configuration_error, input_path, optional_text},
    io_error, resolve_config,
};
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

fn builtin(path: &Path) -> bool {
    path.components().any(|part| {
        matches!(
            part.as_os_str().to_str(),
            Some(".git" | ".godot" | "node_modules" | "target")
        )
    })
}

fn ignore_rules(
    path: &Path,
    patterns: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<Gitignore, WorkspaceError> {
    let mut builder = GitignoreBuilder::new(path.parent().expect("ignore file has parent"));
    for (index, pattern) in patterns.into_iter().enumerate() {
        builder
            .add_line(Some(path.into()), pattern.as_ref())
            .map_err(|error| {
                configuration_error(path, format!("ignore line {}: {error}", index + 1))
            })?;
    }
    builder
        .build()
        .map_err(|error| configuration_error(path, error))
}

pub(super) fn matches_ignore(
    config: &Path,
    patterns: &[String],
    path: &Path,
    is_dir: bool,
) -> Result<bool, WorkspaceError> {
    let matcher = ignore_rules(config, patterns)?;
    Ok(
        path.starts_with(config.parent().expect("config has parent"))
            && matcher
                .matched_path_or_any_parents(path, is_dir)
                .is_ignore(),
    )
}

fn excluded(matchers: &[Gitignore], path: &Path, is_dir: bool) -> bool {
    matchers
        .iter()
        .rev()
        .map(|rules| rules.matched(path, is_dir))
        .find(|result| !result.is_none())
        .is_some_and(|result| result.is_ignore())
}

/// Local ignore files apply to explicit files too. Ignored directories cannot be
/// resurrected by a negation below them; global Git excludes are never consulted.
pub(super) fn is_ignored(path: &Path) -> Result<bool, WorkspaceError> {
    if builtin(path) {
        return Ok(true);
    }
    let directory = if path.is_dir() {
        path
    } else {
        path.parent().expect("input has parent")
    };
    let mut ancestors: Vec<_> = directory.ancestors().collect();
    ancestors.reverse();
    let mut matchers = Vec::new();
    for ancestor in ancestors {
        if excluded(&matchers, ancestor, true) {
            return Ok(true);
        }
        for name in [".git/info/exclude", ".gitignore", ".ignore", ".gdfmtignore"] {
            if name == ".git/info/exclude" && !ancestor.join(".git").is_dir() {
                continue;
            }
            let file = ancestor.join(name);
            if let Some(source) = optional_text(&file)? {
                // Git's info/exclude patterns are relative to the repository root.
                let base = if name == ".git/info/exclude" {
                    ancestor.join(".gitignore")
                } else {
                    file.clone()
                };
                let rules = ignore_rules(&base, source.lines())
                    .map_err(|error| configuration_error(&file, error))?;
                matchers.push(rules);
            }
        }
    }
    Ok(excluded(&matchers, path, path.is_dir()))
}

/// Stable deduplicated discovery, respecting local ignore files and JSON ignore.
/// Symlinks are skipped during traversal and rejected as explicit inputs.
pub fn discover_files(paths: &[PathBuf], cwd: &Path) -> Result<Vec<PathBuf>, WorkspaceError> {
    discover_files_with_config(paths, cwd, None)
}

pub fn discover_files_with_config(
    paths: &[PathBuf],
    cwd: &Path,
    explicit: Option<&Path>,
) -> Result<Vec<PathBuf>, WorkspaceError> {
    let defaults = [PathBuf::from(".")];
    let paths = if paths.is_empty() {
        &defaults[..]
    } else {
        paths
    };
    let mut candidates = BTreeSet::new();
    for input in paths {
        let path = absolute(input, cwd);
        let metadata = fs::symlink_metadata(&path).map_err(|error| io_error(&path, error))?;
        if metadata.is_file() {
            if path.extension().is_none_or(|ext| ext != "gd") {
                return Err(WorkspaceError::Input(format!(
                    "{}: expected a .gd file",
                    path.display()
                )));
            }
            let path = input_path(&path, cwd)?;
            if !is_ignored(&path)? {
                candidates.insert(path);
            }
        } else if metadata.is_dir() {
            let mut pending = vec![input_path(&path, cwd)?];
            while let Some(directory) = pending.pop() {
                if is_ignored(&directory)? {
                    continue;
                }
                for entry in
                    fs::read_dir(&directory).map_err(|error| io_error(&directory, error))?
                {
                    let entry = entry.map_err(|error| io_error(&directory, error))?;
                    let kind = entry
                        .file_type()
                        .map_err(|error| io_error(&entry.path(), error))?;
                    if kind.is_dir() {
                        pending.push(entry.path());
                    } else if kind.is_file()
                        && entry.path().extension().is_some_and(|ext| ext == "gd")
                        && !is_ignored(&entry.path())?
                    {
                        candidates.insert(entry.path());
                    }
                }
            }
        } else {
            return Err(WorkspaceError::Input(format!(
                "{}: expected a regular file or directory",
                path.display()
            )));
        }
    }
    let mut files = Vec::new();
    for path in candidates {
        if !resolve_config(&path, cwd, explicit)?.ignored {
            files.push(path);
        }
    }
    if files.is_empty() {
        return Err(WorkspaceError::Input(
            "no .gd files found (all inputs may be ignored)".into(),
        ));
    }
    Ok(files)
}
