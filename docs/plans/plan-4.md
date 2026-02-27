# Phase 4: Multi-Instance Entities — Complete Implementation

The spec amendment for Multi-Instance Entities is in TENOR.md §6.5, with changes throughout §9, §11, §15, §16, §17 (E15-E17), §20 (AL74-AL79), §21 (D.11), and §22. The private repo's PostgresStorage already uses `(entity_id, instance_id)` composite keys. The evaluator does NOT — it uses `BTreeMap<String, String>` keyed by entity_id alone. There is collapsing logic somewhere in the private repo that maps the instance-keyed storage down to single-instance for the evaluator.

This prompt makes the evaluator natively instance-aware, updates the WASM API, removes the collapsing logic, and makes the agent runtime instance-aware. When this is done, multiple instances of the same entity type can exist simultaneously with independent state, the action space reports per-instance availability, and flows target specific instances via InstanceBindingMap.

**Source of truth:** `TENOR.md`. Specifically:

- §6.2 — Entity DAG properties (no dynamic entity _type_ creation; instances are runtime)
- §6.4 — State machine semantics (multiple instances, independent state, InstanceId opaque)
- §6.5 — Instance model (InstanceId, EntityStateMap definition, single-instance degenerate case, instance absence)
- §9.2 — Operation execution (gains InstanceId parameter, instance_binding in effects)
- §9.5 — Operation provenance (instance_binding, per-instance state_before/state_after)
- §11.1 — Flow definition (FlowExecutionContext with InstanceBindingMap)
- §11.4 — Flow evaluation (resolve_bindings from InstanceBindingMap, sub-flow inheritance)
- §15.2 — Flow initiation (executor_provided_instance_bindings)
- §15.3 — Per-evaluation sequence (instance-targeted execute)
- §15.4 — Provenance chain (instance-scoped)
- §15.6 — Action space (instance-keyed, per-instance actions, Action.instance_bindings)
- §17.2 — E15 (instance creation in initial state), E16 (identity stability), E17 (instance enumeration)
- §19.1 — Manifest capabilities (multi_instance_entities)
- §20 — AL74-AL79 (runtime-only multiplicity, no cross-instance preconditions, deferred flow binding, no same-type multi-binding, creation not modeled as Operation, action space scaling)
- §21 D.11 — Multi-instance worked example

Read ALL of these sections before writing any code.

---

## What "done" means

1. `EntityStateMap` in the evaluator is `Map<(EntityId, InstanceId), StateId>` — not `Map<EntityId, StateId>`
2. `compute_action_space` returns per-instance results: which instances are in valid source states for each flow
3. `execute` takes `(EntityId, InstanceId)` pairs for effect targets
4. `execute_flow` takes an `InstanceBindingMap` that maps each entity type to a specific instance
5. Operation provenance records `instance_binding`, per-instance `state_before`/`state_after`
6. Flow provenance records which instances were targeted
7. Single-instance degenerate case works: one instance per entity with `"_default"` InstanceId, existing contracts unchanged
8. WASM evaluator accepts instance-keyed entity states in its JSON API
9. Private repo's collapsing logic is removed — storage passes instance-keyed state directly to evaluator
10. Agent runtime reasons about which instance to act on
11. Manifest advertises `multi_instance_entities: true`

---

## Part A: Public Repo — Evaluator Changes (~/src/riverline/tenor)

### A1: EntityStateMap type change

This is the foundational change. Find where `EntityStateMap` (or equivalent) is defined. It's currently something like:

```rust
type EntityStateMap = BTreeMap<String, String>;  // entity_id → state
```

Change it to:

```rust
type EntityStateMap = BTreeMap<(String, String), String>;  // (entity_id, instance_id) → state
```

Or define a proper struct:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityInstanceKey {
    pub entity_id: String,
    pub instance_id: String,
}

type EntityStateMap = BTreeMap<EntityInstanceKey, String>;
```

Follow whatever pattern the codebase already uses for composite keys. Read the existing code before deciding.

This change will break many call sites. Fix them all. Every place that reads or writes entity state must now use the `(entity_id, instance_id)` pair.

**Single-instance degenerate case (§6.5):** When only one instance exists per entity type, the map has one entry per entity with instance_id `"_default"` (or any non-empty string). Provide a helper:

```rust
impl EntityStateMap {
    /// Create a single-instance state map (backward compat)
    pub fn single_instance(states: BTreeMap<String, String>) -> Self {
        states.into_iter()
            .map(|(entity_id, state)| ((entity_id, "_default".to_string()), state))
            .collect()
    }
}
```

This ensures all existing tests and callers that provide `entity_id → state` maps can trivially convert. Existing contracts work unchanged.

### A2: Operation execution — instance targeting

Update the `execute` function signature per §9.2:

```
execute(op, persona, verdict_set, (entity_id, instance_id), entity_state)
    → (EntityState', OutcomeLabel) | Error
```

The key change: the executor provides which specific instance to target. For Operations with effects spanning multiple entity types, each effect targets a specific `(EntityId, InstanceId)` pair from the flow's instance binding map.

Update E2 (transition source validation) to validate against the targeted instance's current state, not just the entity type.

### A3: Flow execution — InstanceBindingMap

Add `InstanceBindingMap` per §11.1:

```rust
pub type InstanceBindingMap = BTreeMap<String, String>;  // entity_id → instance_id
```

Update `FlowExecutionContext` (or whatever the flow execution input struct is):

```rust
pub struct FlowExecutionContext {
    pub flow_id: String,
    pub initiating_persona: String,
    pub snapshot: Snapshot,
    pub instance_bindings: InstanceBindingMap,
}
```

Update `execute_flow` per §11.4:

- Takes `InstanceBindingMap` as parameter
- At each OperationStep, resolves instance targets from bindings: `resolve_bindings(step.op, bindings)` maps the Operation's entity effect targets to specific instances
- Sub-flows inherit parent's instance bindings (§11.5, consistent with snapshot inheritance)
- Missing bindings for an entity referenced in effects = execution error

### A4: Action space — per-instance

Update `compute_action_space` per §15.6. The function now receives the instance-keyed EntityStateMap and produces per-instance results:

```rust
pub struct Action {
    pub flow_id: String,
    pub instance_bindings: BTreeMap<String, BTreeSet<String>>,  // entity_id → set of valid instance_ids
    pub verdicts_enabling: Vec<VerdictInstance>,
    pub personas: BTreeSet<String>,
}

pub struct BlockedAction {
    pub flow_id: String,
    pub instance_bindings: BTreeMap<String, BTreeSet<String>>,  // entity_id → instances that block
    pub reason: BlockReason,
}
```

For each flow, the action space reports:

- **Available:** which instances are in states that satisfy the flow's effect source states, AND verdicts/preconditions are met
- **Blocked:** which instances are in states that block the flow, with the specific reason

An action is available for a _specific combination_ of instance bindings. The `instance_bindings` on an Action lists, per entity type, the set of instances for which the action's effects are applicable.

**Action space size (AL79):** The action space is O(|flows| × product of |instances per entity type|). For large instance counts, this may be large. The evaluator computes the full action space — presentation grouping/filtering is a consumer concern.

### A5: Provenance — instance-scoped

Update OperationProvenance per §9.5:

```rust
pub struct OperationProvenance {
    pub op: String,
    pub persona: String,
    pub outcome: String,
    pub instance_binding: BTreeMap<String, String>,  // which instances were affected
    pub facts_used: BTreeSet<String>,
    pub verdicts_used: ResolvedVerdictSet,
    pub state_before: BTreeMap<(String, String), String>,  // per-instance
    pub state_after: BTreeMap<(String, String), String>,   // per-instance
}
```

`state_before` and `state_after` are per-instance maps restricted to the affected instances. Every transition is traceable to a specific instance.

Update FlowProvenance to carry instance bindings through step records.

### A6: Validation — contract load time

Update §15.1 contract load checks. No new checks needed for multi-instance (the contract doesn't declare instances — they're runtime). But verify:

- Entity declarations remain unchanged (no instances field)
- All existing validation still works
- The evaluator does not attempt to validate instance counts or ids at contract load time (AL74)

### A7: Update all existing tests

Every test that constructs an `EntityStateMap` must be updated to use the instance-keyed form. Use the `single_instance()` helper for backward compatibility:

```rust
// Before:
let states = btreemap! { "Order" => "draft" };

// After:
let states = EntityStateMap::single_instance(btreemap! { "Order".to_string() => "draft".to_string() });
```

This is tedious but mechanical. Every test must pass after the change.

### A8: New multi-instance tests

Add tests that exercise multi-instance behavior:

**Test: multiple instances same entity type**

- Create EntityStateMap with Order/ord-001 = "draft", Order/ord-002 = "submitted", Order/ord-003 = "approved"
- Compute action space
- Verify submit_order is available for ord-001 (draft), blocked for ord-002 (already submitted), blocked for ord-003

**Test: action space per-instance**

- Two Order instances in different states
- Verify action space correctly reports which instances are valid for which flows

**Test: execute with instance targeting**

- Execute an operation targeting a specific instance
- Verify only that instance's state changes
- Verify other instances are untouched

**Test: flow with instance bindings**

- Execute a flow with InstanceBindingMap { Order: "ord-001", DeliveryRecord: "del-001" }
- Verify the flow targets those specific instances
- Verify provenance records the instance bindings

**Test: missing instance binding = error**

- Execute a flow without providing a binding for an entity referenced in effects
- Verify execution error

**Test: single-instance degenerate case**

- Use `EntityStateMap::single_instance()` with one entity
- Verify behavior is identical to pre-multi-instance behavior
- This is the backward compat proof

**Test: instance absence (§6.5)**

- Instance not in EntityStateMap = doesn't exist from evaluator's perspective
- Verify operations targeting absent instances fail appropriately

### A9: WASM evaluator update

Update the WASM API to accept instance-keyed entity states. The JSON shape changes from:

```json
{
  "entity_states": {
    "Order": "draft",
    "DeliveryRecord": "pending"
  }
}
```

To:

```json
{
  "entity_states": {
    "Order": {
      "ord-001": "draft",
      "ord-002": "submitted"
    },
    "DeliveryRecord": {
      "del-001": "pending"
    }
  }
}
```

The WASM API must also accept the old flat format for backward compatibility and treat it as single-instance with `"_default"` instance ids. Detect format by checking whether the values are strings (old format) or objects (new format).

Update WASM action space output to include instance bindings.

Update WASM flow execution to accept instance bindings.

### Acceptance criteria — Part A

- [ ] EntityStateMap keyed by (entity_id, instance_id)
- [ ] `single_instance()` helper for backward compat
- [ ] `execute` takes (EntityId, InstanceId) for effect targets
- [ ] `execute_flow` takes InstanceBindingMap
- [ ] `resolve_bindings` maps effects to specific instances
- [ ] Sub-flows inherit parent's instance bindings
- [ ] Missing binding = execution error
- [ ] `compute_action_space` returns per-instance results
- [ ] OperationProvenance has instance_binding, per-instance state_before/state_after
- [ ] FlowProvenance carries instance bindings
- [ ] All existing tests updated and passing (using single_instance helper)
- [ ] New multi-instance tests passing
- [ ] WASM API accepts both old and new format
- [ ] WASM action space includes instance bindings
- [ ] All conformance tests pass (82+)
- [ ] All workspace tests pass (660+)
- [ ] `cargo clippy` clean
- [ ] Commits with descriptive messages

---

## Part B: Private Repo — Remove Collapsing Logic (~/src/riverline/tenor-platform)

After Part A is pushed, update deps:

```
cargo update -p tenor-eval -p tenor-storage
```

### B1: Find and remove collapsing logic

The private repo's storage uses `(entity_id, instance_id)` composite keys. Somewhere there is logic that collapses this to single-instance `entity_id → state` for the evaluator. Find it. Remove it. The evaluator now accepts the instance-keyed format natively.

Search for:

- Any place that converts `(entity_id, instance_id) → state` to `entity_id → state`
- Any place that strips instance_id before passing to evaluator functions
- Any `"_default"` instance id injection
- Any comment mentioning "collapse", "flatten", or "single-instance"

Replace with direct pass-through: storage returns `(entity_id, instance_id) → state`, evaluator receives `(entity_id, instance_id) → state`. No transformation.

### B2: Executor instance targeting

Update the executor's Operation execution to pass instance targets through to the evaluator:

- `execute` receives `(entity_id, instance_id)` from the flow's instance bindings
- Passes through to `tenor_eval::execute` (or whatever the evaluator function is called)
- Storage reads and writes use the composite key (they already do — just verify nothing is stripping instance_id)

### B3: Executor flow execution with InstanceBindingMap

Update the executor's flow execution:

- Accept `InstanceBindingMap` at flow invocation
- Pass it through to `tenor_eval::execute_flow`
- Storage operations use instance-keyed state throughout

### B4: Agent runtime instance awareness

Update the agent runtime's observe/evaluate/choose/execute loop:

**Observe:**

- `observe()` reads ALL instances from storage: `storage.list_entity_instances(entity_id)` returns all `(instance_id, state)` pairs
- Build the full instance-keyed EntityStateMap

**Evaluate:**

- Pass the instance-keyed EntityStateMap to `compute_action_space`
- Action space now reports per-instance availability

**Choose:**

- The `AgentPolicy` receives the per-instance action space
- The policy can reason about which instance to act on
- The policy's chosen Action includes the specific instance bindings

**Execute:**

- The chosen Action's instance bindings are passed to `execute_flow`
- The executor targets specific instances

### B5: HTTP endpoint updates

Update platform-serve endpoints to handle instance-keyed state:

**POST /{contract_id}/actions:**

- Returns per-instance action space
- Response includes `instance_bindings` on each Action

**POST /{contract_id}/flows/{flow_id}/execute:**

- Request body accepts `instance_bindings` parameter
- If not provided, falls back to `"_default"` for each entity type (backward compat)

**POST /{contract_id}/flows/{flow_id}/simulate:**

- Same instance binding support as execute

**POST /{contract_id}/entities/{entity_id}/initialize:**

- Already creates instances — verify it uses the instance model correctly (E15: initial state)

**GET /{contract_id}/entities/{entity_id}:**

- Already lists instances — verify response format is correct

### B6: Manifest capability

Advertise `multi_instance_entities: true` in the manifest capabilities:

```json
"capabilities": {
  "migration_analysis_mode": "conservative",
  "source_adapters": true,
  "multi_instance_entities": true
}
```

Per §19.1: "An executor advertising `multi_instance_entities: true` supports multiple runtime instances per entity type." This must reflect actual capability — after this prompt, it does.

### B7: Integration tests

Write integration tests (require Postgres) that:

**Test: multi-instance flow execution**

- Create multiple Order instances in different states
- Execute a flow targeting a specific instance
- Verify only that instance transitions
- Verify other instances untouched
- Verify provenance records instance binding

**Test: per-instance action space from HTTP**

- Create multiple instances
- Call POST /actions
- Verify response shows per-instance availability

**Test: instance binding in flow execution HTTP**

- Call POST /flows/{id}/execute with instance_bindings in request body
- Verify correct instance targeted

**Test: backward compat — no instance bindings**

- Call POST /flows/{id}/execute WITHOUT instance_bindings
- Verify falls back to \_default instance
- Verify behavior identical to pre-multi-instance

**Test: E15 — instance creation in initial state**

- Initialize a new entity instance
- Verify it's in the declared initial state
- Verify creating in non-initial state is rejected

**Test: E17 — instance enumeration complete**

- Create several instances
- Verify EntityStateMap provided to evaluator contains ALL of them
- No instance silently omitted

### Acceptance criteria — Part B

- [ ] Collapsing logic removed
- [ ] Storage passes instance-keyed state directly to evaluator
- [ ] Executor passes (entity_id, instance_id) to evaluator's execute
- [ ] Executor passes InstanceBindingMap to evaluator's execute_flow
- [ ] Agent runtime observes all instances
- [ ] Agent runtime passes per-instance action space to policy
- [ ] Agent runtime passes instance bindings to executor
- [ ] HTTP /actions returns per-instance action space
- [ ] HTTP /execute accepts instance_bindings
- [ ] HTTP /simulate accepts instance_bindings
- [ ] Backward compat: no instance_bindings = \_default
- [ ] Manifest: multi_instance_entities: true
- [ ] E15 test: creation in initial state
- [ ] E17 test: complete enumeration
- [ ] Multi-instance flow execution test
- [ ] Per-instance action space HTTP test
- [ ] All non-DB tests pass
- [ ] `cargo check` passes
- [ ] `cargo clippy` clean
- [ ] Commits with descriptive messages

---

## Execution Order

1. **Part A** (public — evaluator, WASM) — this is the foundational change
2. Push public repo
3. Update private repo deps
4. **Part B** (private — remove collapsing, wire instance bindings through executor, agent runtime, HTTP)

---

## Final Report

```
## Phase 4: Multi-Instance Entities — COMPLETE

### Public repo — Evaluator
- EntityStateMap: keyed by (entity_id, instance_id)
- single_instance() helper: backward compat verified
- execute: instance-targeted
- execute_flow: InstanceBindingMap
- compute_action_space: per-instance
- Provenance: instance-scoped (instance_binding, per-instance state_before/state_after)
- WASM: accepts both old (flat) and new (nested) format
- Existing tests: [N] updated, all passing
- New multi-instance tests: [N] passing
- Conformance tests: [N] passing

### Private repo — Executor/Runtime
- Collapsing logic: removed from [location]
- Executor: passes (entity_id, instance_id) natively
- Flow execution: InstanceBindingMap wired through
- Agent runtime: observes all instances, per-instance action space, instance-targeted execution
- HTTP endpoints: instance_bindings accepted, per-instance action space returned
- Backward compat: _default fallback verified
- Manifest: multi_instance_entities: true
- Integration tests: [N] passing

### Tests
- Public: [total] passing
- Private: [total] passing (non-DB) / [total] integration (DB)
- All conformance tests: PASS

### Executor obligations verified
- E15: instance creation in initial state — tested
- E16: identity stability — by design (storage composite key)
- E17: instance enumeration complete — tested

### Commits
Public:
- [hash] [message]
- ...

Private:
- [hash] [message]
- ...
```

Phase 7 is done when the evaluator is natively instance-aware, the private repo's collapsing logic is gone, the agent runtime reasons about instances, HTTP endpoints accept instance bindings, and every checkbox above is checked. Not before.
