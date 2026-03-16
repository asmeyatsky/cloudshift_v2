//! GCP Pattern Catalogue loader (TOML).
//!
//! Architectural Intent:
//! Infrastructure adapter implementing `PatternRepositoryPort` from the domain
//! layer. Loads TOML pattern definition files from the filesystem, compiles
//! them into `CompiledPattern` domain entities, and provides query/search
//! capabilities over the loaded catalogue.
//!
//! The catalogue acts as a read-only repository — patterns are loaded once
//! and then queried repeatedly during transformation.
//!
//! Parallelisation Notes:
//! - Catalogue loading happens once at startup (not on the hot path).
//! - The loaded `Catalogue` is Send + Sync for concurrent read access.

pub mod loader;

pub use loader::{CatalogueLoadError, CatalogueLoadWarning};

use crate::domain::entities::CompiledPattern;
use crate::domain::ports::PatternRepositoryPort;
use crate::domain::value_objects::{Language, PatternId, SourceCloud};
use std::path::Path;

/// GCP pattern catalogue implementing `PatternRepositoryPort`.
///
/// Holds a collection of compiled patterns loaded from TOML files.
/// Provides efficient lookup by language, source cloud, pattern ID, and tags.
#[derive(Debug, Clone)]
pub struct Catalogue {
    patterns: Vec<CompiledPattern>,
    warnings: Vec<CatalogueLoadWarning>,
}

impl Catalogue {
    /// Create an empty catalogue.
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Load a catalogue from a directory of TOML pattern files.
    ///
    /// Recursively walks the directory, compiles all `.toml` files,
    /// and returns the catalogue with any load warnings.
    pub fn from_directory(dir: &Path) -> Result<Self, CatalogueLoadError> {
        let (patterns, warnings) = loader::load_patterns_from_directory(dir)?;
        Ok(Self { patterns, warnings })
    }

    /// Create a catalogue from in-memory TOML strings.
    pub fn from_strings(sources: &[(String, String)]) -> Self {
        let (patterns, warnings) = loader::load_patterns_from_strings(sources);
        Self { patterns, warnings }
    }

    /// Create a catalogue from pre-compiled patterns.
    pub fn from_patterns(patterns: Vec<CompiledPattern>) -> Self {
        Self {
            patterns,
            warnings: Vec::new(),
        }
    }

    /// Add a single compiled pattern to the catalogue.
    pub fn add_pattern(&mut self, pattern: CompiledPattern) {
        self.patterns.push(pattern);
    }

    /// Get warnings generated during catalogue loading.
    pub fn warnings(&self) -> &[CatalogueLoadWarning] {
        &self.warnings
    }

    /// Get all patterns in the catalogue.
    pub fn all_patterns(&self) -> &[CompiledPattern] {
        &self.patterns
    }
}

impl Default for Catalogue {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternRepositoryPort for Catalogue {
    /// Load all compiled patterns matching the given language and source cloud.
    fn get_patterns(&self, language: Language, source: SourceCloud) -> Vec<CompiledPattern> {
        self.patterns
            .iter()
            .filter(|p| {
                p.language == language
                    && (p.source == source
                        || source == SourceCloud::Any
                        || p.source == SourceCloud::Any)
            })
            .cloned()
            .collect()
    }

    /// Find a pattern by its unique ID.
    fn get_by_id(&self, id: &PatternId) -> Option<CompiledPattern> {
        self.patterns.iter().find(|p| &p.id == id).cloned()
    }

    /// Search patterns by tag or description substring.
    fn search(&self, query: &str) -> Vec<CompiledPattern> {
        let query_lower = query.to_lowercase();
        self.patterns
            .iter()
            .filter(|p| {
                p.tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&query_lower))
                    || p.description.to_lowercase().contains(&query_lower)
                    || p.id.as_str().to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect()
    }

    /// Total number of patterns in the catalogue.
    fn count(&self) -> usize {
        self.patterns.len()
    }
}
