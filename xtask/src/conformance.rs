//! Pinned, isolated check-only engine comparisons; never executes fixture code.
use crate::{Manifest, Stage, missing_recovered_declarations};
use gdfmt_parser::{LanguageFeature, LanguageVersion, ParseOptions, parse};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    error::Error,
    ffi::OsStr,
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const TIMEOUT: Duration = Duration::from_secs(15);
const LOG_LIMIT: u64 = 1024 * 1024;
const PROJECT: &str = "config_version=5\n[application]\nconfig/name=\"gdfmt conformance\"\nconfig/use_custom_user_dir=true\nconfig/custom_user_dir_name=\"gdfmt-conformance\"\n[debug]\ngdscript/warnings/treat_warnings_as_errors=false\n";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EngineManifest {
    schema_version: u32,
    engines: Vec<EnginePin>,
    gates: Vec<GateCoverage>,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct EnginePin {
    language_version: LanguageVersion,
    tag: String,
    commit: String,
    engine_version: String,
    linux_sha512: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GateCoverage {
    feature: String,
    positive: String,
    negative: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Case {
    file: String,
    expectations: BTreeMap<LanguageVersion, Expectation>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Expectation {
    kind: CaseKind,
    /// A pinned diagnostic excerpt, not a heuristic classifier of engine stages.
    engine_error: Option<String>,
    note: Option<String>,
}

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum CaseKind {
    Accepted,
    Syntax,
    Analyzer,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Observation {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Observation {
    fn rejected(&self) -> bool {
        self.normalized_log().contains("SCRIPT ERROR: Parse Error:")
    }

    fn normalized_log(&self) -> String {
        // Legacy editors emit CSI colors even with NO_COLOR. Preserve raw logs
        // in reports, but compare text without terminal control sequences.
        let raw = format!("{}\n{}", self.stdout, self.stderr);
        let mut chars = raw.chars().peekable();
        let mut normalized = String::new();
        while let Some(character) = chars.next() {
            if character == '\x1b' && chars.peek() == Some(&'[') {
                chars.next();
                for control in chars.by_ref() {
                    if ('\x40'..='\x7e').contains(&control) {
                        break;
                    }
                }
            } else {
                normalized.push(character);
            }
        }
        normalized
    }

    fn healthy(&self) -> bool {
        // Some builds return zero for script errors. A crash or startup error
        // must never count as a successful negative test.
        matches!(self.exit_code, Some(0 | 1))
            && if self.rejected() {
                let log = self.normalized_log();
                log.contains("res://case.gd:") && log.contains("Failed to load script")
            } else {
                self.exit_code == Some(0)
                    && !self.stdout.contains("ERROR:")
                    && !self.stderr.contains("ERROR:")
            }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CaseReport {
    file: String,
    kind: CaseKind,
    note: Option<String>,
    expected_engine_error: Option<String>,
    parser_diagnostics: Vec<String>,
    expected_recovered_declarations: Vec<String>,
    missing_recovered_declarations: Vec<String>,
    engine: Observation,
    passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    formatted_source: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    schema_version: u32,
    engine_tag: String,
    engine_commit: String,
    engine_version: String,
    language_version: LanguageVersion,
    cases: Vec<CaseReport>,
    formatted: bool,
}

pub(super) fn run(root: &Path, args: &[String]) -> Result<()> {
    run_mode(root, args, false)
}

pub(super) fn run_formatter(root: &Path, args: &[String]) -> Result<()> {
    run_mode(root, args, true)
}

fn run_mode(root: &Path, args: &[String], formatted: bool) -> Result<()> {
    let mut binary = std::env::var_os("GODOT_BIN").map(PathBuf::from);
    let mut report_path = root.join(if formatted {
        "target/conformance/formatter.json"
    } else {
        "target/conformance/report.json"
    });
    let mut language = None;
    let mut arguments = args.iter();
    while let Some(argument) = arguments.next() {
        let value = arguments.next().ok_or("expected a value after option")?;
        match argument.as_str() {
            "--godot" => binary = Some(PathBuf::from(value)),
            "--report" => report_path = PathBuf::from(value),
            "--language" => {
                language = Some(serde_json::from_value::<LanguageVersion>(
                    value.clone().into(),
                )?)
            }
            _ => return Err(format!("unknown conformance option: {argument}").into()),
        }
    }
    let binary = binary.ok_or("set GODOT_BIN or pass --godot <pinned editor binary>")?;
    // Resolve a relative executable before switching the child's cwd.
    let binary = if binary.components().count() > 1 {
        fs::canonicalize(binary)?
    } else {
        binary
    };
    let directory = root.join("tests/compatibility");
    let manifest: EngineManifest =
        serde_json::from_str(&fs::read_to_string(directory.join("engine.json"))?)?;
    let corpus: Manifest =
        serde_json::from_str(&fs::read_to_string(directory.join("manifest.json"))?)?;
    let baseline: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(directory.join("manifest.json"))?)?;
    validate_manifest(&manifest, &corpus, &baseline)?;

    let workspace = tempfile::tempdir()?;
    let version = invoke(&binary, workspace.path(), &[OsStr::new("--version")])?;
    let pin = manifest
        .engines
        .iter()
        .find(|pin| version.exit_code == Some(0) && version.stdout.trim() == pin.engine_version)
        .ok_or_else(|| {
            format!(
                "Godot version mismatch: no exact engine pin for {:?} (exit {:?})",
                version.stdout.trim(),
                version.exit_code
            )
        })?;
    if language.is_some_and(|language| language != pin.language_version) {
        return Err("Godot binary does not match the requested language profile".into());
    }
    // Both controls are required before trusting an engine's command-line mode.
    for (source, rejected) in [("var value = 1\n", false), ("func broken(:\n", true)] {
        let result = check(&binary, source.as_bytes())?;
        if !result.healthy() || result.rejected() != rejected {
            return Err(format!(
                "Godot check-only control failed: {}{}",
                result.stdout, result.stderr
            )
            .into());
        }
    }
    let mut report = Report {
        schema_version: 2,
        engine_tag: pin.tag.clone(),
        engine_commit: pin.commit.clone(),
        engine_version: pin.engine_version.clone(),
        language_version: pin.language_version,
        cases: Vec::new(),
        formatted,
    };
    for case in &manifest.cases {
        let expectation = &case.expectations[&pin.language_version];
        if formatted && expectation.kind != CaseKind::Accepted {
            continue;
        }
        let fixture = corpus
            .fixtures
            .iter()
            .find(|fixture| fixture.file == case.file)
            .ok_or("engine case missing corpus fixture")?;
        let source_path = fs::canonicalize(directory.join(&case.file))?;
        if !source_path.starts_with(fs::canonicalize(&directory)?) {
            return Err(format!("fixture escapes corpus: {}", case.file).into());
        }
        let source = fs::read_to_string(source_path)?;
        let parsed = parse(
            &source,
            &ParseOptions {
                language_version: pin.language_version,
                ..Default::default()
            },
        );
        let lossless = parsed
            .tree
            .as_ref()
            .is_some_and(|tree| tree.text() == source);
        let parser_rejected = parsed.has_errors();
        let missing_declarations = parsed
            .tree
            .as_ref()
            .map(|tree| missing_recovered_declarations(tree, &fixture.recovered_declarations))
            .unwrap_or_else(|| fixture.recovered_declarations.clone());
        let mut parser_diagnostics: Vec<_> = parsed
            .diagnostics
            .iter()
            .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message))
            .collect();
        let formatted_source = if formatted {
            let result = gdfmt::format(
                &source,
                &gdfmt::FormatRequest {
                    parse: ParseOptions {
                        language_version: pin.language_version,
                        ..Default::default()
                    },
                    format: gdfmt::FormatOptions {
                        line_width: 40,
                        ..Default::default()
                    },
                },
            );
            parser_diagnostics.extend(
                result
                    .diagnostics
                    .iter()
                    .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.message)),
            );
            result.output
        } else {
            None
        };
        let engine = check(
            &binary,
            formatted_source.as_deref().unwrap_or(&source).as_bytes(),
        )?;
        let expected_rejection = expectation.kind != CaseKind::Accepted;
        let matching_error = expectation
            .engine_error
            .as_ref()
            .is_none_or(|message| engine.normalized_log().contains(message));
        let passed = lossless
            && (!formatted || formatted_source.is_some())
            && missing_declarations.is_empty()
            && engine.healthy()
            && engine.rejected() == expected_rejection
            && parser_rejected == (expectation.kind == CaseKind::Syntax)
            && matching_error;
        if !passed {
            eprintln!(
                "FAIL {}: parser rejected={parser_rejected}, engine rejected={}\n{}{}",
                case.file,
                engine.rejected(),
                engine.stdout,
                engine.stderr
            );
        }
        report.cases.push(CaseReport {
            file: case.file.clone(),
            kind: expectation.kind,
            note: expectation.note.clone(),
            expected_engine_error: expectation.engine_error.clone(),
            parser_diagnostics,
            expected_recovered_declarations: fixture.recovered_declarations.clone(),
            missing_recovered_declarations: missing_declarations,
            engine,
            passed,
            formatted_source,
        });
    }
    if let Some(parent) = report_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &report_path,
        format!("{}\n", serde_json::to_string_pretty(&report)?),
    )?;
    let failures = report.cases.iter().filter(|case| !case.passed).count();
    println!(
        "{}: {} engine cases, {} failures; report {}",
        report.engine_version,
        report.cases.len(),
        failures,
        report_path.display()
    );
    if failures > 0 {
        return Err(
            "engine conformance failed; inspect the report for parser diagnostics and engine logs"
                .into(),
        );
    }
    Ok(())
}

fn validate_manifest(
    manifest: &EngineManifest,
    corpus: &Manifest,
    baseline: &serde_json::Value,
) -> Result<()> {
    if manifest.schema_version != 2 || manifest.cases.is_empty() {
        return Err("unsupported or empty engine manifest".into());
    }
    let mut versions = BTreeSet::new();
    for pin in &manifest.engines {
        if !versions.insert(pin.language_version) {
            return Err("duplicate engine profile".into());
        }
        let (version, status) = pin.tag.rsplit_once('-').ok_or("invalid Godot tag")?;
        if pin.commit.len() != 40
            || !pin.commit.bytes().all(|b| b.is_ascii_hexdigit())
            || pin.linux_sha512.len() != 128
            || !pin.linux_sha512.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("expected a full Godot commit and Linux archive SHA-512".into());
        }
        let expected = format!("{version}.{status}.official.{}", &pin.commit[..9]);
        let profile = pin.language_version.to_string();
        if pin.engine_version != expected
            || status != "stable"
            || !(version == profile || version.starts_with(&format!("{profile}.")))
        {
            return Err("engine pin must agree with tag, commit and language profile".into());
        }
        if pin.language_version == LanguageVersion::default()
            && (baseline["baseline"]["godotCommit"].as_str() != Some(&pin.commit)
                || baseline["baseline"]["godotTag"].as_str() != Some(&pin.tag))
        {
            return Err("default engine pin must agree with corpus baseline".into());
        }
    }
    if LanguageVersion::ALL
        .iter()
        .any(|version| !versions.contains(version))
    {
        return Err("every language profile requires an engine pin".into());
    }
    let mut files = HashSet::new();
    for case in &manifest.cases {
        if !files.insert(&case.file) {
            return Err(format!("duplicate engine case: {}", case.file).into());
        }
        if !corpus
            .fixtures
            .iter()
            .any(|fixture| fixture.file == case.file && fixture.stage == Stage::Script)
        {
            return Err(
                format!("engine case must reference a script fixture: {}", case.file).into(),
            );
        }
        for version in LanguageVersion::ALL {
            let expectation = case
                .expectations
                .get(&version)
                .ok_or_else(|| format!("{}: missing {version} engine expectation", case.file))?;
            if expectation.kind != CaseKind::Accepted
                && expectation
                    .engine_error
                    .as_ref()
                    .is_none_or(|s| s.trim().is_empty())
            {
                return Err(format!(
                    "negative case requires a pinned engine diagnostic: {} ({version})",
                    case.file
                )
                .into());
            }
            if expectation.kind == CaseKind::Accepted && expectation.engine_error.is_some() {
                return Err(format!("accepted case cannot expect an error: {}", case.file).into());
            }
            if expectation.kind == CaseKind::Analyzer
                && expectation
                    .note
                    .as_ref()
                    .is_none_or(|s| s.trim().is_empty())
            {
                return Err(
                    format!("analyzer case requires a scope explanation: {}", case.file).into(),
                );
            }
        }
    }
    for fixture in corpus
        .fixtures
        .iter()
        .filter(|fixture| fixture.stage == Stage::Script)
    {
        if !files.contains(&fixture.file) {
            return Err(format!(
                "script fixture lacks engine classification: {}",
                fixture.file
            )
            .into());
        }
        if !fixture.recovered_declarations.is_empty() {
            let mut names = HashSet::new();
            if fixture
                .recovered_declarations
                .iter()
                .any(|name| name.trim().is_empty() || !names.insert(name))
            {
                return Err("recovery declarations must be nonempty and unique".into());
            }
            let case = manifest
                .cases
                .iter()
                .find(|case| case.file == fixture.file)
                .unwrap();
            if case
                .expectations
                .values()
                .any(|expectation| expectation.kind != CaseKind::Syntax)
            {
                return Err("recovery cases must be syntax errors in every profile".into());
            }
        }
    }
    let mut features = HashSet::new();
    for gate in &manifest.gates {
        let feature = LanguageFeature::ALL
            .into_iter()
            .find(|feature| format!("{feature:?}") == gate.feature)
            .ok_or_else(|| format!("unknown gate feature: {}", gate.feature))?;
        if !features.insert(gate.feature.as_str()) {
            return Err("duplicate gate coverage".into());
        }
        for (file, positive) in [(&gate.positive, true), (&gate.negative, false)] {
            let case = manifest
                .cases
                .iter()
                .find(|case| &case.file == file)
                .ok_or("gate fixture missing engine case")?;
            for version in LanguageVersion::ALL {
                let kind = if positive && version.supports(feature) {
                    CaseKind::Accepted
                } else {
                    CaseKind::Syntax
                };
                if case.expectations[&version].kind != kind {
                    return Err(format!(
                        "{}: gate coverage disagrees with {version} feature support",
                        gate.feature
                    )
                    .into());
                }
            }
        }
    }
    if features.len() != LanguageFeature::ALL.len() {
        return Err("every language gate requires positive and negative engine coverage".into());
    }
    Ok(())
}

fn check(binary: &Path, source: &[u8]) -> Result<Observation> {
    let project = tempfile::tempdir()?;
    fs::write(project.path().join("project.godot"), PROJECT)?;
    fs::write(project.path().join("case.gd"), source)?;
    invoke(
        binary,
        project.path(),
        &[
            OsStr::new("--headless"),
            OsStr::new("--path"),
            project.path().as_os_str(),
            OsStr::new("--check-only"),
            OsStr::new("--script"),
            OsStr::new("res://case.gd"),
            OsStr::new("--log-file"),
            project.path().join("engine.log").as_os_str(),
        ],
    )
}

fn invoke(binary: &Path, directory: &Path, args: &[&OsStr]) -> Result<Observation> {
    invoke_with_timeout(binary, directory, args, TIMEOUT)
}

fn invoke_with_timeout(
    binary: &Path,
    directory: &Path,
    args: &[&OsStr],
    timeout: Duration,
) -> Result<Observation> {
    let stdout = directory.join("stdout.log");
    let stderr = directory.join("stderr.log");
    let mut child = Command::new(binary)
        .args(args)
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(File::create(&stdout)?)
        .stderr(File::create(&stderr)?)
        .env("NO_COLOR", "1")
        .spawn()?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() >= timeout
            || fs::metadata(&stdout)?.len() + fs::metadata(&stderr)?.len() > LOG_LIMIT
        {
            child.kill()?;
            child.wait()?;
            return Err("Godot exceeded the 15 second / 1 MiB output limit".into());
        }
        thread::sleep(Duration::from_millis(10));
    };
    if fs::metadata(&stdout)?.len() + fs::metadata(&stderr)?.len() > LOG_LIMIT {
        return Err("Godot exceeded the 1 MiB output limit".into());
    }
    Ok(Observation {
        exit_code: status.code(),
        stdout: fs::read_to_string(stdout)?,
        stderr: fs::read_to_string(stderr)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_exit_script_errors_are_rejections_but_crashes_are_not() {
        let mut result = Observation {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: "SCRIPT ERROR: Parse Error: Expected parameter name.\nat: GDScript::reload (res://case.gd:1)\nERROR: Failed to load script".into(),
        };
        assert!(result.healthy() && result.rejected());
        result.exit_code = None;
        assert!(!result.healthy());
        result.exit_code = Some(2);
        assert!(!result.healthy());
        result.exit_code = Some(1);
        assert!(result.healthy());
        result.stderr = "ERROR: Cannot open project".into();
        assert!(!result.healthy() && !result.rejected());
        result.exit_code = Some(0);
        assert!(!result.healthy());
        result.stderr.clear();
        assert!(result.healthy() && !result.rejected());
    }

    #[test]
    fn colored_legacy_engine_errors_are_rejected() {
        let observation = Observation {
            exit_code: Some(1), stdout: String::new(),
            stderr: "\x1b[1;35mSCRIPT ERROR:\x1b[0;95m Parse Error: bad syntax\n\x1b[0;90m at: GDScript::reload (res://case.gd:1)\x1b[0m\n\x1b[1;31mERROR:\x1b[0;91m Failed to load script".into(),
        };
        assert!(observation.rejected() && observation.healthy());
    }

    #[test]
    fn matrix_requires_every_pin_expectation_and_gate() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let directory = root.join("tests/compatibility");
        let source = fs::read_to_string(directory.join("manifest.json")).unwrap();
        let corpus = serde_json::from_str(&source).unwrap();
        let baseline = serde_json::from_str(&source).unwrap();
        let load = || {
            serde_json::from_str::<EngineManifest>(
                &fs::read_to_string(directory.join("engine.json")).unwrap(),
            )
            .unwrap()
        };
        let mut engine = load();
        validate_manifest(&engine, &corpus, &baseline).unwrap();
        engine.engines[0].engine_version.push_str("-custom");
        assert!(validate_manifest(&engine, &corpus, &baseline).is_err());
        let mut engine = load();
        engine.engines.pop();
        assert!(validate_manifest(&engine, &corpus, &baseline).is_err());
        let mut engine = load();
        engine.cases[0]
            .expectations
            .remove(&LanguageVersion::Godot4_3);
        assert!(validate_manifest(&engine, &corpus, &baseline).is_err());
        let mut engine = load();
        engine.cases.pop();
        assert!(validate_manifest(&engine, &corpus, &baseline).is_err());
        let mut engine = load();
        engine.gates.pop();
        assert!(validate_manifest(&engine, &corpus, &baseline).is_err());
        let mut engine = load();
        let analyzer = engine
            .cases
            .iter_mut()
            .flat_map(|case| case.expectations.values_mut())
            .find(|expectation| expectation.kind == CaseKind::Analyzer)
            .unwrap();
        analyzer.note = None;
        assert!(validate_manifest(&engine, &corpus, &baseline).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn no_op_and_wrong_version_engines_cannot_pass_controls() {
        use std::os::unix::fs::PermissionsExt;
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let directory = tempfile::tempdir().unwrap();
        let binary = directory.path().join("fake-godot");
        fs::write(&binary, "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 4.7.2.stable.official.ed1daf0bf; fi\nexit 0\n").unwrap();
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
        let args = vec!["--godot".into(), binary.to_str().unwrap().into()];
        let error = run(root, &args).unwrap_err().to_string();
        assert!(error.contains("control failed"), "{error}");
        let mut wrong_profile = args.clone();
        wrong_profile.extend(["--language".into(), "4.3".into()]);
        let error = run(root, &wrong_profile).unwrap_err().to_string();
        assert!(error.contains("requested language profile"), "{error}");
        fs::write(&binary, "#!/bin/sh\necho 4.6.stable.official.wrong\n").unwrap();
        let error = run(root, &args).unwrap_err().to_string();
        assert!(error.contains("version mismatch"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn engine_timeouts_and_large_logs_fail_without_pipe_deadlocks() {
        let directory = tempfile::tempdir().unwrap();
        let error = invoke_with_timeout(
            Path::new("/bin/sh"),
            directory.path(),
            &[OsStr::new("-c"), OsStr::new("exec sleep 5")],
            Duration::from_millis(30),
        )
        .err()
        .unwrap()
        .to_string();
        assert!(error.contains("limit"));
        let error = invoke(
            Path::new("/bin/sh"),
            directory.path(),
            &[OsStr::new("-c"), OsStr::new("head -c 1100000 /dev/zero")],
        )
        .err()
        .unwrap()
        .to_string();
        assert!(error.contains("output limit"));
    }
}
