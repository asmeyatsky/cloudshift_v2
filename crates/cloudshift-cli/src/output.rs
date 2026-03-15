//! Output formatting helpers for the CLI.
//!
//! Provides table formatting for analysis results and coloured diff output.

use cloudshift_core::{RepoReport, TransformResult};

use crate::commands::transform::TransformOutputFormat;

/// Print a single file transform result in the requested format.
pub fn print_transform_result(result: &TransformResult, format: &TransformOutputFormat) {
    match format {
        TransformOutputFormat::Diff => {
            if result.diff.is_empty() {
                println!("No changes detected in {}", result.path);
            } else {
                print!("{}", result.diff);
            }

            if !result.warnings.is_empty() {
                eprintln!("\nWarnings:");
                for w in &result.warnings {
                    eprintln!("  [{:?}] {}", w.severity, w.message);
                }
            }
        }
        TransformOutputFormat::Json => {
            let json = serde_json::to_string_pretty(result)
                .expect("TransformResult is always serialisable");
            println!("{}", json);
        }
        TransformOutputFormat::Sarif => {
            // SARIF output will be handled by the core DiffEmitterPort.
            let json = serde_json::to_string_pretty(result)
                .expect("TransformResult is always serialisable");
            println!("{}", json);
        }
    }
}

/// Print a repository-level report in the requested format.
pub fn print_repo_report(report: &RepoReport, format: &TransformOutputFormat) {
    match format {
        TransformOutputFormat::Diff => {
            for change in &report.changes {
                if !change.diff.is_empty() {
                    print!("{}", change.diff);
                    println!();
                }
            }

            println!(
                "\nSummary: {} file(s), {} pattern(s) matched, avg confidence {}",
                report.changes.len(),
                report.total_patterns_matched,
                report.average_confidence,
            );
        }
        TransformOutputFormat::Json | TransformOutputFormat::Sarif => {
            let json = serde_json::to_string_pretty(report)
                .expect("RepoReport is always serialisable");
            println!("{}", json);
        }
    }
}

/// Print analysis results as a formatted table.
pub fn print_analysis_table(report: &RepoReport) {
    // Header
    println!(
        "{:<50} {:<12} {:<8} {:<10} {:<8}",
        "File", "Language", "Patterns", "Confidence", "Effort"
    );
    println!("{}", "-".repeat(88));

    for change in &report.changes {
        println!(
            "{:<50} {:<12} {:<8} {:<10} {:<8}",
            truncate_path(&change.file, 50),
            change.language,
            change.patterns_matched,
            change.confidence,
            change.effort,
        );
    }

    println!("{}", "-".repeat(88));
    println!(
        "Total: {} file(s), {} construct(s), {} pattern(s), avg confidence {}, effort: {}",
        report.changes.len(),
        report.total_constructs,
        report.total_patterns_matched,
        report.average_confidence,
        report.overall_effort,
    );
}

/// Truncate a path string to fit within a column width.
fn truncate_path(path: &str, max_width: usize) -> String {
    if path.len() <= max_width {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - (max_width - 3)..])
    }
}
