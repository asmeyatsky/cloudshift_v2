//! Python-specific semantic analyser.
//!
//! Detects boto3/botocore imports, S3/DynamoDB/Lambda/SageMaker SDK calls,
//! and AWS environment variable references. Uses tree-sitter-python to walk
//! the AST and extract `CloudConstruct` instances.

use crate::domain::entities::{CloudConstruct, ConstructKind};
use crate::domain::ports::AnalysisError;
use crate::domain::value_objects::{Language, SourceCloud};
use super::treesitter;

/// AWS Python SDK import patterns we look for.
const AWS_IMPORTS: &[&str] = &[
    "boto3",
    "botocore",
    "moto",
    "aws_cdk",
    "awscli",
];

/// Azure Python SDK import patterns we look for.
const AZURE_IMPORTS: &[&str] = &[
    "azure.storage.blob",
    "azure.identity",
    "azure.keyvault",
    "azure.cosmos",
    "azure.servicebus",
    "azure.ai",
    "azure.cognitiveservices",
    "azure.functions",
    "azure.mgmt",
    "azure.core",
];

/// AWS service client method calls we detect.
const AWS_SDK_METHODS: &[&str] = &[
    "put_object",
    "get_object",
    "delete_object",
    "list_objects",
    "list_objects_v2",
    "create_bucket",
    "delete_bucket",
    "put_item",
    "get_item",
    "delete_item",
    "query",
    "scan",
    "update_item",
    "invoke",
    "create_function",
    "update_function_code",
    "create_training_job",
    "create_endpoint",
    "describe_instances",
    "run_instances",
    "send_message",
    "receive_message",
    "publish",
];

/// AWS environment variable names commonly used.
const AWS_ENV_VARS: &[&str] = &[
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "AWS_DEFAULT_REGION",
    "AWS_REGION",
    "AWS_PROFILE",
    "AWS_ENDPOINT_URL",
    "AWS_LAMBDA_FUNCTION_NAME",
    "AWS_EXECUTION_ENV",
];

/// Analyse Python source code for cloud constructs (AWS and Azure).
#[tracing::instrument(skip(source), level = "debug")]
pub fn analyse_python(source: &[u8]) -> Result<Vec<CloudConstruct>, AnalysisError> {
    let tree = treesitter::parse_source(source, Language::Python)?;
    let mut constructs = Vec::new();

    detect_imports(source, &tree, &mut constructs)?;
    detect_sdk_calls(source, &tree, &mut constructs)?;
    detect_env_vars(source, &tree, &mut constructs)?;
    detect_client_creation(source, &tree, &mut constructs)?;
    detect_azure_imports(source, &tree, &mut constructs)?;
    detect_azure_sdk_calls(source, &tree, &mut constructs)?;

    Ok(constructs)
}

/// Detect AWS SDK import statements.
fn detect_imports(
    source: &[u8],
    tree: &tree_sitter::Tree,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (import_statement
          name: (dotted_name) @import_name)
        (import_from_statement
          module_name: (dotted_name) @from_module)
    "#;

    let query = treesitter::compile_query(Language::Python, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        for capture in &m.captures {
            let text = &capture.text;
            let is_aws = AWS_IMPORTS.iter().any(|imp| {
                text.as_str() == *imp || text.starts_with(&format!("{imp}."))
            });

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

/// Detect AWS SDK function/method calls (e.g., `client.put_object(...)`, `s3.get_object(...)`).
fn detect_sdk_calls(
    source: &[u8],
    tree: &tree_sitter::Tree,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (call
          function: (attribute
            object: (_) @client_obj
            attribute: (identifier) @method_name)
          arguments: (argument_list) @args)
    "#;

    let query = treesitter::compile_query(Language::Python, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        for capture in &m.captures {
            if capture.name == "method_name" {
                let method = &capture.text;
                if AWS_SDK_METHODS.contains(&method.as_str()) {
                    // Use the full match span by finding the broadest span in this match
                    let span = broadest_span(m);
                    constructs.push(CloudConstruct {
                        kind: ConstructKind::SdkFunctionCall,
                        source_cloud: SourceCloud::Aws,
                        span,
                        description: format!("AWS SDK call: {method}"),
                        sdk_import: None,
                    });
                }
            }
        }
    }

    Ok(())
}

/// Detect AWS environment variable references (e.g., `os.environ["AWS_REGION"]`).
fn detect_env_vars(
    source: &[u8],
    tree: &tree_sitter::Tree,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (string
          (string_content) @str_content)
    "#;

    let query = treesitter::compile_query(Language::Python, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        for capture in &m.captures {
            let text = &capture.text;
            if AWS_ENV_VARS.contains(&text.as_str()) {
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

/// Detect boto3.client() / boto3.resource() calls to identify which services are used.
fn detect_client_creation(
    source: &[u8],
    tree: &tree_sitter::Tree,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (call
          function: (attribute
            object: (identifier) @boto_obj
            attribute: (identifier) @create_method)
          arguments: (argument_list
            (string
              (string_content) @service_name)))
    "#;

    let query = treesitter::compile_query(Language::Python, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        let boto_name = find_capture(m, "boto_obj");
        let method_name = find_capture(m, "create_method");
        let service_name = find_capture(m, "service_name");

        let is_boto = matches!(boto_name.as_deref(), Some("boto3") | Some("botocore"));
        let is_create =
            matches!(method_name.as_deref(), Some("client") | Some("resource") | Some("Session"));

        if is_boto && is_create {
            if let Some(svc) = &service_name {
                let span = find_capture_span(m, "service_name")
                    .unwrap_or_else(|| broadest_span(m));
                let boto = boto_name.as_deref().unwrap_or("boto3");
                let method = method_name.as_deref().unwrap_or("client");
                constructs.push(CloudConstruct {
                    kind: ConstructKind::SdkFunctionCall,
                    source_cloud: SourceCloud::Aws,
                    span,
                    description: format!(
                        "AWS service client creation: {boto}.{method}('{svc}')"
                    ),
                    sdk_import: Some(format!("{boto}.{method}")),
                });
            }
        }
    }

    Ok(())
}

/// Find a capture value by name within a match.
fn find_capture(m: &treesitter::OwnedMatch, name: &str) -> Option<String> {
    m.captures
        .iter()
        .find(|c| c.name == name)
        .map(|c| c.text.clone())
}

/// Find a capture span by name within a match.
fn find_capture_span(
    m: &treesitter::OwnedMatch,
    name: &str,
) -> Option<crate::domain::value_objects::SourceSpan> {
    m.captures.iter().find(|c| c.name == name).map(|c| c.span)
}

/// Get the broadest span across all captures in a match.
fn broadest_span(m: &treesitter::OwnedMatch) -> crate::domain::value_objects::SourceSpan {
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
        return crate::domain::value_objects::SourceSpan {
            start_byte: 0,
            end_byte: 0,
            start_row: 0,
            start_col: 0,
            end_row: 0,
            end_col: 0,
        };
    }

    crate::domain::value_objects::SourceSpan {
        start_byte,
        end_byte,
        start_row,
        start_col,
        end_row,
        end_col,
    }
}

/// Azure SDK method calls we detect.
const AZURE_SDK_METHODS: &[&str] = &[
    "upload_blob",
    "download_blob",
    "delete_blob",
    "get_blob_client",
    "get_container_client",
    "get_secret",
    "set_secret",
    "create_item",
    "read_item",
    "query_items",
    "send_messages",
    "receive_messages",
];

/// Detect Azure SDK import statements.
fn detect_azure_imports(
    source: &[u8],
    tree: &tree_sitter::Tree,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (import_statement
          name: (dotted_name) @import_name)
        (import_from_statement
          module_name: (dotted_name) @from_module)
    "#;

    let query = treesitter::compile_query(Language::Python, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        for capture in &m.captures {
            let text = &capture.text;
            let is_azure = AZURE_IMPORTS.iter().any(|imp| {
                text.as_str() == *imp || text.starts_with(&format!("{imp}."))
            });

            if is_azure {
                constructs.push(CloudConstruct {
                    kind: ConstructKind::SdkImport,
                    source_cloud: SourceCloud::Azure,
                    span: capture.span,
                    description: format!("Azure SDK import: {text}"),
                    sdk_import: Some(text.clone()),
                });
            }
        }
    }

    Ok(())
}

/// Detect Azure SDK function/method calls (e.g., `blob_client.upload_blob(...)`).
fn detect_azure_sdk_calls(
    source: &[u8],
    tree: &tree_sitter::Tree,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    let query_src = r#"
        (call
          function: (attribute
            object: (_) @client_obj
            attribute: (identifier) @method_name)
          arguments: (argument_list) @args)
    "#;

    let query = treesitter::compile_query(Language::Python, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        for capture in &m.captures {
            if capture.name == "method_name" {
                let method = &capture.text;
                if AZURE_SDK_METHODS.contains(&method.as_str()) {
                    let span = broadest_span(m);
                    constructs.push(CloudConstruct {
                        kind: ConstructKind::SdkFunctionCall,
                        source_cloud: SourceCloud::Azure,
                        span,
                        description: format!("Azure SDK call: {method}"),
                        sdk_import: None,
                    });
                }
            }
        }
    }

    Ok(())
}
