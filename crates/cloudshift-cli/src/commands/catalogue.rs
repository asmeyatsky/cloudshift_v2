//! Catalogue command — manage and query the GCP Pattern Catalogue.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Result};
use clap::{Args, Subcommand};
use tracing::info;

use cloudshift_core::catalogue::Catalogue;
use cloudshift_core::domain::ports::PatternRepositoryPort;
use cloudshift_core::learning::store::PatternStore;
use cloudshift_core::{Language, PatternId, SourceCloud};

use crate::commands::{discover_catalogue_path, LanguageFilter, SourceCloudFilter};

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

/// Convert a CLI LanguageFilter to the core Language type.
/// Returns None for LanguageFilter::All (meaning no filter).
fn language_filter_to_core(filter: &LanguageFilter) -> Option<Language> {
    match filter {
        LanguageFilter::Python => Some(Language::Python),
        LanguageFilter::TypeScript => Some(Language::TypeScript),
        LanguageFilter::Java => Some(Language::Java),
        LanguageFilter::Go => Some(Language::Go),
        LanguageFilter::Hcl => Some(Language::Hcl),
        LanguageFilter::All => None,
    }
}

/// Load the catalogue from the discovered path.
fn load_catalogue() -> Result<Catalogue> {
    let cat_path = discover_catalogue_path().ok_or_else(|| {
        anyhow::anyhow!(
            "No patterns directory found (set CLOUDSHIFT_CATALOGUE_PATH or run from project root)"
        )
    })?;
    Catalogue::from_directory(std::path::Path::new(&cat_path))
        .map_err(|e| anyhow::anyhow!("Failed to load catalogue: {e:?}"))
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

            let catalogue = load_catalogue()?;
            let mut patterns = catalogue.all_patterns().to_vec();

            // Apply language filter
            if let Some(ref lang) = language {
                if let Some(core_lang) = language_filter_to_core(lang) {
                    patterns.retain(|p| p.language == core_lang);
                }
            }

            // Apply source cloud filter
            if let Some(ref src) = source {
                let core_src = src.to_core();
                if core_src != SourceCloud::Any {
                    patterns.retain(|p| p.source == core_src || p.source == SourceCloud::Any);
                }
            }

            // Apply tag filter
            if let Some(ref t) = tag {
                patterns.retain(|p| p.tags.iter().any(|pt| pt.contains(t.as_str())));
            }

            println!("{} pattern(s) found:\n", patterns.len());
            println!(
                "{:<55} {:<12} {:<8} {:<6}",
                "ID", "Language", "Source", "Conf"
            );
            println!("{}", "-".repeat(85));
            for p in &patterns {
                println!(
                    "{:<55} {:<12} {:<8} {:.2}",
                    p.id.0,
                    format!("{:?}", p.language),
                    format!("{:?}", p.source),
                    p.confidence.value()
                );
            }
            Ok(())
        }
        CatalogueSubcommand::Search { ref query } => {
            info!(query = %query, "Searching catalogue");

            let catalogue = load_catalogue()?;
            let results = catalogue.search(query);

            if results.is_empty() {
                println!("No patterns matching \"{}\".", query);
            } else {
                println!(
                    "{} pattern(s) matching \"{}\":\n",
                    results.len(),
                    query
                );
                println!(
                    "{:<55} {:<12} {:<8} {:<6}",
                    "ID", "Language", "Source", "Conf"
                );
                println!("{}", "-".repeat(85));
                for p in &results {
                    println!(
                        "{:<55} {:<12} {:<8} {:.2}",
                        p.id.0,
                        format!("{:?}", p.language),
                        format!("{:?}", p.source),
                        p.confidence.value()
                    );
                }
            }
            Ok(())
        }
        CatalogueSubcommand::Info { ref id } => {
            info!(id = %id, "Showing pattern info");

            let catalogue = load_catalogue()?;
            let pattern_id = PatternId::new(id.clone());
            match catalogue.get_by_id(&pattern_id) {
                Some(p) => {
                    println!("Pattern: {}", p.id.0);
                    println!("Description: {}", p.description);
                    println!("Language:    {:?}", p.language);
                    println!("Source:      {:?}", p.source);
                    println!("Confidence:  {:.2}", p.confidence.value());
                    println!(
                        "Tags:        {}",
                        if p.tags.is_empty() {
                            "(none)".to_string()
                        } else {
                            p.tags.join(", ")
                        }
                    );
                    println!("\nDetect query:\n  {}", p.detect_query);
                    println!("\nTransform template:\n  {}", p.transform_template);
                    if !p.import_add.is_empty() {
                        println!("\nImports to add:");
                        for imp in &p.import_add {
                            println!("  + {}", imp);
                        }
                    }
                    if !p.import_remove.is_empty() {
                        println!("\nImports to remove:");
                        for imp in &p.import_remove {
                            println!("  - {}", imp);
                        }
                    }
                    Ok(())
                }
                None => bail!("Pattern not found: {}", id),
            }
        }
        CatalogueSubcommand::Stats => {
            info!("Showing catalogue stats");

            let catalogue = load_catalogue()?;
            let patterns = catalogue.all_patterns();
            let total = catalogue.count();

            println!("Catalogue Statistics\n");
            println!("Total patterns: {}\n", total);

            // Count by language
            let mut by_language: HashMap<String, usize> = HashMap::new();
            for p in patterns {
                *by_language
                    .entry(format!("{:?}", p.language))
                    .or_insert(0) += 1;
            }
            println!("By language:");
            let mut lang_entries: Vec<_> = by_language.iter().collect();
            lang_entries.sort_by_key(|(k, _)| (*k).clone());
            for (lang, count) in &lang_entries {
                println!("  {:<15} {}", lang, count);
            }

            // Count by source cloud
            let mut by_source: HashMap<String, usize> = HashMap::new();
            for p in patterns {
                *by_source.entry(format!("{:?}", p.source)).or_insert(0) += 1;
            }
            println!("\nBy source cloud:");
            let mut source_entries: Vec<_> = by_source.iter().collect();
            source_entries.sort_by_key(|(k, _)| (*k).clone());
            for (src, count) in &source_entries {
                println!("  {:<15} {}", src, count);
            }

            // Average confidence
            if total > 0 {
                let avg_conf: f64 =
                    patterns.iter().map(|p| p.confidence.value()).sum::<f64>() / total as f64;
                println!("\nAverage confidence: {:.2}", avg_conf);
            }

            Ok(())
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
                println!("{} pending candidate pattern(s):\n", pending.len());
                for candidate in &pending {
                    println!(
                        "  [{}] language={}, source_file={}\n       {}",
                        candidate.candidate_id,
                        candidate.language,
                        candidate.source_file,
                        candidate.file_path.display()
                    );
                }
                println!("\nTo promote: cloudshift catalogue promote <candidate_id>");
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
