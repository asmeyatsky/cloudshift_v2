//! Go-specific semantic analyser.
//!
//! Detects aws-sdk-go (v1 and v2) imports and AWS service client usage.
//! Uses tree-sitter-go for AST analysis.

use crate::domain::entities::{CloudConstruct, ConstructKind};
use crate::domain::ports::AnalysisError;
use crate::domain::value_objects::{Language, SourceCloud, SourceSpan};
use super::treesitter;

/// AWS Go SDK import path patterns.
const AWS_IMPORT_PATTERNS: &[&str] = &[
    "github.com/aws/aws-sdk-go",
    "github.com/aws/aws-sdk-go-v2",
    "github.com/aws/aws-lambda-go",
    "github.com/aws/aws-cdk-go",
    "github.com/aws/constructs-go",
];

/// AWS Go SDK service package names.
const AWS_SERVICE_PACKAGES: &[&str] = &[
    "s3",
    "dynamodb",
    "lambda",
    "sqs",
    "sns",
    "ec2",
    "iam",
    "sts",
    "kms",
    "secretsmanager",
    "ssm",
    "cloudformation",
    "cloudwatch",
    "sagemaker",
];

/// AWS Go SDK method names.
const AWS_GO_METHODS: &[&str] = &[
    "PutObject",
    "GetObject",
    "DeleteObject",
    "ListObjectsV2",
    "CreateBucket",
    "PutItem",
    "GetItem",
    "DeleteItem",
    "Query",
    "Scan",
    "Invoke",
    "SendMessage",
    "ReceiveMessage",
    "Publish",
    "RunInstances",
    "DescribeInstances",
    "CreateFunction",
    "NewFromConfig",
    "New",
];

/// Analyse Go source code for AWS cloud constructs.
#[tracing::instrument(skip(source), level = "debug")]
pub fn analyse_go(source: &[u8]) -> Result<Vec<CloudConstruct>, AnalysisError> {
    let tree = treesitter::parse_source(source, Language::Go)?;
    let mut constructs = Vec::new();

    detect_imports(source, &tree, &mut constructs)?;
    detect_function_calls(source, &tree, &mut constructs)?;
    detect_struct_literals(source, &tree, &mut constructs)?;

    Ok(constructs)
}

/// Detect AWS SDK import paths in Go import statements.
fn detect_imports(
    source: &[u8],
    tree: &tree_sitter::Tree,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (import_spec
          path: (interpreted_string_literal) @import_path)
    "#;

    let query = treesitter::compile_query(Language::Go, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        for capture in &m.captures {
            let text = capture.text.trim_matches('"');
            let is_aws = AWS_IMPORT_PATTERNS.iter().any(|pat| text.starts_with(pat));

            if is_aws {
                constructs.push(CloudConstruct {
                    kind: ConstructKind::SdkImport,
                    source_cloud: SourceCloud::Aws,
                    span: capture.span,
                    description: format!("AWS SDK import: {text}"),
                    sdk_import: Some(text.to_string()),
                });
            }
        }
    }

    Ok(())
}

/// Detect AWS SDK function calls (e.g., `s3.PutObject`, `dynamodb.GetItem`).
fn detect_function_calls(
    source: &[u8],
    tree: &tree_sitter::Tree,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (call_expression
          function: (selector_expression
            operand: (_) @receiver
            field: (field_identifier) @method_name)
          arguments: (argument_list) @args)
    "#;

    let query = treesitter::compile_query(Language::Go, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        let receiver_name = m
            .captures
            .iter()
            .find(|c| c.name == "receiver")
            .map(|c| c.text.as_str())
            .unwrap_or("");
        let method_name = m
            .captures
            .iter()
            .find(|c| c.name == "method_name")
            .map(|c| c.text.as_str())
            .unwrap_or("");

        let is_aws_package = AWS_SERVICE_PACKAGES.contains(&receiver_name);
        let is_aws_method = AWS_GO_METHODS.contains(&method_name);

        if is_aws_package || is_aws_method {
            let span = broadest_span(m);
            constructs.push(CloudConstruct {
                kind: ConstructKind::SdkFunctionCall,
                source_cloud: SourceCloud::Aws,
                span,
                description: format!("AWS SDK call: {receiver_name}.{method_name}"),
                sdk_import: None,
            });
        }
    }

    Ok(())
}

/// Detect AWS SDK struct literals (e.g., `&s3.PutObjectInput{...}`).
fn detect_struct_literals(
    source: &[u8],
    tree: &tree_sitter::Tree,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (composite_literal
          type: (qualified_type
            package: (package_identifier) @pkg_name
            name: (type_identifier) @type_name))
    "#;

    let query = treesitter::compile_query(Language::Go, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        let pkg_name = m
            .captures
            .iter()
            .find(|c| c.name == "pkg_name")
            .map(|c| c.text.as_str())
            .unwrap_or("");
        let type_name = m
            .captures
            .iter()
            .find(|c| c.name == "type_name")
            .map(|c| c.text.as_str())
            .unwrap_or("");

        if AWS_SERVICE_PACKAGES.contains(&pkg_name) {
            let span = m
                .captures
                .iter()
                .find(|c| c.name == "type_name")
                .map(|c| c.span)
                .unwrap_or_else(|| broadest_span(m));

            constructs.push(CloudConstruct {
                kind: ConstructKind::SdkFunctionCall,
                source_cloud: SourceCloud::Aws,
                span,
                description: format!("AWS SDK struct: {pkg_name}.{type_name}"),
                sdk_import: None,
            });
        }
    }

    Ok(())
}

/// Get the broadest span across all captures in a match.
fn broadest_span(m: &treesitter::OwnedMatch) -> SourceSpan {
    let mut start_byte = usize::MAX;
    let mut end_byte = 0;
    let mut start_row = usize::MAX;
    let mut start_col = 0;
    let mut end_row = 0;
    let mut end_col = 0;

    for capture in &m.captures {
        if capture.span.start_byte < start_byte {
            start_byte = capture.span.start_byte;
            start_row = capture.span.start_row;
            start_col = capture.span.start_col;
        }
        if capture.span.end_byte > end_byte {
            end_byte = capture.span.end_byte;
            end_row = capture.span.end_row;
            end_col = capture.span.end_col;
        }
    }

    if start_byte == usize::MAX {
        return SourceSpan {
            start_byte: 0, end_byte: 0, start_row: 0, start_col: 0, end_row: 0, end_col: 0,
        };
    }

    SourceSpan { start_byte, end_byte, start_row, start_col, end_row, end_col }
}
