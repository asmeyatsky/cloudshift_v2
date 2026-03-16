//! Pattern DSL compiler.
//!
//! Compiles TOML pattern definitions into `CompiledPattern` instances.
//! Parses the [pattern], [pattern.detect], [pattern.transform], and
//! [pattern.bindings] sections from TOML files as specified in PRD section 4.6.
//!
//! Architectural Intent:
//! Pure transformation — takes TOML text and produces domain entities.
//! No I/O, no side effects, no business logic.

use crate::domain::entities::{CompiledPattern, PatternBinding};
use crate::domain::value_objects::{Confidence, Language, PatternId, SourceCloud};
use serde::Deserialize;

/// Error type for pattern compilation failures.
#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("TOML parse error: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid source cloud value: {0}")]
    InvalidSourceCloud(String),

    #[error("Invalid language value: {0}")]
    InvalidLanguage(String),

    #[error("Invalid confidence value: {0}")]
    InvalidConfidence(f64),
}

/// Raw TOML structure for a pattern definition file.
#[derive(Debug, Deserialize)]
struct RawPatternFile {
    pattern: RawPattern,
}

#[derive(Debug, Deserialize)]
struct RawPattern {
    id: String,
    #[serde(default)]
    description: Option<String>,
    source: String,
    language: String,
    confidence: f64,
    #[serde(default)]
    tags: Vec<String>,
    detect: RawDetect,
    transform: RawTransform,
    #[serde(default)]
    bindings: Option<toml::value::Table>,
}

#[derive(Debug, Deserialize)]
struct RawDetect {
    query: String,
    #[serde(default)]
    imports: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawTransform {
    template: String,
    #[serde(default)]
    import_add: Vec<String>,
    #[serde(default)]
    import_remove: Vec<String>,
}

/// Compile a TOML pattern definition string into a `CompiledPattern`.
///
/// The expected format follows PRD section 4.6:
/// ```toml
/// [pattern]
/// id = "aws.s3.put_object -> gcp.gcs.blob.upload"
/// source = "aws"
/// language = "python"
/// confidence = 0.97
/// tags = ["storage", "s3", "gcs"]
///
/// [pattern.detect]
/// query = '''...'''
/// imports = ["boto3"]
///
/// [pattern.transform]
/// template = '''...'''
/// import_add = ["from google.cloud import storage"]
/// import_remove = ["import boto3"]
///
/// [pattern.bindings]
/// bucket_var = "args.Bucket"
/// key_expr = "args.Key"
/// ```
#[tracing::instrument(skip(toml_source), level = "debug")]
pub fn compile_pattern(toml_source: &str) -> Result<CompiledPattern, CompileError> {
    let raw: RawPatternFile = toml::from_str(toml_source)?;
    let p = raw.pattern;

    // Parse source cloud
    let source: SourceCloud = p
        .source
        .parse()
        .map_err(|_| CompileError::InvalidSourceCloud(p.source.clone()))?;

    // Parse language
    let language = parse_language(&p.language)?;

    // Validate confidence
    if !(0.0..=1.0).contains(&p.confidence) {
        return Err(CompileError::InvalidConfidence(p.confidence));
    }
    let confidence = Confidence::new(p.confidence);

    // Parse bindings
    let bindings = match p.bindings {
        Some(table) => table
            .into_iter()
            .map(|(variable, value)| PatternBinding {
                variable,
                capture: value.as_str().unwrap_or("").to_string(),
            })
            .collect(),
        None => Vec::new(),
    };

    let description = p.description.unwrap_or_else(|| p.id.clone());

    Ok(CompiledPattern {
        id: PatternId::new(&p.id),
        description,
        source,
        language,
        confidence,
        tags: p.tags,
        detect_query: p.detect.query,
        detect_imports: p.detect.imports,
        transform_template: p.transform.template,
        import_add: p.transform.import_add,
        import_remove: p.transform.import_remove,
        bindings,
    })
}

/// Parse a language string into a `Language` value.
fn parse_language(s: &str) -> Result<Language, CompileError> {
    match s.to_lowercase().as_str() {
        "python" => Ok(Language::Python),
        "typescript" => Ok(Language::TypeScript),
        "javascript" => Ok(Language::JavaScript),
        "java" => Ok(Language::Java),
        "go" => Ok(Language::Go),
        "hcl" | "terraform" => Ok(Language::Hcl),
        "yaml" => Ok(Language::Yaml),
        "dockerfile" | "docker" => Ok(Language::Dockerfile),
        "json" => Ok(Language::Json),
        other => Err(CompileError::InvalidLanguage(other.to_string())),
    }
}

/// Compile multiple TOML pattern definitions, collecting results and errors.
///
/// Returns all successfully compiled patterns and a list of errors for
/// patterns that failed to compile. This is useful for catalogue loading
/// where a single bad pattern file should not prevent other patterns from loading.
pub fn compile_patterns(
    sources: &[(String, String)],
) -> (Vec<CompiledPattern>, Vec<(String, CompileError)>) {
    let mut patterns = Vec::new();
    let mut errors = Vec::new();

    for (name, toml_source) in sources {
        match compile_pattern(toml_source) {
            Ok(pattern) => patterns.push(pattern),
            Err(err) => errors.push((name.clone(), err)),
        }
    }

    (patterns, errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_PATTERN: &str = r#"
[pattern]
id = "aws.s3.put_object -> gcp.gcs.blob.upload"
source = "aws"
language = "python"
confidence = 0.97
tags = ["storage", "s3", "gcs"]

[pattern.detect]
query = '''
  (call
    function: (attribute
      object: (identifier) @client
      attribute: (identifier) @method)
    arguments: (argument_list) @args)
'''
imports = ["boto3"]

[pattern.transform]
template = '''
  {bucket_var}.blob({key_expr}).upload_from_string({body_expr})
'''
import_add = ["from google.cloud import storage"]
import_remove = ["import boto3"]

[pattern.bindings]
bucket_var = "args.Bucket"
key_expr = "args.Key"
body_expr = "args.Body"
"#;

    #[test]
    fn compile_valid_pattern() {
        let result = compile_pattern(SAMPLE_PATTERN);
        assert!(result.is_ok(), "Expected Ok, got {:?}", result.err());

        let pattern = result.unwrap();
        assert_eq!(
            pattern.id.as_str(),
            "aws.s3.put_object -> gcp.gcs.blob.upload"
        );
        assert_eq!(pattern.source, SourceCloud::Aws);
        assert_eq!(pattern.language, Language::Python);
        assert!((pattern.confidence.value() - 0.97).abs() < 0.001);
        assert_eq!(pattern.tags, vec!["storage", "s3", "gcs"]);
        assert_eq!(pattern.detect_imports, vec!["boto3"]);
        assert_eq!(pattern.import_add, vec!["from google.cloud import storage"]);
        assert_eq!(pattern.import_remove, vec!["import boto3"]);
        assert_eq!(pattern.bindings.len(), 3);
    }

    #[test]
    fn compile_invalid_toml() {
        let result = compile_pattern("not valid toml {{{{");
        assert!(result.is_err());
    }

    #[test]
    fn compile_pattern_with_target_and_notes() {
        let toml = r#"
[pattern]
id = "aws.s3.create_bucket -> gcp.gcs.client.create_bucket"
source = "aws"
target = "gcp"
language = "python"
confidence = 0.93
tags = ["storage"]

[pattern.detect]
query = "(identifier)"
imports = ["boto3"]

[pattern.transform]
template = "x"
import_add = ["from google.cloud import storage"]
import_remove = ["import boto3"]

[pattern.bindings]
bucket_name_expr = "args.Bucket"

[pattern.notes]
description = "Some migration notes here."
"#;
        let result = compile_pattern(toml);
        assert!(
            result.is_ok(),
            "Pattern with target and notes should compile, got: {:?}",
            result.err()
        );
    }

    #[test]
    fn compile_real_pattern_toml_file() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let pattern_path = root.join("patterns/python/aws_s3_create_bucket.toml");
        let content = std::fs::read_to_string(&pattern_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", pattern_path.display(), e));
        let result = compile_pattern(&content);
        assert!(
            result.is_ok(),
            "Real pattern file should compile, got: {:?}",
            result.err()
        );
    }

    #[test]
    fn compile_invalid_confidence() {
        let toml = r#"
[pattern]
id = "test"
source = "aws"
language = "python"
confidence = 2.5

[pattern.detect]
query = "(identifier)"

[pattern.transform]
template = "x"
"#;
        let result = compile_pattern(toml);
        assert!(matches!(result, Err(CompileError::InvalidConfidence(_))));
    }
}
