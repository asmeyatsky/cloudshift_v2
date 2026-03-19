//! CloudShift Core — Pure transformation engine for GCP code migration.
//!
//! # Architectural Intent
//!
//! This crate contains the deterministic, memory-safe transformation engine.
//! It operates on ASTs (via tree-sitter), not on regex or text. Every
//! transformation is structurally correct by construction.
//!
//! # Layer Structure (Clean/Hexagonal Architecture)
//!
//! - `domain` — Entities, value objects, domain events, port traits, domain services.
//!   Zero infrastructure dependencies.
//! - `analyser` — Language-specific semantic extractors (tree-sitter bridge).
//! - `pattern` — Pattern DSL compiler and matcher.
//! - `diff` — AST diff → unified diff emitter.
//! - `catalogue` — GCP Pattern Catalogue loader (TOML).
//! - `ingestion` — File tree walker, language detection, file queuing.
//! - `pipeline` — Parallelism-first orchestration (fan-out/fan-in, DAG).
//!
//! # Parallelisation Notes
//!
//! - File ingestion and parsing run concurrently via rayon.
//! - Pattern matching across files is embarrassingly parallel.
//! - Diff generation is per-file and parallelised.
//! - The pipeline module coordinates via DAG orchestration.

pub mod analyser;
pub mod catalogue;
pub mod diff;
pub mod domain;
pub mod fixup;
pub mod ibte;
pub mod ingestion;
pub mod learning;
pub mod llm_fallback;
pub mod pattern;
pub mod pipeline;

// Re-export key public types for ergonomic usage
pub use domain::entities::{FileAnalysis, FileChange, PatternMatch, RepoReport, TransformResult};
pub use domain::ports::{DiffEmitterPort, FileSystemPort, PatternRepositoryPort};
pub use domain::ports::{LlmFallbackContext, LlmFallbackError, LlmFallbackPort};
pub use domain::value_objects::{Confidence, Language, OutputFormat, PatternId, SourceCloud};
pub use pipeline::{transform_file, transform_repo, TransformConfig};
