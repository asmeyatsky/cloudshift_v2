//! Integration tests for the self-learning pipeline.
//!
//! Tests the full flow: delta extraction → analysis → candidate generation → persistence.

use cloudshift_core::domain::value_objects::Language;
use cloudshift_core::learning::{
    analyze_changes, extract_llm_delta, generate_candidate_pattern, PatternStore,
};
use cloudshift_core::pipeline::learn_from_diff;

fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

// ---------------------------------------------------------------------------
// End-to-end: extract → analyse → generate → persist
// ---------------------------------------------------------------------------

#[test]
fn test_e2e_learning_pipeline_python_s3() {
    let pattern_output = "\
import boto3

s3 = boto3.client('s3')
s3.put_object(Bucket='my-bucket', Key='data.json', Body='{\"key\": \"value\"}')
response = s3.get_object(Bucket='my-bucket', Key='data.json')
data = response['Body'].read()
";

    let llm_output = "\
from google.cloud import storage

client = storage.Client()
bucket = client.bucket('my-bucket')
blob = bucket.blob('data.json')
blob.upload_from_string('{\"key\": \"value\"}')
blob = bucket.blob('data.json')
data = blob.download_as_bytes()
";

    let dir = temp_dir();
    let saved = learn_from_diff(
        pattern_output,
        llm_output,
        Language::Python,
        "test_s3_migration.py",
        dir.path(),
    )
    .expect("learn_from_diff should succeed");

    assert!(saved > 0, "Expected at least one candidate pattern saved");

    // Verify candidates are on disk
    let store = PatternStore::from_root(dir.path());
    let pending = store.list_pending().expect("list_pending should succeed");
    assert_eq!(
        pending.len(),
        saved,
        "Pending count should match saved count"
    );

    // Each candidate should have a valid TOML with pattern metadata
    for candidate in &pending {
        assert!(!candidate.candidate_id.is_empty());
        assert_eq!(candidate.language, "python");
        let content =
            std::fs::read_to_string(&candidate.file_path).expect("Should read candidate file");
        assert!(content.contains("[pattern]"));
        assert!(content.contains("[pattern.detect]"));
        assert!(content.contains("[pattern.transform]"));
        assert!(content.contains("[pattern.metadata]"));
        assert!(content.contains("review_status = \"pending\""));
        assert!(content.contains("origin = \"llm-learning\""));
    }
}

#[test]
fn test_e2e_learning_pipeline_dynamodb() {
    let pattern_output = "\
import boto3

dynamodb = boto3.resource('dynamodb')
table = dynamodb.Table('users')
table.put_item(Item={'user_id': '123', 'name': 'Alice'})
response = table.get_item(Key={'user_id': '123'})
item = response['Item']
";

    let llm_output = "\
from google.cloud import firestore

db = firestore.Client()
doc_ref = db.collection('users').document('123')
doc_ref.set({'user_id': '123', 'name': 'Alice'})
doc = doc_ref.get()
item = doc.to_dict()
";

    let dir = temp_dir();
    let saved = learn_from_diff(
        pattern_output,
        llm_output,
        Language::Python,
        "test_dynamodb_migration.py",
        dir.path(),
    )
    .expect("learn_from_diff should succeed");

    assert!(saved > 0, "Expected at least one candidate pattern saved");
}

#[test]
fn test_identical_code_produces_no_candidates() {
    let code = "\
from google.cloud import storage

client = storage.Client()
bucket = client.bucket('my-bucket')
blob = bucket.blob('data.json')
data = blob.download_as_bytes()
";

    let dir = temp_dir();
    let saved = learn_from_diff(
        code,
        code,
        Language::Python,
        "already_migrated.py",
        dir.path(),
    )
    .expect("learn_from_diff should succeed");

    assert_eq!(saved, 0, "Identical code should produce no candidates");
}

// ---------------------------------------------------------------------------
// Store: promote / reject lifecycle
// ---------------------------------------------------------------------------

#[test]
fn test_promote_candidate() {
    let pattern_output = "import boto3\ns3 = boto3.client('s3')\n";
    let llm_output = "from google.cloud import storage\nclient = storage.Client()\n";

    let dir = temp_dir();
    let saved = learn_from_diff(
        pattern_output,
        llm_output,
        Language::Python,
        "promote_test.py",
        dir.path(),
    )
    .expect("learn_from_diff should succeed");
    assert!(saved > 0);

    let store = PatternStore::from_root(dir.path());
    let pending = store.list_pending().expect("list_pending");
    let candidate_id = &pending[0].candidate_id;

    // Promote
    let target = store.promote(candidate_id).expect("promote should succeed");
    assert!(target.exists(), "Promoted file should exist");
    assert!(
        target.starts_with(dir.path().join("patterns")),
        "Promoted file should be under patterns/"
    );

    // Verify the promoted file has updated review_status
    let content = std::fs::read_to_string(&target).expect("Should read promoted file");
    assert!(content.contains("review_status = \"promoted\""));

    // Pending should now be empty
    let pending_after = store.list_pending().expect("list_pending");
    assert!(
        pending_after.is_empty(),
        "No pending candidates after promote"
    );
}

#[test]
fn test_reject_candidate() {
    let pattern_output = "import boto3\ns3 = boto3.client('s3')\n";
    let llm_output = "from google.cloud import storage\nclient = storage.Client()\n";

    let dir = temp_dir();
    learn_from_diff(
        pattern_output,
        llm_output,
        Language::Python,
        "reject_test.py",
        dir.path(),
    )
    .expect("learn_from_diff should succeed");

    let store = PatternStore::from_root(dir.path());
    let pending = store.list_pending().expect("list_pending");
    assert!(!pending.is_empty());

    let candidate_id = pending[0].candidate_id.clone();
    let file_path = pending[0].file_path.clone();

    // Reject
    store.reject(&candidate_id).expect("reject should succeed");
    assert!(!file_path.exists(), "Rejected file should be deleted");

    // Pending should be empty
    let pending_after = store.list_pending().expect("list_pending");
    assert!(
        pending_after.is_empty(),
        "No pending candidates after reject"
    );
}

// ---------------------------------------------------------------------------
// Store: stats
// ---------------------------------------------------------------------------

#[test]
fn test_stats() {
    let pattern_output = "import boto3\ns3 = boto3.client('s3')\n";
    let llm_output = "from google.cloud import storage\nclient = storage.Client()\n";

    let dir = temp_dir();
    let saved = learn_from_diff(
        pattern_output,
        llm_output,
        Language::Python,
        "stats_test.py",
        dir.path(),
    )
    .expect("learn_from_diff should succeed");

    let store = PatternStore::from_root(dir.path());
    let stats = store.stats();
    assert_eq!(stats.pending, saved);
    assert_eq!(stats.promoted, 0);

    // Promote one and check stats again
    let pending = store.list_pending().expect("list_pending");
    if !pending.is_empty() {
        store
            .promote(&pending[0].candidate_id)
            .expect("promote should succeed");
        let stats = store.stats();
        assert_eq!(stats.pending, saved - 1);
        assert_eq!(stats.promoted, 1);
    }
}

// ---------------------------------------------------------------------------
// Component-level: extractor
// ---------------------------------------------------------------------------

#[test]
fn test_extractor_multiline_delta() {
    let before = "\
line1
line2
import boto3
s3 = boto3.client('s3')
s3.put_object(Bucket='b', Key='k', Body='data')
line6
line7
";

    let after = "\
line1
line2
from google.cloud import storage
client = storage.Client()
bucket = client.bucket('b')
blob = bucket.blob('k')
blob.upload_from_string('data')
line6
line7
";

    let deltas = extract_llm_delta(before, after);
    assert!(!deltas.is_empty(), "Should find at least one delta");

    // The delta should contain the boto3/GCS transformation
    let delta = &deltas[0];
    assert!(
        delta.pattern_output.contains("boto3") || delta.llm_output.contains("storage"),
        "Delta should capture the cloud SDK change"
    );
}

// ---------------------------------------------------------------------------
// Component-level: analyzer classifications
// ---------------------------------------------------------------------------

#[test]
fn test_analyzer_classifies_import_delta() {
    let deltas = extract_llm_delta("import boto3\n", "from google.cloud import storage\n");
    let analyzed = analyze_changes(&deltas, Language::Python);
    assert!(!analyzed.is_empty());
    assert_eq!(
        analyzed[0].change_type,
        cloudshift_core::learning::analyzer::ChangeType::ImportChange
    );
}

#[test]
fn test_analyzer_classifies_client_init_delta() {
    let deltas = extract_llm_delta("s3 = boto3.client('s3')\n", "client = storage.Client()\n");
    let analyzed = analyze_changes(&deltas, Language::Python);
    assert!(!analyzed.is_empty());
    assert_eq!(
        analyzed[0].change_type,
        cloudshift_core::learning::analyzer::ChangeType::ClientInit
    );
}

// ---------------------------------------------------------------------------
// Component-level: generator TOML output
// ---------------------------------------------------------------------------

#[test]
fn test_generated_toml_is_parseable() {
    let deltas = extract_llm_delta(
        "s3.put_object(Bucket='b', Key='k', Body='d')\n",
        "blob.upload_from_string('d')\n",
    );
    let analyzed = analyze_changes(&deltas, Language::Python);
    assert!(!analyzed.is_empty());

    let candidate = generate_candidate_pattern(&analyzed[0], Language::Python, "test.py");

    // The TOML should be parseable (at least as a raw TOML table)
    let parsed: Result<toml::Value, _> = toml::from_str(&candidate.toml_content);
    assert!(
        parsed.is_ok(),
        "Generated TOML should parse: {:?}",
        parsed.err()
    );

    let table = parsed.unwrap();
    assert!(
        table.get("pattern").is_some(),
        "Should have [pattern] section"
    );
    assert!(
        table["pattern"].get("metadata").is_some(),
        "Should have [pattern.metadata]"
    );
}

// ---------------------------------------------------------------------------
// Promote nonexistent candidate fails gracefully
// ---------------------------------------------------------------------------

#[test]
fn test_promote_nonexistent_fails() {
    let dir = temp_dir();
    let store = PatternStore::from_root(dir.path());
    let result = store.promote("nonexistent-id");
    assert!(result.is_err());
}

#[test]
fn test_reject_nonexistent_fails() {
    let dir = temp_dir();
    let store = PatternStore::from_root(dir.path());
    let result = store.reject("nonexistent-id");
    assert!(result.is_err());
}
