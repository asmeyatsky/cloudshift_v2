//! Command submodule declarations and shared types.

use cloudshift_core::SourceCloud;

pub mod analyse;
pub mod apply;
pub mod catalogue;
pub mod diff;
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
