# Tenor Author Guide

A guide for writing behavioral contracts in the Tenor DSL.

---

## Part 1 -- Why Tenor Exists

Tenor is non-Turing complete by design. That is not a limitation -- it is how we can prove things about your contracts.

### The fragmentation problem

In most systems, behavior lives in fragments. API specs describe endpoints but not authorization boundaries. RBAC configs enumerate permissions but not the state machines they gate. Policy YAML captures conditions but not the workflows that depend on them. Workflow engines orchestrate steps but cannot verify that the orchestration is complete.

The result: no single artifact describes what a system is supposed to do. Auditing requires cross-referencing five different formats. Automated agents cannot reason about behavior because behavior is never stated in one place. And when fragments disagree, the implementation decides -- silently.

### What Tenor is

Tenor is a behavioral contract language. A `.tenor` file declares what a system does -- the facts it consumes, the entities it tracks, the rules that produce verdicts, the operations that change state, and the flows that orchestrate those operations. One file, one truth.

Tenor is not a programming language. You cannot write loops, call functions, or allocate memory. Tenor is not a configuration format. You cannot set feature flags or toggle behavior at runtime. The language's constraints are the mechanism that enables formal guarantees.

Every constraint exists because it enables a proof:

- **Non-Turing completeness** means the elaborator can enumerate every possible execution path. No halting problem. No unbounded computation.
- **No aggregates** means every value the contract acts on has a declared external source. The contract is honest about what it knows and what it trusts.
- **Frozen verdicts** mean a flow's decisions are deterministic with respect to its snapshot. No TOCTOU races between rule evaluation and operation execution.
- **Finite state machines** mean entity behavior is exhaustively checkable. Every state is reachable or the elaborator rejects the contract.

### The four-layer trust model

Tenor's guarantees come from four independent layers, each verifiable by a different party:

**Layer 1: Language guarantees.** Properties S1 through S8 hold for any valid contract, by construction. If your contract elaborates without error, you know: every entity's state space is finite and fully enumerable (S1). Every reachable state has been verified (S2). Every persona's authority boundary is statically derivable (S4). Every flow path terminates (S6). Every verdict type is produced by exactly one rule (S8). These are not runtime checks. They are structural truths the elaborator proves during compilation.

**Layer 2: Elaborator trust.** A conforming elaborator guarantees that the interchange JSON faithfully represents the contract. The conformance test suite (127 fixtures) verifies this. If two elaborators produce the same interchange JSON for the same `.tenor` file, they agree on the contract's meaning. This is how you get tool interoperability without trusting any single implementation.

**Layer 3: Executor trust.** Executor obligations E1 through E14 are precisely specified in the Tenor specification. An executor that claims conformance must honor frozen verdict semantics, entity state machine transitions, persona authority checks, and flow orchestration rules. These obligations are testable: you can verify an executor's conformance against known contract/fact/expected-outcome triples.

**Layer 4: Logic conformance vs. operational conformance.** Within executor trust, there are two distinct claims. Logic conformance means the executor produces the correct verdicts, state transitions, and flow outcomes given the interchange JSON — this is testable with known inputs and expected outputs. Operational conformance means the executor honors those obligations under real conditions: atomicity under failure, snapshot isolation under concurrency, external source integrity across network boundaries. A conforming elaborator plus a signed interchange artifact proves logic conformance. It proves nothing about operational conformance. That is the executor's responsibility, verified separately.

Each layer is independently auditable. You do not need to trust the person who wrote the contract, the team that built the elaborator, or the company that runs the executor. You need to verify that each layer meets its specification. The specification is public. The conformance suite is open. The proofs are structural.

---

## Part 2 -- Core Concepts

Tenor contracts are built from seven constructs, introduced here in dependency order. Each later construct can reference earlier ones; none can reference later ones.

For formal definitions, see the [Tenor Specification](../TENOR.md). This section teaches you how to think about each construct as a contract author.

### 1. Facts

A Fact declares a typed value that comes from outside the contract. Facts are ground truth -- the contract does not compute them, verify them, or derive them. It accepts them from a named external source and acts on them.

```tenor
fact cargo_weight_kg {
  type:   Int(min: 0, max: 1000000)
  source: "cargo_service.total_weight_kg"
}

fact delivery_confirmed {
  type:    Bool
  source:  "tracking_service.delivered"
  default: false
}
```

The `source` field names the external system that provides this value. The type constrains what values are acceptable. The optional `default` provides a value when the source has not yet reported.

**Common mistake: computing a value inside the contract and calling it a Fact.** Facts come from external systems. If you need a derived value (like "total weight of all items"), that value must be computed externally and supplied as a Fact. The contract cannot sum a list -- and that constraint is deliberate. See Pattern 3 in Part 3.

### 2. Entities

An Entity is a finite state machine. It declares every state the entity can be in and every legal transition between states. Nothing else changes an Entity's state -- not external events, not implicit timeouts, not side effects.

```tenor
entity Order {
  states:  [pending, confirmed, shipped, delivered, cancelled]
  initial: pending
  transitions: [
    (pending, confirmed),
    (pending, cancelled),
    (confirmed, shipped),
    (confirmed, cancelled),
    (shipped, delivered)
  ]
}
```

The elaborator proves this state machine is well-formed: the initial state is in the state set, every transition references valid states, and (via static analysis) every state is reachable from the initial state. If you declare a state that no transition ever reaches, the contract is rejected.

**Common mistake: assuming transitions happen automatically.** A transition declared in an Entity is a *permission*, not a trigger. The transition `(pending, confirmed)` means "it is structurally legal for Order to move from pending to confirmed." The actual transition only happens when an Operation with effect `(Order, pending, confirmed)` executes successfully. Entities declare the shape. Operations make things happen.

### 3. Rules

A Rule evaluates a predicate expression against the current facts and, when the predicate is satisfied, produces a verdict. Rules are stratified: stratum 0 rules see only facts, stratum 1 rules can reference verdicts from stratum 0, and so on. No rule may reference a verdict from its own stratum or a higher one.

```tenor
rule weight_acceptable {
  stratum: 0
  when:    cargo_weight_kg <= 50000
  produce: verdict weight_ok { payload: Bool = true }
}

rule shipment_ready {
  stratum: 1
  when:    verdict_present(weight_ok)
         ∧ verdict_present(docs_complete)
  produce: verdict ready_to_ship { payload: Bool = true }
}
```

Each rule produces exactly one verdict type, and each verdict type is produced by exactly one rule in the entire contract. This is property S8 -- verdict uniqueness. The elaborator enforces it structurally.

**Common mistake: putting two rules at the same stratum where one references the other's verdict.** If `rule_a` at stratum 0 produces `verdict_x`, and `rule_b` also at stratum 0 tries to use `verdict_present(verdict_x)`, the contract is rejected. Stratum ordering exists so the elaborator can evaluate rules in a single pass per stratum with no circular dependencies. Move `rule_b` to stratum 1.

### 4. Personas

A Persona declares an authority boundary -- a named role that can invoke operations. Personas are not users, not groups, not API keys. They are abstract roles that the executor maps to concrete identities at runtime.

```tenor
persona warehouse_manager
persona shipping_clerk
persona auditor
```

Personas are simple declarations, but they are the foundation of property S4 (authority topology). By declaring which personas can invoke which operations, the contract makes authority boundaries visible and checkable. The static analyzer can answer: "Can the shipping_clerk cancel an order?" without running anything.

**Common mistake: creating a persona for every user.** Personas represent roles, not individuals. If Alice and Bob are both warehouse managers, they share the `warehouse_manager` persona. The executor handles identity-to-persona mapping. The contract reasons about what roles can do, not who holds those roles.

### 5. Operations

An Operation is a persona-gated state transition. It declares which personas may invoke it, what precondition must hold, and what entity state changes result from successful execution.

```tenor
operation confirm_order {
  allowed_personas: [warehouse_manager]
  precondition:     verdict_present(ready_to_ship)
  effects:          [(Order, pending, confirmed)]
  error_contract:   [precondition_failed, persona_rejected]
}
```

The `precondition` is a predicate expression (same syntax as rule conditions). The `effects` list declares entity state transitions. The `error_contract` declares the exhaustive set of error outcomes -- there are no surprises at runtime.

**Common mistake: omitting error outcomes.** Every Operation must declare its `error_contract`. This is not optional error handling bolted on after the happy path. The error outcomes are part of the contract's formal behavior. An Operation that can fail in ways not declared in its `error_contract` violates the contract. The executor must reject undeclared failure modes.

### 6. Flows

A Flow is a finite directed acyclic graph of steps that orchestrates Operations. Each step is one of several types: OperationStep (invoke an operation), BranchStep (route based on a condition), HandoffStep (transfer authority between personas), SubFlowStep (delegate to another flow), ParallelStep (concurrent branches), or Terminal (end the flow with a named outcome).

```tenor
flow order_fulfillment {
  snapshot: at_initiation
  entry:    step_confirm

  steps: {
    step_confirm: OperationStep {
      op:      confirm_order
      persona: warehouse_manager
      outcomes: {
        success: step_ship
      }
      on_failure: Terminate(outcome: confirmation_failed)
    }

    step_ship: OperationStep {
      op:      ship_order
      persona: shipping_clerk
      outcomes: {
        success: Terminal(order_shipped)
      }
      on_failure: Terminate(outcome: shipping_failed)
    }
  }
}
```

The `snapshot: at_initiation` declares frozen verdict semantics -- the verdict set is computed once when the flow starts and does not change during flow execution. This eliminates time-of-check-to-time-of-use races between rule evaluation and operation execution.

Every path through the flow must reach a Terminal. The elaborator proves this (property S6 -- flow path enumeration). If there is any step that can reach a dead end without a Terminal, the contract is rejected.

**Common mistake: assuming verdicts are re-evaluated between steps.** Once a flow starts, its verdict snapshot is frozen. If `step_confirm` succeeds and changes entity state in a way that would change verdict outcomes, the subsequent steps still see the original verdicts. This is intentional. The flow's decisions are consistent with respect to a single point-in-time evaluation. If you need fresh verdicts, start a new flow.

### 7. System

A System composes multiple contracts into a cross-contract workflow. It declares which contracts participate, which personas and entities are shared between them, and what triggers connect one contract's flow outcomes to another contract's flow entries.

```tenor
system order_system {
  members: [
    warehouse: "warehouse.tenor",
    shipping:  "shipping.tenor"
  ]
  shared_personas: [warehouse_manager]
  triggers: [
    {
      source: warehouse.fulfillment_flow,
      on: success,
      target: shipping.dispatch_flow,
      persona: warehouse_manager
    }
  ]
  shared_entities: []
}
```

System composition preserves all single-contract guarantees and adds cross-contract analysis. The `tenor check` command reports cross-contract flow paths (S6 extended) and verifies that trigger chains do not form cycles.

**Common mistake: sharing personas or entities that do not genuinely overlap.** If two contracts have no natural persona overlap, use empty `shared_personas: []`. Do not manufacture shared personas to "connect" contracts. The trigger mechanism handles cross-contract coordination. Shared personas mean the same role has authority in both contracts -- that is a domain truth, not a wiring convenience.

### 8. Source Declarations

A Source declaration describes how a Fact's external data is fetched — the protocol, endpoint, field mapping, and polling or subscription behavior. Sources separate the *what* (the Fact) from the *how* (the adapter wiring). A Fact names its source system and field; a Source declaration provides the connection details the adapter framework needs to actually retrieve the value. Sources support multiple protocols (`rest`, `graphql`, `grpc`, `database`, `message_queue`) and can declare extension-specific metadata for custom adapter implementations. See §5 of the specification for the full Source grammar.

### 9. TaggedUnion Type

A TaggedUnion is a sum type: a value that is exactly one of several named variants, where each variant carries its own typed payload. Declare a TaggedUnion with `type` and use it as a Fact type or Record field type. TaggedUnions are useful when a single Fact can take structurally different shapes depending on context — for example, a payment method that is either a credit card (with card number and expiry) or a bank transfer (with routing and account numbers). The elaborator resolves TaggedUnion references through TypeDecl and inlines the full variant structure into interchange JSON. See §4.4 of the specification.

### 10. Multi-Instance Entities

By default, an Entity tracks a single state machine instance. A multi-instance Entity tracks many instances of the same state machine, each identified by a key Fact. Declare a multi-instance Entity by adding `instance_key: <fact_id>` where the referenced Fact uniquely identifies each instance. Operations on multi-instance Entities apply their effects to the specific instance identified by the key value at execution time. The elaborator validates that the instance key references a declared Fact and that all structural guarantees (S1, S2) hold per-instance. See §6.2 of the specification.

### 11. Migration

When you change a contract — adding Facts, renaming states, modifying transitions — the `tenor migrate` command computes the structural diff between the old and new interchange JSON and generates a migration plan. The migration engine classifies each change by compatibility level (backward-compatible additions, breaking removals, state remappings) and produces executable migration steps. Migration plans are deterministic: given the same old and new contracts, the same plan is always generated. This lets you evolve contracts over time without manual diffing or ad-hoc upgrade scripts. See the migration engine in `crates/eval/src/migration/`.

---

## Part 3 -- Patterns

Part 2 gave you the constructs. This section shows you how to combine them in patterns drawn from real domain contracts. Each pattern addresses a specific coordination problem, demonstrates the Tenor idiom for solving it, and identifies the formal property that makes the solution trustworthy.

### Pattern 1 -- Parallel approval with compensation

**Domain:** Supply chain cargo inspection at an international port.

**Problem:** A shipment must pass both quality inspection and compliance inspection before it can be cleared. These inspections are independent -- a quality inspector and a customs officer evaluate different aspects of the cargo concurrently. Both must succeed for clearance. If either fails, the shipment is held, and any partially-committed inspection state must be rolled back.

**The Tenor idiom:** Use a ParallelStep with disjoint entities per branch, a BranchStep for the clearance decision, and a Compensate handler for rollback.

Here is the relevant structure from `domains/supply_chain/inspection.tenor`:

Two separate entities track the independent inspections:

```tenor
entity QualityLot {
  states:  [pending, in_progress, passed, failed]
  initial: pending
  transitions: [
    (pending, in_progress),
    (in_progress, passed),
    (in_progress, failed)
  ]
}

entity ComplianceLot {
  states:  [pending, in_progress, passed, failed]
  initial: pending
  transitions: [
    (pending, in_progress),
    (in_progress, passed),
    (in_progress, failed)
  ]
}
```

QualityLot and ComplianceLot are separate entities -- not by modeling preference, but by spec requirement. Parallel branches must have disjoint entity effect sets. If both branches modified the same entity, the parallel execution would be non-deterministic: the final state would depend on branch ordering. Separate entities make the branches truly independent, and the elaborator can verify this statically.

The flow orchestrates the parallel inspection and the clearance decision:

```tenor
flow inspection_flow {
  snapshot: at_initiation
  entry:    step_begin

  steps: {
    step_begin: OperationStep {
      op:      begin_inspection
      persona: customs_officer
      outcomes: { success: step_parallel_inspect }
      on_failure: Terminate(outcome: inspection_blocked)
    }

    step_parallel_inspect: ParallelStep {
      branches: [
        Branch {
          id:    branch_quality
          entry: step_quality
          steps: {
            step_quality: OperationStep {
              op:      record_quality_pass
              persona: quality_inspector
              outcomes: { success: Terminal(quality_cleared) }
              on_failure: Terminate(outcome: quality_failed)
            }
          }
        },
        Branch {
          id:    branch_compliance
          entry: step_compliance
          steps: {
            step_compliance: OperationStep {
              op:      record_compliance_pass
              persona: customs_officer
              outcomes: { success: Terminal(compliance_cleared) }
              on_failure: Terminate(outcome: compliance_failed)
            }
          }
        }
      ]
      join: JoinPolicy {
        on_all_success:  step_release_decision
        on_any_failure:  Terminate(outcome: inspection_failed)
        on_all_complete: null
      }
    }

    step_release_decision: BranchStep {
      condition: verdict_present(clearance_approved)
      persona:   port_authority
      if_true:   step_release
      if_false:  step_hold
    }

    // ... step_release and step_hold follow
  }
}
```

The Compensate handler on the hold path rolls back partially-committed inspection state:

```tenor
    step_hold: OperationStep {
      op:      hold_shipment
      persona: customs_officer
      outcomes: { success: Terminal(shipment_held) }
      on_failure: Compensate(
        steps: [{
          op:         revert_quality
          persona:    customs_officer
          on_failure: Terminal(revert_failed)
        }]
        then: Terminal(inspection_reverted)
      )
    }
```

Compensation is not error handling. It is rollback of committed state. The `revert_quality` operation transitions QualityLot back to a consistent state. The Compensate handler guarantees that if the hold operation fails, the contract does not leave inspection state in a half-committed condition.

**Formal property demonstrated:** S6 (flow path enumeration). The elaborator proves that every path through `inspection_flow` -- including both parallel branch outcomes and the compensation path -- reaches a Terminal. No path is missing a failure handler.

### Pattern 2 -- Threshold-gated handoff

**Domain:** Escrow release in a buyer-seller transaction.

**Problem:** After delivery is confirmed, the escrow should be released. But if the escrow amount exceeds a compliance threshold, the release requires a compliance officer's review -- not just the escrow agent's authority. The decision to route to compliance must be based on the facts at flow initiation, not at the moment the branch is evaluated.

**The Tenor idiom:** Use a BranchStep to route based on a threshold verdict, routing either to a direct release or to a HandoffStep that transfers authority to the compliance officer.

From `conformance/positive/integration_escrow.tenor`:

```tenor
flow standard_release {
  snapshot: at_initiation
  entry:    step_confirm

  steps: {
    step_confirm: OperationStep {
      op:      confirm_delivery
      persona: seller
      outcomes: { success: step_check_threshold }
      on_failure: Terminate(outcome: failure)
    }

    step_check_threshold: BranchStep {
      condition: verdict_present(within_threshold)
      persona:   escrow_agent
      if_true:   step_auto_release
      if_false:  step_handoff_compliance
    }

    step_auto_release: OperationStep {
      op:      release_escrow
      persona: escrow_agent
      outcomes: { success: Terminal(success) }
      on_failure: Compensate(
        steps: [{
          op:         revert_delivery_confirmation
          persona:    escrow_agent
          on_failure: Terminal(failure)
        }]
        then: Terminal(failure)
      )
    }

    step_handoff_compliance: HandoffStep {
      from_persona: escrow_agent
      to_persona:   compliance_officer
      next:         step_compliance_release
    }

    step_compliance_release: OperationStep {
      op:      release_escrow_with_compliance
      persona: compliance_officer
      outcomes: { success: Terminal(success) }
      on_failure: Compensate(
        steps: [{
          op:         revert_delivery_confirmation
          persona:    escrow_agent
          on_failure: Terminal(failure)
        }]
        then: Terminal(failure)
      )
    }
  }
}
```

The key insight is `snapshot: at_initiation` combined with the BranchStep. The verdict `within_threshold` is computed from the facts as they exist when `standard_release` is initiated. Even if the escrow amount changes between initiation and the moment `step_check_threshold` executes, the branch decision uses the frozen snapshot. This is frozen verdict semantics.

Why this matters: without frozen verdicts, a race condition is possible. Imagine the escrow amount is \$9,000 at flow initiation (under the \$10,000 threshold), but by the time the branch evaluates, it has changed to \$11,000. Without freezing, the flow would route to auto-release using the old routing decision but the amount would now exceed the threshold. Frozen verdicts eliminate this class of bug by construction.

**Formal property demonstrated:** Frozen verdict semantics (executor obligation E3) and S6 (every path terminates, including both Compensate paths).

### Pattern 3 -- External aggregate as Fact

**Domain:** Any domain requiring computed totals, averages, or summaries.

**Problem:** A procurement contract needs to enforce a spending threshold against the total value of a requisition. The total is the sum of all line item amounts. The naive approach: compute the sum inside the contract.

**Why that is wrong:** Tenor does not support aggregation. You cannot sum a list, compute an average, or count elements. This is not an oversight. Aggregates are derived values -- they depend on the completeness and correctness of the underlying data. A contract cannot verify that it received all line items, or that the amounts were not tampered with before summation. Pretending the contract can compute a trustworthy aggregate is dishonest about the trust boundary.

**The incorrect pattern:**

```
// THIS DOES NOT WORK -- Tenor has no aggregation
rule check_total {
  stratum: 0
  when:    sum(line_items.amount) <= budget_limit   // not valid Tenor
  produce: verdict within_budget { payload: Bool = true }
}
```

**The correct pattern:**

```tenor
fact requisition_total {
  type:   Money(currency: "USD")
  source: "procurement_service.requisition_total"
}

fact budget_limit {
  type:   Money(currency: "USD")
  source: "budget_service.department_limit"
}

rule within_budget {
  stratum: 0
  when:    requisition_total <= budget_limit
  produce: verdict budget_ok { payload: Bool = true }
}
```

The `requisition_total` is a Fact -- it comes from an external system that computed the sum. The contract acts on the pre-computed result. This is honest about the boundary: the contract cannot verify the arithmetic itself. It trusts the `procurement_service` to compute the total correctly, and that trust is visible in the `source` field.

This pattern applies whenever you need a derived value: totals, counts, averages, maximums, percentiles. Each becomes a Fact with a named external source. The contract's job is to make decisions based on those values, not to compute them.

**Formal property demonstrated:** The trust boundary is explicit in the contract structure. Every value the contract acts on is traceable to a declared source. This is not a formal S-property, but it is a consequence of the language's design: the absence of aggregation forces honesty about what the contract knows versus what it assumes.

### Pattern 4 -- Multi-stratum verdict chaining

**Domain:** Healthcare prior authorization.

**Problem:** A prior authorization decision depends on multiple independent checks (documentation completeness, policy criteria, clinical review) that must compose into a final approve/deny decision. Some checks depend on the results of others. The naive approach: put all the logic in one giant rule.

**Why stratification:** Rules at different strata form a directed dependency chain. Stratum 0 rules see only facts. Stratum 1 rules can reference stratum 0 verdicts. Stratum 2 rules can reference stratum 0 and 1 verdicts. No rule references a verdict from its own stratum -- this prohibition ensures the evaluation order is well-defined and the elaborator can process each stratum in a single pass.

From `domains/healthcare/prior_auth.tenor`, four strata build a complete authorization decision:

**Stratum 0** -- ground checks against facts:

```tenor
rule all_records_complete {
  stratum: 0
  when:    forall record in medical_records . record.is_complete = true
  produce: verdict records_complete { payload: Bool = true }
}

rule diagnosis_covered {
  stratum: 0
  when:    policy_criteria.diagnosis_covered = true
  produce: verdict diagnosis_is_covered { payload: Bool = true }
}

// ... plus 7 more stratum 0 rules checking individual criteria
```

**Stratum 1** -- compose ground verdicts into eligibility determinations:

```tenor
rule documentation_sufficient {
  stratum: 1
  when:    verdict_present(records_complete)
         ∧ verdict_present(records_relevant)
  produce: verdict documentation_ok { payload: Bool = true }
}

rule policy_criteria_satisfied {
  stratum: 1
  when:    verdict_present(diagnosis_is_covered)
         ∧ verdict_present(treatment_is_formulary)
         ∧ verdict_present(provider_in_network)
         ∧ verdict_present(step_therapy_done)
  produce: verdict policy_satisfied { payload: Bool = true }
}
```

**Stratum 2** -- final authorization decision:

```tenor
rule can_approve {
  stratum: 2
  when:    verdict_present(documentation_ok)
         ∧ verdict_present(policy_satisfied)
         ∧ verdict_present(clinical_criteria_passed)
  produce: verdict authorization_approved { payload: Bool = true }
}

rule should_deny {
  stratum: 2
  when:    verdict_present(documentation_ok)
         ∧ ¬verdict_present(clinical_criteria_passed)
  produce: verdict authorization_denied { payload: Bool = true }
}
```

**Stratum 3** -- appeal outcome (depends on stratum 2 appeal merit verdicts):

```tenor
rule can_overturn_denial {
  stratum: 3
  when:    verdict_present(appeal_meritorious)
         ∨ verdict_present(new_evidence_available)
  produce: verdict overturn_recommended { payload: Bool = true }
}
```

The full prior authorization contract has 13 rules across 4 strata, 8 operations, 6 personas, and 2 flows (including a SubFlowStep for the appeal process with a HandoffStep to the appeals board). This is genuine healthcare complexity -- not a toy example -- and Tenor makes every decision step auditable.

**If you get the strata wrong:** Suppose you accidentally write `documentation_sufficient` and `policy_criteria_satisfied` at stratum 0 and try to reference `records_complete` (also stratum 0). The elaborator rejects the contract: same-stratum verdict references are prohibited. The fix is the re-expression theorem -- move the dependent rule to a higher stratum. This is not busywork. It forces you to make the dependency chain explicit, which is exactly what an auditor needs to see.

**Formal property demonstrated:** S8 (verdict uniqueness) -- each verdict type in this contract is produced by exactly one rule. `authorization_approved` comes only from `can_approve`. `authorization_denied` comes only from `should_deny`. An auditor can trace any verdict to its single source without searching the entire contract. The stratification makes the data flow visible: facts feed stratum 0, stratum 0 feeds stratum 1, and so on. No cycles, no ambiguity.

---

## Part 4 -- What You Can Prove

After Parts 1-3, you know how to write Tenor contracts and how to apply common patterns. This section shows you what those contracts give you: concrete, structural proofs about your system's behavior. Not aspirational claims. Proofs that are visible in the contract structure and verifiable by running `tenor check`.

### S4 proof: authority boundaries

**Property S4** states that for any persona and any entity state, the set of operations that persona can invoke is statically derivable.

Consider the supply chain inspection contract. It declares four personas:

```tenor
persona customs_officer
persona quality_inspector
persona port_authority
persona shipping_agent
```

Now look at the operations that affect the Shipment entity:

| Operation | allowed_personas | Shipment effect |
|-----------|-----------------|-----------------|
| `begin_inspection` | `[customs_officer]` | arrived -> inspecting |
| `release_shipment` | `[port_authority]` | inspecting -> cleared |
| `hold_shipment` | `[customs_officer]` | inspecting -> held |

The `customs_officer` can begin inspection and hold a shipment. The `port_authority` can release a shipment. The `quality_inspector` can record quality results but cannot touch the Shipment entity at all -- their authority is limited to QualityLot. And `shipping_agent`? Declared but given no operations. Perhaps they are used in a different contract in the System.

The proof that `customs_officer` cannot release a shipment is visible in the contract text: `release_shipment` lists `[port_authority]` in `allowed_personas`, and `customs_officer` is not in that list. No runtime check needed. No log analysis. The authority topology is a structural fact of the contract.

Running `tenor check` on this contract produces:

```
Authority: 3 personas, 8 authority entries
```

The static analyzer has enumerated every (persona, operation) pair. Anyone can verify the authority boundaries by reading the contract or running the analysis tool.

### S6 proof: flow termination

**Property S6** states that for each flow, the complete set of possible execution paths is derivable, and every path reaches a terminal.

Take `inspection_flow` from the supply chain contract. The flow has two possible paths through the ParallelStep (both succeed or any failure), a BranchStep after the join, and Terminal outcomes on every branch. Let us enumerate:

**Path 1 -- both inspections pass, clearance approved:**
`step_begin` (success) -> `step_parallel_inspect` (all_success) -> `step_release_decision` (true) -> `step_release` (success) -> Terminal(shipment_cleared)

**Path 2 -- both inspections pass, clearance not approved:**
`step_begin` (success) -> `step_parallel_inspect` (all_success) -> `step_release_decision` (false) -> `step_hold` (success) -> Terminal(shipment_held)

**Path 3 -- any inspection fails:**
`step_begin` (success) -> `step_parallel_inspect` (any_failure) -> Terminate(inspection_failed)

**Path 4 -- begin inspection fails:**
`step_begin` (failure) -> Terminate(inspection_blocked)

**Path 5 -- hold fails, compensation triggered:**
`step_begin` (success) -> `step_parallel_inspect` (all_success) -> `step_release_decision` (false) -> `step_hold` (failure) -> Compensate -> Terminal(inspection_reverted) or Terminal(revert_failed)

**Path 6 -- release fails:**
`step_begin` (success) -> `step_parallel_inspect` (all_success) -> `step_release_decision` (true) -> `step_release` (failure) -> Terminate(release_failed)

Every path ends at a Terminal or Terminate. No dead ends. No paths that silently drop execution. The elaborator verifies this exhaustively. Running `tenor check`:

```
Flow Paths: 2 total paths across 1 flows
```

The tool counts the primary paths. Every success and failure outcome at every step is accounted for.

### Cross-contract S6: System-level flow analysis

When contracts are composed into a System, the S6 guarantee extends across contract boundaries. The `tenor check` output for the trade inspection system demonstrates this:

```
$ tenor check domains/system_scenario/trade_inspection_system.tenor

Cross-Contract Flow Paths (S6): 1 cross-contract triggers, 1 cross-contract paths

Findings:
  [s6_cross/INFO]: Cross-contract flow trigger:
    inspection.inspection_flow --[success]--> letter_of_credit.lc_presentation_flow
    (persona: beneficiary)
```

One line proves the trigger chain is correctly wired: when `inspection_flow` completes with a success outcome, `lc_presentation_flow` is triggered with `beneficiary` as the initiating persona. The analyzer has verified that:

1. `inspection_flow` in the inspection contract has a `success` terminal outcome.
2. `lc_presentation_flow` in the letter_of_credit contract exists and is a valid flow.
3. `beneficiary` is a declared persona in the letter_of_credit contract.
4. The trigger chain does not form a cycle.

### Independent auditability

Because Tenor is fully specified and deterministic, anyone can verify a contract independently. Given the same `.tenor` file and the same facts, any conforming elaborator produces the same interchange JSON, any conforming evaluator produces the same verdicts, and any conforming executor makes the same state transitions.

This means:

- A regulator can run your contract through their own toolchain and verify outputs.
- An automated agent can evaluate the contract against facts and determine whether an operation is permitted -- without executing it.
- A third-party auditor can enumerate all authority boundaries, all flow paths, and all verdict sources without access to your runtime system.

The contract is the source of truth, and anyone can check it.

---

## Part 5 -- System Composition

A single Tenor contract describes one bounded context. Real systems span multiple bounded contexts. The System construct composes contracts into a coordinated whole.

### Domain rationale

In international trade, a cargo shipment arriving at a port must pass inspection before the financial instruments backing that shipment can proceed. Specifically: the supply chain inspection contract determines whether goods meet quality and compliance standards. The trade finance letter of credit contract handles document presentation and payment. The business rule is straightforward -- do not present LC documents until the shipment has cleared inspection.

This is not an arbitrary coupling. It reflects a real-world dependency: banks will not honor a letter of credit for goods that have been rejected at port. The System construct makes this dependency explicit and verifiable.

### The contract

Here is `domains/system_scenario/trade_inspection_system.tenor` in full:

```tenor
system trade_inspection_system {
  members: [
    inspection: "../supply_chain/inspection.tenor",
    letter_of_credit: "../trade_finance/letter_of_credit.tenor"
  ]
  shared_personas: []
  triggers: [
    {
      source: inspection.inspection_flow,
      on: success,
      target: letter_of_credit.lc_presentation_flow,
      persona: beneficiary
    }
  ]
  shared_entities: []
}
```

### What this declares

**Members.** Two contracts participate: the supply chain inspection contract (aliased as `inspection`) and the trade finance letter of credit contract (aliased as `letter_of_credit`). Each is a standalone, valid Tenor contract that elaborates independently.

**Trigger.** When `inspection.inspection_flow` completes with the `success` outcome (meaning the shipment cleared inspection), the system triggers `letter_of_credit.lc_presentation_flow` with `beneficiary` as the acting persona. The beneficiary (exporter) can now present documents against the LC.

**Shared personas: none.** The supply chain domain uses `customs_officer`, `quality_inspector`, `port_authority`, and `shipping_agent`. The trade finance domain uses `applicant`, `beneficiary`, `issuing_bank`, `advising_bank`, and `confirming_bank`. These domains have no natural persona overlap, and the System declaration is honest about that. The trigger is the only relationship, and it is the right one.

**Shared entities: none.** The supply chain tracks Shipment, QualityLot, and ComplianceLot. The trade finance contract tracks LetterOfCredit and Document. No entity exists in both domains. Again, the System declaration reflects the domain reality rather than manufacturing artificial connections.

### The S6 output

Running `tenor check` on this system produces:

```
Cross-Contract Flow Paths (S6): 1 cross-contract triggers, 1 cross-contract paths

Findings:
  [s6_cross/INFO]: Cross-contract flow trigger:
    inspection.inspection_flow --[success]--> letter_of_credit.lc_presentation_flow
    (persona: beneficiary)
```

This confirms:
- The trigger chain is acyclic (no circular dependencies between contracts).
- The source flow outcome (`success`) exists in the inspection contract.
- The target flow (`lc_presentation_flow`) exists in the LC contract.
- The persona (`beneficiary`) is declared in the target contract.

The full execution sequence, from cargo arrival to LC payment, crosses two independent contracts through one verified trigger. Each contract maintains its own S1-S8 guarantees. The System adds cross-contract S6 verification on top.

---

*For the formal specification of all constructs, properties, and executor obligations, see [docs/TENOR.md](../TENOR.md).*
