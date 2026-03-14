//! Integration tests for CloudShift Core.
//!
//! These tests verify the full pipeline from fixture loading through
//! pattern matching, transformation, and diff emission.

use std::fs;
use std::path::{Path, PathBuf};

use cloudshift_core::domain::value_objects::{Confidence, PatternId};

/// Metadata from a test fixture's meta.toml.
#[derive(Debug, serde::Deserialize)]
struct FixtureMeta {
    pattern_ids: Vec<String>,
    expected_confidence: f64,
    #[serde(default)]
    expected_warnings: Vec<String>,
}

/// Resolve the workspace root (two levels up from cloudshift-core).
fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("Could not resolve workspace root")
        .to_path_buf()
}

/// Load a fixture's meta.toml and parse it.
fn load_fixture_meta(fixture_dir: &Path) -> FixtureMeta {
    let meta_path = fixture_dir.join("meta.toml");
    let content = fs::read_to_string(&meta_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", meta_path.display(), e));
    toml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", meta_path.display(), e))
}

/// Load a fixture file's content as a string.
fn load_fixture_file(fixture_dir: &Path, filename: &str) -> String {
    let path = fixture_dir.join(filename);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e))
}

// ---------------------------------------------------------------------------
// Fixture loading tests
// ---------------------------------------------------------------------------

#[test]
fn test_load_python_s3_fixture_meta() {
    let root = workspace_root();
    let fixture_dir = root.join("tests/patterns/python/aws_s3_to_gcs");
    let meta = load_fixture_meta(&fixture_dir);

    assert_eq!(meta.pattern_ids.len(), 3);
    assert!(meta.pattern_ids[0].contains("put_object"));
    assert!(meta.pattern_ids[1].contains("get_object"));
    assert!(meta.pattern_ids[2].contains("list_objects_v2"));
    assert!((meta.expected_confidence - 0.94).abs() < 0.001);
    assert!(meta.expected_warnings.is_empty());
}

#[test]
fn test_load_python_s3_before_after() {
    let root = workspace_root();
    let fixture_dir = root.join("tests/patterns/python/aws_s3_to_gcs");

    let before = load_fixture_file(&fixture_dir, "before.py");
    let after = load_fixture_file(&fixture_dir, "after.py");

    // Before should reference boto3/S3
    assert!(before.contains("import boto3"));
    assert!(before.contains("put_object"));
    assert!(before.contains("get_object"));
    assert!(before.contains("list_objects_v2"));

    // After should reference google.cloud.storage
    assert!(after.contains("from google.cloud import storage"));
    assert!(after.contains("upload_from_string"));
    assert!(after.contains("download_as_bytes"));
    assert!(after.contains("list_blobs"));

    // After should NOT contain AWS references
    assert!(!after.contains("boto3"));
    assert!(!after.contains("s3."));
}

// ---------------------------------------------------------------------------
// Tree-sitter parsing tests
// ---------------------------------------------------------------------------

#[test]
fn test_parse_python_fixture_with_tree_sitter() {
    let root = workspace_root();
    let fixture_dir = root.join("tests/patterns/python/aws_s3_to_gcs");
    let source = load_fixture_file(&fixture_dir, "before.py");

    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_python::LANGUAGE;
    parser
        .set_language(&language.into())
        .expect("Failed to set Python language");

    let tree = parser.parse(&source, None).expect("Failed to parse Python source");
    let root_node = tree.root_node();

    // Should parse without errors
    assert!(!root_node.has_error(), "Parse tree contains errors");

    // Root should be a module
    assert_eq!(root_node.kind(), "module");

    // Should contain function definitions
    let child_count = root_node.named_child_count();
    assert!(child_count >= 4, "Expected at least 4 top-level nodes (import, assignment, 3 functions)");
}

#[test]
fn test_parse_typescript_fixture_with_tree_sitter() {
    let root = workspace_root();
    let fixture_dir = root.join("tests/patterns/typescript/aws_s3_to_gcs");
    let source = load_fixture_file(&fixture_dir, "before.ts");

    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_typescript::LANGUAGE_TYPESCRIPT;
    parser
        .set_language(&language.into())
        .expect("Failed to set TypeScript language");

    let tree = parser.parse(&source, None).expect("Failed to parse TypeScript source");
    let root_node = tree.root_node();

    assert!(!root_node.has_error(), "Parse tree contains errors");
    assert_eq!(root_node.kind(), "program");
}

// ---------------------------------------------------------------------------
// Pattern catalogue loading tests
// ---------------------------------------------------------------------------

#[test]
fn test_load_pattern_toml_files() {
    let root = workspace_root();
    let patterns_dir = root.join("patterns/python");

    assert!(patterns_dir.exists(), "Patterns directory not found");

    let toml_files: Vec<_> = fs::read_dir(&patterns_dir)
        .expect("Failed to read patterns directory")
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map_or(false, |ext| ext == "toml")
        })
        .collect();

    assert!(
        !toml_files.is_empty(),
        "No pattern TOML files found in {}",
        patterns_dir.display()
    );

    // Verify each TOML file parses correctly
    for entry in &toml_files {
        let content = fs::read_to_string(entry.path())
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", entry.path().display(), e));
        let value: toml::Value = toml::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", entry.path().display(), e));

        // Each pattern file should have a [pattern] table
        assert!(
            value.get("pattern").is_some(),
            "Pattern file {} missing [pattern] table",
            entry.path().display()
        );
    }
}

#[test]
fn test_parse_s3_put_object_pattern() {
    let root = workspace_root();
    let pattern_path = root.join("patterns/python/aws_s3_put_object.toml");
    let content = fs::read_to_string(&pattern_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", pattern_path.display(), e));

    let value: toml::Value = toml::from_str(&content).expect("Failed to parse pattern TOML");
    let pattern = value.get("pattern").expect("Missing [pattern] table");

    let id = pattern.get("id").and_then(|v| v.as_str()).unwrap();
    assert!(id.contains("s3") && id.contains("put_object"));

    let source = pattern.get("source").and_then(|v| v.as_str()).unwrap();
    assert_eq!(source, "aws");

    let language = pattern.get("language").and_then(|v| v.as_str()).unwrap();
    assert_eq!(language, "python");

    let confidence = pattern.get("confidence").and_then(|v| v.as_float()).unwrap();
    assert!(confidence > 0.90);

    // Verify detect query exists and is non-empty
    let detect = pattern.get("detect").expect("Missing [pattern.detect]");
    let query = detect.get("query").and_then(|v| v.as_str()).unwrap();
    assert!(!query.is_empty());
    assert!(query.contains("put_object"));

    // Verify transform template exists
    let transform = pattern.get("transform").expect("Missing [pattern.transform]");
    let template = transform.get("template").and_then(|v| v.as_str()).unwrap();
    assert!(template.contains("google.cloud"));
}

// ---------------------------------------------------------------------------
// Diff emitter tests (using the `similar` crate)
// ---------------------------------------------------------------------------

#[test]
fn test_diff_generation_between_fixtures() {
    let root = workspace_root();
    let fixture_dir = root.join("tests/patterns/python/aws_s3_to_gcs");

    let before = load_fixture_file(&fixture_dir, "before.py");
    let after = load_fixture_file(&fixture_dir, "after.py");

    // Generate a unified diff using the `similar` crate (same dep as cloudshift-core)
    let diff = similar::TextDiff::from_lines(&before, &after);
    let unified = diff
        .unified_diff()
        .context_radius(3)
        .header("before.py", "after.py")
        .to_string();

    // Diff should not be empty (files are different)
    assert!(!unified.is_empty(), "Diff should not be empty");

    // Diff should show boto3 removal and google.cloud addition
    assert!(unified.contains("-import boto3"));
    assert!(unified.contains("+from google.cloud import storage"));

    // Diff should show function body changes
    assert!(unified.contains("-    s3.put_object("));
    assert!(unified.contains("+    blob.upload_from_string(content)"));
}

// ---------------------------------------------------------------------------
// Domain value object tests (confidence, pattern IDs)
// ---------------------------------------------------------------------------

#[test]
fn test_pattern_id_from_fixture_meta() {
    let root = workspace_root();
    let fixture_dir = root.join("tests/patterns/python/aws_s3_to_gcs");
    let meta = load_fixture_meta(&fixture_dir);

    for id_str in &meta.pattern_ids {
        let pattern_id = PatternId::new(id_str.clone());
        assert!(!pattern_id.as_str().is_empty());
        // All pattern IDs should follow the "source -> target" convention
        assert!(
            pattern_id.as_str().contains("->"),
            "Pattern ID '{}' missing '->' separator",
            pattern_id
        );
    }
}

#[test]
fn test_confidence_from_fixture_meta() {
    let root = workspace_root();

    let test_cases = vec![
        ("tests/patterns/python/aws_s3_to_gcs", true),             // 0.94 — high
        ("tests/patterns/python/aws_dynamodb_to_firestore", false), // 0.88 — medium
        ("tests/patterns/hcl/aws_iam_to_google_iam", false),       // 0.78 — medium
    ];

    for (fixture_rel, expect_high) in test_cases {
        let fixture_dir = root.join(fixture_rel);
        let meta = load_fixture_meta(&fixture_dir);
        let confidence = Confidence::new(meta.expected_confidence);

        if expect_high {
            assert!(
                confidence.is_high(),
                "Expected high confidence for {}, got {}",
                fixture_rel,
                confidence
            );
        } else {
            assert!(
                confidence.is_medium(),
                "Expected medium confidence for {}, got {}",
                fixture_rel,
                confidence
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Cross-language fixture consistency tests
// ---------------------------------------------------------------------------

#[test]
fn test_all_fixtures_have_required_files() {
    let root = workspace_root();
    let test_patterns_dir = root.join("tests/patterns");

    let languages = ["python", "typescript", "hcl"];
    let mut fixture_count = 0;

    for lang in &languages {
        let lang_dir = test_patterns_dir.join(lang);
        if !lang_dir.exists() {
            continue;
        }

        for entry in fs::read_dir(&lang_dir).expect("Failed to read language dir") {
            let entry = entry.expect("Failed to read directory entry");
            if !entry.file_type().unwrap().is_dir() {
                continue;
            }

            let fixture_dir = entry.path();
            let fixture_name = fixture_dir.file_name().unwrap().to_string_lossy();

            // Determine expected extension
            let ext = match *lang {
                "python" => "py",
                "typescript" => "ts",
                "hcl" => "tf",
                _ => continue,
            };

            // Check required files exist
            let before_path = fixture_dir.join(format!("before.{ext}"));
            let after_path = fixture_dir.join(format!("after.{ext}"));
            let meta_path = fixture_dir.join("meta.toml");

            assert!(
                before_path.exists(),
                "Missing before.{ext} in {lang}/{fixture_name}"
            );
            assert!(
                after_path.exists(),
                "Missing after.{ext} in {lang}/{fixture_name}"
            );
            assert!(
                meta_path.exists(),
                "Missing meta.toml in {lang}/{fixture_name}"
            );

            // Verify meta.toml is valid
            let meta = load_fixture_meta(&fixture_dir);
            assert!(
                !meta.pattern_ids.is_empty(),
                "Empty pattern_ids in {lang}/{fixture_name}"
            );
            assert!(
                meta.expected_confidence > 0.0 && meta.expected_confidence <= 1.0,
                "Invalid confidence {} in {lang}/{fixture_name}",
                meta.expected_confidence
            );

            fixture_count += 1;
        }
    }

    assert!(
        fixture_count >= 10,
        "Expected at least 10 test fixtures, found {fixture_count}"
    );
}

#[test]
fn test_before_after_files_are_not_identical() {
    let root = workspace_root();
    let test_patterns_dir = root.join("tests/patterns");

    for lang in &["python", "typescript", "hcl"] {
        let lang_dir = test_patterns_dir.join(lang);
        if !lang_dir.exists() {
            continue;
        }

        let ext = match *lang {
            "python" => "py",
            "typescript" => "ts",
            "hcl" => "tf",
            _ => continue,
        };

        for entry in fs::read_dir(&lang_dir).expect("Failed to read language dir") {
            let entry = entry.expect("Failed to read directory entry");
            if !entry.file_type().unwrap().is_dir() {
                continue;
            }

            let fixture_dir = entry.path();
            let fixture_name = fixture_dir.file_name().unwrap().to_string_lossy();

            let before = load_fixture_file(&fixture_dir, &format!("before.{ext}"));
            let after = load_fixture_file(&fixture_dir, &format!("after.{ext}"));

            assert_ne!(
                before.trim(),
                after.trim(),
                "before and after files are identical in {lang}/{fixture_name}"
            );
        }
    }
}
