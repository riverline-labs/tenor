---
phase: 04-multi-instance-entities
plan: 03
subsystem: eval
tags: [action-space, provenance, instance-binding, multi-instance, per-instance-state]
dependency_graph:
  requires: [InstanceBindingMap, EntityStateMap-composite-key, DEFAULT_INSTANCE_ID, EffectRecord-instance-id]
  provides: [Action.instance_bindings, BlockedAction.instance_bindings, OperationProvenance.instance_binding, OperationProvenance.state_before, OperationProvenance.state_after, StepRecord.instance_bindings]
  affects: [crates/eval/src/action_space.rs, crates/eval/src/operation.rs, crates/eval/src/flow.rs, crates/eval/src/policy.rs]
tech_stack:
  added: []
  patterns: [per-instance-action-space, two-pass-effect-loop, state-snapshot-provenance, step-record-instance-tracing]
key_files:
  modified:
    - crates/eval/src/action_space.rs
    - crates/eval/src/operation.rs
    - crates/eval/src/flow.rs
    - crates/eval/src/policy.rs
decisions:
  - Action.instance_bindings is BTreeMap<String, BTreeSet<String>> (entity_id to set of valid instance_ids) per §15.6
  - BlockedAction.instance_bindings same shape, carries blocking instance_ids
  - Action space available if at least one instance per entity is in valid source state
  - OperationProvenance.state_before/state_after use BTreeMap<(String,String),String> tuple keys (not serialized, internal only)
  - Two-pass effect loop: validate+capture state_before first, then apply effects, then capture state_after
  - StepRecord.instance_bindings empty for non-operation steps (branch, handoff, escalation)
  - Compensation step success populates StepRecord.instance_bindings from comp_result.provenance.instance_binding
metrics:
  duration_seconds: 536
  completed: 2026-02-27
  tasks_completed: 2
  files_modified: 4
---

# Phase 4 Plan 3: Per-Instance Action Space and Instance-Scoped Provenance Summary

Per-instance action space computation via `Action.instance_bindings` (entity_id to set of valid instance_ids) and full instance-scoped operation provenance via `OperationProvenance.instance_binding`, `state_before`, `state_after` per TENOR.md §15.6 and §9.5.

## What Was Built

`compute_action_space()` now performs per-instance entity state analysis: for each entity referenced in a flow's entry operation effects, it collects all instances in valid source states into `Action.instance_bindings`. An action is available if at least one instance per entity qualifies. `BlockedAction` gains `instance_bindings` to record which instances were in wrong states.

`OperationProvenance` gains three new fields: `instance_binding` (which specific instance was targeted per entity), `state_before` (pre-effect state snapshot per (entity_id, instance_id)), and `state_after` (post-effect snapshot). `execute_operation()` uses a two-pass loop: first pass validates all targets and captures `state_before`, second pass applies effects and captures `state_after`. `StepRecord` gains `instance_bindings` to trace which instances were targeted at each flow step.

## Tasks

### Task 1: Per-instance action space
**Commit:** `50f9111`

- `Action.instance_bindings: BTreeMap<String, BTreeSet<String>>` added per §15.6
- `BlockedAction.instance_bindings: BTreeMap<String, BTreeSet<String>>` added
- `compute_action_space()` now iterates all (entity_id, instance_id) pairs in EntityStateMap:
  - Collects instances in valid source state into `valid_instance_bindings`
  - Collects blocking instances into `blocking_instance_bindings`
  - Action available if every required entity has at least one valid instance
  - Action blocked only if no instances qualify for a required entity
- Single-instance backward compat: `_default` appears naturally in instance_bindings
- Updated `policy.rs` test helper to include `instance_bindings: BTreeMap::new()`
- 3 new multi-instance tests: partial validity (some instances valid), full block (all wrong state), all instances valid

### Task 2: Instance-scoped provenance
**Commit:** `4950473`

- `OperationProvenance.instance_binding: BTreeMap<String, String>` — entity_id to targeted instance_id per §9.5
- `OperationProvenance.state_before: BTreeMap<(String, String), String>` — per-instance state before effects
- `OperationProvenance.state_after: BTreeMap<(String, String), String>` — per-instance state after effects
- `execute_operation()` two-pass effect loop: first pass validates state + captures `state_before`; second pass applies state transitions + builds `state_after` from updated entity_states
- `StepRecord.instance_bindings: BTreeMap<String, String>` added per §11.4:
  - OperationStep success: from `op_result.provenance.instance_binding`
  - OperationStep failure: from `op_bindings` (the resolved bindings that were attempted)
  - SubFlowStep: parent `instance_bindings` (inherited per §11.4/§11.5)
  - ParallelStep: parent `instance_bindings`
  - BranchStep, HandoffStep, Escalation, Compensation-error: empty map
  - Compensation success: from `comp_result.provenance.instance_binding`
- Updated `provenance_records_all_state_changes` test to assert new fields
- 3 new provenance tests: single-instance backward compat, explicit instance binding, multi-effect multi-entity

## Deviations from Plan

None — plan executed exactly as written.

The plan specified `BTreeMap<(String, String), String>` for `state_before`/`state_after`. Since `OperationProvenance` is an internal Rust type (no `#[derive(Serialize, Deserialize)]`), tuple keys work correctly. This follows the plan specification exactly.

## Verification

- `cargo build --workspace` — clean
- `cargo test --workspace` — all tests pass (363 eval tests, 690+ total workspace)
- `cargo run -p tenor-cli -- test conformance` — 82/82 pass
- `cargo clippy --workspace -- -D warnings` — clean
- `Action.instance_bindings` populated for each flow with per-instance valid sets
- `OperationProvenance.instance_binding` records targeted instances
- `state_before`/`state_after` are per-instance snapshots around effect application
- `StepRecord.instance_bindings` traces which instances were targeted at each step

## Self-Check: PASSED

- SUMMARY.md: will be created at `.planning/phases/04-multi-instance-entities/04-03-SUMMARY.md`
- Commit `50f9111`: Task 1 — per-instance action space with instance_bindings
- Commit `4950473`: Task 2 — instance-scoped provenance in OperationProvenance and StepRecord
