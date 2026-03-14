//! Language-specific semantic extractors (tree-sitter bridge).
//!
//! Architectural Intent:
//! This module is an infrastructure adapter that implements the `SemanticAnalyserPort`
//! from the domain layer. It dispatches analysis by language to specialised sub-modules,
//! each of which uses tree-sitter grammars to walk ASTs and extract `CloudConstruct`
//! instances without embedding any business logic.
//!
//! Supported languages:
//! - Python (boto3/botocore)
//! - TypeScript / JavaScript (aws-sdk-js v2/v3)
//! - Java (AWS SDK v1/v2)
//! - Go (aws-sdk-go v1/v2)
//! - HCL (Terraform AWS provider/resources)
//! - YAML, JSON, Dockerfile (string-based fallback)
//!
//! Parallelisation Notes:
//! - Each file is analysed independently — `SemanticAnalyser` is Send + Sync.
//! - tree-sitter parsers are created per-call (no shared mutable state).

pub mod treesitter;
pub mod python;
pub mod typescript;
pub mod java;
pub mod go_lang;
pub mod hcl;

use crate::domain::entities::CloudConstruct;
use crate::domain::ports::{AnalysisError, SemanticAnalyserPort};
use crate::domain::value_objects::Language;

/// Semantic analyser that dispatches to language-specific extractors.
///
/// Implements `SemanticAnalyserPort` — the domain port for AST-level
/// cloud construct extraction. Zero business logic; only parsing and
/// pattern detection.
#[derive(Debug, Default)]
pub struct SemanticAnalyser;

impl SemanticAnalyser {
    /// Create a new `SemanticAnalyser`.
    pub fn new() -> Self {
        Self
    }
}

impl SemanticAnalyserPort for SemanticAnalyser {
    /// Analyse source code and extract cloud-relevant constructs.
    ///
    /// Dispatches to the appropriate language-specific analyser based on the
    /// `Language` value. For languages without tree-sitter support (Dockerfile),
    /// falls back to string-based detection.
    #[tracing::instrument(skip(self, source), level = "debug")]
    fn analyse(
        &self,
        source: &[u8],
        language: Language,
    ) -> Result<Vec<CloudConstruct>, AnalysisError> {
        match language {
            Language::Python => python::analyse_python(source),
            Language::TypeScript => typescript::analyse_typescript(source, Language::TypeScript),
            Language::JavaScript => typescript::analyse_typescript(source, Language::JavaScript),
            Language::Java => java::analyse_java(source),
            Language::Go => go_lang::analyse_go(source),
            Language::Hcl => hcl::analyse_hcl(source),
            Language::Yaml => analyse_yaml_fallback(source),
            Language::Json => analyse_json_fallback(source),
            Language::Dockerfile => analyse_dockerfile_fallback(source),
        }
    }
}

/// String-based fallback analysis for YAML files.
///
/// Detects AWS service references, ARNs, and region patterns in YAML config files
/// (e.g., CloudFormation templates, SAM templates, GitHub Actions with AWS steps).
fn analyse_yaml_fallback(source: &[u8]) -> Result<Vec<CloudConstruct>, AnalysisError> {
    let text = std::str::from_utf8(source)
        .map_err(|e| AnalysisError::Internal(format!("Invalid UTF-8: {e}")))?;

    let mut constructs = Vec::new();

    for (line_num, line) in text.lines().enumerate() {
        let trimmed = line.trim();

        // Detect AWS ARN references
        if trimmed.contains("arn:aws:") {
            constructs.push(CloudConstruct {
                kind: crate::domain::entities::ConstructKind::ServiceEndpoint,
                source_cloud: crate::domain::value_objects::SourceCloud::Aws,
                span: line_to_span(text, line_num),
                description: format!("AWS ARN reference: {}", truncate(trimmed, 80)),
                sdk_import: None,
            });
        }

        // Detect AWS service type references in CloudFormation
        if (trimmed.starts_with("Type:") || trimmed.starts_with("type:"))
            && trimmed.contains("AWS::")
        {
            constructs.push(CloudConstruct {
                kind: crate::domain::entities::ConstructKind::ResourceDefinition,
                source_cloud: crate::domain::value_objects::SourceCloud::Aws,
                span: line_to_span(text, line_num),
                description: format!("AWS CloudFormation resource: {}", truncate(trimmed, 80)),
                sdk_import: None,
            });
        }

        // Detect AWS region references
        if (trimmed.contains("us-east-1")
            || trimmed.contains("us-west-2")
            || trimmed.contains("eu-west-1")
            || trimmed.contains("ap-southeast-1"))
            && (trimmed.contains("aws") || trimmed.contains("region"))
        {
            constructs.push(CloudConstruct {
                kind: crate::domain::entities::ConstructKind::ConnectionString,
                source_cloud: crate::domain::value_objects::SourceCloud::Aws,
                span: line_to_span(text, line_num),
                description: format!("AWS region reference: {}", truncate(trimmed, 80)),
                sdk_import: None,
            });
        }
    }

    Ok(constructs)
}

/// String-based fallback analysis for JSON files.
///
/// Detects AWS ARNs, service endpoints, and IAM policy documents.
fn analyse_json_fallback(source: &[u8]) -> Result<Vec<CloudConstruct>, AnalysisError> {
    let text = std::str::from_utf8(source)
        .map_err(|e| AnalysisError::Internal(format!("Invalid UTF-8: {e}")))?;

    let mut constructs = Vec::new();

    for (line_num, line) in text.lines().enumerate() {
        let trimmed = line.trim();

        if trimmed.contains("arn:aws:") {
            constructs.push(CloudConstruct {
                kind: crate::domain::entities::ConstructKind::ServiceEndpoint,
                source_cloud: crate::domain::value_objects::SourceCloud::Aws,
                span: line_to_span(text, line_num),
                description: format!("AWS ARN in JSON: {}", truncate(trimmed, 80)),
                sdk_import: None,
            });
        }

        // IAM policy detection
        if (trimmed.contains("\"Effect\"") || trimmed.contains("\"Action\""))
            && (trimmed.contains("Allow")
                || trimmed.contains("Deny")
                || trimmed.contains("s3:")
                || trimmed.contains("iam:"))
        {
            constructs.push(CloudConstruct {
                kind: crate::domain::entities::ConstructKind::IamReference,
                source_cloud: crate::domain::value_objects::SourceCloud::Aws,
                span: line_to_span(text, line_num),
                description: format!("AWS IAM policy element: {}", truncate(trimmed, 80)),
                sdk_import: None,
            });
        }

        // AWS service endpoints
        if trimmed.contains(".amazonaws.com") {
            constructs.push(CloudConstruct {
                kind: crate::domain::entities::ConstructKind::ServiceEndpoint,
                source_cloud: crate::domain::value_objects::SourceCloud::Aws,
                span: line_to_span(text, line_num),
                description: format!("AWS service endpoint: {}", truncate(trimmed, 80)),
                sdk_import: None,
            });
        }
    }

    Ok(constructs)
}

/// String-based fallback analysis for Dockerfiles.
///
/// Detects AWS CLI installation, ECR image references, and AWS environment
/// variable declarations. Dockerfile tree-sitter grammar uses an incompatible
/// tree-sitter version (0.20), so we use string-based detection.
fn analyse_dockerfile_fallback(source: &[u8]) -> Result<Vec<CloudConstruct>, AnalysisError> {
    let text = std::str::from_utf8(source)
        .map_err(|e| AnalysisError::Internal(format!("Invalid UTF-8: {e}")))?;

    let mut constructs = Vec::new();

    for (line_num, line) in text.lines().enumerate() {
        let trimmed = line.trim();

        // FROM instructions with ECR images
        if trimmed.starts_with("FROM") && trimmed.contains(".dkr.ecr.") {
            constructs.push(CloudConstruct {
                kind: crate::domain::entities::ConstructKind::ServiceEndpoint,
                source_cloud: crate::domain::value_objects::SourceCloud::Aws,
                span: line_to_span(text, line_num),
                description: format!("AWS ECR image reference: {}", truncate(trimmed, 80)),
                sdk_import: None,
            });
        }

        // AWS CLI installation
        if trimmed.contains("awscli") || trimmed.contains("aws-cli") {
            constructs.push(CloudConstruct {
                kind: crate::domain::entities::ConstructKind::SdkImport,
                source_cloud: crate::domain::value_objects::SourceCloud::Aws,
                span: line_to_span(text, line_num),
                description: "AWS CLI installation in Dockerfile".to_string(),
                sdk_import: Some("awscli".to_string()),
            });
        }

        // ENV instructions with AWS variables
        if trimmed.starts_with("ENV") {
            let aws_vars = [
                "AWS_ACCESS_KEY_ID",
                "AWS_SECRET_ACCESS_KEY",
                "AWS_DEFAULT_REGION",
                "AWS_REGION",
            ];
            for var in &aws_vars {
                if trimmed.contains(var) {
                    constructs.push(CloudConstruct {
                        kind: crate::domain::entities::ConstructKind::EnvVariable,
                        source_cloud: crate::domain::value_objects::SourceCloud::Aws,
                        span: line_to_span(text, line_num),
                        description: format!("AWS environment variable in Dockerfile: {var}"),
                        sdk_import: None,
                    });
                }
            }
        }
    }

    Ok(constructs)
}

/// Helper: convert a line number to a `SourceSpan`.
fn line_to_span(
    text: &str,
    line_num: usize,
) -> crate::domain::value_objects::SourceSpan {
    let mut byte_offset = 0;
    for (i, line) in text.lines().enumerate() {
        if i == line_num {
            return crate::domain::value_objects::SourceSpan {
                start_byte: byte_offset,
                end_byte: byte_offset + line.len(),
                start_row: line_num,
                start_col: 0,
                end_row: line_num,
                end_col: line.len(),
            };
        }
        byte_offset += line.len() + 1; // +1 for newline
    }
    crate::domain::value_objects::SourceSpan {
        start_byte: 0,
        end_byte: 0,
        start_row: 0,
        start_col: 0,
        end_row: 0,
        end_col: 0,
    }
}

/// Truncate a string to a maximum length, appending "..." if truncated.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}
