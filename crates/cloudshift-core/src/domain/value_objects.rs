use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Source cloud provider being migrated FROM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceCloud {
    Aws,
    Azure,
    Any,
}

impl FromStr for SourceCloud {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "aws" => Ok(Self::Aws),
            "azure" => Ok(Self::Azure),
            "any" => Ok(Self::Any),
            other => Err(DomainError::InvalidSourceCloud(other.to_string())),
        }
    }
}

impl fmt::Display for SourceCloud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Aws => write!(f, "aws"),
            Self::Azure => write!(f, "azure"),
            Self::Any => write!(f, "any"),
        }
    }
}

/// Programming language detected or targeted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Python,
    TypeScript,
    JavaScript,
    Java,
    Go,
    Hcl,
    Yaml,
    Dockerfile,
    Json,
}

impl Language {
    /// File extensions associated with this language.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Python => &["py", "pyi"],
            Self::TypeScript => &["ts", "tsx"],
            Self::JavaScript => &["js", "jsx", "mjs"],
            Self::Java => &["java"],
            Self::Go => &["go"],
            Self::Hcl => &["tf", "hcl"],
            Self::Yaml => &["yml", "yaml"],
            Self::Dockerfile => &["Dockerfile"],
            Self::Json => &["json"],
        }
    }

    /// Detect language from file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "py" | "pyi" => Some(Self::Python),
            "ts" | "tsx" => Some(Self::TypeScript),
            "js" | "jsx" | "mjs" => Some(Self::JavaScript),
            "java" => Some(Self::Java),
            "go" => Some(Self::Go),
            "tf" | "hcl" => Some(Self::Hcl),
            "yml" | "yaml" => Some(Self::Yaml),
            "json" => Some(Self::Json),
            _ => None,
        }
    }

    /// Detect language from filename (handles Dockerfile).
    pub fn from_filename(name: &str) -> Option<Self> {
        if name == "Dockerfile" || name.starts_with("Dockerfile.") {
            return Some(Self::Dockerfile);
        }
        let ext = name.rsplit('.').next()?;
        Self::from_extension(ext)
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Python => write!(f, "python"),
            Self::TypeScript => write!(f, "typescript"),
            Self::JavaScript => write!(f, "javascript"),
            Self::Java => write!(f, "java"),
            Self::Go => write!(f, "go"),
            Self::Hcl => write!(f, "hcl"),
            Self::Yaml => write!(f, "yaml"),
            Self::Dockerfile => write!(f, "dockerfile"),
            Self::Json => write!(f, "json"),
        }
    }
}

/// Confidence score for a pattern match (0.0 to 1.0).
///
/// Calculated from four weighted factors per PRD §4.7:
/// - Pattern specificity (35%)
/// - Version alignment (25%)
/// - Argument completeness (25%)
/// - Test coverage (15%)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Confidence(f64);

impl Confidence {
    /// Create a new confidence score. Clamps to [0.0, 1.0].
    pub fn new(value: f64) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    pub fn value(self) -> f64 {
        self.0
    }

    /// >= 0.90: Auto-transform in --auto mode.
    pub fn is_high(self) -> bool {
        self.0 >= 0.90
    }

    /// 0.70–0.89: Transform with review flag.
    pub fn is_medium(self) -> bool {
        self.0 >= 0.70 && self.0 < 0.90
    }

    /// < 0.70: Flagged for AI-assisted fallback or manual review.
    pub fn is_low(self) -> bool {
        self.0 < 0.70
    }

    /// Compute from the four weighted factors.
    pub fn from_factors(
        pattern_specificity: f64,
        version_alignment: f64,
        argument_completeness: f64,
        test_coverage: f64,
    ) -> Self {
        let score = pattern_specificity * 0.35
            + version_alignment * 0.25
            + argument_completeness * 0.25
            + test_coverage * 0.15;
        Self::new(score)
    }
}

impl fmt::Display for Confidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}", self.0)
    }
}

impl PartialOrd for Confidence {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

/// Unique identifier for a pattern in the catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PatternId(pub String);

impl PatternId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PatternId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Output format for CLI and SDK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Diff,
    Json,
    Sarif,
}

impl FromStr for OutputFormat {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "diff" => Ok(Self::Diff),
            "json" => Ok(Self::Json),
            "sarif" => Ok(Self::Sarif),
            other => Err(DomainError::InvalidOutputFormat(other.to_string())),
        }
    }
}

/// Effort estimation for a file migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationEffort {
    Low,
    Medium,
    High,
}

impl MigrationEffort {
    /// Estimate effort from average confidence across patterns.
    pub fn from_confidence(avg_confidence: Confidence) -> Self {
        if avg_confidence.is_high() {
            Self::Low
        } else if avg_confidence.is_medium() {
            Self::Medium
        } else {
            Self::High
        }
    }
}

impl fmt::Display for MigrationEffort {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "Low"),
            Self::Medium => write!(f, "Medium"),
            Self::High => write!(f, "High"),
        }
    }
}

/// Span within a source file (byte offsets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_row: usize,
    pub start_col: usize,
    pub end_row: usize,
    pub end_col: usize,
}

/// Domain-level errors.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Invalid source cloud: {0}")]
    InvalidSourceCloud(String),
    #[error("Invalid output format: {0}")]
    InvalidOutputFormat(String),
    #[error("Invalid confidence value: {0}")]
    InvalidConfidence(f64),
    #[error("Pattern not found: {0}")]
    PatternNotFound(PatternId),
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
    #[error("Domain invariant violated: {0}")]
    InvariantViolation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_clamps_to_valid_range() {
        assert_eq!(Confidence::new(1.5).value(), 1.0);
        assert_eq!(Confidence::new(-0.5).value(), 0.0);
        assert_eq!(Confidence::new(0.85).value(), 0.85);
    }

    #[test]
    fn confidence_thresholds() {
        assert!(Confidence::new(0.95).is_high());
        assert!(Confidence::new(0.80).is_medium());
        assert!(Confidence::new(0.50).is_low());
    }

    #[test]
    fn confidence_from_factors() {
        let c = Confidence::from_factors(1.0, 1.0, 1.0, 1.0);
        assert_eq!(c.value(), 1.0);
        let c = Confidence::from_factors(0.9, 0.8, 0.7, 1.0);
        // 0.9*0.35 + 0.8*0.25 + 0.7*0.25 + 1.0*0.15 = 0.315 + 0.2 + 0.175 + 0.15 = 0.84
        assert!((c.value() - 0.84).abs() < 0.001);
    }

    #[test]
    fn language_detection_from_extension() {
        assert_eq!(Language::from_extension("py"), Some(Language::Python));
        assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
        assert_eq!(Language::from_extension("tf"), Some(Language::Hcl));
        assert_eq!(Language::from_extension("xyz"), None);
    }

    #[test]
    fn language_detection_from_filename() {
        assert_eq!(Language::from_filename("Dockerfile"), Some(Language::Dockerfile));
        assert_eq!(Language::from_filename("Dockerfile.prod"), Some(Language::Dockerfile));
        assert_eq!(Language::from_filename("main.go"), Some(Language::Go));
    }

    #[test]
    fn source_cloud_parsing() {
        assert_eq!("aws".parse::<SourceCloud>().unwrap(), SourceCloud::Aws);
        assert_eq!("Azure".parse::<SourceCloud>().unwrap(), SourceCloud::Azure);
        assert!("gcp".parse::<SourceCloud>().is_err());
    }

    #[test]
    fn migration_effort_from_confidence() {
        assert_eq!(MigrationEffort::from_confidence(Confidence::new(0.95)), MigrationEffort::Low);
        assert_eq!(MigrationEffort::from_confidence(Confidence::new(0.80)), MigrationEffort::Medium);
        assert_eq!(MigrationEffort::from_confidence(Confidence::new(0.50)), MigrationEffort::High);
    }
}
