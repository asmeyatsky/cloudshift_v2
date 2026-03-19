//! HTTP API integration tests (env is shared — run with `--test-threads=1` or serial).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serial_test::serial;
use std::path::Path;
use tower::ServiceExt;

fn patterns_dir() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../patterns")
        .canonicalize()
        .expect("patterns dir")
        .to_string_lossy()
        .into_owned()
}

#[tokio::test]
#[serial]
async fn auth_check_without_credentials_is_401() {
    std::env::set_var("CLOUDSHIFT_API_KEY", "only-api-key-auth");
    std::env::remove_var("CLOUDSHIFT_IAP_AUDIENCE");
    let st = cloudshift_server::build_state().unwrap();
    let app = cloudshift_server::app(st, "/nonexistent-static-xyz");

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/auth-check")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn auth_check_with_valid_api_key_is_200() {
    std::env::set_var("CLOUDSHIFT_API_KEY", "secret-api-key-123");
    std::env::remove_var("CLOUDSHIFT_IAP_AUDIENCE");
    let st = cloudshift_server::build_state().unwrap();
    let app = cloudshift_server::app(st, "/nonexistent-static-xyz");

    let res = app
        .oneshot(
            Request::builder()
                .uri("/api/auth-check")
                .header("x-api-key", "secret-api-key-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = res.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&body).contains("true"));
}

#[tokio::test]
#[serial]
async fn transform_with_valid_key_returns_200_and_json() {
    std::env::set_var("CLOUDSHIFT_API_KEY", "valid-key");
    std::env::remove_var("CLOUDSHIFT_IAP_AUDIENCE");
    std::env::set_var("CLOUDSHIFT_PATTERNS_DIR", patterns_dir());
    let st = cloudshift_server::build_state().unwrap();
    let app = cloudshift_server::app(st, "/nonexistent-static-xyz");

    let body = r#"{"source":"x = 1","language":"python"}"#;
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/transform")
                .header("content-type", "application/json")
                .header("x-api-key", "valid-key")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(json.get("path").is_some() || json.get("transformed_source").is_some());
}

#[tokio::test]
#[serial]
async fn transform_rejects_wrong_api_key() {
    std::env::set_var("CLOUDSHIFT_API_KEY", "good-key");
    std::env::remove_var("CLOUDSHIFT_IAP_AUDIENCE");
    std::env::set_var("CLOUDSHIFT_PATTERNS_DIR", patterns_dir());
    let st = cloudshift_server::build_state().unwrap();
    let app = cloudshift_server::app(st, "/nonexistent-static-xyz");

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/transform")
                .header("content-type", "application/json")
                .header("x-api-key", "wrong")
                .body(Body::from(r#"{"source":"a=1","language":"python"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn transform_payload_too_large() {
    std::env::set_var("CLOUDSHIFT_API_KEY", "k");
    std::env::remove_var("CLOUDSHIFT_IAP_AUDIENCE");
    std::env::set_var("CLOUDSHIFT_PATTERNS_DIR", patterns_dir());
    let st = cloudshift_server::build_state().unwrap();
    let app = cloudshift_server::app(st, "/nonexistent-static-xyz");

    let huge = "x".repeat(1_100_000);
    let body = format!(
        "{{\"source\":{},\"language\":\"python\"}}",
        serde_json::to_string(&huge).unwrap()
    );
    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/transform")
                .header("content-type", "application/json")
                .header("x-api-key", "k")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
#[serial]
async fn health_unauthenticated() {
    std::env::set_var("CLOUDSHIFT_API_KEY", "k");
    std::env::remove_var("CLOUDSHIFT_IAP_AUDIENCE");
    let st = cloudshift_server::build_state().unwrap();
    let app = cloudshift_server::app(st, "/nonexistent-static-xyz");

    let res = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
#[serial]
async fn github_repo_requires_auth() {
    std::env::set_var("CLOUDSHIFT_API_KEY", "secret-key");
    std::env::remove_var("CLOUDSHIFT_IAP_AUDIENCE");
    let st = cloudshift_server::build_state().unwrap();
    let app = cloudshift_server::app(st, "/nonexistent-static-xyz");

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/github/repo")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"url":"https://github.com/foo/bar"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn github_repo_rejects_blank_url() {
    std::env::set_var("CLOUDSHIFT_API_KEY", "k");
    std::env::remove_var("CLOUDSHIFT_IAP_AUDIENCE");
    let st = cloudshift_server::build_state().unwrap();
    let app = cloudshift_server::app(st, "/nonexistent-static-xyz");

    let res = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/github/repo")
                .header("content-type", "application/json")
                .header("x-api-key", "k")
                .body(Body::from(r#"{"url":"   "}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}
