//! Intent-Based Transformation Engine (IBTE) — PRD Supplement §4.8.
//!
//! Multi-pass buffered execution: build a Stateful Context Registry (SCR) from
//! provider entry points, then detect chain patterns (e.g. resource + Table + put_item)
//! and collapse them into a single consolidated replacement (N-to-1).

mod chains;
mod discovery;
mod registry;

pub use registry::{RegistryEntry, StatefulContextRegistry};

use crate::analyser::treesitter;
use crate::domain::entities::PatternMatch;
use crate::domain::ports::AnalysisError;
use crate::domain::value_objects::{Language, SourceCloud};

/// Run IBTE for Python: parse, build SCR, detect chains, return consolidated matches.
/// Caller merges these with existing pattern matches; IBTE matches use high confidence
/// so they supersede overlapping 1:1 matches when apply_all deduplicates.
pub fn run_ibte_python(
    source: &[u8],
    source_cloud: SourceCloud,
) -> Result<Vec<PatternMatch>, AnalysisError> {
    let tree = treesitter::parse_source(source, Language::Python)?;
    let mut registry = StatefulContextRegistry::new();
    discovery::discover_python(source, &tree, &mut registry)?;

    let mut matches = Vec::new();
    if source_cloud == SourceCloud::Aws {
        matches.extend(chains::detect_dynamodb_put_chain(source, &tree, &registry)?);
    }
    if source_cloud == SourceCloud::Azure {
        matches.extend(chains::detect_azure_blob_upload_chain(source, &tree, &registry)?);
    }

    Ok(matches)
}
