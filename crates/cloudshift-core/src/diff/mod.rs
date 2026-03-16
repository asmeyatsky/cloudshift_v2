//! AST diff to unified diff emitter.
//!
//! Architectural Intent:
//! Infrastructure adapter implementing `DiffEmitterPort` from the domain layer.
//! Converts AST-level transformations (represented as original/transformed text
//! pairs) into human-readable unified diffs, structured JSON diffs, and SARIF
//! output for CI integration.
//!
//! Uses the `similar` crate for line-level diffing with configurable context.
//!
//! Parallelisation Notes:
//! - `DiffGenerator` is stateless and Send + Sync.
//! - Diff generation per file is independent and parallelisable.

pub mod emitter;

use crate::domain::entities::TransformResult;
use crate::domain::ports::DiffEmitterPort;

pub use emitter::ChangeSummary;

/// Diff generator implementing the `DiffEmitterPort` trait.
///
/// Stateless adapter — safe to share across threads.
#[derive(Debug, Default, Clone)]
pub struct DiffGenerator;

impl DiffGenerator {
    /// Create a new `DiffGenerator`.
    pub fn new() -> Self {
        Self
    }

    /// Get a summary of changes between original and transformed text.
    pub fn summarize(&self, original: &str, transformed: &str) -> ChangeSummary {
        emitter::change_summary(original, transformed)
    }
}

impl DiffEmitterPort for DiffGenerator {
    /// Generate a unified diff from original and transformed source.
    #[tracing::instrument(skip(self, original, transformed), level = "debug")]
    fn emit_unified_diff(&self, path: &str, original: &str, transformed: &str) -> String {
        emitter::unified_diff(path, original, transformed)
    }

    /// Generate a JSON-format diff.
    #[tracing::instrument(skip(self, original, transformed), level = "debug")]
    fn emit_json_diff(&self, path: &str, original: &str, transformed: &str) -> String {
        emitter::json_diff(path, original, transformed)
    }

    /// Generate SARIF output for CI integration.
    #[tracing::instrument(skip(self, results), level = "debug")]
    fn emit_sarif(&self, results: &[TransformResult]) -> String {
        emitter::sarif_output(results)
    }
}
