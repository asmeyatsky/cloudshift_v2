//! CloudShift LSP server for IDE integration.
//!
//! A minimal Language Server Protocol implementation that surfaces
//! migration diagnostics and quick-fix code actions in editors.
//!
//! This is a GA-scope skeleton (Q4 2026). It reads JSON-RPC messages
//! from stdin, handles the core LSP lifecycle, and stubs out the
//! analysis / code-action hooks that will call into cloudshift-core.

use std::io::{self, BufRead, Write as _};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

// ---------------------------------------------------------------------------
// JSON-RPC types (minimal subset needed for LSP)
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
// LSP message framing (Content-Length header)
// ---------------------------------------------------------------------------

/// Read a single LSP message from stdin using Content-Length framing.
fn read_message(reader: &mut impl BufRead) -> Result<Option<String>> {
    let mut content_length: Option<usize> = None;
    let mut header_line = String::new();

    // Read headers until blank line
    loop {
        header_line.clear();
        let bytes_read = reader
            .read_line(&mut header_line)
            .context("failed to read header line")?;
        if bytes_read == 0 {
            return Ok(None); // EOF
        }
        let trimmed = header_line.trim();
        if trimmed.is_empty() {
            break; // end of headers
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(
                value
                    .trim()
                    .parse()
                    .context("invalid Content-Length value")?,
            );
        }
        // Ignore other headers (Content-Type, etc.)
    }

    let length = content_length.context("missing Content-Length header")?;
    let mut body = vec![0u8; length];
    reader
        .read_exact(&mut body)
        .context("failed to read message body")?;
    let text = String::from_utf8(body).context("message body is not valid UTF-8")?;
    Ok(Some(text))
}

/// Write a JSON-RPC response to stdout with Content-Length framing.
fn send_response(response: &RpcResponse) -> Result<()> {
    let body = serde_json::to_string(response)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let stdout = io::stdout();
    let mut out = stdout.lock();
    out.write_all(header.as_bytes())?;
    out.write_all(body.as_bytes())?;
    out.flush()?;
    Ok(())
}

/// Write a JSON-RPC notification to stdout with Content-Length framing.
fn send_notification(notification: &RpcNotification) -> Result<()> {
    let body = serde_json::to_string(notification)?;
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    let stdout = io::stdout();
    let mut out = stdout.lock();
    out.write_all(header.as_bytes())?;
    out.write_all(body.as_bytes())?;
    out.flush()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// LSP handlers
// ---------------------------------------------------------------------------

/// Handle the `initialize` request — return server capabilities.
fn handle_initialize(id: serde_json::Value) -> Result<()> {
    let capabilities = serde_json::json!({
        "capabilities": {
            "textDocumentSync": 1,  // Full sync
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

/// Handle `textDocument/didOpen` — analyse the file and publish diagnostics.
fn handle_did_open(params: &serde_json::Value) -> Result<()> {
    let uri = params
        .pointer("/textDocument/uri")
        .and_then(|v| v.as_str())
        .unwrap_or("<unknown>");

    info!("didOpen: {}", uri);

    // TODO: call cloudshift_core::transform_file on the document content
    // and publish diagnostics for each pattern match.
    // For now, publish an empty diagnostics list.
    let notification = RpcNotification {
        jsonrpc: "2.0".into(),
        method: "textDocument/publishDiagnostics".into(),
        params: serde_json::json!({
            "uri": uri,
            "diagnostics": []
        }),
    };
    send_notification(&notification)
}

/// Handle `textDocument/codeAction` — suggest migration transforms.
fn handle_code_action(id: serde_json::Value, _params: &serde_json::Value) -> Result<()> {
    // TODO: inspect the document at the given range, check for
    // cloudshift pattern matches, and return CodeAction items with
    // workspace edits that apply the transform.
    // For now, return an empty list.
    send_response(&RpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(serde_json::json!([])),
        error: None,
    })
}

/// Handle the `shutdown` request.
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
    // Initialize tracing to stderr so it doesn't interfere with the
    // JSON-RPC protocol on stdout.
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
            None => {
                info!("stdin closed, exiting");
                break;
            }
        };

        let request: RpcRequest = match serde_json::from_str(&message) {
            Ok(r) => r,
            Err(e) => {
                warn!("failed to parse JSON-RPC message: {}", e);
                continue;
            }
        };

        if request.jsonrpc != "2.0" {
            warn!("unsupported jsonrpc version: {}", request.jsonrpc);
            continue;
        }

        match request.method.as_str() {
            "initialize" => {
                if let Some(id) = request.id {
                    handle_initialize(id)?;
                    initialized = true;
                    info!("initialized");
                }
            }
            "initialized" => {
                // Client acknowledgement — nothing to do.
                info!("client sent initialized notification");
            }
            "shutdown" => {
                if let Some(id) = request.id {
                    handle_shutdown(id)?;
                    shutdown_requested = true;
                    info!("shutdown requested");
                }
            }
            "exit" => {
                info!("exit notification received");
                std::process::exit(if shutdown_requested { 0 } else { 1 });
            }
            "textDocument/didOpen" => {
                if initialized {
                    handle_did_open(&request.params)?;
                }
            }
            "textDocument/didChange" => {
                // TODO: incremental analysis on content changes
            }
            "textDocument/didClose" => {
                // TODO: clean up per-document state
            }
            "textDocument/codeAction" => {
                if let Some(id) = request.id {
                    handle_code_action(id, &request.params)?;
                }
            }
            other => {
                warn!("unhandled method: {}", other);
                // For requests (with id), return method-not-found error
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
