---
phase: 18-platform-hardening
plan: 08
subsystem: api
tags: [axum, cors, rate-limiting, api-key-auth, input-validation, tower-http, security]

# Dependency graph
requires:
  - phase: 18-06
    provides: "Production-grade async HTTP server with axum + tokio"
provides:
  - "Input validation on elaborate endpoint (size, encoding, imports, filename)"
  - "CORS headers on all HTTP responses via tower-http"
  - "Per-IP rate limiting (60 req/min default, configurable)"
  - "Optional API key authentication via TENOR_API_KEY env var"
affects: [22-hosted-evaluator-service]

# Tech tracking
tech-stack:
  added: [tower-http 0.6]
  patterns: [axum middleware layers, per-IP rate limiting, optional env-var auth]

key-files:
  created: []
  modified: [Cargo.toml, crates/cli/Cargo.toml, crates/cli/src/serve.rs, crates/cli/tests/serve_integration.rs]

key-decisions:
  - "Used tower-http CorsLayer with permissive Any origin for local dev -- Phase 22 will tighten for production"
  - "Per-IP rate limiter uses in-memory HashMap with tokio Mutex -- simple and sufficient for single-instance deployment"
  - "API key auth checks both Authorization: Bearer and X-API-Key headers for flexibility"
  - "/health endpoint exempt from auth to support load balancer health checks"
  - "Input validation happens before temp file creation to prevent disk I/O on malicious input"

patterns-established:
  - "Axum middleware pattern: from_fn_with_state for auth and rate limiting layers"
  - "ConnectInfo<SocketAddr> extractor for per-IP tracking in middleware"
  - "Environment variable configuration pattern: TENOR_API_KEY, TENOR_RATE_LIMIT"

requirements-completed: [HARD-10, HARD-11]

# Metrics
duration: 5min
completed: 2026-02-23
---

# Phase 18 Plan 08: HTTP Security Hardening Summary

**Input validation, CORS headers, per-IP rate limiting, and optional API key auth on the axum HTTP server**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-23T16:15:21Z
- **Completed:** 2026-02-23T16:20:07Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Added 5-layer input validation on elaborate endpoint: content size (1MB), null byte rejection, import injection prevention, filename sanitization, encoding validation
- CORS configured via tower-http CorsLayer with permissive origins for local development
- Per-IP rate limiting with 60 req/min default, configurable via TENOR_RATE_LIMIT env var
- Optional API key auth via TENOR_API_KEY env var with /health exempt from auth
- 4 new integration tests covering security validation (oversized source, path traversal, import escape, null bytes)

## Task Commits

Each task was committed atomically:

1. **Task 1: Input validation on elaborate endpoint** - `d3453d4` (feat)
2. **Task 2: Add CORS, rate limiting, and optional API key auth** - `92a320d` (feat)

## Files Created/Modified
- `Cargo.toml` - Added tower-http to workspace dependencies
- `Cargo.lock` - Updated lockfile for tower-http
- `crates/cli/Cargo.toml` - Added tower-http dependency
- `crates/cli/src/serve.rs` - Input validation, CORS layer, rate limiter, auth middleware
- `crates/cli/tests/serve_integration.rs` - 4 new security validation integration tests

## Decisions Made
- Used tower-http CorsLayer with permissive `Any` origin for local dev; Phase 22 (Hosted Evaluator Service) will tighten origins for production
- Per-IP rate limiter uses in-memory HashMap with tokio Mutex -- simple, no external dependencies, sufficient for single-instance deployment
- API key auth checks both `Authorization: Bearer` and `X-API-Key` headers for maximum client flexibility
- /health endpoint exempt from authentication to support load balancer health checks without credentials
- All input validation runs before temp file creation to prevent disk I/O on malicious or oversized input

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed clippy manual_strip lint**
- **Found during:** Task 1 (clippy verification)
- **Issue:** `starts_with` + manual slice indexing flagged by clippy::manual_strip
- **Fix:** Replaced with `strip_prefix` as recommended by clippy
- **Files modified:** crates/cli/src/serve.rs
- **Verification:** `cargo clippy --workspace -- -D warnings` clean
- **Committed in:** d3453d4

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Trivial clippy style fix. No scope creep.

## Issues Encountered
None

## User Setup Required

None - no external service configuration required. Security features are opt-in via environment variables:
- `TENOR_API_KEY` -- set to enable API key authentication (omit for no auth)
- `TENOR_RATE_LIMIT` -- set to override 60 req/min default rate limit

## Next Phase Readiness
- HTTP server now hardened with input validation, CORS, rate limiting, and optional auth
- Ready for Hosted Evaluator Service (Phase 22) which will tighten CORS origins and add production auth
- All 15 serve integration tests pass (11 existing + 4 new security tests)

---
*Phase: 18-platform-hardening*
*Completed: 2026-02-23*
