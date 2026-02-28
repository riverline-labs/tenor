---
phase: 11-marketplace
plan: "03"
subsystem: api
tags: [registry, marketplace, postgres, axum, multipart, full-text-search, archive-store]

requires:
  - phase: 11-02
    provides: RegistryClient, publish/search/install CLI commands

provides:
  - Registry API with 7 endpoints at /api/v1/registry/templates
  - PostgreSQL storage for templates, versions, ratings with full-text search
  - LocalArchiveStore for template archive persistence
  - Optional-auth pattern for public registry GET routes
  - 10 integration tests covering full publish-search-download-rate lifecycle

affects: [11-04, 11-05]

tech-stack:
  added:
    - axum multipart feature (for publish endpoint)
    - async-trait in platform-serve (for ArchiveStore dyn compatibility)
    - tempfile in tests (already present as dev-dep)
  patterns:
    - RegistryStorage trait (storage-postgres) consumed by platform-serve handlers via Arc<dyn RegistryStorage>
    - Optional-auth: is_public_route() in auth_middleware passes GET /api/v1/registry/* without requiring token; injects AuthContext when valid token voluntarily provided
    - resolve_auth_context() helper extracted from auth_middleware for reuse
    - LocalArchiveStore stores archives as {base_dir}/{template_name}/{version}.tar.gz
    - Download counter incremented via tokio::spawn (non-blocking, fire-and-forget)

key-files:
  created:
    - ~/src/riverline/tenor-platform/crates/platform-serve/src/registry/models.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/src/registry/handlers.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/src/registry/store.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/src/registry/mod.rs
    - ~/src/riverline/tenor-platform/crates/storage-postgres/src/registry.rs
    - ~/src/riverline/tenor-platform/crates/storage-postgres/migrations/20260228000001_registry_tables.sql
    - ~/src/riverline/tenor-platform/crates/platform-serve/tests/registry_integration.rs
  modified:
    - ~/src/riverline/tenor-platform/crates/platform-serve/src/lib.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/src/routes.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/src/auth.rs
    - ~/src/riverline/tenor-platform/crates/storage-postgres/src/lib.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/Cargo.toml
    - ~/src/riverline/tenor-platform/crates/platform/src/commands/serve.rs

key-decisions:
  - "Registry routes mounted inside authenticated router — auth middleware passes GET /api/v1/registry/* without requiring token (is_public_route), injects AuthContext when valid token provided"
  - "StorageError::Backend used (not StorageError::Other) — matches actual tenor_storage error variant"
  - "delete_version soft-deletes (sets status=withdrawn) rather than hard-deleting from DB"
  - "update_latest_version picks most recent approved-or-pending version by published_at DESC"
  - "Download counter incremented via tokio::spawn (non-blocking); integration tests verify via storage directly to avoid timing dependency"
  - "SQL builder pattern for search: build_search_sql() generates dynamic WHERE clause with correct $N parameter indices based on which filters are active"
  - "registry_archive_dir defaults to ./data/registry/archives in ServerConfig; None disables registry (returns 503)"

requirements-completed: [REG-API-01, REG-API-02, REG-STR-01, REG-STR-02, REG-STR-03, SRC-FT-01, QLT-01]

duration: 1201s
completed: 2026-02-28
---

# Phase 11 Plan 03: Registry API and Storage Summary

**PostgreSQL-backed marketplace registry API with 7 endpoints, full-text search, category/tag filtering, archive storage, and rating system — all integrated into the tenor-platform HTTP server**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-02-28T03:20:46Z
- **Completed:** 2026-02-28T03:40:47Z
- **Tasks:** 6 completed
- **Files modified:** 13

## Accomplishments

- 7 REST endpoints at `/api/v1/registry/templates`: publish, list/search, get template, get version, download, unpublish, rate
- PostgreSQL schema with 3 tables (`templates`, `template_versions`, `template_ratings`), GIN full-text search index, category/status indexes
- `RegistryStorage` trait + `PostgresRegistry` implementation with search supporting full-text (`ts_rank`), category/tag filter, sort by downloads/rating/newest, pagination
- `LocalArchiveStore` stores `.tar.gz` archives on local filesystem; `ArchiveStore` trait ready for S3 extension
- Auth pattern: public GET routes (search/download) pass without token; mutating endpoints require auth injected by middleware
- 10 integration tests: all pass against real PostgreSQL

## Task Commits

1. **Task 1: Define registry domain models** - `b4b9001` (feat)
2. **Task 2: Create PostgreSQL migration and storage layer** - `aaa4e8c` (feat)
3. **Task 3: Implement registry API handlers** - `fe024f1` (feat)
4. **Task 4: Mount registry router in platform server** - `31aa026` (feat)
5. **Task 5: Integration tests** - `4095e7b` (feat)
6. **Task 6: Quality gates** - `ac30455` (chore)

## Files Created/Modified

- `crates/platform-serve/src/registry/models.rs` — Template, TemplateVersion, ReviewStatus, SearchQuery, SearchResponse, PublishManifest, RateRequest domain types
- `crates/platform-serve/src/registry/handlers.rs` — 7 Axum handlers: publish_template, list_templates, get_template, get_version, download_template, unpublish_template, rate_template
- `crates/platform-serve/src/registry/store.rs` — ArchiveStore trait, LocalArchiveStore implementation, sha256_hex helper
- `crates/platform-serve/src/registry/mod.rs` — router() function building Axum Router with all 7 routes
- `crates/storage-postgres/src/registry.rs` — RegistryStorage trait, PostgresRegistry implementation with dynamic SQL search builder
- `crates/storage-postgres/migrations/20260228000001_registry_tables.sql` — registry tables + indexes
- `crates/platform-serve/tests/registry_integration.rs` — 10 integration tests
- `crates/platform-serve/src/auth.rs` — is_public_route(), resolve_auth_context() helpers for optional-auth pattern
- `crates/platform-serve/src/routes.rs` — registry nested at /api/v1/registry inside authenticated router
- `crates/platform-serve/src/lib.rs` — registry module declared, ServerConfig.registry_archive_dir added
- `crates/storage-postgres/src/lib.rs` — PostgresRegistry and RegistryStorage re-exported
- `crates/platform-serve/Cargo.toml` — axum multipart feature, async-trait dependency added
- `crates/platform/src/commands/serve.rs` — registry_archive_dir: None for single-tenant serve mode

## Decisions Made

- Auth middleware extended with `is_public_route()` to allow unauthenticated access to registry GET endpoints; `resolve_auth_context()` extracted as a helper for optional-auth injection pattern.
- SQL search query built dynamically in `build_search_sql()` to handle variable WHERE clause length with correct `$N` PostgreSQL parameter indices.
- Download counter uses `tokio::spawn` to avoid blocking the response; integration tests verify via `PostgresRegistry` storage directly (not HTTP) to avoid timing dependency.
- `delete_version` soft-deletes by setting `status = 'withdrawn'` rather than removing from DB.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] StorageError variant mismatch**
- **Found during:** Task 2 (storage layer compilation)
- **Issue:** Used `StorageError::Other(...)` but actual enum variant is `StorageError::Backend(...)`
- **Fix:** Replaced all `StorageError::Other` with `StorageError::Backend`
- **Files modified:** `crates/storage-postgres/src/registry.rs`
- **Verification:** `cargo build -p tenor-storage-postgres` clean
- **Committed in:** `aaa4e8c`

**2. [Rule 3 - Blocking] axum multipart not available without feature flag**
- **Found during:** Task 3 (handlers compilation)
- **Issue:** `axum::extract::Multipart` not importable without `multipart` feature; `async_trait` not a direct dependency
- **Fix:** Added `multipart` feature to axum and `async_trait = "0.1"` to Cargo.toml
- **Files modified:** `crates/platform-serve/Cargo.toml`
- **Verification:** `cargo build -p tenor-platform-serve` clean
- **Committed in:** `fe024f1`

**3. [Rule 1 - Bug] Registry GET routes blocked by auth middleware**
- **Found during:** Task 5 (integration tests — all failing with 401)
- **Issue:** All registry routes inside `authenticated` router were requiring auth token; unauthenticated search/download returned 401
- **Fix:** Added `is_public_route()` to auth middleware to pass GET /api/v1/registry/* without requiring token; `resolve_auth_context()` helper for optional token injection
- **Files modified:** `crates/platform-serve/src/auth.rs`, `crates/platform-serve/src/routes.rs`
- **Verification:** All 10 integration tests pass
- **Committed in:** `4095e7b`

**4. [Rule 1 - Bug] Missing `registry_archive_dir` field in tenor-platform serve.rs**
- **Found during:** Task 6 (workspace build)
- **Issue:** `ServerConfig` gained new field; `tenor-platform/src/commands/serve.rs` didn't set it → compile error
- **Fix:** Added `registry_archive_dir: None` (registry disabled in single-tenant mode)
- **Files modified:** `crates/platform/src/commands/serve.rs`
- **Verification:** `cargo build --workspace` clean
- **Committed in:** `ac30455`

---

**Total deviations:** 4 auto-fixed (2 Rule 1 bugs, 1 Rule 3 blocking, 1 Rule 1 bug)
**Impact on plan:** All fixes necessary for correctness and compilation. No scope creep.

## Issues Encountered

- Download counter test was flaky with 50ms sleep timing; changed to verify download count via `PostgresRegistry` storage directly instead of HTTP to eliminate timing dependency.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Registry API fully functional: all 7 endpoints working with real PostgreSQL
- Integration tests cover the complete publish-search-download-rate lifecycle
- Pending template moderation (approve/reject) requires admin API endpoint (not in this plan scope — can be added to Phase 11 Plan 04 or via a separate admin endpoint)
- Ready for Phase 11 Plan 04 (platform-level registry wiring / hosted mode integration)

---
*Phase: 11-marketplace*
*Completed: 2026-02-28*

## Self-Check: PASSED

All 8 files exist on disk. All 6 task commits verified in git log.
- b4b9001: feat(11-03): define registry domain models and API types
- aaa4e8c: feat(11-03): create PostgreSQL migration and RegistryStorage implementation
- fe024f1: feat(11-03): implement registry API handlers and archive store
- 31aa026: feat(11-03): mount registry router in platform server
- 4095e7b: feat(11-03): add registry integration tests and fix auth for public registry routes
- ac30455: chore(11-03): quality gates - fix formatting and download test reliability
