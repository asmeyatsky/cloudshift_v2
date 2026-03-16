//! Self-learning pattern store.
//!
//! When the LLM fallback completes a migration, this module extracts
//! the delta between the pattern engine's partial output and the LLM's
//! complete output. It analyzes the changes using tree-sitter and
//! generates candidate TOML pattern files that can be reviewed and
//! promoted to the compiled catalogue.
//!
//! This creates a flywheel: more migrations -> more LLM fallbacks ->
//! more candidate patterns -> fewer LLM fallbacks -> faster migrations.

pub mod analyzer;
pub mod extractor;
pub mod generator;
pub mod store;

pub use analyzer::analyze_changes;
pub use extractor::extract_llm_delta;
pub use generator::generate_candidate_pattern;
pub use store::PatternStore;
