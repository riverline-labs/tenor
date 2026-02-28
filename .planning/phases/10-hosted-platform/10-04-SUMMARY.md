---
phase: 10-hosted-platform
plan: "04"
subsystem: api
tags: [rate-limiting, cors, middleware, token-bucket, axum, tower-http]

requires:
  - phase: 10-hosted-platform/10-02
    provides: AuthContext, auth_middleware, PlanTier, TenantStore

provides:
  - Token bucket rate limiter with per-plan-tier limits (Free/Pro/Enterprise)
  - CorsConfig with per-org origin overrides and static CorsLayer
  - Structured request logging middleware (no body, X-Request-Id)
  - 429 Too Many Requests with Retry-After header
  - ContractsMap type alias for shared RwLock contracts map
  - Gateway middleware stack wired into apply_middleware()

affects:
  - 10-05 (subsequent gateway/serving plans)
  - Any plan modifying middleware.rs, routes.rs, or AppState

tech-stack:
  added:
    - tower-http cors feature (CorsLayer, AllowOrigin)
    - tokio::sync::RwLock (for ContractsMap)
    - sqlx as production dependency (management.rs uses PgPool directly)
  patterns:
    - Token bucket with lazy eviction (10-minute idle threshold)
    - Optional rate_limiter/cors_config args to apply_middleware (None = single-tenant passthrough)
    - Route_layer for rate limiting (runs after auth middleware's route_layer)
    - ContractsMap<S> type alias to avoid clippy::type_complexity

key-files:
  created:
    - crates/platform-serve/src/rate_limit.rs
    - crates/platform-serve/src/gateway.rs
  modified:
    - crates/platform-serve/src/middleware.rs
    - crates/platform-serve/src/lib.rs
    - crates/platform-serve/src/provisioning.rs
    - crates/platform-serve/Cargo.toml

key-decisions:
  - "Token bucket per (api_key_id, RateLimitCategory) pair with lazy eviction at 10 min idle"
  - "rate_limit_middleware uses route_layer (not layer) so auth middleware sets AuthContext first"
  - "429 retry_after uses Duration::ceil() rounded to full seconds"
  - "CorsConfig.org_origins falls back to default_origins for unrecognized org IDs"
  - "MigrationPolicy::from_str renamed to parse() to avoid clippy::should_implement_trait"
  - "ContractsMap<S> type alias added to lib.rs to satisfy clippy::type_complexity"
  - "sqlx added as production dependency (was dev-only; management.rs needs PgPool at compile time)"
  - "apply_middleware() signature gains Option<Arc<RateLimiterStore>> and Option<Arc<CorsConfig>>; None = no rate limiting (single-tenant mode)"

patterns-established:
  - "Gateway integration tests use combined closure that injects AuthContext then calls middleware directly"
  - "classify_request() checks path for /evaluate, /simulate, /execute substrings"
  - "extract_contract_id() skips api/health prefixes to avoid false positives"

requirements-completed:
  - Token bucket rate limiting per API key based on plan tier
  - Request routing to correct tenant executor
  - CORS configuration per organization
  - Request/response logging (no body logging)
  - 429 Too Many Requests with Retry-After header

duration: 21min
completed: 2026-02-27
---

# Phase 10 Plan 04: API Gateway Summary

**Token bucket rate limiting (Free/Pro/Enterprise tiers), per-org CORS, structured request logging without body content, and 429 responses with Retry-After headers**

## Performance

- **Duration:** 21 min (1266 seconds)
- **Started:** 2026-02-27T~19:10:46Z
- **Completed:** 2026-02-27T~19:31:52Z
- **Tasks:** 7
- **Files modified:** 7

## Accomplishments

- Implemented token bucket rate limiter (`rate_limit.rs`): per-(api_key, category) buckets, lazy eviction, `RateLimiterStore` with async `check()`, `RateLimits::for_plan()` (Free: 100 eval/10 exec, Pro: 10k/1k, Enterprise: unlimited)
- Implemented gateway middleware (`gateway.rs`): `rate_limit_middleware` returning 429+Retry-After, `cors_layer()` building tower-http `CorsLayer`, `CorsConfig` with per-org overrides, `request_logging_middleware` with structured tracing and X-Request-Id generation/propagation
- Wired gateway into middleware stack (`middleware.rs`): CORS before auth, logging outermost, rate limiting as route_layer after auth
- Added 11 gateway unit tests + 6 rate_limit unit tests, all passing

## Task Commits

1. **Tasks 1-4: rate_limit.rs + gateway.rs** - `5d7cca7` (feat)
2. **Task 5: middleware stack wiring** - `e8cd308` (feat)
3. **Task 6: gateway integration tests** - `be3e441` (feat)
4. **Task 7: quality gates** - `90392a4` (chore)

## Files Created/Modified

- `crates/platform-serve/src/rate_limit.rs` — TokenBucket, RateLimiterStore, RateLimits::for_plan()
- `crates/platform-serve/src/gateway.rs` — rate_limit_middleware, cors_layer(), CorsConfig, request_logging_middleware, classify_request(), extract_contract_id()
- `crates/platform-serve/src/middleware.rs` — apply_middleware() gains rate_limiter + cors_config args
- `crates/platform-serve/src/lib.rs` — AppState gains rate_limiter + cors_config fields; ContractsMap type alias; cors_default_origins in ServerConfig
- `crates/platform-serve/src/provisioning.rs` — MigrationPolicy renamed from_str->parse(), derive(Default)
- `crates/platform-serve/Cargo.toml` — tower-http cors feature, sqlx as production dep

## Decisions Made

- `apply_middleware()` takes `Option<Arc<RateLimiterStore>>` / `Option<Arc<CorsConfig>>` — `None` means single-tenant mode (no rate limiting). Clean backward compat.
- `rate_limit_middleware` used as `route_layer` so auth middleware's `route_layer` sets `AuthContext` first.
- `MigrationPolicy::from_str` renamed to `parse()` to satisfy clippy::should_implement_trait.
- `ContractsMap<S>` type alias added to avoid clippy::type_complexity on `Arc<RwLock<HashMap<(Uuid, String), ContractEntry<S>>>>`.
- `sqlx` promoted from dev-dependency to production dependency since `management.rs` uses `sqlx::PgPool` in production code.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed clippy::if_same_then_else in cors_layer()**
- **Found during:** Task 7 (quality gates)
- **Issue:** `if origins.any("*") { AllowOrigin::any() } else if origins.is_empty() { AllowOrigin::any() }` — identical branches
- **Fix:** Collapsed to single condition: `if any("*") || is_empty()`
- **Files modified:** `crates/platform-serve/src/gateway.rs`
- **Verification:** `cargo clippy --workspace -- -D warnings` passes
- **Committed in:** `90392a4`

**2. [Rule 1 - Bug] Fixed Plan 10-03 clippy errors in provisioning.rs**
- **Found during:** Task 7 (quality gates) — Plan 10-03 ran in parallel and left clippy issues
- **Issue:** `MigrationPolicy::from_str` triggers `clippy::should_implement_trait`; manual `Default` impl triggers `clippy::derivable_impls`
- **Fix:** Renamed `from_str` to `parse`; added `#[derive(Default)]` + `#[default]` to `InPlace` variant
- **Files modified:** `crates/platform-serve/src/provisioning.rs`
- **Verification:** `cargo clippy --workspace -- -D warnings` clean
- **Committed in:** `90392a4` (merged with 10-03's provisioning.rs which already included the rename in tests)

**3. [Rule 3 - Blocking] Promoted sqlx from dev-dependency to production dependency**
- **Found during:** Task 7 (quality gates) — Plan 10-03's management.rs uses `sqlx::PgPool` in production code
- **Issue:** `cargo check --workspace` fails with `use of unresolved module sqlx`
- **Fix:** Added `sqlx` to `[dependencies]` (kept in `[dev-dependencies]` too)
- **Files modified:** `crates/platform-serve/Cargo.toml`
- **Verification:** `cargo check --workspace` passes
- **Committed in:** `90392a4`

**4. [Rule 1 - Bug] Fixed test_rate_limit_returns_429 — wrong category pre-drained**
- **Found during:** Task 6 (gateway integration tests)
- **Issue:** Test pre-drained `RateLimitCategory::Execution` tokens but sent request to `/contract/evaluate` (classified as `Evaluation`). Free tier has 100 eval tokens, not 10.
- **Fix:** Rewrote test to inject `AuthContext` via a combined closure that sets extensions before calling `rate_limit_middleware`, and uses `/contract/flows/main/execute` path for Execution category
- **Files modified:** `crates/platform-serve/src/gateway.rs`
- **Verification:** `cargo test gateway::tests::test_rate_limit_returns_429` passes
- **Committed in:** `be3e441`

---

**Total deviations:** 4 auto-fixed (2 Rule 1 bugs, 1 Rule 1 plan-10-03 cleanup, 1 Rule 3 blocking)
**Impact on plan:** All auto-fixes necessary for compilability and correctness. No scope creep.

## Issues Encountered

- Plan 10-03 ran in parallel and committed significant changes to `handlers.rs`, `lib.rs`, `management.rs`, `routes.rs`, and `provisioning.rs`. Some of 10-03's changes left clippy errors. All were fixed as part of Task 7.
- axum middleware ordering with `route_layer` vs `layer` required investigation — `route_layer` middleware doesn't receive extensions set by `layer` middleware in the same `.layer()` call because `route_layer` sits between handler and `layer`. Resolved by using a combined closure that injects AuthContext and calls the middleware function directly.

## Next Phase Readiness

- Gateway layer operational: token bucket rate limiting, CORS, request logging, X-Request-Id
- `AppState` now carries `rate_limiter` and `cors_config` for all request handlers
- Ready for Phase 10 Plan 5 (webhook/event streaming or other hosted platform features)

---
*Phase: 10-hosted-platform*
*Completed: 2026-02-27*
