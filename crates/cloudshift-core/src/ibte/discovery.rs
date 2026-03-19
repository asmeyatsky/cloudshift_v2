//! SCR discovery — run tree-sitter queries to populate the Stateful Context Registry (PRD §4.8).

use crate::analyser::treesitter;
use crate::domain::ports::AnalysisError;
use crate::domain::value_objects::{Language, SourceSpan};
use crate::ibte::registry::{RegistryEntry, StatefulContextRegistry};
use tree_sitter::Tree;

fn find_capture(
    captures: &[(String, String, SourceSpan)],
    name: &str,
) -> Option<(String, SourceSpan)> {
    captures
        .iter()
        .find(|(n, _, _)| n == name)
        .map(|(_, t, s)| (t.clone(), *s))
}

fn merged_span(captures: &[(String, String, SourceSpan)]) -> SourceSpan {
    let (start, end) = captures
        .iter()
        .map(|(_, _, s)| (s.start_byte, s.end_byte))
        .fold((usize::MAX, 0), |(a, b), (s, e)| (a.min(s), b.max(e)));
    SourceSpan {
        start_byte: start,
        end_byte: end,
        start_row: 0,
        start_col: 0,
        end_row: 0,
        end_col: 0,
    }
}

/// Populate SCR for Python AWS DynamoDB and Azure Blob patterns.
pub fn discover_python(
    source: &[u8],
    tree: &Tree,
    registry: &mut StatefulContextRegistry,
) -> Result<(), AnalysisError> {
    let lang = Language::Python;

    // AWS: assignment left=identifier @client_var, right=call boto3.client('s3'|'sqs'|'sns')
    let aws_client_q = r#"
    (assignment
      left: (identifier) @client_var
      right: (call
        function: (attribute
          object: (identifier) @mod_name (#eq? @mod_name "boto3")
          attribute: (identifier) @method (#eq? @method "client"))
        arguments: (argument_list (string) @service_string))
    )
    "#;
    let q_aws_client = treesitter::compile_query(lang, aws_client_q)?;
    let aws_client_matches = treesitter::run_query(&q_aws_client, tree, source);
    for m in &aws_client_matches {
        let caps: Vec<_> = m
            .captures
            .iter()
            .map(|c| (c.name.clone(), c.text.clone(), c.span))
            .collect();
        if let Some((var, _)) = find_capture(&caps, "client_var") {
            let service = find_capture(&caps, "service_string")
                .map(|(t, _)| t.trim_matches(&['\'', '"'][..]).to_string());
            let span = merged_span(&caps);
            match service.as_deref() {
                Some("s3") => registry.set(var, RegistryEntry::AwsS3Client { span }),
                Some("sqs") => registry.set(var, RegistryEntry::AwsSqsClient { span }),
                Some("sns") => registry.set(var, RegistryEntry::AwsSnsClient { span }),
                _ => {}
            }
        }
    }

    // AWS: assignment left=identifier @client_var, right=call boto3.resource('dynamodb')
    let aws_resource_q = r#"
    (assignment
      left: (identifier) @client_var
      right: (call
        function: (attribute
          object: (identifier) @mod_name (#eq? @mod_name "boto3")
          attribute: (identifier) @method (#eq? @method "resource"))
        arguments: (argument_list (string) @service_string))
    )
    "#;
    let q_aws_res = treesitter::compile_query(lang, aws_resource_q)?;
    let aws_matches = treesitter::run_query(&q_aws_res, tree, source);
    for m in &aws_matches {
        let caps: Vec<_> = m
            .captures
            .iter()
            .map(|c| (c.name.clone(), c.text.clone(), c.span))
            .collect();
        if let Some((var, _)) = find_capture(&caps, "client_var") {
            let service = find_capture(&caps, "service_string")
                .map(|(t, _)| t.trim_matches(&['\'', '"'][..]).to_string());
            if service.as_deref() == Some("dynamodb") {
                let span = merged_span(&caps);
                registry.set(var, RegistryEntry::AwsDynamoDbResource { span });
            }
        }
    }

    // AWS: assignment table_var = dyndb.Table('Orders')
    let aws_table_q = r#"
    (assignment
      left: (identifier) @table_var
      right: (call
        function: (attribute
          object: (identifier) @client_var
          attribute: (identifier) @method (#eq? @method "Table"))
        arguments: (argument_list (string) @table_name))
    )
    "#;
    let q_aws_table = treesitter::compile_query(lang, aws_table_q)?;
    let aws_table_matches = treesitter::run_query(&q_aws_table, tree, source);
    for m in &aws_table_matches {
        let caps: Vec<_> = m
            .captures
            .iter()
            .map(|c| (c.name.clone(), c.text.clone(), c.span))
            .collect();
        if let (Some((table_var, _)), Some((client_var, _)), Some((table_name, _))) = (
            find_capture(&caps, "table_var"),
            find_capture(&caps, "client_var"),
            find_capture(&caps, "table_name"),
        ) {
            let span = merged_span(&caps);
            let table_name = table_name.trim_matches(&['\'', '"'][..]).to_string();
            registry.set(
                table_var,
                RegistryEntry::AwsDynamoDbTable {
                    table_name,
                    parent_var: client_var,
                    span,
                },
            );
        }
    }

    // Azure: client = BlobServiceClient.from_connection_string(...)
    let azure_client_q = r#"
    (assignment
      left: (identifier) @client_var
      right: (call
        function: (attribute
          object: (identifier) @cls
          attribute: (identifier) @method (#eq? @method "from_connection_string"))
        arguments: (_)))
    "#;
    let q_azure_client = treesitter::compile_query(lang, azure_client_q)?;
    let azure_client_matches = treesitter::run_query(&q_azure_client, tree, source);
    for m in &azure_client_matches {
        let caps: Vec<_> = m
            .captures
            .iter()
            .map(|c| (c.name.clone(), c.text.clone(), c.span))
            .collect();
        if let Some((var, _)) = find_capture(&caps, "client_var") {
            let span = merged_span(&caps);
            registry.set(var, RegistryEntry::AzureBlobClient { span });
        }
    }

    // Azure: container = client.get_container_client("assets")
    let azure_cont_q = r#"
    (assignment
      left: (identifier) @cont_var
      right: (call
        function: (attribute
          object: (identifier) @client_var
          attribute: (identifier) @method (#eq? @method "get_container_client"))
        arguments: (argument_list (_) @bucket_arg)))
    "#;
    let q_azure_cont = treesitter::compile_query(lang, azure_cont_q)?;
    let azure_cont_matches = treesitter::run_query(&q_azure_cont, tree, source);
    for m in &azure_cont_matches {
        let caps: Vec<_> = m
            .captures
            .iter()
            .map(|c| (c.name.clone(), c.text.clone(), c.span))
            .collect();
        if let (Some((cont_var, _)), Some((client_var, _)), Some((bucket_name, _))) = (
            find_capture(&caps, "cont_var"),
            find_capture(&caps, "client_var"),
            find_capture(&caps, "bucket_arg"),
        ) {
            let span = merged_span(&caps);
            let container_name = bucket_name.trim_matches('"').trim_matches('\'').to_string();
            registry.set(
                cont_var,
                RegistryEntry::AzureBlobContainer {
                    container_name,
                    parent_var: client_var,
                    span,
                },
            );
        }
    }

    Ok(())
}
