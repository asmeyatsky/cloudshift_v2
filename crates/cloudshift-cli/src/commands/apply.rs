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

/// A parsed unified diff for a single file.
struct FilePatch {
    path: String,
    hunks: Vec<Hunk>,
}

/// A single hunk within a unified diff.
struct Hunk {
    /// 1-based start line in the original file.
    old_start: usize,
    lines: Vec<HunkLine>,
}

/// A line within a diff hunk.
enum HunkLine {
    Context(String),
    Remove(()),
    Add(String),
}

/// Parse a unified diff string into per-file patches.
fn parse_unified_diff(content: &str) -> Result<Vec<FilePatch>> {
    let mut patches = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_hunks: Vec<Hunk> = Vec::new();
    let mut current_hunk_lines: Vec<HunkLine> = Vec::new();
    let mut current_old_start: usize = 0;
    let mut in_hunk = false;

    for line in content.lines() {
        if line.starts_with("--- a/") || line.starts_with("--- ") {
            // If we have a previous file, flush it
            if let Some(path) = current_path.take() {
                if in_hunk && !current_hunk_lines.is_empty() {
                    current_hunks.push(Hunk {
                        old_start: current_old_start,
                        lines: std::mem::take(&mut current_hunk_lines),
                    });
                }
                if !current_hunks.is_empty() {
                    patches.push(FilePatch {
                        path,
                        hunks: std::mem::take(&mut current_hunks),
                    });
                }
                in_hunk = false;
            }
            // We'll get the path from the +++ line
            continue;
        }

        if let Some(rest) = line.strip_prefix("+++ b/") {
            current_path = Some(rest.to_string());
            continue;
        }
        if let Some(rest) = line.strip_prefix("+++ ") {
            current_path = Some(rest.to_string());
            continue;
        }

        if line.starts_with("@@") {
            // Flush previous hunk
            if in_hunk && !current_hunk_lines.is_empty() {
                current_hunks.push(Hunk {
                    old_start: current_old_start,
                    lines: std::mem::take(&mut current_hunk_lines),
                });
            }

            // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@
            let header = line.trim_start_matches("@@ ").trim_start_matches('@');
            let parts: Vec<&str> = header.split_whitespace().collect();
            if let Some(old_part) = parts.first() {
                let old_part = old_part.trim_start_matches('-');
                let old_start_str = old_part.split(',').next().unwrap_or("1");
                current_old_start = old_start_str.parse::<usize>().unwrap_or(1);
            }
            in_hunk = true;
            continue;
        }

        if in_hunk {
            if line.starts_with('-') {
                current_hunk_lines.push(HunkLine::Remove(()));
            } else if let Some(rest) = line.strip_prefix('+') {
                current_hunk_lines.push(HunkLine::Add(rest.to_string()));
            } else if let Some(rest) = line.strip_prefix(' ') {
                current_hunk_lines.push(HunkLine::Context(rest.to_string()));
            } else if line.is_empty() {
                // Empty context line (no leading space in some diffs)
                current_hunk_lines.push(HunkLine::Context(String::new()));
            }
        }
    }

    // Flush last file
    if let Some(path) = current_path {
        if in_hunk && !current_hunk_lines.is_empty() {
            current_hunks.push(Hunk {
                old_start: current_old_start,
                lines: std::mem::take(&mut current_hunk_lines),
            });
        }
        if !current_hunks.is_empty() {
            patches.push(FilePatch {
                path,
                hunks: current_hunks,
            });
        }
    }

    Ok(patches)
}

/// Apply hunks to the original file content and return the result.
fn apply_hunks(original: &str, hunks: &[Hunk]) -> Result<String> {
    let original_lines: Vec<&str> = original.lines().collect();
    let mut result_lines: Vec<String> = Vec::new();
    // Track our position in the original file (0-based index).
    let mut orig_pos: usize = 0;

    for hunk in hunks {
        // hunk.old_start is 1-based; convert to 0-based
        let hunk_start = if hunk.old_start > 0 {
            hunk.old_start - 1
        } else {
            0
        };

        // Copy lines before this hunk
        while orig_pos < hunk_start && orig_pos < original_lines.len() {
            result_lines.push(original_lines[orig_pos].to_string());
            orig_pos += 1;
        }

        // Apply hunk lines
        for hline in &hunk.lines {
            match hline {
                HunkLine::Context(_ctx) => {
                    // Keep the original line
                    if orig_pos < original_lines.len() {
                        result_lines.push(original_lines[orig_pos].to_string());
                        orig_pos += 1;
                    }
                }
                HunkLine::Remove(_) => {
                    // Skip the original line
                    if orig_pos < original_lines.len() {
                        orig_pos += 1;
                    }
                }
                HunkLine::Add(content) => {
                    // Insert new line
                    result_lines.push(content.clone());
                }
            }
        }
    }

    // Copy remaining lines after the last hunk
    while orig_pos < original_lines.len() {
        result_lines.push(original_lines[orig_pos].to_string());
        orig_pos += 1;
    }

    // Preserve trailing newline if original had one
    let mut result = result_lines.join("\n");
    if original.ends_with('\n') {
        result.push('\n');
    }

    Ok(result)
}

pub fn run(args: ApplyArgs) -> Result<()> {
    info!(path = %args.path, dry_run = %args.dry_run, "Applying diff");

    let content = std::fs::read_to_string(&args.path)
        .with_context(|| format!("Failed to read diff file: {}", args.path))?;

    if content.trim().is_empty() {
        bail!("Diff file is empty: {}", args.path);
    }

    let patches = parse_unified_diff(&content)?;
    if patches.is_empty() {
        bail!(
            "No patches found in {}. Expected a unified diff format.",
            args.path
        );
    }

    let mut applied = 0;
    let mut skipped = 0;
    for patch in &patches {
        let target = std::path::Path::new(&patch.path);
        if !target.exists() {
            eprintln!("Warning: {} not found, skipping", patch.path);
            skipped += 1;
            continue;
        }
        let original = std::fs::read_to_string(target)
            .with_context(|| format!("Failed to read {}", patch.path))?;
        let result = apply_hunks(&original, &patch.hunks)?;
        if args.dry_run {
            println!("Would patch {} ({} hunk(s))", patch.path, patch.hunks.len());
        } else {
            std::fs::write(target, &result)
                .with_context(|| format!("Failed to write {}", patch.path))?;
            println!("Patched {} ({} hunk(s))", patch.path, patch.hunks.len());
        }
        applied += 1;
    }
    println!("\n{} file(s) patched, {} skipped", applied, skipped);
    Ok(())
}
