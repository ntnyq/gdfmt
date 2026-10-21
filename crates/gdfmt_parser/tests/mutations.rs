//! Deterministic mutations at physical token boundaries, isolated so a parser
//! hang or stack overflow fails the parent test instead of hanging the suite.
use gdfmt_parser::{LanguageVersion, LexOptions, ParseOptions, lex, parse, parse_expression};
use std::{
    fs,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[test]
fn token_mutations_terminate_and_preserve_source_in_every_profile() {
    for version in LanguageVersion::ALL {
        let log = tempfile::NamedTempFile::new().unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "mutation_worker", "--ignored", "--nocapture"])
            .env("GDFMT_MUTATION_PROFILE", version.to_string())
            .stdout(Stdio::from(log.as_file().try_clone().unwrap()))
            .stderr(Stdio::from(log.as_file().try_clone().unwrap()))
            .spawn()
            .unwrap();
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if start.elapsed() > Duration::from_secs(60) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "mutation timeout for {version}: {}",
                    fs::read_to_string(log.path()).unwrap()
                );
            }
            thread::sleep(Duration::from_millis(20));
        };
        let output = fs::read_to_string(log.path()).unwrap();
        assert!(
            status.success(),
            "mutation worker {version}: {status}\n{output}"
        );
        // Keep the case count visible with --nocapture, without every seed name.
        println!(
            "{}",
            output
                .lines()
                .find(|line| line.starts_with("checked "))
                .unwrap()
        );
    }
}

#[test]
#[ignore = "subprocess worker invoked by token_mutations_terminate_and_preserve_source_in_every_profile"]
fn mutation_worker() {
    let profile = std::env::var("GDFMT_MUTATION_PROFILE").expect("worker profile");
    let version = LanguageVersion::ALL
        .into_iter()
        .find(|version| version.to_string() == profile)
        .unwrap();
    let options = ParseOptions {
        language_version: version,
        lex: LexOptions {
            max_diagnostics: 8,
            ..Default::default()
        },
        ..Default::default()
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/compatibility");
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/compatibility/manifest.json")).unwrap();
    let mut count = 0;
    for fixture in manifest["fixtures"].as_array().unwrap() {
        let file = fixture["file"].as_str().unwrap();
        println!("seed {file}");
        let source = fs::read_to_string(root.join(file)).unwrap();
        let tokens = lex(&source, &LexOptions::default()).tokens;
        for token in tokens.into_iter().filter(|token| !token.kind.is_virtual()) {
            let start = token.span.start() as usize;
            let end = token.span.end() as usize;
            let original = &source[start..end];
            // Deletion, duplication, delimiter, keyword, newline/indentation and
            // assignment substitutions exercise recovery as well as retokenizing.
            for replacement in [
                String::new(),
                original.repeat(2),
                ")".into(),
                "var".into(),
                "\n\t".into(),
                "=".into(),
            ] {
                let mutated = format!("{}{}{}", &source[..start], replacement, &source[end..]);
                let result = if fixture["stage"] == "expression" {
                    parse_expression(&mutated, &options)
                } else {
                    parse(&mutated, &options)
                };
                let context =
                    || format!("{profile} {file} {start}..{end} -> {replacement:?}: {mutated:?}");
                assert!(
                    result.diagnostics.len() <= 9,
                    "diagnostic budget: {}",
                    context()
                );
                for diagnostic in &result.diagnostics {
                    assert!(
                        diagnostic.span.slice(&mutated).is_ok(),
                        "invalid span: {}",
                        context()
                    );
                }
                let tree = result.tree.expect("small source always yields a tree");
                assert_eq!(tree.text(), mutated, "{}", context());
                let mut offset = 0;
                for token in tree
                    .root()
                    .descendants_with_tokens()
                    .filter_map(|element| element.into_token())
                {
                    assert_eq!(
                        u32::from(token.text_range().start()),
                        offset,
                        "{}",
                        context()
                    );
                    assert!(
                        !token.kind().is_virtual() || token.text().is_empty(),
                        "{}",
                        context()
                    );
                    offset += token.text().len() as u32;
                }
                assert_eq!(offset as usize, mutated.len(), "{}", context());
                count += 1;
            }
        }
    }
    println!("checked {count} token mutations for {profile}");
}
