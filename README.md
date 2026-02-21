# Tenor

**A behavioral contract calculus.**

Tenor is a finite, stratified, verdict-producing formal system for describing the complete observable behavior of a system. The entire state space, all authority boundaries, and all possible verdict outcomes are statically derivable from the contract alone — without executing any implementation.

> Any agent that can read this specification can fully understand a system described in it, without reading any implementation code.

**Status:** Pre-alpha. Core constructs canonicalized. Tenor v1.0 defined. Elaborator 47/47 conformance tests passing.  
**Stability:** Pre-release (v0.3). Do not build production systems against this version. See [`STABILITY.md`](STABILITY.md).

---

## Why Tenor exists — and why nothing else does this

Systems today describe behavior across OpenAPI specs, JSON Schema, policy YAML, ad hoc state machines, workflow engines, RBAC configurations, and implementation code. None of it is formally unified. None of it is fully agent-legible. The fragmentation is real and worsening. Anywhere authority, state, and audit matter. In multi-tenant SaaS, healthcare workflows, procurement, internal approvals — the behavioral description of a system is scattered across a dozen artifacts, none of which agree.

Tenor is not a configuration format, a policy DSL, or a workflow engine. It is not a smart contract language — it has no notion of cryptography, distributed consensus, tokens, or blockchain. It is a **behavioral contract calculus**: a formal language where a contract is the complete description of a system's observable behavior, readable by humans, machines, and agents alike.

The research literature has behavioral contract calculi — formal models for proving properties about systems. Production languages (Clarity, Michelson) share some constraints but not this architecture. What's missing is a language that combines all of the following:

**Stratification is a language construct, not a derived property.** Strata are declared. Same-stratum rule references are illegal. Termination is structurally guaranteed — not proved after the fact, not checked at runtime.

**Provenance is part of the evaluation relation.** `eval_rules` returns `Set<Verdict × Provenance>`, not `Set<Verdict>`. The audit log is a theorem derived from evaluation, not a logging feature bolted on. Every decision carries its complete proof chain back to ground facts.

**No built-in functions. None.** No `now()`. No `sum()`. No regex. Time is a Fact. Totals are Facts. Everything that varies at runtime enters through the ground layer. This is a harder constraint than any production language we found — and it's what makes static analysis complete rather than approximate.

**Flow is part of the contract.** Orchestration is not external to the formal system. The flow graph, personas at each step, compensation logic, parallel branches — all of it is in the contract and statically analyzable. A static analyzer can enumerate every possible execution path without running anything.

**Static analyzability is a rejection filter, not a goal.** Any proposed feature that breaks complete static derivability is rejected regardless of ergonomic benefit.

Tenor is the first implemented language that combines these properties — with a working elaborator, a formal spec, and a conformance suite.

---

## A minimal example

A contract governing escrow release. Two entities, four personas, stratified verdict logic, a compensation flow.

```
type LineItemRecord {
  id:     Text(max_length: 64)
  amount: Money(currency: "USD")
  valid:  Bool
}

fact escrow_amount   { type: Money(currency: "USD"),   source: escrow_service.balance }
fact delivery_status { type: Enum(["pending", "confirmed", "failed"]), source: delivery_service.status }
fact line_items      { type: List(element_type: LineItemRecord, max: 100), source: order_service.items }

entity EscrowAccount {
  states:  [held, released, refunded]
  initial: held
  transitions: [held -> released, held -> refunded]
}

rule all_items_valid {
  stratum: 0
  when:    all item in line_items: item.valid = true
  produce: items_validated(true)
}

rule delivery_confirmed {
  stratum: 0
  when:    delivery_status = "confirmed"
  produce: delivery_confirmed(true)
}

rule can_release {
  stratum: 1
  when:    items_validated present and delivery_confirmed present
  produce: release_approved(true)
}

operation release_escrow {
  personas: [escrow_agent]
  require:  release_approved present
  effects:  [EscrowAccount: held -> released]
  on_error: [precondition_failed, persona_rejected]
}

flow release {
  snapshot: at_initiation
  entry:    step_release
  step step_release {
    op:         release_escrow
    as:         escrow_agent
    on_success: done(success)
    on_failure: terminate(failure)
  }
}
```

From this contract alone a static analyzer can derive: every reachable entity state, every persona's authority in every state, every verdict the rules can produce, every execution path through every flow, and the complete provenance chain for any outcome. No implementation required.

---

## Design constraints

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

Eleven constructs across two layers.

**Semantic layer** — dependency order:

| Construct               | Purpose                                                                                                           |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------- |
| **BaseType**            | Closed value type set: Bool, Int, Decimal, Text, Enum, Date, DateTime, Money, Record, TaggedUnion, List, Duration |
| **Fact**                | Ground typed assertions from external sources — the evaluation root and provenance origin                         |
| **Entity**              | Finite state machines in a static DAG                                                                             |
| **Rule**                | Stratified verdict-producing evaluation functions                                                                 |
| **Operation**           | Persona-gated, precondition-guarded state transitions                                                             |
| **PredicateExpression** | Quantifier-free FOL with arithmetic and bounded quantification over List-typed facts                              |
| **Flow**                | Finite DAG orchestration of Operations with sequential, branching, handoff, sub-flow, and parallel steps          |
| **NumericModel**        | Fixed-point decimal arithmetic with total promotion rules (cross-cutting)                                         |

**Tooling layer:**

| Artifact               | Purpose                                                                                  |
| ---------------------- | ---------------------------------------------------------------------------------------- |
| **Elaborator**         | Transforms `.tenor` source into a canonical JSON bundle through six deterministic passes |
| **Interchange format** | Canonical JSON bundle — the single source of truth for all downstream tooling            |

---

## Evaluation model

```
Read path:     assemble_facts → eval_strata → resolve → ResolvedVerdictSet
Write path:    execute(op, persona, verdict_set) → EntityState' | Error
Orchestration: execute_flow(flow, persona, snapshot) → FlowOutcome
               execute_parallel(branches, snapshot) → {BranchId → BranchOutcome}
               evaluate_join(join_policy, branch_outcomes) → StepTarget
```

Every step is bounded, deterministic, and statically analyzable.

---

## Repository structure

```
docs/TENOR.md     — full formal specification (v0.3)
STABILITY.md      — pre-release stability notice
CONTRIBUTING.md   — contribution guidelines
conformance/      — elaborator conformance suite (47/47 tests passing)
elaborator/       — reference elaborator implementation (Rust)
```

---

## License

Apache 2.0. Copyright 2026 Riverline Labs.
