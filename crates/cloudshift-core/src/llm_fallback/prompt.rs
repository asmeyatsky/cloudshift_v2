//! Builds the LLM prompt with context about what was already transformed
//! and what remains.
//!
//! The prompt is structured to give the LLM maximum context:
//! 1. What patterns were already applied (so it doesn't redo work)
//! 2. What cloud references remain (so it knows what to fix)
//! 3. A GCP SDK reference for the target language

use crate::domain::value_objects::{Language, SourceCloud};

use super::detector::RemainingReference;

/// Build the fallback prompt for the LLM.
pub fn build_fallback_prompt(
    transformed_source: &str,
    _original_source: &str,
    language: Language,
    source_cloud: SourceCloud,
    remaining_refs: &[RemainingReference],
    applied_patterns: &[String],
) -> String {
    let lang_name = language.to_string();
    let cloud_name = source_cloud.to_string();

    let remaining_summary: String = remaining_refs
        .iter()
        .map(|r| {
            format!(
                "  Line {}: {} ({:?})",
                r.line_number,
                r.line_content.trim(),
                r.reference_type
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let patterns_summary = if applied_patterns.is_empty() {
        "None -- no deterministic patterns matched.".to_string()
    } else {
        applied_patterns
            .iter()
            .map(|p| format!("  - {}", p))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        r#"You are a cloud migration expert. Complete the migration of this {lang_name} code from {cloud_name} to Google Cloud Platform (GCP).

## Context
A deterministic pattern engine has already partially transformed this code. The following patterns were applied:
{patterns_summary}

However, the code still contains {cloud_name} references that need to be migrated to GCP equivalents:
{remaining_summary}

## Partially Transformed Code
```{lang_name}
{transformed_source}
```

## Instructions
1. Complete the migration by replacing ALL remaining {cloud_name} references with GCP equivalents
2. Ensure the output code is FULLY RUNNABLE on GCP -- no {cloud_name} imports, no {cloud_name} client constructors, no {cloud_name} API calls, no {cloud_name} endpoints or ARNs
3. Use idiomatic GCP SDK patterns for {lang_name}
4. Preserve the code's logic and structure -- only change what's necessary for GCP
5. Return ONLY the complete migrated source code, no explanations

## GCP SDK Reference
- Storage: `from google.cloud import storage` -> `storage.Client().bucket(...).blob(...)`
- Firestore: `from google.cloud import firestore` -> `firestore.Client().collection(...).document(...)`
- Pub/Sub: `from google.cloud import pubsub_v1` -> `pubsub_v1.PublisherClient()`, `pubsub_v1.SubscriberClient()`
- Secret Manager: `from google.cloud import secretmanager` -> `secretmanager.SecretManagerServiceClient()`
- BigQuery: `from google.cloud import bigquery` -> `bigquery.Client().query(...)`
- Vertex AI: `from google.cloud import aiplatform` -> `aiplatform.init()`, `GenerativeModel(...)`
- Vision AI: `from google.cloud import vision` -> `vision.ImageAnnotatorClient()`
- Cloud Functions: `import functions_framework` -> `@functions_framework.http`

Return the complete migrated code:"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_fallback::detector::{ReferenceType, RemainingReference};

    #[test]
    fn prompt_includes_language_and_cloud() {
        let prompt = build_fallback_prompt(
            "import boto3",
            "import boto3",
            Language::Python,
            SourceCloud::Aws,
            &[RemainingReference {
                line_number: 1,
                line_content: "import boto3".to_string(),
                reference_type: ReferenceType::Import,
                cloud: SourceCloud::Aws,
            }],
            &[],
        );
        assert!(prompt.contains("python"));
        assert!(prompt.contains("aws"));
        assert!(prompt.contains("Google Cloud Platform"));
    }

    #[test]
    fn prompt_includes_applied_patterns() {
        let prompt = build_fallback_prompt(
            "source",
            "source",
            Language::Python,
            SourceCloud::Aws,
            &[],
            &[
                "aws.s3.put_object".to_string(),
                "aws.s3.get_object".to_string(),
            ],
        );
        assert!(prompt.contains("aws.s3.put_object"));
        assert!(prompt.contains("aws.s3.get_object"));
    }

    #[test]
    fn prompt_includes_remaining_refs() {
        let prompt = build_fallback_prompt(
            "import boto3",
            "import boto3",
            Language::Python,
            SourceCloud::Aws,
            &[RemainingReference {
                line_number: 1,
                line_content: "import boto3".to_string(),
                reference_type: ReferenceType::Import,
                cloud: SourceCloud::Aws,
            }],
            &[],
        );
        assert!(prompt.contains("Line 1"));
        assert!(prompt.contains("import boto3"));
    }

    #[test]
    fn prompt_shows_no_patterns_when_empty() {
        let prompt = build_fallback_prompt(
            "source",
            "source",
            Language::Python,
            SourceCloud::Aws,
            &[],
            &[],
        );
        assert!(prompt.contains("no deterministic patterns matched"));
    }
}
