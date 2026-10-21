use gdfmt_workspace::{discover_files, resolve_config};
use std::{fs, path::PathBuf};

#[test]
fn discovers_parent_layers_and_explains_false_and_zero() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(
        root.join(".gdfmtrc.json"),
        r#"{"root":true,"language":{"godotVersion":"4.3"},"format":{"lineWidth":120,"finalNewline":true}}"#,
    )
    .unwrap();
    fs::write(
        root.join("src/.gdfmtrc.json"),
        r#"{"language":{},"format":{"maxBlankLines":0,"finalNewline":false}}"#,
    )
    .unwrap();
    fs::write(root.join("src/main.gd"), "pass\n").unwrap();
    let resolved = resolve_config(&PathBuf::from("src/main.gd"), root, None).unwrap();
    assert_eq!(resolved.files.len(), 2);
    assert_eq!(resolved.config.format.line_width, 120);
    assert_eq!(resolved.config.format.max_blank_lines, 0);
    assert!(!resolved.config.format.final_newline);
    assert!(
        PathBuf::from(&resolved.sources["format.finalNewline"])
            .ends_with(PathBuf::from("src").join(".gdfmtrc.json"))
    );
    assert_eq!(resolved.sources["format.indentWidth"], "default");
    let source = "var values: Dictionary[String, int] = {}\n";
    let parsed = gdfmt_parser::parse(
        source,
        &gdfmt_parser::ParseOptions {
            language_version: resolved.config.language.godot_version,
            ..Default::default()
        },
    );
    assert!(parsed.diagnostics.iter().any(|d| d.code == "PAR017"));
    assert_eq!(parsed.tree.unwrap().text(), source);
    assert_eq!(
        resolved.config.language.godot_version,
        gdfmt_config::LanguageVersion::Godot4_3
    );
    assert!(PathBuf::from(&resolved.sources["language.godotVersion"]).ends_with(".gdfmtrc.json"));
    let explicit = resolve_config(
        &PathBuf::from("src/main.gd"),
        root,
        Some(&PathBuf::from(".gdfmtrc.json")),
    )
    .unwrap();
    assert_eq!(explicit.files.len(), 1);
    assert!(explicit.config.format.final_newline);
}

#[test]
fn directory_discovery_is_sorted_deduplicated_and_respects_ignores() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for dir in [".git", ".godot", "node_modules", "src"] {
        fs::create_dir(root.join(dir)).unwrap();
    }
    fs::write(root.join(".gitignore"), "ignored.gd\n").unwrap();
    for file in [
        "z.gd",
        "src/a.gd",
        "ignored.gd",
        ".godot/cache.gd",
        "node_modules/test.gd",
    ] {
        fs::write(root.join(file), "pass\n").unwrap();
    }
    let files = discover_files(&[".".into(), "src/a.gd".into()], root).unwrap();
    assert_eq!(files.len(), 2);
    assert!(files[0].ends_with("src/a.gd"));
    assert!(files[1].ends_with("z.gd"));
    assert!(discover_files(&["missing.gd".into()], root).is_err());
}

#[test]
fn root_config_stops_search_before_invalid_parent() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir(root.join("project")).unwrap();
    fs::write(root.join(".gdfmtrc.json"), "invalid").unwrap();
    fs::write(root.join("project/.gdfmtrc.json"), r#"{"root":true}"#).unwrap();
    let config = resolve_config(&PathBuf::from("project"), root, None).unwrap();
    assert_eq!(config.files.len(), 1);
}
#[test]
fn replacement_refuses_stale_sources_and_keeps_permissions_and_temp_cleanup() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("script.gd");
    std::fs::write(&path, "var a = 1\n").unwrap();
    assert!(gdfmt_workspace::write_source(&path, "stale", "replacement").is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "var a = 1\n");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    let permissions = std::fs::metadata(&path).unwrap().permissions();
    gdfmt_workspace::write_source(&path, "var a = 1\n", "var a = 2\n").unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "var a = 2\n");
    assert_eq!(std::fs::metadata(&path).unwrap().permissions(), permissions);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    let mut readonly = permissions.clone();
    readonly.set_readonly(true);
    std::fs::set_permissions(&path, readonly).unwrap();
    assert!(gdfmt_workspace::write_source(&path, "var a = 2\n", "changed").is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "var a = 2\n");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    std::fs::set_permissions(&path, permissions).unwrap();
}

#[cfg(unix)]
#[test]
fn replacement_refuses_symbolic_links() {
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("target.gd");
    let link = directory.path().join("link.gd");
    std::fs::write(&target, "pass\n").unwrap();
    std::os::unix::fs::symlink(&target, &link).unwrap();
    assert!(gdfmt_workspace::write_source(&link, "pass\n", "var a = 1\n").is_err());
    assert_eq!(std::fs::read_to_string(target).unwrap(), "pass\n");
    assert!(std::fs::symlink_metadata(link).unwrap().is_symlink());
}

fn write(root: &std::path::Path, path: &str, source: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, source).unwrap();
}

#[test]
fn local_extends_and_ordered_overrides_keep_their_declaring_directory() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(
        root,
        "base.json",
        r#"{"format":{"lineWidth":120,"indentWidth":6},"overrides":[{"files":["src/**/*.gd"],"format":{"lineWidth":80,"finalNewline":false}}]}"#,
    );
    write(root, "second.json", r#"{"format":{"indentWidth":3}}"#);
    write(
        root,
        ".gdfmtrc.json",
        r#"{"root":true,"extends":["base.json","second.json"],"format":{"lineWidth":100},"overrides":[{"files":["src/**/*.gd"],"exclude":["src/skip.gd"],"format":{"lineWidth":60}}]}"#,
    );
    write(
        root,
        "src/.gdfmtrc.json",
        r#"{"format":{"indentStyle":"space"},"overrides":[{"files":["special.gd"],"format":{"lineWidth":40,"maxBlankLines":0}}]}"#,
    );
    let resolved = resolve_config(std::path::Path::new("src/special.gd"), root, None).unwrap();
    assert_eq!(resolved.files.len(), 4);
    assert_eq!(resolved.config.format.line_width, 40);
    assert_eq!(resolved.config.format.indent_width, 3);
    assert_eq!(resolved.config.format.max_blank_lines, 0);
    assert!(!resolved.config.format.final_newline);
    assert!(
        resolved.sources["format.lineWidth"]
            .replace('\\', "/")
            .ends_with("src/.gdfmtrc.json#overrides[0]")
    );
    assert!(resolved.sources["format.indentWidth"].ends_with("second.json"));
    let skipped = resolve_config(std::path::Path::new("src/skip.gd"), root, None).unwrap();
    assert_eq!(skipped.config.format.line_width, 80);
    let outside = resolve_config(std::path::Path::new("other.gd"), root, None).unwrap();
    assert_eq!(outside.config.format.line_width, 100);
    let explicit = resolve_config(
        std::path::Path::new("src/special.gd"),
        root,
        Some(std::path::Path::new("base.json")),
    )
    .unwrap();
    assert_eq!(explicit.files.len(), 1);
    assert_eq!(explicit.config.format.line_width, 80);
}

#[test]
fn extends_replays_shared_parents_and_rejects_cycles_missing_files_and_limits() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(root, "base.json", r#"{"format":{"lineWidth":120}}"#);
    write(
        root,
        "left.json",
        r#"{"extends":["base.json"],"format":{"lineWidth":80}}"#,
    );
    write(root, "right.json", r#"{"extends":["base.json"]}"#);
    write(
        root,
        ".gdfmtrc.json",
        r#"{"root":true,"extends":["left.json","right.json"]}"#,
    );
    let resolved = resolve_config(root, root, None).unwrap();
    assert_eq!(resolved.files.len(), 5);
    assert_eq!(resolved.config.format.line_width, 120);
    for source in [
        r#"{"extends":[".gdfmtrc.json"]}"#,
        r#"{"extends":["missing.json"]}"#,
        r#"{"extends":["https://example.test/a.json"]}"#,
    ] {
        write(root, ".gdfmtrc.json", source);
        assert!(resolve_config(root, root, None).is_err());
    }
    for index in 0..33 {
        write(
            root,
            &format!("{index}.json"),
            &format!(r#"{{"extends":["{}.json"]}}"#, index + 1),
        );
    }
    assert!(
        resolve_config(root, root, Some(std::path::Path::new("0.json")))
            .unwrap_err()
            .to_string()
            .contains("limit")
    );
    write(
        root,
        "repeat.json",
        &format!(
            r#"{{"extends":[{}]}}"#,
            vec!["\"base.json\""; 129].join(",")
        ),
    );
    assert!(
        resolve_config(root, root, Some(std::path::Path::new("repeat.json")))
            .unwrap_err()
            .to_string()
            .contains("limit")
    );
}

#[test]
fn all_override_patterns_are_validated_even_when_they_do_not_match() {
    let temp = tempfile::tempdir().unwrap();
    for glob in [
        "[",
        "../*.gd",
        "/absolute.gd",
        "C:/absolute.gd",
        "",
        r"src\*.gd",
    ] {
        let source = format!(
            r#"{{"root":true,"overrides":[{{"files":[{}]}}]}}"#,
            serde_json::to_string(glob).unwrap()
        );
        write(temp.path(), ".gdfmtrc.json", &source);
        assert!(
            resolve_config(temp.path(), temp.path(), None).is_err(),
            "{glob}"
        );
    }
}

#[test]
fn editorconfig_sections_unset_fallbacks_and_json_precedence_are_explained() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(
        root,
        ".editorconfig",
        "root = true\n[*.{gd,txt}]\nindent_style = SPACE\nindent_size = 2\nend_of_line = crlf\ninsert_final_newline = false\nmax_line_length = 80\ncharset = latin1\ntrim_trailing_whitespace = false\n",
    );
    write(
        root,
        "src/.editorconfig",
        "[case{1..3}.gd]\nindent_size = unset\ntab_width = 6\nmax_line_length = off\n",
    );
    write(
        root,
        ".gdfmtrc.json",
        r#"{"root":true,"format":{"lineWidth":120},"overrides":[{"files":["src/case2.gd"],"format":{"maxBlankLines":0}}]}"#,
    );
    let resolved = resolve_config(std::path::Path::new("src/case2.gd"), root, None).unwrap();
    assert_eq!(resolved.editorconfig_files.len(), 2);
    assert_eq!(
        resolved.config.format.indent_style,
        gdfmt_config::IndentStyle::Space
    );
    assert_eq!(resolved.config.format.indent_width, 6);
    assert_eq!(resolved.config.format.line_width, 120);
    assert_eq!(
        resolved.config.format.line_ending,
        gdfmt_config::LineEnding::Crlf
    );
    assert!(!resolved.config.format.final_newline);
    assert!(
        resolved.sources["format.indentWidth"]
            .replace('\\', "/")
            .ends_with("src/.editorconfig:3 (tab_width)")
    );
    assert!(resolved.sources["format.lineWidth"].ends_with(".gdfmtrc.json"));
    assert_eq!(resolved.sources.len(), 8);
    write(
        root,
        "src/.editorconfig",
        "root = true\n[*.gd]\nindent_style = tab\nindent_size = tab\ntab_width = 8\n",
    );
    let resolved = resolve_config(std::path::Path::new("src/new/missing.gd"), root, None).unwrap();
    assert_eq!(resolved.editorconfig_files.len(), 1);
    assert_eq!(resolved.config.format.indent_width, 8);
    assert_eq!(
        resolved.config.format.line_ending,
        gdfmt_config::LineEnding::Lf
    );
    assert!(resolved.config.format.final_newline);
}

#[test]
fn unsupported_editorconfig_values_do_not_claim_to_be_effective() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        ".editorconfig",
        "root=true\n[*]\nindent_size=20\nindent_style=unknown\nend_of_line=cr\ninsert_final_newline=maybe\nmax_line_length=off\ncharset=utf-16le\n",
    );
    let resolved = resolve_config(std::path::Path::new("x.gd"), temp.path(), None).unwrap();
    assert!(resolved.sources.values().all(|value| value == "default"));
    write(
        temp.path(),
        ".editorconfig",
        "root=true\n[*.gd]\nthis is malformed\n",
    );
    assert!(resolve_config(std::path::Path::new("x.gd"), temp.path(), None).is_err());
}

#[test]
fn virtual_paths_normalize_missing_suffixes_without_losing_filename_overrides() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        ".gdfmtrc.json",
        r#"{"root":true,"overrides":[{"files":["new/case.gd"],"format":{"lineWidth":40}}]}"#,
    );
    let resolved = resolve_config(
        std::path::Path::new("new/missing/../case.gd"),
        temp.path(),
        None,
    )
    .unwrap();
    assert_eq!(resolved.config.format.line_width, 40);
}

#[test]
fn ignore_files_apply_to_explicit_paths_with_ordered_negation_and_parent_pruning() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(root, ".gdfmtrc.json", r#"{"root":true}"#);
    write(root, ".gitignore", "*.gd\n");
    write(
        root,
        ".gdfmtignore",
        "!keep.gd\n!sub/\nsub/*\n!sub/keep.gd\nignored/\n.godot/\n!.godot/keep.gd\n",
    );
    write(root, "sub/.gdfmtignore", "!nested.gd\n");
    write(root, "ignored/.gdfmtignore", "!keep.gd\n");
    for file in [
        "keep.gd",
        "drop.gd",
        "sub/keep.gd",
        "sub/nested.gd",
        "ignored/keep.gd",
        ".godot/keep.gd",
    ] {
        write(root, file, "pass\n");
    }
    let files = discover_files(&[".".into(), "keep.gd".into()], root).unwrap();
    assert_eq!(files.len(), 3);
    assert!(discover_files(&["drop.gd".into()], root).is_err());
    assert!(discover_files(&["ignored/keep.gd".into()], root).is_err());
    assert!(discover_files(&[".godot/keep.gd".into()], root).is_err());
    assert!(
        resolve_config(std::path::Path::new("drop.gd"), root, None)
            .unwrap()
            .ignored
    );
}

#[test]
fn json_ignore_replaces_inherited_arrays_and_retains_its_origin() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(root, "base.json", r#"{"ignore":["skip/","root.gd"]}"#);
    write(
        root,
        ".gdfmtrc.json",
        r#"{"root":true,"extends":["base.json"]}"#,
    );
    write(root, "root.gd", "pass\n");
    write(root, "skip/bad.gd", "invalid");
    write(root, "src/.gdfmtrc.json", r#"{"ignore":[]}"#);
    write(root, "src/root.gd", "pass\n");
    let files = discover_files(&[".".into()], root).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("src/root.gd"));
    write(root, "src/.gdfmtrc.json", r#"{"ignore":["root.gd"]}"#);
    let resolution = resolve_config(std::path::Path::new("src/root.gd"), root, None).unwrap();
    assert!(resolution.ignored);
    assert!(
        resolution.sources["ignore"]
            .replace('\\', "/")
            .ends_with("src/.gdfmtrc.json")
    );
    let explicit = gdfmt_workspace::discover_files_with_config(
        &["src/root.gd".into()],
        root,
        Some(std::path::Path::new("base.json")),
    );
    assert!(explicit.is_err());
}

#[test]
fn configuration_and_ignore_files_have_bounded_reads_and_invalid_utf8_errors() {
    let temp = tempfile::tempdir().unwrap();
    for name in [".gdfmtrc.json", ".editorconfig", ".gdfmtignore"] {
        for bytes in [vec![0xff], vec![b' '; 1024 * 1024 + 1]] {
            fs::write(temp.path().join(name), bytes).unwrap();
            assert!(
                resolve_config(std::path::Path::new("x.gd"), temp.path(), None).is_err(),
                "{name}"
            );
            fs::remove_file(temp.path().join(name)).unwrap();
        }
    }
}

#[cfg(unix)]
#[test]
fn discovery_skips_symlink_loops_and_replacement_refuses_hardlinks() {
    let temp = tempfile::tempdir().unwrap();
    write(temp.path(), "a.gd", "pass\n");
    std::os::unix::fs::symlink(temp.path(), temp.path().join("loop")).unwrap();
    std::os::unix::fs::symlink(temp.path().join("a.gd"), temp.path().join("link.gd")).unwrap();
    assert_eq!(discover_files(&[".".into()], temp.path()).unwrap().len(), 1);
    assert!(discover_files(&["link.gd".into()], temp.path()).is_err());
    fs::hard_link(temp.path().join("a.gd"), temp.path().join("hard.gd")).unwrap();
    assert!(gdfmt_workspace::write_source(&temp.path().join("a.gd"), "pass\n", "changed").is_err());
    assert_eq!(
        fs::read_to_string(temp.path().join("hard.gd")).unwrap(),
        "pass\n"
    );
}

#[test]
fn total_json_budget_is_bounded_independently_of_depth_and_layer_count() {
    let temp = tempfile::tempdir().unwrap();
    let padded = format!("{{}}{}", " ".repeat(1024 * 1024 - 2));
    write(temp.path(), "large.json", &padded);
    let source = format!(
        r#"{{"root":true,"extends":[{}]}}"#,
        vec!["\"large.json\""; 17].join(",")
    );
    write(temp.path(), ".gdfmtrc.json", &source);
    assert!(
        resolve_config(temp.path(), temp.path(), None)
            .unwrap_err()
            .to_string()
            .contains("total configuration byte limit")
    );
}

#[test]
fn ignore_priority_and_git_worktree_marker_files_do_not_break_discovery() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(root, ".git/info/exclude", "*.gd\n");
    write(root, ".gitignore", "!keep.gd\n");
    write(root, ".ignore", "keep.gd\n");
    write(root, ".gdfmtignore", "!keep.gd\n");
    write(root, "keep.gd", "pass\n");
    write(root, "drop.gd", "pass\n");
    assert_eq!(discover_files(&[".".into()], root).unwrap().len(), 1);
    fs::remove_file(root.join(".gdfmtignore")).unwrap();
    assert!(discover_files(&[".".into()], root).is_err());
    fs::remove_dir_all(root.join(".git")).unwrap();
    write(root, ".git", "gitdir: ../main/.git/worktrees/test\n");
    fs::remove_file(root.join(".ignore")).unwrap();
    assert_eq!(discover_files(&[".".into()], root).unwrap().len(), 2);
}

#[cfg(unix)]
#[test]
fn config_cycle_detection_uses_the_real_file_identity_path() {
    let temp = tempfile::tempdir().unwrap();
    write(
        temp.path(),
        ".gdfmtrc.json",
        r#"{"root":true,"extends":["alias.json"]}"#,
    );
    std::os::unix::fs::symlink(
        temp.path().join(".gdfmtrc.json"),
        temp.path().join("alias.json"),
    )
    .unwrap();
    assert!(
        resolve_config(temp.path(), temp.path(), None)
            .unwrap_err()
            .to_string()
            .contains("cycle")
    );
}
