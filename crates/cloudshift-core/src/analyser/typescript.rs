//! TypeScript-specific semantic analyser.
//!
//! Detects aws-sdk-js imports (v2 and v3), S3Client/DynamoDBClient calls,
//! and AWS-specific patterns in TypeScript/JavaScript source code.
//! Uses tree-sitter-typescript for AST analysis.

use super::treesitter;
use crate::domain::entities::{CloudConstruct, ConstructKind};
use crate::domain::ports::AnalysisError;
use crate::domain::value_objects::{Language, SourceCloud, SourceSpan};

/// AWS SDK v2 and v3 package patterns.
const AWS_IMPORT_PATTERNS: &[&str] = &[
    "aws-sdk",
    "@aws-sdk/",
    "@aws-cdk/",
    "aws-cdk-lib",
    "aws-amplify",
    "@aws-amplify/",
    "amazon-cognito-identity-js",
];

/// AWS SDK class/type names commonly instantiated.
const AWS_SDK_CLASSES: &[&str] = &[
    "S3Client",
    "S3",
    "DynamoDBClient",
    "DynamoDB",
    "LambdaClient",
    "Lambda",
    "SQSClient",
    "SQS",
    "SNSClient",
    "SNS",
    "EC2Client",
    "EC2",
    "IAMClient",
    "IAM",
    "CloudFormation",
    "CloudWatch",
    "SageMaker",
    "SageMakerRuntime",
    "SecretsManager",
    "SSM",
    "KMS",
    "STS",
    "CognitoIdentityServiceProvider",
    "CognitoIdentity",
];

/// AWS SDK method names on client objects.
const AWS_SDK_METHODS: &[&str] = &[
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
    "send",
    "publish",
    "sendMessage",
    "receiveMessage",
];

/// Analyse TypeScript source code for AWS cloud constructs.
#[tracing::instrument(skip(source), level = "debug")]
pub fn analyse_typescript(
    source: &[u8],
    lang: Language,
) -> Result<Vec<CloudConstruct>, AnalysisError> {
    let tree = treesitter::parse_source(source, lang)?;
    let mut constructs = Vec::new();

    detect_imports(source, &tree, lang, &mut constructs)?;
    detect_sdk_instantiation(source, &tree, lang, &mut constructs)?;
    detect_sdk_calls(source, &tree, lang, &mut constructs)?;
    detect_env_vars(source, &tree, lang, &mut constructs)?;

    Ok(constructs)
}

/// Detect AWS SDK import/require statements.
fn detect_imports(
    source: &[u8],
    tree: &tree_sitter::Tree,
    lang: Language,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (import_statement
          source: (string) @import_source)
        (call_expression
          function: (identifier) @require_fn
          arguments: (arguments
            (string) @require_source))
    "#;

    let query = treesitter::compile_query(lang, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        // Handle ES6 imports
        if let Some(capture) = m.captures.iter().find(|c| c.name == "import_source") {
            let text = capture
                .text
                .trim_matches(|c| c == '\'' || c == '"' || c == '`');
            if AWS_IMPORT_PATTERNS.iter().any(|pat| text.contains(pat)) {
                constructs.push(CloudConstruct {
                    kind: ConstructKind::SdkImport,
                    source_cloud: SourceCloud::Aws,
                    span: capture.span,
                    description: format!("AWS SDK import: {text}"),
                    sdk_import: Some(text.to_string()),
                });
            }
        }

        // Handle require() calls
        let has_require = m
            .captures
            .iter()
            .any(|c| c.name == "require_fn" && c.text == "require");
        if has_require {
            if let Some(capture) = m.captures.iter().find(|c| c.name == "require_source") {
                let text = capture
                    .text
                    .trim_matches(|c| c == '\'' || c == '"' || c == '`');
                if AWS_IMPORT_PATTERNS.iter().any(|pat| text.contains(pat)) {
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
    }

    Ok(())
}

/// Detect `new S3Client(...)` / `new AWS.S3(...)` instantiation.
fn detect_sdk_instantiation(
    source: &[u8],
    tree: &tree_sitter::Tree,
    lang: Language,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (new_expression
          constructor: (identifier) @class_name)
        (new_expression
          constructor: (member_expression
            object: (_) @ns_obj
            property: (property_identifier) @ns_class))
    "#;

    let query = treesitter::compile_query(lang, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        for capture in &m.captures {
            if (capture.name == "class_name" || capture.name == "ns_class")
                && AWS_SDK_CLASSES.contains(&capture.text.as_str())
            {
                constructs.push(CloudConstruct {
                    kind: ConstructKind::SdkFunctionCall,
                    source_cloud: SourceCloud::Aws,
                    span: capture.span,
                    description: format!("AWS SDK instantiation: new {}(...)", capture.text),
                    sdk_import: None,
                });
            }
        }
    }

    Ok(())
}

/// Detect AWS SDK method calls on client objects.
fn detect_sdk_calls(
    source: &[u8],
    tree: &tree_sitter::Tree,
    lang: Language,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (call_expression
          function: (member_expression
            object: (_) @client_obj
            property: (property_identifier) @method_name)
          arguments: (arguments) @args)
    "#;

    let query = treesitter::compile_query(lang, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        if let Some(method_capture) = m.captures.iter().find(|c| c.name == "method_name") {
            if AWS_SDK_METHODS.contains(&method_capture.text.as_str()) {
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

/// Detect AWS environment variable references.
fn detect_env_vars(
    source: &[u8],
    tree: &tree_sitter::Tree,
    lang: Language,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (string) @string_lit
    "#;

    let query = treesitter::compile_query(lang, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    let env_vars = [
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
        "AWS_SESSION_TOKEN",
        "AWS_DEFAULT_REGION",
        "AWS_REGION",
    ];

    for m in &matches {
        for capture in &m.captures {
            let text = capture
                .text
                .trim_matches(|c| c == '\'' || c == '"' || c == '`');
            if env_vars.contains(&text) {
                constructs.push(CloudConstruct {
                    kind: ConstructKind::EnvVariable,
                    source_cloud: SourceCloud::Aws,
                    span: capture.span,
                    description: format!("AWS environment variable: {text}"),
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
            start_byte: 0,
            end_byte: 0,
            start_row: 0,
            start_col: 0,
            end_row: 0,
            end_col: 0,
        };
    }

    SourceSpan {
        start_byte,
        end_byte,
        start_row,
        start_col,
        end_row,
        end_col,
    }
}
