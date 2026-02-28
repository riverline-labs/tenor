---
phase: 10-hosted-platform
plan: "02"
subsystem: auth
tags: [axum, sha256, bearer-token, api-keys, persona-mapping, multi-tenant, management-api]

# Dependency graph
requires:
  - phase: 10-01
    provides: TenantStore trait with Organization, ApiKey, Deployment, PersonaMapping CRUD, RLS schema

provides:
  - Bearer token authentication middleware via SHA-256 hash lookup
  - AuthContext struct injected into all authenticated request extensions
  - Persona-to-identity mapping enforcement on execute_flow and simulate_flow
  - Management API at /api/v1/organizations/... for org, key, deployment, and mapping CRUD
  - Backward-compatible single-tenant mode (tenant_store = None skips auth)

affects: [10-03, 10-04, 10-05]

# Tech tracking
tech-stack:
  added:
    - sha2 = "0.10" (storage-postgres and platform-serve)
    - constant_time_eq = "0.3" (storage-postgres, for constant-time key comparison)
  patterns:
    - Bearer token auth: SHA-256 hash stored in DB for O(1) lookup, constant-time comparison
    - AuthContext injected via axum middleware::from_fn_with_state into request extensions
    - Handlers extract Option<Extension<AuthContext>> for backward-compatible legacy mode
    - Management routes only added to router when tenant_store is Some (feature-flag pattern)
    - fire-and-forget last_used_at update via tokio::spawn

key-files:
  created:
    - crates/platform-serve/src/auth.rs (AuthContext, auth_middleware, check_persona_authorization, require_permission)
    - crates/platform-serve/src/management.rs (org/key/deployment/mapping CRUD handlers)
    - crates/platform-serve/tests/auth_management.rs (14 integration tests)
  modified:
    - crates/platform-serve/src/lib.rs (AppState.tenant_storage, AppState::new_with_tenant, serve() signature)
    - crates/platform-serve/src/routes.rs (public/authenticated split, management routes, auth middleware layer)
    - crates/platform-serve/src/handlers.rs (execute_flow/simulate_flow persona enforcement)
    - crates/platform-serve/src/error.rs (ApiError::Http variant)
    - crates/storage-postgres/src/tenant.rs (SHA-256 hash_key instead of bcrypt)
    - crates/storage-postgres/Cargo.toml (sha2, constant_time_eq added; bcrypt removed)
    - crates/platform-serve/Cargo.toml (tenor-storage-postgres dependency added)
    - crates/platform/src/commands/serve.rs (pass None for legacy single-tenant mode)

key-decisions:
  - "SHA-256 (not bcrypt) for API key hash_key: bcrypt is non-deterministic, making WHERE key_hash = $1 lookup impossible; plaintext has 128+ bits entropy so SHA-256 is sufficient"
  - "Backward-compat pattern: tenant_store: Option<Arc<TenantStore>> in serve() and AppState; None = legacy mode, no auth middleware applied"
  - "Auth middleware uses route_layer not layer so /health public route is cleanly excluded"
  - "Management routes only wired when tenant_store is Some — avoids dead endpoints in single-tenant deployments"
  - "AuthContext extracted as Option<Extension<AuthContext>> in handlers — handlers run in both authenticated and unauthenticated modes"
  - "fire-and-forget last_used_at: tokio::spawn with cloned Arc<TenantStore> — auth latency not blocked by DB update"
  - "ApiError::Http variant added for auth/persona errors — maps (StatusCode, String) from middleware to HTTP response"

patterns-established:
  - "Route splitting pattern: public Router merged with authenticated Router wrapped in route_layer"
  - "State injection for management handlers: State<Arc<TenantStore>> separate from State<AppState<S>>"
  - "Extension extraction for cross-cutting auth context: Option<Extension<T>> for optional middleware data"

requirements-completed:
  - "API key authentication via Authorization Bearer header"
  - "Persona-to-identity mapping enforcement on operations"
  - "Management API for organizations, API keys, deployments, persona mappings"
  - "Auth middleware injecting org_id into request context"

# Metrics
duration: 13min
completed: 2026-02-28
---

# Phase 10 Plan 02: Auth and Management API Summary

**Bearer token auth with SHA-256 hash lookup, AuthContext middleware, persona enforcement, and full management API for organizations/keys/deployments/mappings**

## Performance

- **Duration:** 13 min
- **Started:** 2026-02-28T00:34:56Z
- **Completed:** 2026-02-28T00:47:29Z
- **Tasks:** 7
- **Files modified:** 13

## Accomplishments

- API key authentication middleware: `Authorization: Bearer tk_<key>` extracted, SHA-256 hashed, looked up in DB, AuthContext injected
- Persona-to-identity enforcement: `execute_flow` and `simulate_flow` check that the calling key is mapped to the requested persona (admin bypass)
- Full management API at `/api/v1/organizations/...`: create/get/update org, create/list/revoke API keys, create/list/get/archive deployments, create/list/delete persona mappings
- Backward-compatible: single-tenant mode passes `None` for tenant_store — no auth middleware applied, no management routes registered
- 14 integration tests: auth middleware (missing header, invalid key, valid key, health skip), persona checks (mapped, unmapped, admin bypass), management CRUD, cross-org access denial

## Task Commits

1. **Tasks 1+2: AuthContext, auth middleware, persona authorization** - `a69d469` (feat)
2. **Task 3: Management API handlers** - `b540798` (feat)
3. **Task 4: Wire auth middleware and management routes** - `0b94332` (feat)
4. **Task 5: Update handlers for AuthContext** - `a9b2fd9` (feat)
5. **Task 6: Auth and management tests** - `fb8284a` (test)
6. **Task 7: Quality gates (clippy fix)** - `e0d1a77` (chore)

## Files Created/Modified

- `crates/platform-serve/src/auth.rs` - AuthContext struct, auth_middleware (axum middleware::from_fn_with_state), require_permission, check_persona_authorization, sha256_hex helper
- `crates/platform-serve/src/management.rs` - Full management API: CreateOrgRequest/UpdateOrgRequest, CreateApiKeyRequest/PermissionsInput, CreatePersonaMappingRequest, all CRUD handlers
- `crates/platform-serve/tests/auth_management.rs` - 14 integration tests covering auth and management
- `crates/platform-serve/src/lib.rs` - AppState.tenant_storage field, AppState::new_with_tenant(), serve() takes Option<Arc<TenantStore>>
- `crates/platform-serve/src/routes.rs` - Public/authenticated route split, management routes, auth route_layer
- `crates/platform-serve/src/handlers.rs` - execute_flow/simulate_flow persona enforcement with Option<Extension<AuthContext>>
- `crates/platform-serve/src/error.rs` - ApiError::Http(StatusCode, String) variant
- `crates/storage-postgres/src/tenant.rs` - ApiKey::hash_key changed from bcrypt to SHA-256; ApiKey::verify uses constant_time_eq
- `crates/storage-postgres/Cargo.toml` - sha2, constant_time_eq added; bcrypt removed
- `crates/platform-serve/Cargo.toml` - tenor-storage-postgres dependency added
- `crates/platform/src/commands/serve.rs` - serve() call passes None for backward compat
- `crates/platform-serve/tests/api.rs` - build_router() calls updated to pass None

## Decisions Made

- **SHA-256 over bcrypt for API key hash**: bcrypt is non-deterministic (same input → different hash each call), making `WHERE key_hash = $1` DB lookup impossible. API keys have 128+ bits of entropy (UUID v4 simple format), so SHA-256 is sufficient. Changed `ApiKey::hash_key()` and `ApiKey::verify()` accordingly.
- **Backward-compatible router split**: `build_router()` takes `Option<Arc<TenantStore>>`. When None, no auth middleware and no management routes. When Some, auth middleware applied via `route_layer` and management routes registered.
- **fire-and-forget last_used_at**: `tokio::spawn` with cloned `Arc<TenantStore>`. Auth latency is not blocked by the DB update.
- **ApiError::Http variant**: Added to map `(StatusCode, String)` errors from persona check to HTTP response without changing executor error types.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Changed API key hashing from bcrypt to SHA-256**
- **Found during:** Task 1 (auth middleware implementation)
- **Issue:** `TenantStore::get_api_key_by_hash()` uses `WHERE key_hash = $1`, but bcrypt is non-deterministic — the same plaintext produces a different hash each invocation, making DB lookup by hash impossible. Auth middleware cannot function with bcrypt hashes.
- **Fix:** Changed `ApiKey::hash_key()` to SHA-256 (deterministic, 64-char hex), updated `ApiKey::verify()` to use `constant_time_eq` for constant-time comparison. Removed `bcrypt` dep from storage-postgres, added `sha2` and `constant_time_eq`.
- **Files modified:** `crates/storage-postgres/src/tenant.rs`, `crates/storage-postgres/Cargo.toml`
- **Verification:** `cargo test` — `test_api_key_hash_verify` and `test_api_key_hash_deterministic` pass; all 9 existing tenant tests pass; all 14 new auth tests pass.
- **Committed in:** `a69d469` (Task 1+2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 — bug preventing correct operation)
**Impact on plan:** The bcrypt-to-SHA-256 change was required for correctness. No scope creep.

## Issues Encountered

None beyond the bcrypt lookup issue documented above.

## Next Phase Readiness

- Auth and management API complete — phase 10-03 (deployment provisioning) can build on top
- `AppState::new_with_tenant()` factory ready for multi-tenant server initialization
- All 14 auth/management tests pass; all existing 17 API tests still pass
- Persona mapping enforcement in place — operations are secured per-tenant

---
*Phase: 10-hosted-platform*
*Completed: 2026-02-28*
