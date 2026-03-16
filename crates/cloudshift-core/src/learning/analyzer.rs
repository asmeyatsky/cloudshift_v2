//! Change analysis using tree-sitter.
//!
//! Analyzes LLM deltas to determine what kind of cloud SDK transformation
//! was performed: import change, method call replacement, client init
//! replacement, resource block rewrite, etc.

use super::extractor::LlmDelta;
use crate::domain::value_objects::{Language, SourceCloud};

/// Classification of what an LLM delta represents.
#[derive(Debug, Clone)]
pub struct AnalyzedChange {
    /// The original delta from the LLM.
    pub delta: LlmDelta,
    /// What kind of change this is.
    pub change_type: ChangeType,
    /// Source cloud provider detected in the original code.
    pub source_cloud: SourceCloud,
    /// The AWS/Azure method or construct that was replaced.
    pub source_construct: String,
    /// The GCP equivalent that replaced it.
    pub target_construct: String,
    /// Detected import additions.
    pub import_additions: Vec<String>,
    /// Detected import removals.
    pub import_removals: Vec<String>,
    /// Estimated confidence for a generated pattern (0.0-1.0).
    pub suggested_confidence: f64,
    /// Suggested tags for the pattern.
    pub suggested_tags: Vec<String>,
}

/// What kind of transformation the LLM performed.
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    /// Import statement change (e.g., boto3 -> google.cloud)
    ImportChange,
    /// SDK client constructor replacement
    ClientInit,
    /// API method call replacement (most common)
    MethodCallReplacement,
    /// Multi-line code block rewrite
    CodeBlockRewrite,
    /// Resource definition rewrite (Terraform/HCL)
    ResourceBlockRewrite,
    /// Exception/error handling change
    ExceptionHandling,
    /// String literal change (ARNs, endpoints, URIs)
    StringLiteralChange,
    /// Unknown/complex change requiring manual review
    Unknown,
}

/// Analyze a list of LLM deltas to classify each change.
pub fn analyze_changes(deltas: &[LlmDelta], language: Language) -> Vec<AnalyzedChange> {
    deltas
        .iter()
        .map(|delta| analyze_single_delta(delta, language))
        .collect()
}

fn analyze_single_delta(delta: &LlmDelta, _language: Language) -> AnalyzedChange {
    let pattern_text = &delta.pattern_output;
    let llm_text = &delta.llm_output;

    // Detect source cloud from the original code
    let source_cloud = if pattern_text.contains("boto3")
        || pattern_text.contains("aws")
        || pattern_text.contains("s3")
        || pattern_text.contains("dynamodb")
    {
        SourceCloud::Aws
    } else if pattern_text.contains("azure") || pattern_text.contains("Azure") {
        SourceCloud::Azure
    } else {
        SourceCloud::Any
    };

    // Classify the change type
    let change_type = classify_change(pattern_text, llm_text);

    // Extract source construct (what AWS/Azure thing was replaced)
    let source_construct = extract_source_construct(pattern_text);

    // Extract target construct (what GCP thing replaced it)
    let target_construct = extract_target_construct(llm_text);

    // Extract import changes
    let import_additions = extract_imports(llm_text);
    let import_removals = extract_imports(pattern_text);

    // Suggest confidence based on change complexity
    let suggested_confidence = match change_type {
        ChangeType::ImportChange => 0.95,
        ChangeType::ClientInit => 0.90,
        ChangeType::MethodCallReplacement => 0.85,
        ChangeType::StringLiteralChange => 0.92,
        ChangeType::ExceptionHandling => 0.88,
        ChangeType::CodeBlockRewrite => 0.75,
        ChangeType::ResourceBlockRewrite => 0.78,
        ChangeType::Unknown => 0.65,
    };

    // Generate tags
    let mut tags = vec!["learned".to_string(), "llm-generated".to_string()];
    if !source_construct.is_empty() {
        tags.push(source_construct.to_lowercase().replace('.', "-"));
    }

    AnalyzedChange {
        delta: delta.clone(),
        change_type,
        source_cloud,
        source_construct,
        target_construct,
        import_additions,
        import_removals,
        suggested_confidence,
        suggested_tags: tags,
    }
}

fn classify_change(original: &str, replacement: &str) -> ChangeType {
    let orig_trimmed = original.trim();
    let repl_trimmed = replacement.trim();

    // Import changes
    if (orig_trimmed.starts_with("import ") || orig_trimmed.starts_with("from "))
        && (repl_trimmed.starts_with("import ") || repl_trimmed.starts_with("from "))
    {
        return ChangeType::ImportChange;
    }

    // Client initialization
    for keyword in &[
        "boto3.client",
        "boto3.resource",
        "boto3.Session",
        "BlobServiceClient",
        "SecretClient",
        "CosmosClient",
    ] {
        if orig_trimmed.contains(keyword) {
            return ChangeType::ClientInit;
        }
    }

    // Exception handling
    if orig_trimmed.contains("except") || orig_trimmed.contains("ClientError") {
        return ChangeType::ExceptionHandling;
    }

    // String literal (ARN, endpoint)
    if orig_trimmed.contains("arn:aws:")
        || orig_trimmed.contains(".amazonaws.com")
        || orig_trimmed.contains("s3://")
    {
        return ChangeType::StringLiteralChange;
    }

    // HCL resource block
    if orig_trimmed.starts_with("resource ") || orig_trimmed.contains("aws_") {
        return ChangeType::ResourceBlockRewrite;
    }

    // Method call (single line with a function call)
    if (orig_trimmed.contains('(') && orig_trimmed.contains(')'))
        || (repl_trimmed.contains('(') && repl_trimmed.contains(')'))
    {
        if original.lines().count() <= 5 {
            return ChangeType::MethodCallReplacement;
        }
        return ChangeType::CodeBlockRewrite;
    }

    ChangeType::Unknown
}

fn extract_source_construct(text: &str) -> String {
    // Try to find the method being called
    let methods = [
        "put_object",
        "get_object",
        "delete_object",
        "list_objects",
        "send_message",
        "receive_message",
        "publish",
        "put_item",
        "get_item",
        "delete_item",
        "query",
        "scan",
        "invoke_model",
        "detect_labels",
        "detect_sentiment",
        "get_secret_value",
        "assume_role",
        "upload_blob",
        "download_blob",
        "get_secret",
    ];

    for method in &methods {
        if text.contains(method) {
            return method.to_string();
        }
    }

    // Fallback: first non-empty line trimmed
    text.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim()
        .chars()
        .take(60)
        .collect()
}

fn extract_target_construct(text: &str) -> String {
    // Try to find GCP method/class
    let constructs = [
        "storage.Client",
        "firestore.Client",
        "pubsub_v1",
        "secretmanager",
        "bigquery.Client",
        "aiplatform",
        "vision.ImageAnnotatorClient",
        "language_v1",
        "upload_from_string",
        "download_as_bytes",
        "list_blobs",
    ];

    for construct in &constructs {
        if text.contains(construct) {
            return construct.to_string();
        }
    }

    text.lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
        .trim()
        .chars()
        .take(60)
        .collect()
}

fn extract_imports(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("import ") || t.starts_with("from ")
        })
        .map(|l| l.trim().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::learning::extractor::LlmDelta;

    #[test]
    fn test_classify_import_change() {
        assert_eq!(
            classify_change("import boto3", "from google.cloud import storage"),
            ChangeType::ImportChange
        );
    }

    #[test]
    fn test_classify_client_init() {
        assert_eq!(
            classify_change("s3 = boto3.client('s3')", "client = storage.Client()"),
            ChangeType::ClientInit
        );
    }

    #[test]
    fn test_classify_method_call() {
        assert_eq!(
            classify_change(
                "s3.put_object(Bucket='b', Key='k')",
                "blob.upload_from_string(data)"
            ),
            ChangeType::MethodCallReplacement
        );
    }

    #[test]
    fn test_suggested_confidence() {
        let delta = LlmDelta {
            original_line_start: 0,
            original_line_end: 1,
            pattern_output: "import boto3".into(),
            llm_output: "from google.cloud import storage".into(),
            context_before: vec![],
            context_after: vec![],
        };
        let analyzed = analyze_single_delta(&delta, Language::Python);
        assert!(analyzed.suggested_confidence >= 0.90);
    }
}
