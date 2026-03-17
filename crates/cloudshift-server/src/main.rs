//! HTTP server for Cloud Run — health, auth, transformation API, and static frontend.

use axum::{
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use cloudshift_core::{
    pipeline::{transform_source_for_api, TransformConfig},
    Language, SourceCloud,
};
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::Path;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;

const AUTH_REQUIRED_MSG: &str = "IAP / X-Searce-ID / Bearer or valid X-API-Key required";

fn has_valid_auth_from_headers(headers: &HeaderMap) -> bool {
    if headers.get("X-Goog-IAP-JWT-Assertion").is_some() {
        return true;
    }
    if headers
        .get("X-Searce-ID")
        .and_then(|v| v.to_str().ok())
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    if let Some(auth) = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if auth.starts_with("Bearer ") && !auth[7..].trim().is_empty() {
            return true;
        }
    }
    if let Some(api_key) = std::env::var("CLOUDSHIFT_API_KEY").ok() {
        let api_key = api_key.trim();
        if !api_key.is_empty()
            && headers
                .get("X-API-Key")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim() == api_key)
                .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

/// Root fallback when static dir is not present (API-only mode).
async fn root() -> &'static str {
    "ok"
}

async fn health() -> &'static str {
    "ok"
}

async fn ready() -> &'static str {
    "ready"
}

/// Avoid 404 for browser favicon requests.
async fn favicon() -> Response {
    (StatusCode::NO_CONTENT, ()).into_response()
}

async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "Not found").into_response()
}

/// Max request body size for /api/transform (1 MiB).
const MAX_TRANSFORM_BODY: usize = 1024 * 1024;

#[derive(Deserialize)]
struct TransformRequestBody {
    source: String,
    language: Language,
    #[serde(default)]
    source_cloud: Option<SourceCloud>,
    #[serde(default)]
    path_hint: Option<String>,
}

async fn api_transform(headers: HeaderMap, body: Json<TransformRequestBody>) -> Response {
    if !has_valid_auth_from_headers(&headers) {
        return (StatusCode::UNAUTHORIZED, AUTH_REQUIRED_MSG).into_response();
    }
    if body.source.len() > MAX_TRANSFORM_BODY {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            format!("Source exceeds max size ({} bytes)", MAX_TRANSFORM_BODY),
        )
            .into_response();
    }
    let path_hint = body
        .path_hint
        .as_deref()
        .unwrap_or_else(|| match body.language {
            Language::Python => "main.py",
            Language::TypeScript => "main.ts",
            Language::JavaScript => "main.js",
            Language::Java => "Main.java",
            Language::Go => "main.go",
            Language::Hcl => "main.tf",
            Language::Yaml => "main.yaml",
            Language::Dockerfile => "Dockerfile",
            Language::Json => "config.json",
        });
    let config = TransformConfig {
        source_cloud: body.source_cloud.unwrap_or(SourceCloud::Any),
        dry_run: true,
        catalogue_path: std::env::var("CLOUDSHIFT_PATTERNS_DIR").ok(),
        ..Default::default()
    };
    match transform_source_for_api(path_hint, &body.source, body.language, &config) {
        Ok(result) => Json(result).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Transform failed: {}", e),
        )
            .into_response(),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let static_dir = std::env::var("CLOUDSHIFT_STATIC_DIR").unwrap_or_else(|_| "static".into());
    let has_static = Path::new(&static_dir).is_dir();

    let app = Router::new()
        .route("/favicon.ico", get(favicon))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/api/transform", post(api_transform));
    let app = if has_static {
        app.nest_service("/", ServeDir::new(static_dir))
    } else {
        app.route("/", get(root)).route("/index.html", get(root))
    };
    let app = app.fallback(get(not_found));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
