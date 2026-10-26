//! Reusable CLI runner. Only the outer binary chooses when to exit the process.

pub mod lsp;

use clap::{Parser, Subcommand, ValueEnum};
use gdfmt_config::{PartialConfig, schema_json};
use gdfmt_parser::{LexOptions, LexResult, lex};
use gdfmt_syntax::SyntaxKind;
use gdfmt_text::{Diagnostic, DiagnosticCategory, Severity, SourceId, Span};
use gdfmt_workspace::{
    CONFIG_FILENAME, CONFIG_FILENAMES, ConfigResolution, LexedFile, WorkspaceError,
    discover_files_with_config, format_request, lex_file, read_source, resolve_config,
    write_source,
};
use serde::Serialize;
use std::{
    ffi::OsString,
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum Reporter {
    #[default]
    Human,
    Json,
}

#[derive(Debug, Parser)]
#[command(
    name = "gdfmt",
    bin_name = "gdfmt",
    version,
    about = "Experimental Rust GDScript syntax checks and script formatting",
    after_help = "Formats GDScript with paired formatter-off/on regions, reparse, structure and idempotence validation. Lint rules and semantic fixes are outside the project scope."
)]
struct Args {
    /// Run the formatting language server over stdio.
    #[arg(long, conflicts_with_all = ["paths", "check", "diff", "stdin_filepath"])]
    lsp: bool,
    #[arg(long, global = true, value_enum, default_value = "human")]
    reporter: Reporter,
    #[arg(long, global = true, value_name = "FILE")]
    config: Option<PathBuf>,
    #[arg(
        long,
        global = true,
        conflicts_with = "diff",
        help = "Check syntax and script formatting without writing"
    )]
    check: bool,
    #[arg(
        long,
        global = true,
        help = "Print formatting differences without writing"
    )]
    diff: bool,
    #[arg(
        long,
        global = true,
        value_name = "PATH",
        help = "Read stdin and resolve configuration using this virtual .gd path"
    )]
    stdin_filepath: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(value_name = "PATH")]
    paths: Vec<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Inspect tokens and lexical diagnostics without modifying files.
    Lex {
        #[arg(
            value_name = "PATH",
            help = "Files/directories, or - for stdin; defaults to current directory"
        )]
        paths: Vec<PathBuf>,
    },
    /// Check syntax and script formatting without modifying files.
    Check {
        paths: Vec<PathBuf>,
        #[arg(long, conflicts_with = "diff")]
        syntax_only: bool,
    },
    /// Format scripts, including comments, lambdas, declarations and suites.
    Format { paths: Vec<PathBuf> },
    /// Print the JSON Schema generated from the Rust configuration model.
    Schema,
    /// Create .gdfmtrc.json unless a JSON/JSONC configuration already exists.
    Init,
    /// Print resolved configuration and the source of each field.
    ExplainConfig {
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

impl Args {
    fn validate(&self) -> Result<(), &'static str> {
        if self.lsp {
            if self.command.is_some() {
                return Err("--lsp cannot be combined with a subcommand");
            }
            if matches!(self.reporter, Reporter::Json) {
                return Err(
                    "--reporter=json cannot be combined with --lsp; LSP uses framed JSON-RPC",
                );
            }
        }
        let formatting = matches!(
            self.command,
            None | Some(Command::Format { .. } | Command::Check { .. })
        );
        if !formatting && (self.check || self.diff || self.stdin_filepath.is_some()) {
            return Err("--check, --diff and --stdin-filepath require a formatting/check command");
        }
        if self.config.is_some() && matches!(self.command, Some(Command::Init | Command::Schema)) {
            return Err("--config cannot be combined with init or schema");
        }
        let paths = match &self.command {
            Some(
                Command::Lex { paths } | Command::Format { paths } | Command::Check { paths, .. },
            ) => paths,
            None => &self.paths,
            _ => return Ok(()),
        };
        let from_stdin =
            self.stdin_filepath.is_some() || paths.iter().any(|path| path == Path::new("-"));
        if from_stdin && !paths.is_empty() && (paths.len() != 1 || paths[0] != Path::new("-")) {
            return Err("stdin cannot be combined with file paths");
        }
        if self
            .stdin_filepath
            .as_ref()
            .is_some_and(|path| path.extension().is_none_or(|extension| extension != "gd"))
        {
            return Err("stdin filepath must have a .gd extension");
        }
        Ok(())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TokenReport {
    kind: SyntaxKind,
    span: Span,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileReport {
    path: PathBuf,
    source_id: SourceId,
    tokens: Vec<TokenReport>,
    diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    changed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HostDiagnostic {
    code: String,
    category: DiagnosticCategory,
    severity: Severity,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    schema_version: u32,
    command: String,
    status: &'static str,
    span_encoding: &'static str,
    files: Vec<FileReport>,
    diagnostics: Vec<HostDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    configuration: Option<ConfigResolution>,
}

impl Report {
    fn new(command: &str) -> Self {
        Self {
            schema_version: 1,
            command: command.into(),
            status: "ok",
            span_encoding: "utf8Bytes",
            files: vec![],
            diagnostics: vec![],
            configuration: None,
        }
    }

    fn error(command: &str, code: &str, category: DiagnosticCategory, message: String) -> Self {
        let mut report = Self::new(command);
        report.status = "error";
        report.diagnostics.push(HostDiagnostic {
            code: code.into(),
            category,
            severity: Severity::Error,
            message,
        });
        report
    }
}

/// Returns 0 on success, 1 for source/format findings, 2 for host/internal failures.
pub fn run<I, T>(
    arguments: I,
    cwd: &Path,
    stdin: &mut (dyn Read + Send),
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let arguments: Vec<OsString> = arguments.into_iter().map(Into::into).collect();
    let option_arguments: Vec<_> = arguments
        .iter()
        .skip(1)
        .take_while(|arg| *arg != "--")
        .collect();
    let requested_json = option_arguments.iter().any(|arg| *arg == "--reporter=json")
        || option_arguments
            .windows(2)
            .any(|args| args[0] == "--reporter" && args[1] == "json");
    let args = match Args::try_parse_from(&arguments) {
        Ok(args) => args,
        Err(error) => {
            let code = error.exit_code();
            let written = if code == 0 {
                write!(stdout, "{error}")
            } else if requested_json {
                write_json(
                    stdout,
                    &Report::error(
                        "arguments",
                        "CLI_ARGUMENT",
                        DiagnosticCategory::Configuration,
                        error.to_string(),
                    ),
                )
            } else {
                write!(stderr, "{error}")
            };
            return if written.is_ok() { code } else { 2 };
        }
    };
    if let Err(message) = args.validate() {
        let report = Report::error(
            "arguments",
            "CLI_ARGUMENT",
            DiagnosticCategory::Configuration,
            message.into(),
        );
        let _ = match args.reporter {
            Reporter::Json => write_json(stdout, &report),
            Reporter::Human => writeln!(stderr, "{message}"),
        };
        return 2;
    }
    if args.lsp {
        return match lsp::run(stdin, stdout, cwd, args.config) {
            Ok(code) => code,
            Err(error) => {
                let _ = writeln!(stderr, "gdfmt LSP: {error}");
                2
            }
        };
    }
    let reporter = args.reporter;
    match execute(args, cwd, stdin, stdout, stderr) {
        Ok(code) => code,
        Err(error) => {
            let (code, category) = match &error {
                WorkspaceError::Configuration { .. } => {
                    ("CONFIG_INVALID", DiagnosticCategory::Configuration)
                }
                _ => ("CLI_IO", DiagnosticCategory::Io),
            };
            let report = Report::error("execution", code, category, error.to_string());
            let _ = match reporter {
                Reporter::Json => write_json(stdout, &report),
                Reporter::Human => writeln!(stderr, "{code}: {error}"),
            };
            2
        }
    }
}

fn execute(
    args: Args,
    cwd: &Path,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<i32, WorkspaceError> {
    let io_failure = |error| WorkspaceError::Input(format!("cannot write command output: {error}"));
    match args.command {
        Some(Command::Schema) => {
            stdout
                .write_all(schema_json().as_bytes())
                .map_err(io_failure)?;
            Ok(0)
        }
        Some(Command::Init) => {
            for filename in CONFIG_FILENAMES {
                let existing = cwd.join(filename);
                if existing.try_exists().map_err(|source| WorkspaceError::Io {
                    path: existing.clone(),
                    source,
                })? {
                    return Err(WorkspaceError::Input(format!(
                        "configuration already exists: {}",
                        existing.display()
                    )));
                }
            }
            let path = cwd.join(CONFIG_FILENAME);
            let initial = PartialConfig {
                schema_version: Some(1),
                root: Some(true),
                ..Default::default()
            };
            let json =
                serde_json::to_string_pretty(&initial).expect("config is serializable") + "\n";
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|source| WorkspaceError::Io {
                    path: path.clone(),
                    source,
                })?;
            file.write_all(json.as_bytes())
                .map_err(|source| WorkspaceError::Io {
                    path: path.clone(),
                    source,
                })?;
            if matches!(args.reporter, Reporter::Json) {
                write_json(stdout, &Report::new("init")).map_err(io_failure)?;
            } else {
                writeln!(stdout, "Created {}", path.display()).map_err(io_failure)?;
            }
            Ok(0)
        }
        Some(Command::ExplainConfig { path }) => {
            let mut report = Report::new("explain-config");
            report.configuration = Some(resolve_config(&path, cwd, args.config.as_deref())?);
            // This command is always machine-readable, including provenance.
            write_json(stdout, &report).map_err(io_failure)?;
            Ok(0)
        }
        Some(Command::Lex { paths }) => {
            let mut report = Report::new("lex");
            if paths.iter().any(|path| path == Path::new("-")) {
                resolve_config(cwd, cwd, args.config.as_deref())?;
                let options = LexOptions::default();
                let mut bytes = Vec::new();
                stdin
                    .take(options.max_source_bytes as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|error| WorkspaceError::Input(format!("stdin: {error}")))?;
                if bytes.len() > options.max_source_bytes {
                    return Err(WorkspaceError::Input("stdin exceeds byte limit".into()));
                }
                let source = String::from_utf8(bytes).map_err(|error| {
                    WorkspaceError::Input(format!("stdin: invalid UTF-8: {error}"))
                })?;
                let result = lex(&source, &options);
                report.files.push(file_report(
                    PathBuf::from("<stdin>"),
                    &source,
                    result,
                    options.source_id,
                ));
            } else {
                let files = discover_files_with_config(&paths, cwd, args.config.as_deref())?;
                // Validate every configuration before processing any source files.
                for file in &files {
                    resolve_config(file, cwd, args.config.as_deref())?;
                }
                for (index, file) in files.iter().enumerate() {
                    let source_id = SourceId(
                        u32::try_from(index)
                            .map_err(|_| WorkspaceError::Input("too many input files".into()))?,
                    );
                    let options = LexOptions {
                        source_id,
                        ..Default::default()
                    };
                    let LexedFile {
                        path,
                        source,
                        result,
                    } = lex_file(file, &options)?;
                    report
                        .files
                        .push(file_report(path, &source, result, source_id));
                }
            }
            let failed = report.files.iter().any(|file| !file.diagnostics.is_empty());
            if failed {
                report.status = "error";
            }
            match args.reporter {
                Reporter::Json => write_json(stdout, &report).map_err(io_failure)?,
                Reporter::Human => {
                    for file in &report.files {
                        writeln!(stdout, "{}:", file.path.display()).map_err(io_failure)?;
                        for token in &file.tokens {
                            writeln!(
                                stdout,
                                "  {:?} {}..{} {:?}",
                                token.kind,
                                token.span.start(),
                                token.span.end(),
                                token.text
                            )
                            .map_err(io_failure)?;
                        }
                        for diagnostic in &file.diagnostics {
                            writeln!(
                                stderr,
                                "{}:{}..{} {}: {}",
                                file.path.display(),
                                diagnostic.span.start(),
                                diagnostic.span.end(),
                                diagnostic.code,
                                diagnostic.message
                            )
                            .map_err(io_failure)?;
                        }
                    }
                }
            }
            Ok(i32::from(failed))
        }
        Some(Command::Check {
            ref paths,
            syntax_only,
        }) => execute_format(
            &args,
            paths,
            if syntax_only {
                FormatMode::SyntaxOnly
            } else {
                FormatMode::Check
            },
            cwd,
            stdin,
            stdout,
            stderr,
        ),
        Some(Command::Format { ref paths }) => execute_format(
            &args,
            paths,
            if args.check {
                FormatMode::Check
            } else {
                FormatMode::Write
            },
            cwd,
            stdin,
            stdout,
            stderr,
        ),
        None => execute_format(
            &args,
            &args.paths,
            if args.check {
                FormatMode::Check
            } else {
                FormatMode::Write
            },
            cwd,
            stdin,
            stdout,
            stderr,
        ),
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FormatMode {
    Write,
    Check,
    SyntaxOnly,
}

struct FormatInput {
    path: PathBuf,
    source: String,
    config: gdfmt_config::ResolvedConfig,
}

fn format_worker_count(files: usize, largest_source: usize, available: usize) -> usize {
    // Bound aggregate working memory and avoid thread startup for editor-sized requests.
    if files < 8 || largest_source > 256 * 1024 {
        1
    } else {
        available.clamp(1, 4)
    }
}

fn format_input(
    args: &Args,
    mode: FormatMode,
    read_only: bool,
    from_stdin: bool,
    index: usize,
    input: FormatInput,
) -> (i32, FileReport) {
    let FormatInput {
        path,
        source,
        config,
    } = input;
    let mut code = 0;
    let source_id = SourceId(u32::try_from(index).expect("input count checked before formatting"));
    let request = format_request(&config, source_id);
    let result = if mode == FormatMode::SyntaxOnly {
        let parsed = gdfmt_parser::parse(&source, &request.parse);
        gdfmt::FormatResult {
            output: None,
            changed: false,
            diagnostics: parsed.diagnostics,
        }
    } else {
        gdfmt::format(&source, &request)
    };
    let mut file = FileReport {
        path: path.clone(),
        source_id,
        tokens: vec![],
        diagnostics: result.diagnostics,
        changed: (mode != FormatMode::SyntaxOnly).then_some(result.changed),
        output: None,
        diff: None,
    };
    let failed = file
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error);
    if failed {
        code = code.max(
            if file
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.category == DiagnosticCategory::Internal)
            {
                2
            } else {
                1
            },
        );
    } else if let Some(output) = result.output {
        if read_only && result.changed {
            code = code.max(1);
        }
        if args.diff && result.changed {
            file.diff = Some(unified_diff(&path, &source, &output));
        }
        if from_stdin && !read_only {
            file.output = Some(output);
        } else if !read_only
            && result.changed
            && let Err(error) = write_source(&path, &source, &output)
        {
            code = 2;
            file.diagnostics.push(Diagnostic {
                code: "CLI_IO".into(),
                category: DiagnosticCategory::Io,
                severity: Severity::Error,
                message: error.to_string(),
                source_id,
                span: Span::empty(0),
            });
        }
    }
    (code, file)
}

fn format_inputs(
    args: &Args,
    mode: FormatMode,
    read_only: bool,
    from_stdin: bool,
    inputs: Vec<FormatInput>,
    workers: usize,
) -> Result<Vec<(i32, FileReport)>, WorkspaceError> {
    u32::try_from(inputs.len().saturating_sub(1))
        .map_err(|_| WorkspaceError::Input("too many input files".into()))?;
    if workers == 1 {
        return Ok(inputs
            .into_iter()
            .enumerate()
            .map(|(index, input)| format_input(args, mode, read_only, from_stdin, index, input))
            .collect());
    }
    let jobs = std::sync::Mutex::new(inputs.into_iter().enumerate());
    let (sender, receiver) = std::sync::mpsc::channel();
    {
        let work = || loop {
            // Release the queue lock before any parsing, validation or I/O.
            let job = jobs.lock().expect("format queue lock").next();
            let Some((index, input)) = job else { break };
            let report = format_input(args, mode, read_only, from_stdin, index, input);
            sender
                .send((index, report))
                .expect("format receiver remains alive");
        };
        std::thread::scope(|scope| {
            for _ in 1..workers {
                if std::thread::Builder::new()
                    .spawn_scoped(scope, work)
                    .is_err()
                {
                    // The main worker drains remaining jobs if thread creation fails.
                    break;
                }
            }
            work();
        });
    }
    drop(sender);
    let mut reports: Vec<_> = receiver.into_iter().collect();
    reports.sort_unstable_by_key(|(index, _)| *index);
    Ok(reports.into_iter().map(|(_, report)| report).collect())
}

fn execute_format(
    args: &Args,
    paths: &[PathBuf],
    mode: FormatMode,
    cwd: &Path,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Result<i32, WorkspaceError> {
    let from_stdin =
        args.stdin_filepath.is_some() || paths.iter().any(|path| path == Path::new("-"));
    let read_only = mode != FormatMode::Write || args.diff;
    let mut report = Report::new(if args.diff {
        "diff"
    } else if read_only {
        "check"
    } else {
        "format"
    });
    let mut inputs = Vec::new();
    if from_stdin {
        let path = args.stdin_filepath.as_ref().map(|path| {
            if path.is_absolute() {
                path.clone()
            } else {
                cwd.join(path)
            }
        });
        let virtual_path = path.clone().unwrap_or_else(|| cwd.join("stdin.gd"));
        let resolution = resolve_config(&virtual_path, cwd, args.config.as_deref())?;
        if resolution.ignored {
            return Err(WorkspaceError::Input(format!(
                "{}: stdin filepath is ignored",
                virtual_path.display()
            )));
        }
        let config = resolution.config;
        let mut bytes = Vec::new();
        let limit = LexOptions::default().max_source_bytes;
        stdin
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| WorkspaceError::Input(format!("stdin: {error}")))?;
        if bytes.len() > limit {
            return Err(WorkspaceError::Input("stdin exceeds byte limit".into()));
        }
        let source = String::from_utf8(bytes)
            .map_err(|error| WorkspaceError::Input(format!("stdin: invalid UTF-8: {error}")))?;
        inputs.push(FormatInput {
            path: path.unwrap_or_else(|| PathBuf::from("<stdin>")),
            source,
            config,
        });
    } else {
        let files = discover_files_with_config(paths, cwd, args.config.as_deref())?;
        // Preflight all configurations and source reads before any replacement.
        let configs = files
            .iter()
            .map(|path| {
                resolve_config(path, cwd, args.config.as_deref())
                    .map(|resolution| resolution.config)
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (path, config) in files.into_iter().zip(configs) {
            let source = read_source(&path)?;
            inputs.push(FormatInput {
                path,
                source,
                config,
            });
        }
    }
    let available = if inputs.len() >= 8 {
        std::thread::available_parallelism().map_or(1, |count| count.get())
    } else {
        1
    };
    let largest_source = inputs
        .iter()
        .map(|input| input.source.len())
        .max()
        .unwrap_or(0);
    let workers = format_worker_count(inputs.len(), largest_source, available);
    let files = format_inputs(args, mode, read_only, from_stdin, inputs, workers)?;
    let code = files.iter().map(|(code, _)| *code).max().unwrap_or(0);
    report.files = files.into_iter().map(|(_, file)| file).collect();
    if code != 0 {
        report.status = "error";
    }
    let io_failure = |error| WorkspaceError::Input(format!("cannot write command output: {error}"));
    match args.reporter {
        Reporter::Json => write_json(stdout, &report).map_err(io_failure)?,
        Reporter::Human => {
            for file in &report.files {
                if let Some(diff) = &file.diff {
                    stdout.write_all(diff.as_bytes()).map_err(io_failure)?;
                } else if let Some(output) = &file.output {
                    stdout.write_all(output.as_bytes()).map_err(io_failure)?;
                } else if !from_stdin && file.diagnostics.is_empty() && file.changed == Some(true) {
                    writeln!(
                        stdout,
                        "{}: {}",
                        file.path.display(),
                        if read_only {
                            "formatting differs"
                        } else {
                            "formatted"
                        }
                    )
                    .map_err(io_failure)?;
                }
                for diagnostic in &file.diagnostics {
                    writeln!(
                        stderr,
                        "{}:{}..{} {}: {}",
                        file.path.display(),
                        diagnostic.span.start(),
                        diagnostic.span.end(),
                        diagnostic.code,
                        diagnostic.message
                    )
                    .map_err(io_failure)?;
                }
            }
            for diagnostic in &report.diagnostics {
                writeln!(stderr, "{}: {}", diagnostic.code, diagnostic.message)
                    .map_err(io_failure)?;
            }
        }
    }
    Ok(code)
}

fn unified_diff(path: &Path, source: &str, output: &str) -> String {
    let before: Vec<_> = source.split_inclusive('\n').collect();
    let after: Vec<_> = output.split_inclusive('\n').collect();
    let mut diff = format!(
        "--- {}\n+++ {}\n@@ -{},{} +{},{} @@\n",
        path.display(),
        path.display(),
        usize::from(!before.is_empty()),
        before.len(),
        usize::from(!after.is_empty()),
        after.len()
    );
    for (prefix, lines) in [('-', before), ('+', after)] {
        for line in lines {
            diff.push(prefix);
            diff.push_str(line);
            if !line.ends_with('\n') {
                diff.push_str("\n\\ No newline at end of file\n");
            }
        }
    }
    diff
}

fn file_report(path: PathBuf, source: &str, result: LexResult, source_id: SourceId) -> FileReport {
    FileReport {
        path,
        source_id,
        tokens: result
            .tokens
            .into_iter()
            .map(|token| TokenReport {
                kind: token.kind,
                span: token.span,
                text: token
                    .span
                    .slice(source)
                    .expect("lexer span is valid")
                    .into(),
            })
            .collect(),
        diagnostics: result.diagnostics,
        changed: None,
        output: None,
        diff: None,
    }
}

fn write_json(writer: &mut dyn Write, report: &Report) -> std::io::Result<()> {
    serde_json::to_writer_pretty(&mut *writer, report)?;
    writeln!(writer)
}

#[cfg(test)]
mod format_worker_tests {
    use super::*;

    #[test]
    fn small_requests_and_large_sources_remain_serial_and_workers_are_bounded() {
        assert_eq!(format_worker_count(1, 100, 64), 1);
        assert_eq!(format_worker_count(7, 100, 64), 1);
        assert_eq!(format_worker_count(8, 256 * 1024, 64), 4);
        assert_eq!(format_worker_count(8, 256 * 1024 + 1, 64), 1);
        assert_eq!(format_worker_count(100, 100, 2), 2);
        assert_eq!(format_worker_count(100, 100, 1), 1);
    }

    #[test]
    fn worker_results_keep_input_order_source_ids_and_read_only_behavior() {
        let args = Args::try_parse_from(["gdfmt"]).unwrap();
        let inputs = (0..8)
            .map(|index| FormatInput {
                path: PathBuf::from(format!("case_{index}.gd")),
                source: if index == 0 {
                    (0..1000)
                        .map(|number| format!("var value_{number}= {number}\n"))
                        .collect()
                } else {
                    format!("var value_{index}= {index}\n")
                },
                config: PartialConfig::default().resolve().unwrap(),
            })
            .collect();
        let reports = format_inputs(&args, FormatMode::Check, true, false, inputs, 4).unwrap();
        assert_eq!(reports.len(), 8);
        for (index, (code, file)) in reports.iter().enumerate() {
            assert_eq!(*code, 1);
            assert_eq!(file.path, PathBuf::from(format!("case_{index}.gd")));
            assert_eq!(file.source_id.0 as usize, index);
            assert_eq!(file.changed, Some(true));
            assert!(file.diagnostics.is_empty());
            assert!(file.output.is_none());
        }
    }
}
