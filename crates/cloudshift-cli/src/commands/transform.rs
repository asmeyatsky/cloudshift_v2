//! Transform command — transforms code files or repositories to GCP.

use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use cloudshift_core::{
    OutputFormat, TransformConfig, transform_file, transform_repo,
};

use crate::commands::{LanguageFilter, SourceCloudFilter};
use crate::output;

/// CLI output format for the transform command.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum TransformOutputFormat {
    Diff,
    Json,
    Sarif,
}

impl TransformOutputFormat {
    pub(crate) fn to_core(&self) -> OutputFormat {
        match self {
            Self::Diff => OutputFormat::Diff,
            Self::Json => OutputFormat::Json,
            Self::Sarif => OutputFormat::Sarif,
        }
    }
}

/// Transform code files or repositories from AWS/Azure to GCP.
#[derive(Args, Debug)]
#[command(about = "Transform code files or repositories from AWS/Azure to GCP")]
pub struct TransformArgs {
    /// File, directory, or Git repo URL (default: current directory).
    #[arg(default_value = ".")]
    pub path: String,

    /// Source cloud provider to migrate from.
    #[arg(long = "source", value_enum, default_value = "any")]
    pub source_cloud: SourceCloudFilter,

    /// Restrict transformation to a specific language.
    #[arg(long = "language", value_enum)]
    pub language: Option<LanguageFilter>,

    /// Show diff without applying changes.
    #[arg(long, default_value = "true")]
    pub dry_run: bool,

    /// Auto-apply high-confidence transformations (>= 0.90).
    #[arg(long)]
    pub auto: bool,

    /// Minimum confidence to include in diff.
    #[arg(long, default_value = "0.70")]
    pub threshold: f64,

    /// Output format.
    #[arg(long = "output", value_enum, default_value = "diff")]
    pub output_format: TransformOutputFormat,

    /// Write JSON migration report to path.
    #[arg(long = "report")]
    pub report_path: Option<String>,

    /// Worker threads for repo-level transforms (default: number of CPUs).
    #[arg(long)]
    pub parallel: Option<usize>,

    /// Skip Infrastructure as Code files.
    #[arg(long)]
    pub no_iac: bool,

    /// Skip CI/CD pipeline files.
    #[arg(long)]
    pub no_ci: bool,

    /// Include files matching glob pattern.
    #[arg(long)]
    pub include: Option<String>,

    /// Exclude files matching glob pattern.
    #[arg(long)]
    pub exclude: Option<String>,
}

impl TransformArgs {
    /// Build a core TransformConfig from CLI arguments.
    fn to_config(&self) -> TransformConfig {
        TransformConfig {
            source_cloud: self.source_cloud.to_core(),
            output_format: self.output_format.to_core(),
            dry_run: if self.auto { false } else { self.dry_run },
            parallel: self.parallel.unwrap_or(0),
            threshold: self.threshold,
            auto_apply_threshold: if self.auto { 0.90 } else { 1.0 },
            no_iac: self.no_iac,
            no_ci: self.no_ci,
            report_path: self.report_path.clone(),
            include_globs: self.include.iter().cloned().collect(),
            exclude_globs: self.exclude.iter().cloned().collect(),
            ..TransformConfig::default()
        }
    }
}

pub fn run(args: TransformArgs) -> Result<()> {
    let config = args.to_config();
    let path = &args.path;

    info!(path = %path, source = %config.source_cloud, dry_run = %config.dry_run, "Starting transform");

    let target = Path::new(path);

    if target.is_file() {
        let result = transform_file(path, &config)
            .with_context(|| format!("Failed to transform file: {}", path))?;

        output::print_transform_result(&result, &args.output_format);

        if let Some(ref report_path) = args.report_path {
            let json = serde_json::to_string_pretty(&result)
                .context("Failed to serialise report")?;
            std::fs::write(report_path, json)
                .with_context(|| format!("Failed to write report to {}", report_path))?;
            info!(path = %report_path, "Report written");
        }
    } else {
        let report = transform_repo(path, &config)
            .with_context(|| format!("Failed to transform repository: {}", path))?;

        output::print_repo_report(&report, &args.output_format);

        if let Some(ref report_path) = args.report_path {
            let json = serde_json::to_string_pretty(&report)
                .context("Failed to serialise report")?;
            std::fs::write(report_path, json)
                .with_context(|| format!("Failed to write report to {}", report_path))?;
            info!(path = %report_path, "Report written");
        }
    }

    Ok(())
}
