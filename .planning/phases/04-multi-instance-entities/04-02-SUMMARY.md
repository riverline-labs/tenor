---
phase: 04-multi-instance-entities
plan: 02
subsystem: eval
tags: [instance-binding, operation-execution, flow-execution, multi-instance, backward-compat]
dependency_graph:
  requires: [EntityStateMap-composite-key, single_instance-helper, DEFAULT_INSTANCE_ID]
  provides: [InstanceBindingMap, execute_operation-instance-targeted, execute_flow-instance-binding, resolve_bindings, EffectRecord-instance-id]
  affects: [crates/eval, crates/cli, crates/tenor-eval-wasm, conformance/eval]
tech_stack:
  added: []
  patterns: [InstanceBindingMap-parameter, resolve_bindings-per-op, sub-flow-inheritance, backward-compat-empty-map]
key_files:
  modified:
    - crates/eval/src/operation.rs
    - crates/eval/src/flow.rs
    - crates/eval/src/lib.rs
    - crates/cli/src/serve.rs
    - crates/cli/src/main.rs
    - crates/cli/src/agent.rs
    - crates/eval/tests/conformance.rs
    - crates/tenor-eval-wasm/src/lib.rs
    - conformance/eval/positive/compensate_handler.verdicts.json
    - conformance/eval/positive/escalate_handler.verdicts.json
    - conformance/eval/positive/flow_error_escalate.verdicts.json
decisions:
  - InstanceBindingMap is BTreeMap<String, String> (entity_id to instance_id) not a struct
  - Empty InstanceBindingMap falls back to DEFAULT_INSTANCE_ID for full backward compat
  - EffectRecord gains instance_id field per §9.5 provenance requirements
  - InvalidEntityState and EntityNotFound errors now include instance_id for clarity
  - handle_failure gets #[allow(clippy::too_many_arguments)] since it's a private internal helper
  - resolve_bindings() delegates to resolve_instance_id() from operation.rs (single source of truth)
  - evaluate_flow() public API gains instance_bindings parameter with empty-map backward compat
metrics:
  duration_seconds: 633
  completed: 2026-02-27
  tasks_completed: 2
  files_modified: 11
---

# Phase 4 Plan 2: Instance-Targeted Operation and Flow Execution Summary

Instance-targeted operation execution via `InstanceBindingMap` (entity_id → instance_id) added to `execute_operation()` and `execute_flow()`, completing the behavioral core of multi-instance entity support per TENOR.md §9.2, §11.1, §11.4.

## What Was Built

`execute_operation()` now accepts an `InstanceBindingMap` that tells it which specific instance of each entity type to target for state transitions. `execute_flow()` accepts the same map and resolves per-operation bindings via `resolve_bindings()`. Sub-flows inherit the parent's bindings. Empty bindings fall back to `DEFAULT_INSTANCE_ID` for full backward compatibility. `EffectRecord` now carries `instance_id` for provenance per §9.5.

## Tasks

### Task 1: Instance-targeted operation execution
**Commit:** `7da3636`

- Added `InstanceBindingMap = BTreeMap<String, String>` type alias to `operation.rs`
- Added `resolve_instance_id()` helper (entity_id -> instance_id, falls back to DEFAULT_INSTANCE_ID)
- Updated `execute_operation()` signature: new `instance_bindings: &InstanceBindingMap` parameter
- Effects now look up instance_id via `resolve_instance_id()` before forming composite key
- `EffectRecord` gains `instance_id: String` field per §9.5 provenance requirements
- `OperationError::InvalidEntityState` and `EntityNotFound` gain `instance_id` field
- Updated all 15 test call sites in `operation.rs` to pass `&InstanceBindingMap::new()`
- Added 5 new multi-instance tests: targeted transition, wrong-state on target, empty-binding fallback, nonexistent instance error, EffectRecord.instance_id assertion
- Updated 3 conformance fixtures for new error message format: `entity 'X' instance '_default' in state ...`

### Task 2: Flow execution with InstanceBindingMap
**Commit:** `d7240c5`

- Added `resolve_bindings(op, bindings) -> InstanceBindingMap` in `flow.rs` per §11.4
- Updated `execute_flow()` signature: new `instance_bindings: &InstanceBindingMap` parameter
- `OperationStep`: calls `resolve_bindings()` per operation, passes resolved bindings to `execute_operation()`
- `SubFlowStep`: sub-flows inherit parent `instance_bindings` per §11.4/§11.5
- `ParallelStep`: each branch inherits parent bindings; merge now uses `EffectRecord.instance_id` for composite key (previously hardcoded `DEFAULT_INSTANCE_ID`)
- `handle_failure()`: compensation steps pass `instance_bindings` to `execute_operation()`
- `evaluate_flow()` in `lib.rs`: gains `instance_bindings` parameter, passes to `flow::execute_flow()`
- Re-exported `InstanceBindingMap` and `resolve_instance_id` from `lib.rs`
- Updated all callers: `cli/src/serve.rs` (2 sites), `cli/src/main.rs`, `cli/src/agent.rs`, `eval/tests/conformance.rs`, `tenor-eval-wasm/src/lib.rs`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Conformance fixtures had old error message format**
- **Found during:** Task 1
- **Issue:** Three conformance fixtures expected `entity 'X' in state 'Y', expected 'Z'` but `InvalidEntityState::Display` now includes instance_id: `entity 'X' instance '_default' in state 'Y', expected 'Z'`
- **Fix:** Updated `compensate_handler.verdicts.json`, `escalate_handler.verdicts.json`, `flow_error_escalate.verdicts.json`
- **Files modified:** `conformance/eval/positive/*.verdicts.json` (3 files)
- **Commit:** `7da3636`

**2. [Rule 2 - Missing critical functionality] Parallel branch merge had hardcoded DEFAULT_INSTANCE_ID**
- **Found during:** Task 2
- **Issue:** Flow.rs parallel step entity state merge inserted `(entity_id, DEFAULT_INSTANCE_ID)` even though Plan 04-01 had noted this as a TODO for Plan 04-02. With `EffectRecord` now carrying `instance_id`, the merge could directly use it.
- **Fix:** Changed parallel merge to use `change.instance_id.clone()` instead of `DEFAULT_INSTANCE_ID`
- **Files modified:** `crates/eval/src/flow.rs`
- **Commit:** `d7240c5`

**3. [Rule 2 - Clippy compliance] handle_failure exceeded 7-argument limit**
- **Found during:** Task 2 (after adding `instance_bindings` parameter)
- **Issue:** Adding `instance_bindings` brought `handle_failure` to 8 parameters, triggering `clippy::too_many_arguments`
- **Fix:** Added `#[allow(clippy::too_many_arguments)]` on the private internal helper
- **Files modified:** `crates/eval/src/flow.rs`
- **Commit:** `d7240c5`

## Verification

- `cargo build --workspace` — clean
- `cargo test --workspace` — all tests pass (680+ tests across all crates)
- `cargo run -p tenor-cli -- test conformance` — 82/82 pass
- `cargo clippy --workspace -- -D warnings` — clean
- `wasm-pack build --target nodejs && wasm-pack test --node` — 21/21 WASM tests pass
- `execute_operation()` accepts `instance_bindings: &InstanceBindingMap`
- `execute_flow()` accepts `instance_bindings: &InstanceBindingMap`
- `resolve_bindings()` correctly maps entity effects to instances
- Missing instance falls back to DEFAULT_INSTANCE_ID (not an error)
- Sub-flows inherit parent bindings
- EffectRecord.instance_id carries the targeted instance

## Self-Check: PASSED

- SUMMARY.md: FOUND at `.planning/phases/04-multi-instance-entities/04-02-SUMMARY.md`
- Commit `7da3636`: Task 1 — instance-targeted execute_operation
- Commit `d7240c5`: Task 2 — execute_flow with InstanceBindingMap
