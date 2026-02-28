# Tenor

**A behavioral contract calculus.**

Tenor is a finite, stratified, verdict-producing formal system for describing the complete observable behavior of a system. The entire state space, all authority boundaries, and all possible verdict outcomes are statically derivable from the contract alone — without executing any implementation.

> Any agent that can read this specification can fully understand a system described in it, without reading any implementation code.

**Status:** v1.0 spec complete, three amendments integrated (Source Declarations, Multi-Instance Entities, Trust & Security). Elaborator, evaluator, static analyzer, migration engine, adapter framework, multi-instance entities, LLM-powered fact wiring, and full trust/signing test suite implemented and validated. 849 workspace + 107 conformance tests passing (956 total). Six domain contracts proven across real industries.

---

## Differentiation

Systems describe behavior across OpenAPI specs, policy YAML, RBAC configs, state machines, workflow engines, and implementation code. None of it is unified. None of it is fully legible. The fragmentation is real and worsening.

Tenor is a behavioral contract calculus. Not a smart contract language. Not a policy DSL. Not a workflow engine. A contract is the complete description of a system's observable behavior — statically analyzable, provenance-complete, agent-legible.

What no other implemented language combines:

- **Stratification is declared, not derived.** Termination is structural.
- **Provenance is part of the evaluation relation.** The audit log is a theorem.
- **No built-in functions.** Time, totals, classification — all Facts. Static analysis is complete, not approximate.
- **Flow is in the contract.** Every execution path is statically enumerable.
- **Static analyzability is a rejection filter.** Anything that breaks it is out.
- **Multi-contract composition.** System construct coordinates contracts with shared personas, cross-contract triggers, and entity relationships.
- **Source declarations.** Contracts declare where facts come from — protocol, endpoint, schema — without embedding credentials or coupling to infrastructure.
- **Contract migration.** Contracts evolve safely with classified diffs, three-layer flow compatibility analysis, and atomic version transitions.

---

## Example

A contract governing escrow release with source declarations, stratified verdict logic, and a release flow.

```tenor
source escrow_service {
  protocol:    http
  base_url:    "https://api.escrow.com/v2"
  auth:        bearer_token
  schema_ref:  "https://api.escrow.com/v2/openapi.json"
  description: "Escrow account management API"
}

source delivery_service {
  protocol:    http
  base_url:    "https://api.delivery.com/v1"
  auth:        bearer_token
  description: "Delivery tracking service"
}

type LineItemRecord {
  id:     Text(max_length: 64)
  amount: Money(currency: "USD")
  valid:  Bool
}

fact escrow_amount {
  type:   Money(currency: "USD")
  source: escrow_service { path: "accounts.{id}.balance" }
}

fact delivery_status {
  type:   Enum(values: ["pending", "confirmed", "failed"])
  source: delivery_service { path: "shipments.{id}.status" }
}

fact line_items {
  type:   List(element_type: LineItemRecord, max: 100)
  source: "order_service.items"
}

entity EscrowAccount {
  states:  [held, released, refunded]
  initial: held
  transitions: [(held, released), (held, refunded)]
}

rule all_items_valid {
  stratum: 0
  when:    ∀ item ∈ line_items . item.valid = true
  produce: verdict items_validated { payload: Bool = true }
}

rule delivery_confirmed {
  stratum: 0
  when:    delivery_status = "confirmed"
  produce: verdict delivery_confirmed { payload: Bool = true }
}

rule can_release {
  stratum: 1
  when:    verdict_present(items_validated)
         ∧ verdict_present(delivery_confirmed)
  produce: verdict release_approved { payload: Bool = true }
}

operation release_escrow {
  allowed_personas: [escrow_agent]
  precondition:     verdict_present(release_approved)
  effects:          [(EscrowAccount, held, released)]
  error_contract:   [precondition_failed, persona_rejected]
}

flow release {
  snapshot: at_initiation
  entry:    step_release

  steps: {
    step_release: OperationStep {
      op:      release_escrow
      persona: escrow_agent
      outcomes: {
        success: Terminal(success)
      }
      on_failure: Terminate(outcome: failure)
    }
  }
}
```

From this contract alone a static analyzer can derive: every reachable entity state, every persona's authority in every state, every verdict the rules can produce, every execution path through every flow, and the complete provenance chain for any outcome. The source declarations tell adapters where to fetch facts — without embedding secrets or coupling to infrastructure. No implementation required.

---

## Constraints

Non-negotiable. Any proposed feature that violates them is rejected regardless of ergonomic benefit.

|        | Constraint                                                                                                                     |
| ------ | ------------------------------------------------------------------------------------------------------------------------------ |
| **C1** | **Decidability.** Non-Turing complete by design.                                                                               |
| **C2** | **Termination.** Evaluation terminates for all valid contracts — a structural guarantee, not a runtime check.                  |
| **C3** | **Determinism.** Identical inputs produce identical outputs across all conforming implementations.                             |
| **C4** | **Static analyzability.** Complete state space derivable without execution.                                                    |
| **C5** | **Closed-world semantics.** The contract is the complete system description. No implicit behaviors.                            |
| **C6** | **Explicit over implicit.** No authority, propagation, or evaluation order is inferred. Everything is declared.                |
| **C7** | **Provenance as semantics.** Every value carries its derivation. The audit log is a theorem derived from the evaluation trace. |

---

## Constructs

Fourteen constructs across three layers.

**Semantic layer** — dependency order:

| Construct               | Purpose                                                                                                           |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **BaseType**            | Closed value type set: Bool, Int, Decimal, Text, Enum, Date, DateTime, Money, Record, TaggedUnion, List, Duration |
| **Fact**                | Ground typed assertions from external sources — the evaluation root and provenance origin                         |
| **Entity**              | Finite state machines in a static DAG                                                                             |
| **Rule**                | Stratified verdict-producing evaluation functions                                                                 |
| **Persona**             | Declared identity tokens for authority gating                                                                     |
| **Operation**           | Persona-gated, precondition-guarded state transitions with declared outcomes                                      |
| **PredicateExpression** | Quantifier-free FOL with arithmetic and bounded quantification over List-typed facts                              |
| **Flow**                | Finite DAG orchestration of Operations with sequential, branching, handoff, sub-flow, and parallel steps          |
| **NumericModel**        | Fixed-point decimal arithmetic with total promotion rules (cross-cutting)                                         |

**Composition layer:**

| Construct  | Purpose                                                                                            |
| ---------- | -------------------------------------------------------------------------------------------------- |
| **System** | Multi-contract composition with shared personas, cross-contract triggers, and entity relationships |

**Infrastructure layer:**

| Construct  | Purpose                                                                                                    |
| ---------- | ---------------------------------------------------------------------------------------------------------- |
| **Source** | Declarative external data source bindings — protocol, endpoint, schema reference — invisible to evaluation |

**Tooling layer:**

| Artifact             | Purpose                                                                                  |
| -------------------- | ---------------------------------------------------------------------------------------- |
| **ElaboratorSpec**   | Transforms `.tenor` source into a canonical JSON bundle through six deterministic passes |
| **TenorInterchange** | Canonical JSON bundle — the single source of truth for all downstream tooling            |

Named type aliases (TypeDecl) are a DSL-layer convenience. The elaborator resolves all named type references during Pass 3 and inlines the full BaseType structure at every point of use. TypeDecl does not appear in interchange output.

---

## Evaluation

```
Read path:     assemble_facts -> eval_strata -> resolve -> ResolvedVerdictSet
Write path:    execute(op, persona, verdict_set, (entity_id, instance_id)) -> EntityState' | Error
Orchestration: execute_flow(flow, persona, snapshot, instance_bindings) -> FlowOutcome
               execute_parallel(branches, snapshot) -> {BranchId -> BranchOutcome}
               evaluate_join(join_policy, branch_outcomes) -> StepTarget
```

Every step is bounded, deterministic, and statically analyzable. The evaluator (`tenor eval`) is fully implemented — it evaluates contracts against fact sets and produces verdict outcomes with complete provenance traces.

---

## Migration

Contracts evolve. The migration engine classifies every change between two contract versions:

- **BREAKING** — may invalidate existing entity state or in-flight flows
- **REQUIRES_ANALYSIS** — treated as breaking unless proven otherwise
- **INFRASTRUCTURE** — source or tooling changes, no evaluation impact
- **NON_BREAKING** — additive, safe to deploy

The three-layer flow compatibility checker analyzes in-flight flows per-instance: Layer 1 (verdict isolation), Layer 2 (entity state equivalence), Layer 3 (operation/flow structure), with short-circuit on first failure. Migration executes atomically — all entity state transitions, flow terminations, and version swaps commit together or all roll back.

```bash
# Analyze migration impact
tenor migrate v1.json v2.json

# Skip confirmation prompt
tenor migrate v1.json v2.json --yes
```

---

## Source Declarations & Adapters

Contracts declare where facts come from. Adapters fetch them.

```tenor
source order_service {
  protocol:    http
  base_url:    "https://api.orders.com/v2"
  auth:        bearer_token
  schema_ref:  "https://api.orders.com/v2/openapi.json"
}

fact escrow_amount {
  type:   Money(currency: "USD")
  source: order_service { path: "orders.{id}.balance" }
}
```

The elaborator validates source declarations (C-SRC-01 through C-SRC-06) without connecting to anything. At runtime, adapters resolve structured sources to live data with enriched provenance — tracing fact values back through the adapter, the fetch timestamp, and the external system.

Six core protocols: `http`, `database`, `graphql`, `grpc`, `static`, `manual`. Extension protocols via `x_*` namespace.

---

## tenor connect

LLM-powered fact wiring. Given a contract and an environment (OpenAPI spec, GraphQL SDL, SQL DDL), `tenor connect` proposes fact-to-source mappings and generates adapter configurations.

```bash
# Interactive mode — review each mapping
tenor connect escrow.tenor --environment openapi.json

# Batch mode — output review file for offline editing
tenor connect escrow.tenor --environment openapi.json --batch

# Apply reviewed mappings
tenor connect --apply tenor-connect-review.toml

# Heuristic matching (no LLM required)
tenor connect escrow.tenor --environment openapi.json --heuristic
```

---

## Static Analysis

The `tenor check` command runs eight static analysis checks on any `.tenor` file:

| Check | Analysis                | What it finds                                     |
| ----- | ----------------------- | ------------------------------------------------- |
| S1    | State space enumeration | All entity states, total state count              |
| S2    | Reachability analysis   | Unreachable entity states                         |
| S3a   | Admissibility           | Operation admissibility across state combinations |
| S4    | Authority mapping       | Persona authority in every reachable state        |
| S5    | Verdict enumeration     | All producible verdicts and their dependencies    |
| S6    | Flow path enumeration   | All execution paths through all flows             |
| S7    | Complexity metrics      | Predicate depth, flow depth, branching factors    |
| S8    | Verdict uniqueness      | Duplicate or conflicting verdict productions      |

Cross-contract analysis extends S4 and S6 to System constructs, checking authority and trigger cycles across contract boundaries.

---

## Domain Contracts

Six validated domain contracts ship with the repository, each proven end-to-end through elaboration, evaluation, and static analysis:

| Domain             | Contract                        | Directory                     |
| ------------------ | ------------------------------- | ----------------------------- |
| SaaS               | Subscription lifecycle          | `domains/saas/`               |
| Healthcare         | Prior authorization workflow    | `domains/healthcare/`         |
| Supply Chain       | Goods inspection                | `domains/supply_chain/`       |
| Energy Procurement | RFP workflow                    | `domains/energy_procurement/` |
| Trade Finance      | Letter of credit                | `domains/trade_finance/`      |
| System Scenario    | Cross-contract trade inspection | `domains/system_scenario/`    |

The System scenario composes the supply chain and trade finance contracts via cross-contract triggers, demonstrating multi-contract coordination.

---

## Documentation

| Document                                      | Audience              |
| --------------------------------------------- | --------------------- |
| [Narrative](docs/guide/narrative.md)          | Everyone              |
| [Formal specification](docs/TENOR.md)         | Language implementors |
| [Author guide](docs/guide/author-guide.md)    | Contract authors      |
| [What is Tenor?](docs/guide/what-is-tenor.md) | Decision makers       |

---

## SDKs

| Language   | Directory          | Runtime                    | Install             |
| ---------- | ------------------ | -------------------------- | ------------------- |
| TypeScript | `sdks/typescript/` | WASM evaluator + HTTP client | `npm install`     |
| Python     | `sdks/python/`     | PyO3 native module         | `pip install`       |
| Go         | `sdks/go/`         | wazero WASM runtime        | `go get`            |

Cross-SDK conformance fixtures live in `sdks/conformance/` — every SDK is validated against the same test cases.

---

## Structure

```
docs/
  TENOR.md                -- full formal specification (v1.0, three amendments)
  guide/                  -- documentation for authors and decision makers
schema/
  tenor-interchange-v1.0.json  -- JSON Schema for interchange format
conformance/              -- elaborator conformance suite (103 tests)
  positive/               -- valid DSL -> expected interchange JSON
  negative/               -- invalid DSL -> expected error JSON
  numeric/                -- decimal/money precision fixtures
  promotion/              -- numeric type promotion fixtures
  shorthand/              -- DSL shorthand expansion fixtures
  cross_file/             -- multi-file import fixtures
  parallel/               -- parallel entity conflict fixtures
  analysis/               -- static analysis fixtures
  eval/                   -- evaluator fixtures
  manifest/               -- manifest-based test fixtures
  ambiguity/              -- AI ambiguity testing fixtures
domains/                  -- validated domain contracts
sdks/
  typescript/             -- TypeScript SDK (WASM evaluator + HTTP client)
  python/                 -- Python SDK (PyO3 native module)
  go/                     -- Go SDK (wazero WASM runtime)
  conformance/            -- cross-SDK conformance test fixtures
builder/                  -- Tenor Builder SPA (visual contract editor)
crates/
  core/                   -- library: elaboration pipeline (6-pass)
  cli/                    -- binary: tenor command-line tool
  eval/                   -- library: contract evaluator + migration engine + adapter framework
  interchange/            -- library: interchange format types and serialization
  analyze/                -- library: static analysis (S1-S8)
  storage/                -- library: storage trait and conformance suite
  codegen/                -- library: code generation (scaffold)
  lsp/                    -- library: Language Server Protocol (scaffold)
  tenor-eval-wasm/        -- library: WASM evaluator for browsers and edge
  executor-conformance/   -- library: executor conformance suite
```

---

## Build

```bash
# Build all crates
cargo build --workspace

# Run conformance suite (103 tests)
cargo run -p tenor-cli -- test conformance

# Run all tests (849 workspace + 107 conformance)
cargo test --workspace
```

## CLI

25 subcommands. Run `tenor --help` for full details.

```bash
# Elaboration & validation
tenor elaborate file.tenor              # Elaborate .tenor to interchange JSON
tenor elaborate --manifest file.tenor   # Generate TenorManifest with interchange bundle
tenor validate bundle.json              # Validate interchange JSON against schema
tenor check file.tenor                  # Run static analysis (S1-S8)
tenor diff v1.json v2.json              # Diff two interchange bundles
tenor diff v1.json v2.json --breaking   # Classify changes as breaking/non-breaking
tenor explain file.tenor                # Explain contract in natural language

# Evaluation & execution
tenor eval bundle.json --facts facts.json                  # Evaluate contract against facts
tenor eval bundle.json --facts facts.json --flow release   # Execute a flow
tenor migrate v1.json v2.json                              # Analyze migration between versions
tenor serve --port 8080 contract.tenor                     # Start HTTP API server
tenor agent file.tenor                                     # Start interactive agent shell

# Source wiring
tenor connect file.tenor --environment openapi.json             # LLM-powered fact wiring
tenor connect file.tenor --environment openapi.json --heuristic # Heuristic matching (no LLM)
tenor connect --apply tenor-connect-review.toml                 # Apply reviewed mappings

# Code generation & UI
tenor generate typescript file.tenor --out ./generated   # Generate TypeScript bindings
tenor ui contract.tenor --out ./tenor-ui                 # Generate React application
tenor builder                                            # Start Builder SPA dev server

# Template registry
tenor pack                        # Package contract template
tenor publish --token TOKEN       # Publish to registry
tenor search "query"              # Search templates
tenor install template-name       # Install template
tenor deploy template-name        # Deploy to hosted platform

# Trust & signing
tenor keygen                             # Generate Ed25519 signing keypair
tenor sign bundle.json --key secret.key [--out signed.json]  # Sign interchange bundle
tenor verify bundle.signed.json          # Verify signed bundle
tenor sign-wasm eval.wasm --key secret.key --bundle-etag ETAG  # Sign WASM binary
tenor verify-wasm eval.wasm --sig eval.sig --pubkey key.pub    # Verify WASM binary

# Tooling
tenor test conformance    # Run conformance suite
tenor lsp                 # Start Language Server Protocol server
tenor ambiguity suite/    # Run AI ambiguity testing
```

---

## License

Apache 2.0. Copyright 2026 Riverline Labs.
