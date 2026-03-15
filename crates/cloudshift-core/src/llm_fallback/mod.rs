//! LLM-assisted fallback for completing GCP migrations.
//!
//! After deterministic pattern transforms, if the code still contains
//! AWS/Azure references, this module invokes an LLM to complete the
//! migration. This implements PRD section 4.7: "< 0.70: Flagged for AI-assisted
//! fallback."
//!
//! Architecture:
//! - `detector` -- Scans code for remaining cloud SDK references
//! - `prompt` -- Builds the LLM prompt with context
//! - `client` -- HTTP client for Claude API (infrastructure adapter)

pub mod detector;
pub mod prompt;
#[cfg(feature = "llm-fallback")]
pub mod client;

pub use detector::{detect_remaining_cloud_refs, needs_llm_fallback, RemainingReference};
pub use prompt::build_fallback_prompt;
