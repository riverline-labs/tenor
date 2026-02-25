# Tenor

**A behavioral contract calculus.**

Tenor is a finite, stratified, verdict-producing formal system for describing the complete observable behavior of a system. The entire state space, all authority boundaries, and all possible verdict outcomes are statically derivable from the contract alone -- without executing any implementation.

> Any agent that can read this specification can fully understand a system described in it, without reading any implementation code.

**Status:** v1.0 complete. Elaborator, evaluator, and static analyzer implemented and validated. 72/72 conformance tests passing. Six domain contracts proven across real industries.

---

## Differentiation

Systems describe behavior across OpenAPI specs, policy YAML, RBAC configs, state machines, workflow engines, and implementation code. None of it is unified. None of it is fully legible. The fragmentation is real and worsening.

Tenor is a behavioral contract calculus. Not a smart contract language. Not a policy DSL. Not a workflow engine. A contract is the complete description of a system's observable behavior -- statically analyzable, provenance-complete, agent-legible.

What no other implemented language combines:

- **Stratification is declared, not derived.** Termination is structural.
- **Provenance is part of the evaluation relation.** The audit log is a theorem.
- **No built-in functions.** Time, totals, classification -- all Facts. Static analysis is complete, not approximate.
- **Flow is in the contract.** Every execution path is statically enumerable.
- **Static analyzability is a rejection filter.** Anything that breaks it is out.
- **Multi-contract composition.** System construct coordinates contracts with shared personas, cross-contract triggers, and entity relationships.

---

## Example

A contract governing escrow release. One entity, stratified verdict logic, and a release flow.

```tenor
type LineItemRecord {
  id:     Text(max_length: 64)
  amount: Money(currency: "USD")
  valid:  Bool
}

fact escrow_amount   {
  type:   Money(currency: "USD")
  source: "escrow_service.balance"
}

fact delivery_status {
  type:   Enum(values: ["pending", "confirmed", "failed"])
  source: "delivery_service.status"
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

From this contract alone a static analyzer can derive: every reachable entity state, every persona's authority in every state, every verdict the rules can produce, every execution path through every flow, and the complete provenance chain for any outcome. No implementation required.

---

## Constraints

Non-negotiable. Any proposed feature that violates them is rejected regardless of ergonomic benefit.

|        | Constraint                                                                                                                     |
| ------ | ------------------------------------------------------------------------------------------------------------------------------ |
| **C1** | **Decidability.** Non-Turing complete by design.                                                                               |
| **C2** | **Termination.** Evaluation terminates for all valid contracts -- a structural guarantee, not a runtime check.                  |
| **C3** | **Determinism.** Identical inputs produce identical outputs across all conforming implementations.                             |
| **C4** | **Static analyzability.** Complete state space derivable without execution.                                                    |
| **C5** | **Closed-world semantics.** The contract is the complete system description. No implicit behaviors.                            |
| **C6** | **Explicit over implicit.** No authority, propagation, or evaluation order is inferred. Everything is declared.                |
| **C7** | **Provenance as semantics.** Every value carries its derivation. The audit log is a theorem derived from the evaluation trace. |

---

## Constructs

Thirteen constructs across three layers.

**Semantic layer** -- dependency order:

| Construct               | Purpose                                                                                              |
| ----------------------- | ---------------------------------------------------------------------------------------------------- |
| **BaseType**            | Closed value type set: Bool, Int, Decimal, Text, Enum, Date, DateTime, Money, Record, TaggedUnion, List, Duration |
| **Fact**                | Ground typed assertions from external sources -- the evaluation root and provenance origin            |
| **Entity**              | Finite state machines in a static DAG                                                                |
| **Rule**                | Stratified verdict-producing evaluation functions                                                    |
| **Persona**             | Declared identity tokens for authority gating                                                        |
| **Operation**           | Persona-gated, precondition-guarded state transitions with declared outcomes                         |
| **PredicateExpression** | Quantifier-free FOL with arithmetic and bounded quantification over List-typed facts                 |
| **Flow**                | Finite DAG orchestration of Operations with sequential, branching, handoff, sub-flow, and parallel steps |
| **NumericModel**        | Fixed-point decimal arithmetic with total promotion rules (cross-cutting)                            |

**Composition layer:**

| Construct  | Purpose                                                                                   |
| ---------- | ----------------------------------------------------------------------------------------- |
| **System** | Multi-contract composition with shared personas, cross-contract triggers, and entity relationships |

**Tooling layer:**

| Artifact               | Purpose                                                                                  |
| ---------------------- | ---------------------------------------------------------------------------------------- |
| **ElaboratorSpec**     | Transforms `.tenor` source into a canonical JSON bundle through six deterministic passes  |
| **TenorInterchange**   | Canonical JSON bundle -- the single source of truth for all downstream tooling            |

Named type aliases (TypeDecl) are a DSL-layer convenience. The elaborator resolves all named type references during Pass 3 and inlines the full BaseType structure at every point of use. TypeDecl does not appear in interchange output.

---

## Evaluation

```
Read path:     assemble_facts -> eval_strata -> resolve -> ResolvedVerdictSet
Write path:    execute(op, persona, verdict_set) -> EntityState' | Error
Orchestration: execute_flow(flow, persona, snapshot) -> FlowOutcome
               execute_parallel(branches, snapshot) -> {BranchId -> BranchOutcome}
               evaluate_join(join_policy, branch_outcomes) -> StepTarget
```

Every step is bounded, deterministic, and statically analyzable. The evaluator (`tenor eval`) is fully implemented -- it evaluates contracts against fact sets and produces verdict outcomes with complete provenance traces.

---

## Static Analysis

The `tenor check` command runs eight static analysis checks on any `.tenor` file:

| Check | Analysis                       | What it finds                                    |
| ----- | ------------------------------ | ------------------------------------------------ |
| S1    | State space enumeration        | All entity states, total state count              |
| S2    | Reachability analysis          | Unreachable entity states                         |
| S3a   | Admissibility                  | Operation admissibility across state combinations |
| S4    | Authority mapping              | Persona authority in every reachable state        |
| S5    | Verdict enumeration            | All producible verdicts and their dependencies    |
| S6    | Flow path enumeration          | All execution paths through all flows             |
| S7    | Complexity metrics             | Predicate depth, flow depth, branching factors    |
| S8    | Verdict uniqueness             | Duplicate or conflicting verdict productions      |

Cross-contract analysis extends S4 and S6 to System constructs, checking authority and trigger cycles across contract boundaries.

---

## Domain Contracts

Six validated domain contracts ship with the repository, each proven end-to-end through elaboration, evaluation, and static analysis:

| Domain              | Contract                        | Directory                       |
| ------------------- | ------------------------------- | ------------------------------- |
| SaaS                | Subscription lifecycle          | `domains/saas/`                 |
| Healthcare          | Prior authorization workflow    | `domains/healthcare/`           |
| Supply Chain        | Goods inspection                | `domains/supply_chain/`         |
| Energy Procurement  | RFP workflow                    | `domains/energy_procurement/`   |
| Trade Finance       | Letter of credit                | `domains/trade_finance/`        |
| System Scenario     | Cross-contract trade inspection | `domains/system_scenario/`      |

The System scenario composes the supply chain and trade finance contracts via cross-contract triggers, demonstrating multi-contract coordination.

---

## Documentation

| Document                                      | Audience                |
| --------------------------------------------- | ----------------------- |
| [Narrative](docs/guide/narrative.md)           | Everyone                |
| [Formal specification](docs/TENOR.md)         | Language implementors   |
| [Author guide](docs/guide/author-guide.md)    | Contract authors        |
| [What is Tenor?](docs/guide/what-is-tenor.md) | Decision makers         |

---

## Structure

```
docs/
  TENOR.md              -- full formal specification (v1.0)
  guide/                -- documentation for authors and decision makers
conformance/            -- elaborator conformance suite (72 tests)
  positive/             -- valid DSL -> expected interchange JSON
  negative/             -- invalid DSL -> expected error JSON
  numeric/              -- decimal/money precision fixtures
  promotion/            -- numeric type promotion fixtures
  shorthand/            -- DSL shorthand expansion fixtures
  cross_file/           -- multi-file import fixtures
  parallel/             -- parallel entity conflict fixtures
  analysis/             -- static analysis fixtures
  eval/                 -- evaluator fixtures
  manifest/             -- manifest-based test fixtures
domains/                -- validated domain contracts
  saas/                 -- SaaS subscription lifecycle
  healthcare/           -- healthcare prior authorization
  supply_chain/         -- supply chain goods inspection
  energy_procurement/   -- energy RFP workflow
  trade_finance/        -- trade finance letter of credit
  system_scenario/      -- multi-contract System composition
crates/
  core/                 -- library: elaboration pipeline (6-pass)
  cli/                  -- binary: tenor command-line tool
  eval/                 -- library: contract evaluator
  analyze/              -- library: static analysis (S1-S8)
  codegen/              -- library: code generation (scaffold)
  lsp/                  -- library: Language Server Protocol (scaffold)
```

## Build

```bash
# Build all crates
cargo build --workspace

# Run conformance suite (72 tests)
cargo run -p tenor-cli -- test conformance

# Run all unit tests
cargo test --workspace

# Elaborate a .tenor file to interchange JSON
cargo run -p tenor-cli -- elaborate path/to/file.tenor

# Validate interchange JSON against schema
cargo run -p tenor-cli -- validate path/to/bundle.json

# Run static analysis on a contract
cargo run -p tenor-cli -- check path/to/file.tenor

# Evaluate a contract against facts
cargo run -p tenor-cli -- eval path/to/bundle.json --facts path/to/facts.json
```

---

## License

Apache 2.0. Copyright 2026 Riverline Labs.
