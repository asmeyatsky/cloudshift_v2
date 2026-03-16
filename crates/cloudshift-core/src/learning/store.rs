//! Pattern store for managing learned candidate patterns.
//!
//! Provides read/write access to the `learned/` directory where
//! candidate patterns await human review before promotion.

use std::path::{Path, PathBuf};

use super::generator::CandidatePattern;

/// Manages the learned patterns directory.
pub struct PatternStore {
    /// Path to the `learned/` directory for pending candidates.
    learned_dir: PathBuf,
    /// Path to the `patterns/` directory for promoted patterns.
    patterns_dir: PathBuf,
}

impl PatternStore {
    /// Create a new pattern store.
    pub fn new(learned_dir: PathBuf, patterns_dir: PathBuf) -> Self {
        Self {
            learned_dir,
            patterns_dir,
        }
    }

    /// Create from a project root path.
    pub fn from_root(root: &Path) -> Self {
        Self {
            learned_dir: root.join("learned"),
            patterns_dir: root.join("patterns"),
        }
    }

    /// Save a candidate pattern to the learned/ directory.
    pub fn save_candidate(&self, candidate: &CandidatePattern) -> Result<PathBuf, std::io::Error> {
        // Ensure the learned directory exists
        let lang_dir = self.learned_dir.join(candidate.language.to_string());
        std::fs::create_dir_all(&lang_dir)?;

        let file_path = lang_dir.join(&candidate.suggested_filename);
        std::fs::write(&file_path, &candidate.toml_content)?;

        tracing::info!(
            "Saved candidate pattern {} to {}",
            candidate.candidate_id,
            file_path.display()
        );

        Ok(file_path)
    }

    /// Save multiple candidates and return their paths.
    pub fn save_candidates(
        &self,
        candidates: &[CandidatePattern],
    ) -> Vec<Result<PathBuf, std::io::Error>> {
        candidates.iter().map(|c| self.save_candidate(c)).collect()
    }

    /// List all pending candidate patterns.
    pub fn list_pending(&self) -> Result<Vec<PendingCandidate>, std::io::Error> {
        let mut pending = Vec::new();

        if !self.learned_dir.exists() {
            return Ok(pending);
        }

        for entry in walkdir(&self.learned_dir)? {
            if entry.extension().is_some_and(|e| e == "toml") {
                let content = std::fs::read_to_string(&entry)?;
                if content.contains("review_status = \"pending\"") {
                    let candidate_id = extract_field(&content, "candidate_id")
                        .unwrap_or_else(|| "unknown".to_string());
                    let language = extract_field(&content, "language")
                        .unwrap_or_else(|| "unknown".to_string());
                    let source_file = extract_field(&content, "source_file")
                        .unwrap_or_else(|| "unknown".to_string());

                    pending.push(PendingCandidate {
                        candidate_id,
                        file_path: entry,
                        language,
                        source_file,
                    });
                }
            }
        }

        Ok(pending)
    }

    /// Promote a candidate pattern from learned/ to patterns/.
    pub fn promote(&self, candidate_id: &str) -> Result<PathBuf, PatternStoreError> {
        let candidate_file = self.find_candidate(candidate_id)?;
        let content = std::fs::read_to_string(&candidate_file)
            .map_err(|e| PatternStoreError::Io(e.to_string()))?;

        // Update review status
        let promoted_content = content.replace(
            "review_status = \"pending\"",
            "review_status = \"promoted\"",
        );

        // Determine the target directory based on language
        let language = extract_field(&content, "language").ok_or_else(|| {
            PatternStoreError::InvalidCandidate("Missing language field".into())
        })?;
        let target_dir = self.patterns_dir.join(&language);
        std::fs::create_dir_all(&target_dir)
            .map_err(|e| PatternStoreError::Io(e.to_string()))?;

        // Copy to patterns directory with a clean name
        let filename = candidate_file
            .file_name()
            .ok_or_else(|| PatternStoreError::InvalidCandidate("No filename".into()))?;
        let target_path = target_dir.join(filename);
        std::fs::write(&target_path, promoted_content)
            .map_err(|e| PatternStoreError::Io(e.to_string()))?;

        // Remove from learned directory
        std::fs::remove_file(&candidate_file)
            .map_err(|e| PatternStoreError::Io(e.to_string()))?;

        tracing::info!(
            "Promoted candidate {} from {} to {}",
            candidate_id,
            candidate_file.display(),
            target_path.display()
        );

        Ok(target_path)
    }

    /// Reject a candidate pattern (delete it).
    pub fn reject(&self, candidate_id: &str) -> Result<(), PatternStoreError> {
        let candidate_file = self.find_candidate(candidate_id)?;
        std::fs::remove_file(&candidate_file)
            .map_err(|e| PatternStoreError::Io(e.to_string()))?;

        tracing::info!("Rejected candidate {}", candidate_id);
        Ok(())
    }

    /// Get statistics about the pattern store.
    pub fn stats(&self) -> PatternStoreStats {
        let pending = self.list_pending().unwrap_or_default().len();
        let promoted = count_files(&self.patterns_dir, "toml");

        PatternStoreStats { pending, promoted }
    }

    fn find_candidate(&self, candidate_id: &str) -> Result<PathBuf, PatternStoreError> {
        if !self.learned_dir.exists() {
            return Err(PatternStoreError::NotFound(candidate_id.to_string()));
        }

        for entry in
            walkdir(&self.learned_dir).map_err(|e| PatternStoreError::Io(e.to_string()))?
        {
            if entry.extension().is_some_and(|e| e == "toml") {
                let content = std::fs::read_to_string(&entry)
                    .map_err(|e| PatternStoreError::Io(e.to_string()))?;
                if content.contains(&format!("candidate_id = \"{}\"", candidate_id)) {
                    return Ok(entry);
                }
            }
        }

        Err(PatternStoreError::NotFound(candidate_id.to_string()))
    }
}

/// A pending candidate pattern awaiting review.
#[derive(Debug, Clone)]
pub struct PendingCandidate {
    pub candidate_id: String,
    pub file_path: PathBuf,
    pub language: String,
    pub source_file: String,
}

/// Statistics about the pattern store.
#[derive(Debug)]
pub struct PatternStoreStats {
    pub pending: usize,
    pub promoted: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum PatternStoreError {
    #[error("Candidate not found: {0}")]
    NotFound(String),
    #[error("I/O error: {0}")]
    Io(String),
    #[error("Invalid candidate: {0}")]
    InvalidCandidate(String),
}

/// Recursively walk a directory and collect file paths.
fn walkdir(dir: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                files.extend(walkdir(&path)?);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn count_files(dir: &Path, extension: &str) -> usize {
    walkdir(dir)
        .unwrap_or_default()
        .iter()
        .filter(|p| p.extension().is_some_and(|e| e == extension))
        .count()
}

fn extract_field(toml_content: &str, field: &str) -> Option<String> {
    let pattern = format!("{} = \"", field);
    if let Some(start) = toml_content.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = toml_content[value_start..].find('"') {
            return Some(toml_content[value_start..value_start + end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_field() {
        let content = r#"
candidate_id = "abc123"
language = "python"
"#;
        assert_eq!(
            extract_field(content, "candidate_id"),
            Some("abc123".to_string())
        );
        assert_eq!(
            extract_field(content, "language"),
            Some("python".to_string())
        );
        assert_eq!(extract_field(content, "missing"), None);
    }
}
