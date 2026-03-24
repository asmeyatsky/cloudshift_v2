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
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e))
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
fn ibte_sqs_send_chain_produces_consolidated_match() {
    use cloudshift_core::domain::value_objects::SourceCloud;
    use cloudshift_core::ibte::run_ibte_python;

    let src = r#"import boto3
sqs = boto3.client('sqs')
sqs.send_message(QueueUrl=url, MessageBody=body)
"#;
    let matches = run_ibte_python(src.as_bytes(), SourceCloud::Aws).expect("ibte");
    assert!(
        !matches.is_empty(),
        "IBTE should produce SQS send_message chain match"
    );
    let m = &matches[0];
    assert!(m.pattern_id.as_str().contains("sqs") && m.pattern_id.as_str().contains("pubsub"));
    assert!(m.replacement_text.contains("pubsub_v1"));
}

#[test]
fn ibte_s3_put_chain_produces_consolidated_match() {
    use cloudshift_core::domain::value_objects::SourceCloud;
    use cloudshift_core::ibte::run_ibte_python;

    let src = r#"import boto3
s3 = boto3.client('s3')
s3.put_object(Bucket='mybucket', Key='path/to/key', Body=data)
"#;
    let matches = run_ibte_python(src.as_bytes(), SourceCloud::Aws).expect("ibte");
    assert!(
        !matches.is_empty(),
        "IBTE should produce S3 put_object chain match"
    );
    let m = &matches[0];
    assert!(
        m.pattern_id.as_str().contains("s3") && m.pattern_id.as_str().contains("upload"),
        "pattern_id should reference S3 and upload, got: {}",
        m.pattern_id
    );
    assert!(m.replacement_text.contains("storage.Client"));
    assert!(m.replacement_text.contains("upload_from_string"));
}

#[test]
fn ibte_dynamodb_chain_produces_consolidated_match() {
    use cloudshift_core::domain::value_objects::SourceCloud;
    use cloudshift_core::ibte::run_ibte_python;

    for src in [
        r#"import boto3
dyndb = boto3.resource('dynamodb')
table = dyndb.Table('Orders')
table.put_item(Item={'id': 'A1', 'status': 'shipped'})
"#,
        r#"import boto3
dyndb = boto3.resource("dynamodb")
table = dyndb.Table("Orders")
table.put_item(Item={"id": "A1", "status": "shipped"})
"#,
        // No trailing newline (like echo -n)
        "import boto3\ndyndb = boto3.resource(\"dynamodb\")\ntable = dyndb.Table(\"Orders\")\ntable.put_item(Item={\"id\": \"A1\", \"status\": \"shipped\"})",
    ] {
        let matches = run_ibte_python(src.as_bytes(), SourceCloud::Aws).expect("ibte");
        assert!(
            !matches.is_empty(),
            "IBTE should produce consolidated match for DynamoDB chain (single and double quotes)"
        );
        let m = &matches[0];
        assert!(
            m.pattern_id.as_str().contains("ibte") && m.pattern_id.as_str().contains("firestore"),
            "expected IBTE Firestore pattern, got {}",
            m.pattern_id
        );
        assert!(
            m.replacement_text.contains("firestore.Client()") && m.replacement_text.contains("collection("),
            "replacement should contain consolidated Firestore code"
        );
    }
}

#[test]
fn azure_functions_handler_fixup_rewrites_to_cloud_functions() {
    use cloudshift_core::domain::value_objects::Language;
    use cloudshift_core::fixup::apply_fixups;

    // Azure Functions handler is now converted via fixup, not pattern.
    let src = "import azure.functions as func\n\napp = func.FunctionApp()\n\n@app.function_name(\"Test\")\ndef main(req: func.HttpRequest):\n    return 1\n";
    let result = apply_fixups(src, Language::Python);
    assert!(
        result.contains("functions_framework"),
        "fixup should add functions_framework import, got:\n{result}"
    );
    assert!(
        result.contains("def main(request):"),
        "fixup should rewrite param to (request), got:\n{result}"
    );
    assert!(
        !result.contains("azure.functions"),
        "fixup should remove azure.functions import, got:\n{result}"
    );
}

#[test]
#[ignore = "manual AST dump"]
fn debug_python_function_parameters_structure() {
    let src = "import azure.functions as func\ndef main(req):\n    return 1\n";
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(src.as_bytes(), None).unwrap();
    let mut cursor = tree.walk();
    fn walk(cursor: &mut tree_sitter::TreeCursor, src: &str, depth: usize) {
        let n = cursor.node();
        let slice = &src[n.start_byte()..n.end_byte().min(src.len())];
        let preview: String = slice.chars().take(50).collect();
        println!(
            "{:indent$}{} [{}..{}] {:?}",
            "",
            n.kind(),
            n.start_byte(),
            n.end_byte(),
            preview,
            indent = depth * 2
        );
        if !cursor.goto_first_child() {
            return;
        }
        loop {
            walk(cursor, src, depth + 1);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
        cursor.goto_parent();
    }
    walk(&mut cursor, src, 0);
}

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

    let tree = parser
        .parse(&source, None)
        .expect("Failed to parse Python source");
    let root_node = tree.root_node();

    // Should parse without errors
    assert!(!root_node.has_error(), "Parse tree contains errors");

    // Root should be a module
    assert_eq!(root_node.kind(), "module");

    // Should contain function definitions
    let child_count = root_node.named_child_count();
    assert!(
        child_count >= 4,
        "Expected at least 4 top-level nodes (import, assignment, 3 functions)"
    );
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

    let tree = parser
        .parse(&source, None)
        .expect("Failed to parse TypeScript source");
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
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "toml"))
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

    let confidence = pattern
        .get("confidence")
        .and_then(|v| v.as_float())
        .unwrap();
    assert!(confidence > 0.90);

    // Verify detect query exists and is non-empty
    let detect = pattern.get("detect").expect("Missing [pattern.detect]");
    let query = detect.get("query").and_then(|v| v.as_str()).unwrap();
    assert!(!query.is_empty());
    assert!(query.contains("put_object"));

    // Verify transform template exists and contains the GCP replacement expression
    let transform = pattern
        .get("transform")
        .expect("Missing [pattern.transform]");
    let template = transform.get("template").and_then(|v| v.as_str()).unwrap();
    assert!(template.contains("storage.Client()"));

    // Verify imports are handled separately from the template
    let import_add = transform
        .get("import_add")
        .and_then(|v| v.as_array())
        .unwrap();
    assert!(import_add
        .iter()
        .any(|v| v.as_str().unwrap().contains("google.cloud")));
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
        ("tests/patterns/python/aws_s3_to_gcs", true), // 0.94 — high
        ("tests/patterns/python/aws_dynamodb_to_firestore", false), // 0.88 — medium
        ("tests/patterns/hcl/aws_iam_to_google_iam", false), // 0.78 — medium
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
        fixture_count >= 19,
        "Expected at least 19 test fixtures, found {fixture_count}"
    );
}

// ---------------------------------------------------------------------------
// Full pipeline integration tests
// ---------------------------------------------------------------------------

#[test]
fn test_full_pipeline_with_catalogue() {
    // Load the actual pattern catalogue from patterns/
    let catalogue_path = workspace_root().join("patterns");

    let catalogue = cloudshift_core::catalogue::Catalogue::from_directory(&catalogue_path).unwrap();

    // Verify we loaded a substantial number of patterns (62 expected)
    assert!(
        catalogue.all_patterns().len() >= 50,
        "Expected at least 50 patterns, got {}",
        catalogue.all_patterns().len()
    );

    // Verify patterns can be queried by language
    use cloudshift_core::domain::ports::PatternRepositoryPort;
    let python_aws = catalogue.get_patterns(
        cloudshift_core::Language::Python,
        cloudshift_core::SourceCloud::Aws,
    );
    assert!(
        python_aws.len() >= 15,
        "Expected at least 15 Python AWS patterns, got {}",
        python_aws.len()
    );

    let hcl_patterns = catalogue.get_patterns(
        cloudshift_core::Language::Hcl,
        cloudshift_core::SourceCloud::Aws,
    );
    assert!(
        hcl_patterns.len() >= 10,
        "Expected at least 10 HCL AWS patterns, got {}",
        hcl_patterns.len()
    );
}

#[test]
fn test_transform_file_with_real_patterns() {
    // Use the actual pattern catalogue and a test fixture.
    // transform_file validates paths against current_dir(), so we need
    // to set the working directory to the workspace root.
    let root = workspace_root();
    std::env::set_current_dir(&root).expect("Failed to set working directory");

    let catalogue_path = root.join("patterns");
    let fixture_path = root.join("tests/patterns/python/aws_s3_to_gcs/before.py");

    let config = cloudshift_core::TransformConfig {
        source_cloud: cloudshift_core::SourceCloud::Aws,
        catalogue_path: Some(catalogue_path.to_string_lossy().to_string()),
        threshold: 0.0,
        ..Default::default()
    };

    let result = cloudshift_core::transform_file(&fixture_path.to_string_lossy(), &config);

    // The transform should succeed
    assert!(result.is_ok(), "transform_file failed: {:?}", result.err());
    let result = result.unwrap();
    assert_eq!(result.language, cloudshift_core::Language::Python);
}

#[test]
fn test_transform_repo_with_real_patterns() {
    // Point at the test fixtures directory and run a repo transform.
    // transform_repo uses validate_path internally; set cwd to workspace root.
    let root = workspace_root();
    std::env::set_current_dir(&root).expect("Failed to set working directory");

    let fixtures_dir = root.join("tests/patterns/python/aws_s3_to_gcs");
    let catalogue_path = root.join("patterns");

    let config = cloudshift_core::TransformConfig {
        source_cloud: cloudshift_core::SourceCloud::Aws,
        catalogue_path: Some(catalogue_path.to_string_lossy().to_string()),
        ..Default::default()
    };

    let result = cloudshift_core::transform_repo(&fixtures_dir.to_string_lossy(), &config);

    assert!(result.is_ok(), "transform_repo failed: {:?}", result.err());
    let report = result.unwrap();
    // Should have emitted at least one domain event (the RepoScanCompleted event)
    assert!(
        !report.domain_events.is_empty(),
        "Expected at least 1 domain event"
    );
}

#[test]
fn test_all_pattern_toml_files_compile() {
    // Verify EVERY pattern TOML file in the catalogue can be compiled
    let catalogue_path = workspace_root().join("patterns");

    let catalogue = cloudshift_core::catalogue::Catalogue::from_directory(&catalogue_path).unwrap();
    let warnings = catalogue.warnings();

    // No pattern file should fail to compile (file non-empty = compile error; empty file = e.g. duplicate ID)
    let compile_warnings: Vec<_> = warnings.iter().filter(|w| !w.file.is_empty()).collect();
    if !compile_warnings.is_empty() {
        let msgs: Vec<String> = compile_warnings
            .iter()
            .map(|w| format!("{}: {}", w.file, w.message))
            .collect();
        panic!("Pattern compilation warnings:\n{}", msgs.join("\n"));
    }
}

#[test]
fn test_pattern_matching_on_boto3_code() {
    // Test that the pattern matcher can find boto3 patterns in real Python code
    let source = b"import boto3\n\ns3 = boto3.client('s3')\ns3.put_object(Bucket='mybucket', Key='mykey', Body=b'data')\n";

    let catalogue_path = workspace_root().join("patterns");
    let catalogue = cloudshift_core::catalogue::Catalogue::from_directory(&catalogue_path).unwrap();

    use cloudshift_core::domain::ports::PatternRepositoryPort;
    let python_patterns = catalogue.get_patterns(
        cloudshift_core::Language::Python,
        cloudshift_core::SourceCloud::Aws,
    );

    // Test the analyser finds constructs
    use cloudshift_core::domain::ports::SemanticAnalyserPort;
    let analyser = cloudshift_core::analyser::SemanticAnalyser::new();
    let constructs = analyser
        .analyse(source, cloudshift_core::Language::Python)
        .unwrap();
    assert!(
        !constructs.is_empty(),
        "Analyser should find boto3 constructs"
    );

    // Test the pattern engine can match
    use cloudshift_core::domain::ports::PatternMatcherPort;
    let matcher = cloudshift_core::pattern::PatternEngine::new();
    let matches = matcher.match_patterns(
        source,
        cloudshift_core::Language::Python,
        cloudshift_core::SourceCloud::Aws,
        &python_patterns,
    );

    // Log results for debugging
    println!(
        "Found {} constructs and {} pattern matches",
        constructs.len(),
        matches.len()
    );
}

// ---------------------------------------------------------------------------
// Binding resolution tests (tree-sitter-based arg extraction)
// ---------------------------------------------------------------------------

#[test]
fn test_binding_resolution_with_complex_args() {
    // Source with complex expressions: commas inside strings, nested calls, f-strings
    let source = br#"import boto3
s3 = boto3.client('s3')
s3.put_object(Bucket=get_bucket_name("prod"), Key=f"{prefix}/file.txt", Body=json.dumps(data))
"#;

    let catalogue_path = workspace_root().join("patterns");
    let catalogue = cloudshift_core::catalogue::Catalogue::from_directory(&catalogue_path).unwrap();

    use cloudshift_core::domain::ports::PatternRepositoryPort;
    let python_patterns = catalogue.get_patterns(
        cloudshift_core::Language::Python,
        cloudshift_core::SourceCloud::Aws,
    );

    use cloudshift_core::domain::ports::PatternMatcherPort;
    let matcher = cloudshift_core::pattern::PatternEngine::new();
    let matches = matcher.match_patterns(
        source,
        cloudshift_core::Language::Python,
        cloudshift_core::SourceCloud::Aws,
        &python_patterns,
    );

    // Find the put_object match
    let put_match = matches
        .iter()
        .find(|m| m.pattern_id.as_str().contains("put_object"));
    assert!(
        put_match.is_some(),
        "Expected to find a put_object pattern match"
    );
    let put_match = put_match.unwrap();

    // The replacement should contain the full complex bucket expression,
    // not a truncated version from naive comma splitting
    assert!(
        put_match
            .replacement_text
            .contains("get_bucket_name(\"prod\")"),
        "Bucket binding should contain the full call expression, got: {}",
        put_match.replacement_text
    );

    // The key binding should contain the f-string
    assert!(
        put_match
            .replacement_text
            .contains("f\"{prefix}/file.txt\""),
        "Key binding should contain the full f-string, got: {}",
        put_match.replacement_text
    );

    // The body binding should contain the full json.dumps call
    assert!(
        put_match.replacement_text.contains("json.dumps(data)"),
        "Body binding should contain the full call expression, got: {}",
        put_match.replacement_text
    );
}

#[test]
fn test_binding_resolution_with_comma_in_string() {
    // Regression test: commas inside string arguments must not split incorrectly
    let source = br#"import boto3
s3 = boto3.client('s3')
s3.put_object(Bucket="my-bucket", Key="path/to/file, with comma.txt", Body=b"data")
"#;

    let catalogue_path = workspace_root().join("patterns");
    let catalogue = cloudshift_core::catalogue::Catalogue::from_directory(&catalogue_path).unwrap();

    use cloudshift_core::domain::ports::PatternRepositoryPort;
    let python_patterns = catalogue.get_patterns(
        cloudshift_core::Language::Python,
        cloudshift_core::SourceCloud::Aws,
    );

    use cloudshift_core::domain::ports::PatternMatcherPort;
    let matcher = cloudshift_core::pattern::PatternEngine::new();
    let matches = matcher.match_patterns(
        source,
        cloudshift_core::Language::Python,
        cloudshift_core::SourceCloud::Aws,
        &python_patterns,
    );

    let put_match = matches
        .iter()
        .find(|m| m.pattern_id.as_str().contains("put_object"));
    assert!(
        put_match.is_some(),
        "Expected to find a put_object pattern match"
    );
    let put_match = put_match.unwrap();

    // Key should contain the full string including the comma
    assert!(
        put_match
            .replacement_text
            .contains("\"path/to/file, with comma.txt\""),
        "Key binding should preserve string with comma, got: {}",
        put_match.replacement_text
    );
}

// ---------------------------------------------------------------------------
// Regression corpus: full pipeline on fixtures, assert expected output
// ---------------------------------------------------------------------------

#[test]
fn regression_transform_s3_fixture_produces_gcs() {
    let root = workspace_root();
    std::env::set_current_dir(&root).expect("set cwd");
    let catalogue_path = root.join("patterns");
    let fixture_path = root.join("tests/patterns/python/aws_s3_to_gcs/before.py");
    let config = cloudshift_core::TransformConfig {
        source_cloud: cloudshift_core::SourceCloud::Aws,
        catalogue_path: Some(catalogue_path.to_string_lossy().to_string()),
        threshold: 0.0,
        ..Default::default()
    };
    let result = cloudshift_core::transform_file(&fixture_path.to_string_lossy(), &config)
        .expect("transform");
    assert!(
        result.pattern_count() > 0,
        "expected at least one pattern match"
    );
    let out = &result.transformed_source;
    assert!(
        out.contains("storage") || out.contains("google.cloud"),
        "output should contain GCP storage reference"
    );
    assert!(
        out.contains("upload_from_string")
            || out.contains("download_as_bytes")
            || out.contains("list_blobs"),
        "output should contain GCS API usage"
    );
}

// ---------------------------------------------------------------------------
// Before/after fixture consistency tests
// ---------------------------------------------------------------------------

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
