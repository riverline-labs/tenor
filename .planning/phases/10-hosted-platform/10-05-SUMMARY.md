---
phase: 10-hosted-platform
plan: "05"
subsystem: platform-serve / storage-postgres
tags: [metering, billing, plan-enforcement, usage-tracking, postgres]
dependency_graph:
  requires: [10-03, 10-04]
  provides: [usage-metering, plan-enforcement, billing-export]
  affects: [gateway-middleware, appstate, management-api]
tech_stack:
  added:
    - time = "0.3" (in platform-serve Cargo.toml)
    - usage_records and plan_limits SQL tables
  patterns:
    - Upsert counters with INSERT ... ON CONFLICT ... DO UPDATE SET col = col + N
    - GREATEST() for monotonic peak tracking
    - In-memory accumulator with periodic async flush (drain-swap pattern)
    - tokio::sync::Mutex for non-blocking increment on hot path
key_files:
  created:
    - ~/src/riverline/tenor-platform/crates/storage-postgres/migrations/20260301000002_usage_metering.sql
    - ~/src/riverline/tenor-platform/crates/storage-postgres/src/metering.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/src/metering.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/src/billing.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/tests/metering.rs
  modified:
    - ~/src/riverline/tenor-platform/crates/storage-postgres/src/lib.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/src/lib.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/src/gateway.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/src/middleware.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/src/management.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/src/routes.rs
    - ~/src/riverline/tenor-platform/crates/platform-serve/Cargo.toml
decisions:
  - "UsageMeter uses drain-swap pattern: swap HashMap under Mutex to drain, then write to DB outside lock — minimizes lock contention"
  - "UsageMeter::increment is non-blocking (tokio Mutex, no DB call on hot path)"
  - "plan_enforcement_middleware sits inside rate_limit (plan enforcement before rate-limit consumption)"
  - "Only 2xx responses increment usage counters — failed requests not counted"
  - "enforce_plan_limits allows on DB error (fail-open) to avoid false 402s on transient failures"
  - "BillingExportAccum::new() removed in favor of #[derive(Default)] + or_default()"
  - "rate_limit_app helper in gateway tests uses #[allow(dead_code)] (retained for reference)"
metrics:
  duration_secs: 980
  tasks_completed: 7
  files_created: 5
  files_modified: 7
  completed_date: "2026-02-28"
---

# Phase 10 Plan 05: Usage Metering and Billing Summary

Daily per-org usage metering with Free-tier hard limits (402), Pro-tier soft limits (log + allow), and Enterprise unlimited. Billing export endpoint produces JSON usage summaries for external billing systems.

## What Was Built

### Task 1: Database Migration (5315d18)

`20260301000002_usage_metering.sql` adds:
- `usage_records` table: daily per-org counters (evaluations, flow_executions, simulations, entity_instances, storage_bytes), UNIQUE(org_id, period) for upsert semantics, RLS enabled with missing_ok=true
- `plan_limits` table: Free/Pro/Enterprise defaults (evaluations, executions, simulations, entity instances, storage bytes)

### Task 2: Storage Layer (f6ce87f)

`MeteringStore` in `storage-postgres/src/metering.rs`:
- `increment_usage(org_id, UsageCategory, count)` — atomic upsert increment
- `update_peak_entities(org_id, count)` — GREATEST() monotonic
- `update_storage_bytes(org_id, bytes)` — point-in-time snapshot
- `get_current_usage(org_id)` — returns zero record if none exists today
- `get_usage_range` and `get_all_usage_range` for billing export
- `get_plan_limits(plan)` with fallback to hard-coded defaults
- `UsageCategory` enum (Evaluation, FlowExecution, Simulation)
- `PlanLimits` struct with hard-coded defaults per tier

### Task 3: In-Memory Usage Meter (d62abb7)

`UsageMeter` in `platform-serve/src/metering.rs`:
- `increment(org_id, UsageCategory)` — async, non-blocking, O(1) in-memory
- `flush_all()` — drain-swap: atomically exchanges HashMap with empty map, then writes to MeteringStore outside the lock
- `start_flush_task()` — spawns background tokio task on configurable interval (10s default)
- `get_current()` — combines DB state + pending in-memory counts for plan enforcement
- `UsageSnapshot` struct returned by `get_current`

### Task 4: Plan Enforcement + Billing Export (4c1ff8d)

`billing.rs`:
- `enforce_plan_limits(meter, org_id, plan, category)` — Free=hard 402, Pro=log+allow, Enterprise=allow
- 402 body: `{"error":"plan_limit_exceeded","plan":"free","limit":"evaluations","current":N,"max":M,"upgrade_url":"/api/v1/organizations/{id}/upgrade"}`
- `BillingExport` struct with totals per period
- `generate_billing_export(storage, from, to, org_names)` — aggregates by org
- `GET /api/v1/billing/export?from=YYYY-MM-DD&to=YYYY-MM-DD` (admin only, in management.rs)
- Route registered in both `build_router_with_management` branches in routes.rs

### Task 5: Gateway Wiring (cb9a204)

- `plan_enforcement_middleware` in gateway.rs: classify_usage → check plan limits → run handler → increment on 2xx
- `classify_usage()` mirrors `classify_request()` but returns `UsageCategory`
- `AppState` gains `usage_meter: Option<Arc<UsageMeter>>`
- `serve()` creates MeteringStore + UsageMeter from TenantStore pool, starts flush task
- `apply_middleware()` accepts `Option<Arc<UsageMeter>>` and wires `plan_enforcement_middleware` as route_layer

### Task 6: Integration Tests (b5a9cb1)

11 DB-backed integration tests in `tests/metering.rs`:
- Storage layer (4): increment creates record, increments accumulate, date range query, peak uses GREATEST
- Metering flush (2): flush writes to DB, second flush does not double-count
- Billing enforcement (4): Free within/over limit (402), Pro over limit (allow), Enterprise unlimited
- Billing export (1): all metrics aggregated correctly

### Task 7: Quality Gates (967a5df)

Fixed clippy warnings found by `--tests` flag:
- `billing.rs`: `or_insert_with(BillingExportAccum::new)` → `or_default()`
- `billing.rs`: `match` → `matches!` macro
- `gateway.rs`: removed unused `RateLimits` import; added `#[allow(dead_code)]` on test helper

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Dep] Added `time = "0.3"` to platform-serve Cargo.toml**
- **Found during:** Task 4 implementation
- **Issue:** `billing.rs` uses `time::Date` for billing period dates; platform-serve had no direct `time` dependency (only transitively via storage-postgres)
- **Fix:** Added `time = { version = "0.3", features = ["serde"] }` to `[dependencies]`
- **Files modified:** `crates/platform-serve/Cargo.toml`

**2. [Rule 1 - Design Simplification] Used `tokio::sync::Mutex<HashMap>` instead of `HashMap<..., AtomicI64>`**
- **Found during:** Task 3 design
- **Issue:** The plan suggested `AtomicI64` values in a `HashMap`, but `AtomicI64` requires a stable memory address — you can't store it by value and move it during HashMap resizing. Using `Arc<AtomicI64>` per key would add heap allocation per counter.
- **Fix:** `tokio::sync::Mutex<HashMap<(Uuid, UsageCategory), i64>>` — simpler, correct, and the lock is held very briefly (no DB calls under the lock)
- **Files modified:** `crates/platform-serve/src/metering.rs`

**3. [Rule 1 - Bug] `BillingExportAccum::new()` dead code after `or_default()` fix**
- **Found during:** Task 7 clippy
- **Issue:** After changing `or_insert_with(BillingExportAccum::new)` to `or_default()`, the `new()` method became dead code; a separate use in test code still called `::new()`
- **Fix:** Changed test code to `BillingExportAccum::default()`, removed `new()` method
- **Files modified:** `crates/platform-serve/src/billing.rs`

## Test Count

Total workspace tests: 175+ (52 unit + 11 metering integration + 9 provisioning + 14 auth + 17 api + 9 storage tenant + 1 conformance + 4 storage unit + others)

## Self-Check: PASSED
