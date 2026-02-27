# Project State

## Current Position

**Phase**: 4 of 11 — Multi-Instance Entities
**Plan**: 5 of 5 completed in current phase
**Status**: In progress
**Last activity**: 2026-02-27 — Completed plan 04-04 (multi-instance entity integration test suite)

Progress: ████████░░░░░░░░░░░░ 36% (Phases 1-3 complete, plans 04-01 through 04-05 done, 32 plans remaining)

## Decisions

- Phase plans authored by PM-level Claude (spec/vision access, no codebase access)
- Flag code-level discrepancies between plans and actual codebase to user
- Phase 4 Part A (public repo) first; Part B (private repo) after push
- EntityStateMap uses (entity_id, instance_id) composite key per §6.5
- `_default` instance ID for backward compat
- WASM auto-detects old vs new format
- Use DEFAULT_INSTANCE_ID = "_default" for all single-instance backward-compat paths
- Public API (WASM, HTTP serve) accepts flat entity_id->state JSON and converts via single_instance()
- single_instance() and get_instance_state() re-exported from crates/eval/src/lib.rs
- No function signature changes in plan 04-01 (reserved for plan 04-02)
- InstanceBindingMap is BTreeMap<String, String> (entity_id to instance_id)
- Empty InstanceBindingMap falls back to DEFAULT_INSTANCE_ID for full backward compat
- EffectRecord gains instance_id field per §9.5 provenance requirements
- evaluate_flow() public API gains instance_bindings parameter (empty map = backward compat)
- Action.instance_bindings is BTreeMap<String, BTreeSet<String>> (entity_id to set of valid instance_ids) per §15.6
- OperationProvenance.state_before/state_after use BTreeMap<(String,String),String> tuple keys (internal type, not serialized)
- Two-pass effect loop: validate+capture state_before first, then apply, then capture state_after
- StepRecord.instance_bindings empty for non-operation steps (branch, handoff, escalation)
- parse_entity_states() WASM helper: string value = old flat (-> _default), object value = new nested (-> direct parse)
- simulate_flow_with_bindings() is new 6-arg WASM export; simulate_flow() kept as 5-arg backward-compat wrapper
- missing_instance_binding at flow level: on_failure terminates with failure outcome (Ok result), not Rust Err; direct execute_operation returns OperationError::EntityNotFound

## Blockers/Concerns

- Part B (private repo) depends on Part A being pushed to main first
- WASM crate excluded from workspace — needs separate build/test

## Performance Metrics

| Phase | Plan | Duration (s) | Tasks | Files |
|-------|------|-------------|-------|-------|
| 04 | 01 | 740 | 2 | 7 |
| 04 | 02 | 633 | 2 | 11 |
| 04 | 03 | 536 | 2 | 4 |
| 04 | 04 | 280 | 2 | 1 |
| 04 | 05 | 248 | 2 | 2 |

## Session Continuity

Last session: 2026-02-27
Stopped at: Completed plan 04-04 (multi-instance entity integration test suite)
Next action: Push phase 4 to main; then execute Part B in private repo (plan 04-05 already complete)
