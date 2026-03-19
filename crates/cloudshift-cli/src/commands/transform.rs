//! Transform command — transforms code files or repositories to GCP.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use cloudshift_core::Language;
use cloudshift_core::{transform_file, transform_repo, OutputFormat, TransformConfig};

use crate::commands::{LanguageFilter, SourceCloudFilter};
use crate::output;

/// Preset migration scenarios (set source cloud and optionally language).
#[derive(Debug, Clone, Default, clap::ValueEnum)]
pub enum TransformPreset {
    #[default]
    #[value(name = "none")]
    None,
    /// AWS Lambda + DynamoDB → Cloud Functions + Firestore (Python).
    #[value(name = "aws-lambda-dynamodb")]
    AwsLambdaDynamodb,
    /// AWS S3 → GCS (any language in path).
    #[value(name = "aws-s3")]
    AwsS3,
    /// Azure Blob Storage → GCS (Python).
    #[value(name = "azure-blob")]
    AzureBlob,
}

/// CLI output format for the transform command.
#[derive(Debug, Clone, clap::ValueEnum)]
pub enum TransformOutputFormat {
    Diff,
    Json,
    Sarif,
}

impl TransformOutputFormat {
    pub(crate) fn to_core(&self) -> OutputFormat {
        match self {
            Self::Diff => OutputFormat::Diff,
            Self::Json => OutputFormat::Json,
            Self::Sarif => OutputFormat::Sarif,
        }
    }
}

/// Transform code files or repositories from AWS/Azure to GCP.
#[derive(Args, Debug)]
#[command(about = "Transform code files or repositories from AWS/Azure to GCP")]
pub struct TransformArgs {
    /// File, directory, or Git repo URL (default: current directory).
    #[arg(default_value = ".")]
    pub path: String,

    /// Preset migration scenario (overrides --source and optionally --language when set).
    #[arg(long, value_enum, default_value = "none")]
    pub preset: TransformPreset,

    /// Source cloud provider to migrate from.
    #[arg(long = "source", value_enum, default_value = "any")]
    pub source_cloud: SourceCloudFilter,

    /// Restrict transformation to a specific language.
    #[arg(long = "language", value_enum)]
    pub language: Option<LanguageFilter>,

    /// Show diff without applying changes.
    #[arg(long, default_value = "true")]
    pub dry_run: bool,

    /// Auto-apply high-confidence transformations (>= 0.90).
    #[arg(long)]
    pub auto: bool,

    /// Minimum confidence to include in diff.
    #[arg(long, default_value = "0.70")]
    pub threshold: f64,

    /// Output format.
    #[arg(long = "output", value_enum, default_value = "diff")]
    pub output_format: TransformOutputFormat,

    /// Write JSON migration report to path.
    #[arg(long = "report")]
    pub report_path: Option<String>,

    /// Worker threads for repo-level transforms (default: number of CPUs).
    #[arg(long)]
    pub parallel: Option<usize>,

    /// Skip Infrastructure as Code files.
    #[arg(long)]
    pub no_iac: bool,

    /// Skip CI/CD pipeline files.
    #[arg(long)]
    pub no_ci: bool,

    /// Include files matching glob pattern.
    #[arg(long)]
    pub include: Option<String>,

    /// Exclude files matching glob pattern.
    #[arg(long)]
    pub exclude: Option<String>,

    /// Only transform these files (e.g. from git diff: git diff --name-only | xargs cloudshift transform . --only-files).
    #[arg(long = "only-files", value_name = "FILE", num_args = 1..)]
    pub only_files: Option<Vec<String>>,

    /// Enable LLM-assisted fallback for remaining cloud references.
    #[arg(long)]
    pub llm_fallback: bool,
}

impl TransformArgs {
    /// Build a core TransformConfig from CLI arguments (preset overrides source/language when set).
    fn to_config(&self) -> TransformConfig {
        let (source_cloud, language_filter) = match &self.preset {
            TransformPreset::None => (self.source_cloud.to_core(), None),
            TransformPreset::AwsLambdaDynamodb => {
                (SourceCloudFilter::Aws.to_core(), Some(Language::Python))
            }
            TransformPreset::AwsS3 => (SourceCloudFilter::Aws.to_core(), None),
            TransformPreset::AzureBlob => {
                (SourceCloudFilter::Azure.to_core(), Some(Language::Python))
            }
        };
        let language_filter = language_filter.or_else(|| {
            self.language.as_ref().and_then(|l| match l {
                LanguageFilter::Python => Some(Language::Python),
                LanguageFilter::TypeScript => Some(Language::TypeScript),
                LanguageFilter::Java => Some(Language::Java),
                LanguageFilter::Go => Some(Language::Go),
                LanguageFilter::Hcl => Some(Language::Hcl),
                LanguageFilter::All => None,
            })
        });
        TransformConfig {
            source_cloud,
            language_filter,
            output_format: self.output_format.to_core(),
            dry_run: if self.auto { false } else { self.dry_run },
            parallel: self.parallel.unwrap_or(0),
            threshold: self.threshold,
            auto_apply_threshold: if self.auto { 0.90 } else { 1.0 },
            no_iac: self.no_iac,
            no_ci: self.no_ci,
            report_path: self.report_path.clone(),
            include_globs: self.include.iter().cloned().collect(),
            exclude_globs: self.exclude.iter().cloned().collect(),
            catalogue_path: crate::commands::discover_catalogue_path(),
            only_files: self.only_files.clone(),
            llm_fallback: self.llm_fallback,
            llm_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            llm_model: None,
            progress_callback: None,
        }
    }
}

pub fn run(args: TransformArgs) -> Result<()> {
    let config = args.to_config();
    let path = &args.path;

    info!(path = %path, source = %config.source_cloud, dry_run = %config.dry_run, "Starting transform");

    let target = Path::new(path);

    if target.is_file() {
        let result = transform_file(path, &config)
            .with_context(|| format!("Failed to transform file: {}", path))?;

        output::print_transform_result(&result, &args.output_format);

        if let Some(ref report_path) = args.report_path {
            let json =
                serde_json::to_string_pretty(&result).context("Failed to serialise report")?;
            std::fs::write(report_path, json)
                .with_context(|| format!("Failed to write report to {}", report_path))?;
            info!(path = %report_path, "Report written");
        }
    } else {
        let pb = indicatif::ProgressBar::new(0);
        pb.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{msg} [{bar:40.cyan/blue}] {pos}/{len}")
                .expect("template"),
        );
        pb.set_message("Transforming…");
        let progress_cb: Arc<dyn Fn(usize, usize) + Send + Sync> = Arc::new({
            let pb = pb.clone();
            move |done, total| {
                if total > 0 && pb.length().unwrap_or(0) == 0 {
                    pb.set_length(total as u64);
                }
                pb.set_position(done as u64);
                pb.set_message(format!("Transformed {} / {} files", done, total));
            }
        });
        let mut config = config;
        config.progress_callback = Some(progress_cb);

        let report = transform_repo(path, &config)
            .with_context(|| format!("Failed to transform repository: {}", path))?;

        pb.finish_with_message("Done");

        output::print_repo_report(&report, &args.output_format);

        if let Some(ref report_path) = args.report_path {
            let json =
                serde_json::to_string_pretty(&report).context("Failed to serialise report")?;
            std::fs::write(report_path, json)
                .with_context(|| format!("Failed to write report to {}", report_path))?;
            info!(path = %report_path, "Report written");
        }
    }

    Ok(())
}
