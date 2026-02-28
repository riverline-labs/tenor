---
phase: 10-hosted-platform
plan: "01"
subsystem: database
tags: [postgres, sqlx, multi-tenancy, rls, bcrypt, uuid, migrations]

# Dependency graph
requires:
  - phase: 05-source-declarations
    provides: enriched_fact_provenance table and PostgresStorage foundation

provides:
  - Multi-tenant Organization/ApiKey/Deployment/PersonaMapping data model
  - org_id column on all existing tables (entity_states, flow_executions, etc.)
  - Row Level Security policies on all tenant-scoped tables
  - TenantStore with full CRUD for tenant management
  - PostgresStorage tenant scoping via with_tenant()/for_org() constructors
  - Default organization (nil UUID) for backward-compat operation
  - Database migration 20260301000001_multi_tenancy.sql

affects:
  - 10-02 (API key auth middleware uses TenantStore)
  - 10-03 (deployment management uses TenantStore + Deployment CRUD)
  - 10-04 (hosted execution uses PostgresStorage::with_tenant())

# Tech tracking
tech-stack:
  added:
    - bcrypt 0.15 (API key hashing)
    - serde 1 with derive feature (tenant model serialization)
    - uuid with v4+serde features (Uuid::new_v4() for IDs, serde for JSON)
    - time with serde feature (rfc3339 serialization for timestamps)
  patterns:
    - TenantStore as dedicated control-plane struct (separate from TenorStorage data plane)
    - PostgresStorage carries TenantContext; all queries bind org_id
    - PgSnapshot carries org_id; begin_snapshot sets SET LOCAL app.current_org_id for RLS
    - Default org (nil UUID) as backward-compat sentinel for all pre-existing data
    - unique_org_name() with UUID suffix for test isolation across parallel runs

key-files:
  created:
    - crates/storage-postgres/migrations/20260301000001_multi_tenancy.sql
    - crates/storage-postgres/src/tenant.rs
    - crates/storage-postgres/tests/tenant_tests.rs
  modified:
    - crates/storage-postgres/src/storage.rs
    - crates/storage-postgres/src/lib.rs
    - crates/storage-postgres/Cargo.toml
    - crates/platform/tests/migration.rs

key-decisions:
  - "TenantStore is a separate struct from PostgresStorage: control-plane vs data-plane separation"
  - "org_id stored in PostgresStorage struct (not passed per-call) to preserve TenorStorage trait signatures"
  - "PgSnapshot carries org_id and sets SET LOCAL app.current_org_id on begin — RLS-ready without requiring superuser bypass"
  - "Nil UUID (00000000-0000-0000-0000-000000000000) as default org_id — all legacy rows use it, PostgresStorage::new() defaults to it"
  - "DEFAULT dropped from org_id columns after migration backfill — explicit org_id required for all new inserts"
  - "bcrypt 0.15 for API key hashing — DEFAULT_COST (12 rounds), generate_plaintext uses tk_ prefix + UUID v4 simple"
  - "Permissions struct backed by JSONB Value with accessor methods — avoids a separate junction table"
  - "current_setting('app.current_org_id', true) with true = missing_ok — RLS policy does not error when setting is absent (superuser connections bypass anyway)"

patterns-established:
  - "API key format: tk_<uuid_v4_simple> (35 chars, no hyphens, URL-safe)"
  - "Tenant isolation tests use unique_org_name(prefix) with UUID suffix to avoid cross-run collisions"
  - "Test cleanup pattern: DELETE FROM organizations WHERE id = $1 AND name != '_default' (cascade deletes children)"
  - "Migration test seeds use DEFAULT_ORG_ID constant (nil UUID) for direct SQL inserts"

requirements-completed:
  - Organization, ApiKey, Deployment tenant model
  - Row-level security with org_id on all existing tables
  - Database migrations adding org_id and RLS policies
  - Default organization for backward compatibility
  - All existing tests pass unchanged

# Metrics
duration: 13min
completed: 2026-02-27
---

# Phase 10 Plan 01: Multi-Tenancy Data Model Summary

**PostgreSQL multi-tenant schema with Organization/ApiKey/Deployment tables, org_id on all existing tables, bcrypt-hashed API keys, RLS policies, and org-scoped PostgresStorage queries via TenantContext**

## Performance

- **Duration:** 13 min
- **Started:** 2026-02-27T23:57:44Z
- **Completed:** 2026-02-27T00:11:14Z
- **Tasks:** 6
- **Files modified:** 7

## Accomplishments
- Created `organizations`, `api_keys`, `deployments`, `persona_mappings` tables with FK integrity and CASCADE deletes
- Added `org_id UUID NOT NULL` to all 6 existing tables (entity_states, flow_executions, operation_executions, entity_transitions, provenance_records, enriched_fact_provenance)
- Enabled Row Level Security on all 9 tenant-scoped tables with `current_setting('app.current_org_id', true)` policies
- Default organization (nil UUID) inserted at migration time; all existing rows backfilled; DEFAULT dropped after backfill
- TenantStore provides full CRUD for Organization, ApiKey (bcrypt), Deployment, PersonaMapping
- PostgresStorage carries TenantContext; all queries bind org_id; begin_snapshot runs SET LOCAL for RLS
- 9 tenant isolation integration tests + 6 model unit tests all pass
- All 56+ existing workspace tests continue to pass unchanged

## Task Commits

1. **Task 1: Define tenant model types** - `8e144b5` (feat)
2. **Task 2: Create multi-tenancy database migration** - `a55236e` (feat)
3. **Task 3: Add tenant CRUD operations to TenantStore** - `3e41dbc` (feat)
4. **Task 4: Update existing storage queries with org_id scoping** - `f0c1b5a` (feat)
5. **Task 5: Add unit tests for tenant model and isolation** - `4017961` (test)
6. **Task 6: Ensure existing workspace tests pass** - `72047cc` (feat)

## Files Created/Modified
- `crates/storage-postgres/migrations/20260301000001_multi_tenancy.sql` - Full schema migration with enums, new tables, org_id columns, indexes, RLS
- `crates/storage-postgres/src/tenant.rs` - PlanTier, DeploymentStatus, Organization, ApiKey, Permissions, Deployment, PersonaMapping types + TenantStore CRUD
- `crates/storage-postgres/src/storage.rs` - TenantContext struct, PostgresStorage carries tenant, all queries org_id-scoped
- `crates/storage-postgres/src/lib.rs` - Exports TenantContext, TenantStore, and all tenant model types
- `crates/storage-postgres/Cargo.toml` - Added bcrypt, serde, uuid v4+serde, time serde features
- `crates/storage-postgres/tests/tenant_tests.rs` - 9 integration tests covering CRUD, isolation, backward-compat
- `crates/platform/tests/migration.rs` - Updated seed_v1_state to include org_id in direct SQL inserts

## Decisions Made
- TenantStore is a separate struct from PostgresStorage: control-plane (management) vs data-plane (execution) separation
- org_id stored in PostgresStorage struct to preserve TenorStorage trait signatures (no per-call threading)
- PgSnapshot carries org_id; begin_snapshot sets `SET LOCAL app.current_org_id` for RLS activation
- Nil UUID as default org — all pre-existing data uses it; PostgresStorage::new() defaults to it
- `current_setting('app.current_org_id', true)` with `missing_ok=true` — RLS policy non-fatal when setting absent (superuser bypasses RLS anyway)
- bcrypt DEFAULT_COST for key hashing; generate_plaintext produces `tk_<uuid_v4_simple>` format

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added serde, bcrypt, uuid v4+serde, time serde features to Cargo.toml**
- **Found during:** Task 1 (Define tenant model types)
- **Issue:** tenant.rs uses serde derives, bcrypt, time::serde::rfc3339, and Uuid::new_v4() — all missing from Cargo.toml
- **Fix:** Added `serde = {version="1", features=["derive"]}`, `bcrypt = "0.15"`, updated time and uuid feature flags
- **Files modified:** crates/storage-postgres/Cargo.toml
- **Verification:** cargo check -p tenor-storage-postgres succeeds
- **Committed in:** `8e144b5` (Task 1 commit)

**2. [Rule 1 - Bug] Fixed migration test seed_v1_state inserting entity_states without org_id**
- **Found during:** Task 6 (Ensure existing workspace tests pass)
- **Issue:** After adding NOT NULL org_id column, existing test seed code did raw SQL INSERT without org_id — violates NOT NULL constraint
- **Fix:** Added `org_id` and `$4::uuid` bind to both entity_states and flow_executions INSERT in seed_v1_state
- **Files modified:** crates/platform/tests/migration.rs
- **Verification:** All 4 migration tests pass (test_force_migrate_success, test_missing_state_mapping_fails_atomically, test_force_migrate_blocks_without_force, test_breaking_changes_require_policy)
- **Committed in:** `72047cc` (Task 6 commit)

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** Both auto-fixes essential for correctness. No scope creep.

## Issues Encountered
- sqlx CLI not installed on dev machine — installed with `cargo install sqlx-cli` to run migration
- Integration tests used fixed org names causing "already exists" errors on re-runs — fixed with UUID-suffixed unique_org_name() helper

## User Setup Required
None - migration applies automatically via sqlx MIGRATOR on server startup. The `organizations` table with the default org is created by the migration.

## Next Phase Readiness
- Organization, ApiKey, Deployment, PersonaMapping tables and CRUD are ready
- org_id scoping in PostgresStorage ready for Phase 10-02 (API key auth middleware)
- TenantStore ready for Phase 10-03 (deployment management endpoints)
- PostgresStorage::with_tenant() ready for Phase 10-04 (hosted execution scoped to org)
- Blocker: None — all foundations in place

## Self-Check: PASSED

All key files verified:
- 20260301000001_multi_tenancy.sql: FOUND
- tenant.rs: FOUND
- storage.rs: FOUND
- tenant_tests.rs: FOUND
- 10-01-SUMMARY.md: FOUND

All commits verified (8e144b5, a55236e, 3e41dbc, f0c1b5a, 4017961, 72047cc): ALL FOUND

---
*Phase: 10-hosted-platform*
*Completed: 2026-02-27*
