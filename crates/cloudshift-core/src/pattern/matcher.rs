//! Pattern matcher.
//!
//! Matches compiled patterns against source code using tree-sitter queries.
//! Captures bindings from matched AST nodes, resolves template variables,
//! and produces `PatternMatch` instances with replacement text.
//!
//! Architectural Intent:
//! Infrastructure adapter implementing `PatternMatcherPort`. Pure matching
//! logic — no business decisions about whether to accept or reject matches.
//! Confidence calculation is delegated to the domain `ConfidenceCalculator`.

use crate::analyser::treesitter;
use crate::domain::entities::{CompiledPattern, PatternMatch};
use crate::domain::ports::AnalysisError;
use crate::domain::services::ConfidenceCalculator;
use crate::domain::value_objects::{Language, SourceCloud, SourceSpan};
use std::collections::HashMap;

/// Match a single compiled pattern against source code.
///
/// Returns all matches found in the source. Each match includes captured
/// bindings resolved against the template to produce replacement text.
#[tracing::instrument(skip(source, pattern), fields(pattern_id = %pattern.id), level = "debug")]
pub fn match_pattern(
    source: &[u8],
    language: Language,
    pattern: &CompiledPattern,
) -> Result<Vec<PatternMatch>, AnalysisError> {
    // Early exit: check if required imports are present (string-based pre-filter)
    if !pattern.detect_imports.is_empty() {
        let source_str = std::str::from_utf8(source).unwrap_or("");
        let has_import = pattern
            .detect_imports
            .iter()
            .any(|imp| source_str.contains(imp));
        if !has_import {
            return Ok(Vec::new());
        }
    }

    // Parse the source into an AST
    let tree = treesitter::parse_source(source, language)?;

    // Compile the detection query
    let query = match treesitter::compile_query(language, &pattern.detect_query) {
        Ok(q) => q,
        Err(e) => {
            tracing::warn!(
                pattern_id = %pattern.id,
                "Pattern query failed to compile: {e}"
            );
            return Ok(Vec::new());
        }
    };

    // Run the query and collect owned match data
    let owned_matches = treesitter::run_query(&query, &tree, source);
    let mut results = Vec::new();

    for m in &owned_matches {
        // Build capture map: capture_name -> (text, span)
        let mut captures: HashMap<String, (String, SourceSpan)> = HashMap::new();
        let mut match_span: Option<SourceSpan> = None;
        let mut match_text = String::new();

        for capture in &m.captures {
            captures.insert(capture.name.clone(), (capture.text.clone(), capture.span));

            // Compute the broadest span across all captures
            match &match_span {
                None => {
                    match_span = Some(capture.span);
                    match_text = capture.text.clone();
                }
                Some(existing) => {
                    if capture.span.start_byte < existing.start_byte
                        || capture.span.end_byte > existing.end_byte
                    {
                        let merged = SourceSpan {
                            start_byte: capture.span.start_byte.min(existing.start_byte),
                            end_byte: capture.span.end_byte.max(existing.end_byte),
                            start_row: capture.span.start_row.min(existing.start_row),
                            start_col: if capture.span.start_row < existing.start_row {
                                capture.span.start_col
                            } else {
                                existing.start_col
                            },
                            end_row: capture.span.end_row.max(existing.end_row),
                            end_col: if capture.span.end_row > existing.end_row {
                                capture.span.end_col
                            } else {
                                existing.end_col
                            },
                        };
                        match_text =
                            std::str::from_utf8(&source[merged.start_byte..merged.end_byte])
                                .unwrap_or("")
                                .to_string();
                        match_span = Some(merged);
                    }
                }
            }
        }

        let Some(span) = match_span else {
            continue;
        };

        // Resolve bindings: map pattern binding variables to captured text
        let resolved = resolve_bindings(pattern, &captures, source, language);

        // Apply bindings to the template to produce replacement text
        let replacement = apply_template(&pattern.transform_template, &resolved);

        // Calculate binding completeness for confidence scoring
        let binding_completeness = if pattern.bindings.is_empty() {
            1.0
        } else {
            let resolved_count = pattern
                .bindings
                .iter()
                .filter(|b| resolved.contains_key(&b.variable))
                .count();
            resolved_count as f64 / pattern.bindings.len() as f64
        };

        // Calculate match confidence using the domain service
        let confidence = ConfidenceCalculator::calculate(pattern, binding_completeness, true);

        results.push(PatternMatch {
            pattern_id: pattern.id.clone(),
            span,
            confidence,
            source_text: match_text,
            replacement_text: replacement,
            import_add: pattern.import_add.clone(),
            import_remove: pattern.import_remove.clone(),
        });
    }

    Ok(results)
}

/// Match multiple patterns against source code, returning all matches.
#[tracing::instrument(skip(source, patterns), level = "debug")]
pub fn match_all_patterns(
    source: &[u8],
    language: Language,
    source_cloud: SourceCloud,
    patterns: &[CompiledPattern],
) -> Vec<PatternMatch> {
    patterns
        .iter()
        .filter(|p| {
            p.language == language
                && (p.source == source_cloud
                    || source_cloud == SourceCloud::Any
                    || p.source == SourceCloud::Any)
        })
        .flat_map(|p| match match_pattern(source, language, p) {
            Ok(matches) => matches,
            Err(e) => {
                tracing::warn!(
                    pattern_id = %p.id,
                    "Pattern matching failed: {e}"
                );
                Vec::new()
            }
        })
        .collect()
}

/// Resolve pattern bindings against captured AST node text.
///
/// Uses tree-sitter-based extraction for dot-notation bindings (e.g. `args.Bucket`)
/// to correctly handle complex expressions containing commas, nested calls, and
/// string literals. Falls back to string-based extraction if tree-sitter parsing fails.
fn resolve_bindings(
    pattern: &CompiledPattern,
    captures: &HashMap<String, (String, SourceSpan)>,
    source: &[u8],
    language: Language,
) -> HashMap<String, String> {
    let mut resolved = HashMap::new();

    for binding in &pattern.bindings {
        let capture_path = &binding.capture;

        // If the capture path directly matches a capture name, use it
        if let Some((text, _span)) = captures.get(capture_path) {
            resolved.insert(binding.variable.clone(), text.clone());
            continue;
        }

        // Try dot-notation resolution: "args.Bucket" means look in the "args" capture
        // for a keyword argument named "Bucket"
        if let Some(dot_pos) = capture_path.find('.') {
            let base_capture = &capture_path[..dot_pos];
            let field_name = &capture_path[dot_pos + 1..];

            if let Some((args_text, args_span)) = captures.get(base_capture) {
                if let Some(value) =
                    extract_named_arg_from_node(source, args_text, args_span, field_name, language)
                {
                    resolved.insert(binding.variable.clone(), value);
                    continue;
                }
            }
        }

        // If no resolution was found, insert the capture path as a placeholder
        resolved.insert(
            binding.variable.clone(),
            format!("/* unresolved: {} */", capture_path),
        );
    }

    // Also insert all raw captures as fallback variables
    for (name, (text, _)) in captures {
        resolved.entry(name.clone()).or_insert_with(|| text.clone());
    }

    resolved
}

/// Extract a named argument value from an argument_list using tree-sitter.
/// Falls back to string-based extraction if tree-sitter parsing fails.
fn extract_named_arg_from_node(
    source: &[u8],
    args_text: &str,
    args_span: &SourceSpan,
    field_name: &str,
    language: Language,
) -> Option<String> {
    // Try tree-sitter based extraction first
    if let Ok(tree) = crate::analyser::treesitter::parse_source(source, language) {
        let root = tree.root_node();
        // Find the node at the args span
        if let Some(args_node) =
            root.descendant_for_byte_range(args_span.start_byte, args_span.end_byte)
        {
            // Walk children looking for keyword_argument nodes
            let mut cursor = args_node.walk();
            for child in args_node.children(&mut cursor) {
                // Python: keyword_argument, TypeScript: pair, Java: various
                if child.kind() == "keyword_argument" || child.kind() == "pair" {
                    let mut child_cursor = child.walk();
                    let mut name_node = None;
                    let mut value_node = None;
                    for grandchild in child.children(&mut child_cursor) {
                        if grandchild.kind() == "identifier" && name_node.is_none() {
                            name_node = Some(grandchild);
                        } else if name_node.is_some()
                            && grandchild.kind() != "="
                            && grandchild.kind() != ":"
                        {
                            value_node = Some(grandchild);
                        }
                    }
                    if let (Some(name), Some(value)) = (name_node, value_node) {
                        let name_text = crate::analyser::treesitter::node_text(name, source);
                        if name_text == field_name {
                            return Some(
                                crate::analyser::treesitter::node_text(value, source).to_string(),
                            );
                        }
                    }
                }
            }
        }
    }

    // Fallback to string-based extraction for non-Python languages
    extract_named_arg_string(args_text, field_name)
}

/// Extract a named argument value from an argument list text (string-based fallback).
///
/// This is a naive splitter that breaks on commas. It works for simple cases like
/// `Bucket='my-bucket', Key='file.txt'` but fails on complex expressions containing
/// commas inside strings, nested calls, or f-strings. Used only as a fallback when
/// tree-sitter-based extraction is not available.
fn extract_named_arg_string(args_text: &str, field_name: &str) -> Option<String> {
    let pattern_eq = format!("{field_name}=");
    let pattern_colon = format!("{field_name}:");

    let text = args_text.trim_start_matches('(').trim_end_matches(')');

    for part in text.split(',') {
        let trimmed = part.trim();
        if trimmed.starts_with(&pattern_eq) {
            let value = trimmed[pattern_eq.len()..].trim();
            return Some(value.to_string());
        }
        if trimmed.starts_with(&pattern_colon) {
            let value = trimmed[pattern_colon.len()..].trim();
            return Some(value.to_string());
        }
    }

    None
}

/// Apply resolved bindings to a template string.
fn apply_template(template: &str, bindings: &HashMap<String, String>) -> String {
    let mut result = template.trim().to_string();
    for (key, value) in bindings {
        result = result.replace(&format!("{{{key}}}"), value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_template() {
        let mut bindings = HashMap::new();
        bindings.insert("bucket".to_string(), "my_bucket".to_string());
        bindings.insert("key".to_string(), "my_key".to_string());

        let template = "{bucket}.blob({key}).upload()";
        let result = apply_template(template, &bindings);
        assert_eq!(result, "my_bucket.blob(my_key).upload()");
    }

    #[test]
    fn test_extract_named_arg_string_simple() {
        let args = "(Bucket='my-bucket', Key='file.txt', Body=data)";
        assert_eq!(
            extract_named_arg_string(args, "Bucket"),
            Some("'my-bucket'".to_string())
        );
        assert_eq!(
            extract_named_arg_string(args, "Key"),
            Some("'file.txt'".to_string())
        );
        assert_eq!(
            extract_named_arg_string(args, "Body"),
            Some("data".to_string())
        );
        assert_eq!(extract_named_arg_string(args, "Missing"), None);
    }

    #[test]
    fn test_extract_named_arg_treesitter_simple() {
        let source = b"s3.put_object(Bucket='my-bucket', Key='file.txt', Body=data)\n";
        let args_text = "(Bucket='my-bucket', Key='file.txt', Body=data)";
        // The argument_list spans from byte 13 (the '(') to byte 60 (the ')')
        let args_start = source
            .windows(args_text.len())
            .position(|w| w == args_text.as_bytes())
            .unwrap();
        let args_span = SourceSpan {
            start_byte: args_start,
            end_byte: args_start + args_text.len(),
            start_row: 0,
            start_col: args_start,
            end_row: 0,
            end_col: args_start + args_text.len(),
        };

        assert_eq!(
            extract_named_arg_from_node(source, args_text, &args_span, "Bucket", Language::Python),
            Some("'my-bucket'".to_string())
        );
        assert_eq!(
            extract_named_arg_from_node(source, args_text, &args_span, "Key", Language::Python),
            Some("'file.txt'".to_string())
        );
        assert_eq!(
            extract_named_arg_from_node(source, args_text, &args_span, "Body", Language::Python),
            Some("data".to_string())
        );
        assert_eq!(
            extract_named_arg_from_node(source, args_text, &args_span, "Missing", Language::Python),
            None
        );
    }

    #[test]
    fn test_extract_named_arg_treesitter_complex_expressions() {
        // Commas inside string arguments
        let source = b"s3.put_object(Bucket=get_name(\"foo, bar\"), Key=f\"{prefix}/file.txt\", Body=json.dumps({\"key\": \"value\"}))\n";
        let source_str = std::str::from_utf8(source).unwrap();
        let args_start = source_str.find('(').unwrap();
        let args_end = source_str.rfind(')').unwrap() + 1;
        let args_text = &source_str[args_start..args_end];
        let args_span = SourceSpan {
            start_byte: args_start,
            end_byte: args_end,
            start_row: 0,
            start_col: args_start,
            end_row: 0,
            end_col: args_end,
        };

        let bucket =
            extract_named_arg_from_node(source, args_text, &args_span, "Bucket", Language::Python);
        assert_eq!(bucket, Some("get_name(\"foo, bar\")".to_string()));

        let key =
            extract_named_arg_from_node(source, args_text, &args_span, "Key", Language::Python);
        assert_eq!(key, Some("f\"{prefix}/file.txt\"".to_string()));

        let body =
            extract_named_arg_from_node(source, args_text, &args_span, "Body", Language::Python);
        assert_eq!(body, Some("json.dumps({\"key\": \"value\"})".to_string()));
    }
}
