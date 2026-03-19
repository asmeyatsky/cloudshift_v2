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
    let (start_byte, end_byte) = spans.iter().fold((usize::MAX, 0usize), |(s, e), sp| {
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
        let call_span = caps
            .iter()
            .fold((usize::MAX, 0usize), |(s, e), (_, _, sp)| {
                (min(s, sp.start_byte), max(e, sp.end_byte))
            });
        let call_source_span = SourceSpan {
            start_byte: call_span.0,
            end_byte: call_span.1,
            start_row: 0,
            start_col: 0,
            end_row: 0,
            end_col: 0,
        };
        let merged = merge_spans(&[resource_span, table_span, call_source_span]);
        let source_slice =
            std::str::from_utf8(&source[call_source_span.start_byte..call_source_span.end_byte])
                .unwrap_or("");
        // Simplified: document id and set payload from Item=... (AC 4.8.2: warn if DynamoDB marshaled types)
        let (doc_id, item_data) = extract_put_item_bindings(source, &caps, source_slice);
        // Convert DynamoDB AttributeValue JSON to standard JSON for Firestore (fixup helper)
        let item_payload = crate::fixup::dynamodb_item_json_string_to_standard(&item_data)
            .unwrap_or_else(|_| item_data.clone());
        let replacement = format!(
            "db = firestore.Client()  # IBTE: consolidated boto3.resource + Table + put_item\ndb.collection('{}').document({}).set({})",
            table_name,
            doc_id,
            item_payload
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
            return ("__item_id__".to_string(), item_str.to_string());
        }
    }
    ("__item_id__".to_string(), "{}".to_string())
}

/// Extract Bucket=, Key=, Body= from S3 call; returns (bucket_expr, key_expr) for replacement.
/// Values are either string literals 'x'/"x" or left as-is for variables.
fn extract_s3_call_args(call_slice: &str) -> (String, String) {
    let bucket =
        extract_named_arg_expr(call_slice, "Bucket").unwrap_or_else(|| "'__bucket__'".to_string());
    let key = extract_named_arg_expr(call_slice, "Key").unwrap_or_else(|| "'__key__'".to_string());
    (bucket, key)
}

/// Return the next argument expression after name= (string literal or identifier).
fn extract_named_arg_expr(call: &str, name: &str) -> Option<String> {
    let marker = format!("{}=", name);
    let start = call.find(&marker)?;
    let after = call[start + marker.len()..].trim_start();
    if after.is_empty() {
        return None;
    }
    if after.starts_with('"') {
        let content = after.strip_prefix('"')?;
        let end = content.find('"')?;
        return Some(format!("\"{}\"", &content[..end]));
    }
    if after.starts_with('\'') {
        let content = after.strip_prefix('\'')?;
        let end = content.find('\'')?;
        return Some(format!("'{}'", &content[..end]));
    }
    // Identifier or expression: take until comma or closing paren
    let end = after.find(|c| c == ',' || c == ')').unwrap_or(after.len());
    Some(after[..end].trim().to_string())
}

/// Detect AWS S3 chain (client + put_object) and produce one consolidated match.
pub fn detect_s3_put_chain(
    source: &[u8],
    tree: &tree_sitter::Tree,
    registry: &StatefulContextRegistry,
) -> Result<Vec<PatternMatch>, AnalysisError> {
    let mut out = Vec::new();
    let lang = Language::Python;

    let put_q = r#"
    (call
      function: (attribute
        object: (identifier) @client_var
        attribute: (identifier) @method (#eq? @method "put_object"))
      arguments: (argument_list) @args)
    "#;
    let q = treesitter::compile_query(lang, put_q)?;
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
        let Some(client_span) = registry.s3_client_span(client_var) else {
            continue;
        };
        let call_span = caps
            .iter()
            .fold((usize::MAX, 0usize), |(s, e), (_, _, sp)| {
                (min(s, sp.start_byte), max(e, sp.end_byte))
            });
        let call_source_span = SourceSpan {
            start_byte: call_span.0,
            end_byte: call_span.1,
            start_row: 0,
            start_col: 0,
            end_row: 0,
            end_col: 0,
        };
        let merged = merge_spans(&[client_span, call_source_span]);
        let call_slice = std::str::from_utf8(&source[call_span.0..call_span.1]).unwrap_or("");
        let (bucket, key) = extract_s3_call_args(call_slice);
        let replacement = format!(
            "storage_client = storage.Client()  # IBTE: consolidated S3 put_object -> GCS\nstorage_client.bucket({}).blob({}).upload_from_string(raw_data)",
            bucket, key
        );
        out.push(PatternMatch {
            pattern_id: PatternId::new("ibte.aws.s3.put_object -> gcp.gcs.blob.upload_from_string"),
            span: merged,
            confidence: Confidence::new(0.90),
            source_text: String::new(),
            replacement_text: replacement,
            import_add: vec!["from google.cloud import storage".into()],
            import_remove: vec!["import boto3".into()],
        });
    }

    Ok(out)
}

/// Detect AWS S3 chain (client + get_object) and produce one consolidated match.
pub fn detect_s3_get_chain(
    source: &[u8],
    tree: &tree_sitter::Tree,
    registry: &StatefulContextRegistry,
) -> Result<Vec<PatternMatch>, AnalysisError> {
    let mut out = Vec::new();
    let lang = Language::Python;

    let get_q = r#"
    (call
      function: (attribute
        object: (identifier) @client_var
        attribute: (identifier) @method (#eq? @method "get_object"))
      arguments: (argument_list) @args)
    "#;
    let q = treesitter::compile_query(lang, get_q)?;
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
        let Some(client_span) = registry.s3_client_span(client_var) else {
            continue;
        };
        let call_span = caps
            .iter()
            .fold((usize::MAX, 0usize), |(s, e), (_, _, sp)| {
                (min(s, sp.start_byte), max(e, sp.end_byte))
            });
        let call_source_span = SourceSpan {
            start_byte: call_span.0,
            end_byte: call_span.1,
            start_row: 0,
            start_col: 0,
            end_row: 0,
            end_col: 0,
        };
        let merged = merge_spans(&[client_span, call_source_span]);
        let call_slice = std::str::from_utf8(&source[call_span.0..call_span.1]).unwrap_or("");
        let (bucket, key) = extract_s3_call_args(call_slice);
        let replacement = format!(
            "storage_client = storage.Client()  # IBTE: consolidated S3 get_object -> GCS\ncontent = storage_client.bucket({}).blob({}).download_as_bytes()",
            bucket, key
        );
        out.push(PatternMatch {
            pattern_id: PatternId::new("ibte.aws.s3.get_object -> gcp.gcs.blob.download_as_bytes"),
            span: merged,
            confidence: Confidence::new(0.90),
            source_text: String::new(),
            replacement_text: replacement,
            import_add: vec!["from google.cloud import storage".into()],
            import_remove: vec!["import boto3".into()],
        });
    }

    Ok(out)
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
        let call_span = caps
            .iter()
            .fold((usize::MAX, 0usize), |(s, e), (_, _, sp)| {
                (min(s, sp.start_byte), max(e, sp.end_byte))
            });
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
            pattern_id: PatternId::new(
                "ibte.azure.blob.upload_blob -> gcp.gcs.blob.upload_from_string",
            ),
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
