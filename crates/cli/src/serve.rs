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

use std::collections::HashMap;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, State};
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::{Any, CorsLayer};

use super::explain;
use super::manifest;

/// Maximum request body size: 10 MB.
const MAX_BODY_SIZE: usize = 10 * 1024 * 1024;

/// Maximum source content size for the elaborate endpoint: 1 MB.
const MAX_SOURCE_SIZE: usize = 1024 * 1024;

/// Default rate limit: 60 requests per minute per IP.
const DEFAULT_RATE_LIMIT: u64 = 60;

/// Rate limit window duration in seconds (1 minute).
const RATE_LIMIT_WINDOW_SECS: u64 = 60;

// --- Rate limiter ---

/// Per-IP request tracker: (request count, window start time).
type IpTracker = HashMap<IpAddr, (u64, Instant)>;

/// In-memory per-IP rate limiter.
struct RateLimiter {
    /// Request counts per IP per window.
    tracker: Mutex<IpTracker>,
    /// Maximum requests per window.
    max_requests: u64,
}

impl RateLimiter {
    fn new(max_requests: u64) -> Self {
        Self {
            tracker: Mutex::new(HashMap::new()),
            max_requests,
        }
    }

    /// Check if a request from the given IP is allowed.
    /// Returns Ok(()) if allowed, Err(retry_after_secs) if rate limited.
    async fn check(&self, ip: IpAddr) -> Result<(), u64> {
        let mut tracker = self.tracker.lock().await;
        let now = Instant::now();

        let entry = tracker.entry(ip).or_insert((0, now));

        // Reset window if expired
        let elapsed = now.duration_since(entry.1).as_secs();
        if elapsed >= RATE_LIMIT_WINDOW_SECS {
            entry.0 = 0;
            entry.1 = now;
        }

        entry.0 += 1;
        if entry.0 > self.max_requests {
            let retry_after = RATE_LIMIT_WINDOW_SECS.saturating_sub(elapsed);
            Err(retry_after)
        } else {
            Ok(())
        }
    }
}

/// Application state shared across request handlers.
struct AppState {
    /// Loaded contract bundles keyed by bundle ID.
    contracts: RwLock<HashMap<String, serde_json::Value>>,
    /// Per-IP rate limiter.
    rate_limiter: RateLimiter,
    /// Optional API key for authentication. None = no auth required.
    api_key: Option<String>,
}

// --- Middleware ---

/// Rate limiting middleware. Checks per-IP request rate before routing.
async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let ip = addr.ip();
    match state.rate_limiter.check(ip).await {
        Ok(()) => next.run(request).await,
        Err(retry_after) => {
            let body = serde_json::json!({
                "error": "rate limit exceeded",
                "retry_after": retry_after,
            });
            (StatusCode::TOO_MANY_REQUESTS, Json(body)).into_response()
        }
    }
}

/// API key authentication middleware.
///
/// If `TENOR_API_KEY` is set, all requests (except /health) must include
/// either `Authorization: Bearer <key>` or `X-API-Key: <key>`.
async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let expected_key = match &state.api_key {
        Some(k) => k,
        None => return next.run(request).await, // No auth configured
    };

    // /health is exempt from auth (for load balancer health checks)
    if request.uri().path() == "/health" {
        return next.run(request).await;
    }

    // Check Authorization: Bearer <key>
    let auth_header = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok());

    if let Some(auth) = auth_header {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            if token == expected_key {
                return next.run(request).await;
            }
            return json_error(StatusCode::FORBIDDEN, "invalid API key").into_response();
        }
    }

    // Check X-API-Key header
    let api_key_header = request
        .headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok());

    if let Some(key) = api_key_header {
        if key == expected_key {
            return next.run(request).await;
        }
        return json_error(StatusCode::FORBIDDEN, "invalid API key").into_response();
    }

    json_error(StatusCode::UNAUTHORIZED, "authentication required").into_response()
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
        contracts: RwLock::new(contracts),
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
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(middleware::from_fn_with_state(
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

/// Construct a JSON error response with the given status code and message.
fn json_error(status: StatusCode, message: &str) -> impl IntoResponse {
    (status, Json(serde_json::json!({"error": message})))
}

// --- Route handlers ---

/// Fallback handler for unmatched routes.
async fn handle_not_found() -> impl IntoResponse {
    json_error(StatusCode::NOT_FOUND, "not found")
}

/// GET /health
async fn handle_health() -> impl IntoResponse {
    let response = serde_json::json!({
        "status": "ok",
        "tenor_version": tenor_core::TENOR_BUNDLE_VERSION,
    });
    (StatusCode::OK, Json(response))
}

/// GET /contracts
async fn handle_list_contracts(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let contracts = state.contracts.read().await;

    let contract_list: Vec<serde_json::Value> = contracts
        .iter()
        .map(|(id, bundle)| {
            let constructs = bundle
                .get("constructs")
                .and_then(|c| c.as_array())
                .cloned()
                .unwrap_or_default();

            let mut facts = Vec::new();
            let mut operations = Vec::new();
            let mut flows = Vec::new();
            let mut construct_count = 0;

            for c in &constructs {
                construct_count += 1;
                let kind = c.get("kind").and_then(|k| k.as_str()).unwrap_or("");
                let cid = c.get("id").and_then(|i| i.as_str()).unwrap_or("");
                match kind {
                    "Fact" => facts.push(cid.to_string()),
                    "Operation" => operations.push(cid.to_string()),
                    "Flow" => flows.push(cid.to_string()),
                    _ => {}
                }
            }

            serde_json::json!({
                "id": id,
                "construct_count": construct_count,
                "facts": facts,
                "operations": operations,
                "flows": flows,
            })
        })
        .collect();

    let response = serde_json::json!({ "contracts": contract_list });
    (StatusCode::OK, Json(response))
}

/// GET /contracts/{id}/operations
async fn handle_get_operations(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let contracts = state.contracts.read().await;

    let bundle = match contracts.get(&id) {
        Some(b) => b,
        None => {
            return json_error(
                StatusCode::NOT_FOUND,
                &format!("contract '{}' not found", id),
            )
            .into_response()
        }
    };

    let constructs = bundle
        .get("constructs")
        .and_then(|c| c.as_array())
        .cloned()
        .unwrap_or_default();

    let operations: Vec<serde_json::Value> = constructs
        .iter()
        .filter(|c| c.get("kind").and_then(|k| k.as_str()) == Some("Operation"))
        .map(|op| {
            let op_id = op.get("id").and_then(|i| i.as_str()).unwrap_or("");
            let allowed_personas: Vec<String> = op
                .get("allowed_personas")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let effects: Vec<serde_json::Value> = op
                .get("effects")
                .and_then(|e| e.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|eff| {
                            serde_json::json!({
                                "entity_id": eff.get("entity_id").and_then(|v| v.as_str()).unwrap_or(""),
                                "from": eff.get("from").and_then(|v| v.as_str()).unwrap_or(""),
                                "to": eff.get("to").and_then(|v| v.as_str()).unwrap_or(""),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            let summary: Vec<String> = effects
                .iter()
                .map(|e| {
                    format!(
                        "{} -> {}",
                        e.get("from").and_then(|v| v.as_str()).unwrap_or("?"),
                        e.get("to").and_then(|v| v.as_str()).unwrap_or("?"),
                    )
                })
                .collect();

            serde_json::json!({
                "id": op_id,
                "allowed_personas": allowed_personas,
                "effects": effects,
                "preconditions_summary": summary.join(", "),
            })
        })
        .collect();

    let response = serde_json::json!({ "operations": operations });
    (StatusCode::OK, Json(response)).into_response()
}

/// Sanitize a filename for use in a temp directory.
///
/// Rejects filenames with path separators or `..` components.
/// Returns `source.tenor` for invalid or missing filenames.
fn sanitize_filename(raw: Option<&str>) -> String {
    let name = match raw {
        Some(n) if !n.is_empty() => n,
        _ => return "source.tenor".to_string(),
    };

    // Reject path separators and parent directory references
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return "source.tenor".to_string();
    }

    name.to_string()
}

/// Check if an import path would escape the sandbox directory.
///
/// Scans source text for `import` directives whose paths contain
/// parent-directory traversals (`..`). This is a fast pre-check before
/// writing to disk; pass1_bundle.rs performs full canonicalization later.
fn has_escaping_imports(source: &str) -> bool {
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("import") {
            // Check for .. in the import path
            if rest.contains("..") {
                return true;
            }
        }
    }
    false
}

/// POST /elaborate
async fn handle_elaborate(Json(parsed): Json<serde_json::Value>) -> impl IntoResponse {
    let source = match parsed.get("source").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => {
            return json_error(StatusCode::BAD_REQUEST, "missing 'source' field").into_response()
        }
    };

    // --- Input validation (before any disk I/O) ---

    // 1. Content size validation
    if source.len() > MAX_SOURCE_SIZE {
        return json_error(
            StatusCode::BAD_REQUEST,
            "source content exceeds maximum size",
        )
        .into_response();
    }

    // 2. Null byte rejection
    if source.contains('\0') {
        return json_error(
            StatusCode::BAD_REQUEST,
            "source content must not contain null bytes",
        )
        .into_response();
    }

    // 3. Import injection prevention
    if has_escaping_imports(&source) {
        return json_error(
            StatusCode::BAD_REQUEST,
            "import paths must not escape sandbox",
        )
        .into_response();
    }

    // 4. Filename sanitization
    let filename = sanitize_filename(parsed.get("filename").and_then(|v| v.as_str()));

    let result = tokio::task::spawn_blocking(move || {
        let tmp_dir = tempfile::tempdir()?;
        let tmp_path = tmp_dir.path().join(&filename);
        std::fs::write(&tmp_path, &source)?;
        let elab_result = tenor_core::elaborate::elaborate(&tmp_path);
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(elab_result)
    })
    .await;

    match result {
        Ok(Ok(Ok(bundle))) => (StatusCode::OK, Json(bundle)).into_response(),
        Ok(Ok(Err(e))) => {
            let err_response = serde_json::json!({
                "error": format!("{:?}", e),
                "details": e.to_json_value(),
            });
            (StatusCode::BAD_REQUEST, Json(err_response)).into_response()
        }
        Ok(Err(e)) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("failed to elaborate: {}", e),
        )
        .into_response(),
        Err(e) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("task join error: {}", e),
        )
        .into_response(),
    }
}

/// POST /evaluate
async fn handle_evaluate(
    State(state): State<Arc<AppState>>,
    Json(parsed): Json<serde_json::Value>,
) -> impl IntoResponse {
    let bundle_id = match parsed.get("bundle_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return json_error(StatusCode::BAD_REQUEST, "missing 'bundle_id' field").into_response()
        }
    };

    let facts = match parsed.get("facts") {
        Some(f) => f.clone(),
        None => {
            return json_error(StatusCode::BAD_REQUEST, "missing 'facts' field").into_response()
        }
    };

    let flow_id = parsed
        .get("flow_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let persona = parsed
        .get("persona")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let contracts = state.contracts.read().await;
    let bundle = match contracts.get(&bundle_id) {
        Some(b) => b.clone(),
        None => {
            return json_error(
                StatusCode::NOT_FOUND,
                &format!("contract '{}' not found", bundle_id),
            )
            .into_response()
        }
    };
    drop(contracts);

    if let Some(fid) = flow_id {
        let p = match persona {
            Some(p) => p,
            None => {
                return json_error(
                    StatusCode::BAD_REQUEST,
                    "'persona' is required when 'flow_id' is specified",
                )
                .into_response()
            }
        };

        let fid_for_response = fid.clone();

        let result = tokio::task::spawn_blocking(move || {
            tenor_eval::evaluate_flow(&bundle, &facts, &fid, &p, None)
        })
        .await;

        match result {
            Ok(Ok(result)) => {
                let mut json_output = serde_json::Map::new();
                json_output.insert("flow_id".to_string(), serde_json::json!(fid_for_response));
                json_output.insert(
                    "outcome".to_string(),
                    serde_json::json!(result.flow_result.outcome),
                );
                json_output.insert(
                    "initiating_persona".to_string(),
                    serde_json::json!(result.flow_result.initiating_persona),
                );
                let entity_changes: serde_json::Value = result
                    .flow_result
                    .entity_state_changes
                    .iter()
                    .map(|e| {
                        serde_json::json!({
                            "entity_id": e.entity_id,
                            "from": e.from_state,
                            "to": e.to_state
                        })
                    })
                    .collect();
                json_output.insert("entity_state_changes".to_string(), entity_changes);
                let steps: serde_json::Value = result
                    .flow_result
                    .steps_executed
                    .iter()
                    .map(|s| {
                        serde_json::json!({
                            "step_id": s.step_id,
                            "result": s.result
                        })
                    })
                    .collect();
                json_output.insert("steps_executed".to_string(), steps);
                json_output.insert("verdicts".to_string(), result.verdicts.to_json());
                (StatusCode::OK, Json(serde_json::Value::Object(json_output))).into_response()
            }
            Ok(Err(e)) => {
                json_error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", e)).into_response()
            }
            Err(e) => json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("task join error: {}", e),
            )
            .into_response(),
        }
    } else {
        let result =
            tokio::task::spawn_blocking(move || tenor_eval::evaluate(&bundle, &facts)).await;

        match result {
            Ok(Ok(result)) => (StatusCode::OK, Json(result.verdicts.to_json())).into_response(),
            Ok(Err(e)) => {
                json_error(StatusCode::INTERNAL_SERVER_ERROR, &format!("{}", e)).into_response()
            }
            Err(e) => json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("task join error: {}", e),
            )
            .into_response(),
        }
    }
}

/// POST /explain
async fn handle_explain(
    State(state): State<Arc<AppState>>,
    Json(parsed): Json<serde_json::Value>,
) -> impl IntoResponse {
    let bundle_id = match parsed.get("bundle_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return json_error(StatusCode::BAD_REQUEST, "missing 'bundle_id' field").into_response()
        }
    };

    let contracts = state.contracts.read().await;
    let bundle = match contracts.get(&bundle_id) {
        Some(b) => b.clone(),
        None => {
            return json_error(
                StatusCode::NOT_FOUND,
                &format!("contract '{}' not found", bundle_id),
            )
            .into_response()
        }
    };
    drop(contracts);

    let result = tokio::task::spawn_blocking(move || explain::explain_bundle(&bundle)).await;

    match result {
        Ok(Ok(explanation)) => (StatusCode::OK, Json(explanation)).into_response(),
        Ok(Err(e)) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("explain error: {}", e),
        )
        .into_response(),
        Err(e) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("task join error: {}", e),
        )
        .into_response(),
    }
}

// --- Phase 4 endpoints ---

/// GET /.well-known/tenor
///
/// Contract manifest endpoint per spec §19. Returns the TenorManifest JSON
/// with the full interchange bundle inlined. Sets ETag response header and
/// supports If-None-Match for conditional requests (304 Not Modified).
/// Satisfies executor obligations E10, E11, E12.
async fn handle_well_known_tenor(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Response {
    let contracts = state.contracts.read().await;

    let bundle = match contracts.values().next() {
        Some(b) => b.clone(),
        None => return json_error(StatusCode::NOT_FOUND, "no contracts loaded").into_response(),
    };
    drop(contracts);

    let manifest_value = manifest::build_manifest(bundle);
    let etag = manifest_value
        .get("etag")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let etag_quoted = format!("\"{}\"", etag);

    // Check If-None-Match
    if let Some(inm) = headers.get(header::IF_NONE_MATCH) {
        if let Ok(inm_str) = inm.to_str() {
            if inm_str == etag_quoted || inm_str == etag {
                return StatusCode::NOT_MODIFIED.into_response();
            }
        }
    }

    let mut response = Json(manifest_value).into_response();
    if let Ok(val) = etag_quoted.parse() {
        response.headers_mut().insert(header::ETAG, val);
    }
    response
}

/// Internal error type for simulate_flow to distinguish persona errors from eval errors.
enum SimulateError {
    PersonaNotFound(String),
    Eval(tenor_eval::EvalError),
}

/// Core simulation logic, runs synchronously in a blocking task.
fn simulate_flow_inner(
    bundle: &serde_json::Value,
    facts: &serde_json::Value,
    flow_id: &str,
    persona_id: &str,
    entity_states_input: Option<&serde_json::Value>,
) -> Result<serde_json::Value, SimulateError> {
    let contract = tenor_eval::Contract::from_interchange(bundle).map_err(SimulateError::Eval)?;

    // Check persona exists in contract
    if !contract.personas.contains(&persona_id.to_string()) {
        return Err(SimulateError::PersonaNotFound(persona_id.to_string()));
    }

    // Assemble facts
    let fact_set =
        tenor_eval::assemble::assemble_facts(&contract, facts).map_err(SimulateError::Eval)?;

    // Evaluate rules to produce verdicts
    let verdict_set =
        tenor_eval::rules::eval_strata(&contract, &fact_set).map_err(SimulateError::Eval)?;

    // Create frozen snapshot
    let snapshot = tenor_eval::Snapshot {
        facts: fact_set,
        verdicts: verdict_set.clone(),
    };

    // Build entity states: start from contract defaults, override with request.
    // Overrides use DEFAULT_INSTANCE_ID since the API takes plain entity_id -> state.
    let mut entity_states = tenor_eval::operation::init_entity_states(&contract);
    if let Some(es_input) = entity_states_input {
        if let Some(obj) = es_input.as_object() {
            for (entity_id, state_info) in obj {
                if let Some(state) = state_info.get("state").and_then(|v| v.as_str()) {
                    entity_states.insert(
                        (
                            entity_id.clone(),
                            tenor_eval::DEFAULT_INSTANCE_ID.to_string(),
                        ),
                        state.to_string(),
                    );
                }
            }
        }
    }

    // Find the flow
    let target_flow = contract.get_flow(flow_id).ok_or_else(|| {
        SimulateError::Eval(tenor_eval::EvalError::DeserializeError {
            message: format!("flow '{}' not found in contract", flow_id),
        })
    })?;

    // Build step index for enriching the response path with operation/persona info
    let step_info: HashMap<String, (String, String)> = target_flow
        .steps
        .iter()
        .filter_map(|s| match s {
            tenor_eval::types::FlowStep::OperationStep {
                id, op, persona, ..
            } => Some((id.clone(), (op.clone(), persona.clone()))),
            _ => None,
        })
        .collect();

    // Execute the flow
    let flow_result =
        tenor_eval::flow::execute_flow(target_flow, &contract, &snapshot, &mut entity_states, None)
            .map_err(SimulateError::Eval)?;

    // Build path
    let path: Vec<serde_json::Value> = flow_result
        .steps_executed
        .iter()
        .map(|step| {
            let mut step_json = serde_json::json!({
                "step": step.step_id,
                "type": step.step_type,
                "outcome": step.result,
            });
            if let Some((op_name, step_persona)) = step_info.get(&step.step_id) {
                step_json["operation"] = serde_json::json!(op_name);
                step_json["persona"] = serde_json::json!(step_persona);
            }
            step_json
        })
        .collect();

    // Build would_transition
    let would_transition: Vec<serde_json::Value> = flow_result
        .entity_state_changes
        .iter()
        .map(|e| {
            serde_json::json!({
                "entity_id": e.entity_id,
                "from": e.from_state,
                "to": e.to_state,
            })
        })
        .collect();

    // Build verdicts
    let verdicts: Vec<serde_json::Value> = verdict_set
        .0
        .iter()
        .map(|v| {
            serde_json::json!({
                "type": v.verdict_type,
                "payload": tenor_eval::types::value_to_json(&v.payload),
            })
        })
        .collect();

    Ok(serde_json::json!({
        "simulation": true,
        "flow_id": flow_id,
        "outcome": flow_result.outcome,
        "path": path,
        "would_transition": would_transition,
        "verdicts": verdicts,
    }))
}

/// POST /flows/{flow_id}/simulate
///
/// Dedicated flow simulation endpoint. Stateless — entity states come from
/// the request body, nothing is persisted. Returns simulation: true always.
async fn handle_simulate_flow(
    State(state): State<Arc<AppState>>,
    Path(flow_id): Path<String>,
    Json(parsed): Json<serde_json::Value>,
) -> Response {
    let persona_id = match parsed.get("persona_id").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            return json_error(StatusCode::BAD_REQUEST, "missing 'persona_id' field")
                .into_response()
        }
    };

    let facts = parsed
        .get("facts")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let entity_states_input = parsed.get("entity_states").cloned();

    // Find the contract containing this flow
    let contracts = state.contracts.read().await;
    let mut found_bundle: Option<serde_json::Value> = None;

    for bundle in contracts.values() {
        if let Some(constructs) = bundle.get("constructs").and_then(|c| c.as_array()) {
            let has_flow = constructs.iter().any(|c| {
                c.get("kind").and_then(|k| k.as_str()) == Some("Flow")
                    && c.get("id").and_then(|i| i.as_str()) == Some(&flow_id)
            });
            if has_flow {
                found_bundle = Some(bundle.clone());
                break;
            }
        }
    }
    drop(contracts);

    let bundle = match found_bundle {
        Some(b) => b,
        None => {
            return json_error(
                StatusCode::NOT_FOUND,
                &format!("flow '{}' not found", flow_id),
            )
            .into_response()
        }
    };

    let fid = flow_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        simulate_flow_inner(
            &bundle,
            &facts,
            &fid,
            &persona_id,
            entity_states_input.as_ref(),
        )
    })
    .await;

    match result {
        Ok(Ok(response_json)) => (StatusCode::OK, Json(response_json)).into_response(),
        Ok(Err(SimulateError::PersonaNotFound(p))) => json_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            &format!("persona '{}' not found in contract", p),
        )
        .into_response(),
        Ok(Err(SimulateError::Eval(e))) => {
            json_error(StatusCode::UNPROCESSABLE_ENTITY, &format!("{}", e)).into_response()
        }
        Err(e) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("task join error: {}", e),
        )
        .into_response(),
    }
}

/// POST /actions
///
/// Compute the action space for a persona. Stateless — facts and entity
/// states come from the request body, nothing is persisted. Returns all
/// available and blocked actions with reasons.
///
/// Input: { "persona_id": "...", "facts": {...}, "entity_states": {...} }
/// Output: ActionSpace JSON
async fn handle_actions(
    State(state): State<Arc<AppState>>,
    Json(parsed): Json<serde_json::Value>,
) -> Response {
    let persona_id = match parsed.get("persona_id").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            return json_error(StatusCode::BAD_REQUEST, "missing 'persona_id' field")
                .into_response()
        }
    };

    let facts = parsed
        .get("facts")
        .cloned()
        .unwrap_or(serde_json::json!({}));
    let entity_states_input: std::collections::BTreeMap<String, String> =
        match parsed.get("entity_states") {
            Some(v) => match serde_json::from_value(v.clone()) {
                Ok(m) => m,
                Err(e) => {
                    return json_error(
                        StatusCode::BAD_REQUEST,
                        &format!("invalid entity_states: {}", e),
                    )
                    .into_response()
                }
            },
            None => std::collections::BTreeMap::new(),
        };

    // Find first loaded contract (same pattern as /evaluate)
    let contracts = state.contracts.read().await;
    let bundle = match contracts.values().next() {
        Some(b) => b.clone(),
        None => return json_error(StatusCode::NOT_FOUND, "no contracts loaded").into_response(),
    };
    drop(contracts);

    let result = tokio::task::spawn_blocking(move || {
        let contract = match tenor_eval::Contract::from_interchange(&bundle) {
            Ok(c) => c,
            Err(e) => return Err(format!("invalid contract: {}", e)),
        };

        // Convert flat entity_id -> state map to composite (entity_id, instance_id) key format.
        let entity_states = tenor_eval::single_instance(entity_states_input);
        tenor_eval::action_space::compute_action_space(
            &contract,
            &facts,
            &entity_states,
            &persona_id,
        )
        .map_err(|e| format!("{}", e))
    })
    .await;

    match result {
        Ok(Ok(action_space)) => match serde_json::to_value(&action_space) {
            Ok(json) => (StatusCode::OK, Json(json)).into_response(),
            Err(e) => json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("serialization error: {}", e),
            )
            .into_response(),
        },
        Ok(Err(e)) => json_error(StatusCode::UNPROCESSABLE_ENTITY, &e).into_response(),
        Err(e) => json_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("task join error: {}", e),
        )
        .into_response(),
    }
}

/// GET /inspect
///
/// Structured summary of the loaded contract. Returns all declared constructs
/// with their key properties — enough for an agent to cold-start or a UI to
/// render a contract explorer.
async fn handle_inspect(State(state): State<Arc<AppState>>) -> Response {
    let contracts = state.contracts.read().await;

    let mut all_facts = Vec::new();
    let mut all_entities = Vec::new();
    let mut all_rules = Vec::new();
    let mut all_personas = Vec::new();
    let mut all_operations = Vec::new();
    let mut all_flows = Vec::new();
    let mut etag_bundle = None;

    for bundle in contracts.values() {
        if etag_bundle.is_none() {
            etag_bundle = Some(bundle.clone());
        }

        let parsed = match tenor_interchange::from_interchange(bundle) {
            Ok(p) => p,
            Err(e) => {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("failed to parse bundle: {}", e),
                )
                .into_response()
            }
        };

        for construct in &parsed.constructs {
            match construct {
                tenor_interchange::InterchangeConstruct::Fact(f) => {
                    let base = f
                        .fact_type
                        .get("base")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let mut fact_json = serde_json::json!({
                        "id": f.id,
                        "type": base,
                    });
                    if let Some(ref src) = f.source {
                        fact_json["source"] = src.clone();
                    }
                    if f.default.is_some() {
                        fact_json["has_default"] = serde_json::json!(true);
                    }
                    // Include full type spec for richer inspection
                    fact_json["type_spec"] = f.fact_type.clone();
                    all_facts.push(fact_json);
                }
                tenor_interchange::InterchangeConstruct::Entity(e) => {
                    let transitions: Vec<serde_json::Value> = e
                        .transitions
                        .iter()
                        .map(|t| {
                            serde_json::json!({
                                "from": t.from,
                                "to": t.to,
                            })
                        })
                        .collect();
                    all_entities.push(serde_json::json!({
                        "id": e.id,
                        "states": e.states,
                        "initial": e.initial,
                        "transitions": transitions,
                    }));
                }
                tenor_interchange::InterchangeConstruct::Rule(r) => {
                    let condition_summary = r.when().map(summarize_condition).unwrap_or_default();
                    let verdict_type = r
                        .produce()
                        .and_then(|p| p.get("verdict_type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    all_rules.push(serde_json::json!({
                        "id": r.id,
                        "stratum": r.stratum,
                        "produces": verdict_type,
                        "condition_summary": condition_summary,
                    }));
                }
                tenor_interchange::InterchangeConstruct::Persona(p) => {
                    all_personas.push(serde_json::json!({ "id": p.id }));
                }
                tenor_interchange::InterchangeConstruct::Operation(op) => {
                    let effects: Vec<serde_json::Value> = op
                        .effects
                        .iter()
                        .map(|e| {
                            serde_json::json!({
                                "entity_id": e.entity_id,
                                "from": e.from,
                                "to": e.to,
                            })
                        })
                        .collect();
                    let precondition_summary = op
                        .precondition
                        .as_ref()
                        .map(summarize_condition)
                        .unwrap_or_else(|| "none".to_string());
                    all_operations.push(serde_json::json!({
                        "id": op.id,
                        "allowed_personas": op.allowed_personas,
                        "effects": effects,
                        "precondition_summary": precondition_summary,
                        "outcomes": op.outcomes,
                    }));
                }
                tenor_interchange::InterchangeConstruct::Flow(f) => {
                    let step_ids: Vec<&str> = f
                        .steps
                        .iter()
                        .filter_map(|s| s.get("id").and_then(|v| v.as_str()))
                        .collect();
                    all_flows.push(serde_json::json!({
                        "id": f.id,
                        "entry": f.entry,
                        "steps": step_ids,
                    }));
                }
                tenor_interchange::InterchangeConstruct::Source(_)
                | tenor_interchange::InterchangeConstruct::System(_)
                | tenor_interchange::InterchangeConstruct::TypeDecl(_) => {}
            }
        }
    }
    drop(contracts);

    let etag = etag_bundle
        .map(|b| manifest::compute_etag(&b))
        .unwrap_or_default();

    let response = serde_json::json!({
        "facts": all_facts,
        "entities": all_entities,
        "rules": all_rules,
        "personas": all_personas,
        "operations": all_operations,
        "flows": all_flows,
        "etag": etag,
    });

    (StatusCode::OK, Json(response)).into_response()
}

/// Produce a human-readable summary of a condition expression for inspect output.
fn summarize_condition(cond: &serde_json::Value) -> String {
    if cond.is_null() {
        return "always".to_string();
    }
    if let Some(vp) = cond.get("verdict_present").and_then(|v| v.as_str()) {
        return format!("verdict '{}' present", vp);
    }
    if let Some(op) = cond.get("op").and_then(|v| v.as_str()) {
        let left = cond
            .get("left")
            .and_then(|l| l.get("fact_ref").and_then(|v| v.as_str()))
            .unwrap_or("...");
        let right = cond
            .get("right")
            .and_then(|r| {
                r.get("literal")
                    .map(|l| format!("{}", l))
                    .or_else(|| r.get("fact_ref").and_then(|v| v.as_str()).map(String::from))
            })
            .unwrap_or_else(|| "...".to_string());
        return format!("{} {} {}", left, op, right);
    }
    "complex".to_string()
}
