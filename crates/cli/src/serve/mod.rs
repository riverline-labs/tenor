//! `tenor serve` -- HTTP JSON API server for the Tenor evaluator.
//!
//! Exposes the Rust elaborator and evaluator as an async HTTP service
//! using `axum` + `tokio`. Supports concurrent request handling.
//!
//! Security features:
//! - Input validation on elaborate endpoint (size, encoding, imports, filename)
//! - CORS headers on all responses (permissive for local dev)
//! - Per-IP rate limiting (default: 60 req/min, configurable)
//! - Optional API key authentication via TENOR_API_KEY env var
//!
//! Endpoints:
//! - GET  /health                      - Server status (exempt from auth)
//! - GET  /contracts                   - List loaded contract bundles
//! - GET  /contracts/{id}/operations   - Operations for a specific contract
//! - GET  /.well-known/tenor           - Contract manifest with ETag (spec §19)
//! - GET  /inspect                     - Structured contract summary
//! - POST /elaborate                   - Elaborate .tenor source text
//! - POST /evaluate                    - Evaluate a contract against facts
//! - POST /explain                     - Explain a contract bundle
//! - POST /flows/{flow_id}/simulate    - Stateless flow simulation
//! - POST /actions                     - Action space for a persona
//!
//! All responses use Content-Type: application/json.

mod handlers;
mod inspect;
mod middleware;
mod simulate;
mod state;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::http::{Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{middleware as axum_middleware, Json, Router};
use tower_http::cors::{Any, CorsLayer};

use self::handlers::{
    handle_elaborate, handle_evaluate, handle_explain, handle_get_operations, handle_health,
    handle_list_contracts, handle_not_found,
};
use self::inspect::{handle_inspect, handle_well_known_tenor};
use self::middleware::{auth_middleware, rate_limit_middleware};
use self::simulate::{handle_actions, handle_simulate_flow};
use self::state::{AppState, RateLimiter};

/// Maximum request body size: 10 MB.
const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;

/// Maximum source content size for the elaborate endpoint: 1 MB.
const MAX_SOURCE_SIZE: usize = 1024 * 1024;

/// Default rate limit: 60 requests per minute per IP.
const DEFAULT_RATE_LIMIT: u64 = 60;

/// Rate limit window duration in seconds (1 minute).
const RATE_LIMIT_WINDOW_SECS: u64 = 60;

/// Construct a JSON error response with the given status code and message.
fn json_error(status: StatusCode, message: &str) -> impl IntoResponse {
    (status, Json(serde_json::json!({"error": message})))
}

/// Start the HTTP server on the given port, optionally pre-loading contracts.
///
/// When TLS cert/key paths are provided, the server listens over HTTPS
/// using `axum-server` with rustls. Otherwise it uses plain HTTP.
///
/// Security:
/// - CORS: Permissive (`Any` origin) for local dev; tighten for production.
/// - Rate limit: Per-IP, configurable via `rate_limit` param (default 60 req/min).
/// - API key: If `TENOR_API_KEY` env var is set, all endpoints except /health require auth.
pub async fn start_server(
    port: u16,
    contract_paths: Vec<PathBuf>,
    _tls_cert: Option<PathBuf>,
    _tls_key: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut contracts = HashMap::new();

    // Pre-load contracts
    for path in &contract_paths {
        match tenor_core::elaborate::elaborate(path) {
            Ok(bundle) => {
                let bundle_id = bundle
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                eprintln!("Loaded contract: {} (from {})", bundle_id, path.display());
                contracts.insert(bundle_id, bundle);
            }
            Err(e) => {
                eprintln!("Warning: failed to load {}: {:?}", path.display(), e);
            }
        }
    }

    // Rate limit: from TENOR_RATE_LIMIT env var, or default
    let rate_limit = std::env::var("TENOR_RATE_LIMIT")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_RATE_LIMIT);

    // API key: from TENOR_API_KEY env var (None = no auth)
    let api_key = std::env::var("TENOR_API_KEY")
        .ok()
        .filter(|k| !k.is_empty());

    if api_key.is_some() {
        eprintln!("API key authentication enabled");
    }
    eprintln!("Rate limit: {} requests per minute per IP", rate_limit);

    let state = Arc::new(AppState {
        contracts: tokio::sync::RwLock::new(contracts),
        rate_limiter: RateLimiter::new(rate_limit),
        api_key,
    });

    // CORS: permissive for local dev (Phase 22 will tighten for production)
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(handle_health))
        .route("/contracts", get(handle_list_contracts))
        .route("/contracts/{id}/operations", get(handle_get_operations))
        .route("/.well-known/tenor", get(handle_well_known_tenor))
        .route("/inspect", get(handle_inspect))
        .route("/elaborate", post(handle_elaborate))
        .route("/evaluate", post(handle_evaluate))
        .route("/explain", post(handle_explain))
        .route("/flows/{flow_id}/simulate", post(handle_simulate_flow))
        .route("/actions", post(handle_actions))
        .fallback(handle_not_found)
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .layer(cors)
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);

    // TLS support via axum-server + rustls (requires `tls` feature)
    #[cfg(feature = "tls")]
    if let (Some(cert_path), Some(key_path)) = (&_tls_cert, &_tls_key) {
        let config =
            axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path).await?;
        let socket_addr: std::net::SocketAddr = addr.parse()?;
        eprintln!("Tenor evaluator listening on https://0.0.0.0:{}", port);
        axum_server::bind_rustls(socket_addr, config)
            .serve(app.into_make_service_with_connect_info::<std::net::SocketAddr>())
            .await?;
        return Ok(());
    }

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("Tenor evaluator listening on http://0.0.0.0:{}", port);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    eprintln!("\nServer shut down.");
    Ok(())
}

/// Wait for a shutdown signal (Ctrl+C).
async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    eprintln!("\nReceived shutdown signal...");
}
