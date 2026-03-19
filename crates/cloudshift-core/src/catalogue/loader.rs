//! TOML pattern catalogue loader.
//!
//! Walks a directory tree to find `.toml` pattern definition files,
//! reads them, and compiles them into `CompiledPattern` instances
//! using the pattern compiler.
//!
//! Architectural Intent:
//! Infrastructure adapter for loading pattern catalogues from the filesystem.
//! Delegates compilation to `pattern::compiler`. Reports errors per-file
//! without failing the entire catalogue load.

use crate::domain::entities::CompiledPattern;
use crate::pattern::compiler::{self, CompileError};
use std::path::Path;

/// Error type for catalogue loading.
#[derive(Debug, thiserror::Error)]
pub enum CatalogueLoadError {
    #[error("I/O error reading catalogue: {0}")]
    Io(#[from] std::io::Error),

    #[error("Pattern compilation error in {file}: {error}")]
    CompileError { file: String, error: CompileError },
}

/// Load all TOML pattern files from a directory tree.
///
/// Recursively walks the directory, finds all `.toml` files, reads them,
/// and compiles them into `CompiledPattern` instances. Files that fail
/// to compile are collected as warnings rather than aborting the load.
///
/// Returns the successfully compiled patterns and a list of load warnings.
#[tracing::instrument(level = "info")]
pub fn load_patterns_from_directory(
    dir: &Path,
) -> Result<(Vec<CompiledPattern>, Vec<CatalogueLoadWarning>), CatalogueLoadError> {
    let mut sources = Vec::new();
    let mut warnings = Vec::new();

    // Collect all .toml files from the directory tree
    collect_toml_files(dir, &mut sources, &mut warnings)?;

    if sources.is_empty() {
        tracing::info!("No pattern files found in {}", dir.display());
        return Ok((Vec::new(), warnings));
    }

    tracing::info!("Found {} pattern files in {}", sources.len(), dir.display());

    // Compile all collected sources
    let (patterns, errors) = compiler::compile_patterns(&sources);

    // Convert compilation errors to warnings
    for (file, error) in errors {
        tracing::warn!("Failed to compile pattern {file}: {error}");
        warnings.push(CatalogueLoadWarning {
            file,
            message: error.to_string(),
        });
    }

    // Validation: duplicate pattern IDs (same ID in multiple TOML files)
    let mut by_id: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for p in &patterns {
        *by_id.entry(p.id.as_str().to_string()).or_insert(0) += 1;
    }
    for (id, count) in by_id {
        if count > 1 {
            warnings.push(CatalogueLoadWarning {
                file: String::new(),
                message: format!("Duplicate pattern ID: {} ({} occurrences)", id, count),
            });
        }
    }

    tracing::info!(
        "Loaded {} patterns ({} warnings)",
        patterns.len(),
        warnings.len()
    );

    Ok((patterns, warnings))
}

/// A non-fatal warning during catalogue loading.
#[derive(Debug, Clone)]
pub struct CatalogueLoadWarning {
    pub file: String,
    pub message: String,
}

/// Recursively collect `.toml` files from a directory.
fn collect_toml_files(
    dir: &Path,
    sources: &mut Vec<(String, String)>,
    warnings: &mut Vec<CatalogueLoadWarning>,
) -> Result<(), CatalogueLoadError> {
    if !dir.exists() {
        tracing::warn!("Catalogue directory does not exist: {}", dir.display());
        return Ok(());
    }

    if !dir.is_dir() {
        // Single file mode
        if dir.extension().is_some_and(|ext| ext == "toml") {
            match std::fs::read_to_string(dir) {
                Ok(content) => {
                    sources.push((dir.display().to_string(), content));
                }
                Err(e) => {
                    warnings.push(CatalogueLoadWarning {
                        file: dir.display().to_string(),
                        message: format!("Failed to read file: {e}"),
                    });
                }
            }
        }
        return Ok(());
    }

    let entries = std::fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_toml_files(&path, sources, warnings)?;
        } else if path.extension().is_some_and(|ext| ext == "toml") {
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    sources.push((path.display().to_string(), content));
                }
                Err(e) => {
                    warnings.push(CatalogueLoadWarning {
                        file: path.display().to_string(),
                        message: format!("Failed to read file: {e}"),
                    });
                }
            }
        }
    }

    Ok(())
}

/// Load patterns from in-memory TOML strings (useful for testing and embedding).
pub fn load_patterns_from_strings(
    sources: &[(String, String)],
) -> (Vec<CompiledPattern>, Vec<CatalogueLoadWarning>) {
    let (patterns, errors) = compiler::compile_patterns(sources);

    let warnings: Vec<CatalogueLoadWarning> = errors
        .into_iter()
        .map(|(file, error)| CatalogueLoadWarning {
            file,
            message: error.to_string(),
        })
        .collect();

    (patterns, warnings)
}
