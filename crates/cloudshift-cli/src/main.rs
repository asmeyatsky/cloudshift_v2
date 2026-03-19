//! CloudShift CLI — entry point.
//!
//! Presentation layer only: parses arguments via clap, delegates to
//! cloudshift-core for all business logic, and formats output.

mod commands;
mod output;

use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use commands::{analyse, apply, catalogue, diff, learn, report, transform, validate};

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
    /// Transform code files or repositories from AWS/Azure to GCP.
    #[command(about = "Transform code files or repositories from AWS/Azure to GCP")]
    Transform(transform::TransformArgs),

    /// Analyse code to detect cloud usage patterns without transforming.
    #[command(about = "Analyse code to detect cloud usage patterns without transforming")]
    Analyse(analyse::AnalyseArgs),

    /// Show what would change without applying.
    #[command(about = "Show what would change without applying (dry-run diff)")]
    Diff(diff::DiffArgs),

    /// Apply a previously generated diff/patch file.
    #[command(about = "Apply a previously generated diff/patch file")]
    Apply(apply::ApplyArgs),

    /// Manage and query the GCP Pattern Catalogue.
    #[command(about = "Manage and query the GCP Pattern Catalogue")]
    Catalogue(catalogue::CatalogueArgs),

    /// Run post-transformation validation checks.
    #[command(about = "Run post-transformation validation checks on transformed code")]
    Validate(validate::ValidateArgs),

    /// Generate a migration report from a previous run.
    #[command(about = "Generate a human-readable migration report from a previous run")]
    Report(report::ReportArgs),

    /// Learn new patterns from before/after code pairs.
    #[command(about = "Learn new patterns by comparing before/after code transformations")]
    Learn(learn::LearnArgs),
}

fn main() {
    // Initialise tracing; use JSON when CLOUDSHIFT_LOG_JSON=1 (e.g. in production/CI).
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let use_json = std::env::var("CLOUDSHIFT_LOG_JSON")
        .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
    if use_json {
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(env_filter)
            .init();
    } else {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    }

    let cli = Cli::parse();

    let result = match cli.command {
        Command::Transform(args) => transform::run(args),
        Command::Analyse(args) => analyse::run(args),
        Command::Diff(args) => diff::run(args),
        Command::Apply(args) => apply::run(args),
        Command::Catalogue(args) => catalogue::run(args),
        Command::Validate(args) => validate::run(args),
        Command::Report(args) => report::run(args),
        Command::Learn(args) => learn::run(args),
    };

    if let Err(err) = result {
        eprintln!("Error: {err:?}");
        std::process::exit(1);
    }
}
