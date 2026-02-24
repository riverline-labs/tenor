# Phase 2 Context: Execution Kernel

## What We're Building

A new crate: `tenor-executor` — a thin transactional wrapper around the existing evaluator. Postgres-backed. Enforces executor obligations (E2, E3, E4, E7, C7) mechanically against real storage.

## The Execution Flow

```
1. Begin Postgres transaction
2. Read current entity states with version numbers (SELECT ... FOR UPDATE)
3. Build FactSet from caller-provided external inputs
4. Call the existing evaluator (evaluate_flow or evaluate)
5. If the evaluator produces entity state transitions:
   a. Validate each entity's current version matches what was read in step 2
   b. Apply state updates (UPDATE entity state, increment version)
   c. Insert provenance records (atomically, in the same transaction)
6. Commit transaction
7. On version mismatch: rollback, return typed conflict error
```

## Design Constraints (Non-Negotiable)

### From the Spec

- **E2 — Transition source validation.** Validate current entity state matches transition source before applying effects. Mismatch aborts with typed error.
- **E3 — Atomicity enforcement.** An Operation's effect set applied atomically via Postgres transaction boundary.
- **E4 — Snapshot isolation.** Evaluator handles this. Executor must not interfere.
- **E7 — Numeric model conformance.** Use NUMERIC in Postgres, not FLOAT. Evaluator handles arithmetic.
- **C7 — Provenance as semantics.** Provenance atomically coupled to commit. Commit succeeds → provenance exists. Provenance fails → commit rolls back.

### Architectural Decisions

- **Conflicts are first-class outcomes, not silent retries.** Return typed `ConcurrentConflict`. No automatic retry. Caller decides.
- **Optimistic concurrency control.** Snapshot read, evaluate, version-validated atomic commit. `SELECT ... FOR UPDATE` within transaction prevents read skew; optimism is across concurrent executions.
- **No premature abstraction.** Postgres directly. No `StorageTrait`. No `StorageBackend`. No generic interface. Extract later when second backend needed.
- **No adapter framework.** Facts provided by caller as typed values.

## Postgres Schema

### Core Tables

- **`entity_states`** — entity_id, instance_id, state, version (OCC), updated_at, last_flow_id, last_operation_id
- **`flow_executions`** — id, flow_id, contract_id, persona_id, started_at, completed_at, outcome, snapshot_facts (JSONB), snapshot_verdicts (JSONB)
- **`operation_executions`** — id, flow_execution_id, operation_id, persona_id, outcome, executed_at, step_id
- **`entity_transitions`** — id, operation_execution_id, entity_id, instance_id, from_state, to_state, from_version, to_version
- **`provenance_records`** — id, operation_execution_id, facts_used (JSONB), verdicts_used (JSONB), verdict_set_snapshot (JSONB)

All timestamps TIMESTAMPTZ/UTC. All decimals NUMERIC. All structured data JSONB.

## Public API

```rust
pub struct TenorExecutor {
    pool: PgPool,
    contract: Contract,
}

impl TenorExecutor {
    pub fn new(pool: PgPool, contract: Contract) -> Self;
    pub async fn execute_flow(&self, flow_id, persona_id, facts, instance_ids) -> Result<FlowExecutionResult, ExecutorError>;
    pub async fn evaluate(&self, facts) -> Result<EvaluationResult, ExecutorError>;
    pub async fn initialize_entity(&self, entity_id, instance_id) -> Result<(), ExecutorError>;
}
```

### Error Types

- `ConcurrentConflict` — version mismatch, typed with entity/instance/expected/actual
- `PersonaRejected` — unauthorized persona for operation
- `PreconditionFailed` — operation precondition not met
- `TransitionSourceMismatch` — entity not in expected state
- `FactAssemblyError` — missing required fact or type error
- `StorageError` — database error
- `EvaluatorError` — evaluator error (shouldn't happen with valid contracts)

## What This Crate Must NOT Do

- No pluggable storage (no traits, no generics)
- No automatic retry on conflict
- No adapter/ingestion logic
- No HTTP/gRPC layer (that's a separate crate)
- No WASM concerns
- No async evaluation (evaluator is sync; executor async only at DB boundary)
- No migration logic (§18 is future)

## Implementation Details

- Use `sqlx` with Postgres driver for async DB access
- Use `sqlx::Transaction` for atomic commit boundary
- `SELECT ... FOR UPDATE` within transaction for read skew prevention
- Import evaluator types directly from `tenor-eval` — do not redefine
- UUID v7 (time-ordered) for execution IDs
- `migrations/` directory with sqlx migrate files

## Required Integration Tests

- Execute flow → verify entity state transitions
- Verify provenance records created atomically with state changes
- Concurrent conflict detection (two transactions racing on same entity)
- Rollback on version mismatch (no partial state)
- Persona rejection → typed error
- Precondition failure → typed error
- Transition source mismatch → typed error
