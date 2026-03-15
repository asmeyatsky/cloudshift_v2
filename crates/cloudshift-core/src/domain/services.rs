use crate::domain::entities::*;
use crate::domain::value_objects::*;

/// Domain service: Calculates confidence scores from weighted factors.
pub struct ConfidenceCalculator;

impl ConfidenceCalculator {
    /// Calculate confidence for a pattern match based on the four-factor model.
    pub fn calculate(
        pattern: &CompiledPattern,
        binding_completeness: f64,
        version_match: bool,
    ) -> Confidence {
        let pattern_specificity = pattern.confidence.value();
        let version_alignment = if version_match { 1.0 } else { 0.5 };
        let test_coverage = if !pattern.tags.is_empty() { 0.8 } else { 0.5 };

        Confidence::from_factors(
            pattern_specificity,
            version_alignment,
            binding_completeness,
            test_coverage,
        )
    }
}

/// Domain service: Applies pattern transformations to source text.
pub struct TransformApplicator;

impl TransformApplicator {
    /// Apply a single pattern match to source text, producing the transformed text.
    /// Returns None if the transformation cannot be applied.
    pub fn apply_match(source: &str, pattern_match: &PatternMatch) -> Option<String> {
        let span = &pattern_match.span;
        if span.start_byte > source.len() || span.end_byte > source.len() {
            return None;
        }

        let mut result = String::with_capacity(source.len());
        result.push_str(&source[..span.start_byte]);
        result.push_str(&pattern_match.replacement_text);
        result.push_str(&source[span.end_byte..]);
        Some(result)
    }

    /// Apply all pattern matches to source text, deduplicating overlapping spans.
    ///
    /// When matches overlap, keeps the highest-confidence match and discards
    /// lower-confidence ones that touch the same byte range. This prevents
    /// garbled output from conflicting pattern transformations.
    ///
    /// Returns the transformed text and the matches that were actually applied
    /// (filtered to non-overlapping). The caller should only use import changes
    /// from the returned matches.
    pub fn apply_all(source: &str, matches: &mut Vec<PatternMatch>) -> String {
        // Sort by confidence descending, then by span start ascending
        matches.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.span.start_byte.cmp(&b.span.start_byte))
        });

        // Greedily select non-overlapping matches (highest confidence first)
        let mut keep = vec![false; matches.len()];
        let mut occupied: Vec<(usize, usize)> = Vec::new();
        for (i, m) in matches.iter().enumerate() {
            let overlaps = occupied.iter().any(|&(start, end)| {
                m.span.start_byte < end && m.span.end_byte > start
            });
            if !overlaps {
                keep[i] = true;
                occupied.push((m.span.start_byte, m.span.end_byte));
            }
        }

        // Filter matches to only those that were kept
        let mut i = 0;
        matches.retain(|_| {
            let k = keep[i];
            i += 1;
            k
        });

        // Sort by start position descending to apply from end to start
        matches.sort_by(|a, b| b.span.start_byte.cmp(&a.span.start_byte));

        let mut result = source.to_string();
        for m in matches.iter() {
            if let Some(transformed) = Self::apply_match(&result, m) {
                result = transformed;
            }
        }
        result
    }
}

/// Domain service: Import management — adds and removes import statements.
pub struct ImportManager;

impl ImportManager {
    /// Update imports in source text: remove old cloud SDK imports, add GCP imports.
    pub fn update_imports(
        source: &str,
        language: Language,
        imports_to_add: &[String],
        imports_to_remove: &[String],
    ) -> String {
        let mut lines: Vec<String> = source.lines().map(String::from).collect();

        // Remove matching import lines
        lines.retain(|line| {
            !imports_to_remove.iter().any(|imp| {
                let trimmed = line.trim();
                match language {
                    Language::Python => trimmed == imp || trimmed.starts_with(&format!("{} ", imp)),
                    Language::TypeScript | Language::JavaScript => {
                        trimmed.contains(imp)
                    }
                    Language::Java => trimmed == format!("{};", imp) || trimmed == imp,
                    Language::Go => trimmed.contains(imp),
                    _ => trimmed == imp,
                }
            })
        });

        // Find the last import line to insert new imports after
        let insert_pos = Self::find_import_insertion_point(&lines, language);

        for (i, imp) in imports_to_add.iter().enumerate() {
            let import_line = match language {
                Language::Python => imp.clone(),
                Language::TypeScript | Language::JavaScript => {
                    imp.clone()
                }
                Language::Java => format!("{};", imp),
                Language::Go => imp.clone(),
                _ => imp.clone(),
            };
            // Only add if not already present
            if !lines.iter().any(|l| l.trim() == import_line.trim()) {
                lines.insert(insert_pos + i, import_line);
            }
        }

        lines.join("\n")
    }

    fn find_import_insertion_point(lines: &[String], language: Language) -> usize {
        let import_keyword = match language {
            Language::Python => "import ",
            Language::TypeScript | Language::JavaScript => "import ",
            Language::Java => "import ",
            Language::Go => "import ",
            _ => return 0,
        };

        let mut last_import = 0;
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with(import_keyword) || trimmed.starts_with("from ") {
                last_import = i + 1;
            }
        }
        last_import
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confidence_calculator_perfect_match() {
        let pattern = CompiledPattern {
            id: PatternId::new("test"),
            description: String::new(),
            source: SourceCloud::Aws,
            language: Language::Python,
            confidence: Confidence::new(0.97),
            tags: vec!["storage".into()],
            detect_query: String::new(),
            detect_imports: vec![],
            transform_template: String::new(),
            import_add: vec![],
            import_remove: vec![],
            bindings: vec![],
        };

        let c = ConfidenceCalculator::calculate(&pattern, 1.0, true);
        assert!(c.is_high());
    }

    #[test]
    fn transform_applicator_single_match() {
        let source = "old_call(arg1, arg2)";
        let m = PatternMatch {
            pattern_id: PatternId::new("test"),
            span: SourceSpan {
                start_byte: 0,
                end_byte: 20,
                start_row: 0,
                start_col: 0,
                end_row: 0,
                end_col: 20,
            },
            confidence: Confidence::new(0.95),
            source_text: "old_call(arg1, arg2)".into(),
            replacement_text: "new_call(arg1, arg2)".into(),
            import_add: vec![],
            import_remove: vec![],
        };

        let result = TransformApplicator::apply_match(source, &m).unwrap();
        assert_eq!(result, "new_call(arg1, arg2)");
    }

    #[test]
    fn import_manager_python() {
        let source = "import boto3\nimport os\n\nclient = boto3.client('s3')";
        let result = ImportManager::update_imports(
            source,
            Language::Python,
            &["from google.cloud import storage".into()],
            &["import boto3".into()],
        );
        assert!(result.contains("from google.cloud import storage"));
        assert!(!result.contains("import boto3"));
        assert!(result.contains("import os"));
    }
}
