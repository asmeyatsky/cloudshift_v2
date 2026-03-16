//! HCL-specific semantic analyser.
//!
//! Detects AWS provider blocks, AWS resource types (aws_s3_bucket, aws_lambda_function,
//! aws_dynamodb_table, etc.), and AWS data source references in Terraform/HCL files.
//! Uses tree-sitter-hcl for AST analysis.

use super::treesitter;
use crate::domain::entities::{CloudConstruct, ConstructKind};
use crate::domain::ports::AnalysisError;
use crate::domain::value_objects::{Language, SourceCloud};

/// AWS Terraform resource type prefix.
const AWS_RESOURCE_PREFIX: &str = "aws_";

/// Well-known AWS resource types for higher-confidence detection.
const AWS_KNOWN_RESOURCES: &[&str] = &[
    "aws_s3_bucket",
    "aws_s3_bucket_policy",
    "aws_s3_bucket_object",
    "aws_s3_object",
    "aws_dynamodb_table",
    "aws_dynamodb_table_item",
    "aws_lambda_function",
    "aws_lambda_permission",
    "aws_lambda_layer_version",
    "aws_sqs_queue",
    "aws_sns_topic",
    "aws_sns_topic_subscription",
    "aws_ec2_instance",
    "aws_instance",
    "aws_security_group",
    "aws_vpc",
    "aws_subnet",
    "aws_iam_role",
    "aws_iam_policy",
    "aws_iam_role_policy_attachment",
    "aws_iam_user",
    "aws_iam_group",
    "aws_kms_key",
    "aws_kms_alias",
    "aws_secretsmanager_secret",
    "aws_ssm_parameter",
    "aws_cloudwatch_log_group",
    "aws_cloudwatch_metric_alarm",
    "aws_cloudformation_stack",
    "aws_sagemaker_endpoint",
    "aws_sagemaker_model",
    "aws_ecs_cluster",
    "aws_ecs_service",
    "aws_ecs_task_definition",
    "aws_eks_cluster",
    "aws_rds_cluster",
    "aws_db_instance",
    "aws_elasticache_cluster",
    "aws_api_gateway_rest_api",
    "aws_apigatewayv2_api",
    "aws_lb",
    "aws_lb_target_group",
    "aws_lb_listener",
    "aws_route53_zone",
    "aws_route53_record",
    "aws_cloudfront_distribution",
    "aws_cognito_user_pool",
    "aws_sts_caller_identity",
];

/// Analyse HCL/Terraform source code for AWS cloud constructs.
#[tracing::instrument(skip(source), level = "debug")]
pub fn analyse_hcl(source: &[u8]) -> Result<Vec<CloudConstruct>, AnalysisError> {
    let tree = treesitter::parse_source(source, Language::Hcl)?;
    let mut constructs = Vec::new();

    detect_blocks(source, &tree, &mut constructs)?;

    Ok(constructs)
}

/// Detect HCL blocks (provider, resource, data) with AWS types.
fn detect_blocks(
    source: &[u8],
    tree: &tree_sitter::Tree,
    constructs: &mut Vec<CloudConstruct>,
) -> Result<(), AnalysisError> {
    // Match blocks: identifier string_lit [string_lit] body
    let query_src = r#"
        (block
          (identifier) @block_type
          (string_lit) @first_label)
    "#;

    let query = treesitter::compile_query(Language::Hcl, query_src)?;
    let matches = treesitter::run_query(&query, tree, source);

    for m in &matches {
        let block_type = m
            .captures
            .iter()
            .find(|c| c.name == "block_type")
            .map(|c| c.text.as_str())
            .unwrap_or("");
        let first_label_capture = m.captures.iter().find(|c| c.name == "first_label");

        let first_label = first_label_capture
            .map(|c| c.text.trim_matches('"'))
            .unwrap_or("");

        match block_type {
            "provider" if first_label == "aws" => {
                if let Some(capture) = first_label_capture {
                    constructs.push(CloudConstruct {
                        kind: ConstructKind::ResourceDefinition,
                        source_cloud: SourceCloud::Aws,
                        span: capture.span,
                        description: "AWS provider block".to_string(),
                        sdk_import: Some("provider.aws".to_string()),
                    });
                }
            }
            "resource" if first_label.starts_with(AWS_RESOURCE_PREFIX) => {
                if let Some(capture) = first_label_capture {
                    let is_known = AWS_KNOWN_RESOURCES.contains(&first_label);
                    let kind = if first_label.contains("iam") {
                        ConstructKind::IamReference
                    } else {
                        ConstructKind::ResourceDefinition
                    };

                    constructs.push(CloudConstruct {
                        kind,
                        source_cloud: SourceCloud::Aws,
                        span: capture.span,
                        description: format!(
                            "AWS resource: {first_label}{}",
                            if is_known { "" } else { " (unknown type)" }
                        ),
                        sdk_import: Some(first_label.to_string()),
                    });
                }
            }
            "data" if first_label.starts_with(AWS_RESOURCE_PREFIX) => {
                if let Some(capture) = first_label_capture {
                    constructs.push(CloudConstruct {
                        kind: ConstructKind::ResourceDefinition,
                        source_cloud: SourceCloud::Aws,
                        span: capture.span,
                        description: format!("AWS data source: {first_label}"),
                        sdk_import: Some(first_label.to_string()),
                    });
                }
            }
            _ => {}
        }
    }

    Ok(())
}
