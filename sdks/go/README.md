# tenor-go

Go SDK for the [Tenor](https://github.com/riverline-labs/tenor) contract evaluator.

Wraps the Tenor WASM evaluator via [wazero](https://github.com/tetratelabs/wazero) — a pure-Go WebAssembly runtime with no CGo or native dependencies. The WASM binary is embedded into the Go binary at compile time via `go:embed`.

## Installation

```bash
go get github.com/riverline-labs/tenor-go
```

> **Note:** The module ships with a pre-built WASM binary (`internal/wasm/tenor_eval.wasm`).
> To rebuild it from source, see [Build from source](#build-from-source).

## Quick start

```go
package main

import (
    "fmt"
    "log"
    "os"

    tenor "github.com/riverline-labs/tenor-go"
)

func main() {
    // Load your interchange bundle JSON (produced by `tenor elaborate`)
    bundleJSON, err := os.ReadFile("my_contract.json")
    if err != nil {
        log.Fatal(err)
    }

    eval, err := tenor.NewEvaluatorFromBundle(bundleJSON)
    if err != nil {
        log.Fatal(err)
    }
    defer eval.Close()

    // Evaluate rules against facts
    verdicts, err := eval.Evaluate(tenor.FactSet{
        "is_active":   true,
        "credit_score": 720,
    })
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Verdicts: %+v\n", verdicts.Verdicts)

    // Compute available actions for a persona
    space, err := eval.ComputeActionSpace(
        tenor.FactSet{"is_active": true},
        tenor.EntityStateMap{"Order": "pending"},
        "admin",
    )
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Available actions: %d\n", len(space.Actions))
    fmt.Printf("Blocked actions:   %d\n", len(space.BlockedActions))

    // Simulate a flow
    result, err := eval.ExecuteFlow(
        "approval_flow",
        tenor.FactSet{"is_active": true},
        tenor.EntityStateMap{"Order": "pending"},
        "admin",
    )
    if err != nil {
        log.Fatal(err)
    }
    fmt.Printf("Flow outcome: %s\n", result.Outcome)
    for _, t := range result.WouldTransition {
        fmt.Printf("  %s %s -> %s\n", t.EntityID, t.FromState, t.ToState)
    }
}
```

## API reference

### Creating an evaluator

```go
eval, err := tenor.NewEvaluatorFromBundle(bundleJSON []byte) (*Evaluator, error)
```

Creates an Evaluator from a Tenor interchange bundle JSON.
The bundle must be produced by `tenor elaborate` or the Tenor elaboration pipeline.

### Evaluator methods

#### `Evaluate`

```go
func (e *Evaluator) Evaluate(facts FactSet) (*VerdictSet, error)
```

Runs stratified rule evaluation against the provided facts.
Returns all verdicts with full provenance (rule, stratum, facts used).

#### `ComputeActionSpace`

```go
func (e *Evaluator) ComputeActionSpace(
    facts FactSet,
    entityStates EntityStateMap,
    persona string,
) (*ActionSpace, error)
```

Computes available and blocked actions for a persona given current facts and entity states.

For multi-instance contracts, use `ComputeActionSpaceNested`:

```go
func (e *Evaluator) ComputeActionSpaceNested(
    facts FactSet,
    entityStates EntityStateMapNested, // map[entity_id]map[instance_id]state
    persona string,
) (*ActionSpace, error)
```

#### `ExecuteFlow`

```go
func (e *Evaluator) ExecuteFlow(
    flowID string,
    facts FactSet,
    entityStates EntityStateMap,
    persona string,
) (*FlowResult, error)
```

Simulates a flow execution. Returns outcome, path, entity state changes, and verdicts.
No side effects — this is a pure simulation.

For multi-instance contracts with explicit instance bindings, use `ExecuteFlowWithBindings`:

```go
func (e *Evaluator) ExecuteFlowWithBindings(
    flowID string,
    facts FactSet,
    entityStates EntityStateMapNested,
    persona string,
    bindings InstanceBindings, // map[entity_id]instance_id
) (*FlowResult, error)
```

#### `Close`

```go
func (e *Evaluator) Close() error
```

Releases all WASM runtime resources. Call via `defer` after creating an Evaluator.

## Key types

| Type | Description |
|------|-------------|
| `FactSet` | `map[string]interface{}` — maps fact IDs to values |
| `EntityStateMap` | `map[string]string` — entity_id to state (single-instance) |
| `EntityStateMapNested` | `map[string]map[string]string` — entity_id to instance_id to state |
| `InstanceBindings` | `map[string]string` — entity_id to instance_id for flow targeting |
| `VerdictSet` | Evaluation result: `[]Verdict` |
| `Verdict` | One verdict: `Type`, `Payload`, `Provenance` |
| `VerdictProvenance` | `Rule`, `Stratum`, `FactsUsed` |
| `ActionSpace` | `PersonaID`, `Actions`, `BlockedActions`, `CurrentVerdicts` |
| `Action` | `FlowID`, `PersonaID`, `EntryOperationID`, `EnablingVerdicts`, `AffectedEntities` |
| `BlockedAction` | `FlowID`, `Reason` (type: PersonaNotAuthorized, PreconditionNotMet, EntityNotInSourceState, MissingFacts) |
| `FlowResult` | `FlowID`, `Outcome`, `Path`, `WouldTransition`, `Verdicts` |

## Architecture

```
tenor-go/
  tenor.go            — Evaluator API (NewEvaluatorFromBundle, Evaluate, ComputeActionSpace, ExecuteFlow)
  types.go            — Go type definitions (FactSet, ActionSpace, FlowResult, ...)
  tenor_test.go       — Test suite (17 tests)
  internal/wasm/
    runtime.go        — wazero runtime wrapper (alloc/dealloc memory protocol)
    tenor_eval.wasm   — Embedded WASM binary (built from wasm-bridge/)
  wasm-bridge/
    Cargo.toml        — Rust crate (wasm32-wasip1, no wasm-bindgen)
    src/lib.rs        — C-ABI exports: load_contract, evaluate, compute_action_space, simulate_flow
  scripts/
    build-wasm.sh     — Build script: cargo build --target wasm32-wasip1
```

**Why wazero?** Pure Go — no CGo, no native toolchain required at runtime.
The WASM binary is embedded via `go:embed`, so the final binary has zero external dependencies.

**Why wasm32-wasip1?** The WASM bridge needs WASI for system calls (rand, time).
wazero has a built-in WASI snapshot_preview1 implementation, so no JS runtime is needed.

## Build from source

To rebuild the WASM binary from the Rust source:

```bash
# Ensure wasm32-wasip1 target is installed
rustup target add wasm32-wasip1

# Build the WASM bridge and copy it to the Go embed location
./scripts/build-wasm.sh
```

Then rebuild the Go module:

```bash
go build ./...
go test ./...
```

## Requirements

- Go 1.21+
- No CGo, no native dependencies
- The WASM binary is pre-built and embedded — no Rust toolchain needed at build time
  (unless you want to rebuild the WASM from source)

## License

Same as the Tenor project.
