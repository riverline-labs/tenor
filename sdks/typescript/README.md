# @tenor/sdk

TypeScript/JavaScript SDK for the Tenor contract evaluator. Wraps the WASM evaluator compiled from `crates/tenor-eval-wasm/` and exposes a TypeScript-first API for evaluating Tenor contracts in Node.js.

## Installation

```bash
npm install @tenor/sdk
```

The package includes pre-built WASM artifacts in `wasm/`. No additional build step is required for npm consumers.

## Quick start

```typescript
import { TenorEvaluator, isFlowAvailable } from '@tenor/sdk';
import * as fs from 'fs';

// Load your interchange bundle (output of `tenor elaborate`)
const bundleJson = fs.readFileSync('my-contract.json', 'utf-8');

const evaluator = TenorEvaluator.fromJson(bundleJson);
try {
  // Evaluate rules
  const verdicts = evaluator.evaluate({ is_active: true });
  console.log(`${verdicts.verdicts.length} verdicts`);

  // Compute action space for a persona
  const space = evaluator.computeActionSpace(
    { is_active: true },
    { Order: 'pending' },
    'admin'
  );
  console.log(`${space.actions.length} available actions`);

  if (isFlowAvailable(space, 'approval_flow')) {
    // Execute a flow (simulation — no state mutation)
    const result = evaluator.executeFlow(
      'approval_flow',
      { is_active: true },
      { Order: 'pending' },
      'admin'
    );
    console.log(`Outcome: ${result.outcome}`);
    console.log(`Transitions: ${JSON.stringify(result.would_transition)}`);
  }
} finally {
  evaluator.free();
}
```

## API reference

### `TenorEvaluator`

The main class wrapping a loaded Tenor contract.

#### Static factory methods

```typescript
static fromJson(json: string): TenorEvaluator
```
Load a contract from an interchange bundle JSON string. Throws if the JSON is invalid or not a valid contract.

```typescript
static fromBundle(bundle: InterchangeBundle): TenorEvaluator
```
Load a contract from an interchange bundle object.

#### Instance methods

```typescript
evaluate(facts: FactSet): VerdictSet
```
Evaluate rules against the provided facts. Returns the set of verdicts produced.

```typescript
computeActionSpace(
  facts: FactSet,
  entityStates: EntityStateInput,
  persona: string
): ActionSpace
```
Compute the complete action space for a persona given current facts and entity states. Returns available and blocked actions.

```typescript
executeFlow(
  flowId: string,
  facts: FactSet,
  entityStates: EntityStateInput,
  persona: string
): FlowResult
```
Simulate a flow execution (read-only — no state mutation). Returns the outcome, execution path, and entity state transitions that would occur.

```typescript
executeFlowWithBindings(
  flowId: string,
  facts: FactSet,
  entityStates: EntityStateInput,
  persona: string,
  instanceBindings: InstanceBindings
): FlowResult
```
Simulate a flow with explicit instance bindings for multi-instance entities.

```typescript
inspect(): InspectResult
```
Inspect the loaded contract's structure: facts, entities, rules, operations, flows.

```typescript
free(): void
```
Free the contract handle. The evaluator cannot be used after calling `free()`. Calling `free()` on an already-freed evaluator is a no-op.

```typescript
get isFreed(): boolean
```
Whether this evaluator has been freed.

### Action space helpers

Pure functions for working with `ActionSpace` objects:

```typescript
actionsForFlow(space: ActionSpace, flowId: string): Action[]
isFlowAvailable(space: ActionSpace, flowId: string): boolean
isFlowBlocked(space: ActionSpace, flowId: string): boolean
getBlockReason(space: ActionSpace, flowId: string): BlockedReason | undefined
getBlockedAction(space: ActionSpace, flowId: string): BlockedAction | undefined
availableFlowIds(space: ActionSpace): string[]
blockedFlowIds(space: ActionSpace): string[]
hasVerdict(space: ActionSpace, verdictType: string): boolean
```

## Key types

### `FactSet`

```typescript
interface FactSet {
  [factId: string]: FactValue;
}
type FactValue = boolean | string | number | MoneyValue | FactRecord | FactList | null;
```

### `EntityStateInput`

The WASM module accepts both flat (single-instance) and nested (multi-instance) entity state formats:

```typescript
// Flat (single-instance):
{ Order: 'pending' }

// Nested (multi-instance):
{ Order: { 'ord-001': 'pending', 'ord-002': 'approved' } }
```

### `ActionSpace`

```typescript
interface ActionSpace {
  persona_id: string;
  actions: Action[];               // Available (non-blocked) actions
  current_verdicts: VerdictSummary[]; // Active verdicts given current facts
  blocked_actions: BlockedAction[]; // Blocked actions with reasons
}
```

### `BlockedReason`

Discriminated union:

```typescript
type BlockedReason =
  | { type: 'PersonaNotAuthorized' }
  | { type: 'PreconditionNotMet'; missing_verdicts: string[] }
  | { type: 'EntityNotInSourceState'; entity_id: string; current_state: string; required_state: string }
  | { type: 'MissingFacts'; fact_ids: string[] };
```

### `FlowResult`

```typescript
interface FlowResult {
  simulation: boolean;       // Always true (simulation mode)
  flow_id: string;
  persona: string;
  outcome: string;           // The terminal outcome name
  path: StepResult[];        // Steps executed
  would_transition: EntityStateChange[]; // State transitions that would occur
  verdicts: Verdict[];       // Verdicts active at flow initiation
}
```

## Runtime requirements

- **Node.js** 18 or later
- The package works in Node.js. Browser support via a bundler may be possible but is not tested in this release.

## Build from source

Requirements: Rust toolchain, `wasm-pack`

```bash
# 1. Clone the repository
git clone https://github.com/riverline/tenor
cd tenor

# 2. Build the WASM module and npm package
cd sdks/typescript
npm run build:wasm  # builds crates/tenor-eval-wasm with wasm-pack
npm run build       # compiles TypeScript to dist/

# 3. Run tests
npm test
```

The `scripts/build-wasm.sh` script runs:
```bash
cd crates/tenor-eval-wasm && wasm-pack build --target nodejs --out-dir ../../sdks/typescript/wasm
```

## License

MIT
