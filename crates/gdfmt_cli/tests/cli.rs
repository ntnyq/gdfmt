use std::{fs, path::Path, process::Command};

fn run(args: &[&str], cwd: &Path, mut input: &[u8]) -> (i32, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = gdfmt_cli::run(
        std::iter::once("gdfmt").chain(args.iter().copied()),
        cwd,
        &mut input,
        &mut stdout,
        &mut stderr,
    );
    (
        code,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

#[test]
fn protected_regions_share_file_check_diff_and_stdin_behavior() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let source = "# gdfmt: formatter-off\r\nvar a=[ 1,2 ]\r\n# gdfmt: formatter-on\r\nvar b=2\r\n";
    let protected = source.split("var b").next().unwrap();
    fs::write(root.join("test.gd"), source).unwrap();
    for args in [vec!["check", "test.gd"], vec!["--diff", "test.gd"]] {
        assert_eq!(run(&args, root, b"").0, 1);
        assert_eq!(fs::read_to_string(root.join("test.gd")).unwrap(), source);
    }
    let (code, stdout, stderr) = run(&["format", "-"], root, source.as_bytes());
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.starts_with(protected));
    assert!(stdout.ends_with("var b = 2\n"));
    assert_eq!(run(&["format", "test.gd"], root, b"").0, 0);
    assert_eq!(fs::read_to_string(root.join("test.gd")).unwrap(), stdout);
    assert_eq!(run(&["check", "test.gd"], root, b"").0, 0);
}

#[test]
fn malformed_regions_and_hidden_syntax_errors_never_write() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    for source in [
        "# gdfmt: formatter-off\nvar a=1\n",
        "# gdfmt: formatter-off\nvar =\n# gdfmt: formatter-on\n",
    ] {
        fs::write(root.join("test.gd"), source).unwrap();
        let (code, stdout, _) = run(&["format", "test.gd", "--reporter=json"], root, b"");
        assert_eq!(code, 1);
        assert_eq!(fs::read_to_string(root.join("test.gd")).unwrap(), source);
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(report["files"][0]["changed"], false);
        assert!(
            !report["files"][0]["diagnostics"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let (code, stdout, _) = run(&["format", "-"], root, source.as_bytes());
        assert_eq!(code, 1);
        assert!(stdout.is_empty());
    }
    // Syntax-only does not interpret formatter instructions.
    assert_eq!(
        run(
            &["check", "-", "--syntax-only"],
            root,
            b"# gdfmt: formatter-off\nvar a=1\n"
        )
        .0,
        0
    );
}

#[test]
fn formatter_does_not_expose_lint_or_semantic_fix_workflows() {
    let temp = tempfile::tempdir().unwrap();
    for args in [
        vec!["check", "--fix"],
        vec!["check", "--deny-warnings"],
        vec!["lint"],
    ] {
        assert_eq!(run(&args, temp.path(), b"").0, 2);
    }
    let source = "func BadName():\n\tpass\n\treturn\n\tprint(1)\n";
    let (code, stdout, stderr) = run(&["format", "-"], temp.path(), source.as_bytes());
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, source);
}

#[test]
fn help_and_argument_errors_ignore_launcher_filename() {
    let temp = tempfile::tempdir().unwrap();
    for args in [
        vec!["--help"],
        vec!["--version"],
        vec!["check", "--help"],
        vec!["--unknown"],
    ] {
        let expected = run(&args, temp.path(), b"");
        for launcher in ["gdfmt.exe", "renamed-gdfmt", "/tmp/gdfmt.exe"] {
            let mut stdout = Vec::new();
            let mut stderr = Vec::new();
            let code = gdfmt_cli::run(
                std::iter::once(launcher).chain(args.iter().copied()),
                temp.path(),
                &mut &b""[..],
                &mut stdout,
                &mut stderr,
            );
            assert_eq!(
                (
                    code,
                    String::from_utf8(stdout).unwrap(),
                    String::from_utf8(stderr).unwrap(),
                ),
                expected,
                "{launcher} {args:?}"
            );
        }
    }
}

#[test]
fn help_version_and_argument_errors_have_stable_exit_codes() {
    let temp = tempfile::tempdir().unwrap();
    for arg in ["--help", "--version"] {
        let (code, stdout, stderr) = run(&[arg], temp.path(), b"");
        assert_eq!(code, 0);
        assert!(stdout.contains("gdfmt"));
        assert!(stderr.is_empty());
    }
    let (code, stdout, stderr) = run(&["--reporter=json", "--unknown"], temp.path(), b"");
    assert_eq!(code, 2);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["diagnostics"][0]["code"], "CLI_ARGUMENT");
    assert!(stderr.is_empty());
    assert_eq!(run(&["--unknown"], temp.path(), b"").0, 2);
}

#[test]
fn lex_stdin_reports_byte_spans_and_lexical_errors() {
    let temp = tempfile::tempdir().unwrap();
    let source = "var 文本 = \"😀\"\r\n";
    let (code, stdout, stderr) = run(
        &["lex", "-", "--reporter", "json"],
        temp.path(),
        source.as_bytes(),
    );
    assert_eq!(code, 0, "{stdout}\n{stderr}");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["spanEncoding"], "utf8Bytes");
    let reconstructed: String = report["files"][0]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|token| token["text"].as_str().unwrap())
        .collect();
    assert_eq!(reconstructed, source);
    let (code, stdout, _) = run(
        &["lex", "-", "--reporter=json"],
        temp.path(),
        b"var x = 'broken\n",
    );
    assert_eq!(code, 1);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["files"][0]["diagnostics"][0]["code"], "LEX006");
    assert_eq!(run(&["lex", "-"], temp.path(), &[0xff]).0, 2);
    assert_eq!(run(&["lex", "-", "file.gd"], temp.path(), b"").0, 2);
}

#[test]
fn init_never_overwrites_and_config_can_be_explained() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    assert_eq!(run(&["init"], root, b"").0, 0);
    let config = fs::read(root.join(".gdfmtrc.json")).unwrap();
    assert_eq!(run(&["init"], root, b"").0, 2);
    assert_eq!(fs::read(root.join(".gdfmtrc.json")).unwrap(), config);
    let source = "var x=1";
    fs::write(root.join("test.gd"), source).unwrap();
    let (code, stdout, _) = run(&["explain-config", "test.gd"], root, b"");
    assert_eq!(code, 0);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        report["configuration"]["config"]["format"]["lineWidth"],
        100
    );
}

#[test]
fn format_check_diff_and_default_workflow_share_the_verified_core() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let source = "var values = call( 1,[2,3] )\n";
    let expected = "var values = call(1, [2, 3])\n";
    fs::write(root.join("test.gd"), source).unwrap();
    for args in [
        vec!["check", "test.gd"],
        vec!["--check", "test.gd"],
        vec!["--diff", "test.gd"],
    ] {
        let (code, stdout, stderr) = run(&args, root, b"");
        assert_eq!(code, 1, "{stdout}\n{stderr}");
        assert_eq!(fs::read_to_string(root.join("test.gd")).unwrap(), source);
        if args[0] == "--diff" {
            assert!(stdout.contains("@@ -1,1 +1,1 @@\n-var values"));
            assert!(stdout.contains("+var values = call(1, [2, 3])"));
        }
    }
    assert_eq!(run(&["check", "--syntax-only", "test.gd"], root, b"").0, 0);
    let (code, stdout, _) = run(&["format", "test.gd", "--reporter=json"], root, b"");
    assert_eq!(code, 0);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["files"][0]["changed"], true);
    assert_eq!(fs::read_to_string(root.join("test.gd")).unwrap(), expected);
    assert_eq!(run(&["check", "test.gd"], root, b"").0, 0);
    assert_eq!(run(&[], root, b"").0, 0);
    fs::write(root.join("test.gd"), source).unwrap();
    assert_eq!(run(&["test.gd"], root, b"").0, 0);
    assert_eq!(fs::read_to_string(root.join("test.gd")).unwrap(), expected);
}

#[test]
fn invalid_scripts_are_never_written_and_stdin_stdout_stays_clean() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let invalid = "var values = [1, )\n";
    fs::write(root.join("bad.gd"), invalid).unwrap();
    for args in [
        vec!["bad.gd"],
        vec!["check", "bad.gd"],
        vec!["check", "--syntax-only", "bad.gd"],
    ] {
        assert_eq!(run(&args, root, b"").0, 1);
        assert_eq!(fs::read_to_string(root.join("bad.gd")).unwrap(), invalid);
    }
    let (code, stdout, stderr) = run(
        &["--stdin-filepath", "new/path.gd"],
        root,
        b"var a = [1,2]\n",
    );
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, "var a = [1, 2]\n");
    assert!(stderr.is_empty());
    let (code, stdout, _) = run(&["format", "-"], root, invalid.as_bytes());
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert_eq!(run(&["format", "-"], root, &[0xff]).0, 2);
    assert_eq!(run(&["--stdin-filepath", "x.gd", "bad.gd"], root, b"").0, 2);
}

#[test]
fn formatting_uses_config_profile_width_and_indentation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join(".gdfmtrc.json"), r#"{"language":{"godotVersion":"4.3"},"format":{"lineWidth":40,"indentStyle":"space","indentWidth":2}}"#).unwrap();
    let (code, stdout, _) = run(
        &["format", "-"],
        root,
        b"var a = call(abcdefghijk, lmnopqrstuv, xyzabcdefghi)\n",
    );
    assert_eq!(code, 0);
    assert_eq!(
        stdout,
        "var a = call(\n  abcdefghijk,\n  lmnopqrstuv,\n  xyzabcdefghi\n)\n"
    );
    let (code, stdout, _) = run(
        &["--stdin-filepath", "x.gd"],
        root,
        b"var a: Dictionary[String, int] = {}\n",
    );
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    fs::write(root.join("ok.gd"), "var a = [1,2]\n").unwrap();
    fs::create_dir(root.join("nested")).unwrap();
    fs::write(root.join("nested/b.gd"), "pass\n").unwrap();
    fs::write(
        root.join("nested/.gdfmtrc.json"),
        r#"{"format":{"lineWidth":0}}"#,
    )
    .unwrap();
    assert_eq!(run(&[], root, b"").0, 2);
    assert_eq!(
        fs::read_to_string(root.join("ok.gd")).unwrap(),
        "var a = [1,2]\n"
    );
}

#[test]
fn whole_file_settings_apply_through_stdin_and_file_writes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join(".gdfmtrc.json"), r#"{"format":{"indentStyle":"space","indentWidth":2,"lineEnding":"crlf","maxBlankLines":0,"finalNewline":false}}"#).unwrap();
    let source = "## header\n\n\nfunc run( value : int=1 ): return value+1\n";
    let expected = "## header\r\nfunc run(value: int = 1):\r\n  return value + 1";
    let (code, stdout, stderr) = run(&["format", "-"], root, source.as_bytes());
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, expected);
    fs::write(root.join("case.gd"), source).unwrap();
    assert_eq!(run(&["format", "case.gd"], root, b"").0, 0);
    assert_eq!(fs::read_to_string(root.join("case.gd")).unwrap(), expected);
    assert_eq!(run(&["check", "case.gd"], root, b"").0, 0);
}

#[test]
fn configurations_are_preflighted_and_invalid_utf8_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir(root.join("broken")).unwrap();
    fs::write(root.join("a.gd"), "pass\n").unwrap();
    fs::write(root.join("broken/b.gd"), "pass\n").unwrap();
    fs::write(
        root.join("broken/.gdfmtrc.json"),
        r#"{"format":{"lineWidth":0}}"#,
    )
    .unwrap();
    let (code, stdout, _) = run(&["lex", ".", "--reporter=json"], root, b"");
    assert_eq!(code, 2);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["diagnostics"][0]["code"], "CONFIG_INVALID");
    assert!(report["files"].as_array().unwrap().is_empty());
    fs::write(root.join("invalid.gd"), [0xff]).unwrap();
    assert_eq!(run(&["lex", "invalid.gd"], root, b"").0, 2);
}

#[test]
fn compiled_binary_uses_the_shared_runner() {
    let output = Command::new(env!("CARGO_BIN_EXE_gdfmt"))
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .starts_with("gdfmt 0.1.0-alpha.0")
    );
}

#[test]
fn inherited_overrides_and_editorconfig_match_stdin_file_and_explain() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("base.json"), r#"{"format":{"maxBlankLines":0}}"#).unwrap();
    fs::write(root.join(".gdfmtrc.json"), r#"{"root":true,"extends":["base.json"],"overrides":[{"files":["src/case.gd"],"format":{"indentWidth":2}}]}"#).unwrap();
    fs::write(root.join(".editorconfig"), "root=true\n[*.gd]\nindent_style=space\nindent_size=6\nend_of_line=crlf\ninsert_final_newline=false\n").unwrap();
    let source = b"func run():\n\treturn 1\n";
    let expected = "func run():\r\n  return 1";
    let (code, stdout, stderr) = run(&["--stdin-filepath", "src/case.gd"], root, source);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout, expected);
    assert!(!root.join("src/case.gd").exists());
    let (_, stdout, _) = run(&["explain-config", "src/case.gd"], root, b"");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        report["configuration"]["config"]["format"]["indentWidth"],
        2
    );
    assert_eq!(report["configuration"]["ignored"], false);
    fs::write(root.join("src/case.gd"), source).unwrap();
    assert_eq!(run(&["format", "src/case.gd"], root, b"").0, 0);
    assert_eq!(
        fs::read_to_string(root.join("src/case.gd")).unwrap(),
        expected
    );
    assert_eq!(run(&["check", "src/case.gd"], root, b"").0, 0);
}

#[test]
fn ignored_explicit_files_and_stdin_fail_cleanly_and_never_write() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join(".gdfmtrc.json"), r#"{"root":true}"#).unwrap();
    fs::write(root.join(".gdfmtignore"), "ignored.gd\n").unwrap();
    fs::write(root.join("ignored.gd"), "var a=1\n").unwrap();
    for args in [
        vec!["format", "ignored.gd"],
        vec!["check", "ignored.gd"],
        vec!["lex", "ignored.gd"],
        vec!["--stdin-filepath", "ignored.gd"],
    ] {
        let (code, stdout, stderr) = run(&args, root, b"var a=2\n");
        assert_eq!(code, 2);
        assert!(stdout.is_empty());
        assert!(!stderr.is_empty());
        let mut json_args = args;
        json_args.push("--reporter=json");
        let (code, stdout, stderr) = run(&json_args, root, b"var a=2\n");
        assert_eq!(code, 2);
        assert!(stderr.is_empty());
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(report["status"], "error");
        assert!(report["files"].as_array().unwrap().is_empty());
        assert_eq!(
            fs::read_to_string(root.join("ignored.gd")).unwrap(),
            "var a=1\n"
        );
    }
    let (_, stdout, _) = run(&["explain-config", "ignored.gd"], root, b"");
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["configuration"]["ignored"], true);
    assert_eq!(run(&["format", "."], root, b"").0, 2);
}

#[test]
fn discovery_reports_stable_deduplicated_ids_and_mixed_errors_choose_exit_two() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(root.join("a.gd"), "var =\n").unwrap();
    fs::write(root.join("b.gd"), "var b=1\n").unwrap();
    let path = root.join("b.gd");
    let permissions = fs::metadata(&path).unwrap().permissions();
    let mut readonly = permissions.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly).unwrap();
    let (code, stdout, stderr) = run(
        &["format", "b.gd", ".", "a.gd", "--reporter=json"],
        root,
        b"",
    );
    fs::set_permissions(&path, permissions).unwrap();
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.is_empty());
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let files = report["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
    assert!(files[0]["path"].as_str().unwrap().ends_with("a.gd"));
    assert_eq!(files[0]["sourceId"], 0);
    assert_eq!(files[1]["sourceId"], 1);
    assert_eq!(files[1]["diagnostics"][0]["code"], "CLI_IO");
    assert_eq!(fs::read_to_string(path).unwrap(), "var b=1\n");
    assert_eq!(fs::read_to_string(root.join("a.gd")).unwrap(), "var =\n");
}

#[test]
fn bad_later_source_reads_and_inheritance_fail_before_any_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let original = "var a=1\n";
    fs::write(root.join("a.gd"), original).unwrap();
    fs::write(root.join("z.gd"), [0xff]).unwrap();
    assert_eq!(run(&["format", "."], root, b"").0, 2);
    assert_eq!(fs::read_to_string(root.join("a.gd")).unwrap(), original);
    fs::remove_file(root.join("z.gd")).unwrap();
    fs::create_dir(root.join("z")).unwrap();
    fs::write(root.join("z/z.gd"), "pass\n").unwrap();
    fs::write(
        root.join("z/.gdfmtrc.json"),
        r#"{"extends":[".gdfmtrc.json"]}"#,
    )
    .unwrap();
    let (code, stdout, _) = run(&["format", ".", "--reporter=json"], root, b"");
    assert_eq!(code, 2);
    let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["diagnostics"][0]["code"], "CONFIG_INVALID");
    assert!(report["files"].as_array().unwrap().is_empty());
    assert_eq!(fs::read_to_string(root.join("a.gd")).unwrap(), original);
}

#[test]
fn stdin_and_file_size_limits_refuse_output_without_touching_sources() {
    let temp = tempfile::tempdir().unwrap();
    let oversized = vec![b' '; 16 * 1024 * 1024 + 1];
    assert_eq!(run(&["format", "-"], temp.path(), &oversized).0, 2);
    assert_eq!(run(&["lex", "-"], temp.path(), &oversized).0, 2);
    fs::write(temp.path().join("a.gd"), "var a=1\n").unwrap();
    fs::write(temp.path().join("z.gd"), &oversized).unwrap();
    assert_eq!(run(&["format", "."], temp.path(), b"").0, 2);
    assert_eq!(
        fs::read_to_string(temp.path().join("a.gd")).unwrap(),
        "var a=1\n"
    );
    assert_eq!(fs::read(temp.path().join("z.gd")).unwrap(), oversized);
}

#[test]
fn invalid_mode_combinations_and_empty_discovery_have_no_side_effects() {
    let temp = tempfile::tempdir().unwrap();
    for args in [
        vec!["format", "--check", "--diff"],
        vec!["check", "--syntax-only", "--diff"],
        vec!["lex", "--diff"],
        vec!["schema", "--stdin-filepath", "x.gd"],
        vec!["format", "-", "x.gd"],
        vec!["--stdin-filepath", "x.txt"],
        vec!["format", "."],
    ] {
        assert_eq!(run(&args, temp.path(), b"").0, 2, "{args:?}");
    }
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn stream_read_and_write_failures_return_io_status() {
    struct Broken;
    impl std::io::Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("injected read failure"))
        }
    }
    impl std::io::Write for Broken {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    assert_eq!(
        gdfmt_cli::run(
            ["gdfmt", "format", "-"],
            temp.path(),
            &mut Broken,
            &mut stdout,
            &mut stderr
        ),
        2
    );
    assert!(stdout.is_empty());
    let mut source: &[u8] = b"var a=1\n";
    assert_eq!(
        gdfmt_cli::run(
            ["gdfmt", "format", "-"],
            temp.path(),
            &mut source,
            &mut Broken,
            &mut stderr
        ),
        2
    );
    let mut source: &[u8] = b"var =\n";
    assert_eq!(
        gdfmt_cli::run(
            ["gdfmt", "format", "-"],
            temp.path(),
            &mut source,
            &mut stdout,
            &mut Broken
        ),
        2
    );
}
