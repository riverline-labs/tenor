# Phase 8: Automatic UI — Complete Implementation

A complete, themed React application is generated from any Tenor contract. The contract declares every entity, every state, every operation, every persona, every flow, and every fact. That's a complete UI specification. This phase generates a working web application from the interchange bundle alone — no additional configuration, no custom code.

**Repo:** Public only (`~/src/riverline/tenor`). The generated UI calls the executor's HTTP API (platform-serve endpoints) but is itself a static frontend artifact.

---

## What "done" means

1. `tenor ui <contract>` generates a complete React application from a contract
2. The UI shows: current entity states, available actions per persona, blocked actions with reasons, fact inputs, flow execution, execution history, provenance drill-down
3. The UI is themed and professional — not a dev tool wireframe
4. The UI works against any executor's HTTP API (the endpoints are spec-defined: §19)
5. Persona selection, entity instance browsing, multi-instance support
6. The generated app is a standalone SPA — deployable to any static hosting
7. Works for ANY valid Tenor contract, not just the escrow example

---

## Step 1: UI architecture

### 1A: Generation approach

`tenor ui` reads the interchange bundle and generates a React application. Two approaches — choose one:

**Option A — Static generation:** Generate `.tsx` files, `package.json`, and build scripts. The user runs `npm install && npm run build` to get a deployable SPA. Pro: fully customizable. Con: requires Node.js toolchain.

**Option B — Runtime interpretation:** Generate a single HTML file with an embedded React app that reads the interchange bundle at runtime. The bundle is either inlined or fetched from the executor's `/.well-known/tenor` endpoint. Pro: zero build step. Con: less customizable.

**Choose Option A** for this phase. The generated code is reviewable, editable, and can be customized by the user. This is more valuable for a production tool.

### 1B: Generated project structure

```
<output-dir>/
  package.json
  tsconfig.json
  vite.config.ts        — or similar bundler config
  src/
    App.tsx             — main app with routing
    api.ts              — executor HTTP client (spec-defined endpoints)
    types.ts            — TypeScript types from interchange bundle
    theme.ts            — theming configuration
    components/
      Layout.tsx        — app shell, navigation, persona selector
      Dashboard.tsx     — overview: entity states, action summary, verdicts
      EntityList.tsx    — list all entity types with instance counts
      EntityDetail.tsx  — single entity type: instances, states, transitions
      InstanceDetail.tsx — single instance: state, history, available actions
      ActionSpace.tsx   — available actions per persona with execute buttons
      BlockedActions.tsx — blocked actions with reasons and unblock hints
      FactInput.tsx     — fact value input form (type-aware: Money, Date, Bool, etc.)
      FlowExecution.tsx — flow execution UI: select persona, bind instances, execute
      FlowHistory.tsx   — execution history with timeline
      ProvenanceDrill.tsx — provenance chain visualization (verdict → rule → facts)
      VerdictDisplay.tsx — current verdicts with stratum hierarchy
    hooks/
      useActionSpace.ts — poll/fetch action space
      useEntities.ts    — entity state management
      useExecution.ts   — flow execution + result handling
  public/
    index.html
  README.md
```

### 1C: Executor API client

The API client calls the spec-defined HTTP endpoints (§19, platform-serve):

```typescript
class TenorClient {
  constructor(baseUrl: string, contractId: string);

  async getManifest(): Promise<TenorManifest>;
  async getActionSpace(persona: string, facts: FactSet): Promise<ActionSpace>;
  async executeFlow(
    flowId: string,
    persona: string,
    facts: FactSet,
    instanceBindings: InstanceBindingMap,
  ): Promise<FlowResult>;
  async simulateFlow(
    flowId: string,
    persona: string,
    facts: FactSet,
    instanceBindings: InstanceBindingMap,
  ): Promise<FlowResult>;
  async getEntityInstances(entityId: string): Promise<EntityInstance[]>;
  async getEntityState(entityId: string, instanceId: string): Promise<string>;
  async initializeEntity(entityId: string, instanceId: string): Promise<void>;
  async getExecutionHistory(): Promise<FlowExecution[]>;
  async getExecution(executionId: string): Promise<FlowExecution>;
}
```

---

## Step 2: Contract-driven UI generation

### 2A: Type generation from interchange bundle

Read the interchange bundle and generate TypeScript types for:

- Every entity type with its states (as string union types)
- Every fact with its type (mapped to TypeScript: `Money` → `{ amount: number, currency: string }`, etc.)
- Every persona (as string union)
- Every operation with its personas, precondition summary, effects
- Every flow with its steps, entry point, terminal outcomes

```typescript
// Generated from interchange bundle
export type EscrowAccountState = "pending" | "active" | "released" | "refunded";
export type DeliveryRecordState =
  | "awaiting_shipment"
  | "shipped"
  | "delivered"
  | "failed";

export type Persona =
  | "buyer"
  | "seller"
  | "compliance_officer"
  | "escrow_agent";

export interface Facts {
  escrow_amount: { amount: number; currency: string };
  delivery_status: "pending" | "confirmed" | "failed";
  compliance_threshold: { amount: number; currency: string };
  // ...
}
```

### 2B: Dashboard generation

The dashboard shows:

- **Entity overview:** Each entity type as a card, showing instance count and state distribution (e.g., "Order: 5 instances — 2 draft, 2 submitted, 1 approved")
- **Action summary:** Count of available vs blocked actions for the selected persona
- **Verdict summary:** Key verdicts and their current values
- **Recent activity:** Last N executions with outcome

### 2C: Action space UI

The action space view is the core of the UI:

- Select a persona from the contract's declared personas
- See all available actions (flow_id, applicable instances, enabling verdicts)
- See all blocked actions with specific reasons (missing verdict, wrong entity state, unauthorized persona)
- For each available action: "Execute" and "Simulate" buttons
- Instance binding selector: when a flow targets multiple entity types, dropdowns to select which instance

### 2D: Fact input

Generate type-aware input components for each fact:

| Fact Type | Input Component                  |
| --------- | -------------------------------- |
| Bool      | Toggle switch                    |
| Int       | Number input (integer only)      |
| Decimal   | Number input (decimal)           |
| Text      | Text input                       |
| Date      | Date picker                      |
| DateTime  | DateTime picker                  |
| Money     | Amount input + currency selector |
| Enum      | Dropdown with declared values    |
| List      | Multi-item input with add/remove |
| Record    | Grouped fields per record schema |

The fact input form is used before evaluating the action space — facts drive verdicts, verdicts drive actions.

### 2E: Flow execution UI

When the user clicks "Execute":

1. Confirm persona, instance bindings, and current facts
2. Call the executor's execute endpoint
3. Show result: success with state transitions, or failure with error
4. Show provenance: which verdicts enabled the operation, which facts those verdicts examined

When the user clicks "Simulate":

1. Same flow but calls the simulate endpoint
2. Response carries `simulation: true`
3. UI clearly marks the result as simulated (visual indicator)
4. No state changes — UI can show "would transition Order/ord-001 from draft to submitted"

### 2F: Provenance drill-down

When the user clicks on a verdict or execution result:

- Show the provenance chain: verdict → rule → facts examined → fact values
- For operation execution: operation → persona → precondition verdicts → verdicts → facts
- Visual tree or chain representation
- Click any node to drill deeper

### 2G: Entity instance browser

- List all instances of an entity type
- Show current state for each instance
- Show state machine diagram (declared states and transitions)
- Highlight current state in the diagram
- Show which operations can transition this instance from its current state

---

## Step 3: Theming

### 3A: Default theme

Generate a clean, professional default theme. Not Material UI, not Bootstrap — something that looks like it was designed, not templated.

- Color palette derived from the contract name (hash to HSL for primary color, derive secondary and accent)
- Clean typography (system font stack)
- Card-based layout with good whitespace
- Status colors: green for available, amber for blocked, red for errors, blue for info
- Responsive: works on desktop and tablet

### 3B: Theme customization

The generated `theme.ts` is easily customizable:

```typescript
export const theme = {
  colors: {
    primary: "#2563eb",
    secondary: "#64748b",
    success: "#16a34a",
    warning: "#d97706",
    error: "#dc2626",
    background: "#f8fafc",
  },
  fonts: {
    body: "system-ui, -apple-system, sans-serif",
    heading: "system-ui, -apple-system, sans-serif",
    mono: "ui-monospace, monospace",
  },
  // ...
};
```

---

## Step 4: CLI command

```
tenor ui <contract> [OPTIONS]

OPTIONS:
  --output <dir>           Output directory (default: ./tenor-ui/)
  --api-url <url>          Executor API base URL (default: http://localhost:3000)
  --contract-id <id>       Contract ID for multi-contract executors
  --theme <file>           Custom theme file (JSON)
  --title <string>         Application title (default: contract id)
```

The command:

1. Reads the contract (`.tenor` file or `.json` interchange bundle)
2. Elaborates if needed (`.tenor` → interchange)
3. Generates the React application
4. Reports: "UI generated at ./tenor-ui/. Run `cd tenor-ui && npm install && npm run dev` to start."

---

## Step 5: Tests

- Test: generate UI from escrow contract → all files created, correct entity types, correct fact types
- Test: generate UI from a minimal contract (one entity, one operation) → works
- Test: generate UI from a multi-entity, multi-flow contract → works
- Test: generated TypeScript compiles without errors (`tsc --noEmit`)
- Test: generated app builds without errors (`npm run build`)
- Test: fact input components match declared fact types
- Test: persona list matches declared personas
- Test: entity states match declared states

---

## Final Report

```
## Phase 8: Automatic UI — COMPLETE

### Generation
- CLI: `tenor ui` generates React app from any contract
- Components: Dashboard, EntityList, EntityDetail, InstanceDetail, ActionSpace, BlockedActions, FactInput, FlowExecution, FlowHistory, ProvenanceDrill, VerdictDisplay
- Type generation: entities, facts, personas, operations, flows
- API client: spec-compliant executor HTTP client

### Features
- Persona selection with action space per persona
- Entity instance browser with state machine visualization
- Type-aware fact input (Money, Date, Enum, etc.)
- Flow execution and simulation with provenance drill-down
- Blocked actions with reasons and unblock hints
- Multi-instance support with instance binding selector

### Theming
- Default theme: contract-derived color palette
- Customizable via theme.ts

### Tests
- [N] generation tests
- Generated TypeScript compiles
- Generated app builds

### Commits
- [hash] [message]
- ...
```

Phase 8 is done when `tenor ui` generates a complete, themed, working React application from any valid contract, and every checkbox above is checked. Not before.
