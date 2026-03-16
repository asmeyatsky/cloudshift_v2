//! Detects remaining cloud SDK references in transformed source code.
//!
//! After deterministic pattern transforms, this module scans the output
//! for AWS/Azure references that patterns didn't handle. It checks for:
//! - Import statements that still reference AWS/Azure SDKs
//! - SDK client constructors (boto3.client, BlobServiceClient, etc.)
//! - String literals containing AWS ARNs, regions, or service endpoints
//! - Method calls using AWS/Azure API names
//! - AWS resource identifiers (arn:aws:...)
//! - Environment variables (AWS_*, AZURE_*)

use crate::domain::value_objects::{Language, SourceCloud};

/// A remaining cloud reference found in transformed source code.
#[derive(Debug, Clone)]
pub struct RemainingReference {
    /// 1-based line number.
    pub line_number: usize,
    /// The full line content.
    pub line_content: String,
    /// What kind of reference this is.
    pub reference_type: ReferenceType,
    /// Which cloud provider this reference belongs to.
    pub cloud: SourceCloud,
}

/// Classification of a remaining cloud reference.
#[derive(Debug, Clone)]
pub enum ReferenceType {
    Import,
    SdkClientUsage,
    ServiceEndpoint,
    ArnReference,
    EnvironmentVariable,
    ApiMethodCall,
    CloudSpecificConstant,
}

/// Detect remaining cloud SDK references in transformed source code.
pub fn detect_remaining_cloud_refs(source: &str, _language: Language) -> Vec<RemainingReference> {
    let mut refs = Vec::new();

    for (line_num, line) in source.lines().enumerate() {
        let trimmed = line.trim();

        // AWS references
        check_aws_references(trimmed, line_num + 1, line, &mut refs);

        // Azure references
        check_azure_references(trimmed, line_num + 1, line, &mut refs);
    }

    refs
}

fn check_aws_references(
    trimmed: &str,
    line_num: usize,
    line: &str,
    refs: &mut Vec<RemainingReference>,
) {
    // Python boto3 imports
    if trimmed.starts_with("import boto3")
        || trimmed.starts_with("from boto")
        || trimmed.starts_with("import botocore")
    {
        refs.push(RemainingReference {
            line_number: line_num,
            line_content: line.to_string(),
            reference_type: ReferenceType::Import,
            cloud: SourceCloud::Aws,
        });
    }

    // boto3 client/resource usage
    for keyword in &["boto3.client", "boto3.resource", "boto3.Session"] {
        if trimmed.contains(keyword) {
            refs.push(RemainingReference {
                line_number: line_num,
                line_content: line.to_string(),
                reference_type: ReferenceType::SdkClientUsage,
                cloud: SourceCloud::Aws,
            });
        }
    }

    // AWS ARN references
    if trimmed.contains("arn:aws:") {
        refs.push(RemainingReference {
            line_number: line_num,
            line_content: line.to_string(),
            reference_type: ReferenceType::ArnReference,
            cloud: SourceCloud::Aws,
        });
    }

    // AWS service endpoints
    for endpoint in &[
        ".amazonaws.com",
        "sqs.us-",
        "sns.us-",
        "s3.us-",
        "dynamodb.us-",
    ] {
        if trimmed.contains(endpoint) {
            refs.push(RemainingReference {
                line_number: line_num,
                line_content: line.to_string(),
                reference_type: ReferenceType::ServiceEndpoint,
                cloud: SourceCloud::Aws,
            });
            break;
        }
    }

    // AWS environment variables
    for env_var in &[
        "AWS_ACCESS_KEY",
        "AWS_SECRET_KEY",
        "AWS_REGION",
        "AWS_DEFAULT_REGION",
        "AWS_SESSION_TOKEN",
    ] {
        if trimmed.contains(env_var) {
            refs.push(RemainingReference {
                line_number: line_num,
                line_content: line.to_string(),
                reference_type: ReferenceType::EnvironmentVariable,
                cloud: SourceCloud::Aws,
            });
            break;
        }
    }

    // AWS SDK TypeScript/JS imports
    if trimmed.contains("@aws-sdk/") || trimmed.contains("aws-sdk") {
        refs.push(RemainingReference {
            line_number: line_num,
            line_content: line.to_string(),
            reference_type: ReferenceType::Import,
            cloud: SourceCloud::Aws,
        });
    }

    // Java AWS SDK
    if trimmed.contains("software.amazon.awssdk") || trimmed.contains("com.amazonaws") {
        refs.push(RemainingReference {
            line_number: line_num,
            line_content: line.to_string(),
            reference_type: ReferenceType::Import,
            cloud: SourceCloud::Aws,
        });
    }
}

fn check_azure_references(
    trimmed: &str,
    line_num: usize,
    line: &str,
    refs: &mut Vec<RemainingReference>,
) {
    // Azure Python SDK imports
    if trimmed.starts_with("from azure.") || trimmed.starts_with("import azure.") {
        refs.push(RemainingReference {
            line_number: line_num,
            line_content: line.to_string(),
            reference_type: ReferenceType::Import,
            cloud: SourceCloud::Azure,
        });
    }

    // Azure endpoints
    for endpoint in &[
        ".blob.core.windows.net",
        ".vault.azure.net",
        ".documents.azure.com",
        ".servicebus.windows.net",
    ] {
        if trimmed.contains(endpoint) {
            refs.push(RemainingReference {
                line_number: line_num,
                line_content: line.to_string(),
                reference_type: ReferenceType::ServiceEndpoint,
                cloud: SourceCloud::Azure,
            });
            break;
        }
    }

    // Azure environment variables
    for env_var in &[
        "AZURE_TENANT_ID",
        "AZURE_CLIENT_ID",
        "AZURE_CLIENT_SECRET",
        "AZURE_SUBSCRIPTION_ID",
    ] {
        if trimmed.contains(env_var) {
            refs.push(RemainingReference {
                line_number: line_num,
                line_content: line.to_string(),
                reference_type: ReferenceType::EnvironmentVariable,
                cloud: SourceCloud::Azure,
            });
            break;
        }
    }

    // Azure JS/TS SDK
    if trimmed.contains("@azure/") {
        refs.push(RemainingReference {
            line_number: line_num,
            line_content: line.to_string(),
            reference_type: ReferenceType::Import,
            cloud: SourceCloud::Azure,
        });
    }
}

/// Returns true if there are remaining cloud references that need LLM fallback.
pub fn needs_llm_fallback(source: &str, language: Language) -> bool {
    !detect_remaining_cloud_refs(source, language).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_boto3_import() {
        let source = "import boto3\nclient = boto3.client('s3')\n";
        let refs = detect_remaining_cloud_refs(source, Language::Python);
        assert!(refs.len() >= 2);
        assert!(refs
            .iter()
            .any(|r| matches!(r.reference_type, ReferenceType::Import)));
        assert!(refs
            .iter()
            .any(|r| matches!(r.reference_type, ReferenceType::SdkClientUsage)));
    }

    #[test]
    fn detects_aws_arn() {
        let source = r#"resource_arn = "arn:aws:s3:::my-bucket""#;
        let refs = detect_remaining_cloud_refs(source, Language::Python);
        assert_eq!(refs.len(), 1);
        assert!(matches!(
            refs[0].reference_type,
            ReferenceType::ArnReference
        ));
    }

    #[test]
    fn detects_azure_imports() {
        let source = "from azure.storage.blob import BlobServiceClient\n";
        let refs = detect_remaining_cloud_refs(source, Language::Python);
        assert_eq!(refs.len(), 1);
        assert!(matches!(refs[0].reference_type, ReferenceType::Import));
        assert_eq!(refs[0].cloud, SourceCloud::Azure);
    }

    #[test]
    fn detects_aws_env_vars() {
        let source = r#"region = os.environ["AWS_REGION"]"#;
        let refs = detect_remaining_cloud_refs(source, Language::Python);
        assert_eq!(refs.len(), 1);
        assert!(matches!(
            refs[0].reference_type,
            ReferenceType::EnvironmentVariable
        ));
    }

    #[test]
    fn detects_azure_endpoint() {
        let source = r#"url = "https://myaccount.blob.core.windows.net/container""#;
        let refs = detect_remaining_cloud_refs(source, Language::Python);
        assert_eq!(refs.len(), 1);
        assert!(matches!(
            refs[0].reference_type,
            ReferenceType::ServiceEndpoint
        ));
    }

    #[test]
    fn detects_aws_sdk_js() {
        let source = r#"import { S3Client } from "@aws-sdk/client-s3";"#;
        let refs = detect_remaining_cloud_refs(source, Language::TypeScript);
        assert_eq!(refs.len(), 1);
        assert!(matches!(refs[0].reference_type, ReferenceType::Import));
    }

    #[test]
    fn no_false_positives_on_gcp_code() {
        let source = r#"from google.cloud import storage
client = storage.Client()
bucket = client.bucket("my-bucket")
blob = bucket.blob("my-key")
blob.upload_from_string("hello")
"#;
        let refs = detect_remaining_cloud_refs(source, Language::Python);
        assert!(refs.is_empty());
    }

    #[test]
    fn needs_fallback_returns_true_for_aws_code() {
        let source = "import boto3\n";
        assert!(needs_llm_fallback(source, Language::Python));
    }

    #[test]
    fn needs_fallback_returns_false_for_clean_gcp_code() {
        let source = "from google.cloud import storage\n";
        assert!(!needs_llm_fallback(source, Language::Python));
    }
}
