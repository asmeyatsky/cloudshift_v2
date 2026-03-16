//! Apply command — apply a previously generated diff.

use anyhow::{bail, Context, Result};
use clap::Args;
use tracing::info;

/// Apply a previously generated diff/patch file to the working tree.
#[derive(Args, Debug)]
#[command(about = "Apply a previously generated diff/patch file")]
pub struct ApplyArgs {
    /// Path to the diff file to apply.
    pub path: String,

    /// Perform a dry run without writing changes.
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(args: ApplyArgs) -> Result<()> {
    info!(path = %args.path, dry_run = %args.dry_run, "Applying diff");

    let content = std::fs::read_to_string(&args.path)
        .with_context(|| format!("Failed to read diff file: {}", args.path))?;

    if content.trim().is_empty() {
        bail!("Diff file is empty: {}", args.path);
    }

    // Count unified-diff hunks (lines starting with "@@").
    let hunk_count = content.lines().filter(|l| l.starts_with("@@")).count();

    if hunk_count == 0 {
        bail!(
            "No diff hunks found in {}. Expected a unified diff format.",
            args.path
        );
    }

    if args.dry_run {
        println!(
            "Dry run: {} contains {} hunk(s). No changes applied.",
            args.path, hunk_count
        );
    } else {
        println!(
            "Diff file validated: {} contains {} hunk(s). \
             Applying patches is not yet implemented.",
            args.path, hunk_count
        );
    }

    Ok(())
}
