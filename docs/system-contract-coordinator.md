# SystemContract Coordinator Design

## Problem Statement

The `system` construct in Tenor DSL allows declaring cross-contract relationships:
shared personas, shared entities, and flow triggers that link contracts together.
The elaborator validates these declarations statically (Pass 5), and the analyzer
reports cross-contract authority and trigger paths (S4, S6). However, no runtime
exists to actually **execute** cross-contract interactions -- dispatching triggers
from one contract's flow to another, maintaining shared entity state consistency,
and mapping personas across contract boundaries.

This document describes the `SystemRuntime` coordinator architecture for
multi-contract execution.

## Coordinator Architecture

```
                    +-------------------+
                    |  SystemRuntime    |
                    |                   |
                    |  - contracts[]    |
                    |  - shared_state   |
                    |  - trigger_map    |
                    |  - persona_map    |
                    +--------+----------+
                             |
              +--------------+--------------+
              |              |              |
        +-----+----+  +-----+----+  +------+---+
        | Contract  |  | Contract |  | Contract |
        | "billing" |  | "orders" |  | "shipping"|
        +----------+  +----------+  +----------+
```

### Key Types

```rust
/// Runtime coordinator for multi-contract execution.
pub struct SystemRuntime {
    /// System construct definition (from elaboration).
    system: InterchangeSystem,

    /// Loaded contract bundles, keyed by member ID.
    contracts: HashMap<String, ContractInstance>,

    /// Shared entity state (single source of truth).
    shared_state: SharedEntityState,

    /// Trigger dispatch table: (source_contract, source_flow, event) -> target.
    trigger_map: Vec<TriggerRoute>,

    /// Persona mapping across contract boundaries.
    persona_map: PersonaMap,
}

/// A loaded contract ready for evaluation.
pub struct ContractInstance {
    pub id: String,
    pub bundle: serde_json::Value,
    /// Per-contract entity state (for non-shared entities).
    pub local_state: EntityStateMap,
}

/// Single source of truth for shared entities.
pub struct SharedEntityState {
    /// Maps (entity_id) -> current state.
    /// Shared entities are owned by the coordinator, not individual contracts.
    states: HashMap<String, String>,
}

/// A route from a flow event to a target flow in another contract.
pub struct TriggerRoute {
    pub source_contract: String,
    pub source_flow: String,
    pub on: String, // event name (e.g., "success", "failure")
    pub target_contract: String,
    pub target_flow: String,
    pub persona: String,
}

/// Maps persona names across contract boundaries.
pub struct PersonaMap {
    /// (persona_name) -> list of contract IDs where this persona is recognized.
    mappings: HashMap<String, Vec<String>>,
}
```

## Trigger Dispatch

When a flow step in Contract A completes with a named outcome, the coordinator
checks the trigger map for a matching route:

```rust
impl SystemRuntime {
    /// Execute a flow step and dispatch any matching triggers.
    pub fn execute_step(
        &mut self,
        contract_id: &str,
        flow_id: &str,
        step_outcome: &str,
        persona: &str,
    ) -> Result<StepResult, SystemError> {
        // 1. Execute the step in the source contract
        let contract = self.contracts.get_mut(contract_id)?;
        let result = evaluate_flow_step(contract, flow_id, persona)?;

        // 2. Check for matching trigger routes
        let triggers: Vec<&TriggerRoute> = self.trigger_map.iter()
            .filter(|t| {
                t.source_contract == contract_id
                && t.source_flow == flow_id
                && t.on == step_outcome
            })
            .collect();

        // 3. Dispatch each matching trigger
        for trigger in triggers {
            let target = self.contracts.get_mut(&trigger.target_contract)?;
            self.dispatch_trigger(target, trigger)?;
        }

        Ok(result)
    }

    fn dispatch_trigger(
        &mut self,
        target: &mut ContractInstance,
        trigger: &TriggerRoute,
    ) -> Result<(), SystemError> {
        // Validate persona is authorized in target contract
        self.validate_persona(&trigger.persona, &trigger.target_contract)?;

        // Execute the target flow with shared entity state
        evaluate_flow(
            target,
            &trigger.target_flow,
            &trigger.persona,
            &mut self.shared_state,
        )
    }
}
```

### Trigger Cycle Prevention

The static analyzer already detects trigger cycles (S6 cross-contract paths).
At runtime, the coordinator additionally maintains a call stack to prevent
re-entrant trigger dispatch:

```rust
pub struct TriggerCallStack {
    stack: Vec<(String, String)>, // (contract_id, flow_id)
    max_depth: usize,            // configurable limit (default: 16)
}
```

## Shared Entity State

Entities listed in the system's `shared_entities` declaration are managed by
the coordinator rather than individual contracts:

1. **Initialization**: Shared entities start in their declared `initial` state.
2. **Reads**: Any contract can read the current state of a shared entity.
3. **Writes**: Operations that transition a shared entity go through the coordinator.
4. **Consistency**: Entity state transitions are atomic -- if an operation fails,
   the shared state is not modified (rollback on error).

```rust
impl SharedEntityState {
    /// Get the current state of a shared entity.
    pub fn get_state(&self, entity_id: &str) -> Option<&str> {
        self.states.get(entity_id).map(|s| s.as_str())
    }

    /// Attempt a state transition. Returns error if current state doesn't match.
    pub fn transition(
        &mut self,
        entity_id: &str,
        expected_from: &str,
        to: &str,
    ) -> Result<(), SharedEntityError> {
        let current = self.states.get(entity_id)
            .ok_or(SharedEntityError::NotFound(entity_id.to_string()))?;

        if current != expected_from {
            return Err(SharedEntityError::InvalidState {
                entity_id: entity_id.to_string(),
                expected: expected_from.to_string(),
                actual: current.to_string(),
            });
        }

        self.states.insert(entity_id.to_string(), to.to_string());
        Ok(())
    }
}
```

## Persona Mapping

The system's `shared_personas` declaration maps persona names across contracts.
When a trigger fires, the coordinator resolves the triggering persona to the
target contract's namespace:

```rust
impl PersonaMap {
    /// Check if a persona is recognized in a given contract.
    pub fn is_authorized(&self, persona: &str, contract_id: &str) -> bool {
        self.mappings.get(persona)
            .map(|contracts| contracts.contains(&contract_id.to_string()))
            .unwrap_or(false)
    }
}
```

## Error Handling

The coordinator defines system-level errors that wrap contract-level errors:

```rust
pub enum SystemError {
    /// Contract not found in the system.
    ContractNotFound(String),
    /// Persona not authorized in target contract.
    UnauthorizedPersona { persona: String, contract: String },
    /// Trigger cycle detected at runtime.
    TriggerCycle { stack: Vec<(String, String)> },
    /// Shared entity state conflict.
    SharedEntityConflict(SharedEntityError),
    /// Underlying contract evaluation error.
    ContractError { contract: String, error: String },
}
```

## API Sketch

```rust
// Construction
let runtime = SystemRuntime::from_interchange(system_bundle)?;

// Load member contracts
runtime.load_contract("billing", billing_bundle)?;
runtime.load_contract("orders", orders_bundle)?;

// Execute a flow in a member contract
let result = runtime.execute_flow("orders", "order_flow", "buyer")?;

// Inspect shared state
let order_state = runtime.shared_state().get_state("Order");
```

## Implementation Phases

### Phase 25: Multi-party Contract Execution

1. **SystemRuntime scaffold**: Create `crates/system/` crate with `SystemRuntime`,
   `ContractInstance`, and `SharedEntityState` types.
2. **Contract loading**: Load and validate member contracts against the system
   declaration (path resolution, persona verification).
3. **Shared entity management**: Implement `SharedEntityState` with atomic
   transitions and rollback on error.
4. **Trigger dispatch**: Implement trigger routing with cycle detection and
   configurable depth limits.
5. **Persona mapping**: Implement cross-contract persona resolution from
   `shared_personas` declarations.
6. **CLI integration**: Add `tenor system eval` subcommand that loads a system
   bundle, resolves member contracts, and executes flows with the coordinator.
7. **Conformance fixtures**: Create system evaluation fixtures that exercise
   cross-contract triggers, shared entities, and persona mapping.

### Future Work (Beyond v1.0)

- **Concurrent execution**: Parallel evaluation of independent contract branches.
- **Transaction semantics**: Multi-contract ACID-like guarantees for shared state.
- **Event sourcing**: Record all trigger dispatches and state transitions for audit.
- **Distributed execution**: gRPC-based coordinator for contracts running in
  separate processes.
