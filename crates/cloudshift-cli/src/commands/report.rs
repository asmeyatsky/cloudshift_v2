//! Report command — generate a migration report from a previous run.

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use cloudshift_core::RepoReport;

/// Generate a human-readable migration report from a previous run.
#[derive(Args, Debug)]
#[command(about = "Generate a human-readable migration report from a previous run")]
pub struct ReportArgs {
    /// Path to the JSON report from a previous transform run.
    pub path: String,

    /// Output report file path.
    #[arg(long = "output")]
    pub output: Option<String>,
}

pub fn run(args: ReportArgs) -> Result<()> {
    info!(path = %args.path, "Generating report");

    let content = std::fs::read_to_string(&args.path)
        .with_context(|| format!("Failed to read report input: {}", args.path))?;

    let report: RepoReport = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse report JSON: {}", args.path))?;

    let summary = format!(
        "Migration Report\n\
         ================\n\
         Repository:          {}\n\
         Files changed:       {}\n\
         Total constructs:    {}\n\
         Patterns matched:    {}\n\
         Average confidence:  {}\n\
         Overall effort:      {}\n",
        report.path,
        report.changes.len(),
        report.total_constructs,
        report.total_patterns_matched,
        report.average_confidence,
        report.overall_effort,
    );

    match args.output {
        Some(ref output_path) => {
            std::fs::write(output_path, &summary)
                .with_context(|| format!("Failed to write report to {}", output_path))?;
            info!(path = %output_path, "Report written");
        }
        None => {
            println!("{}", summary);
        }
    }

    Ok(())
}
