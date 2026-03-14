//! Analyse command — detect cloud patterns without transforming.

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use cloudshift_core::{OutputFormat, TransformConfig, transform_repo};

use crate::commands::{LanguageFilter, SourceCloudFilter};

/// CLI output format for analysis results.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum AnalyseOutputFormat {
    Json,
    Table,
}

/// Analyse code without transforming (detect cloud usage patterns only).
#[derive(Args, Debug)]
#[command(about = "Analyse code to detect cloud usage patterns without transforming")]
pub struct AnalyseArgs {
    /// File or directory to analyse (default: current directory).
    #[arg(default_value = ".")]
    pub path: String,

    /// Source cloud provider to detect.
    #[arg(long = "source", value_enum, default_value = "any")]
    pub source_cloud: SourceCloudFilter,

    /// Restrict analysis to a specific language.
    #[arg(long = "language", value_enum)]
    pub language: Option<LanguageFilter>,

    /// Output format.
    #[arg(long = "output", value_enum, default_value = "table")]
    pub output_format: AnalyseOutputFormat,

    /// Alias for --output (deprecated, use --output instead).
    #[arg(long = "format", value_enum, hide = true)]
    pub format: Option<AnalyseOutputFormat>,
}

pub fn run(args: AnalyseArgs) -> Result<()> {
    // Emit deprecation warning when --format is used.
    if args.format.is_some() {
        eprintln!("Warning: --format is deprecated, use --output instead");
    }

    let source = args.source_cloud.to_core();

    info!(path = %args.path, source = %source, "Starting analysis");

    // Analysis uses the same pipeline as transform but in dry-run mode;
    // the core will return pattern match data without applying changes.
    let config = TransformConfig {
        source_cloud: source,
        output_format: OutputFormat::Json,
        dry_run: true,
        ..TransformConfig::default()
    };

    let report = transform_repo(&args.path, &config)
        .with_context(|| format!("Failed to analyse: {}", args.path))?;

    let output_fmt = args.format.as_ref().unwrap_or(&args.output_format);
    match output_fmt {
        AnalyseOutputFormat::Json => {
            let json = serde_json::to_string_pretty(&report)
                .context("Failed to serialise analysis")?;
            println!("{}", json);
        }
        AnalyseOutputFormat::Table => {
            crate::output::print_analysis_table(&report);
        }
    }

    Ok(())
}
