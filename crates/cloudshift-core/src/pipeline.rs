//! Parallelism-first orchestration (fan-out/fan-in, DAG).
//!
//! Architectural Intent:
//! This module coordinates the full transformation pipeline using a DAG-style
//! execution model. Independent operations (ingestion, catalogue loading)
//! run concurrently, then pattern matching fans out across files using rayon.
//!
//! Pipeline stages:
//! 1. Ingestion — walk file tree, detect languages (parallel discovery)
//! 2. Catalogue loading — load and compile TOML pattern files (concurrent with ingestion)
//! 3. Analysis — per-file semantic extraction via tree-sitter (parallel fan-out)
//! 4. Pattern matching — match compiled patterns against each file (parallel fan-out)
//! 5. Transformation — apply matches, produce replacement text
//! 6. Diff emission — generate diffs in the configured output format
//! 7. Aggregation — collect per-file results into a RepoReport
//!
//! Parallelisation Notes:
//! - Ingestion and catalogue loading are independent (DAG level 0) and run concurrently.
//! - Steps 3-6 are per-file and embarrassingly parallel via rayon.
//! - Step 7 is a sequential fan-in (reduction).

use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::analyser::SemanticAnalyser;
use crate::catalogue::Catalogue;
use crate::diff::DiffGenerator;
use crate::domain::entities::{
    FileChange, PatternMatch, RepoReport, TransformResult, Warning, WarningSeverity,
};
use crate::domain::events::{DomainEvent, EventKind};
use crate::domain::ports::{DiffEmitterPort, PatternMatcherPort, SemanticAnalyserPort};
use crate::domain::services::{ImportManager, TransformApplicator};
use crate::domain::value_objects::{
    Confidence, Language, MigrationEffort, OutputFormat, SourceCloud,
};
use crate::ingestion::{DiscoveredFile, Ingestion, IngestionConfig};
use crate::pattern::PatternEngine;

/// Configuration for a transformation run (PRD section 6.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformConfig {
    /// Source cloud provider to migrate from.
    pub source_cloud: SourceCloud,
    /// Optional language filter — only process files of this language.
    pub language_filter: Option<Language>,
    /// Dry run mode — generate diffs without applying changes.
    pub dry_run: bool,
    /// Confidence threshold for auto-applying transforms.
    /// Matches below this threshold are flagged for review.
    pub auto_apply_threshold: f64,
    /// Minimum confidence threshold for including matches in output.
    pub threshold: f64,
    /// Output format for diff generation.
    pub output_format: OutputFormat,
    /// Number of parallel workers (0 = use all available cores).
    pub parallel: usize,
    /// Include glob patterns for file discovery.
    pub include_globs: Vec<String>,
    /// Exclude glob patterns for file discovery.
    pub exclude_globs: Vec<String>,
    /// Skip Infrastructure-as-Code files (Terraform/HCL).
    pub no_iac: bool,
    /// Skip CI/CD files (Dockerfile, GitHub Actions YAML).
    pub no_ci: bool,
    /// Path to write the report file.
    pub report_path: Option<String>,
    /// Path to the pattern catalogue directory.
    pub catalogue_path: Option<String>,
    /// Enable LLM-assisted fallback for remaining cloud references.
    pub llm_fallback: bool,
    /// API key for the LLM service (Claude API).
    pub llm_api_key: Option<String>,
    /// LLM model to use for fallback.
    pub llm_model: Option<String>,
}

impl Default for TransformConfig {
    fn default() -> Self {
        Self {
            source_cloud: SourceCloud::Any,
            language_filter: None,
            dry_run: true,
            auto_apply_threshold: 0.90,
            threshold: 0.0,
            output_format: OutputFormat::Diff,
            parallel: 0,
            include_globs: Vec::new(),
            exclude_globs: Vec::new(),
            no_iac: false,
            no_ci: false,
            report_path: None,
            catalogue_path: None,
            llm_fallback: false,
            llm_api_key: None,
            llm_model: None,
        }
    }
}

/// Maximum file size allowed for transformation (10 MB), consistent with the
/// ingestion module's default limit.
const MAX_FILE_SIZE: u64 = 10 * 1024 * 1024;

/// Validate that `path` does not escape `root` via path traversal.
///
/// Canonicalises both paths and checks that the resolved path is a
/// descendant of `root`.  Returns the canonicalised path on success.
fn validate_path(path: &Path, root: &Path) -> anyhow::Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Cannot resolve path {}: {}", path.display(), e))?;
    let canonical_root = root
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Cannot resolve root {}: {}", root.display(), e))?;
    if !canonical.starts_with(&canonical_root) {
        anyhow::bail!(
            "Path traversal detected: {} escapes root {}",
            canonical.display(),
            canonical_root.display()
        );
    }
    Ok(canonical)
}

/// Transform a single file through the full pipeline.
///
/// Pipeline: parse → analyse → match patterns → apply transforms → emit diff.
///
/// This function operates on file content provided as a string. For file-path
/// based transformation, use `transform_file` which handles I/O.
#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip(source, patterns, llm_fallback), level = "debug")]
fn transform_source(
    path: &str,
    source: &str,
    language: Language,
    source_cloud: SourceCloud,
    patterns: &[crate::domain::entities::CompiledPattern],
    output_format: OutputFormat,
    threshold: f64,
    llm_fallback: Option<&dyn crate::domain::ports::LlmFallbackPort>,
    project_root: Option<&Path>,
) -> TransformResult {
    let analyser = SemanticAnalyser::new();
    let matcher = PatternEngine::new();
    let differ = DiffGenerator::new();
    let source_bytes = source.as_bytes();

    // Stage 1: Semantic analysis — extract cloud constructs
    let constructs = match analyser.analyse(source_bytes, language) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("Analysis failed for {path}: {e}");
            return TransformResult::new(
                path.to_string(),
                language,
                String::new(),
                Vec::new(),
                Confidence::new(0.0),
                vec![Warning {
                    message: format!("Analysis failed: {e}"),
                    span: None,
                    severity: WarningSeverity::Error,
                }],
            );
        }
    };

    if constructs.is_empty() {
        return TransformResult::new(
            path.to_string(),
            language,
            String::new(),
            Vec::new(),
            Confidence::new(1.0),
            Vec::new(),
        );
    }

    // Stage 2: Pattern matching
    let mut matches: Vec<PatternMatch> =
        matcher.match_patterns(source_bytes, language, source_cloud, patterns);

    // Filter by threshold
    matches.retain(|m| m.confidence.value() >= threshold);

    if matches.is_empty() {
        return TransformResult::new(
            path.to_string(),
            language,
            String::new(),
            Vec::new(),
            Confidence::new(1.0),
            Vec::new(),
        );
    }

    // Stage 3: Apply transformations
    let transformed = TransformApplicator::apply_all(source, &mut matches);

    // Stage 4: Apply import changes
    let imports_to_add: Vec<String> = matches
        .iter()
        .flat_map(|m| m.import_add.iter().cloned())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    let imports_to_remove: Vec<String> = matches
        .iter()
        .flat_map(|m| m.import_remove.iter().cloned())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let final_source = if imports_to_add.is_empty() && imports_to_remove.is_empty() {
        transformed
    } else {
        ImportManager::update_imports(&transformed, language, &imports_to_add, &imports_to_remove)
    };

    // Stage 5: Post-transform fixups — make output runnable on GCP
    let final_source = crate::fixup::apply_fixups(&final_source, language);

    // Stage 5b: LLM fallback — if code still has cloud references, invoke LLM
    let mut llm_warnings: Vec<Warning> = Vec::new();
    let pre_llm_source = final_source.clone();
    let mut llm_was_used = false;
    let final_source = match llm_fallback {
        Some(llm) if crate::llm_fallback::detector::needs_llm_fallback(&final_source, language) => {
            let remaining =
                crate::llm_fallback::detector::detect_remaining_cloud_refs(&final_source, language);
            tracing::info!(
                "LLM fallback: {} remaining cloud references",
                remaining.len()
            );

            let context = crate::domain::ports::LlmFallbackContext {
                applied_patterns: matches.iter().map(|m| m.pattern_id.to_string()).collect(),
                remaining_references: remaining.iter().map(|r| r.line_content.clone()).collect(),
                original_source: source.to_string(),
            };

            match llm.complete_migration(&final_source, language, source_cloud, &context) {
                Ok(completed) => {
                    tracing::info!("LLM fallback completed successfully");
                    llm_warnings.push(Warning {
                        message: "LLM-assisted fallback was used to complete this migration. Review carefully.".into(),
                        span: None,
                        severity: WarningSeverity::Info,
                    });
                    llm_was_used = true;
                    completed
                }
                Err(e) => {
                    tracing::warn!("LLM fallback failed: {e}");
                    llm_warnings.push(Warning {
                        message: format!(
                            "LLM fallback failed: {e}. Remaining cloud references may need manual migration."
                        ),
                        span: None,
                        severity: WarningSeverity::Warning,
                    });
                    final_source
                }
            }
        }
        _ => final_source,
    };

    // Stage 5c: Pattern learning — extract what the LLM did and generate candidates
    // This runs ONLY when LLM fallback was used and produced different output
    if llm_was_used {
        let deltas = crate::learning::extract_llm_delta(&pre_llm_source, &final_source);
        if !deltas.is_empty() {
            let analyzed = crate::learning::analyze_changes(&deltas, language);
            let candidates: Vec<_> = analyzed
                .iter()
                .filter(|a| a.change_type != crate::learning::analyzer::ChangeType::Unknown)
                .map(|a| crate::learning::generate_candidate_pattern(a, language, path))
                .collect();

            if !candidates.is_empty() {
                tracing::info!(
                    "Generated {} candidate patterns from LLM fallback",
                    candidates.len()
                );

                // Persist candidates to the learned/ directory
                if let Some(root) = project_root {
                    let store = crate::learning::PatternStore::from_root(root);
                    let results = store.save_candidates(&candidates);
                    let saved = results.iter().filter(|r| r.is_ok()).count();
                    let failed = results.iter().filter(|r| r.is_err()).count();
                    if failed > 0 {
                        tracing::warn!(
                            "Failed to save {} of {} candidate patterns",
                            failed,
                            candidates.len()
                        );
                    }
                    llm_warnings.push(Warning {
                        message: format!(
                            "Saved {} candidate pattern(s) from LLM fallback. Run 'cloudshift catalogue pending' to review.",
                            saved
                        ),
                        span: None,
                        severity: WarningSeverity::Info,
                    });
                } else {
                    llm_warnings.push(Warning {
                        message: format!(
                            "Generated {} candidate pattern(s) from LLM fallback (not persisted — no project root). Run with --project-root to enable learning.",
                            candidates.len()
                        ),
                        span: None,
                        severity: WarningSeverity::Info,
                    });
                }
            }
        }
    }

    // Stage 6: Generate diff
    let diff = match output_format {
        OutputFormat::Diff => differ.emit_unified_diff(path, source, &final_source),
        OutputFormat::Json => differ.emit_json_diff(path, source, &final_source),
        OutputFormat::Sarif => String::new(), // SARIF is generated at report level
    };

    // Calculate aggregate confidence
    let avg_confidence = if matches.is_empty() {
        Confidence::new(1.0)
    } else {
        let sum: f64 = matches.iter().map(|m| m.confidence.value()).sum();
        Confidence::new(sum / matches.len() as f64)
    };

    // Collect warnings for low-confidence matches
    let mut warnings: Vec<Warning> = matches
        .iter()
        .filter(|m| m.confidence.is_low())
        .map(|m| Warning {
            message: format!(
                "Low confidence match ({:.0}%): pattern {}",
                m.confidence.value() * 100.0,
                m.pattern_id
            ),
            span: Some(m.span),
            severity: WarningSeverity::Warning,
        })
        .collect();

    // Add warnings for patterns with known semantic gaps.
    // These patterns produce syntactically valid output but the surrounding
    // code often needs manual updates (e.g. response dict access after a call
    // that now returns bytes directly).
    let known_gaps: &[(&str, &str)] = &[
        (
            "aws.s3.get_object",
            "GCS download_as_bytes() returns bytes directly \u{2014} update code that accesses response['Body'].read()",
        ),
        (
            "aws.s3.list_objects",
            "GCS list_blobs() returns Blob objects \u{2014} update code that accesses response['Contents']",
        ),
        (
            "aws.s3.generate_presigned_url",
            "Review presigned URL parameters \u{2014} GCS uses different method signature; replace BUCKET_NAME/KEY placeholders",
        ),
        (
            "aws.s3.copy_object",
            "Review copy_blob() arguments \u{2014} GCS uses different parameter structure for copy operations",
        ),
        (
            "azure.blob.download_blob",
            "Azure download_blob().readall() chain is only partially transformed \u{2014} review surrounding .readall() calls",
        ),
        (
            "azure.blob.upload_blob",
            "Replace CONTAINER_NAME and BLOB_NAME placeholders with actual values from the original container/blob client",
        ),
        (
            "azure.keyvault.get_secret",
            "GCP Secret Manager returns payload.data bytes \u{2014} update code that accesses .value property",
        ),
    ];

    for pattern_match in &matches {
        for &(pattern_prefix, message) in known_gaps {
            if pattern_match
                .pattern_id
                .as_str()
                .starts_with(pattern_prefix)
            {
                warnings.push(Warning {
                    message: message.to_string(),
                    span: Some(pattern_match.span),
                    severity: WarningSeverity::Warning,
                });
            }
        }
    }

    // Append LLM fallback warnings
    warnings.extend(llm_warnings);

    TransformResult::new(
        path.to_string(),
        language,
        diff,
        matches,
        avg_confidence,
        warnings,
    )
}

/// Transform a single file, reading it from disk.
///
/// Orchestrates: read file → detect language → parse → analyse →
/// match patterns → apply transforms → emit diff.
#[tracing::instrument(skip(config), level = "info")]
pub fn transform_file(path: &str, config: &TransformConfig) -> anyhow::Result<TransformResult> {
    // Validate path does not escape the working directory
    let file_path = Path::new(path);
    let root = std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("Cannot determine current directory: {e}"))?;
    let canonical = validate_path(file_path, &root)?;

    // Check file size before reading
    let metadata =
        std::fs::metadata(&canonical).map_err(|e| anyhow::anyhow!("Cannot stat {path}: {e}"))?;
    if metadata.len() > MAX_FILE_SIZE {
        anyhow::bail!(
            "File {} is too large ({} bytes, max {} bytes)",
            path,
            metadata.len(),
            MAX_FILE_SIZE
        );
    }

    // Read the file
    let source = std::fs::read_to_string(&canonical)
        .map_err(|e| anyhow::anyhow!("Failed to read {path}: {e}"))?;

    // Detect language
    let filename = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path);
    let language = Language::from_filename(filename)
        .ok_or_else(|| anyhow::anyhow!("Cannot detect language for {path}"))?;

    // Apply language filter
    if let Some(filter_lang) = config.language_filter {
        if language != filter_lang {
            return Ok(TransformResult::new(
                path.to_string(),
                language,
                String::new(),
                Vec::new(),
                Confidence::new(1.0),
                Vec::new(),
            ));
        }
    }

    // Load catalogue
    let catalogue = load_catalogue(config)?;
    let patterns = catalogue.all_patterns();

    // Run the transformation pipeline
    let result = transform_source(
        path,
        &source,
        language,
        config.source_cloud,
        patterns,
        config.output_format,
        config.threshold,
        None, // LLM fallback is wired at CLI level
        Some(&root),
    );

    Ok(result)
}

/// Transform an entire repository.
///
/// DAG-style orchestration:
/// - Level 0 (concurrent): ingestion + catalogue loading
/// - Level 1 (parallel fan-out): per-file transform_source
/// - Level 2 (sequential fan-in): aggregate into RepoReport
#[tracing::instrument(skip(config), level = "info")]
pub fn transform_repo(path: &str, config: &TransformConfig) -> anyhow::Result<RepoReport> {
    let root = Path::new(path);

    // Configure the rayon thread pool if a specific parallel count was requested
    if config.parallel > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(config.parallel)
            .build_global()
            .ok(); // Ignore error if pool was already initialized
    }

    // === DAG Level 0: Concurrent ingestion + catalogue loading ===
    // Since we're in a sync context, we run these sequentially but both
    // are independent. In production, these could be parallelized with
    // rayon::join or async tasks.
    let (files, catalogue) = rayon::join(
        || {
            // Ingestion
            let ingestion_config = IngestionConfig {
                include_patterns: config.include_globs.clone(),
                exclude_patterns: if config.exclude_globs.is_empty() {
                    IngestionConfig::default().exclude_patterns
                } else {
                    config.exclude_globs.clone()
                },
                no_iac: config.no_iac,
                no_ci: config.no_ci,
                ..IngestionConfig::default()
            };
            let ingestion = Ingestion::new(ingestion_config);
            ingestion.discover_files(root)
        },
        || {
            // Catalogue loading
            load_catalogue(config)
        },
    );

    let files = files.map_err(|e| anyhow::anyhow!("Ingestion failed: {e}"))?;
    let catalogue = catalogue?;
    let all_patterns = catalogue.all_patterns().to_vec();

    // Log catalogue warnings
    for warn in catalogue.warnings() {
        tracing::warn!("Catalogue warning: {} — {}", warn.file, warn.message);
    }

    tracing::info!(
        "Starting transformation: {} files, {} patterns",
        files.len(),
        all_patterns.len()
    );

    // Apply language filter
    let files: Vec<DiscoveredFile> = files
        .into_iter()
        .filter(|f| {
            config
                .language_filter
                .is_none_or(|filter| f.language == filter)
        })
        .collect();

    // === DAG Level 1: Parallel fan-out — transform each file ===
    let results: Vec<(TransformResult, DiscoveredFile)> = files
        .par_iter()
        .filter_map(|file| {
            // Validate path stays within the repository root
            let file_path = Path::new(&file.path);
            let canonical = match validate_path(file_path, root) {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("Skipping {}: {e}", file.path);
                    return None;
                }
            };

            let source = match std::fs::read_to_string(&canonical) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Failed to read {}: {e}", file.path);
                    return None;
                }
            };

            let result = transform_source(
                &file.path,
                &source,
                file.language,
                config.source_cloud,
                &all_patterns,
                config.output_format,
                config.threshold,
                None, // LLM fallback is wired at CLI level
                Some(root),
            );

            Some((result, file.clone()))
        })
        .collect();

    // === DAG Level 2: Sequential fan-in — aggregate results ===
    let mut changes = Vec::new();
    let mut domain_events = Vec::new();

    for (result, _file) in &results {
        if result.has_changes() {
            // Emit domain events for each pattern match
            for pattern_match in &result.patterns {
                domain_events.push(DomainEvent::new(EventKind::PatternMatched {
                    file_path: result.path.clone(),
                    pattern_id: pattern_match.pattern_id.clone(),
                    span_start: pattern_match.span.start_byte,
                    span_end: pattern_match.span.end_byte,
                }));
            }

            // Emit file-level event
            domain_events.push(DomainEvent::new(EventKind::FileAnalysed {
                file_path: result.path.clone(),
                language: result.language,
                constructs_found: result.pattern_count(),
            }));

            changes.push(FileChange {
                file: result.path.clone(),
                language: result.language,
                constructs_detected: result.pattern_count(),
                patterns_matched: result.pattern_count(),
                confidence: result.confidence,
                effort: MigrationEffort::from_confidence(result.confidence),
                diff: result.diff.clone(),
            });
        }
    }

    // Build the report
    let mut report = RepoReport::new(path.to_string(), changes);

    // Add repo-level domain event
    let scan_event = DomainEvent::new(EventKind::RepoScanCompleted {
        repo_path: path.to_string(),
        files_scanned: results.len(),
        patterns_matched: report.total_patterns_matched,
    });
    report = report.with_event(scan_event);

    // Add all domain events
    for event in domain_events {
        report = report.with_event(event);
    }

    // Generate SARIF output if requested
    if config.output_format == OutputFormat::Sarif {
        let differ = DiffGenerator::new();
        let all_results: Vec<TransformResult> = results.into_iter().map(|(r, _)| r).collect();
        let sarif = differ.emit_sarif(&all_results);
        tracing::info!("Generated SARIF output ({} bytes)", sarif.len());
    }

    tracing::info!(
        "Transformation complete: {} files changed, {} patterns matched, avg confidence {:.2}",
        report.changes.len(),
        report.total_patterns_matched,
        report.average_confidence.value()
    );

    Ok(report)
}

/// Run the learning pipeline on a before/after pair of code.
///
/// This is the public API for manual learning — given the pattern engine's output
/// and the LLM's corrected output, it extracts deltas, analyzes them, generates
/// candidate patterns, and persists them to the `learned/` directory.
///
/// Returns the number of candidate patterns saved.
pub fn learn_from_diff(
    pattern_output: &str,
    llm_output: &str,
    language: Language,
    source_file: &str,
    project_root: &Path,
) -> anyhow::Result<usize> {
    let deltas = crate::learning::extract_llm_delta(pattern_output, llm_output);
    if deltas.is_empty() {
        return Ok(0);
    }

    let analyzed = crate::learning::analyze_changes(&deltas, language);
    let candidates: Vec<_> = analyzed
        .iter()
        .filter(|a| a.change_type != crate::learning::analyzer::ChangeType::Unknown)
        .map(|a| crate::learning::generate_candidate_pattern(a, language, source_file))
        .collect();

    if candidates.is_empty() {
        return Ok(0);
    }

    let store = crate::learning::PatternStore::from_root(project_root);
    let results = store.save_candidates(&candidates);
    let saved = results.iter().filter(|r| r.is_ok()).count();
    let failed: Vec<_> = results.iter().filter_map(|r| r.as_ref().err()).collect();

    for err in &failed {
        tracing::warn!("Failed to save candidate pattern: {err}");
    }

    Ok(saved)
}

/// Load the pattern catalogue from the configured path or return an empty catalogue.
fn load_catalogue(config: &TransformConfig) -> anyhow::Result<Catalogue> {
    match &config.catalogue_path {
        Some(cat_path) => {
            let path = Path::new(cat_path);
            Catalogue::from_directory(path)
                .map_err(|e| anyhow::anyhow!("Failed to load catalogue from {cat_path}: {e}"))
        }
        None => Ok(Catalogue::new()),
    }
}
