use lsp_server::{Message, Notification, Request, Response};
use serde_json::{Value, json};
use std::{
    io::BufReader,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{Receiver, channel},
    thread,
    time::Duration,
};
use tempfile::tempdir;
use url::Url;

struct Client {
    child: Child,
    input: ChildStdin,
    messages: Receiver<Message>,
    next_id: i32,
}

impl Client {
    fn new(cwd: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_gdfmt"))
            .arg("--lsp")
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap());
        let (tx, messages) = channel();
        thread::spawn(move || {
            while let Some(message) = Message::read(&mut output).unwrap() {
                if tx.send(message).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            input,
            messages,
            next_id: 0,
        }
    }

    fn notify(&mut self, method: &str, params: Value) {
        Message::Notification(Notification::new(method.into(), params))
            .write(&mut self.input)
            .unwrap();
    }

    fn send(&mut self, method: &str, params: Value) -> i32 {
        self.next_id += 1;
        Message::Request(Request::new(self.next_id.into(), method.into(), params))
            .write(&mut self.input)
            .unwrap();
        self.next_id
    }

    fn response(&self, id: i32) -> Response {
        loop {
            let message = self
                .messages
                .recv_timeout(Duration::from_secs(15))
                .expect("LSP response timeout");
            if let Message::Response(response) = message {
                assert_eq!(response.id, id.into());
                return response;
            }
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.send(method, params);
        let response = self.response(id);
        assert!(response.error.is_none(), "{:?}", response.error);
        response.result.unwrap_or(Value::Null)
    }

    fn initialize(&mut self, cwd: &std::path::Path) {
        let result = self.request(
            "initialize",
            json!({"rootUri": Url::from_directory_path(cwd).unwrap().as_str(), "capabilities": {}}),
        );
        assert_eq!(result["capabilities"]["positionEncoding"], "utf-16");
        assert_eq!(
            result["capabilities"]["experimental"]["gdfmtTextFormattingProvider"],
            true
        );
        assert!(
            result["capabilities"]
                .get("documentRangeFormattingProvider")
                .is_none()
        );
        self.notify("initialized", json!({}));
    }

    fn open(&mut self, uri: &str, text: &str) {
        self.notify("textDocument/didOpen", json!({"textDocument": {"uri": uri, "text": text, "version": 1, "languageId": "gdscript"}}));
    }

    fn format(&mut self, uri: &str) -> Value {
        self.request(
            "textDocument/formatting",
            json!({"textDocument": {"uri": uri}, "options": {"tabSize": 4, "insertSpaces": false}}),
        )
    }

    fn shutdown(&mut self) {
        assert_eq!(self.request("shutdown", Value::Null), Value::Null);
        self.notify("exit", Value::Null);
        // Exit must complete even while the parent's stdin handle is still open.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(std::time::Instant::now() < deadline, "LSP did not exit");
            thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn lsp_rejects_range_formatting_without_falling_back_to_document_formatting() {
    let root = tempdir().unwrap();
    let mut client = Client::new(root.path());
    client.initialize(root.path());
    let uri = "untitled:range";
    client.open(uri, "var first=1\nvar second=2\n");
    for method in [
        "textDocument/rangeFormatting",
        "textDocument/rangesFormatting",
    ] {
        let id = client.send(
            method,
            json!({"textDocument": {"uri": uri}, "range": {
                "start": {"line": 0, "character": 0},
                "end": {"line": 0, "character": 11}
            }, "options": {"tabSize": 2, "insertSpaces": true}}),
        );
        let response = client.response(id);
        assert_eq!(response.error.unwrap().code, -32601);
        assert!(response.result.is_none());
    }
    assert_ne!(client.format(uri), json!([]));
    client.shutdown();
}

#[test]
fn lsp_text_formatting_returns_target_eol_and_never_edits_invalid_or_ignored_source() {
    let root = tempdir().unwrap();
    let path = root.path().join("example.gd");
    let uri = Url::from_file_path(&path).unwrap().to_string();
    let mut client = Client::new(root.path());
    client.initialize(root.path());
    client.open(&uri, "var x = 1\n");
    for (line_ending, output) in [
        ("crlf", "var x = 1\r\n"),
        ("lf", "var x = 1\n"),
        ("preserve", "var x = 1\n"),
    ] {
        std::fs::write(
            root.path().join(".gdfmtrc.json"),
            json!({"root": true, "format": {"lineEnding": line_ending}}).to_string(),
        )
        .unwrap();
        assert_eq!(
            client.request("gdfmt/formatting", json!({"textDocument": {"uri": uri}})),
            json!({"output": output, "lineEnding": line_ending})
        );
    }
    client.notify("textDocument/didChange", json!({"textDocument": {"uri": uri, "version": 2}, "contentChanges": [{"text": "var =\n"}]}));
    assert_eq!(
        client.request("gdfmt/formatting", json!({"textDocument": {"uri": uri}}))["output"],
        Value::Null
    );
    std::fs::write(
        root.path().join(".gdfmtrc.json"),
        r#"{"root":true,"ignore":["*.gd"]}"#,
    )
    .unwrap();
    assert_eq!(
        client.request("gdfmt/formatting", json!({"textDocument": {"uri": uri}}))["output"],
        Value::Null
    );
    client.shutdown();
    assert!(!path.exists());
}

#[test]
fn lsp_formats_editor_text_and_reloads_configuration_without_writing() {
    let root = tempdir().unwrap();
    let path = root.path().join("中文 space.gd");
    std::fs::write(&path, "var disk=0\n").unwrap();
    let uri = Url::from_file_path(&path).unwrap().to_string();
    let mut client = Client::new(root.path());
    client.initialize(root.path());
    client.open(&uri, "var 名称=\"😀\"\r\n");
    let edits = client.format(&uri);
    assert_eq!(
        edits,
        json!([{
            "range": {"start": {"line": 0, "character": 6}, "end": {"line": 1, "character": 0}},
            "newText": " = \"😀\"\n"
        }])
    );
    client.notify("textDocument/didChange", json!({"textDocument": {"uri": uri, "version": 2}, "contentChanges": [{"text": "var value=1\n"}]}));
    assert_eq!(client.format(&uri)[0]["newText"], " = ");
    client.notify("textDocument/didChange", json!({"textDocument": {"uri": uri, "version": 3}, "contentChanges": [{"text": "var old=0\n"}, {"text": "var latest=3\n"}]}));
    assert_eq!(client.format(&uri)[0]["range"]["start"]["character"], 10);
    client.notify(
        "textDocument/didChange",
        json!({"textDocument": {"uri": uri, "version": 4}, "contentChanges": []}),
    );
    assert_eq!(client.format(&uri)[0]["newText"], " = ");
    std::fs::write(
        root.path().join(".gdfmtrc.jsonc"),
        "{\"root\":true,\"ignore\":[\"*.gd\"]}",
    )
    .unwrap();
    client.notify("workspace/didChangeWatchedFiles", json!({"changes": []}));
    assert_eq!(client.format(&uri), json!([]));
    std::fs::write(
        root.path().join("explicit.json"),
        "{\"format\":{\"finalNewline\":false}}",
    )
    .unwrap();
    client.notify(
        "workspace/didChangeConfiguration",
        json!({"settings": {"gdfmt": {"configPath": "explicit.json"}}}),
    );
    assert_ne!(client.format(&uri), json!([]));
    client.notify("textDocument/didChangeConfiguration", json!({}));
    client.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": uri}}),
    );
    let id = client.send(
        "textDocument/formatting",
        json!({"textDocument": {"uri": uri}}),
    );
    assert_eq!(client.response(id).error.unwrap().code, -32602);
    client.shutdown();
    assert_eq!(std::fs::read_to_string(path).unwrap(), "var disk=0\n");
}

#[test]
fn lsp_returns_diagnostics_without_edits_for_invalid_source() {
    let root = tempdir().unwrap();
    // Untitled buffers must not inherit incidental installation/project config.
    std::fs::write(root.path().join(".gdfmtrc.json"), "invalid config").unwrap();
    let mut client = Client::new(root.path());
    client.initialize(root.path());
    let uri = "untitled:Untitled-1";
    client.open(uri, "var text=\"😀\"; var =\n");
    let id = client.send(
        "textDocument/formatting",
        json!({"textDocument": {"uri": uri}}),
    );
    let message = client
        .messages
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let Message::Notification(diagnostics) = message else {
        panic!("expected diagnostics");
    };
    assert_eq!(diagnostics.method, "textDocument/publishDiagnostics");
    assert_eq!(diagnostics.params["version"], 1);
    assert!(
        !diagnostics.params["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let diagnostics = &diagnostics.params["diagnostics"][0];
    assert!(diagnostics["range"]["start"]["character"].as_u64().unwrap() <= 21);
    assert_eq!(client.response(id).result.unwrap(), json!([]));
    client.shutdown();
}

#[test]
fn lsp_cancels_pending_work_and_discards_changed_document_results() {
    for method in ["textDocument/formatting", "gdfmt/formatting"] {
        let root = tempdir().unwrap();
        let mut client = Client::new(root.path());
        client.initialize(root.path());
        let uri = "untitled:large";
        let source: String = (0..10_000).map(|n| format!("var value_{n}=1\n")).collect();
        client.open(uri, &source);
        let first = client.send(method, json!({"textDocument": {"uri": uri}}));
        client.notify("$/cancelRequest", json!({"id": first}));
        assert_eq!(client.response(first).error.unwrap().code, -32800);
        let second = client.send(method, json!({"textDocument": {"uri": uri}}));
        client.notify("textDocument/didChange", json!({"textDocument": {"uri": uri, "version": 2}, "contentChanges": [{"text": "var fresh = 2\n"}]}));
        assert_eq!(client.response(second).error.unwrap().code, -32801);
        assert_eq!(client.format(uri), json!([]));
        client.shutdown();
    }
}
