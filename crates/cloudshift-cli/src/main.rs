//! CloudShift CLI — entry point.
//!
//! Presentation layer only: parses arguments via clap, delegates to
//! cloudshift-core for all business logic, and formats output.

mod commands;
mod output;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use commands::{analyse, apply, catalogue, diff, report, transform, validate};

/// CloudShift v2 — Universal GCP code refactoring engine.
///
/// Automatically transforms cloud SDK calls, IaC definitions, and CI/CD
/// pipelines from AWS/Azure to Google Cloud Platform.
#[derive(Parser, Debug)]
#[command(name = "cloudshift", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Transform code files or repositories.
    Transform(transform::TransformArgs),

    /// Analyse code without transforming (detect patterns only).
    Analyse(analyse::AnalyseArgs),

    /// Show what would change without applying.
    Diff(diff::DiffArgs),

    /// Apply a previously generated diff.
    Apply(apply::ApplyArgs),

    /// Manage and query the GCP Pattern Catalogue.
    Catalogue(catalogue::CatalogueArgs),

    /// Run post-transformation validation checks.
    Validate(validate::ValidateArgs),

    /// Generate a migration report from a previous run.
    Report(report::ReportArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialise tracing with env filter (RUST_LOG).
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Transform(args) => transform::run(args)?,
        Command::Analyse(args) => analyse::run(args)?,
        Command::Diff(args) => diff::run(args)?,
        Command::Apply(args) => apply::run(args)?,
        Command::Catalogue(args) => catalogue::run(args)?,
        Command::Validate(args) => validate::run(args)?,
        Command::Report(args) => report::run(args)?,
    }

    Ok(())
}
