//! Java-specific semantic analyser.
//!
//! Detects AWS SDK v2 imports (software.amazon.awssdk.*), AWS SDK v1 imports
//! (com.amazonaws.*), and AWS-specific service calls. Uses tree-sitter-java
//! for AST analysis.

use crate::domain::entities::{CloudConstruct, ConstructKind};
use crate::domain::ports::AnalysisError;
use crate::domain::value_objects::{Language, SourceCloud, SourceSpan};
use super::treesitter;

/// AWS Java SDK package prefixes.
const AWS_PACKAGE_PREFIXES: &[&str] = &[
    "software.amazon.awssdk",
    "com.amazonaws",
    "com.amazon.sqs",
    "com.amazon.sns",
    "software.amazon.awscdk",
];

/// AWS SDK class names we detect when instantiated or referenced.
const AWS_SDK_CLASSES: &[&str] = &[
    "S3Client",
    "S3AsyncClient",
    "DynamoDbClient",
    "DynamoDbAsyncClient",
    "LambdaClient",
    "LambdaAsyncClient",
    "SqsClient",
    "SqsAsyncClient",
    "SnsClient",
    "SnsAsyncClient",
    "Ec2Client",
    "IamClient",
    "StsClient",
    "KmsClient",
    "SecretsManagerClient",
    "SsmClient",
    "CloudFormationClient",
    "CloudWatchClient",
    "SageMakerClient",
    "AmazonS3",
    "AmazonS3Client",
    "AmazonDynamoDB",
    "AmazonDynamoDBClient",
    "AWSLambda",
    "AWSLambdaClient",
    "AmazonSQS",
    "AmazonSNS",
];

/// AWS SDK method names.
const AWS_METHODS: &[&str] = &[
    "putObject",
    "getObject",
    "deleteObject",
    "listObjects",
    "listObjectsV2",
    "createBucket",
    "putItem",
    "getItem",
    "deleteItem",
    "query",
    "scan",
    "invoke",
    "sendMessage",
    "receiveMessage",
    "publish",
    "createFunction",
    "updateFunctionCode",
    "describeInstances",
    "runInstances",
];

/// Analyse Java source code for AWS cloud constructs.
#[tracing::instrument(skip(source), level = "debug")]
pub fn analyse_java(source: &[u8]) -> Result<Vec<CloudConstruct>, AnalysisError> {
    let tree = treesitter::parse_source(source, Language::Java)?;
    let mut constructs = Vec::new();

    detect_imports(source, &tree, &mut constructs)?;
    detect_sdk_builder_calls(source, &tree, &mut constructs)?;
    detect_method_calls(source, &tree, &mut constructs)?;

    Ok(constructs)
}

/// Detect AWS SDK import declarations.
fn detect_imports(
    source: &[u8],
    tree: &tree_sitter::Tree,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (import_declaration
          (scoped_identifier) @import_path)
    "#;

    let query = treesitter::compile_query(Language::Java, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        for capture in &m.captures {
            let text = &capture.text;
            let is_aws = AWS_PACKAGE_PREFIXES
                .iter()
                .any(|prefix| text.starts_with(prefix));

            if is_aws {
                constructs.push(CloudConstruct {
                    kind: ConstructKind::SdkImport,
                    source_cloud: SourceCloud::Aws,
                    span: capture.span,
                    description: format!("AWS SDK import: {text}"),
                    sdk_import: Some(text.clone()),
                });
            }
        }
    }

    Ok(())
}

/// Detect AWS SDK builder patterns (e.g., `S3Client.builder().build()`).
fn detect_sdk_builder_calls(
    source: &[u8],
    tree: &tree_sitter::Tree,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (method_invocation
          object: (identifier) @class_name
          name: (identifier) @method_name)
    "#;

    let query = treesitter::compile_query(Language::Java, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        let class_name = m
            .captures
            .iter()
            .find(|c| c.name == "class_name")
            .map(|c| c.text.as_str())
            .unwrap_or("");
        let method_name = m
            .captures
            .iter()
            .find(|c| c.name == "method_name")
            .map(|c| c.text.as_str())
            .unwrap_or("");

        if AWS_SDK_CLASSES.contains(&class_name)
            && (method_name == "builder" || method_name == "create")
        {
            let span = m
                .captures
                .iter()
                .find(|c| c.name == "class_name")
                .map(|c| c.span)
                .unwrap_or_else(|| broadest_span(m));

            constructs.push(CloudConstruct {
                kind: ConstructKind::SdkFunctionCall,
                source_cloud: SourceCloud::Aws,
                span,
                description: format!("AWS SDK client creation: {class_name}.{method_name}()"),
                sdk_import: None,
            });
        }
    }

    Ok(())
}

/// Detect AWS SDK method calls on client instances.
fn detect_method_calls(
    source: &[u8],
    tree: &tree_sitter::Tree,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (method_invocation
          object: (_) @receiver
          name: (identifier) @method_name
          arguments: (argument_list) @args)
    "#;

    let query = treesitter::compile_query(Language::Java, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        if let Some(method_capture) = m.captures.iter().find(|c| c.name == "method_name") {
            if AWS_METHODS.contains(&method_capture.text.as_str()) {
                let span = broadest_span(m);
                constructs.push(CloudConstruct {
                    kind: ConstructKind::SdkFunctionCall,
                    source_cloud: SourceCloud::Aws,
                    span,
                    description: format!("AWS SDK call: {}", method_capture.text),
                    sdk_import: None,
                });
            }
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
