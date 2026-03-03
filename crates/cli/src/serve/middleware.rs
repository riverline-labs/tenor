//! HTTP middleware: rate limiting and API key authentication.

use std::sync::Arc;

use axum::extract::{ConnectInfo, State};
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;

use super::state::AppState;

/// Rate limiting middleware. Checks per-IP request rate before routing.
pub(crate) async fn rate_limit_middleware(
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
pub(crate) async fn auth_middleware(
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
            return super::json_error(StatusCode::FORBIDDEN, "invalid API key").into_response();
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
        return super::json_error(StatusCode::FORBIDDEN, "invalid API key").into_response();
    }

    super::json_error(StatusCode::UNAUTHORIZED, "authentication required").into_response()
}
