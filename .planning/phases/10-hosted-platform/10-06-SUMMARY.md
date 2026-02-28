---
phase: 10-hosted-platform
plan: "06"
subsystem: ui
tags: [react, vite, tailwind, typescript, axum, tower-http, recharts, react-router, react-query]

# Dependency graph
requires:
  - phase: 10-05
    provides: UsageMeter, MeteringStore, billing export — admin API consumes these

provides:
  - Admin API module: system-health, org list, usage history, usage report, deployment stats endpoints
  - AdminState struct with AdminMetrics sliding-window rate tracking
  - React admin dashboard SPA at /admin/: org list, org detail, deployment detail, system health, usage reports
  - Static file serving via ServeDir at /admin/ with client-side routing fallback
  - admin_dashboard_dir field in ServerConfig (opt-in)

affects:
  - 10-07 (final plan) — admin API and dashboard provide ops visibility into hosted platform

# Tech tracking
tech-stack:
  added:
    - tower-http fs feature (ServeDir, ServeFile)
    - React 19 + Vite 7 + TypeScript (admin-dashboard)
    - tailwindcss @tailwindcss/vite (admin-dashboard)
    - react-router-dom (client-side routing)
    - @tanstack/react-query (server state)
    - recharts (usage line charts)
    - lucide-react (icons)
  patterns:
    - Admin module pattern: admin/mod.rs + admin/api.rs (state) + admin/handlers.rs (routes)
    - AdminMetrics: sliding 1-minute window with Vec<(Instant, bool)>, evicts on record
    - build_router_with_management signature extended: admin_state + admin_dashboard_dir params
    - React dashboard: BrowserRouter basename=/admin, localStorage API key auth guard
    - QueryClient: retry=1, staleTime=30s; system health auto-refetches every 30s

key-files:
  created:
    - crates/platform-serve/src/admin/mod.rs
    - crates/platform-serve/src/admin/api.rs
    - crates/platform-serve/src/admin/handlers.rs
    - crates/platform-serve/tests/admin.rs
    - admin-dashboard/src/api/client.ts
    - admin-dashboard/src/App.tsx
    - admin-dashboard/src/pages/OrganizationList.tsx
    - admin-dashboard/src/pages/OrganizationDetail.tsx
    - admin-dashboard/src/pages/DeploymentDetail.tsx
    - admin-dashboard/src/pages/SystemHealth.tsx
    - admin-dashboard/src/pages/UsageReports.tsx
    - admin-dashboard/src/components/Layout.tsx
    - admin-dashboard/src/components/StatusBadge.tsx
    - admin-dashboard/src/components/DataTable.tsx
    - admin-dashboard/src/components/UsageChart.tsx
    - admin-dashboard/src/components/ConfirmDialog.tsx
    - admin-dashboard/src/components/ApiKeyModal.tsx
    - admin-dashboard/src/components/LoginForm.tsx
  modified:
    - crates/platform-serve/src/lib.rs (admin module, PathBuf import, admin_dashboard_dir in ServerConfig)
    - crates/platform-serve/src/routes.rs (admin routes, ServeDir, new function signature)
    - crates/platform-serve/Cargo.toml (tower-http fs feature)
    - crates/platform-serve/tests/provisioning.rs (updated to 5-arg signature)
    - crates/platform/src/commands/serve.rs (added admin_dashboard_dir: None)

key-decisions:
  - "AdminMetrics uses Vec<(Instant, bool)> with eviction on record_request — simpler than a ring buffer for 1-minute window"
  - "build_router_with_management extended with admin_state + admin_dashboard_dir (both Option) — backward compat preserved"
  - "admin_dashboard_dir: Option<PathBuf> in ServerConfig — None skips /admin/ route (no dead routes in single-tenant mode)"
  - "React BrowserRouter basename=/admin — avoids path prefix issues with client-side routing"
  - "API key stored in localStorage, injected as Bearer token — simple admin-only tool, no refresh needed"
  - "erasableSyntaxOnly: ApiError field declared separately (not parameter property) to satisfy TS compiler"
  - "DataTable generic over T with Column<T>.render — avoids per-page table reimplementation"
  - "SystemHealth auto-refetches every 30s via refetchInterval in useQuery"

patterns-established:
  - "Admin-only endpoint pattern: extract Extension<AuthContext>, check require_permission(auth, 'is_admin'), return 403 on failure"
  - "AdminState.pool used to construct MeteringStore per-handler (stateless, cheap)"

requirements-completed: []

# Metrics
duration: 19min
completed: 2026-02-28
---

# Phase 10 Plan 06: Admin Dashboard Summary

**React admin dashboard SPA at /admin/ with admin-only Rust API endpoints for system health, org management, usage reporting, and deployment monitoring**

## Performance

- **Duration:** 19 min
- **Started:** 2026-02-28T01:58:07Z
- **Completed:** 2026-02-28T02:16:34Z
- **Tasks:** 7
- **Files modified:** ~40

## Accomplishments

- Admin API module with 5 endpoints under `/api/v1/admin/` — all require is_admin API key (403 otherwise)
- AdminMetrics struct with 1-minute sliding window for request/error rate tracking
- React dashboard with 5 pages: OrganizationList, OrganizationDetail (tabs: keys/deployments/mappings/usage), DeploymentDetail, SystemHealth (30s auto-refresh), UsageReports (date range + JSON export)
- Static file serving at /admin/ with client-side routing fallback (ServeDir + ServeFile)
- 8 integration tests covering admin endpoint auth, pagination, date filtering, and metrics

## Task Commits

1. **Task 1: Admin-specific API endpoints** - `d00dbe4` (feat)
2. **Task 2: Wire admin routes and static serving** - `fb11e4a` (feat)
3. **Task 3: Scaffold React dashboard** - `ec91f75` (feat)
4. **Task 4: Build dashboard pages** - `ec91f75` (feat, combined with Task 3)
5. **Task 5: Build shared components** - `ec91f75` (feat, combined with Task 3)
6. **Task 6: Add admin API tests** - `9160954` (test)
7. **Task 7: Quality gates** - `1286322` (chore)

## Files Created/Modified

- `crates/platform-serve/src/admin/mod.rs` — module declaration, AdminState re-export
- `crates/platform-serve/src/admin/api.rs` — AdminState struct, AdminMetrics (1-min sliding window)
- `crates/platform-serve/src/admin/handlers.rs` — 5 admin HTTP handlers + unit tests
- `crates/platform-serve/src/lib.rs` — registered admin module, added admin_dashboard_dir to ServerConfig
- `crates/platform-serve/src/routes.rs` — admin API routes + ServeDir at /admin/
- `crates/platform-serve/Cargo.toml` — tower-http fs feature
- `crates/platform-serve/tests/admin.rs` — 8 integration tests
- `admin-dashboard/` — full React SPA: 5 pages, 7 components, API client, Vite config

## Decisions Made

- AdminMetrics uses `Vec<(Instant, bool)>` with eviction on `record_request()` for simplicity over a true ring buffer
- `build_router_with_management` extended with `admin_state: Option<Arc<AdminState>>` and `admin_dashboard_dir: Option<PathBuf>` — both optional for backward compat
- `admin_dashboard_dir: None` in single-tenant/test paths prevents dead routes
- React `BrowserRouter basename="/admin"` handles path prefix for client-side routing
- API key in localStorage injected as Bearer token — appropriate for an admin-only operator tool
- TypeScript `erasableSyntaxOnly` required rewriting ApiError field declaration (no parameter properties)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated provisioning.rs test callers for new build_router_with_management signature**
- **Found during:** Task 2 (Wire admin routes)
- **Issue:** Changing the function signature from 3 to 5 args broke 2 existing test call sites in provisioning.rs
- **Fix:** Added `None, None` for the new `admin_state` and `admin_dashboard_dir` params
- **Files modified:** `crates/platform-serve/tests/provisioning.rs`
- **Committed in:** `fb11e4a` (Task 2 commit)

**2. [Rule 3 - Blocking] Fixed missing ServerConfig field in platform crate**
- **Found during:** Task 2 build
- **Issue:** `crates/platform/src/commands/serve.rs` initializes `ServerConfig` with struct literal; new `admin_dashboard_dir` field caused compile error
- **Fix:** Added `admin_dashboard_dir: None` to the existing struct literal
- **Files modified:** `crates/platform/src/commands/serve.rs`
- **Committed in:** `fb11e4a` (Task 2 commit)

**3. [Rule 3 - Blocking] Fixed TypeScript erasableSyntaxOnly incompatibility in ApiError**
- **Found during:** Task 3 (npm run build)
- **Issue:** `class ApiError extends Error { constructor(public status: number, ...) }` — parameter property syntax banned by `erasableSyntaxOnly: true`
- **Fix:** Declared `status: number` as a class field, assigned `this.status = status` in constructor body
- **Files modified:** `admin-dashboard/src/api/client.ts`
- **Committed in:** `ec91f75` (Task 3 commit)

---

**Total deviations:** 3 auto-fixed (3 blocking)
**Impact on plan:** All three were minor compile-time blockers discovered during build. No scope change.

## Issues Encountered

- AdminMetrics import missing in handlers.rs test module — caught by `cargo test`, fixed by adding `use crate::admin::api::AdminMetrics;`

## Next Phase Readiness

- Admin dashboard foundation complete — operators can monitor orgs, deployments, and system health
- Dashboard serves at /admin/ when `admin_dashboard_dir` is configured pointing to `admin-dashboard/dist`
- Next plan (10-07, final) can extend admin capabilities or ship final platform polish

---
*Phase: 10-hosted-platform*
*Completed: 2026-02-28*
