//! Catalogue command — manage and query the GCP Pattern Catalogue.

use anyhow::Result;
use clap::{Args, Subcommand};
use tracing::info;

use crate::commands::transform::{LanguageFilter, SourceCloudFilter};

/// Manage and query the GCP Pattern Catalogue.
#[derive(Args, Debug)]
pub struct CatalogueArgs {
    #[command(subcommand)]
    pub subcommand: CatalogueSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum CatalogueSubcommand {
    /// List all patterns in the catalogue.
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
    Search {
        /// Search query.
        query: String,
    },

    /// Show details of a specific pattern by ID.
    Info {
        /// Pattern ID.
        id: String,
    },

    /// Show catalogue statistics.
    Stats,
}

pub fn run(args: CatalogueArgs) -> Result<()> {
    match args.subcommand {
        CatalogueSubcommand::List { language, source, tag } => {
            info!(
                language = ?language,
                source = ?source,
                tag = ?tag,
                "Listing catalogue patterns"
            );
            // Stub: will query PatternRepositoryPort once wired up.
            eprintln!("catalogue list: not yet implemented (catalogue loading pending)");
        }
        CatalogueSubcommand::Search { ref query } => {
            info!(query = %query, "Searching catalogue");
            eprintln!("catalogue search: not yet implemented (catalogue loading pending)");
        }
        CatalogueSubcommand::Info { ref id } => {
            info!(id = %id, "Showing pattern info");
            eprintln!("catalogue info: not yet implemented (catalogue loading pending)");
        }
        CatalogueSubcommand::Stats => {
            info!("Showing catalogue stats");
            eprintln!("catalogue stats: not yet implemented (catalogue loading pending)");
        }
    }

    Ok(())
}
