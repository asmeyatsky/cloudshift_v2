//! Chain-pattern detection and N-to-1 consolidated replacement (PRD §4.8).
//!
//! Finds sequences (e.g. resource + table + put_item), merges their spans, and emits
//! a single PatternMatch with consolidated target-SDK code.

use crate::analyser::treesitter;
use crate::domain::entities::PatternMatch;
use crate::domain::ports::AnalysisError;
use crate::domain::value_objects::{Confidence, Language, PatternId, SourceSpan};
use crate::ibte::registry::StatefulContextRegistry;
use std::cmp::{max, min};

/// Merge multiple spans into one (min start_byte, max end_byte).
fn merge_spans(spans: &[SourceSpan]) -> SourceSpan {
    let (start_byte, end_byte) = spans
        .iter()
        .fold((usize::MAX, 0usize), |(s, e), sp| {
            (min(s, sp.start_byte), max(e, sp.end_byte))
        });
    SourceSpan {
        start_byte,
        end_byte,
        start_row: spans.first().map(|s| s.start_row).unwrap_or(0),
        start_col: spans.first().map(|s| s.start_col).unwrap_or(0),
        end_row: spans.last().map(|s| s.end_row).unwrap_or(0),
        end_col: spans.last().map(|s| s.end_col).unwrap_or(0),
    }
}

/// Detect AWS DynamoDB chain (resource + Table + put_item) and produce one consolidated match.
pub fn detect_dynamodb_put_chain(
    source: &[u8],
    tree: &tree_sitter::Tree,
    registry: &StatefulContextRegistry,
) -> Result<Vec<PatternMatch>, AnalysisError> {
    let mut out = Vec::new();
    let lang = Language::Python;

    // Find call: identifier.put_item(Item=..., ...)
    let put_item_q = r#"
    (call
      function: (attribute
        object: (identifier) @client_var
        attribute: (identifier) @method (#eq? @method "put_item"))
      arguments: (argument_list) @args)
    "#;
    let q = treesitter::compile_query(lang, put_item_q)?;
    let matches = treesitter::run_query(&q, tree, source);

    for m in &matches {
        let caps: Vec<_> = m
            .captures
            .iter()
            .map(|c| (c.name.clone(), c.text.clone(), c.span))
            .collect();
        let client_var = caps
            .iter()
            .find(|(n, _, _)| n == "client_var")
            .map(|(_, t, _)| t.as_str());
        let Some(client_var) = client_var else {
            continue;
        };
        let Some((resource_span, table_span, table_name)) =
            registry.dynamodb_chain_spans(client_var)
        else {
            continue;
        };
        // Full call span from captures (call node region)
        let call_span = caps.iter().fold(
            (usize::MAX, 0usize),
            |(s, e), (_, _, sp)| (min(s, sp.start_byte), max(e, sp.end_byte)),
        );
        let call_source_span = SourceSpan {
            start_byte: call_span.0,
            end_byte: call_span.1,
            start_row: 0,
            start_col: 0,
            end_row: 0,
            end_col: 0,
        };
        let merged = merge_spans(&[resource_span, table_span, call_source_span]);
        let source_slice = std::str::from_utf8(&source[call_source_span.start_byte..call_source_span.end_byte]).unwrap_or("");
        // Simplified: document id and set payload from Item=... (AC 4.8.2: warn if DynamoDB marshaled types)
        let (doc_id, item_data) = extract_put_item_bindings(source, &caps, source_slice);
        let replacement = format!(
            "db = firestore.Client()  # IBTE: consolidated boto3.resource + Table + put_item\ndb.collection('{}').document({}).set({})",
            table_name,
            doc_id,
            item_data
        );
        out.push(PatternMatch {
            pattern_id: PatternId::new("ibte.aws.dynamodb.put_item -> gcp.firestore.document.set"),
            span: merged,
            confidence: Confidence::new(0.92),
            source_text: String::new(),
            replacement_text: replacement,
            import_add: vec!["from google.cloud import firestore".into()],
            import_remove: vec!["import boto3".into()],
        });
    }

    Ok(out)
}

/// Naive extraction of Item key and body from put_item(Item={...}) for Firestore document/set.
fn extract_put_item_bindings(
    _source: &[u8],
    _caps: &[(String, String, SourceSpan)],
    call_slice: &str,
) -> (String, String) {
    // Look for Item= or first dict-like argument
    if let Some(start) = call_slice.find("Item=") {
        let after = &call_slice[start + 5..];
        let rest = after.trim_start();
        if rest.starts_with('{') {
            let mut depth = 0u32;
            let mut end = 0;
            for (i, c) in rest.chars().enumerate() {
                match c {
                    '{' => depth += 1,
                    '}' if depth > 0 => {
                        depth -= 1;
                        if depth == 0 {
                            end = i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let item_str = &rest[..end];
            // Use first key as document id placeholder if it looks like id
            let doc_id = if item_str.contains("'id'") || item_str.contains("\"id\"") {
                "__item_id__"
            } else {
                "__item_id__"
            };
            return (doc_id.to_string(), item_str.to_string());
        }
    }
    ("__item_id__".to_string(), "{}".to_string())
}

/// Detect Azure Blob chain (client + get_container_client + upload_blob) and produce one consolidated match.
pub fn detect_azure_blob_upload_chain(
    source: &[u8],
    tree: &tree_sitter::Tree,
    registry: &StatefulContextRegistry,
) -> Result<Vec<PatternMatch>, AnalysisError> {
    let mut out = Vec::new();
    let lang = Language::Python;

    let upload_q = r#"
    (call
      function: (attribute
        object: (identifier) @cont_var
        attribute: (identifier) @method (#eq? @method "upload_blob"))
      arguments: (argument_list) @args)
    "#;
    let q = treesitter::compile_query(lang, upload_q)?;
    let matches = treesitter::run_query(&q, tree, source);

    for m in &matches {
        let caps: Vec<_> = m
            .captures
            .iter()
            .map(|c| (c.name.clone(), c.text.clone(), c.span))
            .collect();
        let cont_var = caps
            .iter()
            .find(|(n, _, _)| n == "cont_var")
            .map(|(_, t, _)| t.as_str());
        let Some(cont_var) = cont_var else {
            continue;
        };
        let Some((client_span, container_span, bucket_name)) =
            registry.azure_blob_chain_spans(cont_var)
        else {
            continue;
        };
        let call_span = caps.iter().fold(
            (usize::MAX, 0usize),
            |(s, e), (_, _, sp)| (min(s, sp.start_byte), max(e, sp.end_byte)),
        );
        let call_source_span = SourceSpan {
            start_byte: call_span.0,
            end_byte: call_span.1,
            start_row: 0,
            start_col: 0,
            end_row: 0,
            end_col: 0,
        };
        let merged = merge_spans(&[client_span, container_span, call_source_span]);
        let replacement = format!(
            "storage_client = storage.Client()  # IBTE: Mapped Azure Container '{}' to GCS bucket\nstorage_client.bucket('{}').blob(__key__).upload_from_string(raw_data)",
            bucket_name,
            bucket_name
        );
        out.push(PatternMatch {
            pattern_id: PatternId::new("ibte.azure.blob.upload_blob -> gcp.gcs.blob.upload_from_string"),
            span: merged,
            confidence: Confidence::new(0.90),
            source_text: String::new(),
            replacement_text: replacement,
            import_add: vec!["from google.cloud import storage".into()],
            import_remove: vec![
                "from azure.storage.blob import BlobServiceClient".into(),
                "from azure.storage.blob import BlobClient".into(),
                "from azure.storage.blob import ContainerClient".into(),
            ],
        });
    }

    Ok(out)
}
