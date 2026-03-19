//! Stateful Context Registry (SCR) — PRD Supplement §4.8.
//!
//! File-scoped metadata store that tracks variable lineage (e.g. Client -> Table -> Operation).
//! When a "Provider Entry Point" (e.g. boto3.resource) is detected, the engine creates a registry
//! entry for that variable. Re-assignment of the same variable overwrites (AC 4.8.1).

use crate::domain::value_objects::SourceSpan;
use std::collections::HashMap;

/// Kind of entry in the SCR (provider-specific).
#[derive(Debug, Clone)]
pub enum RegistryEntry {
    /// AWS: boto3.resource('dynamodb') -> variable is DynamoDB service root.
    AwsDynamoDbResource { span: SourceSpan },
    /// AWS: dyndb.Table('Orders') -> variable is collection/table name.
    AwsDynamoDbTable {
        table_name: String,
        parent_var: String,
        span: SourceSpan,
    },
    /// Azure: BlobServiceClient.from_connection_string(...) -> variable is blob client.
    AzureBlobClient { span: SourceSpan },
    /// Azure: client.get_container_client("assets") -> variable is bucket/container.
    AzureBlobContainer {
        container_name: String,
        parent_var: String,
        span: SourceSpan,
    },
}

/// File-scoped Stateful Context Registry.
#[derive(Debug, Default)]
pub struct StatefulContextRegistry {
    /// Variable name -> entry. Re-assignment overwrites (no leak from previous definition).
    entries: HashMap<String, RegistryEntry>,
}

impl StatefulContextRegistry {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Register or overwrite entry for a variable (AC 4.8.1: re-assignment without leaking).
    pub fn set(&mut self, var_name: String, entry: RegistryEntry) {
        self.entries.insert(var_name, entry);
    }

    pub fn get(&self, var_name: &str) -> Option<&RegistryEntry> {
        self.entries.get(var_name)
    }

    /// Return the table name if this variable is an AWS DynamoDB Table.
    pub fn get_dynamodb_table(&self, var_name: &str) -> Option<(&str, &str)> {
        match self.get(var_name)? {
            RegistryEntry::AwsDynamoDbTable {
                table_name,
                parent_var,
                ..
            } => Some((table_name.as_str(), parent_var.as_str())),
            _ => None,
        }
    }

    /// Return the container name if this variable is an Azure Blob container.
    pub fn get_azure_container(&self, var_name: &str) -> Option<(&str, &str)> {
        match self.get(var_name)? {
            RegistryEntry::AzureBlobContainer {
                container_name,
                parent_var,
                ..
            } => Some((container_name.as_str(), parent_var.as_str())),
            _ => None,
        }
    }

    /// For a table variable (e.g. from put_item object), return resource span, table span, and table name.
    pub fn dynamodb_chain_spans(
        &self,
        table_var: &str,
    ) -> Option<(SourceSpan, SourceSpan, String)> {
        let table_entry = self.get(table_var)?;
        let RegistryEntry::AwsDynamoDbTable {
            table_name,
            parent_var,
            span: table_span,
        } = table_entry
        else {
            return None;
        };
        let parent_entry = self.get(parent_var)?;
        let RegistryEntry::AwsDynamoDbResource {
            span: resource_span,
        } = parent_entry
        else {
            return None;
        };
        Some((*resource_span, *table_span, table_name.clone()))
    }

    /// For a container variable (e.g. from upload_blob object), return client span, container span, and bucket name.
    pub fn azure_blob_chain_spans(
        &self,
        container_var: &str,
    ) -> Option<(SourceSpan, SourceSpan, String)> {
        let cont_entry = self.get(container_var)?;
        let RegistryEntry::AzureBlobContainer {
            container_name,
            parent_var,
            span: container_span,
        } = cont_entry
        else {
            return None;
        };
        let parent_entry = self.get(parent_var)?;
        let RegistryEntry::AzureBlobClient { span: client_span } = parent_entry else {
            return None;
        };
        Some((*client_span, *container_span, container_name.clone()))
    }
}

impl RegistryEntry {
    pub fn span(&self) -> &SourceSpan {
        match self {
            RegistryEntry::AwsDynamoDbResource { span }
            | RegistryEntry::AwsDynamoDbTable { span, .. }
            | RegistryEntry::AzureBlobClient { span }
            | RegistryEntry::AzureBlobContainer { span, .. } => span,
        }
    }
}
