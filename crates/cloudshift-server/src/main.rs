//! Minimal HTTP server for Cloud Run — listens on PORT, auth (IAP / X-Searce-ID / Bearer / X-API-Key), health/ready.

use axum::{
    extract::Request,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

const AUTH_REQUIRED_MSG: &str =
    "IAP / X-Searce-ID / Bearer or valid X-API-Key required";

fn has_valid_auth(req: &Request) -> bool {
    if req.headers().get("X-Goog-IAP-JWT-Assertion").is_some() {
        return true;
    }
    if req.headers().get("X-Searce-ID").and_then(|v| v.to_str().ok()).map(|s| !s.trim().is_empty()).unwrap_or(false) {
        return true;
    }
    if let Some(auth) = req.headers().get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if auth.starts_with("Bearer ") && !auth[7..].trim().is_empty() {
            return true;
        }
    }
    if let Some(api_key) = std::env::var("CLOUDSHIFT_API_KEY").ok().filter(|k| !k.is_empty()) {
        if req.headers().get("X-API-Key").and_then(|v| v.to_str().ok()).map(|s| s.trim() == api_key.trim()).unwrap_or(false) {
            return true;
        }
    }
    false
}

async fn root(req: Request) -> Response {
    if has_valid_auth(&req) {
        "ok".into_response()
    } else {
        (StatusCode::UNAUTHORIZED, AUTH_REQUIRED_MSG).into_response()
    }
}

async fn health() -> &'static str {
    "ok"
}

async fn ready() -> &'static str {
    "ready"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/ready", get(ready));

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
