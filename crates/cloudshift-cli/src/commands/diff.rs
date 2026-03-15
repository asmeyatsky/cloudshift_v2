//! Diff command — show what would change without applying.

use std::path::Path;

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use cloudshift_core::{TransformConfig, transform_file, transform_repo};

use crate::commands::{LanguageFilter, SourceCloudFilter};
use crate::commands::transform::TransformOutputFormat;
use crate::output;

/// Show what would change without applying.
#[derive(Args, Debug)]
#[command(about = "Show what would change without applying (dry-run diff)")]
pub struct DiffArgs {
    /// File or directory to diff (default: current directory).
    #[arg(default_value = ".")]
    pub path: String,

    /// Source cloud provider.
    #[arg(long = "source", value_enum, default_value = "any")]
    pub source_cloud: SourceCloudFilter,

    /// Restrict to a specific language.
    #[arg(long = "language", value_enum)]
    pub language: Option<LanguageFilter>,

    /// Minimum confidence to include in diff.
    #[arg(long, default_value = "0.70")]
    pub threshold: f64,

    /// Output format.
    #[arg(long = "output", value_enum, default_value = "diff")]
    pub output_format: TransformOutputFormat,

    /// Include files matching glob pattern.
    #[arg(long)]
    pub include: Option<String>,

    /// Exclude files matching glob pattern.
    #[arg(long)]
    pub exclude: Option<String>,
}

pub fn run(args: DiffArgs) -> Result<()> {
    let config = TransformConfig {
        source_cloud: args.source_cloud.to_core(),
        output_format: args.output_format.to_core(),
        dry_run: true, // diff is always dry-run
        catalogue_path: crate::commands::discover_catalogue_path(),
        ..TransformConfig::default()
    };

    info!(path = %args.path, "Generating diff");

    let target = Path::new(&args.path);

    if target.is_file() {
        let result = transform_file(&args.path, &config)
            .with_context(|| format!("Failed to diff file: {}", args.path))?;
        output::print_transform_result(&result, &args.output_format);
    } else {
        let report = transform_repo(&args.path, &config)
            .with_context(|| format!("Failed to diff repository: {}", args.path))?;
        output::print_repo_report(&report, &args.output_format);
    }

    Ok(())
}
