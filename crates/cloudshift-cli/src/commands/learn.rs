//! Learn command — manually trigger pattern learning from before/after code pairs.

use anyhow::Result;
use clap::Args;
use tracing::info;

use cloudshift_core::domain::value_objects::Language;
use cloudshift_core::pipeline::learn_from_diff;

/// Learn new patterns from before/after code pairs.
#[derive(Args, Debug)]
pub struct LearnArgs {
    /// Path to the "before" file (pattern engine output).
    #[arg(long)]
    pub before: String,

    /// Path to the "after" file (LLM-corrected output).
    #[arg(long)]
    pub after: String,

    /// Language of the source files.
    #[arg(long, value_enum)]
    pub language: LanguageArg,

    /// Label for the source file (used in generated pattern metadata).
    #[arg(long, default_value = "manual")]
    pub source_label: String,

    /// Project root directory (defaults to current directory).
    #[arg(long)]
    pub project_root: Option<String>,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum LanguageArg {
    Python,
    #[value(name = "typescript")]
    TypeScript,
    Java,
    Go,
    Hcl,
}

impl LanguageArg {
    fn to_core(&self) -> Language {
        match self {
            Self::Python => Language::Python,
            Self::TypeScript => Language::TypeScript,
            Self::Java => Language::Java,
            Self::Go => Language::Go,
            Self::Hcl => Language::Hcl,
        }
    }
}

pub fn run(args: LearnArgs) -> Result<()> {
    info!(
        before = %args.before,
        after = %args.after,
        language = ?args.language,
        "Running manual pattern learning"
    );

    let before = std::fs::read_to_string(&args.before)
        .map_err(|e| anyhow::anyhow!("Failed to read before file '{}': {e}", args.before))?;
    let after = std::fs::read_to_string(&args.after)
        .map_err(|e| anyhow::anyhow!("Failed to read after file '{}': {e}", args.after))?;

    let root = match &args.project_root {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_dir()?,
    };

    let language = args.language.to_core();
    let saved = learn_from_diff(&before, &after, language, &args.source_label, &root)?;

    if saved == 0 {
        println!("No learnable patterns found in the diff.");
    } else {
        println!(
            "Saved {} candidate pattern(s) to {}/learned/",
            saved,
            root.display()
        );
        println!("Run 'cloudshift catalogue pending' to review them.");
    }

    Ok(())
}
