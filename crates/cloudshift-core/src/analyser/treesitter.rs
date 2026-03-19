//! Shared tree-sitter utilities: language-to-parser mapping, parser creation,
//! query helpers, and AST node text extraction.
//!
//! Architectural Intent:
//! This module provides the tree-sitter bridge infrastructure used by all
//! language-specific analysers. It maps domain `Language` values to tree-sitter
//! grammar objects and provides helper functions for parsing and querying.

use crate::domain::ports::AnalysisError;
use crate::domain::value_objects::{Language, SourceSpan};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Language as TsLanguage, Node, Parser, Query, QueryCursor, Tree};

/// Convert a domain `Language` to a tree-sitter `Language` grammar.
///
/// TypeScript uses the TSX grammar so both .ts and .tsx (including JSX) parse correctly.
/// Note: `Dockerfile` is handled via string-based analysis because
/// `tree-sitter-dockerfile` depends on an incompatible tree-sitter version (0.20).
pub fn get_language(lang: Language) -> Result<TsLanguage, AnalysisError> {
    match lang {
        Language::Python => Ok(tree_sitter_python::LANGUAGE.into()),
        Language::TypeScript => Ok(tree_sitter_typescript::LANGUAGE_TSX.into()),
        Language::JavaScript => Ok(tree_sitter_javascript::LANGUAGE.into()),
        Language::Java => Ok(tree_sitter_java::LANGUAGE.into()),
        Language::Go => Ok(tree_sitter_go::LANGUAGE.into()),
        Language::Hcl => Ok(tree_sitter_hcl::LANGUAGE.into()),
        Language::Yaml => Ok(tree_sitter_yaml::LANGUAGE.into()),
        Language::Json => Ok(tree_sitter_json::LANGUAGE.into()),
        Language::Dockerfile => Err(AnalysisError::UnsupportedLanguage(lang)),
    }
}

/// Create a tree-sitter `Parser` configured for the given language.
pub fn create_parser(lang: Language) -> Result<Parser, AnalysisError> {
    let ts_lang = get_language(lang)?;
    let mut parser = Parser::new();
    parser
        .set_language(&ts_lang)
        .map_err(|e| AnalysisError::Internal(format!("Failed to set language {lang}: {e}")))?;
    Ok(parser)
}

/// Parse source code into a tree-sitter `Tree`.
pub fn parse_source(source: &[u8], lang: Language) -> Result<Tree, AnalysisError> {
    let mut parser = create_parser(lang)?;
    parser
        .parse(source, None)
        .ok_or_else(|| AnalysisError::ParseError {
            language: lang,
            message: "tree-sitter returned None (possible timeout or cancellation)".into(),
        })
}

/// Compile a tree-sitter query string for the given language.
pub fn compile_query(lang: Language, query_source: &str) -> Result<Query, AnalysisError> {
    let ts_lang = get_language(lang)?;
    Query::new(&ts_lang, query_source).map_err(|e| AnalysisError::ParseError {
        language: lang,
        message: format!("Query compilation failed: {e}"),
    })
}

/// Extract the text content of a tree-sitter node from the source bytes.
pub fn node_text<'a>(node: Node<'_>, source: &'a [u8]) -> &'a str {
    let start = node.start_byte();
    let end = node.end_byte();
    std::str::from_utf8(&source[start..end]).unwrap_or("")
}

/// Convert a tree-sitter `Node` position into a domain `SourceSpan`.
pub fn node_span(node: Node<'_>) -> SourceSpan {
    let start = node.start_position();
    let end = node.end_position();
    SourceSpan {
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_row: start.row,
        start_col: start.column,
        end_row: end.row,
        end_col: end.column,
    }
}

/// An owned representation of a single captured node from a query match.
#[derive(Debug, Clone)]
pub struct OwnedCapture {
    pub index: u32,
    pub name: String,
    pub text: String,
    pub span: SourceSpan,
}

/// An owned representation of a query match, collected from the streaming iterator.
#[derive(Debug, Clone)]
pub struct OwnedMatch {
    pub pattern_index: usize,
    pub captures: Vec<OwnedCapture>,
}

/// Run a tree-sitter query against a parsed tree and collect all matches
/// into owned data structures.
///
/// This function handles the `StreamingIterator` API used by tree-sitter 0.24's
/// `QueryMatches` type, converting each match into an owned `OwnedMatch` so that
/// downstream code can iterate freely.
pub fn run_query(query: &Query, tree: &Tree, source: &[u8]) -> Vec<OwnedMatch> {
    let root = tree.root_node();
    let mut cursor = QueryCursor::new();
    let mut stream = cursor.matches(query, root, source);
    let mut results = Vec::new();
    let capture_names = query.capture_names();

    stream.advance();
    while let Some(m) = stream.get() {
        let captures: Vec<OwnedCapture> = m
            .captures
            .iter()
            .map(|capture| {
                let name = capture_names[capture.index as usize].to_string();
                let text = node_text(capture.node, source).to_string();
                let span = node_span(capture.node);
                OwnedCapture {
                    index: capture.index,
                    name,
                    text,
                    span,
                }
            })
            .collect();

        results.push(OwnedMatch {
            pattern_index: m.pattern_index,
            captures,
        });

        stream.advance();
    }

    results
}

/// A simplified query match: `(pattern_index, [(capture_name, text, span)])`.
pub type SimpleMatch = (usize, Vec<(String, String, SourceSpan)>);

/// Run a query and return matches, providing start/end byte info for the
/// full matched region from each match's first capture.
pub fn run_query_simple(query: &Query, tree: &Tree, source: &[u8]) -> Vec<SimpleMatch> {
    run_query(query, tree, source)
        .into_iter()
        .map(|m| {
            let captures = m
                .captures
                .into_iter()
                .map(|c| (c.name, c.text, c.span))
                .collect();
            (m.pattern_index, captures)
        })
        .collect()
}
