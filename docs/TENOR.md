# Tenor

## Formal Specification v0.3

**Status:** Pre-alpha. Core constructs canonicalized. Syntax defined (Tenor v1.0). Elaborator conformance suite: 47/47 passing.
**Method:** Human-AI collaborative design.
**Models consulted:** Claude (Anthropic), GPT (OpenAI), DeepSeek, Gemini (Google)
**Changelog:** v0.2 — name established (Tenor); frozen verdict snapshot semantics explicit; Operation effects bound to EntityId; S3 split; TaggedUnion negation specified.
**Changelog:** v0.3 — Duration type; cross-currency pattern (P4 resolved, no new construct); variable×variable multiplication (Rule layer only); ParallelStep; ElaboratorSpec; pending work list updated; AL updated.

> **Stability: Pre-release (v0.3)**  
> This specification is under active development. Breaking changes may occur  
> without notice. Conformance claims against pre-1.0 versions are not  
> recognized by the project. Do not build production systems against this version.

---

## Table of Contents

1. Preamble
2. Design Constraints
3. Core Constructs Overview
4. BaseType
5. Fact
6. Entity
7. Rule
8. Operation
9. PredicateExpression
10. Flow
11. NumericModel
12. ElaboratorSpec
13. Complete Evaluation Model
14. Static Analysis Obligations
15. Executor Obligations
16. Pending Work
17. Appendix A — Acknowledged Limitations
18. Appendix B — Model Convergence Record
19. Appendix C — Worked Example: Escrow Release Contract

---

## 1. Preamble

Systems today describe their behavior across OpenAPI specs, JSON Schema, policy YAML, ad hoc state machines, workflow engines, RBAC configurations, and implementation code. None of it is formally unified. None of it is fully agent-legible. The fragmentation is real and worsening.

This language is not a better configuration format. It is not a policy DSL. It is not a workflow engine. It is a **behavioral contract calculus**: a finite, stratified, verdict-producing formal system for describing the complete observable behavior of a system such that the entire state space, all authority boundaries, and all possible verdict outcomes are statically derivable without executing any implementation.

The language has one core semantic requirement:

> Any agent that can read this specification can fully understand a system described in it, without reading any implementation code.

This is a stronger claim than "machine-readable." It means the semantics are complete enough that an agent can simulate execution, predict outcomes, validate contracts, and generate conforming implementations from the contract alone.

If you removed all mention of AI from this project description, it would still be compelling. Agents amplify the need — they do not create it. The fragmentation problem exists for humans right now. Agents make it acute.

---

## 2. Design Constraints

These constraints are non-negotiable. They are not guidelines. Any proposed feature that violates them must be rejected regardless of ergonomic benefit. They are listed here so that any agent or implementer reading this spec can use them as a rejection filter.

**C1 — Decidability.** The language is non-Turing complete by design. No construct, alone or in composition, may enable arbitrary computation.

**C2 — Termination.** Evaluation must terminate for all valid contracts. Termination is a structural property of the language, not a property verified at runtime.

**C3 — Determinism.** Given identical inputs, evaluation must produce identical outputs across all conforming implementations.

**C4 — Static analyzability.** The complete state space, all reachable states, all admissible operations per state, all authority boundaries, and all possible verdict outcomes are derivable by static analysis of the contract alone, without execution.

**C5 — Closed-world semantics.** The contract is the complete description of the system. There are no implicit behaviors, no ambient authorities, no external references that affect evaluation semantics.

**C6 — Explicit over implicit.** No authority, propagation, dependency, or evaluation order may be inferred. Everything must be declared.

**C7 — Provenance as semantics.** Provenance is not a logging feature. Every value in the evaluation relation carries its derivation. The audit log is a theorem derived from the evaluation trace, not a constructed artifact.

---

## 3. Core Constructs Overview

The language defines eleven constructs across three layers.

**Semantic layer** (dependency order — each depends only on those above):

```
BaseType            — closed value type set (includes Duration)
Fact                — ground typed assertions; the evaluation root
Entity              — finite state machines in a static DAG
Rule                — stratified verdict-producing evaluation (includes variable×variable)
Operation           — persona-gated, precondition-guarded state transitions
PredicateExpression — quantifier-free FOL with arithmetic and bounded quantification
Flow                — finite DAG orchestration with sequential and parallel steps
NumericModel        — fixed-point decimal with total promotion rules (cross-cutting)
```

Named type aliases (TypeDecl) are a DSL-layer convenience only. The elaborator resolves all named type references during Pass 3 and inlines the full BaseType structure at every point of use. TypeDecl does not appear in TenorInterchange output.

**Tooling layer:**

```
Tenor            — human authoring syntax (elaborates to TenorInterchange)
ElaboratorSpec      — six-pass deterministic DSL→Interchange transformation
TenorInterchange    — canonical JSON bundle (single source of truth for all tooling)
```

NumericModel is cross-cutting — it applies to BaseType, Fact, and PredicateExpression. It has no single position in the dependency chain but must be fully specified before any numeric computation is well-defined.

The evaluation model separates into a read path and a write path:

```
Read path:     assemble_facts → eval_strata → resolve → ResolvedVerdictSet
Write path:    execute(op, persona, verdict_set) → EntityState' | Error
Orchestration: execute_flow(flow, persona, snapshot) → FlowOutcome
               execute_parallel(branches, snapshot) → {BranchId → BranchOutcome}
               evaluate_join(join_policy, branch_outcomes) → StepTarget
```

Every step in both paths is bounded, deterministic, and statically analyzable.

---

## 4. BaseType

### 4.1 Definition

BaseType is the closed set of value types available in the language. The set is finite and fully enumerated. No user-defined base types are permitted. All type checking is decidable at contract load time. No runtime type errors occur in a well-typed contract.

```
BaseType ::=
  Bool
  | Int(min: int, max: int)
  | Decimal(precision: nat, scale: nat)
  | Text(max_length: nat)
  | Enum(values: [string])
  | Date
  | DateTime
  | Money(currency: CurrencyCode)
  | Record(fields: { name → BaseType })
  | TaggedUnion(variants: { tag → BaseType })
  | List(element_type: ScalarBaseType, max: nat)
  | Duration(unit: DurationUnit, min: int, max: int)

DurationUnit    ::= "seconds" | "minutes" | "hours" | "days"
ScalarBaseType  ::= Bool | Int(...) | Decimal(...) | Text(...) | Enum(...)
                  | Date | DateTime | Money(...) | Record(...) | TaggedUnion(...)
                  | Duration(...)

Elaborator-internal node (must not appear in interchange output):
  TypeRef(id: TypeId)   — named type reference; resolved to full BaseType during Pass 4
```

List element types must be ScalarBaseType — nested lists are not permitted.

Duration values represent exact, fixed time spans. "Day" means exactly 86,400 seconds — not a calendar day. Month and year are not Duration units; calendar-relative spans must be expressed as Facts populated by adapters.

### 4.2 Operator Definitions

```
Bool:           =, ≠, ∧, ∨, ¬
Int, Decimal:   =, ≠, <, ≤, >, ≥, +, -, × literal_n
                cross-precision arithmetic follows NumericModel promotion rules
Money:          =, ≠, <, ≤, >, ≥, +, - (same currency only)
Text:           =, ≠
Enum:           =, ≠, ∈ declared_values
Date, DateTime: =, ≠, <, ≤, >, ≥
Record:         =, ≠ (field-wise)
                field access: record.field_name → field's BaseType
TaggedUnion:    =, ≠ (tag + payload, field-wise)
                tag-embedded access: union.tag.field_name → field's BaseType | absence
                absence in predicate context evaluates to false
List:           len(list) → Int
                element access: list[i] → element_type (bounds-checked)
Duration:       =, ≠, <, ≤, >, ≥
                Duration + Duration → Duration (same unit, or promoted to smaller unit)
                Duration - Duration → Duration (same unit, or promoted to smaller unit)
                DateTime + Duration → DateTime
                DateTime - DateTime → Duration(seconds, derived_bounds)

Duration promotion rules (cross-unit arithmetic promotes to the smaller unit):
  Duration(days)    + Duration(hours)   → Duration(hours),   days × 24
  Duration(days)    + Duration(minutes) → Duration(minutes), days × 1440
  Duration(days)    + Duration(seconds) → Duration(seconds), days × 86400
  Duration(hours)   + Duration(minutes) → Duration(minutes), hours × 60
  Duration(hours)   + Duration(seconds) → Duration(seconds), hours × 3600
  Duration(minutes) + Duration(seconds) → Duration(seconds), minutes × 60
```

### 4.3 Type Checking

```
type_check : Value × BaseType → Bool | Error

type_check(v, Bool)                  = v ∈ {true, false}
type_check(v, Int(min, max))         = v ∈ ℤ ∧ min ≤ v ≤ max
type_check(v, Decimal(p, s))         = v representable in fixed-point(p, s)
type_check(v, Text(n))               = |v| ≤ n ∧ v is valid UTF-8
type_check(v, Enum(vals))            = v ∈ vals
type_check(v, Date)                  = v is valid ISO 8601 date
type_check(v, DateTime)              = v is valid ISO 8601 datetime, UTC-normalized
type_check(v, Money(c))              = type_check(v.amount, Decimal) ∧ v.currency = c
type_check(v, Record(fields))        = ∀ (name, type) ∈ fields: type_check(v[name], type)
type_check(v, TaggedUnion(vars))     = v.tag ∈ vars ∧ type_check(v.payload, vars[v.tag])
type_check(v, List(elem_type, max))  = len(v) ≤ max ∧ ∀ e ∈ v: type_check(e, elem_type)
type_check(v, Duration(unit, min, max)) = v ∈ ℤ ∧ min ≤ v ≤ max
```

### 4.4 Constraints

- Record and TaggedUnion type declarations must form an acyclic graph. Self-referential type declarations are prohibited. Cycle detection is performed at contract load time via DFS over the type declaration graph.
- DateTime values are normalized to UTC at FactSet assembly time. Timezone offset information from source data is discarded after normalization.
- Money arithmetic is same-currency only. Cross-currency operations require explicit Rule-level conversion.
- Duration "day" means exactly 86,400 seconds. DST-affected calculations must be handled by adapters. Month and year are not supported Duration units.
- DateTime subtraction produces Duration(seconds) with bounds derived from the operand DateTime types. The elaborator computes and materializes the result bounds.
- TaggedUnion payload access uses tag-embedded paths. Mismatched tag access produces a typed absence value, which evaluates to false in predicate context. Absence is not a third logical value — the language is strictly two-valued (Bool). Absence is always coerced to false before any boolean operation. Consequently, `¬(union.tag.field > threshold)` evaluates to `¬false = true` when the tag does not match, not to unknown or absence. Contract authors must account for this: a negated predicate over a TaggedUnion field will evaluate to true for all non-matching variants. Where this is not the intended behavior, authors should gate the predicate with an explicit tag check.

---

### 4.5 Named Type Declarations (TypeDecl)

A TypeDecl assigns a name to a Record or TaggedUnion BaseType, making it referenceable by name from Fact type fields, List element_type positions, and other TypeDecl definitions. Named types allow complex structured types to be declared once and referenced by name throughout a contract.

```
TypeDecl = (
  id:   TypeId,
  type: Record(fields: { name → BaseType })
      | TaggedUnion(variants: { tag → BaseType })
)
```

Named type aliases for scalar BaseTypes (Bool, Int, Decimal, Text, Enum, Date, DateTime, Money, Duration, List) are not permitted. Only Record and TaggedUnion benefit from naming.

**DSL syntax:**

```
type LineItemRecord {
  id:          Text(max_length: 64)
  description: Text(max_length: 256)
  amount:      Money(currency: "USD")
  valid:        Bool
}
```

A TypeDecl may be referenced anywhere a Record or TaggedUnion BaseType is expected:

```
Fact line_items {
  type:   List(element_type: LineItemRecord, max: 100)
  source: "order_service.line_items"
}
```

**Interchange:** TypeDecl is a DSL-layer construct only. The elaborator resolves named type references during Pass 3 (type environment construction) and inlines the full BaseType structure at every point of use during Pass 4 (AST materialization). TypeDecl entries do not appear in the interchange bundle. Interchange is fully self-contained — no TypeRef lookups are required to interpret any interchange document.

**Constraints:**

- TypeDecl ids are unique within a contract. Two TypeDecls may not share an id.
- TypeDecl ids occupy a distinct namespace from other construct kinds. A TypeDecl named `Foo` does not conflict with an Entity named `Foo`.
- The TypeDecl reference graph must be acyclic. If TypeDecl A contains a field of type TypeDecl B, and TypeDecl B contains a field of type TypeDecl A, this is a declaration cycle and is a Pass 3 error. Cycle detection uses DFS over the TypeDecl reference graph.
- A TypeDecl may only alias Record or TaggedUnion types.

---

## 5. Fact

### 5.1 Definition

A Fact is a named, typed, sourced value that forms the ground layer of the evaluation model. Facts are assembled into an immutable FactSet before rule evaluation begins. No Fact is derived by any rule, produced by any operation, or computed by any internal evaluation.

```
Fact = (
  id:       FactId,
  type:     BaseType,
  source:   SourceDecl,
  value?:   BaseType,
  default?: BaseType
)
```

Every Fact has a declared source — an explicit statement of where the value comes from at runtime. A Fact without a declared source is inadmissible.

### 5.2 FactSet Assembly

```
assemble_facts : Contract × ExternalInputs → FactSet | Abort

assemble_facts(contract, inputs) =
  for each fact f in contract.declared_facts:
    if inputs[f.id] present:
      type_check(inputs[f.id], f.type) or Abort("type error: " + f.id)
      FactSet[f.id] = (value: inputs[f.id], assertion_source: "external")
    elif f.default present:
      FactSet[f.id] = (value: f.default, assertion_source: "contract")
    else:
      Abort("missing fact: " + f.id)
  return FactSet  // immutable from this point
```

For List-typed Facts, assembly additionally checks:

```
if len(asserted_list) > f.type.max → Abort("list exceeds declared max: " + f.id)
for each element in asserted_list:
  type_check(element, f.type.element_type) or Abort("type error in list element")
```

### 5.3 Provenance

Facts are the provenance origin points of the evaluation model. Every provenance chain terminates at one or more Facts. A Fact's provenance record is:

```
FactProvenance = (
  id:               FactId,
  source:           SourceDecl,
  value:            BaseType,
  assertion_source: "external" | "contract"
)
```

Facts do not carry derivation provenance — they are the root. When a default is used, `assertion_source` is `"contract"` to distinguish from externally asserted values.

### 5.4 Constraints

- Fact identifiers are unique within a contract.
- The complete set of Fact identifiers is statically enumerable from the contract.
- Fact identifiers are fixed at contract definition time. No Fact may be dynamically named or dynamically typed.
- The ground property is enforced within the evaluation model. Whether a conforming executor populates Facts from genuinely external sources is outside the language's enforcement scope. Conformance requires this.

---

## 6. Entity

### 6.1 Definition

An Entity is a finite state machine embedded in a closed-world, well-founded, acyclic partial order. The entity graph is static and finite. The partial order carries no implicit authority or conflict semantics. Any propagation across the hierarchy must be explicitly declared, monotonic, and statically analyzable.

```
Entity = (
  id:          EntityId,
  states:      Set<StateId>,
  initial:     StateId,
  transitions: Set<(StateId × StateId)>,
  parent?:     EntityId
)
```

### 6.2 Entity DAG Properties

Let E be the set of all entities in a contract:

- |E| is finite and fixed at contract definition time. No dynamic entity creation is permitted.
- For each entity e ∈ E, S(e) is finite and explicitly enumerated.
- For each entity e ∈ E, T(e) ⊆ S(e) × S(e) is finite and explicitly declared.
- The entity hierarchy forms a directed acyclic graph. The transitive closure of the parent relation must be irreflexive.
- The DAG structure is fixed at contract definition time. No dynamic re-parenting is permitted.

### 6.3 Propagation Semantics

Any propagation of properties across the entity hierarchy must be explicitly declared — naming the source entity, the target entity or subtree, the property being propagated, and the direction. Propagation is monotonic: a propagated property may not be negated by a child entity. Propagation evaluation is a single pass over the topologically sorted entity DAG. No fixed-point iteration is required or permitted.

The entity partial order defines no implicit lattice join or meet operators. No conflict resolution is derived from the DAG structure itself.

### 6.4 State Machine Semantics

Each entity e defines a finite non-empty set of states S(e), an explicitly declared initial state s₀(e) ∈ S(e), and an explicitly declared transition relation T(e) ⊆ S(e) × S(e). The current state of an entity instance is a value in S(e). State is not derived — it is stored and updated by Operations exclusively.

---

## 7. Rule

### 7.1 Definition

A Rule is a stratified, verdict-producing, side-effect-free evaluation function. Rules read from Facts and lower-stratum verdict outputs. They produce verdict instances. They do not modify entity state. They do not invoke Operations.

```
Rule = (
  id:      RuleId,
  stratum: nat,
  body:    RuleBody    // produces Set<VerdictInstance>
)
```

Rule bodies may contain variable × variable multiplication of Int-typed Facts. The result range is computed from the operand Fact type ranges and must be contained within the declared verdict payload type. This is the only location in the language where variable × variable multiplication is permitted — it is not available in PredicateExpression.

### 7.2 Verdict Definition

Verdicts form a finite tagged set. Each verdict type has a name, a payload schema, and a declared precedence class.

```
VerdictType = (
  name:             string,
  payload_schema:   BaseType,
  precedence_class: string
)

VerdictInstance = (
  type:       VerdictType,
  payload:    BaseType,
  provenance: VerdictProvenance
)

VerdictProvenance = (
  rule:          RuleId,
  stratum:       nat,
  facts_used:    Set<FactId>,
  verdicts_used: Set<VerdictInstanceId>,
  contract:      ContractId
)
```

### 7.3 Stratification

Rules are assigned to explicit, numbered strata. A rule in stratum N may only reference outputs from rules in strata strictly below N. No same-stratum references are permitted regardless of whether the intra-stratum dependency graph would be acyclic. Multiple rules may occupy the same stratum provided they have no dependencies on each other.

Formally: for any rule r that references the output of rule r′, stratum(r′) < stratum(r). Equality is not permitted.

**Re-expression theorem:** Any acyclic same-stratum dependency graph can be mechanically transformed into strict stratification with no loss of expressive power. Tooling should implement this transformation to ease authoring burden.

### 7.4 Rule Evaluation

```
eval_strata : Contract × FactSet → ResolvedVerdictSet

eval_strata(contract, facts) =
  verdicts = ∅
  for n = 0 to max_stratum:
    stratum_rules = { r ∈ contract.rules | r.stratum = n }
    new_verdicts  = ⋃ { eval_rule(r, facts, verdicts) | r ∈ stratum_rules }
    verdicts      = verdicts ∪ new_verdicts
  return resolve(verdicts)
```

This fold terminates because the number of strata is finite and each stratum is fully evaluated before the next.

### 7.5 Verdict Resolution

The resolution function `resolve : Set<VerdictInstance> → ResolvedVerdictSet` is total, deterministic, explicitly specified in the contract, and provenance-preserving. Resolution does not implicitly merge incompatible verdict types. Any precedence or dominance relations must be explicitly declared and statically analyzable.

### 7.6 Cross-Currency Conversion Pattern

Cross-currency operations are not language primitives. They are expressed using the existing Fact + Rule pattern. No new constructs are required.

```
// 1. Declare the conversion rate as a Fact
fact usd_to_eur_rate {
  type:   Decimal(8, 6)
  source: fx_service.usd_eur_rate
}

// 2. Produce the converted amount as a verdict
rule convert_escrow_to_eur {
  stratum: 0
  when:    usd_to_eur_rate present
  produce: escrow_eur_equivalent(
    escrow_amount.amount * usd_to_eur_rate
  )
}

// 3. Use in downstream rules
rule eur_within_threshold {
  stratum: 1
  when:    escrow_eur_equivalent present
         and escrow_eur_equivalent <= eur_compliance_threshold
  produce: eur_threshold_met(true)
}
```

Rate staleness is an executor obligation (E1). Multi-hop conversions require chained Rules across strata. Each conversion step is separately provenance-tracked.

---

## 8. Operation

### 8.1 Definition

An Operation is a declared, persona-gated, precondition-guarded unit of work that produces entity state transitions as its sole side effect. Operations are the only construct in the evaluation model that produces side effects.

```
Operation = (
  id:               OperationId,
  allowed_personas: Set<PersonaId>,
  precondition:     PredicateExpression,              // over ResolvedVerdictSet
  effects:          Set<(EntityId × StateId × StateId)>,  // entity-scoped transitions
  error_contract:   Set<ErrorType>
)
```

### 8.2 Evaluation

```
execute : Operation × PersonaId × ResolvedVerdictSet × EntityState → EntityState' | Error

execute(op, persona, verdict_set, entity_state) =
  if persona ∉ op.allowed_personas:
    return Error(op.error_contract, "persona_rejected")
  if ¬eval_pred(op.precondition, FactSet, verdict_set):
    return Error(op.error_contract, "precondition_failed")
  // Executor obligation: validate entity_state matches transition source for each effect
  // apply_atomic applies each (entity_id, from, to) transition atomically
  apply_atomic(op.effects, entity_states) → entity_states'
  emit_provenance(op, persona, verdict_set, entity_state, entity_state')
  return entity_state'
```

### 8.3 Execution Sequence

The execution sequence is fixed and invariant: (1) persona check, (2) precondition evaluation, (3) atomic effect application, (4) provenance emission. No step may be reordered. No step may be skipped if the preceding step succeeds.

### 8.4 Constraints

- An Operation with an empty `allowed_personas` set is a contract error detectable at load time.
- Each declared effect `(entity_id, from_state, to_state)` must reference an entity declared in the contract, and the transition `(from_state, to_state)` must exist in that entity's declared transition relation. Checked at contract load time. An Operation may declare effects across multiple entities — each is validated independently.
- Operations do not produce verdict instances. Verdict production belongs exclusively to Rules.
- Atomicity is an executor obligation. Either all declared state transitions occur, or none do.
- The executor must validate that the current entity state matches the transition source for each declared effect. This is an executor obligation not encoded in the Operation formalism.

### 8.5 Provenance

```
OperationProvenance = (
  op:            OperationId,
  persona:       PersonaId,
  facts_used:    Set<FactId>,
  verdicts_used: ResolvedVerdictSet,
  state_before:  EntityState,
  state_after:   EntityState'
)
```

---

## 9. PredicateExpression

### 9.1 Definition

A PredicateExpression is a quantifier-free first-order logic formula over ground terms drawn from the resolved FactSet, the resolved VerdictSet, and literal constants declared in the contract. All terms are ground at evaluation time. No free variables. No recursion. No side effects.

### 9.2 Grammar

```
Pred ::=
  Atom
  | Pred ∧ Pred
  | Pred ∨ Pred
  | ¬ Pred
  | ∀ var : T ∈ list_ref . Pred(var)
  | ∃ var : T ∈ list_ref . Pred(var)

Atom ::=
  fact_ref op literal
  | fact_ref op fact_ref
  | verdict_present(verdict_id)
  | ArithExpr op ArithExpr

ArithExpr ::=
  fact_ref_numeric
  | literal_numeric
  | ArithExpr + ArithExpr
  | ArithExpr - ArithExpr
  | ArithExpr * literal_numeric    // literal only — no variable × variable

op       ::= = | ≠ | < | ≤ | > | ≥

list_ref ::=
  fact_id                          // top-level List-typed Fact
  | fact_id.field_name             // List-typed field of a Record Fact
```

### 9.3 Evaluation

```
eval_pred : PredicateExpression × FactSet × VerdictSet → Bool

eval_pred(fact_ref op literal, F, V)      = F[fact_ref] op literal
eval_pred(fact_ref op fact_ref', F, V)    = F[fact_ref] op F[fact_ref']
eval_pred(verdict_present(vid), F, V)     = vid ∈ V
eval_pred(ArithExpr op ArithExpr', F, V)  = eval_arith(ArithExpr,F) op eval_arith(ArithExpr',F)
eval_pred(P ∧ Q, F, V)                    = eval_pred(P,F,V) ∧ eval_pred(Q,F,V)
eval_pred(P ∨ Q, F, V)                    = eval_pred(P,F,V) ∨ eval_pred(Q,F,V)
eval_pred(¬P, F, V)                       = ¬eval_pred(P,F,V)

eval_pred(∀ x : T ∈ list_ref . P(x), F, V) =
  list = resolve_list_ref(list_ref, F)
  ⋀ { eval_pred(P(elem), F ∪ {x: elem}, V) | elem ∈ list }

eval_pred(∃ x : T ∈ list_ref . P(x), F, V) =
  list = resolve_list_ref(list_ref, F)
  ⋁ { eval_pred(P(elem), F ∪ {x: elem}, V) | elem ∈ list }

eval_arith(fact_ref, F)      = F[fact_ref]
eval_arith(literal_n, F)     = n
eval_arith(e + e', F)        = eval_arith(e,F) + eval_arith(e',F)    // per NumericModel
eval_arith(e - e', F)        = eval_arith(e,F) - eval_arith(e',F)    // per NumericModel
eval_arith(e * literal_n, F) = eval_arith(e,F) * n                   // per NumericModel

resolve_list_ref(fact_id, F)       = F[fact_id]
resolve_list_ref(fact_id.field, F) = F[fact_id].field
```

### 9.4 Constraints

- No implicit type coercions. A comparison between incompatible types is a load-time contract error.
- Quantification domains must be List-typed Facts or List-typed fields of Record Facts with declared max bounds.
- Quantification over verdict sets, entity state sets, or unbounded collections is not permitted.
- Variable × variable multiplication is not permitted.
- All arithmetic follows the NumericModel.

### 9.5 Complexity

Scalar predicate evaluation is O(|expression tree|). Quantified predicate evaluation is O(max × |body|) per quantifier level. Nested quantifiers multiply bounds. All bounds are statically derivable from declared maxes and expression tree size.

---

## 10. Flow

### 10.1 Definition

A Flow is a finite, directed acyclic graph of steps that orchestrates the execution of declared Operations under explicit persona control. Flows do not compute — they sequence. All business logic remains in Rules and Operations.

```
Flow = (
  id:       FlowId,
  entry:    StepId,
  steps:    { StepId → Step },
  snapshot: SnapshotPolicy    // always "at_initiation" in v1
)
```

### 10.2 Step Types

```
Step =
  OperationStep(
    op:         OperationId,
    persona:    PersonaId,
    outcomes:   { OutcomeLabel → StepId | Terminal },
    on_failure: FailureHandler
  )
  | BranchStep(
    condition:  PredicateExpression,
    persona:    PersonaId,
    if_true:    StepId | Terminal,
    if_false:   StepId | Terminal
  )
  | HandoffStep(
    from_persona: PersonaId,
    to_persona:   PersonaId,
    next:         StepId
  )
  | SubFlowStep(
    flow:       FlowId,
    persona:    PersonaId,
    on_success: StepId | Terminal,
    on_failure: FailureHandler
  )
  | Terminal(outcome: "success" | "failure" | "escalation")
  | ParallelStep(
      branches: [Branch],
      join:     JoinPolicy
    )

Branch = (
  id:    BranchId,
  entry: StepId,
  steps: { StepId → Step }
)

JoinPolicy = (
  on_all_success:  StepId | Terminal,
  on_any_failure:  FailureHandler,
  on_all_complete: StepId | Terminal | null
)

// Permitted merge conditions: all_success, any_failure, all_complete
// first_success is not permitted — all branches run to completion
```

### 10.3 Failure Handling

```
FailureHandler =
  Terminate(outcome: Terminal)
  | Compensate(steps: [CompensationStep], then: Terminal)
  | Escalate(to_persona: PersonaId, next: StepId)

CompensationStep = (
  op:         OperationId,
  persona:    PersonaId,
  on_failure: Terminal    // Terminal only — no nested compensation
)
```

Every OperationStep and SubFlowStep must declare a FailureHandler. A missing FailureHandler is a contract error detectable at load time.

### 10.4 Evaluation

**Frozen verdict semantics:** Within a Flow, the ResolvedVerdictSet is computed once at Flow initiation and is not recomputed after intermediate Operation execution. Operations within a Flow do not see entity state changes produced by preceding steps in the same Flow. This is a fundamental semantic commitment: Flows are pure decision graphs over a stable logical universe. The consequence is that a Rule whose inputs include entity state will not reflect mid-Flow transitions — such patterns must be expressed across Flow boundaries, not within them.

```
execute_flow : Flow × PersonaId × Snapshot → FlowOutcome

execute_flow(flow, initiating_persona, snapshot) =
  current = flow.entry
  loop:
    step = flow.steps[current]
    match step:
      OperationStep →
        result  = execute(step.op, step.persona, snapshot.verdict_set)
        emit_provenance(step, result)
        current = step.outcomes[classify(result)] or handle_failure(step.on_failure)
      BranchStep →
        val     = eval_pred(step.condition, snapshot.facts, snapshot.verdict_set)
        emit_branch_record(step, val)
        current = if val then step.if_true else step.if_false
      HandoffStep →
        emit_handoff_record(step)
        current = step.next
      SubFlowStep →
        // snapshot inherited — not re-taken at sub-flow initiation
        result  = execute_flow(lookup(step.flow), step.persona, snapshot)
        emit_provenance(step, result)
        current = if success(result) then step.on_success
                  else handle_failure(step.on_failure)
      Terminal →
        return step.outcome
```

### 10.5 Constraints

- Step graph must be acyclic. Verified at load time via topological sort.
- All StepIds referenced in a Flow must exist in the steps map.
- All OperationIds referenced must exist in the contract.
- Flow reference graph (SubFlowStep references) must be acyclic. Verified via DFS across all contract files.
- Sub-flows inherit the invoking Flow's snapshot. Sub-flows do not take independent snapshots.
- Typed outcome routing is Flow-side classification only. The Operation canonical form is unchanged.
- Compensation failure handlers are Terminal only. No nested compensation.
- Parallel branches execute under the parent Flow's frozen snapshot. No branch sees entity state changes produced by another branch during execution.
- No two parallel branches may declare effects on overlapping entity sets. Verified at contract load time by transitively resolving all Operation effects across all branches.
- All branches run to completion before the join evaluates. Branch execution order is implementation-defined. The join outcome is a function of the set of branch terminal outcomes, not their order.
- `first_success` merge policy is not supported.

### 10.6 Provenance

```
FlowProvenance = [StepRecord]

StepRecord =
  OperationRecord(op_provenance: OperationProvenance)
  | BranchRecord(condition: PredicateExpression, result: Bool, persona: PersonaId)
  | HandoffRecord(from: PersonaId, to: PersonaId)
  | SubFlowRecord(flow: FlowId, provenance: FlowProvenance)
```

Flow provenance is the ordered composition of step-level records. No Flow-level information exists outside what is captured at the step level.

---

## 11. NumericModel

### 11.1 Definition

All numeric computation uses fixed-point decimal arithmetic. No floating-point arithmetic is permitted in conforming implementations. Integer arithmetic is exact within declared range. Decimal arithmetic uses declared precision and scale.

### 11.2 Promotion Rules

The promotion function is total over all numeric type combinations:

```
Int(a,b) + Int(c,d)              → Int(a+c, b+d)
Int(a,b) - Int(c,d)              → Int(a-d, b-c)
Int(a,b) * literal_n             → Int(a*n, b*n) if n≥0 else Int(b*n, a*n)
Decimal(p1,s1) + Decimal(p2,s2)  → Decimal(max(p1,p2)+1, max(s1,s2))
Decimal(p1,s1) - Decimal(p2,s2)  → Decimal(max(p1,p2)+1, max(s1,s2))
Decimal(p,s) * literal_n         → Decimal(p + digits(n), s)
Int op Decimal(p,s)              → promote Int to Decimal(ceil(log10(max(|min|,|max|)))+1, 0)
                                    then apply Decimal rules
any op integer_literal           → literal typed as Int(n,n), then Int rules
any op decimal_literal           → literal typed as Decimal(digits, frac_digits), then rules
```

### 11.3 Overflow

Arithmetic that produces a result outside the declared range aborts evaluation with a typed overflow error. Silent wraparound and saturation are not permitted.

### 11.4 Rounding

Where a Decimal result has more fractional digits than the declared scale, rounding is applied. The rounding mode is **round half to even** (IEEE 754 roundTiesToEven). This mode is mandatory for all conforming implementations.

### 11.5 Literal Types

Integer literals are typed as Int(n, n). Decimal literals are typed as Decimal(total_digits, fractional_digits) derived from the literal's written form.

---

## 12. ElaboratorSpec

### 12.1 Overview

The elaborator is the trust boundary between human authoring and formal guarantees. It transforms Tenor source into a valid TenorInterchange bundle through six deterministic, ordered passes. A bug in the elaborator is more dangerous than a bug in the executor — it silently produces malformed interchange that the executor then operates on correctly, producing wrong results from correct execution.

A conforming elaborator must be deterministic: identical DSL input produces byte-for-byte identical interchange output on every invocation. No environmental dependency (timestamp, process id, random seed) may affect the output.

### 12.2 Elaboration Passes

**Pass 0 — Lexing and parsing**
Input: DSL source text (UTF-8). Output: Parse tree.

- Tokenize per Tenor lexer spec. `->` and `→` are the same token.
- Strip `//` and `/* */` comments. Stripped comments do not appear in the parse tree.
- Parse per Tenor grammar. Non-conforming input is rejected with errors satisfying I4.
- Record source file and line for every parse tree node.

**Pass 1 — Import resolution and bundle assembly**
Input: Parse trees from all DSL files. Output: Unified parse tree.

- Resolve all `import` declarations. Missing files are elaboration errors.
- Detect import cycles. Cycles are elaboration errors.
- Merge parse trees into a unified bundle. Duplicate construct ids across files are elaboration errors.

**Pass 2 — Construct indexing**
Input: Unified parse tree. Output: Construct index keyed by `(construct_kind, id)`.

- Build index of all declared construct ids.
- Same-kind duplicate ids are elaboration errors. Same-id, different-kind pairs do not conflict.

**Pass 3 — Type environment construction**
Input: Construct index (TypeDecl, Fact, and VerdictType declarations). Output: Type environment.

- Resolve all declared TypeDecl definitions. Detect cycles in the TypeDecl reference graph via DFS. Build named type lookup table.
- Resolve all BaseTypes in Fact and VerdictType declarations, expanding named type references using the TypeDecl lookup table. Detect any remaining Record/TaggedUnion declaration cycles.
- Build complete type environment before any expression type-checking begins.
- TypeDecl entries are consumed during type environment construction. They do not propagate to interchange output.

**Pass 4 — Expression type-checking and AST materialization**
Input: Unified parse tree, type environment, construct index. Output: Typed expression AST nodes.

- Type-check all PredicateExpressions. Type errors are elaboration errors.
- Resolve all fact_refs to declared FactIds. Resolve all verdict_refs to declared VerdictTypeIds.
- Apply NumericModel promotion rules. Materialize promoted types on all arithmetic and comparison nodes.
- For quantified expressions: verify domain is a List-typed Fact or List-typed Record field.
- For Rule body variable × variable multiplication: verify Int types, compute product range, verify containment in declared verdict payload type.
- For DateTime - DateTime: compute and materialize Duration result bounds.
- Resolve all TypeRef nodes (named type references from DSL) to their full BaseType structures. No TypeRef nodes appear in the interchange output.
- Expand all DSL shorthand to canonical interchange forms.
- **Error attribution:** elaboration pass functions receive AST nodes directly and destructure the node's embedded `line` field to obtain the source line for error reporting. Error lines are never derived from an enclosing construct's opening-keyword line when a more specific node line is available.

**Pass 5 — Construct validation**
Input: Typed ASTs, construct index, type environment. Output: Validation report.

- Entity: initial ∈ states; transition endpoints ∈ states; hierarchy acyclic.
- Operation: allowed_personas non-empty; effect entity_ids resolve; effect transitions exist in entity; effects ⊆ entity.transitions.
- Rule: stratum ≥ 0; all refs resolve; verdict_refs reference strata < this rule's stratum; produce clauses reference declared VerdictType ids (unresolved VerdictType references are Pass 5 errors).
- Flow: entry exists; all step refs resolve; step graph acyclic; flow reference graph acyclic; all OperationSteps and SubFlowSteps declare FailureHandlers.
- Parallel: branch sub-DAGs acyclic; no overlapping entity effect sets across branches (transitively resolved).
- **Error attribution:** errors are reported at the source line of the specific field or sub-expression responsible for the violation (e.g., the `initial:` field line, not the `Entity` keyword line; the `verdict_present(...)` call line, not the enclosing `Rule` keyword line). This requires AST nodes at all levels — RawExpr variants, RawStep variants, construct sub-field lines — to carry their own source line, set by the parser at token consumption time and treated as immutable by all elaboration passes.

**Pass 6 — Interchange serialization**
Input: Validated construct index with typed ASTs. Output: TenorInterchange JSON bundle.

- Canonical construct order: VerdictTypes, Facts, Entities, Rules (ascending stratum, alphabetical within stratum), Operations (alphabetical), Flows (alphabetical).
- Serialize Flow steps as an array. Entry step is first; remaining steps follow in topological order of the step DAG.
- Sort all JSON object keys lexicographically within each construct document.
- Represent all Decimal, Money, and Duration values as structured typed objects. No JSON native floats for Decimal or Money values.
- Attach provenance blocks (file, line) to all top-level construct documents.
- Preserve DSL source order for commutative binary expression operands.
- Preserve DSL declaration order for all array values. Array values are never sorted.
- Emit `"tenor"` version and `"kind"` on every top-level document.

### 12.3 Error Reporting Obligation

Every elaboration error must identify: construct kind, construct id (if determinable), field name, source file, source line, and a human-readable description of the violation. Errors referencing internal elaborator state or elaborator-internal terminology are not conforming.

### 12.4 Conformance Test Categories

A conforming elaborator must pass all tests in the Tenor Elaborator Conformance Suite:

- **Positive tests:** Valid DSL that must elaborate without error and produce specific expected interchange output byte-for-byte.
- **Negative tests:** Invalid DSL that must be rejected with errors at specific fields and locations.
- **Numeric precision tests:** Decimal and Money values that must produce exact `decimal_value` interchange representations.
- **Type promotion tests:** Mixed numeric expressions that must produce correctly promoted `comparison_type` fields.
- **Shorthand expansion tests:** All shorthand forms must produce interchange identical to their fully explicit equivalents.
- **Cross-file reference tests:** Multi-file bundles with cross-file refs that must resolve correctly.
- **Parallel entity conflict tests:** Parallel blocks with overlapping entity effects that must be rejected.

_Note: The Tenor Elaborator Conformance Suite is at `conformance/`. It is a prerequisite for any implementation to be declared conforming._

## 13. Complete Evaluation Model

### 13.1 Contract Load Time

The following checks are performed when a contract is loaded. A contract that fails any check is inadmissible.

```
1.  type_check(contract)
    — all declared types are well-formed
    — Record and TaggedUnion declaration graphs are acyclic
    — List element types are ScalarBaseType

2.  acyclicity_check(entity_dag)
    — entity parent relation is acyclic

3.  acyclicity_check(flow_step_dag)
    — each Flow's step graph is acyclic

4.  acyclicity_check(flow_reference_dag)
    — SubFlowStep references are acyclic across all Flows

5.  subset_check(op.effects ⊆ entity.transitions)
    — for each effect (entity_id, from_state, to_state) in op.effects:
      entity_id must exist in the contract, and
      (from_state, to_state) must exist in entity.transitions for that entity

6.  persona_check(op.allowed_personas ≠ ∅)
    — no Operation has an empty allowed_personas set

7.  failure_handler_check
    — every OperationStep and SubFlowStep declares a FailureHandler

8.  stratum_check
    — for all rules r, r': if r references output of r' then stratum(r') < stratum(r)
```

### 13.2 Flow Initiation

```
snapshot = take_snapshot(contract, current_rules, current_entity_states)
// Point-in-time. Rule evolution after this point does not affect the Flow.
```

### 13.3 Per-Evaluation Sequence

```
// Read path
facts       = assemble_facts(contract, external_inputs)   // → FactSet | Abort
verdicts    = eval_strata(contract.rules, facts)           // → ResolvedVerdictSet

// Write path (per Operation invocation)
state'      = execute(op, persona, verdicts, state)        // → EntityState' | Error

// Orchestration
outcome     = execute_flow(flow, persona, snapshot)        // → FlowOutcome
```

### 13.4 Provenance Chain

Every terminal outcome has a complete provenance chain:

```
FlowOutcome
  └─ [StepRecord]
       └─ OperationProvenance
            ├─ ResolvedVerdictSet
            │    └─ [VerdictInstance]
            │         └─ VerdictProvenance
            │              └─ [FactId]
            │                   └─ FactProvenance  ← root
            ├─ EntityState (before)
            └─ EntityState (after)
```

Every chain terminates at Facts. Facts are the provenance roots. No derivation precedes them.

---

## 14. Static Analysis Obligations

A conforming static analyzer must derive the following from a contract alone, without execution:

**S1 — Complete state space.** For each Entity, the complete set of states S(e) is enumerable.

**S2 — Reachable states.** For each Entity, the set of states reachable from the initial state via the declared transition relation is derivable.

**S3a — Structural admissibility per state.**  
For each Entity state and each persona, the set of Operations whose preconditions are structurally satisfiable — given only type-level information, without enumerating domain values — and whose effects include a transition from that state is derivable. Structural satisfiability is type-level analysis: a precondition that compares a fact of type `Enum(["pending", "confirmed"])` with the literal `"approved"` is structurally unsatisfiable by type inspection alone. A precondition that compares two compatible typed facts is structurally satisfiable. This analysis is O(|expression tree|) per precondition and is always computationally feasible.

**S3b — Domain satisfiability per state** _(qualified — not always computationally feasible)_  
A stronger version of S3a: for each Entity state and each persona, determine whether there exists a concrete FactSet and VerdictSet assignment under which the precondition evaluates to true. This requires model enumeration over the product of Fact domain sizes. For facts with small declared domains (small Enum sets, narrow Int ranges, short List max bounds) this is feasible. For facts with large declared domains (wide Int ranges, large Decimal precision, long Text max lengths), the enumeration space is O(product of domain sizes), which may be astronomically large for realistic contracts. S3b is decidable in principle for all valid Tenor contracts, but is not computationally feasible in general. Static analysis tools implementing S3b should document their domain size thresholds and fall back to S3a when enumeration is infeasible. S3b should not be treated as an unconditional static analysis obligation.

**S4 — Authority topology.** For any persona P and Entity state S, the set of Operations P can invoke in S is derivable. Whether a persona can cause a transition from S to S′ is answerable.

**S5 — Verdict space.** The complete set of possible verdict types producible by a contract's rules is enumerable.

**S6 — Flow path enumeration.** For each Flow, the complete set of possible execution paths, all personas at each step, all entity states reachable via the Flow, and all terminal outcomes are derivable.

**S7 — Evaluation complexity bounds.** For each PredicateExpression, the evaluation complexity bound is statically derivable. For each Flow, the maximum execution depth is statically derivable.

---

## 15. Executor Obligations

### 15.1 The Conformance Gap

Tenor provides formal guarantees **conditional on executor conformance**. This conditionality is not a minor caveat — it is a structural property of the language that must be understood before building on it.

The language describes a closed world. Its evaluation model is fully specified, deterministic, and statically analyzable. But the foundations of that closed world — the actual values that populate Facts, the actual atomicity of state transitions, the actual isolation of Flow snapshots — depend on an open world: the executor, the storage substrate, and the runtime environment.

Where executor obligations are not met, the provenance chain is **corrupt**, not merely incomplete. A corrupt provenance chain is indistinguishable from a valid one at the language level. Tenor cannot detect non-conformance internally. The formal guarantees — determinism, closed-world semantics, provenance-as-theorem — hold only within the boundary of a conforming executor. Outside that boundary, they do not hold, and the language provides no mechanism to detect the violation.

This is not a temporary limitation to be resolved in a future version. It is the same gap that exists in every formal contract language that relies on external state. The appropriate response is to build attestation and verification mechanisms in the executor layer — not to expect the language to close the gap from within.

Implementers building on Tenor should treat E1, E3, and E4 in particular as **trust boundaries**, not implementation details.

### 15.2 Obligation Definitions

**E1 — External source integrity** _(trust boundary)_  
Facts are populated from genuinely external sources as declared. An executor must not populate Facts from internal computations or cross-Fact dependencies. Violation of E1 corrupts the provenance root — every chain built on a non-externally-sourced Fact is semantically invalid, but the language cannot detect this.

**E2 — Transition source validation.**  
Before applying an Operation's effects, the executor must validate that the current entity state matches the transition source for each declared effect `(entity_id, from_state, to_state)`. A mismatched transition source must abort the Operation with a typed error.

**E3 — Atomicity enforcement** _(trust boundary)_  
An Operation's effect set must be applied atomically. Either all declared state transitions occur, or none do. Partial application is not permitted. Atomicity depends on the executor's storage substrate. The language defines what atomicity means but cannot verify that it was achieved. Partial application that is not detected produces invalid entity states that subsequent Operations may act upon, silently compounding the error.

**E4 — Snapshot isolation** _(trust boundary)_  
A Flow's snapshot must not be modified after initiation. Rule evolution during Flow execution must not affect an in-progress Flow. Snapshot isolation depends on the executor's concurrency model. In a concurrent environment, a non-isolated snapshot may cause a Flow to make decisions against a logical universe that no longer exists.

**E5 — Sub-flow snapshot inheritance.**  
Sub-flows must execute under the invoking Flow's snapshot. A sub-flow must not take an independent snapshot at initiation.

**E6 — UTC normalization.**  
DateTime values must be normalized to UTC at FactSet assembly time, before any predicate evaluation.

**E7 — Numeric model conformance.**  
All arithmetic must use fixed-point decimal arithmetic per the NumericModel. Floating-point arithmetic is not permitted in any conforming executor. The round-half-to-even rounding mode must be used exclusively. Implementations in languages whose standard libraries do not natively support fixed-point decimal must implement the NumericModel explicitly — delegating to native floating-point arithmetic is not conforming regardless of the precision of the result in common cases.

**E8 — Branch isolation** _(trust boundary)_  
Parallel branches execute under the parent Flow's snapshot. No branch sees entity state changes produced by another branch during execution. Branch execution order is implementation-defined. The executor must enforce branch isolation — the language cannot detect violations of this obligation.

**E9 — Join evaluation after full branch completion.**  
The join step evaluates after all branches have reached a terminal state. The join condition is evaluated against the set of branch terminal outcomes. Order of branch completion does not affect join outcome.

---

## 16. Pending Work

**Resolved in v0.3:**

**P1 — Syntax definition.** Tenor v1.0 defined. See construct sections for DSL syntax; Appendix C for a complete worked example.

**P2 — Parallel execution semantics.** Resolved via ParallelStep (§10.2).

**P3 — Duration type.** Resolved via Duration BaseType (§4).

**P4 — Cross-currency Money operations.** Resolved via Rule-layer conversion pattern (§7.6).

**P6 — Variable × variable multiplication.** Resolved; restricted to Rule produce clauses with range containment check (§7).

**Deferred to v2:**

**P5 — Shared type library.** Record and TaggedUnion types are per-contract in v0.3. Cross-contract type reuse is deferred.

**P7 — Operation outcome typing.** Named outcome types on Operations are deferred. Current typed outcome routing is Flow-side classification only.

---

## Appendix A — Acknowledged Limitations

These are conscious design decisions, not oversights.

**AL1 — Fact ground property boundary** _(Fact 1.0, CE1)_  
Facts are ground within the evaluation model. Whether the source populating them is itself derived is outside the language's enforcement scope. Conforming executors must not populate Facts from internal evaluations.

**AL2 — Default assertion source** _(Fact 1.0, CE3)_  
When a default is used, the value is contract-asserted. Visible in provenance via `assertion_source: "contract"`.

**AL3 — DateTime timezone loss** _(BaseType, CE3)_  
DateTime values are normalized to UTC at assembly. Timezone-aware reasoning must be handled by adapters before Fact assertion.

**AL4 — Record and TaggedUnion acyclicity** _(BaseType, CE1)_  
Type declarations must form an acyclic graph. Self-referential types are prohibited.

**AL5 — TaggedUnion absence semantics** _(BaseType, CE2)_  
Mismatched tag access produces a typed absence value, which evaluates to false in predicate context.

**AL6 — Decimal promotion rules** _(BaseType, CF1)_  
Cross-precision Decimal arithmetic uses specified promotion rules. Conforming implementations must implement these exactly.

**AL7 — No nested lists** _(Fact extension, CE2)_  
List element types must be ScalarBaseType. Nested lists are not permitted.

**AL8 — List max is a conservative static bound** _(Fact extension)_  
Runtime lists may be smaller. Static complexity analysis uses the declared max.

**AL9 — Arithmetic determinism requires numeric model** _(PredicateExpression, CE2)_  
Arithmetic is only determinate given the mandated NumericModel.

**AL10 — Bounded quantification domain restriction** _(PredicateExpression extension, CE1)_  
Bounded quantification requires facts with declared max bounds. Unbounded collections may not be used as quantification domains.

**AL11 — Operation source-state validation is executor obligation** _(Operation, CE3)_  
The Operation construct does not encode source-state validation internally.

**AL12 — Operation atomicity is executor obligation** _(Operation)_  
The language defines what atomicity means but cannot enforce it internally.

**AL13 — Flow typed outcomes are Flow-side only** _(Flow, CE1)_  
Typed outcome routing is Flow-side classification of Operation results. The Operation canonical form is unchanged.

**AL14 — Sub-flow snapshot inheritance** _(Flow, CE2)_  
Sub-flows inherit the invoking Flow's snapshot and do not take independent snapshots.

**AL15 — Parallel execution via ParallelStep** _(Flow)_  
Parallel execution is supported via ParallelStep with fork/join semantics. Sequential Flows remain the default. See AL21–AL22 for ParallelStep-specific limitations.

**AL16 — Compensation nesting prohibition** _(Flow, CE4)_  
Compensation failure handlers are Terminal only. No nested compensation.

**AL17 — Branch decision provenance** _(Flow)_  
Branch decisions are recorded in Flow provenance but not in the Operation provenance chain.

**AL18 — Duration calendar independence** _(Duration)_  
Duration "day" means exactly 86,400 seconds. DST transitions, leap seconds, and calendar month/year spans are not representable as Duration values. Adapters must handle calendar-to-Duration conversion before Fact assertion.

**AL19 — Cross-currency via Rule layer** _(P4 resolution)_  
Cross-currency arithmetic is not a language primitive. It is expressed as a Fact (conversion rate) plus a Rule (multiplication). Multi-hop conversions require chained Rules. Bid/ask spread modeling requires two rate Facts.

**AL20 — Variable × variable restricted to Rule bodies** _(P6 resolution)_  
Variable × variable multiplication is available only in Rule bodies, not in PredicateExpression. The verdict payload type declaration serves as the result constraint. Decimal × Decimal variable multiplication is prohibited everywhere.

**AL21 — Parallel first_success not supported** _(ParallelStep)_  
All parallel branches run to completion before the join evaluates. Branch cancellation semantics are not defined.

**AL22 — Post-parallel verdict re-evaluation requires new Flow** _(ParallelStep)_  
Frozen verdict semantics apply within parallel blocks. If parallel branch results must feed into verdict evaluation, a new Flow must be initiated after the parallel block completes.

**AL23 — Elaborator Conformance Suite** _(ElaboratorSpec)_
The Tenor Elaborator Conformance Suite is at `conformance/`. 47/47 tests passing as of v0.3.

---

## Appendix B — Model Convergence Record

The following properties were independently arrived at by Claude (Anthropic), GPT (OpenAI), and DeepSeek without cross-contamination between sessions.

**Epistemological caveat:** LLM convergence is not the same as convergence between independent formal verification approaches. All models consulted share training data, common formal reasoning conventions, and exposure to prior art in formal methods and type theory. Independent convergence across LLM architectures does not constitute proof of soundness, and should not be treated as such for anything load-bearing. What it does constitute: suggestive evidence that the design choices are consistent with established formal reasoning conventions, and that no model found a trivial counterexample to the core commitments in independent sessions. The actual soundness argument rests on the design pressure record — the specific counterexamples raised, the rebuttals accepted or rejected, and the scope narrowings recorded. Those are the artifacts that carry evidential weight. The convergence record is context, not proof.

**Converged without divergence:**

- Provenance is part of the evaluation relation, not a runtime feature. `eval_rules` returns `Set<V × Provenance>`, not `Set<V>`.
- Strict stratification — no same-stratum rule references — is the correct termination guarantee. The re-expression theorem holds.
- Entity hierarchy carries no implicit authority semantics. All propagation must be explicitly declared.
- Verdict resolution is a declared, total, deterministic function. No implicit lattice join or meet.
- Spec completeness: an agent that can read this spec can fully understand a system described in it without reading implementation code.
- The behavioral contract calculus framing is correct. This is not a configuration format, policy DSL, or workflow engine — it is a distinct category.
- The spec must precede the implementation. Formal semantics precede syntax.

**Diverged and resolved under pressure:**

- **Entity authority inheritance (GPT vs. consensus).** GPT initially proposed that the entity partial order should carry implicit authority semantics with lattice joins for permission combination. Rejected under pressure — implicit join semantics introduce non-determinism in conflict resolution. Explicit declaration won. Authority propagation is declared, not inherited.
- **Entity hierarchy as permission backbone (DeepSeek vs. consensus).** DeepSeek proposed the partial order should be the primary backbone of permission propagation. Refined to: the partial order exists but carries no implicit semantics. Propagation is a separate explicit declaration over the DAG.
- **Same-stratum rule references (GPT initially dissenting).** GPT initially defended acyclic same-stratum references with cycle detection. The re-expression theorem argument resolved this — acyclic same-stratum graphs add zero expressive power and the structural termination guarantee is strictly stronger without them. GPT accepted after the formal argument was made.

---

## Appendix C — Worked Example: Escrow Release Contract

This appendix demonstrates a non-trivial Tenor contract covering two entities, monetary threshold rules, multi-persona authority, bounded quantification over line items, and a compensation flow. It is intended as a reference for:

- Contract authors learning the evaluation model
- Implementers validating executor behavior
- Agents verifying their understanding of the spec

The example is deliberately chosen to exercise all major constructs simultaneously.

### D.1 Domain Description

A buyer purchases goods from a seller. Payment is held in escrow. The escrow may be released to the seller upon delivery confirmation, or refunded to the buyer if delivery fails. A compliance officer must approve releases above a threshold. Line items are individually validated before release is permitted.

**Personas:** `buyer`, `seller`, `compliance_officer`, `escrow_agent`

**Entities:** `EscrowAccount`, `DeliveryRecord`

**Key decisions:**

- Can the escrow be released? (all line items valid, delivery confirmed, amount within threshold or compliance approved)
- Can the escrow be refunded? (delivery failed, buyer initiated)

---

### D.2 BaseTypes Used

```
Money(currency: "USD")
Bool
Text(max_length: 256)
Decimal(precision: 12, scale: 2)
Enum(values: ["pending", "confirmed", "failed"])

type LineItemRecord {
  id:          Text(max_length: 64)
  description: Text(max_length: 256)
  amount:      Money(currency: "USD")
  valid:        Bool
}

List(element_type: LineItemRecord, max: 100)
```

---

### D.3 Facts

```
Fact escrow_amount {
  type:   Money("USD")
  source: "escrow_service.current_balance"
}

Fact delivery_status {
  type:   Enum(["pending", "confirmed", "failed"])
  source: "delivery_service.status"
}

Fact line_items {
  type:   List(element_type: LineItemRecord, max: 100)
  source: "order_service.line_items"
}

Fact compliance_threshold {
  type:    Money("USD")
  source:  "compliance_service.release_threshold"
  default: Money { amount: Decimal(10000.00), currency: "USD" }
}

Fact buyer_requested_refund {
  type:    Bool
  source:  "buyer_portal.refund_requested"
  default: false
}
```

---

### D.4 Entities

```
Entity EscrowAccount {
  states:  [held, released, refunded, disputed]
  initial: held
  transitions: [
    (held, released),
    (held, refunded),
    (held, disputed),
    (disputed, released),
    (disputed, refunded)
  ]
}

Entity DeliveryRecord {
  states:  [pending, confirmed, failed]
  initial: pending
  transitions: [
    (pending, confirmed),
    (pending, failed)
  ]
}
```

---

### D.5 Rules

**Stratum 0 — Base fact verdicts:**

```
Rule all_line_items_valid {
  stratum: 0
  when: ∀ item ∈ line_items . item.valid = true
  produce: verdict line_items_validated { payload: Bool = true }
}

Rule delivery_confirmed {
  stratum: 0
  when: delivery_status = "confirmed"
  produce: verdict delivery_confirmed { payload: Bool = true }
}

Rule delivery_failed {
  stratum: 0
  when: delivery_status = "failed"
  produce: verdict delivery_failed { payload: Bool = true }
}

Rule amount_within_threshold {
  stratum: 0
  when: escrow_amount ≤ compliance_threshold
  produce: verdict within_threshold { payload: Bool = true }
}

Rule refund_requested {
  stratum: 0
  when: buyer_requested_refund = true
  produce: verdict refund_requested { payload: Bool = true }
}
```

**Stratum 1 — Composite verdicts:**

```
Rule can_release_without_compliance {
  stratum: 1
  when: verdict_present(line_items_validated)
      ∧ verdict_present(delivery_confirmed)
      ∧ verdict_present(within_threshold)
  produce: verdict release_approved { payload: Text = "auto" }
}

Rule requires_compliance_review {
  stratum: 1
  when: verdict_present(line_items_validated)
      ∧ verdict_present(delivery_confirmed)
      ∧ ¬verdict_present(within_threshold)
  produce: verdict compliance_review_required { payload: Bool = true }
}

Rule can_refund {
  stratum: 1
  when: verdict_present(delivery_failed)
      ∧ verdict_present(refund_requested)
  produce: verdict refund_approved { payload: Bool = true }
}
```

**Verdict resolution:** higher stratum verdicts take precedence within the same type. No same-type conflicts exist in this contract — each verdict type is produced by exactly one rule path.

---

### D.6 Operations

```
Operation release_escrow {
  allowed_personas: [escrow_agent]
  precondition:     verdict_present(release_approved)
  effects:          [(EscrowAccount, held, released)]
  error_contract:   [precondition_failed, persona_rejected]
}

Operation release_escrow_with_compliance {
  allowed_personas: [compliance_officer]
  precondition:     verdict_present(compliance_review_required)
  effects:          [(EscrowAccount, held, released)]
  error_contract:   [precondition_failed, persona_rejected]
}

Operation refund_escrow {
  allowed_personas: [escrow_agent]
  precondition:     verdict_present(refund_approved)
  effects:          [(EscrowAccount, held, refunded)]
  error_contract:   [precondition_failed, persona_rejected]
}

Operation flag_dispute {
  allowed_personas: [buyer, seller]
  precondition:     verdict_present(delivery_confirmed)
                  ∨ verdict_present(delivery_failed)
  effects:          [(EscrowAccount, held, disputed)]
  error_contract:   [precondition_failed, persona_rejected]
}

Operation confirm_delivery {
  allowed_personas: [seller]
  precondition:     ∀ item ∈ line_items . item.valid = true
  effects:          [(DeliveryRecord, pending, confirmed)]
  error_contract:   [precondition_failed, persona_rejected]
}

Operation record_delivery_failure {
  allowed_personas: [escrow_agent]
  precondition:     verdict_present(delivery_failed)
  effects:          [(DeliveryRecord, pending, failed)]
  error_contract:   [precondition_failed, persona_rejected]
}

// Compensation operation — used in failure recovery
Operation revert_delivery_confirmation {
  allowed_personas: [escrow_agent]
  precondition:     verdict_present(delivery_confirmed)
  effects:          [(DeliveryRecord, confirmed, pending)]
  error_contract:   [precondition_failed, persona_rejected]
}
```

---

### D.7 Flows

**Standard release flow:**

```
Flow standard_release {
  snapshot: at_initiation
  entry:    step_confirm

  steps: {
    step_confirm: OperationStep {
      op:      confirm_delivery
      persona: seller
      outcomes: {
        success: step_check_threshold
      }
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
      outcomes: {
        success: Terminal(success)
      }
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
      outcomes: {
        success: Terminal(success)
      }
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

**Refund flow:**

```
Flow refund_flow {
  snapshot: at_initiation
  entry:    step_refund

  steps: {
    step_refund: OperationStep {
      op:      refund_escrow
      persona: escrow_agent
      outcomes: {
        success: Terminal(success)
      }
      on_failure: Terminal(failure)
    }
  }
}
```

---

### D.8 Static Analysis Derivations

Given this contract, a conforming static analyzer must derive:

**S1 — Complete state space:**

```
EscrowAccount:   {held, released, refunded, disputed}
DeliveryRecord:  {pending, confirmed, failed}
```

**S2 — Reachable states from initial:**

```
EscrowAccount:   held → released, held → refunded, held → disputed,
                 disputed → released, disputed → refunded
DeliveryRecord:  pending → confirmed, pending → failed
```

**S3 — Admissible operations for escrow_agent in EscrowAccount.held:**

```
release_escrow              (if release_approved verdict satisfiable)
refund_escrow               (if refund_approved verdict satisfiable)
flag_dispute                (if delivery_confirmed or delivery_failed satisfiable)
record_delivery_failure     (if delivery_failed satisfiable)
```

**S4 — Authority topology sample:**

```
Q: Can buyer cause EscrowAccount to reach state released?
A: No. buyer's only admissible operation is flag_dispute, which transitions to disputed.
   From disputed, only compliance_officer or escrow_agent can release.
   buyer has no path to released.
```

**S6 — Flow paths for standard_release:**

```
Path 1: confirm_delivery (success) → within_threshold (true) → release_escrow (success) → Terminal(success)
Path 2: confirm_delivery (success) → within_threshold (false) → handoff → release_escrow_with_compliance (success) → Terminal(success)
Path 3: confirm_delivery (success) → within_threshold (true) → release_escrow (failure) → revert_delivery_confirmation → Terminal(failure)
Path 4: confirm_delivery (success) → within_threshold (false) → handoff → release_escrow_with_compliance (failure) → revert_delivery_confirmation → Terminal(failure)
Path 5: confirm_delivery (failure) → Terminal(failure)
```

All paths are finite. All terminal outcomes are enumerable. No path is missing a failure handler.

---

### D.9 Evaluation Trace — Sample Execution

**Inputs:**

```
escrow_amount:          Money { amount: 8500.00, currency: "USD" }
delivery_status:        "confirmed"
compliance_threshold:   Money { amount: 10000.00, currency: "USD" }
buyer_requested_refund: false
line_items: [
  { id: "L1", description: "Widget A", amount: Money(5000.00, "USD"), valid: true },
  { id: "L2", description: "Widget B", amount: Money(3500.00, "USD"), valid: true }
]
```

**FactSet assembly:** All facts present, types valid. `delivery_status` normalized — no DateTime in this contract. `line_items` length 2 ≤ max 100. Assembly succeeds.

**Stratum 0 evaluation:**

```
all_line_items_valid:     ∀ item ∈ line_items . item.valid = true  → true
                          → verdict line_items_validated produced

delivery_confirmed:       delivery_status = "confirmed"            → true
                          → verdict delivery_confirmed produced

delivery_failed:          delivery_status = "failed"               → false
                          → no verdict produced

amount_within_threshold:  8500.00 ≤ 10000.00                      → true
                          → verdict within_threshold produced

refund_requested:         buyer_requested_refund = true            → false
                          → no verdict produced
```

**Stratum 1 evaluation:**

```
can_release_without_compliance:
  verdict_present(line_items_validated) = true
  ∧ verdict_present(delivery_confirmed) = true
  ∧ verdict_present(within_threshold)   = true
  → true → verdict release_approved produced

requires_compliance_review:
  verdict_present(within_threshold) = true → ¬true = false
  → false → no verdict produced

can_refund:
  verdict_present(delivery_failed) = false
  → false → no verdict produced
```

**ResolvedVerdictSet:**

```
{ line_items_validated, delivery_confirmed, within_threshold, release_approved }
```

**Flow execution — standard_release (snapshot taken):**

```
step_confirm:
  persona seller ∈ confirm_delivery.allowed_personas → pass
  precondition: ∀ item ∈ line_items . item.valid = true → true (from FactSet)
  effects: (DeliveryRecord, pending, confirmed) — executor validates pending matches current state → apply
  outcome: success → step_check_threshold

step_check_threshold:
  NOTE: verdict_present(within_threshold) evaluated against FROZEN snapshot verdict set
        Entity state is now DeliveryRecord.confirmed — but verdicts are NOT recomputed
        within_threshold is present in frozen verdict set → condition true
  → step_auto_release

step_auto_release:
  persona escrow_agent ∈ release_escrow.allowed_personas → pass
  precondition: verdict_present(release_approved) → true (frozen verdict set)
  effects: (EscrowAccount, held, released) — executor validates held matches current state → apply
  outcome: success → Terminal(success)
```

**FlowOutcome:** success

**Provenance chain:**

```
FlowOutcome(success)
  BranchRecord(condition: verdict_present(within_threshold), result: true, persona: escrow_agent)
  OperationRecord(release_escrow)
    verdicts_used: {release_approved, within_threshold, line_items_validated, delivery_confirmed}
    facts_used:    {escrow_amount, delivery_status, line_items, compliance_threshold}
    state_before:  EscrowAccount.held
    state_after:   EscrowAccount.released
  OperationRecord(confirm_delivery)
    facts_used:    {line_items}
    state_before:  DeliveryRecord.pending
    state_after:   DeliveryRecord.confirmed
```

Every chain terminates at Facts. Provenance is complete.

---

### D.10 Frozen Verdict Semantics — Demonstrated

In the trace above, `step_check_threshold` evaluates `verdict_present(within_threshold)` against the **frozen snapshot** verdict set — not a recomputed one. After `confirm_delivery` executed and transitioned `DeliveryRecord` to `confirmed`, the verdict set was not recomputed. If it had been, `delivery_confirmed` would still be present (delivery_status Fact unchanged), but the point is structural: **the verdict set used at step_check_threshold is identical to the one computed at Flow initiation.** A Rule that depended on `DeliveryRecord.confirmed` entity state (if such a Rule existed) would not reflect the mid-Flow transition.

This is the frozen verdict semantic commitment in concrete form.
