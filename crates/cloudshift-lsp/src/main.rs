//! CloudShift LSP server for IDE integration.
//!
//! Language Server Protocol implementation that runs cloudshift-core
//! on open/changed documents and publishes migration diagnostics and
//! code actions (Transform to GCP).

use std::collections::HashMap;
use std::io::{self, BufRead, Write as _};
use std::str::FromStr;
use std::sync::Mutex;

use anyhow::{Context, Result};
use cloudshift_core::domain::entities::{TransformResult, Warning};
use cloudshift_core::domain::value_objects::{Language, SourceCloud};
use cloudshift_core::pipeline::{transform_source_for_api, TransformConfig};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// Server state (patterns path, source cloud, last result per document)
// ---------------------------------------------------------------------------

struct LspState {
    patterns_path: Option<String>,
    source_cloud: SourceCloud,
    /// URI -> last transform result (for code actions)
    results: HashMap<String, TransformResult>,
}

impl Default for LspState {
    fn default() -> Self {
        Self {
            patterns_path: None,
            source_cloud: SourceCloud::Any,
            results: HashMap::new(),
        }
    }
}

static STATE: Mutex<Option<LspState>> = Mutex::new(None);

fn with_state<T, F>(f: F) -> T
where
    F: FnOnce(&mut LspState) -> T,
{
    let mut g = STATE.lock().unwrap();
    if g.is_none() {
        *g = Some(LspState::default());
    }
    f(g.as_mut().unwrap())
}

// ---------------------------------------------------------------------------
// JSON-RPC types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct RpcResponse {
    jsonrpc: String,
    id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Debug, Serialize)]
struct RpcNotification {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
}

// ---------------------------------------------------------------------------
// LSP message framing
// ---------------------------------------------------------------------------

fn read_message(reader: &mut impl BufRead) -> Result<Option<String>> {
    let mut content_length: Option<usize> = None;
    let mut header_line = String::new();
    loop {
        header_line.clear();
        let bytes_read = reader
            .read_line(&mut header_line)
            .context("failed to read header line")?;
        if bytes_read == 0 {
            return Ok(None);
        }
        let trimmed = header_line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(value.trim().parse().context("invalid Content-Length")?);
        }
    }
    let length = content_length.context("missing Content-Length header")?;
    let mut body = vec![0u8; length];
    reader
        .read_exact(&mut body)
        .context("failed to read body")?;
    let text = String::from_utf8(body).context("message body not UTF-8")?;
    Ok(Some(text))
}

fn send_response(response: &RpcResponse) -> Result<()> {
    let body = serde_json::to_string(response)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut out = io::stdout().lock();
    out.write_all(header.as_bytes())?;
    out.write_all(body.as_bytes())?;
    out.flush()?;
    Ok(())
}

fn send_notification(notification: &RpcNotification) -> Result<()> {
    let body = serde_json::to_string(notification)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let mut out = io::stdout().lock();
    out.write_all(header.as_bytes())?;
    out.write_all(body.as_bytes())?;
    out.flush()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// URI / path helpers
// ---------------------------------------------------------------------------

/// Extract a path hint from a file URI for language detection and display.
fn uri_to_path_hint(uri: &str) -> String {
    let path = uri
        .strip_prefix("file://")
        .unwrap_or(uri)
        .replace("%2F", "/")
        .replace("%3A", ":");
    path.rsplit('/').next().unwrap_or(&path).to_string()
}

fn path_hint_to_language(path_hint: &str) -> Option<Language> {
    Language::from_filename(path_hint)
}

// ---------------------------------------------------------------------------
// Transform and convert to LSP diagnostics
// ---------------------------------------------------------------------------

fn run_transform(
    uri: &str,
    source: &str,
    patterns_path: &Option<String>,
    source_cloud: SourceCloud,
) -> Result<TransformResult> {
    let path_hint = uri_to_path_hint(uri);
    let language = path_hint_to_language(&path_hint)
        .ok_or_else(|| anyhow::anyhow!("Unsupported file type: {}", path_hint))?;
    let config = TransformConfig {
        source_cloud,
        catalogue_path: patterns_path.clone(),
        dry_run: true,
        threshold: 0.0,
        ..Default::default()
    };
    transform_source_for_api(&path_hint, source, language, &config)
        .context("transform_source_for_api failed")
}

/// Map CloudShift Warning + PatternMatch to LSP Diagnostic severity.
fn lsp_severity(warning: &Warning) -> i32 {
    use cloudshift_core::domain::entities::WarningSeverity;
    match warning.severity {
        WarningSeverity::Error => 1,
        WarningSeverity::Warning => 2,
        WarningSeverity::Info => 3,
    }
}

fn span_to_lsp_range(
    start_row: usize,
    start_col: usize,
    end_row: usize,
    end_col: usize,
) -> serde_json::Value {
    serde_json::json!({
        "start": { "line": start_row, "character": start_col },
        "end": { "line": end_row, "character": end_col }
    })
}

fn result_to_diagnostics(result: &TransformResult, _uri: &str) -> Vec<serde_json::Value> {
    let mut diagnostics = Vec::new();
    for w in &result.warnings {
        let range = match &w.span {
            Some(s) => span_to_lsp_range(s.start_row, s.start_col, s.end_row, s.end_col),
            None => span_to_lsp_range(0, 0, 0, 0),
        };
        diagnostics.push(serde_json::json!({
            "range": range,
            "message": w.message,
            "severity": lsp_severity(w),
            "source": "cloudshift"
        }));
    }
    for m in &result.patterns {
        let s = &m.span;
        let range = span_to_lsp_range(s.start_row, s.start_col, s.end_row, s.end_col);
        diagnostics.push(serde_json::json!({
            "range": range,
            "message": format!("[GCP migration] {} → {}", m.pattern_id.0, m.replacement_text.lines().next().unwrap_or("").trim()),
            "severity": 2,
            "source": "cloudshift",
            "code": m.pattern_id.0
        }));
    }
    diagnostics
}

// ---------------------------------------------------------------------------
// LSP handlers
// ---------------------------------------------------------------------------

fn handle_initialize(id: serde_json::Value, params: &serde_json::Value) -> Result<()> {
    let patterns_path = params
        .get("initializationOptions")
        .and_then(|o| o.get("patternsPath"))
        .and_then(|v| v.as_str())
        .map(String::from);
    let source_cloud_str = params
        .get("initializationOptions")
        .and_then(|o| o.get("sourceCloud"))
        .and_then(|v| v.as_str())
        .unwrap_or("any");
    let source_cloud = SourceCloud::from_str(source_cloud_str)
        .ok()
        .unwrap_or(SourceCloud::Any);

    with_state(|s| {
        s.patterns_path = patterns_path.or_else(|| std::env::var("CLOUDSHIFT_PATTERNS_DIR").ok());
        s.source_cloud = source_cloud;
    });

    let capabilities = serde_json::json!({
        "capabilities": {
            "textDocumentSync": 1,
            "codeActionProvider": true,
            "diagnosticProvider": {
                "interFileDependencies": false,
                "workspaceDiagnostics": false
            }
        },
        "serverInfo": {
            "name": "cloudshift-lsp",
            "version": env!("CARGO_PKG_VERSION")
        }
    });

    send_response(&RpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(capabilities),
        error: None,
    })
}

fn handle_did_open(params: &serde_json::Value) -> Result<()> {
    let uri = params
        .pointer("/textDocument/uri")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let text = params
        .pointer("/textDocument/text")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let (patterns_path, source_cloud) = with_state(|s| (s.patterns_path.clone(), s.source_cloud));

    match run_transform(uri, text, &patterns_path, source_cloud) {
        Ok(result) => {
            with_state(|s| {
                s.results.insert(uri.to_string(), result.clone());
            });
            let diagnostics = result_to_diagnostics(&result, uri);
            send_notification(&RpcNotification {
                jsonrpc: "2.0".into(),
                method: "textDocument/publishDiagnostics".into(),
                params: serde_json::json!({ "uri": uri, "diagnostics": diagnostics }),
            })?;
        }
        Err(e) => {
            warn!("transform failed for {}: {}", uri, e);
            send_notification(&RpcNotification {
                jsonrpc: "2.0".into(),
                method: "textDocument/publishDiagnostics".into(),
                params: serde_json::json!({ "uri": uri, "diagnostics": [] }),
            })?;
        }
    }
    Ok(())
}

fn handle_did_change(params: &serde_json::Value) -> Result<()> {
    let uri = params
        .pointer("/textDocument/uri")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let text = params
        .get("contentChanges")
        .and_then(|a| a.as_array())
        .and_then(|a| a.first())
        .and_then(|c| c.get("text"))
        .and_then(|t| t.as_str())
        .unwrap_or("");

    let (patterns_path, source_cloud) = with_state(|s| (s.patterns_path.clone(), s.source_cloud));

    if let Ok(result) = run_transform(uri, text, &patterns_path, source_cloud) {
        with_state(|s| {
            s.results.insert(uri.to_string(), result.clone());
        });
        let diagnostics = result_to_diagnostics(&result, uri);
        send_notification(&RpcNotification {
            jsonrpc: "2.0".into(),
            method: "textDocument/publishDiagnostics".into(),
            params: serde_json::json!({ "uri": uri, "diagnostics": diagnostics }),
        })?;
    }
    Ok(())
}

fn handle_code_action(id: serde_json::Value, params: &serde_json::Value) -> Result<()> {
    let uri = params
        .pointer("/textDocument/uri")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let range = params
        .pointer("/range")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let (start_line, start_char, end_line, end_char) = (
        range
            .get("start")
            .and_then(|s| s.get("line").and_then(|v| v.as_u64()))
            .unwrap_or(0) as usize,
        range
            .get("start")
            .and_then(|s| s.get("character").and_then(|v| v.as_u64()))
            .unwrap_or(0) as usize,
        range
            .get("end")
            .and_then(|e| e.get("line").and_then(|v| v.as_u64()))
            .unwrap_or(0) as usize,
        range
            .get("end")
            .and_then(|e| e.get("character").and_then(|v| v.as_u64()))
            .unwrap_or(0) as usize,
    );

    let result_opt = with_state(|s| s.results.get(uri).cloned());
    let mut actions: Vec<serde_json::Value> = Vec::new();
    if let Some(result) = result_opt {
        for m in &result.patterns {
            if range_overlaps(
                start_line,
                start_char,
                end_line,
                end_char,
                m.span.start_row,
                m.span.start_col,
                m.span.end_row,
                m.span.end_col,
            ) {
                let edit_range = span_to_lsp_range(
                    m.span.start_row,
                    m.span.start_col,
                    m.span.end_row,
                    m.span.end_col,
                );
                let mut changes = serde_json::Map::new();
                changes.insert(
                    uri.to_string(),
                    serde_json::json!([{ "range": edit_range, "newText": m.replacement_text }]),
                );
                actions.push(serde_json::json!({
                    "title": format!("CloudShift: Apply {} → GCP", m.pattern_id.0),
                    "kind": "quickfix",
                    "edit": { "changes": changes }
                }));
            }
        }
        if actions.is_empty() && !result.patterns.is_empty() {
            for m in &result.patterns {
                let edit_range = span_to_lsp_range(
                    m.span.start_row,
                    m.span.start_col,
                    m.span.end_row,
                    m.span.end_col,
                );
                let mut changes = serde_json::Map::new();
                changes.insert(
                    uri.to_string(),
                    serde_json::json!([{ "range": edit_range, "newText": m.replacement_text }]),
                );
                actions.push(serde_json::json!({
                    "title": format!("CloudShift: Apply {} → GCP", m.pattern_id.0),
                    "kind": "quickfix",
                    "edit": { "changes": changes }
                }));
            }
        }
    }

    send_response(&RpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(serde_json::json!(actions)),
        error: None,
    })
}

fn range_overlaps(
    a_start_l: usize,
    a_start_c: usize,
    a_end_l: usize,
    a_end_c: usize,
    b_start_l: usize,
    b_start_c: usize,
    b_end_l: usize,
    b_end_c: usize,
) -> bool {
    let a_before_b = a_end_l < b_start_l || (a_end_l == b_start_l && a_end_c <= b_start_c);
    let b_before_a = b_end_l < a_start_l || (b_end_l == a_start_l && b_end_c <= a_start_c);
    !a_before_b && !b_before_a
}

fn handle_shutdown(id: serde_json::Value) -> Result<()> {
    send_response(&RpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(serde_json::Value::Null),
        error: None,
    })
}

// ---------------------------------------------------------------------------
// Main loop
// ---------------------------------------------------------------------------

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!(
        "cloudshift-lsp starting (version {})",
        env!("CARGO_PKG_VERSION")
    );

    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut initialized = false;
    let mut shutdown_requested = false;

    loop {
        let message = match read_message(&mut reader)? {
            Some(msg) => msg,
            None => break,
        };

        let request: RpcRequest = match serde_json::from_str(&message) {
            Ok(r) => r,
            Err(e) => {
                warn!("failed to parse JSON-RPC: {}", e);
                continue;
            }
        };

        if request.jsonrpc != "2.0" {
            continue;
        }

        match request.method.as_str() {
            "initialize" => {
                if let Some(id) = request.id {
                    handle_initialize(id, &request.params)?;
                    initialized = true;
                }
            }
            "initialized" => {}
            "shutdown" => {
                if let Some(id) = request.id {
                    handle_shutdown(id)?;
                    shutdown_requested = true;
                }
            }
            "exit" => std::process::exit(if shutdown_requested { 0 } else { 1 }),
            "textDocument/didOpen" => {
                if initialized {
                    handle_did_open(&request.params)?;
                }
            }
            "textDocument/didChange" => {
                if initialized {
                    handle_did_change(&request.params)?;
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = request
                    .params
                    .get("textDocument")
                    .and_then(|t| t.get("uri"))
                    .and_then(|u| u.as_str())
                {
                    with_state(|s| {
                        s.results.remove(uri);
                    });
                }
            }
            "textDocument/codeAction" => {
                if let Some(id) = request.id {
                    handle_code_action(id, &request.params)?;
                }
            }
            other => {
                warn!("unhandled method: {}", other);
                if let Some(id) = request.id {
                    send_response(&RpcResponse {
                        jsonrpc: "2.0".into(),
                        id,
                        result: None,
                        error: Some(RpcError {
                            code: -32601,
                            message: format!("method not found: {}", other),
                        }),
                    })?;
                }
            }
        }
    }

    Ok(())
}
