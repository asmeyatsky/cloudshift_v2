//! Diff emitter implementations.
//!
//! Uses the `similar` crate to generate unified diffs, JSON-format diffs,
//! and SARIF output for CI integration.
//!
//! Architectural Intent:
//! Pure transformation functions — takes original and transformed text,
//! produces formatted output. No I/O, no business logic.

use similar::{ChangeTag, TextDiff};
use crate::domain::entities::TransformResult;

/// Generate a unified diff from original and transformed source text.
///
/// Produces standard unified diff format compatible with `patch(1)`,
/// `git apply`, and diff viewers.
pub fn unified_diff(path: &str, original: &str, transformed: &str) -> String {
    if original == transformed {
        return String::new();
    }

    let diff = TextDiff::from_lines(original, transformed);
    let mut output = String::new();

    // Header
    output.push_str(&format!("--- a/{path}\n"));
    output.push_str(&format!("+++ b/{path}\n"));

    // Generate hunks with 3 lines of context
    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        output.push_str(&format!("{hunk}"));
    }

    output
}

/// Generate a JSON-format diff.
///
/// Produces a structured JSON representation of the changes, useful for
/// machine consumption and API responses.
pub fn json_diff(path: &str, original: &str, transformed: &str) -> String {
    if original == transformed {
        return serde_json::json!({
            "path": path,
            "changed": false,
            "hunks": []
        })
        .to_string();
    }

    let diff = TextDiff::from_lines(original, transformed);
    let mut hunks = Vec::new();

    for (idx, group) in diff.grouped_ops(3).iter().enumerate() {
        let mut changes = Vec::new();
        for op in group {
            for change in diff.iter_changes(op) {
                let tag = match change.tag() {
                    ChangeTag::Equal => "equal",
                    ChangeTag::Insert => "insert",
                    ChangeTag::Delete => "delete",
                };
                changes.push(serde_json::json!({
                    "tag": tag,
                    "old_index": change.old_index(),
                    "new_index": change.new_index(),
                    "value": change.value().trim_end_matches('\n'),
                }));
            }
        }

        hunks.push(serde_json::json!({
            "hunk_index": idx,
            "changes": changes,
        }));
    }

    serde_json::json!({
        "path": path,
        "changed": true,
        "hunks": hunks,
    })
    .to_string()
}

/// Generate SARIF output for CI integration.
///
/// Produces SARIF v2.1.0 compliant output that can be consumed by GitHub
/// Code Scanning, Azure DevOps, and other CI tools.
pub fn sarif_output(results: &[TransformResult]) -> String {
    let mut sarif_results = Vec::new();

    for result in results {
        for pattern_match in &result.patterns {
            sarif_results.push(serde_json::json!({
                "ruleId": pattern_match.pattern_id.as_str(),
                "level": if pattern_match.confidence.is_high() {
                    "note"
                } else if pattern_match.confidence.is_medium() {
                    "warning"
                } else {
                    "error"
                },
                "message": {
                    "text": format!(
                        "Cloud SDK migration: {} (confidence: {:.0}%)",
                        pattern_match.pattern_id,
                        pattern_match.confidence.value() * 100.0
                    )
                },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {
                            "uri": result.path,
                        },
                        "region": {
                            "startLine": pattern_match.span.start_row + 1,
                            "startColumn": pattern_match.span.start_col + 1,
                            "endLine": pattern_match.span.end_row + 1,
                            "endColumn": pattern_match.span.end_col + 1,
                        }
                    }
                }],
                "fixes": [{
                    "description": {
                        "text": format!("Replace with GCP equivalent"),
                    },
                    "artifactChanges": [{
                        "artifactLocation": {
                            "uri": result.path,
                        },
                        "replacements": [{
                            "deletedRegion": {
                                "startLine": pattern_match.span.start_row + 1,
                                "startColumn": pattern_match.span.start_col + 1,
                                "endLine": pattern_match.span.end_row + 1,
                                "endColumn": pattern_match.span.end_col + 1,
                            },
                            "insertedContent": {
                                "text": &pattern_match.replacement_text,
                            }
                        }]
                    }]
                }]
            }));
        }
    }

    let sarif = serde_json::json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/main/sarif-2.1/schema/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "CloudShift",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/smeyatsky-labs/cloudshift",
                    "rules": collect_rules(results),
                }
            },
            "results": sarif_results,
        }]
    });

    serde_json::to_string_pretty(&sarif).unwrap_or_else(|_| "{}".to_string())
}

/// Collect unique pattern rules from transform results for SARIF rule definitions.
fn collect_rules(results: &[TransformResult]) -> Vec<serde_json::Value> {
    let mut seen = std::collections::HashSet::new();
    let mut rules = Vec::new();

    for result in results {
        for pattern_match in &result.patterns {
            let id = pattern_match.pattern_id.as_str().to_string();
            if seen.insert(id.clone()) {
                rules.push(serde_json::json!({
                    "id": id,
                    "shortDescription": {
                        "text": format!("Cloud SDK migration pattern: {id}"),
                    },
                    "defaultConfiguration": {
                        "level": "warning",
                    },
                }));
            }
        }
    }

    rules
}

/// Calculate a summary of changes between two texts.
pub fn change_summary(original: &str, transformed: &str) -> ChangeSummary {
    let diff = TextDiff::from_lines(original, transformed);
    let mut additions = 0;
    let mut deletions = 0;
    let mut modifications = 0;

    for op in diff.ops() {
        match op.tag() {
            similar::DiffTag::Insert => additions += op.new_range().len(),
            similar::DiffTag::Delete => deletions += op.old_range().len(),
            similar::DiffTag::Replace => {
                modifications += op.old_range().len().max(op.new_range().len());
            }
            similar::DiffTag::Equal => {}
        }
    }

    ChangeSummary {
        additions,
        deletions,
        modifications,
    }
}

/// Summary of changes between two texts.
#[derive(Debug, Clone, Copy)]
pub struct ChangeSummary {
    pub additions: usize,
    pub deletions: usize,
    pub modifications: usize,
}

impl ChangeSummary {
    pub fn total_changes(&self) -> usize {
        self.additions + self.deletions + self.modifications
    }
}
