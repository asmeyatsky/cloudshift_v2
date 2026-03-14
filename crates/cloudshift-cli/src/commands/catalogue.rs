//! Catalogue command — manage and query the GCP Pattern Catalogue.

use anyhow::{bail, Result};
use clap::{Args, Subcommand};
use tracing::info;

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
    }
}
