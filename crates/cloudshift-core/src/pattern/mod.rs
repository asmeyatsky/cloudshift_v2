//! Pattern DSL compiler and matcher.
//!
//! Architectural Intent:
//! This module is an infrastructure adapter implementing `PatternMatcherPort`.
//! It compiles TOML pattern definitions into `CompiledPattern` domain entities
//! and matches them against source code using tree-sitter queries. The module
//! captures AST bindings and resolves template variables to produce replacement text.
//!
//! Pattern Format (PRD section 4.6):
//! Each pattern is a TOML file with [pattern], [pattern.detect],
//! [pattern.transform], and optional [pattern.bindings] sections.
//!
//! Parallelisation Notes:
//! - `PatternEngine` is Send + Sync for cross-thread use.
//! - Pattern matching per file is independent and parallelisable.

pub mod compiler;
pub mod matcher;

pub use compiler::{compile_pattern, compile_patterns, CompileError};

use crate::domain::entities::{CompiledPattern, PatternMatch};
use crate::domain::ports::PatternMatcherPort;
use crate::domain::value_objects::{Language, SourceCloud};

/// Pattern engine that compiles TOML patterns and matches them against source code.
///
/// Implements `PatternMatcherPort` from the domain layer.
#[derive(Debug, Default)]
pub struct PatternEngine;

impl PatternEngine {
    /// Create a new `PatternEngine`.
    pub fn new() -> Self {
        Self
    }

    /// Compile a TOML pattern definition string into a `CompiledPattern`.
    pub fn compile(&self, toml_source: &str) -> Result<CompiledPattern, CompileError> {
        compiler::compile_pattern(toml_source)
    }

    /// Compile multiple TOML pattern definitions.
    pub fn compile_many(
        &self,
        sources: &[(String, String)],
    ) -> (Vec<CompiledPattern>, Vec<(String, CompileError)>) {
        compiler::compile_patterns(sources)
    }
}

impl PatternMatcherPort for PatternEngine {
    /// Match patterns against source code, returning all matches.
    ///
    /// Filters patterns by language and source cloud before attempting
    /// tree-sitter query matching.
    #[tracing::instrument(skip(self, source, patterns), level = "debug")]
    fn match_patterns(
        &self,
        source: &[u8],
        language: Language,
        source_cloud: SourceCloud,
        patterns: &[CompiledPattern],
    ) -> Vec<PatternMatch> {
        matcher::match_all_patterns(source, language, source_cloud, patterns)
    }
}
