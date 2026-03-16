//! Delta extraction between pattern engine and LLM outputs.

use similar::{ChangeTag, TextDiff};

/// A single change the LLM made beyond what patterns did.
#[derive(Debug, Clone)]
pub struct LlmDelta {
    /// Line range in the pattern-engine output that was changed.
    pub original_line_start: usize,
    pub original_line_end: usize,
    /// The code the pattern engine produced (that the LLM replaced).
    pub pattern_output: String,
    /// The code the LLM produced instead.
    pub llm_output: String,
    /// Lines of context before the change.
    pub context_before: Vec<String>,
    /// Lines of context after the change.
    pub context_after: Vec<String>,
}

/// Extract the deltas between pattern-engine output and LLM-completed output.
/// Each delta represents a contiguous block of changes the LLM made.
pub fn extract_llm_delta(pattern_output: &str, llm_output: &str) -> Vec<LlmDelta> {
    let diff = TextDiff::from_lines(pattern_output, llm_output);
    let pattern_lines: Vec<&str> = pattern_output.lines().collect();
    let mut deltas = Vec::new();
    let mut current_delta: Option<DeltaBuilder> = None;

    for change in diff.iter_all_changes() {
        let line_idx = change.old_index().unwrap_or(0);

        match change.tag() {
            ChangeTag::Equal => {
                // If we were building a delta, finalize it
                if let Some(builder) = current_delta.take() {
                    let context_after: Vec<String> = (line_idx
                        ..std::cmp::min(line_idx + 3, pattern_lines.len()))
                        .map(|i| pattern_lines[i].to_string())
                        .collect();
                    deltas.push(builder.finish(context_after));
                }
            }
            ChangeTag::Delete => {
                let builder = current_delta.get_or_insert_with(|| {
                    let context_start = line_idx.saturating_sub(3);
                    let context_before: Vec<String> = (context_start..line_idx)
                        .map(|i| pattern_lines[i].to_string())
                        .collect();
                    DeltaBuilder::new(line_idx, context_before)
                });
                builder.add_pattern_line(change.value().trim_end_matches('\n'));
                builder.extend_original_end(line_idx + 1);
            }
            ChangeTag::Insert => {
                let builder = current_delta.get_or_insert_with(|| {
                    let context_start = line_idx.saturating_sub(3);
                    let context_before: Vec<String> = (context_start..line_idx)
                        .filter(|&i| i < pattern_lines.len())
                        .map(|i| pattern_lines[i].to_string())
                        .collect();
                    DeltaBuilder::new(line_idx, context_before)
                });
                builder.add_llm_line(change.value().trim_end_matches('\n'));
            }
        }
    }

    // Finalize any remaining delta
    if let Some(builder) = current_delta.take() {
        deltas.push(builder.finish(Vec::new()));
    }

    deltas
}

struct DeltaBuilder {
    original_line_start: usize,
    original_line_end: usize,
    pattern_lines: Vec<String>,
    llm_lines: Vec<String>,
    context_before: Vec<String>,
}

impl DeltaBuilder {
    fn new(start: usize, context_before: Vec<String>) -> Self {
        Self {
            original_line_start: start,
            original_line_end: start,
            pattern_lines: Vec::new(),
            llm_lines: Vec::new(),
            context_before,
        }
    }

    fn add_pattern_line(&mut self, line: &str) {
        self.pattern_lines.push(line.to_string());
    }

    fn add_llm_line(&mut self, line: &str) {
        self.llm_lines.push(line.to_string());
    }

    fn extend_original_end(&mut self, end: usize) {
        self.original_line_end = end;
    }

    fn finish(self, context_after: Vec<String>) -> LlmDelta {
        LlmDelta {
            original_line_start: self.original_line_start,
            original_line_end: self.original_line_end,
            pattern_output: self.pattern_lines.join("\n"),
            llm_output: self.llm_lines.join("\n"),
            context_before: self.context_before,
            context_after,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_single_delta() {
        let pattern =
            "import boto3\n\ns3 = boto3.client('s3')\ns3.put_object(Bucket='b', Key='k', Body='d')\n";
        let llm = "from google.cloud import storage\n\nclient = storage.Client()\nbucket = client.bucket('b')\nblob = bucket.blob('k')\nblob.upload_from_string('d')\n";

        let deltas = extract_llm_delta(pattern, llm);
        assert!(!deltas.is_empty());
    }

    #[test]
    fn test_identical_produces_no_deltas() {
        let code = "from google.cloud import storage\nclient = storage.Client()\n";
        let deltas = extract_llm_delta(code, code);
        assert!(deltas.is_empty());
    }
}
