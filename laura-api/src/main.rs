//! `laura-api` — Fly-hosted HTTP surface for `call-laura`.
//!
//! Deploy-only server binary (see `Cargo.toml`: `publish = false`), matching
//! `ternlang-api`'s convention. `call_laura_core::review()` is fully synchronous
//! and local (no network call, no API key) — key-gating and rate-limiting here
//! exist for basic abuse/DoS hygiene on a public endpoint, not cost protection
//! (there's no per-request external API cost anymore).
//!
//! **Live** at https://laura-api.fly.dev, deployed 2026-07-12. `/mcp` (keyless,
//! rate-limited) is what Smithery's listing points at. `/review` (key-gated) is
//! the REST convenience route, `LAURA_API_KEYS` set as a Fly secret.

use axum::{
    extract::{ConnectInfo, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use call_laura_core::mcp::{handle_request, RpcRequest, RpcResponse};
use call_laura_core::schema::ReviewRequest;
use laura_team::orchestrator::{review_team, TeamRequest};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Mutex,
    time::{Duration, Instant},
};

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);
const RATE_LIMIT_MAX_REQUESTS: usize = 10;

struct AppState {
    allowed_keys: Vec<String>,
    rate_limiter: Mutex<HashMap<String, Vec<Instant>>>,
}

fn check_rate_limit(state: &AppState, ip: &str) -> bool {
    let mut map = state.rate_limiter.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    let entry = map.entry(ip.to_string()).or_default();
    entry.retain(|t| now.duration_since(*t) < RATE_LIMIT_WINDOW);
    if entry.len() >= RATE_LIMIT_MAX_REQUESTS {
        return false;
    }
    entry.push(now);
    true
}

fn require_api_key(state: &AppState, headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    // No key, no entry. The /mcp route is intentionally keyless (local CPU,
    // rate-limited) — but /review and /team are gated. If you curl these
    // without a Bearer token you get 401 before any work happens. We expected
    // you. — RFI-IRFOS, post-hardening, 2026-07-16
    let provided = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if provided.is_empty() {
        return Err((StatusCode::UNAUTHORIZED, "missing Authorization: Bearer <key> header".to_string()));
    }
    if !state.allowed_keys.iter().any(|k| k == provided) {
        return Err((StatusCode::FORBIDDEN, "invalid API key".to_string()));
    }
    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok", "service": "laura-api", "version": env!("CARGO_PKG_VERSION") }))
}

async fn review_handler(
    State(state): State<std::sync::Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ReviewRequest>,
) -> impl IntoResponse {
    if let Err((status, msg)) = require_api_key(&state, &headers) {
        return (status, Json(serde_json::json!({ "error": msg }))).into_response();
    }
    if !check_rate_limit(&state, &addr.ip().to_string()) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({ "error": format!("rate limit exceeded — {RATE_LIMIT_MAX_REQUESTS} requests per {RATE_LIMIT_WINDOW:?} per IP") })),
        )
        .into_response();
    }
    if req.text.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": "text must not be empty" }))).into_response();
    }
    if req.lenses.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "lenses must not be an empty array — omit the field entirely to run all four" })),
        )
        .into_response();
    }

    let response = call_laura_core::review(&req);
    (StatusCode::OK, Json(response)).into_response()
}

/// Paid module: Laura's 15-agent team review (`laura-team` crate, BSL-1.1).
/// Key-gated and rate-limited like `/review`. Same honest-partial-failure
/// semantics as the free core, but over the full expert stack.
async fn team_handler(
    State(state): State<std::sync::Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<TeamRequest>,
) -> impl IntoResponse {
    if let Err((status, msg)) = require_api_key(&state, &headers) {
        return (status, Json(serde_json::json!({ "error": msg }))).into_response();
    }
    if !check_rate_limit(&state, &addr.ip().to_string()) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({ "error": format!("rate limit exceeded — {RATE_LIMIT_MAX_REQUESTS} requests per {RATE_LIMIT_WINDOW:?} per IP") })),
        )
        .into_response();
    }
    // review_team itself refuses empty text and returns a partial marker when an
    // agent has no classifiable signal — no need to pre-check here beyond key/rate.
    let response = review_team(&req);
    (StatusCode::OK, Json(response)).into_response()
}

/// MCP JSON-RPC over HTTP — what Smithery's `startCommand: type: http` expects at
/// its `url`. Deliberately keyless (rate-limited only): matches
/// `ternlang-mcp`'s own Smithery listing ("all tools free, no key needed") and
/// there's no per-request external API cost here to protect against — this is
/// local CPU work, not a paid inference call.
async fn mcp_handler(
    State(state): State<std::sync::Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(req): Json<RpcRequest>,
) -> impl IntoResponse {
    if !check_rate_limit(&state, &addr.ip().to_string()) {
        let id = req.id.clone().unwrap_or(serde_json::Value::Null);
        return (StatusCode::TOO_MANY_REQUESTS, Json(RpcResponse::err(id, -32000, "rate limit exceeded"))).into_response();
    }
    (StatusCode::OK, Json(handle_request(req))).into_response()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let allowed_keys: Vec<String> = std::env::var("LAURA_API_KEYS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if allowed_keys.is_empty() {
        eprintln!("[laura-api] fatal: LAURA_API_KEYS is not set (comma-separated allowlist) — refusing to start with zero valid keys, which would leave every request unauthorized rather than open.");
        std::process::exit(1);
    }

    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
    let state = std::sync::Arc::new(AppState { allowed_keys, rate_limiter: Mutex::new(HashMap::new()) });

    let allowed_origins: Vec<axum::http::HeaderValue> = std::env::var("LAURA_API_CORS_ORIGINS")
        .unwrap_or_default()
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .filter_map(|s| axum::http::HeaderValue::from_str(&s).ok())
        .collect();
    // Secure default: no cross-origin sharing. MCP hosts (Smithery, etc.) call
    // this endpoint server-to-server, so CORS is not required for them; this
    // only restricts browser-based cross-origin callers. Set
    // LAURA_API_CORS_ORIGINS (comma-separated) to allow specific browser origins.
    let cors = if allowed_origins.is_empty() {
        tower_http::cors::CorsLayer::new()
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
            .allow_headers([axum::http::header::AUTHORIZATION, axum::http::header::CONTENT_TYPE])
    } else {
        tower_http::cors::CorsLayer::new()
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
            .allow_headers([axum::http::header::AUTHORIZATION, axum::http::header::CONTENT_TYPE])
            .allow_origin(allowed_origins)
    };

    let app = Router::new()
        .route("/health", get(health))
        .route("/review", post(review_handler))
        .route("/team", post(team_handler))
        .route("/mcp", post(mcp_handler))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("[laura-api] listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind failed");
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await.expect("server failed");
}
