//! Catalogue command — manage and query the GCP Pattern Catalogue.

use std::path::Path;

use anyhow::{bail, Result};
use clap::{Args, Subcommand};
use tracing::info;

use cloudshift_core::learning::store::PatternStore;

use crate::commands::{LanguageFilter, SourceCloudFilter};

/// Manage and query the GCP Pattern Catalogue.
#[derive(Args, Debug)]
#[command(about = "Manage and query the GCP Pattern Catalogue")]
pub struct CatalogueArgs {
    #[command(subcommand)]
    pub subcommand: CatalogueSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum CatalogueSubcommand {
    /// List all patterns in the catalogue.
    #[command(about = "List all patterns in the catalogue, with optional filters")]
    List {
        /// Filter by language.
        #[arg(long = "language", value_enum)]
        language: Option<LanguageFilter>,

        /// Filter by source cloud.
        #[arg(long = "source", value_enum)]
        source: Option<SourceCloudFilter>,

        /// Filter by tag.
        #[arg(long)]
        tag: Option<String>,
    },

    /// Search patterns by keyword.
    #[command(about = "Search catalogue patterns by keyword")]
    Search {
        /// Search query.
        query: String,
    },

    /// Show details of a specific pattern by ID.
    #[command(about = "Show detailed information for a specific pattern")]
    Info {
        /// Pattern ID.
        id: String,
    },

    /// Show catalogue statistics.
    #[command(about = "Display aggregate catalogue statistics")]
    Stats,

    /// List pending candidate patterns from LLM learning.
    #[command(about = "List pending candidate patterns awaiting review")]
    Pending,

    /// Promote a candidate pattern to the catalogue.
    #[command(about = "Promote a learned candidate pattern to the compiled catalogue")]
    Promote {
        /// Candidate ID to promote.
        candidate_id: String,
    },

    /// Reject a candidate pattern.
    #[command(about = "Reject a learned candidate pattern (deletes it)")]
    Reject {
        /// Candidate ID to reject.
        candidate_id: String,
    },

    /// Show learning statistics.
    #[command(about = "Display pattern learning statistics (pending vs promoted)")]
    LearnStats,
}

pub fn run(args: CatalogueArgs) -> Result<()> {
    match args.subcommand {
        CatalogueSubcommand::List {
            language,
            source,
            tag,
        } => {
            info!(
                language = ?language,
                source = ?source,
                tag = ?tag,
                "Listing catalogue patterns"
            );
            bail!("catalogue list is not yet implemented (catalogue loading pending)");
        }
        CatalogueSubcommand::Search { ref query } => {
            info!(query = %query, "Searching catalogue");
            bail!("catalogue search is not yet implemented (catalogue loading pending)");
        }
        CatalogueSubcommand::Info { ref id } => {
            info!(id = %id, "Showing pattern info");
            bail!("catalogue info is not yet implemented (catalogue loading pending)");
        }
        CatalogueSubcommand::Stats => {
            info!("Showing catalogue stats");
            bail!("catalogue stats is not yet implemented (catalogue loading pending)");
        }
        CatalogueSubcommand::Pending => {
            let root = std::env::current_dir()?;
            let store = PatternStore::from_root(&root);
            let pending = store
                .list_pending()
                .map_err(|e| anyhow::anyhow!("Failed to list pending candidates: {e}"))?;

            if pending.is_empty() {
                println!("No pending candidate patterns.");
            } else {
                println!(
                    "{} pending candidate pattern(s):\n",
                    pending.len()
                );
                for candidate in &pending {
                    println!(
                        "  [{}] language={}, source_file={}\n       {}",
                        candidate.candidate_id,
                        candidate.language,
                        candidate.source_file,
                        candidate.file_path.display()
                    );
                }
                println!(
                    "\nTo promote: cloudshift catalogue promote <candidate_id>"
                );
                println!("To reject:  cloudshift catalogue reject <candidate_id>");
            }
            Ok(())
        }
        CatalogueSubcommand::Promote { ref candidate_id } => {
            info!(candidate_id = %candidate_id, "Promoting candidate pattern");
            let root = std::env::current_dir()?;
            let store = PatternStore::from_root(&root);
            match store.promote(candidate_id) {
                Ok(target_path) => {
                    println!(
                        "Promoted candidate {} to {}",
                        candidate_id,
                        target_path.display()
                    );
                    Ok(())
                }
                Err(e) => bail!("Failed to promote candidate {}: {}", candidate_id, e),
            }
        }
        CatalogueSubcommand::Reject { ref candidate_id } => {
            info!(candidate_id = %candidate_id, "Rejecting candidate pattern");
            let root = std::env::current_dir()?;
            let store = PatternStore::from_root(&root);
            match store.reject(candidate_id) {
                Ok(()) => {
                    println!("Rejected candidate {}", candidate_id);
                    Ok(())
                }
                Err(e) => bail!("Failed to reject candidate {}: {}", candidate_id, e),
            }
        }
        CatalogueSubcommand::LearnStats => {
            let root = std::env::current_dir()?;
            let store = PatternStore::from_root(&root);
            let stats = store.stats();
            println!("Pattern Learning Statistics:");
            println!("  Pending candidates:  {}", stats.pending);
            println!("  Promoted patterns:   {}", stats.promoted);

            // Also show the directories being used
            let learned_dir = root.join("learned");
            let patterns_dir = root.join("patterns");
            println!("\nDirectories:");
            println!(
                "  Learned:  {} {}",
                learned_dir.display(),
                if learned_dir.exists() {
                    "(exists)"
                } else {
                    "(not yet created)"
                }
            );
            println!(
                "  Patterns: {} {}",
                patterns_dir.display(),
                if Path::new(&patterns_dir).exists() {
                    "(exists)"
                } else {
                    "(not yet created)"
                }
            );
            Ok(())
        }
    }
}
