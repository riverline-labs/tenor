# @tenor-lang/sdk

TypeScript SDK for Tenor contract evaluation -- client to the Rust evaluator.

## Architecture

The SDK is a **client**. The Rust evaluator is the **trusted core**.

All contract evaluation, rule execution, and flow processing happens in the Rust evaluator. The SDK sends facts over HTTP and receives verdicts. It never reimplements evaluation logic. This means:

- The evaluator is the single source of truth for contract semantics
- The SDK cannot produce incorrect verdicts -- it only relays what the evaluator produces
- You can audit the evaluator independently of any SDK consumer

```
Your App  -->  @tenor-lang/sdk  -->  HTTP  -->  tenor serve (Rust)
               (client)                         (trusted evaluator)
```

## Getting Started

You need a running Tenor evaluator. Two options:

### Option A: `tenor serve` (if you have the Rust toolchain)

```bash
cargo install tenor-cli
tenor serve --port 8080 path/to/contract.tenor
```

### Option B: Docker (no Rust needed)

```bash
docker run -p 8080:8080 -v ./contracts:/contracts tenor/evaluator /contracts/my_contract.tenor
```

Or with docker-compose for local development:

```bash
docker compose up
```

## Installation

```bash
npm install @tenor-lang/sdk
```

## Quick Start

```typescript
import { TenorClient } from '@tenor-lang/sdk';

const client = new TenorClient({ baseUrl: 'http://localhost:8080' });

// List available contracts
const contracts = await client.listContracts();

// Get operations an agent can perform
const ops = await client.getOperations('my_contract');

// Evaluate the contract with facts
const result = await client.invoke('my_contract', {
  is_active: true,
  balance: { amount: '5000.00', currency: 'USD' }
});

// Get a natural-language explanation
const explanation = await client.explain('my_contract');
```

## Agent Skills

The SDK provides three core skills for agent integration:

### getOperations -- "What can I do?"

Returns the list of operations an agent can invoke, including which personas are allowed and what entity state transitions each operation performs.

```typescript
const operations = await client.getOperations('saas_subscription');

for (const op of operations) {
  console.log(`${op.id}:`);
  console.log(`  Personas: ${op.allowed_personas.join(', ')}`);
  for (const effect of op.effects) {
    console.log(`  ${effect.entity_id}: ${effect.from} -> ${effect.to}`);
  }
}
```

### invoke -- "Execute the contract"

Evaluates facts against the contract's rules and returns verdicts with full provenance. Each verdict includes which rule produced it, at which stratum, and which facts were used.

```typescript
// Rule-only evaluation
const result = await client.invoke('saas_subscription', {
  current_seat_count: 5,
  subscription_plan: 'professional',
  plan_features: {
    max_seats: 50,
    api_access: true,
    sso_enabled: true,
    custom_branding: false
  },
  payment_ok: true,
  account_age_days: 365,
  cancellation_requested: false
});

for (const verdict of result.verdicts) {
  console.log(`${verdict.type}: ${JSON.stringify(verdict.payload)}`);
  console.log(`  Rule: ${verdict.provenance.rule} (stratum ${verdict.provenance.stratum})`);
}
```

Optionally execute a flow by providing a `flow_id`:

```typescript
// Flow evaluation
const flowResult = await client.invoke('saas_subscription', facts, {
  flow_id: 'subscription_lifecycle',
  persona: 'billing_system'
});
```

### explain -- "What does this contract do?"

Returns a human-readable summary of the contract's structure, rules, operations, and flows.

```typescript
const explanation = await client.explain('saas_subscription');
console.log(explanation.summary);  // Brief overview
console.log(explanation.verbose);  // Detailed breakdown
```

## API Reference

### `new TenorClient(options?)`

Create a new client instance.

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `baseUrl` | `string` | `http://localhost:8080` | Base URL of the tenor serve instance |
| `timeout` | `number` | `30000` | Request timeout in milliseconds |

### `client.health()`

Check if the evaluator is reachable.

Returns `Promise<HealthResponse>` with `{ status: string, tenor_version: string }`.

### `client.listContracts()`

List all loaded contracts.

Returns `Promise<ContractSummary[]>` where each summary includes `id`, `construct_count`, `facts`, `operations`, and `flows`.

### `client.getOperations(contractId)`

Get operations available in a contract.

| Parameter | Type | Description |
|-----------|------|-------------|
| `contractId` | `string` | The contract identifier |

Returns `Promise<OperationInfo[]>` where each operation includes `id`, `allowed_personas`, `effects` (entity state transitions), and `preconditions_summary`.

### `client.invoke(contractId, facts, options?)`

Evaluate a contract against facts.

| Parameter | Type | Description |
|-----------|------|-------------|
| `contractId` | `string` | The contract identifier |
| `facts` | `Record<string, unknown>` | Fact values to evaluate |
| `options.flow_id` | `string` | Optional flow to execute |
| `options.persona` | `string` | Optional persona for flow execution |

Returns `Promise<EvalResult>` (rule-only) or `Promise<FlowEvalResult>` (with flow).

### `client.explain(contractId)`

Get a human-readable explanation of a contract.

| Parameter | Type | Description |
|-----------|------|-------------|
| `contractId` | `string` | The contract identifier |

Returns `Promise<ExplainResult>` with `{ summary: string, verbose: string }`.

### `client.elaborate(source, filename?)`

Elaborate .tenor source text into interchange JSON.

| Parameter | Type | Description |
|-----------|------|-------------|
| `source` | `string` | Tenor DSL source text |
| `filename` | `string` | Optional filename (default: `input.tenor`) |

Returns `Promise<InterchangeBundle>`.

## Error Handling

The SDK throws specific error types for different failure modes:

```typescript
import {
  TenorClient,
  ConnectionError,
  EvaluationError,
  ElaborationError,
  ContractNotFoundError
} from '@tenor-lang/sdk';

try {
  const result = await client.invoke('unknown_contract', {});
} catch (err) {
  if (err instanceof ContractNotFoundError) {
    console.log(`Contract '${err.contractId}' not found`);
  } else if (err instanceof EvaluationError) {
    console.log(`Evaluation failed: ${err.message}`);
    console.log('Details:', err.details);
  } else if (err instanceof ElaborationError) {
    console.log(`Elaboration failed: ${err.message}`);
  } else if (err instanceof ConnectionError) {
    console.log(`Cannot reach evaluator at ${err.url}`);
  }
}
```

All error classes extend `TenorError`, which extends `Error`.

## Configuration

The `TenorClientOptions` interface:

```typescript
interface TenorClientOptions {
  /** Base URL of the tenor serve instance. Default: http://localhost:8080 */
  baseUrl?: string;
  /** Request timeout in milliseconds. Default: 30000 */
  timeout?: number;
}
```

## Requirements

- **Node.js 22+** (uses built-in `fetch` -- no runtime dependencies)
- A running `tenor serve` instance (via Rust toolchain or Docker)
