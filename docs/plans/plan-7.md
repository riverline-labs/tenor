# Phase 7: SDKs — Complete Implementation

Developers in Rust, Go, TypeScript, and Python can embed Tenor evaluation in their applications. The Rust SDK is already the public crate. The WASM evaluator covers TypeScript/JavaScript in browsers. This phase adds idiomatic SDK wrappers for Go, TypeScript (Node.js), and Python that consume the WASM evaluator or expose the Rust crate via FFI.

**Repo:** Public only (`~/src/riverline/tenor`).

**Source of truth:** The evaluator's public API surface: `evaluate()`, `compute_action_space()`, `execute_flow()`, `assemble_facts()`, plus the interchange format JSON schema. The WASM API already defined in `crates/tenor-eval-wasm/`.

---

## What "done" means

1. **Rust** — already done (the `tenor-eval` crate IS the Rust SDK)
2. **TypeScript/JavaScript (Node.js)** — npm package wrapping the WASM evaluator with TypeScript types
3. **Python** — PyPI package wrapping the evaluator via PyO3 (Rust → Python FFI)
4. **Go** — Go module wrapping the evaluator via CGo FFI (Rust → C → Go)
5. Each SDK: evaluate, compute action space, execute flow, read interchange bundles
6. Each SDK: type-safe representations of ActionSpace, Action, BlockedAction, Verdict, FactSet
7. Each SDK: published package with README, examples, and API docs
8. Each SDK: test suite proving identical results to the Rust evaluator for the same inputs

---

## Part A: TypeScript/JavaScript SDK

### A1: Package structure

Create `sdks/typescript/` (or `packages/tenor-js/`):

```
sdks/typescript/
  package.json
  tsconfig.json
  src/
    index.ts          — main exports
    types.ts          — TypeScript type definitions
    evaluator.ts      — wrapper around WASM module
    action-space.ts   — ActionSpace helpers
  tests/
    evaluator.test.ts
    action-space.test.ts
  README.md
```

The package wraps the existing WASM evaluator (`crates/tenor-eval-wasm/`). The WASM binary is bundled in the npm package.

### A2: TypeScript types

Map every evaluator type to TypeScript:

```typescript
interface Fact {
  id: string;
  type: FactType;
  value: FactValue;
  source?: string | StructuredSource;
}

interface EntityStateMap {
  [entityId: string]: {
    [instanceId: string]: string; // state
  };
}

interface ActionSpace {
  actions: Action[];
  blocked_actions: BlockedAction[];
  current_verdicts: Verdict[];
}

interface Action {
  flow_id: string;
  instance_bindings: Record<string, Set<string>>;
  verdicts_enabling: Verdict[];
  personas: Set<string>;
}

interface BlockedAction {
  flow_id: string;
  reason: BlockReason;
}

// ... complete type coverage
```

These types must match the WASM API's JSON interface exactly. No type drift.

### A3: Evaluator wrapper

```typescript
class TenorEvaluator {
  static async fromBundle(bundle: InterchangeBundle): Promise<TenorEvaluator>;
  static async fromJson(json: string): Promise<TenorEvaluator>;

  evaluate(facts: FactSet): EvaluationResult;
  computeActionSpace(
    facts: FactSet,
    entityStates: EntityStateMap,
    persona: string,
  ): ActionSpace;
  executeFlow(
    flowId: string,
    facts: FactSet,
    entityStates: EntityStateMap,
    persona: string,
    instanceBindings: InstanceBindingMap,
  ): FlowResult;
}
```

The wrapper handles WASM initialization, JSON serialization/deserialization between TypeScript types and the WASM boundary, and error mapping.

### A4: Tests

- Test: evaluate escrow contract with known facts → known verdicts
- Test: compute action space → correct available/blocked actions
- Test: execute flow → correct state transitions
- Test: results match Rust evaluator byte-for-byte (same inputs → same JSON output)
- Test: multi-instance entity states handled correctly

### A5: Build and publish

- Build script: compile WASM from Rust, bundle into npm package
- `npm pack` produces installable package
- README with installation, usage example, API reference
- Works in Node.js and browsers (via bundler)

### Acceptance criteria — Part A

- [ ] npm package builds
- [ ] TypeScript types cover full evaluator API
- [ ] evaluate, computeActionSpace, executeFlow work
- [ ] Tests prove identical results to Rust evaluator
- [ ] Multi-instance support
- [ ] README with examples
- [ ] Works in Node.js

---

## Part B: Python SDK

### B1: PyO3 wrapper

Create `sdks/python/` using PyO3 (maturin build):

```
sdks/python/
  Cargo.toml        — PyO3 crate
  src/
    lib.rs           — #[pymodule] definition
    evaluator.rs     — Python-facing evaluator wrapper
    types.rs         — Python type conversions
  python/
    tenor/
      __init__.py
      types.py       — Python type stubs
  tests/
    test_evaluator.py
  pyproject.toml
  README.md
```

PyO3 compiles the Rust evaluator directly into a Python native module — no WASM, no subprocess, full native speed.

### B2: Python API

```python
from tenor import TenorEvaluator, FactSet, EntityStateMap

evaluator = TenorEvaluator.from_bundle_json(bundle_json)

# Evaluate
result = evaluator.evaluate(facts)
print(result.verdicts)

# Action space
action_space = evaluator.compute_action_space(facts, entity_states, persona="escrow_agent")
for action in action_space.actions:
    print(f"Available: {action.flow_id} for instances {action.instance_bindings}")
for blocked in action_space.blocked_actions:
    print(f"Blocked: {blocked.flow_id} — {blocked.reason}")

# Execute flow
result = evaluator.execute_flow("release_escrow", facts, entity_states, "escrow_agent", instance_bindings)
```

### B3: Type stubs

Python type stubs (`.pyi` files) for IDE autocomplete and mypy:

```python
class TenorEvaluator:
    @staticmethod
    def from_bundle_json(json: str) -> TenorEvaluator: ...
    def evaluate(self, facts: FactSet) -> EvaluationResult: ...
    def compute_action_space(self, facts: FactSet, entity_states: EntityStateMap, persona: str) -> ActionSpace: ...
    def execute_flow(self, flow_id: str, facts: FactSet, entity_states: EntityStateMap, persona: str, instance_bindings: dict[str, str]) -> FlowResult: ...
```

### B4: Tests

- pytest suite mirroring the TypeScript tests
- Same known inputs → same known outputs as Rust evaluator
- Multi-instance support
- Error handling (invalid bundle, missing facts, wrong persona)

### B5: Build and publish

- `maturin build` produces wheel
- `pip install` works
- README with installation, usage example
- Type stubs included for IDE support

### Acceptance criteria — Part B

- [ ] PyO3 crate builds with maturin
- [ ] Python API: evaluate, compute_action_space, execute_flow
- [ ] Type stubs for IDE support
- [ ] Tests prove identical results to Rust evaluator
- [ ] Multi-instance support
- [ ] README with examples

---

## Part C: Go SDK

### C1: CGo FFI wrapper

Create `sdks/go/`:

```
sdks/go/
  go.mod
  tenor.go          — Go API (wraps C FFI)
  tenor_test.go     — tests
  internal/
    ffi/
      tenor.h       — C header generated by cbindgen
      libtenor.a     — static library
  README.md
```

The Rust evaluator is compiled as a C static library (`cdylib` or `staticlib`). A thin C header is generated by `cbindgen`. The Go package calls through CGo.

Alternatively, if the WASM approach is simpler: use `wazero` (pure Go WASM runtime) to execute the WASM evaluator. This avoids CGo entirely and is more portable.

**Decide based on what's simpler.** If the WASM evaluator already works and wazero is straightforward, use that. If CGo + cbindgen is more natural, use that. Document the choice.

### C2: Go API

```go
package tenor

type Evaluator struct { ... }

func NewEvaluatorFromBundle(bundleJSON []byte) (*Evaluator, error)

func (e *Evaluator) Evaluate(facts FactSet) (*EvaluationResult, error)
func (e *Evaluator) ComputeActionSpace(facts FactSet, entityStates EntityStateMap, persona string) (*ActionSpace, error)
func (e *Evaluator) ExecuteFlow(flowID string, facts FactSet, entityStates EntityStateMap, persona string, instanceBindings map[string]string) (*FlowResult, error)

type ActionSpace struct {
    Actions        []Action
    BlockedActions []BlockedAction
    CurrentVerdicts []Verdict
}

type Action struct {
    FlowID           string
    InstanceBindings map[string][]string
    VerdictEnabling  []Verdict
    Personas         []string
}
```

### C3: Tests

- Go test suite: same known inputs → same known outputs
- Multi-instance support
- Error handling

### C4: Build and publish

- Build instructions in README
- Go module publishable (`go get github.com/riverline-labs/tenor-go`)
- The WASM binary or static library must be distributed with the module

### Acceptance criteria — Part C

- [ ] Go module builds
- [ ] Go API: Evaluate, ComputeActionSpace, ExecuteFlow
- [ ] Tests prove identical results to Rust evaluator
- [ ] Multi-instance support
- [ ] README with examples
- [ ] Distributable (go get or vendored binary)

---

## Part D: Cross-SDK conformance test

Create a conformance test suite that runs against ALL SDKs with identical inputs and asserts identical outputs:

```
sdks/conformance/
  fixtures/
    escrow-bundle.json
    escrow-facts.json
    escrow-entity-states.json
    expected-verdicts.json
    expected-action-space.json
    expected-flow-result.json
  run-all.sh         — runs conformance against each SDK
```

The fixtures are generated from the Rust evaluator (source of truth). Each SDK reads the fixtures, evaluates, and asserts identical JSON output.

### Acceptance criteria — Part D

- [ ] Conformance fixtures generated from Rust evaluator
- [ ] TypeScript SDK passes conformance
- [ ] Python SDK passes conformance
- [ ] Go SDK passes conformance
- [ ] `run-all.sh` runs all three

---

## Final Report

```
## Phase 7: SDKs — COMPLETE

### TypeScript/JavaScript
- Package: [name]
- API: evaluate, computeActionSpace, executeFlow
- Mechanism: WASM evaluator
- Tests: [N] passing
- Conformance: PASS

### Python
- Package: [name]
- API: evaluate, compute_action_space, execute_flow
- Mechanism: PyO3 native FFI
- Tests: [N] passing
- Conformance: PASS

### Go
- Package: [name]
- API: Evaluate, ComputeActionSpace, ExecuteFlow
- Mechanism: [WASM via wazero / CGo FFI]
- Tests: [N] passing
- Conformance: PASS

### Cross-SDK conformance
- Fixtures: escrow contract
- All SDKs: identical output for identical inputs

### Commits
- [hash] [message]
- ...
```

Phase 7 is done when all three SDKs build, pass their tests, pass cross-SDK conformance, and are publishable. Not before.
