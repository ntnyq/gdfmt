//! Formatting-only LSP. The reader and formatter run independently so changes
//! and cancellation invalidate in-flight work before it can produce edits.
use crossbeam_channel::{Receiver, Sender, bounded, select_biased, unbounded};
use gdfmt_text::{LineIndex, Severity, SourceId};
use lsp_server::{Message, Notification, Request, RequestId, Response};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    io::{self, BufReader, Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};
use url::Url;

const INVALID_PARAMS: i32 = -32602;
const CONTENT_MODIFIED: i32 = -32801;

#[derive(Clone)]
struct Document {
    text: String,
    version: i64,
}

struct Pending {
    uri: String,
    cancelled: Arc<AtomicBool>,
    text_result: bool,
}

struct Job {
    cancelled: Arc<AtomicBool>,
    id: RequestId,
    uri: String,
    document: Document,
    path: PathBuf,
    cwd: PathBuf,
    config: Option<PathBuf>,
}

struct Completed {
    id: RequestId,
    uri: String,
    version: i64,
    result: Result<FormattedDocument, String>,
}

struct FormattedDocument {
    edits: Value,
    output: Option<String>,
    line_ending: gdfmt_config::LineEnding,
    diagnostics: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Identifier {
    uri: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionedIdentifier {
    uri: String,
    version: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenParams {
    text_document: OpenDocument,
}

#[derive(Deserialize)]
struct OpenDocument {
    uri: String,
    version: i64,
    text: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChangeParams {
    text_document: VersionedIdentifier,
    content_changes: Vec<Change>,
}

#[derive(Deserialize)]
struct Change {
    text: String,
    range: Option<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentParams {
    text_document: Identifier,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Settings {
    config_path: Option<PathBuf>,
}

struct Server<'a> {
    output: &'a mut dyn Write,
    cwd: &'a Path,
    roots: Vec<PathBuf>,
    config: Option<PathBuf>,
    config_error: Option<String>,
    initialized: bool,
    shutdown: bool,
    documents: HashMap<String, Document>,
    pending: HashMap<RequestId, Pending>,
    jobs: Sender<Job>,
}

/// The input thread stops at `exit`, so clients need not close stdin to exit.
pub fn run(
    input: &mut (dyn Read + Send),
    output: &mut dyn Write,
    cwd: &Path,
    config: Option<PathBuf>,
) -> io::Result<i32> {
    thread::scope(|scope| {
        let (messages_tx, messages_rx) = unbounded();
        scope.spawn(move || {
            let mut input = BufReader::new(input);
            loop {
                let message = Message::read(&mut input);
                let stop = !matches!(&message, Ok(Some(message)) if !matches!(message,
                    Message::Notification(notification) if notification.method == "exit"));
                if messages_tx.send(message).is_err() || stop {
                    break;
                }
            }
        });
        let (jobs_tx, jobs_rx) = bounded::<Job>(64);
        let (done_tx, done_rx) = unbounded();
        scope.spawn(move || {
            for job in jobs_rx {
                if job.cancelled.load(Ordering::Relaxed) {
                    continue;
                }
                let result = format_job(&job);
                if done_tx
                    .send(Completed {
                        id: job.id,
                        uri: job.uri,
                        version: job.document.version,
                        result,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Server {
            output,
            cwd,
            roots: vec![],
            config,
            config_error: None,
            initialized: false,
            shutdown: false,
            documents: HashMap::new(),
            pending: HashMap::new(),
            jobs: jobs_tx,
        }
        .serve(messages_rx, done_rx)
    })
}

impl Server<'_> {
    fn send(&mut self, message: impl Into<Message>) -> io::Result<()> {
        message.into().write(&mut self.output)
    }

    fn error(&mut self, id: RequestId, code: i32, message: impl Into<String>) -> io::Result<()> {
        self.send(Response::new_err(id, code, message.into()))
    }

    fn log(&mut self, message: impl ToString) -> io::Result<()> {
        self.send(Notification::new(
            "window/logMessage".into(),
            json!({"type": 1, "message": message.to_string()}),
        ))
    }

    fn invalidate(&mut self, uri: Option<&str>) -> io::Result<()> {
        let ids: Vec<_> = self
            .pending
            .iter()
            .filter(|(_, pending_uri)| uri.is_none_or(|uri| uri == pending_uri.uri.as_str()))
            .map(|(id, _)| id.clone())
            .collect();
        for id in ids {
            if let Some(pending) = self.pending.remove(&id) {
                pending.cancelled.store(true, Ordering::Relaxed);
            }
            self.error(id, CONTENT_MODIFIED, "document or configuration changed")?;
        }
        Ok(())
    }

    fn serve(
        mut self,
        messages: Receiver<io::Result<Option<Message>>>,
        done: Receiver<Completed>,
    ) -> io::Result<i32> {
        loop {
            // Drain document changes/cancellation before applying a finished job.
            let message = match messages.try_recv() {
                Ok(message) => Some(message),
                Err(_) => {
                    select_biased! {
                        recv(messages) -> message => Some(message.unwrap_or(Ok(None))),
                        recv(done) -> completed => {
                            if let Ok(completed) = completed { self.complete(completed)?; }
                            None
                        }
                    }
                }
            };
            if let Some(message) = message {
                match message? {
                    None => return Ok(if self.shutdown { 0 } else { 1 }),
                    Some(Message::Request(request)) => self.request(request)?,
                    Some(Message::Notification(notification)) if notification.method == "exit" => {
                        return Ok(if self.shutdown { 0 } else { 1 });
                    }
                    Some(Message::Notification(notification))
                        if self.initialized && !self.shutdown =>
                    {
                        if let Err(error) = self.notification(notification) {
                            self.log(error)?;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn request(&mut self, request: Request) -> io::Result<()> {
        if self.shutdown {
            return self.error(request.id, -32600, "server has shut down");
        }
        if request.method == "initialize" && !self.initialized {
            return self.initialize(request);
        }
        if !self.initialized {
            return self.error(request.id, -32002, "server is not initialized");
        }
        match request.method.as_str() {
            "shutdown" => {
                self.shutdown = true;
                self.invalidate(None)?;
                self.send(Response::new_ok(request.id, Value::Null))
            }
            "textDocument/formatting" | "gdfmt/formatting" => {
                let text_result = request.method == "gdfmt/formatting";
                let params: DocumentParams = match serde_json::from_value(request.params) {
                    Ok(params) => params,
                    Err(error) => return self.error(request.id, INVALID_PARAMS, error.to_string()),
                };
                let uri = params.text_document.uri;
                let Some(document) = self.documents.get(&uri).cloned() else {
                    return self.error(request.id, INVALID_PARAMS, "document is not open");
                };
                if let Some(error) = self.config_error.clone() {
                    return self.error(request.id, INVALID_PARAMS, error);
                }
                let path = match document_path(&uri, self.cwd) {
                    Ok(path) => path,
                    Err(error) => return self.error(request.id, INVALID_PARAMS, error),
                };
                let cwd = self
                    .roots
                    .iter()
                    .filter(|root| path.starts_with(root))
                    .max_by_key(|root| root.components().count())
                    .cloned()
                    .unwrap_or_else(|| self.cwd.to_path_buf());
                if self.pending.len() >= 64 {
                    return self.error(request.id, -32802, "formatter queue is full; retry later");
                }
                let cancelled = Arc::new(AtomicBool::new(false));
                let id = request.id;
                self.pending.insert(
                    id.clone(),
                    Pending {
                        uri: uri.clone(),
                        cancelled: Arc::clone(&cancelled),
                        text_result,
                    },
                );
                if self
                    .jobs
                    .try_send(Job {
                        id: id.clone(),
                        uri,
                        document,
                        path,
                        cwd,
                        config: self.config.clone(),
                        cancelled,
                    })
                    .is_err()
                {
                    self.pending.remove(&id);
                    return self.error(id, -32802, "formatter queue is full; retry later");
                }
                Ok(())
            }
            _ => self.error(request.id, -32601, "method not supported"),
        }
    }

    fn initialize(&mut self, request: Request) -> io::Result<()> {
        let settings: Settings = match request.params.get("initializationOptions") {
            Some(value) if !value.is_null() => match serde_json::from_value(value.clone()) {
                Ok(settings) => settings,
                Err(error) => return self.error(request.id, INVALID_PARAMS, error.to_string()),
            },
            _ => Settings::default(),
        };
        if settings.config_path.is_some() {
            self.config = settings
                .config_path
                .filter(|path| !path.as_os_str().is_empty());
        }
        if let Some(folders) = request
            .params
            .get("workspaceFolders")
            .and_then(Value::as_array)
        {
            for folder in folders {
                if let Some(uri) = folder.get("uri").and_then(Value::as_str)
                    && let Ok(path) = file_path(uri)
                {
                    self.roots.push(path);
                }
            }
        }
        if self.roots.is_empty()
            && let Some(uri) = request.params.get("rootUri").and_then(Value::as_str)
            && let Ok(path) = file_path(uri)
        {
            self.roots.push(path);
        }
        self.initialized = true;
        self.send(Response::new_ok(request.id, json!({
            "capabilities": {
                "positionEncoding": "utf-16",
                "textDocumentSync": {"openClose": true, "change": 1},
                "documentFormattingProvider": true,
                "experimental": {"gdfmtTextFormattingProvider": true},
                "workspace": {"workspaceFolders": {"supported": true, "changeNotifications": true}}
            },
            "serverInfo": {"name": "gdfmt", "version": env!("CARGO_PKG_VERSION")}
        })))
    }

    fn notification(&mut self, notification: Notification) -> Result<(), String> {
        let invalid = |error: serde_json::Error| error.to_string();
        let io_error = |error: io::Error| error.to_string();
        match notification.method.as_str() {
            "textDocument/didOpen" => {
                let params: OpenParams =
                    serde_json::from_value(notification.params).map_err(invalid)?;
                let document = params.text_document;
                self.invalidate(Some(&document.uri)).map_err(io_error)?;
                self.documents.insert(
                    document.uri,
                    Document {
                        text: document.text,
                        version: document.version,
                    },
                );
            }
            "textDocument/didChange" => {
                let params: ChangeParams =
                    serde_json::from_value(notification.params).map_err(invalid)?;
                let uri = params.text_document.uri;
                let Some(current) = self.documents.get(&uri) else {
                    return Err("document is not open".into());
                };
                if params.text_document.version <= current.version {
                    return Ok(());
                }
                let text = params
                    .content_changes
                    .last()
                    .map_or_else(|| current.text.clone(), |change| change.text.clone());
                self.invalidate(Some(&uri)).map_err(io_error)?;
                if params
                    .content_changes
                    .iter()
                    .any(|change| change.range.is_some())
                {
                    self.documents.remove(&uri);
                    return Err(
                        "full document synchronization is required; reopen the document".into(),
                    );
                }
                self.documents.insert(
                    uri,
                    Document {
                        text,
                        version: params.text_document.version,
                    },
                );
            }
            "textDocument/didClose" => {
                let params: DocumentParams =
                    serde_json::from_value(notification.params).map_err(invalid)?;
                let uri = params.text_document.uri;
                self.invalidate(Some(&uri)).map_err(io_error)?;
                self.documents.remove(&uri);
                self.send(Notification::new(
                    "textDocument/publishDiagnostics".into(),
                    json!({"uri": uri, "diagnostics": []}),
                ))
                .map_err(io_error)?;
            }
            "$/cancelRequest" => {
                let id: RequestId =
                    serde_json::from_value(notification.params["id"].clone()).map_err(invalid)?;
                if let Some(pending) = self.pending.remove(&id) {
                    pending.cancelled.store(true, Ordering::Relaxed);
                    self.error(id, -32800, "request cancelled")
                        .map_err(io_error)?;
                }
            }
            "workspace/didChangeConfiguration" => {
                self.invalidate(None).map_err(io_error)?;
                let value = &notification.params["settings"]["gdfmt"];
                if !value.is_null() {
                    match serde_json::from_value::<Settings>(value.clone()) {
                        Ok(settings) => {
                            self.config = settings
                                .config_path
                                .filter(|path| !path.as_os_str().is_empty());
                            self.config_error = None;
                        }
                        Err(error) => {
                            self.config_error = Some(error.to_string());
                            return Err(error.to_string());
                        }
                    }
                }
            }
            "workspace/didChangeWatchedFiles" => self.invalidate(None).map_err(io_error)?,
            "workspace/didChangeWorkspaceFolders" => {
                self.invalidate(None).map_err(io_error)?;
                if let Some(removed) = notification.params["event"]["removed"].as_array() {
                    for folder in removed {
                        if let Some(uri) = folder["uri"].as_str()
                            && let Ok(path) = file_path(uri)
                        {
                            self.roots.retain(|root| root != &path);
                        }
                    }
                }
                if let Some(added) = notification.params["event"]["added"].as_array() {
                    for folder in added {
                        if let Some(uri) = folder["uri"].as_str()
                            && let Ok(path) = file_path(uri)
                        {
                            self.roots.push(path);
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn complete(&mut self, completed: Completed) -> io::Result<()> {
        let Some(pending) = self.pending.remove(&completed.id) else {
            return Ok(());
        };
        if !self
            .documents
            .get(&completed.uri)
            .is_some_and(|document| document.version == completed.version)
        {
            return self.error(completed.id, CONTENT_MODIFIED, "document changed");
        }
        match completed.result {
            Ok(formatted) => {
                self.send(Notification::new("textDocument/publishDiagnostics".into(), json!({
                    "uri": completed.uri, "version": completed.version, "diagnostics": formatted.diagnostics,
                })))?;
                let result = if pending.text_result {
                    json!({"output": formatted.output, "lineEnding": formatted.line_ending})
                } else {
                    formatted.edits
                };
                self.send(Response::new_ok(completed.id, result))
            }
            Err(error) => {
                self.log(&error)?;
                self.error(completed.id, -32603, error)
            }
        }
    }
}

fn file_path(uri: &str) -> Result<PathBuf, String> {
    Url::parse(uri)
        .map_err(|error| error.to_string())?
        .to_file_path()
        .map_err(|_| "expected a local file URI".into())
}

fn document_path(uri: &str, cwd: &Path) -> Result<PathBuf, String> {
    if Url::parse(uri).is_ok_and(|uri| uri.scheme() == "untitled") {
        Ok(cwd.join("untitled.gd"))
    } else {
        file_path(uri)
    }
}

fn position(index: &LineIndex, offset: usize) -> Value {
    let position = index.utf16_position(offset as u32).expect("UTF-8 boundary");
    json!({"line": position.line, "character": position.column})
}

/// A single minimal replacement, expanded when necessary to retain CRLF pairs.
fn edits(source: &str, output: &str) -> Value {
    if source == output {
        return json!([]);
    }
    let mut start = source
        .chars()
        .zip(output.chars())
        .take_while(|(a, b)| a == b)
        .map(|(character, _)| character.len_utf8())
        .sum::<usize>();
    if start > 0
        && source.as_bytes().get(start - 1) == Some(&b'\r')
        && source.as_bytes().get(start) == Some(&b'\n')
    {
        start -= 1;
    }
    let suffix = source[start..]
        .chars()
        .rev()
        .zip(output[start..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(character, _)| character.len_utf8())
        .sum::<usize>();
    let mut end = source.len() - suffix;
    let mut output_end = output.len() - suffix;
    if end > 0
        && source.as_bytes().get(end - 1) == Some(&b'\r')
        && source.as_bytes().get(end) == Some(&b'\n')
    {
        end += 1;
        output_end += 1;
    }
    let index = LineIndex::new(source).expect("formatter source limit");
    json!([{"range": {"start": position(&index, start), "end": position(&index, end)}, "newText": &output[start..output_end]}])
}

fn format_job(job: &Job) -> Result<FormattedDocument, String> {
    let config = if Url::parse(&job.uri).is_ok_and(|uri| uri.scheme() == "untitled") {
        gdfmt_config::PartialConfig::default()
            .resolve()
            .map_err(|error| error.to_string())?
    } else {
        let resolution =
            gdfmt_workspace::resolve_config(&job.path, &job.cwd, job.config.as_deref())
                .map_err(|error| error.to_string())?;
        if resolution.ignored {
            return Ok(FormattedDocument {
                edits: json!([]),
                output: None,
                line_ending: gdfmt_config::LineEnding::Preserve,
                diagnostics: json!([]),
            });
        }
        resolution.config
    };
    let result = gdfmt::format(
        &job.document.text,
        &gdfmt_config::format_request(&config, SourceId(0)),
    );
    let index = LineIndex::new(&job.document.text).map_err(|error| error.to_string())?;
    let diagnostics: Vec<_> = result.diagnostics.iter().map(|diagnostic| json!({
        "range": {"start": position(&index, diagnostic.span.start() as usize), "end": position(&index, diagnostic.span.end() as usize)},
        "severity": match diagnostic.severity { Severity::Error => 1, Severity::Warning => 2, Severity::Info => 3 },
        "code": diagnostic.code, "source": "gdfmt", "message": diagnostic.message,
    })).collect();
    Ok(FormattedDocument {
        edits: result
            .output
            .as_ref()
            .map_or_else(|| json!([]), |output| edits(&job.document.text, output)),
        output: result.output,
        line_ending: config.format.line_ending,
        diagnostics: json!(diagnostics),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_preserve_unicode_and_newline_boundaries() {
        assert_eq!(
            edits("var 名称=\"😀\"\r\n", "var 名称 = \"😀\"\r\n"),
            json!([{
                "range": {"start": {"line": 0, "character": 6}, "end": {"line": 0, "character": 7}},
                "newText": " = "
            }])
        );
        assert_eq!(
            edits("# 😀\r\n", "# 😀\n"),
            json!([{
                "range": {"start": {"line": 0, "character": 4}, "end": {"line": 1, "character": 0}},
                "newText": "\n"
            }])
        );
    }
}
