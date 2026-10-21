//! Offline formatter invariant gate for hash-verified, externally prepared inputs.
use gdfmt::{FormatRequest, LanguageVersion, LineEnding};
use serde::{Deserialize, Serialize};
use std::{error::Error, fs, path::PathBuf};

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Input {
    project: String,
    path: String,
    source: String,
    #[serde(default)]
    expected_diagnostics: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Layout {
    width: usize,
    indent: String,
    passed: bool,
    diagnostics: Vec<String>,
    output: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    input: Input,
    source_rejected: bool,
    layouts: Vec<Layout>,
}

pub(super) fn run(args: &[String]) -> Result<(), Box<dyn Error>> {
    let mut input = None;
    let mut report = None;
    let mut arguments = args.iter();
    while let Some(argument) = arguments.next() {
        let value = arguments.next().ok_or("expected a value after option")?;
        match argument.as_str() {
            "--input" => input = Some(PathBuf::from(value)),
            "--report" => report = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown project-trials option: {argument}").into()),
        }
    }
    let inputs: Vec<Input> = serde_json::from_str(&fs::read_to_string(
        input.ok_or("project-trials requires --input <prepared JSON>")?,
    )?)?;
    if inputs.is_empty() {
        return Err("project trial inputs are empty".into());
    }
    let mut cases = Vec::new();
    let mut failures = 0;
    for input in inputs {
        if input
            .expected_diagnostics
            .iter()
            .any(|code| !code.starts_with("LEX") && !code.starts_with("PAR"))
        {
            return Err(
                "source rejection expectations may only contain lexical/parser diagnostics".into(),
            );
        }
        let mut layouts = Vec::new();
        for (width, indent, ending, blanks, final_newline) in [
            (100, "\t", LineEnding::Preserve, 2, true),
            (40, "\t", LineEnding::Preserve, 2, true),
            (40, "  ", LineEnding::Crlf, 0, false),
            (100, "    ", LineEnding::Lf, 1, true),
        ] {
            let mut request = FormatRequest::default();
            request.parse.language_version = LanguageVersion::Godot4_7;
            request.format.line_width = width;
            request.format.indent = indent.into();
            request.format.line_ending = ending;
            request.format.max_blank_lines = blanks;
            request.format.final_newline = final_newline;
            let result = gdfmt::format(&input.source, &request);
            let mut actual: Vec<_> = result.diagnostics.iter().map(|d| d.code.clone()).collect();
            actual.sort();
            actual.dedup();
            let passed = if input.expected_diagnostics.is_empty() {
                result.output.is_some()
            } else {
                result.output.is_none() && actual == input.expected_diagnostics
            };
            if !passed {
                failures += 1;
                eprintln!(
                    "{}/{} width={width} indent={indent:?}: {:?}",
                    input.project, input.path, result.diagnostics
                );
            }
            layouts.push(Layout {
                width,
                indent: indent.into(),
                passed,
                diagnostics: result
                    .diagnostics
                    .iter()
                    .map(|d| format!("{}: {} at {:?}", d.code, d.message, d.span))
                    .collect(),
                output: result.output,
            });
        }
        let source_rejected = !input.expected_diagnostics.is_empty();
        cases.push(Case {
            input,
            source_rejected,
            layouts,
        });
    }
    let report = report.ok_or("project-trials requires --report <JSON>")?;
    if let Some(parent) = report.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(report, serde_json::to_string_pretty(&cases)? + "\n")?;
    let rejected = cases.iter().filter(|case| case.source_rejected).count();
    println!(
        "{} project scripts, {} valid layouts, {rejected} expected source rejections, {failures} unexpected failures",
        cases.len(),
        (cases.len() - rejected) * 4
    );
    if failures > 0 {
        return Err("project trial invariant gate failed; inspect report".into());
    }
    Ok(())
}
