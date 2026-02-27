# Phase 9: Builder — Complete Implementation

Users author, visualize, simulate, and export contracts without writing DSL by hand. The Builder is a web application that provides a visual contract editor with real-time validation, state machine visualization, flow DAG editing, and one-click simulation.

**Repo:** Public only (`~/src/riverline/tenor`). The Builder generates `.tenor` files and interchange bundles — it's a frontend for the elaborator.

---

## What "done" means

1. Web application where users can create a Tenor contract from scratch
2. Visual editors for: entities (state machines), facts (typed inputs), rules (stratum-aware), operations (persona-gated, precondition-guarded), flows (DAG editor), personas, sources
3. Real-time elaboration: every change runs the elaborator and shows errors inline
4. State machine visualization: entity states and transitions rendered as interactive diagrams
5. Flow DAG visualization: flow steps rendered as interactive directed graphs
6. Simulation: run evaluation with test facts, see verdicts and action space, execute test flows
7. Export: download `.tenor` source files and/or interchange bundle JSON
8. Import: upload existing `.tenor` files or interchange bundles to edit

---

## Step 1: Architecture

### 1A: Application structure

The Builder is a React SPA that runs the WASM evaluator in the browser. No server required for authoring and simulation — the elaborator runs client-side via WASM.

```
builder/
  package.json
  vite.config.ts
  src/
    App.tsx
    store/              — state management (React context or Zustand)
      contract.ts       — contract model (entities, facts, rules, operations, flows, personas, sources)
      elaboration.ts    — WASM elaborator integration
      simulation.ts     — simulation state
    components/
      Layout.tsx        — app shell, sidebar navigation, toolbar
      ContractOverview.tsx — high-level contract summary
      editors/
        EntityEditor.tsx     — state machine editor (add states, transitions, initial state)
        FactEditor.tsx       — fact type editor (type picker, source, default)
        RuleEditor.tsx       — rule editor (stratum, condition, produce verdict)
        OperationEditor.tsx  — operation editor (personas, precondition, effects, outcomes)
        FlowEditor.tsx       — flow DAG editor (steps, routing, branches, joins)
        PersonaEditor.tsx    — persona list editor
        SourceEditor.tsx     — source declaration editor (protocol, fields)
        SystemEditor.tsx     — system composition editor
      visualizations/
        StateMachine.tsx     — interactive entity state diagram (SVG/Canvas)
        FlowDag.tsx          — interactive flow step DAG (SVG/Canvas)
        StratumView.tsx      — rule stratum hierarchy visualization
        AuthorityMatrix.tsx  — persona × operation authorization matrix
      simulation/
        FactInputPanel.tsx   — input test fact values
        VerdictPanel.tsx     — show computed verdicts
        ActionSpacePanel.tsx — show available/blocked actions
        FlowRunner.tsx       — step through flow execution
        ProvenanceView.tsx   — provenance chain for simulation results
      shared/
        PredicateBuilder.tsx — visual predicate expression builder
        TypePicker.tsx       — BaseType selector with nested types
        ErrorPanel.tsx       — real-time elaboration errors
        ExportDialog.tsx     — export options (DSL, JSON, WASM)
        ImportDialog.tsx     — import from file
    wasm/
      evaluator.ts      — WASM evaluator wrapper
      elaborator.ts     — WASM elaborator wrapper (if available) or DSL-to-JSON client-side
    utils/
      dsl-generator.ts  — generate .tenor DSL from contract model
      layout.ts         — graph layout algorithms for DAG/state machine rendering
  public/
    index.html
```

### 1B: WASM integration

The Builder needs two WASM capabilities:

1. **Evaluator** — already exists (`tenor-eval-wasm`). Used for simulation.
2. **Elaborator** — may need a WASM build of the elaborator for client-side `.tenor` → interchange conversion. If this doesn't exist yet, build it. Alternatively, the Builder can work directly with the interchange JSON model and generate DSL as an export step.

**Recommended approach:** The Builder's internal model is the interchange JSON structure. The DSL is generated from this model at export time. This avoids needing a WASM elaborator — the Builder IS the elaborator for visual editing. Real-time validation uses the WASM evaluator to check that the generated interchange is valid.

---

## Step 2: Entity editor

### 2A: State machine editor

Visual editor for entity state machines:

- Canvas/SVG showing states as nodes and transitions as directed edges
- Click to add a state (prompt for name)
- Drag between states to add a transition
- Click a state to set it as initial (highlighted differently)
- Delete states and transitions
- Validation: initial state must be set, at least one state required, no orphan states

### 2B: State machine rendering

Use a force-directed or hierarchical layout algorithm:

- States as rounded rectangles with the state name
- Initial state has a distinct visual indicator (double border or entry arrow)
- Transitions as curved arrows with hover labels
- Interactive: pan, zoom, drag to rearrange
- Auto-layout on change

---

## Step 3: Fact editor

- List of facts with inline editing
- Type picker: dropdown for BaseType (Bool, Int, Decimal, Text, Date, DateTime, Money, Enum, List, Record, TaggedUnion)
- For parameterized types (Money, Enum, List, Record): sub-fields appear when selected
  - Money: currency parameter
  - Enum: list of values (add/remove)
  - List: element type + max
  - Record: field list with types
- Source: dropdown of declared sources (or freetext)
- For structured source: source_id selector + path input
- Default value: type-appropriate input

---

## Step 4: Rule editor

- List of rules organized by stratum (visual stratum hierarchy)
- Each rule: condition (predicate expression), produce (verdict type)
- Predicate builder: visual expression builder for conditions
  - Operand selectors: facts (from declared facts), verdicts (from lower strata)
  - Operators: comparison, logical (and, or, not), quantifiers
  - Type-aware: only valid comparisons for the selected fact type
- Stratum assignment: automatic (based on dependencies) or manual
- Validation: no same-stratum dependencies, no cycles

---

## Step 5: Operation editor

- Each operation: name, personas (multi-select from declared personas), precondition, effects, outcomes
- Precondition: predicate builder (same as rule conditions but over verdicts)
- Effects: entity type selector + from_state → to_state transition selector (filtered to declared transitions)
- Multi-outcome: add outcome labels, associate effects with outcomes
- Validation: personas non-empty, effects reference valid entity transitions, outcomes non-empty

---

## Step 6: Flow editor

### 6A: DAG editor

Visual DAG editor for flow steps:

- Canvas showing steps as nodes, transitions as directed edges
- Step types: OperationStep, BranchStep, ParallelStep, SubFlowStep
- Drag to connect steps (routing)
- OperationStep: select operation, persona, outcome routing
- BranchStep: add branches with conditions (predicate builder)
- ParallelStep: add branches (each branch is a sub-DAG)
- Entry step highlighted
- Terminal outcomes shown as end nodes
- Validation: all operation outcomes handled, DAG is acyclic, entry step exists

### 6B: Flow DAG rendering

- Topological layout (left-to-right or top-to-bottom)
- Steps as cards with type icon and operation/condition summary
- Edges with outcome labels
- Parallel steps shown as swim lanes
- Branch steps shown as diamond decision nodes
- Interactive: click step to edit, drag to reorder

---

## Step 7: Simulation mode

### 7A: Fact input

- Panel showing all declared facts with type-appropriate inputs
- Fill in test values
- "Evaluate" button: runs WASM evaluator with current facts and entity states

### 7B: Verdict display

- Show all computed verdicts organized by stratum
- Green/red for true/false
- Click a verdict to see its provenance (which rule, which facts)

### 7C: Action space display

- Select a persona
- Show available actions with enabling verdicts
- Show blocked actions with reasons
- "Simulate" button on each action: dry-run the flow with current state

### 7D: Flow stepping

- Select a flow and set instance bindings
- Step through execution one step at a time
- Show current step highlighted in the DAG visualization
- Show state changes at each step
- Show provenance for each step's operation execution

---

## Step 8: Import/Export

### 8A: Export

- **Export as .tenor:** Generate DSL source from the internal model. The DSL generator must produce valid, elaboratable `.tenor` files.
- **Export as JSON:** Export the interchange bundle directly.
- **Export as WASM:** Compile the contract to WASM evaluator (calls `tenor compile --wasm` or equivalent).
- **Download all:** ZIP with `.tenor` source, interchange JSON, and optional WASM binary.

### 8B: Import

- **Import .tenor files:** Parse and load into the Builder's model. Requires a parser — either WASM elaborator or a client-side DSL parser.
- **Import interchange JSON:** Load directly into the model (this is the native format).
- **Import from URL:** Fetch `/.well-known/tenor` from a running executor, extract the bundle.

---

## Step 9: CLI command

```
tenor builder [OPTIONS]

OPTIONS:
  --port <port>            Dev server port (default: 5173)
  --open                   Open browser automatically
  --contract <file>        Pre-load a contract
```

Also:

```
tenor builder build [OPTIONS]

OPTIONS:
  --output <dir>           Build output directory (default: ./builder-dist/)
```

---

## Step 10: Tests

- Test: create a contract from scratch in the model → export as .tenor → elaborate → no errors
- Test: import escrow contract → model matches interchange bundle
- Test: entity editor CRUD: add state, remove state, add transition, set initial
- Test: fact editor: all BaseTypes create correct interchange representation
- Test: rule editor: stratum ordering correct, predicate expressions valid
- Test: flow editor: DAG is acyclic, outcomes handled
- Test: simulation: evaluate with known facts → expected verdicts
- Test: DSL generator produces valid .tenor from model
- Test: generated app builds (`npm run build`)

---

## Final Report

```
## Phase 9: Builder — COMPLETE

### Application
- React SPA with WASM evaluator
- Visual editors: Entity, Fact, Rule, Operation, Flow, Persona, Source
- Visualizations: State machine diagrams, Flow DAGs, Stratum hierarchy, Authority matrix
- Simulation: Fact input, verdict display, action space, flow stepping with provenance
- Import/Export: .tenor DSL, interchange JSON, WASM, URL import

### Components
- Entity editor: interactive state machine canvas
- Flow editor: DAG editor with step types
- Predicate builder: visual expression construction
- Simulation mode: evaluate + step-through

### Tests
- [N] model/generation tests
- App builds
- DSL generation produces valid .tenor

### Commits
- [hash] [message]
- ...
```

Phase 9 is done when the Builder can author, visualize, simulate, and export any valid Tenor contract without writing DSL by hand, and every checkbox above is checked. Not before.
