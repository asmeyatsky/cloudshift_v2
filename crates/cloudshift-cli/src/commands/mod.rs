//! Command submodule declarations and shared types.

use std::path::PathBuf;

use cloudshift_core::SourceCloud;

pub mod analyse;
pub mod apply;
pub mod catalogue;
pub mod diff;
pub mod learn;
pub mod report;
pub mod transform;
pub mod validate;

/// CLI source-cloud filter (shared across commands).
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum SourceCloudFilter {
    Aws,
    Azure,
    Any,
}

impl SourceCloudFilter {
    pub(crate) fn to_core(&self) -> SourceCloud {
        match self {
            Self::Aws => SourceCloud::Aws,
            Self::Azure => SourceCloud::Azure,
            Self::Any => SourceCloud::Any,
        }
    }
}

/// CLI language filter (shared across commands).
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum LanguageFilter {
    Python,
    #[value(name = "typescript")]
    TypeScript,
    Java,
    Go,
    Hcl,
    All,
}

/// Auto-discover the pattern catalogue directory.
///
/// Checks in order:
/// 1. `CLOUDSHIFT_CATALOGUE_PATH` environment variable
/// 2. `./patterns` relative to CWD
/// 3. `../patterns` relative to the running binary
///
/// Returns `None` if no catalogue directory is found.
pub fn discover_catalogue_path() -> Option<String> {
    // 1. Environment variable
    if let Ok(path) = std::env::var("CLOUDSHIFT_CATALOGUE_PATH") {
        let p = PathBuf::from(&path);
        if p.is_dir() {
            return Some(path);
        }
    }

    // 2. ./patterns relative to CWD
    if let Ok(cwd) = std::env::current_dir() {
        let candidate = cwd.join("patterns");
        if candidate.is_dir() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }

    // 3. Relative to the binary location
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            // Check sibling patterns/ (for installed binaries)
            let candidate = exe_dir.join("patterns");
            if candidate.is_dir() {
                return Some(candidate.to_string_lossy().to_string());
            }
            // Check parent (for cargo run from workspace)
            if let Some(parent) = exe_dir.parent() {
                let candidate = parent.join("patterns");
                if candidate.is_dir() {
                    return Some(candidate.to_string_lossy().to_string());
                }
            }
        }
    }

    None
}
