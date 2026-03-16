//! File tree walker, language detection, and file queuing.
//!
//! Architectural Intent:
//! This module handles file discovery and language detection for the
//! transformation pipeline. It walks directory trees using glob patterns,
//! detects file languages from extensions/filenames, and filters by
//! include/exclude patterns. Uses rayon for parallel file discovery.
//!
//! This is infrastructure-level code — it does not contain business logic.
//! The output is a list of `DiscoveredFile` records that feed into the
//! pipeline's transformation stage.
//!
//! Parallelisation Notes:
//! - File discovery uses rayon's parallel iterator for directory walking.
//! - Language detection is purely functional and embarrassingly parallel.

use crate::domain::value_objects::Language;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

/// A file discovered during ingestion, ready for analysis.
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    /// Absolute or relative path to the file.
    pub path: String,
    /// Detected programming language.
    pub language: Language,
    /// File size in bytes.
    pub size_bytes: u64,
}

/// Configuration for file ingestion.
#[derive(Debug, Clone)]
pub struct IngestionConfig {
    /// Glob patterns to include (e.g., `["**/*.py", "**/*.ts"]`).
    /// If empty, all supported language files are included.
    pub include_patterns: Vec<String>,
    /// Glob patterns to exclude (e.g., `["**/node_modules/**", "**/venv/**"]`).
    pub exclude_patterns: Vec<String>,
    /// Whether to skip IaC files (Terraform/HCL).
    pub no_iac: bool,
    /// Whether to skip CI/CD files (Dockerfile, YAML in .github/).
    pub no_ci: bool,
    /// Maximum file size to process (in bytes). Files larger than this are skipped.
    pub max_file_size: u64,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            include_patterns: Vec::new(),
            exclude_patterns: vec![
                "**/node_modules/**".to_string(),
                "**/.git/**".to_string(),
                "**/venv/**".to_string(),
                "**/__pycache__/**".to_string(),
                "**/target/**".to_string(),
                "**/dist/**".to_string(),
                "**/build/**".to_string(),
                "**/.tox/**".to_string(),
                "**/.mypy_cache/**".to_string(),
                "**/.pytest_cache/**".to_string(),
                "**/vendor/**".to_string(),
            ],
            no_iac: false,
            no_ci: false,
            max_file_size: 10 * 1024 * 1024, // 10 MB
        }
    }
}

/// File ingestion engine.
///
/// Walks file trees, detects languages, and produces a list of files
/// ready for parallel analysis.
#[derive(Debug)]
pub struct Ingestion {
    config: IngestionConfig,
}

impl Ingestion {
    /// Create a new `Ingestion` with the given configuration.
    pub fn new(config: IngestionConfig) -> Self {
        Self { config }
    }

    /// Create a new `Ingestion` with default configuration.
    pub fn with_defaults() -> Self {
        Self {
            config: IngestionConfig::default(),
        }
    }

    /// Walk a directory tree and discover all relevant source files.
    ///
    /// Returns files sorted by path for deterministic output.
    #[tracing::instrument(skip(self), level = "info")]
    pub fn discover_files(&self, root: &Path) -> Result<Vec<DiscoveredFile>, IngestionError> {
        if !root.exists() {
            return Err(IngestionError::PathNotFound(root.display().to_string()));
        }

        // If root is a single file, process it directly
        if root.is_file() {
            return self.discover_single_file(root);
        }

        // Collect all files from the directory tree
        let all_files = self.walk_directory(root)?;

        // Filter and detect languages in parallel
        let mut discovered: Vec<DiscoveredFile> = all_files
            .par_iter()
            .filter_map(|path| self.process_file(path, root))
            .collect();

        // Sort for deterministic output
        discovered.sort_by(|a, b| a.path.cmp(&b.path));

        tracing::info!(
            "Discovered {} files from {}",
            discovered.len(),
            root.display()
        );

        Ok(discovered)
    }

    /// Process a single file path.
    fn discover_single_file(&self, path: &Path) -> Result<Vec<DiscoveredFile>, IngestionError> {
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let language = match Language::from_filename(filename) {
            Some(lang) => lang,
            None => return Ok(Vec::new()),
        };

        let metadata = std::fs::metadata(path).map_err(|e| IngestionError::Io(e.to_string()))?;

        Ok(vec![DiscoveredFile {
            path: path.display().to_string(),
            language,
            size_bytes: metadata.len(),
        }])
    }

    /// Walk a directory tree and collect all file paths.
    fn walk_directory(&self, root: &Path) -> Result<Vec<PathBuf>, IngestionError> {
        let mut files = Vec::new();
        self.walk_recursive(root, root, &mut files)?;
        Ok(files)
    }

    /// Recursively walk directories, respecting exclude patterns.
    fn walk_recursive(
        &self,
        current: &Path,
        root: &Path,
        files: &mut Vec<PathBuf>,
    ) -> Result<(), IngestionError> {
        let entries = std::fs::read_dir(current).map_err(|e| {
            IngestionError::Io(format!("Failed to read {}: {e}", current.display()))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| IngestionError::Io(e.to_string()))?;
            let path = entry.path();

            // Check exclude patterns against the relative path
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            if self.is_excluded(&relative) {
                continue;
            }

            if path.is_dir() {
                self.walk_recursive(&path, root, files)?;
            } else if path.is_file() {
                files.push(path);
            }
        }

        Ok(())
    }

    /// Check if a relative path matches any exclude pattern.
    fn is_excluded(&self, relative_path: &str) -> bool {
        for pattern in &self.config.exclude_patterns {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if glob_pattern.matches(relative_path) {
                    return true;
                }
                // Also try with forward slashes normalized
                let normalized = relative_path.replace('\\', "/");
                if glob_pattern.matches(&normalized) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if a relative path matches any include pattern.
    /// If no include patterns are specified, everything is included.
    fn is_included(&self, relative_path: &str) -> bool {
        if self.config.include_patterns.is_empty() {
            return true;
        }

        for pattern in &self.config.include_patterns {
            if let Ok(glob_pattern) = glob::Pattern::new(pattern) {
                if glob_pattern.matches(relative_path) {
                    return true;
                }
                let normalized = relative_path.replace('\\', "/");
                if glob_pattern.matches(&normalized) {
                    return true;
                }
            }
        }

        false
    }

    /// Process a single file: detect language, check filters, produce DiscoveredFile.
    fn process_file(&self, path: &PathBuf, root: &Path) -> Option<DiscoveredFile> {
        let filename = path.file_name()?.to_str()?;

        // Detect language from filename/extension
        let language = Language::from_filename(filename)?;

        // Apply no_iac filter
        if self.config.no_iac && language == Language::Hcl {
            return None;
        }

        // Apply no_ci filter
        if self.config.no_ci {
            match language {
                Language::Dockerfile => return None,
                Language::Yaml => {
                    let relative = path.strip_prefix(root).unwrap_or(path);
                    let rel_str = relative.to_string_lossy();
                    if rel_str.contains(".github") || rel_str.contains(".gitlab-ci") {
                        return None;
                    }
                }
                _ => {}
            }
        }

        // Check include patterns
        let relative = path.strip_prefix(root).unwrap_or(path);
        let rel_str = relative.to_string_lossy().to_string();
        if !self.is_included(&rel_str) {
            return None;
        }

        // Check file size
        let metadata = std::fs::metadata(path).ok()?;
        let size = metadata.len();
        if size > self.config.max_file_size {
            tracing::debug!(
                "Skipping {} (size {} > max {})",
                path.display(),
                size,
                self.config.max_file_size
            );
            return None;
        }

        Some(DiscoveredFile {
            path: path.display().to_string(),
            language,
            size_bytes: size,
        })
    }
}

/// Errors that can occur during file ingestion.
#[derive(Debug, thiserror::Error)]
pub enum IngestionError {
    #[error("Path not found: {0}")]
    PathNotFound(String),

    #[error("I/O error: {0}")]
    Io(String),
}
