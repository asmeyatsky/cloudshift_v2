use crate::domain::entities::*;
use crate::domain::events::DomainEvent;
use crate::domain::value_objects::*;

/// Port: File system access for reading source files.
pub trait FileSystemPort: Send + Sync {
    /// Read a file's content as bytes.
    fn read_file(&self, path: &str) -> Result<Vec<u8>, std::io::Error>;

    /// List files in a directory, recursively.
    fn walk_directory(&self, path: &str) -> Result<Vec<String>, std::io::Error>;

    /// Write content to a file.
    fn write_file(&self, path: &str, content: &[u8]) -> Result<(), std::io::Error>;

    /// Check if a path exists.
    fn exists(&self, path: &str) -> bool;
}

/// Port: Pattern catalogue repository.
pub trait PatternRepositoryPort: Send + Sync {
    /// Load all compiled patterns for a given language and source cloud.
    fn get_patterns(
        &self,
        language: Language,
        source: SourceCloud,
    ) -> Vec<CompiledPattern>;

    /// Find a pattern by its ID.
    fn get_by_id(&self, id: &PatternId) -> Option<CompiledPattern>;

    /// Search patterns by tag.
    fn search(&self, query: &str) -> Vec<CompiledPattern>;

    /// Total number of patterns in the catalogue.
    fn count(&self) -> usize;
}

/// Port: Diff emitter that converts AST changes to unified diffs.
pub trait DiffEmitterPort: Send + Sync {
    /// Generate a unified diff from original source and transformed source.
    fn emit_unified_diff(
        &self,
        path: &str,
        original: &str,
        transformed: &str,
    ) -> String;

    /// Generate a JSON-format diff.
    fn emit_json_diff(
        &self,
        path: &str,
        original: &str,
        transformed: &str,
    ) -> String;

    /// Generate SARIF output for CI integration.
    fn emit_sarif(
        &self,
        results: &[TransformResult],
    ) -> String;
}

/// Port: Event bus for domain event publishing.
pub trait EventBusPort: Send + Sync {
    /// Publish domain events.
    fn publish(&self, events: &[DomainEvent]);
}

/// Port: Semantic analyser that extracts cloud constructs from parsed ASTs.
pub trait SemanticAnalyserPort: Send + Sync {
    /// Analyse source code and extract cloud-relevant constructs.
    fn analyse(
        &self,
        source: &[u8],
        language: Language,
    ) -> Result<Vec<CloudConstruct>, AnalysisError>;
}

/// Port: Pattern matcher that matches compiled patterns against semantic constructs.
pub trait PatternMatcherPort: Send + Sync {
    /// Match patterns against the source code, returning all matches.
    fn match_patterns(
        &self,
        source: &[u8],
        language: Language,
        source_cloud: SourceCloud,
        patterns: &[CompiledPattern],
    ) -> Vec<PatternMatch>;
}

#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("Parse error in {language} file: {message}")]
    ParseError {
        language: Language,
        message: String,
    },
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(Language),
    #[error("Internal error: {0}")]
    Internal(String),
}
