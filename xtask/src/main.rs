use gdfmt_parser::{LanguageVersion, LexOptions, ParseOptions, lex, parse, parse_expression};
use gdfmt_syntax::{LosslessTree, SyntaxKind as K, ast::Declaration};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

mod conformance;
mod coverage;
mod trials;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    fixtures: Vec<Fixture>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    file: String,
    stage: Stage,
    lexical_errors: Vec<String>,
    #[serde(default)]
    syntax_errors: Vec<String>,
    /// Parser-driven lambda suites can expose indentation diagnostics hidden
    /// by the standalone lexer's enclosing delimiter context.
    parser_lexical_errors: Option<Vec<String>>,
    syntax_errors_by_version: Option<BTreeMap<LanguageVersion, Vec<String>>>,
    /// Names that must remain direct source-file declarations after an error.
    #[serde(default)]
    recovered_declarations: Vec<String>,
}

fn missing_recovered_declarations(tree: &LosslessTree, expected: &[String]) -> Vec<String> {
    let names: Vec<_> = tree
        .root()
        .children()
        .filter(|node| node.kind() != K::ExtendsDecl)
        .filter_map(Declaration::cast)
        .filter_map(|node| {
            node.syntax()
                .children_with_tokens()
                .filter_map(|element| element.into_token())
                .find(|token| token.kind().is_identifier(token.text()))
                .map(|token| token.text().to_string())
        })
        .collect();
    expected
        .iter()
        .filter(|name| !names.contains(name))
        .cloned()
        .collect()
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Stage {
    Lexer,
    Expression,
    Script,
}

fn validate_fixture_inventory(
    directory: &Path,
    manifest: &Manifest,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut registered = BTreeSet::new();
    for fixture in &manifest.fixtures {
        let path = Path::new(&fixture.file);
        if path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
            || path.parent() != Some(Path::new("fixtures"))
            || path.extension().is_none_or(|extension| extension != "gd")
            || !registered.insert(fixture.file.clone())
        {
            return Err(format!("invalid or duplicate fixture: {}", fixture.file).into());
        }
    }
    let files: BTreeSet<_> = fs::read_dir(directory.join("fixtures"))?
        .map(|entry| entry.map(|entry| format!("fixtures/{}", entry.file_name().to_string_lossy())))
        .collect::<Result<_, _>>()?;
    if files != registered {
        return Err(format!(
            "fixture inventory differs: unregistered {:?}, missing {:?}",
            files.difference(&registered).collect::<Vec<_>>(),
            registered.difference(&files).collect::<Vec<_>>()
        )
        .into());
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    match args.first().map(String::as_str) {
        Some("schema") if args.len() == 1 || args == ["schema", "--check"] => {
            let path = root.join("schemas/gdfmtrc.schema.json");
            let generated = gdfmt_config::schema_json();
            if args.len() == 2 {
                if fs::read_to_string(&path)? != generated {
                    return Err("schema is stale; run cargo xtask schema".into());
                }
                println!("schema is current");
            } else {
                fs::create_dir_all(path.parent().expect("schema directory"))?;
                fs::write(path, generated)?;
                println!("generated schemas/gdfmtrc.schema.json");
            }
        }
        Some("compatibility") if args.len() == 1 => {
            let directory = root.join("tests/compatibility");
            let manifest: Manifest =
                serde_json::from_str(&fs::read_to_string(directory.join("manifest.json"))?)?;
            if manifest.fixtures.is_empty() {
                return Err("compatibility corpus is empty".into());
            }
            validate_fixture_inventory(&directory, &manifest)?;
            let mut expressions = 0;
            let mut scripts = 0;
            let mut version_checks = 0;
            for fixture in &manifest.fixtures {
                let source = fs::read_to_string(directory.join(&fixture.file))?;
                let result = lex(&source, &LexOptions::default());
                let reconstructed = result.tree.as_ref().ok_or("missing lossless tree")?.text();
                if reconstructed != source {
                    return Err(format!("{}: lossless reconstruction failed", fixture.file).into());
                }
                let mut actual: Vec<_> = result
                    .diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.code.clone())
                    .collect();
                actual.sort();
                actual.dedup();
                let mut expected = fixture.lexical_errors.clone();
                expected.sort();
                expected.dedup();
                if actual != expected {
                    return Err(
                        format!("{}: expected {expected:?}, got {actual:?}", fixture.file).into(),
                    );
                }
                if fixture.stage != Stage::Lexer {
                    let result = if fixture.stage == Stage::Script {
                        scripts += 1;
                        parse(&source, &ParseOptions::default())
                    } else {
                        expressions += 1;
                        parse_expression(&source, &ParseOptions::default())
                    };
                    if result.tree.as_ref().ok_or("missing parser tree")?.text() != source {
                        return Err(
                            format!("{}: parser reconstruction failed", fixture.file).into()
                        );
                    }
                    let missing = missing_recovered_declarations(result.tree.as_ref().unwrap(), &fixture.recovered_declarations);
                    if !missing.is_empty() {
                        return Err(format!("{}: missing recovered declarations {missing:?}", fixture.file).into());
                    }
                    let mut actual: Vec<_> = result
                        .diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.code.clone())
                        .collect();
                    actual.sort();
                    actual.dedup();
                    let mut expected = fixture
                        .parser_lexical_errors
                        .as_ref()
                        .unwrap_or(&fixture.lexical_errors)
                        .clone();
                    expected.extend(fixture.syntax_errors.clone());
                    expected.sort();
                    expected.dedup();
                    if actual != expected {
                        return Err(format!(
                            "{}: expected parser diagnostics {expected:?}, got {actual:?}",
                            fixture.file
                        )
                        .into());
                    }
                } else if !fixture.syntax_errors.is_empty()
                    || fixture.parser_lexical_errors.is_some()
                    || fixture.syntax_errors_by_version.is_some()
                    || !fixture.recovered_declarations.is_empty()
                {
                    return Err(format!(
                        "{}: parser expectations require expression or script stage",
                        fixture.file
                    )
                    .into());
                }
                if let Some(matrix) = &fixture.syntax_errors_by_version {
                    for version in LanguageVersion::ALL {
                        let expected_syntax = matrix.get(&version).ok_or_else(|| {
                            format!("{}: missing {version} expectation", fixture.file)
                        })?;
                        let options = ParseOptions {
                            language_version: version,
                            ..Default::default()
                        };
                        let result = if fixture.stage == Stage::Script {
                            parse(&source, &options)
                        } else {
                            parse_expression(&source, &options)
                        };
                        if result.tree.as_ref().ok_or("missing versioned tree")?.text() != source {
                            return Err(format!(
                                "{} ({version}): lossless reconstruction failed",
                                fixture.file
                            )
                            .into());
                        }
                        let missing = missing_recovered_declarations(result.tree.as_ref().unwrap(), &fixture.recovered_declarations);
                        if !missing.is_empty() {
                            return Err(format!("{} ({version}): missing recovered declarations {missing:?}", fixture.file).into());
                        }
                        let mut actual: Vec<_> =
                            result.diagnostics.iter().map(|d| d.code.clone()).collect();
                        let mut expected = fixture
                            .parser_lexical_errors
                            .as_ref()
                            .unwrap_or(&fixture.lexical_errors)
                            .clone();
                        expected.extend(expected_syntax.clone());
                        actual.sort();
                        actual.dedup();
                        expected.sort();
                        expected.dedup();
                        if actual != expected {
                            return Err(format!(
                                "{} ({version}): expected {expected:?}, got {actual:?}",
                                fixture.file
                            )
                            .into());
                        }
                        version_checks += 1;
                    }
                }
            }
            println!(
                "{} lexical, {} standalone expression and {} script fixtures; {} version checks passed (not engine conformance)",
                manifest.fixtures.len(),
                expressions,
                scripts,
                version_checks
            );
        }
        Some("conformance") => conformance::run(&root, &args[1..])?,
        Some("formatter-conformance") => conformance::run_formatter(&root, &args[1..])?,
        Some("syntax-coverage") if args.len() == 1 => coverage::run(&root)?,
        Some("project-trials") => trials::run(&args[1..])?,
        _ => return Err("usage: cargo xtask schema [--check] | compatibility | syntax-coverage | project-trials --input <JSON> --report <JSON> | conformance|formatter-conformance --godot <binary> [--language <profile>] [--report <file>]".into()),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_requires_independent_declarations_in_the_source_file() {
        let expected = vec!["sentinel".into(), "after".into()];
        for source in [
            "var broken =\nvar sentinel = 1\nfunc after(): pass\n",
            "var broken = [1,\nvar sentinel = 1\nfunc after(): pass\n",
            "class Inner:\n    var sentinel = 1\n    func after(): pass\n",
            "extends sentinel\nfunc after(): pass\n",
        ] {
            let result = parse(source, &ParseOptions::default());
            let missing = missing_recovered_declarations(&result.tree.unwrap(), &expected);
            assert_eq!(
                missing.is_empty(),
                source.starts_with("var broken =\n"),
                "{source:?}: {missing:?}"
            );
        }
    }
}
