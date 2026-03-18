use crate::domain::events::DomainEvent;
use crate::domain::value_objects::*;
use serde::{Deserialize, Serialize};

/// Result of transforming a single file.
/// Immutable — state changes produce new instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformResult {
    pub path: String,
    pub language: Language,
    pub diff: String,
    /// Full source after transformation (API/UI); empty when unchanged or not computed.
    #[serde(default)]
    pub transformed_source: String,
    pub patterns: Vec<PatternMatch>,
    pub confidence: Confidence,
    pub warnings: Vec<Warning>,
    pub applied: bool,
}

impl TransformResult {
    pub fn new(
        path: String,
        language: Language,
        diff: String,
        patterns: Vec<PatternMatch>,
        confidence: Confidence,
        warnings: Vec<Warning>,
    ) -> Self {
        Self {
            path,
            language,
            diff,
            transformed_source: String::new(),
            patterns,
            confidence,
            warnings,
            applied: false,
        }
    }

    /// Attach canonical post-transform source (avoids client-side diff application).
    pub fn with_transformed_source(mut self, source: impl Into<String>) -> Self {
        self.transformed_source = source.into();
        self
    }

    /// Create a new result with applied=true. Original is unchanged.
    pub fn mark_applied(&self) -> Self {
        Self {
            applied: true,
            ..self.clone()
        }
    }

    pub fn has_changes(&self) -> bool {
        !self.diff.is_empty()
    }

    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

/// A single pattern match within a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatch {
    pub pattern_id: PatternId,
    pub span: SourceSpan,
    pub confidence: Confidence,
    pub source_text: String,
    pub replacement_text: String,
    pub import_add: Vec<String>,
    pub import_remove: Vec<String>,
}

/// Analysis of a single file (pre-transform).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysis {
    pub path: String,
    pub language: Language,
    pub constructs: Vec<CloudConstruct>,
    pub patterns_matched: usize,
    pub confidence: Confidence,
    pub effort: MigrationEffort,
}

/// A detected cloud-relevant construct in source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConstruct {
    pub kind: ConstructKind,
    pub source_cloud: SourceCloud,
    pub span: SourceSpan,
    pub description: String,
    pub sdk_import: Option<String>,
}

/// Kind of cloud construct detected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstructKind {
    SdkImport,
    SdkFunctionCall,
    EnvVariable,
    ResourceDefinition,
    IamReference,
    ConnectionString,
    ServiceEndpoint,
}

/// Report for a full repository transformation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoReport {
    pub path: String,
    pub changes: Vec<FileChange>,
    pub total_constructs: usize,
    pub total_patterns_matched: usize,
    pub average_confidence: Confidence,
    pub overall_effort: MigrationEffort,
    pub domain_events: Vec<DomainEvent>,
}

impl RepoReport {
    pub fn new(path: String, changes: Vec<FileChange>) -> Self {
        let total_constructs: usize = changes.iter().map(|c| c.constructs_detected).sum();
        let total_patterns_matched: usize = changes.iter().map(|c| c.patterns_matched).sum();
        let avg = if changes.is_empty() {
            0.0
        } else {
            changes.iter().map(|c| c.confidence.value()).sum::<f64>() / changes.len() as f64
        };
        let average_confidence = Confidence::new(avg);
        let overall_effort = MigrationEffort::from_confidence(average_confidence);

        Self {
            path,
            changes,
            total_constructs,
            total_patterns_matched,
            average_confidence,
            overall_effort,
            domain_events: Vec::new(),
        }
    }

    /// Return a new report with an added domain event.
    pub fn with_event(mut self, event: DomainEvent) -> Self {
        self.domain_events.push(event);
        self
    }
}

/// Summary of changes to a single file within a repo transform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub file: String,
    pub language: Language,
    pub constructs_detected: usize,
    pub patterns_matched: usize,
    pub confidence: Confidence,
    pub effort: MigrationEffort,
    pub diff: String,
}

/// A compiled transformation pattern from the catalogue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledPattern {
    pub id: PatternId,
    pub description: String,
    pub source: SourceCloud,
    pub language: Language,
    pub confidence: Confidence,
    pub tags: Vec<String>,
    pub detect_query: String,
    pub detect_imports: Vec<String>,
    pub transform_template: String,
    pub import_add: Vec<String>,
    pub import_remove: Vec<String>,
    pub bindings: Vec<PatternBinding>,
}

/// A binding from pattern template variable to AST capture.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternBinding {
    pub variable: String,
    pub capture: String,
}

/// Warning produced during transformation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warning {
    pub message: String,
    pub span: Option<SourceSpan>,
    pub severity: WarningSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarningSeverity {
    Info,
    Warning,
    Error,
}
