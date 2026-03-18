//! HTTP server — auth, rate limiting, transform API, static UI.

mod github;

use axum::{
    body::Body,
    extract::{ConnectInfo, DefaultBodyLimit, State},
    http::{header, HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use cloudshift_core::{
    pipeline::{transform_source_for_api, TransformConfig},
    Language, SourceCloud,
};
use iap_jwt::ValidationConfig;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tower::ServiceBuilder;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;

const AUTH_REQUIRED_MSG: &str = "Valid X-API-Key or IAP JWT required";
const RATE_LIMIT_MSG: &str = "Too many transform requests; try again shortly";
const GITHUB_RATE_LIMIT_MSG: &str = "Too many GitHub imports; try again shortly";

const DEFAULT_TRANSFORM_RPM: u32 = 90;
const DEFAULT_GITHUB_RPM: u32 = 15;
const RATE_WINDOW: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct RateLimitState {
    rate: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    transform_rpm: u32,
    github_rpm: u32,
}

pub struct AppState {
    pub api_key: Option<String>,
    pub iap_audiences: Vec<String>,
    pub http: reqwest::Client,
    pub rate: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    pub transform_rpm: u32,
    pub github_rpm: u32,
}

fn client_key(headers: &HeaderMap, addr: SocketAddr) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| addr.ip().to_string())
}

fn check_rate(rl: &RateLimitState, key: &str, rpm: u32) -> bool {
    let max = rpm.max(1) as usize;
    let mut map = rl.rate.lock().unwrap();
    let now = Instant::now();
    let v = map.entry(key.to_string()).or_default();
    v.retain(|t| now.duration_since(*t) < RATE_WINDOW);
    if v.len() >= max {
        return false;
    }
    v.push(now);
    true
}

async fn rate_limit_transform(
    State(rl): State<RateLimitState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let addr = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0)
        .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 0)));
    let key = format!("t:{}", client_key(req.headers(), addr));
    if !check_rate(&rl, &key, rl.transform_rpm) {
        return (StatusCode::TOO_MANY_REQUESTS, RATE_LIMIT_MSG).into_response();
    }
    next.run(req).await
}

async fn rate_limit_github(
    State(rl): State<RateLimitState>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let addr = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|c| c.0)
        .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 1], 0)));
    let key = format!("g:{}", client_key(req.headers(), addr));
    if !check_rate(&rl, &key, rl.github_rpm) {
        return (StatusCode::TOO_MANY_REQUESTS, GITHUB_RATE_LIMIT_MSG).into_response();
    }
    next.run(req).await
}

pub async fn auth_valid(state: &AppState, headers: &HeaderMap) -> bool {
    if let Some(ref k) = state.api_key {
        if !k.is_empty()
            && headers
                .get("x-api-key")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim() == k.as_str())
                .unwrap_or(false)
        {
            return true;
        }
    }
    if !state.iap_audiences.is_empty() {
        if let Some(jwt) = headers.get("x-goog-iap-jwt-assertion") {
            if let Ok(token) = jwt.to_str() {
                let cfg = ValidationConfig::new(state.iap_audiences.iter());
                if cfg.decode_and_validate(token, &state.http).await.is_ok() {
                    return true;
                }
            }
        }
    }
    false
}

async fn root() -> &'static str {
    "ok"
}

async fn health() -> &'static str {
    "ok"
}

async fn ready() -> &'static str {
    "ready"
}

async fn favicon() -> Response {
    (StatusCode::NO_CONTENT, ()).into_response()
}

async fn not_found() -> Response {
    (StatusCode::NOT_FOUND, "Not found").into_response()
}

async fn api_auth_check(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let ok = auth_valid(state.as_ref(), &headers).await;
    let (status, body) = if ok {
        (StatusCode::OK, "{\"ok\":true}")
    } else {
        (StatusCode::UNAUTHORIZED, "{\"ok\":false}")
    };
    (status, [(header::CONTENT_TYPE, "application/json")], body).into_response()
}

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

async fn api_transform(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Json<TransformRequestBody>,
) -> Response {
    if !auth_valid(state.as_ref(), &headers).await {
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
        Err(_e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Transform failed — check server logs",
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
struct GithubRepoBody {
    url: String,
    #[serde(default)]
    r#ref: Option<String>,
}

async fn api_github_repo(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<GithubRepoBody>,
) -> Response {
    if !auth_valid(state.as_ref(), &headers).await {
        return (StatusCode::UNAUTHORIZED, AUTH_REQUIRED_MSG).into_response();
    }
    let url = body.url.trim();
    if url.is_empty() || url.len() > 2048 {
        return (
            StatusCode::BAD_REQUEST,
            "Missing or invalid url (max 2048 chars)",
        )
            .into_response();
    }

    let gh = match reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .user_agent("CloudShift-Server/1")
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "Server misconfiguration").into_response();
        }
    };

    let resp = github::import_github_repo(&gh, url, body.r#ref.as_deref()).await;
    Json(resp).into_response()
}

fn parse_iap_audiences(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

pub fn build_state() -> anyhow::Result<Arc<AppState>> {
    let api_key = std::env::var("CLOUDSHIFT_API_KEY")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let iap_raw = std::env::var("CLOUDSHIFT_IAP_AUDIENCE").unwrap_or_default();
    let iap_audiences = parse_iap_audiences(&iap_raw);

    if api_key.is_none() && iap_audiences.is_empty() {
        anyhow::bail!(
            "Set CLOUDSHIFT_API_KEY and/or CLOUDSHIFT_IAP_AUDIENCE (comma-separated OAuth client IDs)"
        );
    }

    let transform_rpm = std::env::var("CLOUDSHIFT_TRANSFORM_RPM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_TRANSFORM_RPM);

    let github_rpm = std::env::var("CLOUDSHIFT_GITHUB_RPM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_GITHUB_RPM);

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let rate = Arc::new(Mutex::new(HashMap::new()));

    Ok(Arc::new(AppState {
        api_key,
        iap_audiences,
        http,
        rate: rate.clone(),
        transform_rpm,
        github_rpm,
    }))
}

/// Builds the Axum router (for tests and production).
pub fn app(state: Arc<AppState>, static_dir: &str) -> Router {
    let has_static = Path::new(static_dir).is_dir();

    let rate_state = RateLimitState {
        rate: state.rate.clone(),
        transform_rpm: state.transform_rpm,
        github_rpm: state.github_rpm,
    };
    let rate_state_gh = RateLimitState {
        rate: state.rate.clone(),
        transform_rpm: state.transform_rpm,
        github_rpm: state.github_rpm,
    };

    let base = Router::new()
        .route("/favicon.ico", get(favicon))
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/api/auth-check", get(api_auth_check))
        .route(
            "/api/github/repo",
            post(api_github_repo)
                .layer(DefaultBodyLimit::max(4096))
                .layer(middleware::from_fn_with_state(
                    rate_state_gh,
                    rate_limit_github,
                )),
        )
        .route(
            "/api/transform",
            post(api_transform)
                .layer(DefaultBodyLimit::max(4 * 1024 * 1024))
                .layer(middleware::from_fn_with_state(
                    rate_state,
                    rate_limit_transform,
                )),
        );

    let routed = if has_static {
        base.fallback_service(ServeDir::new(static_dir))
    } else {
        base.route("/", get(root))
            .route("/index.html", get(root))
            .fallback(get(not_found))
    };

    // Monaco editor needs wasm-unsafe-eval and blob workers
    let csp = "default-src 'self'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'; worker-src 'self' blob:; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'";
    let security = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_CONTENT_TYPE_OPTIONS,
            header::HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::X_FRAME_OPTIONS,
            header::HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::REFERRER_POLICY,
            header::HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            header::CONTENT_SECURITY_POLICY,
            header::HeaderValue::from_str(csp).expect("static CSP"),
        ));

    routed.layer(security).with_state(state)
}

pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let state = build_state()?;
    if state.api_key.is_some() {
        tracing::info!("CLOUDSHIFT_API_KEY is set");
    }
    if !state.iap_audiences.is_empty() {
        tracing::info!("CLOUDSHIFT_IAP_AUDIENCE is set (IAP JWT verification enabled)");
    }
    tracing::info!(
        "Transform rate limit: {} requests/min per client",
        state.transform_rpm
    );
    tracing::info!(
        "GitHub import rate limit: {} requests/min per client",
        state.github_rpm
    );

    let static_dir = std::env::var("CLOUDSHIFT_STATIC_DIR").unwrap_or_else(|_| "static".into());
    let app = app(state, &static_dir);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
