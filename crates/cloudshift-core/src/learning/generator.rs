//! Candidate pattern generator.
//!
//! Takes analyzed LLM changes and generates TOML pattern definitions
//! that can be reviewed and promoted to the compiled catalogue.

use super::analyzer::AnalyzedChange;
use crate::domain::value_objects::Language;
use chrono::Utc;
use uuid::Uuid;

/// A candidate pattern ready to be written to disk as a TOML file.
#[derive(Debug, Clone)]
pub struct CandidatePattern {
    /// Unique ID for this candidate.
    pub candidate_id: String,
    /// The generated TOML content.
    pub toml_content: String,
    /// Suggested filename.
    pub suggested_filename: String,
    /// Language this pattern applies to.
    pub language: Language,
    /// When this candidate was generated.
    pub generated_at: String,
    /// What source file triggered the learning.
    pub source_file: String,
    /// The analyzed change this was generated from.
    pub change: AnalyzedChange,
}

/// Generate a candidate TOML pattern from an analyzed change.
pub fn generate_candidate_pattern(
    change: &AnalyzedChange,
    language: Language,
    source_file: &str,
) -> CandidatePattern {
    let candidate_id = Uuid::new_v4().to_string()[..8].to_string();
    let generated_at = Utc::now().to_rfc3339();

    // Build the pattern ID
    let pattern_id = format!(
        "learned.{} -> gcp.{}",
        change.source_construct.replace('_', "."),
        change.target_construct.replace("::", ".").replace('_', ".")
    );

    // Build a simple detection query based on the source construct
    let detect_query = build_detection_query(&change.source_construct, language);

    // Build the template from the LLM's output (trimmed)
    let template = change.delta.llm_output.trim().to_string();

    // Build tags
    let tags_str = change
        .suggested_tags
        .iter()
        .map(|t| format!("\"{}\"", t))
        .collect::<Vec<_>>()
        .join(", ");

    // Build import arrays
    let import_add_str = change
        .import_additions
        .iter()
        .map(|i| format!("\"{}\"", i))
        .collect::<Vec<_>>()
        .join(", ");
    let import_remove_str = change
        .import_removals
        .iter()
        .map(|i| format!("\"{}\"", i))
        .collect::<Vec<_>>()
        .join(", ");

    let import_detect = if change.import_removals.is_empty() {
        String::new()
    } else {
        change
            .import_removals
            .iter()
            .map(|i| {
                // Extract module name from import statement
                let module = i
                    .replace("import ", "")
                    .replace("from ", "")
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                format!("\"{}\"", module)
            })
            .collect::<Vec<_>>()
            .join(", ")
    };

    let toml_content = format!(
        r#"# Auto-generated candidate pattern from LLM fallback
# Source file: {source_file}
# Generated: {generated_at}
# Candidate ID: {candidate_id}
# Review status: PENDING
#
# To promote this pattern to the catalogue:
#   cloudshift catalogue promote {candidate_id}
# To reject:
#   cloudshift catalogue reject {candidate_id}

[pattern]
id = "{pattern_id}"
description = "Learned: {source_construct} -> {target_construct}"
source = "{source_cloud}"
language = "{language}"
confidence = {confidence:.2}
tags = [{tags}]

[pattern.detect]
query = '''{detect_query}'''
imports = [{import_detect}]

[pattern.transform]
template = '''{template}'''
import_add = [{import_add}]
import_remove = [{import_remove}]

[pattern.metadata]
origin = "llm-learning"
candidate_id = "{candidate_id}"
generated_at = "{generated_at}"
source_file = "{source_file}"
review_status = "pending"
"#,
        source_file = source_file,
        generated_at = generated_at,
        candidate_id = candidate_id,
        pattern_id = pattern_id,
        source_construct = change.source_construct,
        target_construct = change.target_construct,
        source_cloud = change.source_cloud,
        language = language,
        confidence = change.suggested_confidence,
        tags = tags_str,
        detect_query = detect_query,
        import_detect = import_detect,
        template = template,
        import_add = import_add_str,
        import_remove = import_remove_str,
    );

    // Suggest a filename
    let safe_construct = change
        .source_construct
        .replace(['.', ' ', '/'], "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .to_lowercase();

    let suggested_filename = format!(
        "learned_{}_{}_{}.toml",
        change.source_cloud.to_string().to_lowercase(),
        safe_construct,
        &candidate_id
    );

    CandidatePattern {
        candidate_id,
        toml_content,
        suggested_filename,
        language,
        generated_at,
        source_file: source_file.to_string(),
        change: change.clone(),
    }
}

/// Build a tree-sitter detection query for a given source construct.
fn build_detection_query(construct: &str, language: Language) -> String {
    match language {
        Language::Python => {
            format!(
                r#"
  (call
    function: (attribute
      object: (identifier) @client
      attribute: (identifier) @method (#eq? @method "{}"))
    arguments: (argument_list) @args)"#,
                construct
            )
        }
        Language::TypeScript | Language::JavaScript => {
            format!(
                r#"
  (call_expression
    function: (member_expression
      property: (property_identifier) @method (#eq? @method "{}"))
    arguments: (arguments) @args)"#,
                construct
            )
        }
        Language::Java => {
            format!(
                r#"
  (method_invocation
    name: (identifier) @method (#eq? @method "{}")
    arguments: (argument_list) @args)"#,
                construct
            )
        }
        _ => format!("(identifier) @method (#eq? @method \"{}\")", construct),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::SourceCloud;
    use crate::learning::analyzer::{AnalyzedChange, ChangeType};
    use crate::learning::extractor::LlmDelta;

    fn make_test_change() -> AnalyzedChange {
        AnalyzedChange {
            delta: LlmDelta {
                original_line_start: 5,
                original_line_end: 8,
                pattern_output:
                    "s3.copy_object(Bucket='b', CopySource={'Bucket': 'b', 'Key': 'k'}, Key='dest')"
                        .into(),
                llm_output:
                    "bucket = client.bucket('b')\nsource_blob = bucket.blob('k')\nbucket.copy_blob(source_blob, bucket, 'dest')"
                        .into(),
                context_before: vec!["client = storage.Client()".into()],
                context_after: vec![],
            },
            change_type: ChangeType::MethodCallReplacement,
            source_cloud: SourceCloud::Aws,
            source_construct: "copy_object".into(),
            target_construct: "copy_blob".into(),
            import_additions: vec!["from google.cloud import storage".into()],
            import_removals: vec!["import boto3".into()],
            suggested_confidence: 0.85,
            suggested_tags: vec!["learned".into(), "llm-generated".into(), "s3".into()],
        }
    }

    #[test]
    fn test_generate_candidate_pattern() {
        let change = make_test_change();
        let candidate = generate_candidate_pattern(&change, Language::Python, "test.py");

        assert!(candidate.toml_content.contains("[pattern]"));
        assert!(candidate.toml_content.contains("copy_object"));
        assert!(candidate.toml_content.contains("copy_blob"));
        assert!(candidate.toml_content.contains("llm-learning"));
        assert!(candidate.toml_content.contains("pending"));
        assert!(candidate.suggested_filename.starts_with("learned_aws_"));
        assert!(candidate.suggested_filename.ends_with(".toml"));
    }

    #[test]
    fn test_build_python_detection_query() {
        let query = build_detection_query("put_object", Language::Python);
        assert!(query.contains("#eq? @method \"put_object\""));
        assert!(query.contains("argument_list"));
    }

    #[test]
    fn test_build_typescript_detection_query() {
        let query = build_detection_query("send", Language::TypeScript);
        assert!(query.contains("#eq? @method \"send\""));
        assert!(query.contains("arguments"));
    }
}
