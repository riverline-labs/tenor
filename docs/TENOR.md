# Tenor

## Tenor Language Specification v0.9

**Status:** v0.9 — Spec frozen. Core constructs canonicalized via [CFFP](https://github.com/riverline-labs/iap) (Constraint-First Formalization Protocol).
**Method:** Human-AI collaborative design. See [`CONTRIBUTORS.md`](../CONTRIBUTORS.md).
**Creator:** Brandon W. Bush

> **Stability: Frozen (v0.9)**
> This specification is frozen. No breaking changes to existing construct
> semantics will occur without a new CFFP run. Additive changes (new analysis
> results, new interchange metadata) are permitted.
>
> v1.0 requires the System construct — a composition layer for multi-contract
> systems (shared persona identity, cross-contract flow triggers, cross-contract
> entity relationships). System requires a dedicated CFFP run.
>
> **Freeze date:** 2026-02-21

---

## Table of Contents

1. Preamble
2. Design Constraints
3. Core Constructs Overview
4. BaseType
5. Fact
6. Entity
7. Rule
8. Persona
9. Operation
10. PredicateExpression
11. Flow
12. NumericModel
13. ElaboratorSpec
14. Complete Evaluation Model
15. Static Analysis Obligations
16. Executor Obligations
17. Versioning & Migration
    - 17.1 The Migration Problem
    - 17.2 Breaking Change Taxonomy
    - 17.3 Executor Migration Obligations
    - 17.4 In-Flight Flow Migration Policy
    - 17.5 Migration Contract Representation
    - 17.6 Flow Migration Compatibility
18. Contract Discovery & Agent Orientation
    - 18.1 The Contract Manifest
    - 18.2 Etag Semantics
    - 18.3 Discovery Endpoint
    - 18.4 Cold-Start Protocol
    - 18.5 Change Detection
    - 18.6 Dry-Run Evaluation
    - 18.7 Executor Obligation Summary (E10-E14)
19. Appendix A — Acknowledged Limitations
20. Appendix C — Worked Example: Escrow Release Contract
21. Appendix D — Glossary

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

The language defines twelve constructs across three layers.

**Semantic layer** (dependency order — each depends only on those above):

```
BaseType            — closed value type set (includes Duration)
Fact                — ground typed assertions; the evaluation root
Entity              — finite state machines in a static DAG
Rule                — stratified verdict-producing evaluation (includes variable×variable)
Persona             — declared identity tokens for authority gating
Operation           — persona-gated, precondition-guarded state transitions with declared outcomes
PredicateExpression — quantifier-free FOL with arithmetic and bounded quantification
Flow                — finite DAG orchestration with sequential and parallel steps
NumericModel        — fixed-point decimal with total promotion rules (cross-cutting)
```

Named type aliases (TypeDecl) are a DSL-layer convenience only. The elaborator resolves all named type references during Pass 3 and inlines the full BaseType structure at every point of use. TypeDecl does not appear in TenorInterchange output. TypeDecl definitions may be shared across contracts via shared type library files (§4.6). Shared type libraries are Tenor files containing only TypeDecl constructs, imported via the existing import mechanism.

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

### 4.2.1 Text Comparison Is Exact Only

Text values support only exact equality (`=`) and inequality (`≠`). Pattern matching operations — regex, substring, prefix/suffix, glob, wildcard — are not supported in any context. Pattern-based classification must be pre-computed into a Bool or Enum Fact by the executor.

Incorrect:

```
rule requires_legal_review {
  stratum: 0
  when:    item.sku matches "LEGAL-.*"  // ERROR: pattern matching not supported
  produce: legal_review_required(true)
}
```

Correct:

```
fact requires_legal_review {
  type:   Bool
  source: compliance_system.sku_classification(requisition_id)
}
```

Or, if classification must be per-item, include it as a field in the Record type:

```
type LineItem {
  sku:           Text(max_length: 32)
  legal_review:  Bool  // pre-classified by external system
}
```

### 4.3 Type Checking

```
type_check : Value × BaseType → Bool | Error

type_check(v, Bool)                  = v ∈ {true, false}
type_check(v, Int(min, max))         = v ∈ ℤ ∧ min ≤ v ≤ max
type_check(v, Decimal(p, s))         = v representable in fixed-point(p, s)
type_check(v, Text(n))               = |v| ≤ n ∧ v is valid UTF-8
type_check(v, Enum(vals))            = v ∈ vals
type_check(v, Date)                  = v is valid RFC 3339 full-date (YYYY-MM-DD)
type_check(v, DateTime)              = v is valid RFC 3339 date-time, UTC-normalized
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
fact line_items {
  type:   List(element_type: LineItemRecord, max: 100)
  source: "order_service.line_items"
}
```

**Interchange:** TypeDecl is a DSL-layer construct only. The elaborator resolves named type references during Pass 3 (type environment construction) and inlines the full BaseType structure at every point of use during Pass 4 (AST materialization). TypeDecl entries do not appear in the interchange bundle. Interchange is fully self-contained — no TypeRef lookups are required to interpret any interchange document.

**Constraints:**

- TypeDecl ids are unique within a contract (including imported TypeDecl ids — see §4.6). Two TypeDecls may not share an id, whether both are local, both are imported, or one is local and one is imported.
- TypeDecl ids occupy a distinct namespace from other construct kinds. A TypeDecl named `Foo` does not conflict with an Entity named `Foo`.
- The TypeDecl reference graph must be acyclic. If TypeDecl A contains a field of type TypeDecl B, and TypeDecl B contains a field of type TypeDecl A, this is a declaration cycle and is a Pass 3 error. Cycle detection uses DFS over the TypeDecl reference graph. This applies to the unified TypeDecl graph including both local and imported definitions (§4.6).
- A TypeDecl may only alias Record or TaggedUnion types.

### 4.6 Shared Type Libraries

A shared type library is a Tenor file containing zero or more TypeDecl constructs and no other construct kinds (no Fact, Entity, Rule, Persona, Operation, Flow). Type libraries enable cross-contract type reuse: a Record or TaggedUnion type can be declared once in a library and imported by multiple contracts.

**Import mechanism:** A contract file imports a type library using the existing `import` syntax:

```
import "shared_types.tenor"
```

Imported TypeDecl definitions are merged into the unified parse tree during Pass 1 and participate in type environment construction (Pass 3) identically to local TypeDecl definitions. Imported types may be referenced anywhere a local TypeDecl can be referenced: Fact type fields, List element_type positions, and other TypeDecl definitions.

**DSL example — type library file** (`types/common.tenor`):

```
type Address {
  street: Text(max_length: 256)
  city:   Text(max_length: 128)
  state:  Text(max_length: 64)
  zip:    Text(max_length: 10)
}

type Currency {
  code: Text(max_length: 3)
  name: Text(max_length: 64)
}
```

**DSL example — contract importing type library:**

```
import "types/common.tenor"

fact shipping_address {
  type:   Address
  source: "order_service.shipping"
}

fact line_items {
  type: List(element_type: LineItem, max: 100)
  source: "order_service.line_items"
}

type LineItem {
  id:          Text(max_length: 64)
  description: Text(max_length: 256)
  amount:      Money(currency: "USD")
  address:     Address
}
```

In this example, `Address` is imported from the type library. The local TypeDecl `LineItem` references `Address` as a field type. After Pass 3/4 elaboration, the `Address` reference is inlined to its full Record structure, identical to declaring `Address` locally.

**Type identity:** Type identity is structural. After Pass 3/4 inlining, two types are identical if and only if their fully expanded BaseType structures are recursively equal, regardless of origin (local or imported). A Record imported from a library and a Record declared locally are the same type if they have identical field names and field types. This preserves the existing Tenor type identity semantics without change.

**Type library constraints:**

- A type library file may not contain import declarations. Type libraries are self-contained leaf files in the import graph. If a file identified as a type library (containing only TypeDecl constructs) also contains import declarations, this is an elaboration error. This restriction prevents transitive type propagation: if contract A imports library L, A gets exactly L's TypeDecl definitions. L cannot import other libraries, so there are no hidden transitive dependencies.
- A type library file may contain only TypeDecl constructs. If a file contains any non-TypeDecl construct (Fact, Entity, Rule, Persona, Operation, Flow), it is treated as a regular contract file, not a type library.
- TypeDecl names occupy a flat namespace. Imported TypeDecl ids must not conflict with local TypeDecl ids or with TypeDecl ids from other imported libraries. Duplicate TypeDecl ids across file boundaries are elaboration errors (Pass 1).
- The unified TypeDecl reference graph (local + imported) must be acyclic. Cross-file TypeDecl cycles are structurally impossible under the type library constraint (libraries cannot see importing contract declarations), but the Pass 3 DFS cycle detection operates on the full unified graph as a safety guarantee.

**Interchange representation:** Shared type libraries have no interchange representation. Imported TypeDecl definitions are consumed during Pass 3 (type environment construction) and inlined during Pass 4 (AST materialization). The interchange output for a contract that imports shared types is identical to the interchange for a contract that declares those same types locally. No import reference, file path, or TypeDecl entry appears in the interchange bundle. Interchange remains fully self-contained.

**Elaboration integration:** See §13.2 for the pass-by-pass handling of shared type library imports.

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

### 5.5 No Aggregate Computation

A common incorrect assumption is that aggregate functions (`sum`, `count`, `average`, `min`, `max`) can be computed over List-typed Facts within Rule bodies or PredicateExpressions. This is not permitted. Aggregates are derived values, not verdicts — they must arrive as Facts from external systems.

Incorrect:

```
rule requisition_total {
  stratum: 0
  when:    true
  produce: requisition_total(sum(item.amount for item in line_items))  // ERROR: no aggregate functions
}
```

Correct:

```
fact requisition_total {
  type:   MoneyAmount
  source: order_system.calculated_total(requisition_id)
}
```

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

Verdicts form a finite tagged set. Each verdict type has a name and a payload schema.

```
VerdictType = (
  name:             string,
  payload_schema:   BaseType
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

The resolution function `resolve : Set<VerdictInstance> → ResolvedVerdictSet` is the identity on sets — it returns the accumulated verdict set produced by `eval_strata`. Resolution is total and deterministic because same-VerdictType conflicts are prohibited by static analysis (S8, §15). Each VerdictType in a conforming contract is produced by exactly one rule. The ResolvedVerdictSet therefore contains at most one VerdictInstance per VerdictType, and `verdict_present(X)` unambiguously identifies at most one instance with a well-defined payload.

User-defined verdict precedence and resolution strategies are deferred to a future version (AL50).

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

Rate staleness is an executor concern — E1 (§16.2) requires facts to come from genuinely external sources, but does not mandate freshness guarantees. Executors should document their staleness policies for time-sensitive facts such as exchange rates. Multi-hop conversions require chained Rules across strata. Each conversion step is separately provenance-tracked.

---

## 8. Persona

### 8.1 Definition

A Persona is a declared identity construct. It establishes a named participant identity that may be referenced in Operation `allowed_personas` sets, Flow step `persona` fields, `HandoffStep` `from_persona`/`to_persona` fields, `CompensationStep` `persona` fields, and `Escalate` `to_persona` fields. Personas are the authority namespace of the contract — they define *who* can act.

```
Persona = (
  id: PersonaId
)
```

PersonaId is a non-empty UTF-8 string, unique within the contract. The set of all declared Personas P = {p1, ..., pn} is finite, fixed at contract definition time, and statically enumerable.

A Persona carries no metadata, no description, no delegation, and no semantic content beyond its identity. It is an opaque token whose sole purpose is to make the authority namespace explicit and statically checkable. Documentation-level information about personas (display names, role descriptions) is provided via DSL comments or external documentation, not via construct fields.

**DSL syntax:**

```
persona buyer
persona seller
persona compliance_officer
persona escrow_agent
```

### 8.2 Semantics

Persona declarations are consumed during elaboration and carried into the interchange format. They have no runtime evaluation rule. The existing Operation evaluation rule (Section 9.2) is unchanged: `execute(op, persona, verdict_set, entity_state)` checks `persona in op.allowed_personas` as a simple set membership test.

Persona declaration ensures that the strings in `allowed_personas` sets and step `persona` fields are drawn from a declared, finite, statically known set. No new evaluation step is introduced. No existing evaluation step is modified.

### 8.3 Constraints

- Persona identifiers are unique within a contract. Two Persona declarations may not share the same id.
- Persona ids occupy a distinct namespace from other construct kinds. A Persona named `Foo` does not conflict with an Entity named `Foo`.
- Every persona reference in an Operation `allowed_personas` set must resolve to a declared Persona. Unresolved persona references are elaboration errors (Pass 5).
- Every persona reference in a Flow step `persona` field (OperationStep, BranchStep, SubFlowStep), HandoffStep `from_persona`/`to_persona`, CompensationStep `persona`, and Escalate `to_persona` must resolve to a declared Persona. Unresolved persona references are elaboration errors (Pass 5).
- The complete set of Persona identifiers is statically enumerable from the contract (Pass 2).
- Unreferenced Persona declarations (declared but never used) are not elaboration errors. Static analysis tooling may optionally warn about them.

### 8.4 Provenance

```
PersonaProvenance = (
  id:   PersonaId,
  file: string,
  line: nat
)
```

Persona provenance records the declaration site. Personas do not carry runtime provenance — they are declaration-time constructs whose identity is fixed at contract definition.

### 8.5 Interchange Representation

Persona constructs appear as items in the interchange `constructs` array with `"kind": "Persona"`.

```json
{
  "id": "escrow_agent",
  "kind": "Persona",
  "provenance": {
    "file": "escrow.tenor",
    "line": 4
  },
  "tenor": "1.0"
}
```

All JSON keys are sorted lexicographically. The `id` field is the PersonaId string. No additional fields are present — Persona carries no metadata in the interchange format.

Existing persona string references in Operation `allowed_personas` arrays and Flow step `persona` fields remain as string values in the interchange format. They are not replaced with structured references. The validation that these strings resolve to declared Personas is an elaboration-time check (Pass 5), not an interchange-level structural constraint. This parallels how `fact_ref` strings in PredicateExpressions are validated against declared Facts without being replaced by structured references.

---

## 9. Operation

### 9.1 Definition

An Operation is a declared, persona-gated, precondition-guarded unit of work that produces entity state transitions as its sole side effect and declares a finite set of named outcomes. Operations are the only construct in the evaluation model that produces side effects.

```
Operation = (
  id:               OperationId,
  allowed_personas: Set<PersonaId>,
  precondition:     PredicateExpression,              // over ResolvedVerdictSet
  effects:          Set<(EntityId × StateId × StateId)>,  // entity-scoped transitions
  error_contract:   Set<ErrorType>,
  outcomes:         Set<OutcomeLabel>                  // named success-path results
)
```

OutcomeLabel is a non-empty UTF-8 string, unique within the Operation's outcome set. The outcome set must be non-empty (`|outcomes| >= 1`). The outcome set and error_contract set must be disjoint: `outcomes INTERSECT error_contract = EMPTY`. Outcome labels are Operation-local: the label `"approved"` on one Operation is not related to the label `"approved"` on another.

When an Operation declares multiple outcomes and multiple effects, each effect must be associated with exactly one outcome. This association is part of the Operation declaration, not an executor determination.

**DSL syntax:**

```
operation approve_order {
  personas: [reviewer, admin]
  require:  verdict_present(account_active)
  effects:  [Order: submitted -> approved]
  outcomes: [approved]
}
```

Multi-outcome Operation with effect-to-outcome associations:

```
operation decide_claim {
  personas: [adjudicator]
  require:  verdict_present(claim_eligible)
  outcomes: [approved, rejected]
  effects:  [
    Claim: review -> approved  -> approved,
    Claim: review -> rejected  -> rejected
  ]
}
```

In the multi-outcome form, each effect tuple is extended with `-> OutcomeLabel` to declare which outcome it belongs to. All effects for a given outcome are applied atomically when that outcome is produced.

### 9.2 Evaluation

```
execute : Operation × PersonaId × ResolvedVerdictSet × EntityState → (EntityState', OutcomeLabel) | Error

execute(op, persona, verdict_set, entity_state) =
  if persona ∉ op.allowed_personas:
    return Error(op.error_contract, "persona_rejected")
  if ¬eval_pred(op.precondition, FactSet, verdict_set):
    return Error(op.error_contract, "precondition_failed")
  // Executor obligation: validate entity_state matches transition source for each effect
  // Determine which outcome to produce based on entity state and effect-to-outcome mapping
  outcome = determine_outcome(op, entity_state)
  // apply_atomic applies effects associated with the produced outcome atomically
  apply_atomic(op.effects[outcome], entity_states) → entity_states'
  emit_provenance(op, persona, verdict_set, entity_state, entity_state', outcome)
  return (entity_state', outcome)
```

For single-outcome Operations, `determine_outcome` trivially returns the sole member of `op.outcomes`. For multi-outcome Operations, the outcome is determined by the effect-to-outcome mapping declared in the contract — the executor matches the current entity state against the declared effect source states and selects the outcome whose associated effects are applicable. This is not an executor discretion: the contract declares which effects belong to which outcome, and the entity state determines which effects are applicable.

### 9.3 Execution Sequence

The execution sequence is fixed and invariant: (1) persona check, (2) precondition evaluation, (3) outcome determination, (4) atomic effect application for the determined outcome, (5) provenance emission (including outcome label). No step may be reordered. No step may be skipped if the preceding step succeeds.

### 9.4 Constraints

- An Operation with an empty `allowed_personas` set is a contract error detectable at load time.
- Every PersonaId in `allowed_personas` must resolve to a declared Persona construct (Section 8). Unresolved persona references are elaboration errors (Pass 5).
- Each declared effect `(entity_id, from_state, to_state)` must reference an entity declared in the contract, and the transition `(from_state, to_state)` must exist in that entity's declared transition relation. Checked at contract load time. An Operation may declare effects across multiple entities — each is validated independently.
- An Operation must declare at least one outcome (`|outcomes| >= 1`). An Operation with an empty outcome set is a contract error detectable at load time.
- Outcome labels must be unique within each Operation's outcome set. Duplicate outcome labels are elaboration errors (Pass 5).
- The outcome set and error_contract set must be disjoint (`outcomes INTERSECT error_contract = EMPTY`). A label appearing in both sets is an elaboration error (Pass 5).
- For multi-outcome Operations, every declared effect must be associated with exactly one outcome. Effects with no outcome association, or effects associated with an undeclared outcome, are elaboration errors (Pass 5).
- Operations do not produce verdict instances. Verdict production belongs exclusively to Rules.
- Atomicity is an executor obligation. Either all declared state transitions for the produced outcome occur, or none do.
- The executor must validate that the current entity state matches the transition source for each declared effect. This is an executor obligation not encoded in the Operation formalism.
- Outcome exhaustiveness is a contract authoring obligation. The elaborator validates that Flow routing handles all declared outcomes (see §11.5), but cannot verify that the declared outcome set is exhaustive of all possible executor success-path behaviors. This parallels source-state validation (E2, §16.2).

### 9.4.1 No Wildcard Transitions

Every effect must name an explicit source state and an explicit target state. Wildcard notation (e.g., `* -> approved`) is not permitted. Wildcards would prevent E2 source-state validation (§16). Operations invocable from multiple source states must declare multiple explicit effects.

Incorrect:

```
operation reject_requisition {
  personas: [approver]
  require:  true
  effects:  [Requisition: * -> rejected]  // ERROR: wildcard source not permitted
}
```

Correct (multiple effects in one operation):

```
operation reject_requisition {
  personas: [approver]
  require:  true
  effects:  [
    Requisition: submitted -> rejected,
    Requisition: pm_approved -> rejected,
    Requisition: dept_review -> rejected,
    Requisition: finance_review -> rejected,
    Requisition: legal_review -> rejected
  ]
}
```

### 9.5 Provenance

```
OperationProvenance = (
  op:            OperationId,
  persona:       PersonaId,
  outcome:       OutcomeLabel,
  facts_used:    Set<FactId>,
  verdicts_used: ResolvedVerdictSet,
  state_before:  EntityState,
  state_after:   EntityState'
)
```

The `outcome` field records which declared outcome was produced by this execution. This enables provenance chains to track not just state transitions but which success-path result led to subsequent Flow routing decisions.

### 9.6 Interchange Representation

Operation constructs in the interchange format include the declared `outcomes` field as a sorted array of outcome label strings.

Single-outcome Operation:

```json
{
  "allowed_personas": ["reviewer", "admin"],
  "effects": [
    { "entity_id": "Order", "from": "submitted", "to": "approved" }
  ],
  "error_contract": ["precondition_failed", "persona_rejected"],
  "id": "approve_order",
  "kind": "Operation",
  "outcomes": ["approved"],
  "precondition": { "verdict_present": "account_active" },
  "provenance": { "file": "order.tenor", "line": 33 },
  "tenor": "1.0"
}
```

Multi-outcome Operation with effect-to-outcome associations:

```json
{
  "allowed_personas": ["adjudicator"],
  "effects": [
    { "entity_id": "Claim", "from": "review", "outcome": "approved", "to": "approved" },
    { "entity_id": "Claim", "from": "review", "outcome": "rejected", "to": "rejected" }
  ],
  "error_contract": ["precondition_failed", "persona_rejected"],
  "id": "decide_claim",
  "kind": "Operation",
  "outcomes": ["approved", "rejected"],
  "precondition": { "verdict_present": "claim_eligible" },
  "provenance": { "file": "claims.tenor", "line": 15 },
  "tenor": "1.0"
}
```

For multi-outcome Operations, each effect object includes an `"outcome"` field associating it with a declared outcome label. For single-outcome Operations, the `"outcome"` field on effects is optional (it can be inferred from the sole member of the outcome set). All JSON keys are sorted lexicographically within each object. The `outcomes` array values preserve declaration order (per Pass 6 serialization rules: array values are never sorted).

---

## 10. PredicateExpression

### 10.1 Definition

A PredicateExpression is a quantifier-free first-order logic formula over ground terms drawn from the resolved FactSet, the resolved VerdictSet, and literal constants declared in the contract. All terms are ground at evaluation time. No free variables. No recursion. No side effects.

### 10.2 Grammar

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

### 10.3 Evaluation

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

### 10.4 Constraints

- No implicit type coercions. A comparison between incompatible types is a load-time contract error.
- Quantification domains must be List-typed Facts or List-typed fields of Record Facts with declared max bounds.
- Quantification over verdict sets, entity state sets, or unbounded collections is not permitted.
- Variable × variable multiplication is not permitted.
- All arithmetic follows the NumericModel.

### 10.5 Complexity

Scalar predicate evaluation is O(|expression tree|). Quantified predicate evaluation is O(max × |body|) per quantifier level. Nested quantifiers multiply bounds. All bounds are statically derivable from declared maxes and expression tree size.

### 10.6 Entity State Is Not a Predicate Term

A common incorrect assumption is that the current state of an Entity can be tested in a precondition using expressions like `Requisition.state = "draft"`. This is not permitted. Entity state is not in the term grammar (§10.2) — state constraints are enforced through effect declarations and E2 source-state validation.

Incorrect:

```
operation submit_requisition {
  personas: [requestor]
  require:  requisition_total present and
            Requisition.state = "draft"  // ERROR: entity state not a predicate term
  effects:  [Requisition: draft -> submitted]
}
```

Correct:

```
operation submit_requisition {
  personas: [requestor]
  require:  requisition_total present and
            line_items_validated present
  effects:  [Requisition: draft -> submitted]  // state constraint is here
}
```

---

## 11. Flow

### 11.1 Definition

A Flow is a finite, directed acyclic graph of steps that orchestrates the execution of declared Operations under explicit persona control. Flows do not compute — they sequence. All business logic remains in Rules and Operations.

```
Flow = (
  id:       FlowId,
  entry:    StepId,
  steps:    { StepId → Step },
  snapshot: SnapshotPolicy    // always "at_initiation" in v1
)
```

### 11.2 Step Types

```
Step =
  OperationStep(
    op:         OperationId,
    persona:    PersonaId,
    outcomes:   { OutcomeLabel → StepId | Terminal },  // keys must match op.outcomes
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

### 11.3 Failure Handling

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

### 11.4 Evaluation

**Frozen verdict semantics:** Within a Flow, the ResolvedVerdictSet is computed once at Flow initiation and is not recomputed after intermediate Operation execution. Operations within a Flow do not see entity state changes produced by preceding steps in the same Flow. This is a fundamental semantic commitment: Flows are pure decision graphs over a stable logical universe. The consequence is that a Rule whose inputs include entity state will not reflect mid-Flow transitions — such patterns must be expressed across Flow boundaries, not within them.

```
execute_flow : Flow × PersonaId × Snapshot → FlowOutcome

execute_flow(flow, initiating_persona, snapshot) =
  current = flow.entry
  loop:
    step = flow.steps[current]
    match step:
      OperationStep →
        result = execute(step.op, step.persona, snapshot.verdict_set)
        match result:
          (state', outcome_label) →
            emit_provenance(step, result)
            current = step.outcomes[outcome_label]
          Error →
            handle_failure(step.on_failure)
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

The key change from v0.3: when an OperationStep executes, the result is a `(state', outcome_label)` pair on success. The Flow routes on `outcome_label` by looking it up directly in the step's `outcomes` map — there is no `classify()` function. The outcome label comes from the Operation's declared outcome set, ensuring type-safe routing. Error results continue to be handled by `on_failure`.

### 11.5 Constraints

- Step graph must be acyclic. Verified at load time via topological sort.
- All StepIds referenced in a Flow must exist in the steps map.
- All OperationIds referenced must exist in the contract.
- All PersonaIds in step `persona` fields (OperationStep, BranchStep, SubFlowStep), HandoffStep `from_persona`/`to_persona`, CompensationStep `persona`, and Escalate `to_persona` must resolve to declared Persona constructs (Section 8). Unresolved persona references are elaboration errors (Pass 5).
- Flow reference graph (SubFlowStep references) must be acyclic. Verified via DFS across all contract files.
- Sub-flows inherit the invoking Flow's snapshot. Sub-flows do not take independent snapshots.
- OperationStep outcome routing is grounded in Operation-declared outcomes. Each key in an OperationStep's `outcomes` map must be a member of the referenced Operation's declared outcome set. This is validated at elaboration time (Pass 5).
- OperationStep outcome handling must be exhaustive: the keys of the `outcomes` map must exactly equal the declared outcome set of the referenced Operation. Missing outcomes are elaboration errors (Pass 5). No implicit fall-through to on_failure for unhandled success-path outcomes.
- Compensation failure handlers are Terminal only. No nested compensation.
- Parallel branches execute under the parent Flow's frozen snapshot. No branch sees entity state changes produced by another branch during execution.
- No two parallel branches may declare effects on overlapping entity sets. Verified at contract load time by transitively resolving all Operation effects across all branches.
- All branches run to completion before the join evaluates. Branch execution order is implementation-defined. The join outcome is a function of the set of branch terminal outcomes, not their order.
- `first_success` merge policy is not supported.

### 11.6 Provenance

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

## 12. NumericModel

### 12.1 Definition

All numeric computation uses fixed-point decimal arithmetic. No floating-point arithmetic is permitted in conforming implementations. Integer arithmetic is exact within declared range. Decimal arithmetic uses declared precision and scale.

### 12.2 Promotion Rules

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

### 12.3 Overflow

Arithmetic that produces a result outside the declared range aborts evaluation with a typed overflow error. Silent wraparound and saturation are not permitted.

### 12.4 Rounding

Where a Decimal result has more fractional digits than the declared scale, rounding is applied. The rounding mode is **round half to even** (IEEE 754 roundTiesToEven). This mode is mandatory for all conforming implementations.

### 12.5 Literal Types

Integer literals are typed as Int(n, n). Decimal literals are typed as Decimal(total_digits, fractional_digits) derived from the literal's written form.

### 12.6 Implementation Bounds

All conforming implementations must satisfy these bounds, regardless of language or numeric library used:

| Property | Requirement |
|----------|-------------|
| Maximum significant digits | 28 |
| Scale range | 0–28 (fractional digits) |
| Maximum value | (2^96 − 1) × 10^−28 |
| Minimum value | −(2^96 − 1) × 10^−28 |
| Infinity | Not representable. Any operation producing infinity aborts with overflow error. |
| NaN | Not representable. No operation produces NaN. |
| Signed zero | Not representable. Zero has no sign. |
| Rounding mode | Round half to even (IEEE 754 roundTiesToEven) — see §12.4 |
| Overflow behavior | Abort with typed overflow error — see §12.3 |

These bounds are derived from the reference implementation's numeric library but are stated here as language-level requirements. An implementation that satisfies these bounds is conforming regardless of which numeric library it uses internally. An implementation that delegates to native floating-point arithmetic is not conforming even if its results happen to match within these bounds for common inputs — the requirement is structural, not empirical.

---

## 13. ElaboratorSpec

### 13.1 Overview

The elaborator is the trust boundary between human authoring and formal guarantees. It transforms Tenor source into a valid TenorInterchange bundle through six deterministic, ordered passes. A bug in the elaborator is more dangerous than a bug in the executor — it silently produces malformed interchange that the executor then operates on correctly, producing wrong results from correct execution.

A conforming elaborator must be deterministic: identical DSL input produces byte-for-byte identical interchange output on every invocation. No environmental dependency (timestamp, process id, random seed) may affect the output.

### 13.2 Elaboration Passes

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
- Identify shared type library files: any imported file containing only TypeDecl constructs (no Fact, Entity, Rule, Persona, Operation, Flow) is a type library. Type library files may not contain import declarations — if present, this is an elaboration error (§4.6).
- Merge parse trees into a unified bundle. Duplicate construct ids across files are elaboration errors. This includes TypeDecl ids: an imported TypeDecl id that conflicts with a local TypeDecl id or another imported TypeDecl id is an elaboration error.

**Pass 2 — Construct indexing**
Input: Unified parse tree. Output: Construct index keyed by `(construct_kind, id)`.

- Build index of all declared construct ids.
- Same-kind duplicate ids are elaboration errors. Same-id, different-kind pairs do not conflict.

**Pass 3 — Type environment construction**
Input: Construct index (TypeDecl, Fact, and VerdictType declarations). Output: Type environment.

- Resolve all declared TypeDecl definitions (local and imported — §4.6). Detect cycles in the unified TypeDecl reference graph via DFS. Build named type lookup table. Imported TypeDecl definitions from shared type libraries are treated identically to local TypeDecl definitions.
- Resolve all BaseTypes in Fact and VerdictType declarations, expanding named type references using the TypeDecl lookup table. Detect any remaining Record/TaggedUnion declaration cycles.
- Build complete type environment before any expression type-checking begins.
- TypeDecl entries (both local and imported) are consumed during type environment construction. They do not propagate to interchange output.

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
- Operation: allowed_personas non-empty; all allowed_personas entries resolve to declared Persona constructs; effect entity_ids resolve; effect transitions exist in entity; effects ⊆ entity.transitions; outcomes non-empty; outcome labels unique within Operation; outcomes ∩ error_contract = ∅; for multi-outcome Operations, every effect has an outcome association referencing a declared outcome.
- Rule: stratum ≥ 0; all refs resolve; verdict_refs reference strata < this rule's stratum; produce clauses reference declared VerdictType ids (unresolved VerdictType references are Pass 5 errors); no two Rules may produce the same VerdictType name (S8 — verdict uniqueness).
- Flow: entry exists; all step refs resolve; all step persona fields resolve to declared Persona constructs; step graph acyclic; flow reference graph acyclic; all OperationSteps and SubFlowSteps declare FailureHandlers; all OperationStep outcome map keys are members of the referenced Operation's declared outcomes; OperationStep outcome handling is exhaustive (map keys = Operation's declared outcomes).
- Parallel: branch sub-DAGs acyclic; no overlapping entity effect sets across branches (transitively resolved).
- **Error attribution:** errors are reported at the source line of the specific field or sub-expression responsible for the violation (e.g., the `initial:` field line, not the `Entity` keyword line; the `verdict_present(...)` call line, not the enclosing `Rule` keyword line). This requires AST nodes at all levels — RawExpr variants, RawStep variants, construct sub-field lines — to carry their own source line, set by the parser at token consumption time and treated as immutable by all elaboration passes.

**Pass 6 — Interchange serialization**
Input: Validated construct index with typed ASTs. Output: TenorInterchange JSON bundle.

- Canonical construct order: Personas (alphabetical), VerdictTypes, Facts, Entities, Rules (ascending stratum, alphabetical within stratum), Operations (alphabetical), Flows (alphabetical). "Alphabetical" means lexicographic ordering of UTF-8 encoded byte sequences.
- Serialize Operation `outcomes` as an array of strings preserving declaration order. For multi-outcome Operations, serialize each effect with an `"outcome"` field associating it with the declared outcome label.
- Serialize Flow steps as an array. Entry step is first; remaining steps follow in topological order of the step DAG.
- Sort all JSON object keys lexicographically within each construct document.
- Represent all Decimal, Money, and Duration values as structured typed objects. No JSON native floats for Decimal or Money values.
- Attach provenance blocks (file, line) to all top-level construct documents.
- Preserve DSL source order for commutative binary expression operands.
- Preserve DSL declaration order for all array values. Array values are never sorted.
- Emit `"tenor"` version and `"kind"` on every top-level document.
- Emit `"tenor_version"` at the bundle top level (see §13.2.1).

### 13.2.1 Interchange Format Versioning

The TenorInterchange format is versioned independently of the Tenor language specification version. The canonical structure of TenorInterchange output is defined by the JSON Schema at `docs/interchange-schema.json`.

**Bundle-level `tenor_version` field:**

Every TenorInterchange bundle includes a `tenor_version` field at the top level. This field is a string in semantic versioning format (e.g., `"1.0.0"`).

```json
{
  "constructs": [ ... ],
  "id": "my_contract",
  "kind": "Bundle",
  "tenor": "1.0",
  "tenor_version": "1.1.0"
}
```

**Version field semantics:**

- `tenor_version` (bundle-level, string, semver, required): The canonical interchange format version. Three-component semver: `MAJOR.MINOR.PATCH`.
  - **Major version** — breaking structural changes to the interchange format (new required fields, removed fields, changed field types, changed construct structure).
  - **Minor version** — additive fields or constructs (new optional fields, new construct kinds that do not affect existing construct schemas).
  - **Patch version** — fixes to serialization behavior (corrected key ordering, fixed decimal representation edge cases) that do not change the logical content.
- `tenor` (per-construct, string): Short version identifier emitted on every top-level construct document. Updated to `"1.0"` for v1.0 interchange. The per-construct `tenor` field provides a quick version check; the bundle-level `tenor_version` is the canonical semver.

**Versioning contract:**

1. **Producers (conforming elaborators):** A conforming elaborator MUST include `tenor_version` in bundle output. The value MUST be a valid semver string corresponding to the interchange format version the elaborator targets.
2. **Consumers (executors, validators, tooling):** A conforming consumer MUST check `tenor_version` for compatibility before processing a bundle.
3. **Major version incompatibility:** A consumer receiving a bundle with a higher major version than it supports MUST reject the bundle with a clear error indicating the version mismatch.
4. **Minor version forward-compatibility:** A consumer supporting major version N can process bundles with any minor version of major version N (older consumers can read newer minor versions). Unknown optional fields are ignored.
5. **Patch version transparency:** Patch version differences are transparent to consumers. A consumer need not distinguish between patch versions.

**v0.3 to v1.0 transition:**

The v0.3 to v1.0 transition is a major version bump. v1.0 interchange is not backward compatible with v0.3. Key breaking changes:
- Persona constructs added to the `constructs` array (new construct kind).
- Operation `outcomes` field added (required).
- Multi-outcome effect-to-outcome association added (new field on effect objects).
- Per-construct `tenor` field updated from `"0.3"` to `"1.0"`.
- Bundle-level `tenor_version` field added (required, not present in v0.3).

> **Note:** This section covers interchange **format** versioning (JSON structure changes). For contract **content** versioning (breaking changes to Facts, Entities, Rules, etc.), see §17 (Versioning & Migration).

### 13.3 Error Reporting Obligation

Every elaboration error must identify: construct kind, construct id (if determinable), field name, source file, source line, and a human-readable description of the violation. Errors referencing internal elaborator state or elaborator-internal terminology are not conforming.

### 13.4 Conformance Test Categories

A conforming elaborator must pass all tests in the Tenor Elaborator Conformance Suite:

- **Positive tests:** Valid DSL that must elaborate without error and produce specific expected interchange output byte-for-byte.
- **Negative tests:** Invalid DSL that must be rejected with errors at specific fields and locations.
- **Numeric precision tests:** Decimal and Money values that must produce exact `decimal_value` interchange representations.
- **Type promotion tests:** Mixed numeric expressions that must produce correctly promoted `comparison_type` fields.
- **Shorthand expansion tests:** All shorthand forms must produce interchange identical to their fully explicit equivalents.
- **Cross-file reference tests:** Multi-file bundles with cross-file refs that must resolve correctly.
- **Parallel entity conflict tests:** Parallel blocks with overlapping entity effects that must be rejected.
- **Verdict uniqueness tests:** Contracts with multiple rules producing the same VerdictType that must be rejected (S8).

_Note: The Tenor Elaborator Conformance Suite is at `conformance/`. It is a prerequisite for any implementation to be declared conforming._

## 14. Complete Evaluation Model

### 14.1 Contract Load Time

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
    — all persona references (Operation allowed_personas, Flow step persona fields,
      HandoffStep from/to_persona, CompensationStep persona, Escalate to_persona)
      resolve to declared Persona constructs

7.  failure_handler_check
    — every OperationStep and SubFlowStep declares a FailureHandler

8.  stratum_check
    — for all rules r, r': if r references output of r' then stratum(r') < stratum(r)

9.  outcome_check
    — every Operation declares a non-empty outcome set
    — outcome labels are unique within each Operation
    — outcomes ∩ error_contract = ∅ for each Operation
    — for multi-outcome Operations, every effect is associated with a declared outcome
    — every OperationStep outcome map key is a member of the referenced Operation's outcomes
    — every OperationStep outcome map is exhaustive (covers all declared outcomes)
```

### 14.2 Flow Initiation

```
snapshot = take_snapshot(contract, current_rules, current_entity_states)
// Point-in-time. Rule evolution after this point does not affect the Flow.
```

### 14.3 Per-Evaluation Sequence

```
// Read path
facts       = assemble_facts(contract, external_inputs)   // → FactSet | Abort
verdicts    = eval_strata(contract.rules, facts)           // → ResolvedVerdictSet

// Write path (per Operation invocation)
(state', outcome_label) = execute(op, persona, verdicts, state) // → (EntityState', OutcomeLabel) | Error

// Orchestration
outcome     = execute_flow(flow, persona, snapshot)        // → FlowOutcome
```

### 14.4 Provenance Chain

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

### 14.5 No Built-in Functions

Tenor provides no built-in functions. There is no `now()`, no `length()`, no `abs()`, no `sqrt()`, no string functions, no date arithmetic functions. All values that vary at runtime must enter the evaluation model through Facts. (`len(list)` in §4.2 is an operator on a ground term, not a built-in function.)

Incorrect:

```
rule active_delegation {
  stratum: 2
  when:    d.valid_from <= now() and  // ERROR: no built-in functions
           d.valid_until >= now()
  produce: delegation_active(true)
}
```

Correct:

```
fact current_time {
  type:   DateTime
  source: executor.clock
}

rule active_delegation {
  stratum: 2
  when:    d.valid_from <= current_time and
           d.valid_until >= current_time
  produce: delegation_active(true)
}
```

---

## 15. Static Analysis Obligations

A conforming static analyzer must derive the following from a contract alone, without execution:

**S1 — Complete state space.** For each Entity, the complete set of states S(e) is enumerable.

**S2 — Reachable states.** For each Entity, the set of states reachable from the initial state via the declared transition relation is derivable.

**S3a — Structural admissibility per state.**  
For each Entity state and each persona, the set of Operations whose preconditions are structurally satisfiable — given only type-level information, without enumerating domain values — and whose effects include a transition from that state is derivable. Structural satisfiability is type-level analysis: a precondition that compares a fact of type `Enum(["pending", "confirmed"])` with the literal `"approved"` is structurally unsatisfiable by type inspection alone. A precondition that compares two compatible typed facts is structurally satisfiable. This analysis is O(|expression tree|) per precondition and is always computationally feasible.

**S3b — Domain satisfiability per state** _(qualified — not always computationally feasible)_  
A stronger version of S3a: for each Entity state and each persona, determine whether there exists a concrete FactSet and VerdictSet assignment under which the precondition evaluates to true. This requires model enumeration over the product of Fact domain sizes. For facts with small declared domains (small Enum sets, narrow Int ranges, short List max bounds) this is feasible. For facts with large declared domains (wide Int ranges, large Decimal precision, long Text max lengths), the enumeration space is O(product of domain sizes), which may be astronomically large for realistic contracts. S3b is decidable in principle for all valid Tenor contracts, but is not computationally feasible in general. Static analysis tools implementing S3b should document their domain size thresholds and fall back to S3a when enumeration is infeasible. S3b should not be treated as an unconditional static analysis obligation.

**S4 — Authority topology.** For any declared Persona P (Section 8) and Entity state S, the set of Operations P can invoke in S is derivable. Whether a persona can cause a transition from S to S' is answerable. The complete set of declared Personas is statically known, enabling exhaustive enumeration of the authority relation.

**S5 — Verdict and outcome space.** The complete set of possible verdict types producible by a contract's rules is enumerable. The complete set of possible outcomes for each Operation is enumerable from the declared outcome set (§9.1). For any Operation O, the analyzer can report: "O can produce outcomes {o1, ..., on}" without executing the Operation.

**S6 — Flow path enumeration.** For each Flow, the complete set of possible execution paths, all personas at each step, all Operation outcomes at each OperationStep, all entity states reachable via the Flow, and all terminal outcomes are derivable. Because OperationStep outcome handling is exhaustive (§11.5), the set of possible paths through an OperationStep is exactly the set of declared outcomes of the referenced Operation.

**S7 — Evaluation complexity bounds.** For each PredicateExpression, the evaluation complexity bound is statically derivable. For each Flow, the maximum execution depth is statically derivable.

**S8 — Verdict uniqueness.** For each VerdictType name, at most one Rule in the contract may declare a `produce` clause for that VerdictType. If two or more Rules (at any stratum) produce the same VerdictType name, the contract is statically rejected. This is a structural check — it does not require predicate satisfiability analysis or reachability reasoning. S8 is enforced during Pass 5.

> **Design note:** Contracts that require conditional production of the same logical verdict from different rules should use distinct VerdictType names for each condition and, if needed, a higher-stratum aggregation rule. This preserves explicit provenance and avoids the need for a runtime resolution strategy.

---

## 16. Executor Obligations

### 16.1 The Conformance Gap

Tenor's formal guarantees hold **conditional on executor conformance**. The language describes a closed world, but its foundations — Fact values, transition atomicity, snapshot isolation — depend on the executor and runtime environment. Where executor obligations are not met, the provenance chain is **corrupt**, not merely incomplete, and Tenor cannot detect non-conformance internally. Implementers should treat E1, E3, and E4 in particular as **trust boundaries**, not implementation details.

### 16.2 Obligation Definitions

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

> **Migration obligations:** When deploying updated contract versions, executors have additional obligations beyond E1-E9. See §17.3 for migration-specific obligations M1-M8, including breaking change detection, migration policy declaration, and entity state migration.

---

## 17. Versioning & Migration

### 17.1 The Migration Problem

Tenor's closed-world semantics (C5) mean that any change to a contract's content is visible and classifiable. This section defines **what** constitutes a breaking change and **what** obligations executors have when deploying a new version. It does not prescribe **how** to implement migration. This section covers **business logic content** versioning (Facts, Entities, Rules, etc.), not interchange **format** versioning (§13.2.1).

### 17.2 Breaking Change Taxonomy

Changes between two contract versions are classified into three categories:

- **BREAKING**: The change may invalidate existing executor behavior, entity state, or in-flight flows. Executors deploying a contract version with BREAKING changes must declare a migration policy (§17.4).
- **NON_BREAKING**: The change is safe — no existing executor behavior is affected. Executors may deploy the new version without a migration policy declaration.
- **REQUIRES_ANALYSIS**: The change's impact depends on value-level analysis (typically predicate strength comparison) that cannot be resolved by structural inspection alone. Changes classified as REQUIRES_ANALYSIS must be treated as BREAKING unless static analysis (S1-S7 per §15) proves them non-breaking for the specific contract pair.

For each change, the taxonomy separately assesses: impact on **new flow initiations** (flows started under the new contract version) versus impact on **in-flight flows** (flows initiated under the old contract version that have not yet reached a terminal state). Frozen verdict semantics (§14) provide natural isolation at the verdict layer — in-flight flow snapshot verdicts are immutable, so Rule and Fact changes do not affect in-flight flow verdicts. However, Operation execution against live entity state IS affected by Entity state changes and Operation definition changes.

The taxonomy is exhaustive: every (construct_kind, field, change_type) triple that is representable in the interchange schema has a classification (MI4). The classification function is decidable: given a diff entry, the classification is a static lookup — no runtime information is needed (MI3).

#### 17.2.1 Fact Changes

| Field | Add Construct | Remove Construct | Change Value |
|-------|--------------|-----------------|-------------|
| id | NON_BREAKING: No existing construct references a new Fact. | BREAKING: Rules and Operations may reference this Fact via `fact_ref`; in-flight flows with frozen snapshots referencing this Fact lose their data source. | N/A (id is identity) |
| type | N/A (part of add) | N/A (part of remove) | **Widen** (larger range, more Enum values): NON_BREAKING — all existing values remain valid. **Narrow** (smaller range, fewer Enum values): BREAKING — existing values may be invalid. **Base type change**: BREAKING — expression type-checking changes. In-flight: verdict-layer isolated (frozen snapshot), but new flow initiations affected. |
| source | N/A (part of add) | N/A (part of remove) | NON_BREAKING: Source is executor metadata (§5.2), not evaluation semantics. Changing `source.system` or `source.field` does not affect `eval_pred` or `eval_strata`. |
| default | N/A (part of add) | N/A (part of remove) | **Add default**: NON_BREAKING — provides fallback where none existed. **Remove default**: BREAKING — Facts without values now fail assembly. **Change default value**: REQUIRES_ANALYSIS — may change evaluation outcomes depending on whether the default is exercised. |
| provenance | N/A (part of add) | N/A (part of remove) | NON_BREAKING: Provenance is debugging metadata, not evaluation semantics. |
| kind | N/A | N/A | N/A (discriminator constant, cannot change within same construct type). |
| tenor | N/A (part of add) | N/A (part of remove) | NON_BREAKING: Version annotation, not evaluation semantics. |

#### 17.2.2 Entity Changes

| Field | Add Construct | Remove Construct | Change Value |
|-------|--------------|-----------------|-------------|
| id | NON_BREAKING: No existing construct references a new Entity. | BREAKING: Operations with effects on this Entity are invalid; Flows with steps transitioning this Entity break; in-flight flows with active entity state lose their target. | N/A (id is identity) |
| states | N/A (part of add) | N/A (part of remove) | **Add state**: NON_BREAKING — existing transitions unaffected, new paths available. **Remove state**: BREAKING — entities in the removed state are orphaned, Operations with transitions from/to the removed state are invalid, in-flight flows in the removed state cannot continue. |
| initial | N/A (part of add) | N/A (part of remove) | BREAKING: New entity instances start in a different state. Existing instances are unaffected, but Flow logic expecting a specific initial state may break. |
| transitions | N/A (part of add) | N/A (part of remove) | **Add transition**: NON_BREAKING — new paths available, existing paths unchanged. **Remove transition**: BREAKING — Operations using this transition are invalid, in-flight flows needing this transition cannot proceed. |
| parent | N/A (part of add) | N/A (part of remove) | **Add parent**: REQUIRES_ANALYSIS — depends on propagation semantics, may introduce new state dependencies. **Remove parent**: REQUIRES_ANALYSIS — removes propagation chain, may orphan dependent behavior. **Change parent**: BREAKING — DAG structure and propagation changes, in-flight entity state hierarchies disrupted. |
| provenance | N/A (part of add) | N/A (part of remove) | NON_BREAKING: Debugging metadata. |
| kind | N/A | N/A | N/A (discriminator constant). |
| tenor | N/A (part of add) | N/A (part of remove) | NON_BREAKING: Version annotation. |

#### 17.2.3 Rule Changes

| Field | Add Construct | Remove Construct | Change Value |
|-------|--------------|-----------------|-------------|
| id | REQUIRES_ANALYSIS: Adding a Rule at stratum 0 is NON_BREAKING (no cross-stratum impact on new verdicts). Adding at higher strata may shadow or conflict with existing Rules' verdict production. In-flight: frozen snapshot means existing verdicts unaffected; new flow initiations may see different verdict sets. | BREAKING: Verdicts this Rule produced are no longer available. Operations and Rules referencing `verdict_present()` for this Rule's verdict types may never fire. Cascading impact on all downstream consumers. | N/A (id is identity) |
| stratum | N/A (part of add) | N/A (part of remove) | BREAKING: Evaluation order changes. Rules at higher strata depend on verdicts from lower strata. Reordering strata can change which verdicts are available when a Rule evaluates, producing different verdict sets. |
| body.when | N/A (part of add) | N/A (part of remove) | REQUIRES_ANALYSIS: Predicate change may widen (more verdicts produced — NON_BREAKING for downstream consumers) or narrow (fewer verdicts — BREAKING for downstream consumers expecting them). Static comparison of predicate strength is undecidable in general. Conservative classification. |
| body.produce.verdict_type | N/A (part of add) | N/A (part of remove) | BREAKING: Downstream Rules and Operations referencing this verdict type via `verdict_present()` are affected. Changing the verdict type breaks all consumers of the original verdict. |
| body.produce.payload.type | N/A (part of add) | N/A (part of remove) | BREAKING: Consumers of this verdict expect a specific payload type. Changing the type breaks payload extraction. |
| body.produce.payload.value | N/A (part of add) | N/A (part of remove) | BREAKING: Different verdict values may cause different downstream behavior (threshold comparisons, routing decisions). |
| provenance | N/A (part of add) | N/A (part of remove) | NON_BREAKING: Debugging metadata. |
| kind | N/A | N/A | N/A (discriminator constant). |
| tenor | N/A (part of add) | N/A (part of remove) | NON_BREAKING: Version annotation. |

#### 17.2.4 Persona Changes

| Field | Add Construct | Remove Construct | Change Value |
|-------|--------------|-----------------|-------------|
| id | NON_BREAKING: No existing construct references a new Persona. | BREAKING: Operations and Flows referencing this Persona in `allowed_personas`, step `persona` fields, and HandoffStep `from_persona`/`to_persona` are invalid. | N/A (id is identity) |
| provenance | N/A (part of add) | N/A (part of remove) | NON_BREAKING: Debugging metadata. |
| kind | N/A | N/A | N/A (discriminator constant). |
| tenor | N/A (part of add) | N/A (part of remove) | NON_BREAKING: Version annotation. |

#### 17.2.5 Operation Changes

| Field | Add Construct | Remove Construct | Change Value |
|-------|--------------|-----------------|-------------|
| id | NON_BREAKING: No existing Flow references a new Operation. | BREAKING: Flows with OperationSteps referencing this Operation are invalid; in-flight flows at steps invoking this Operation cannot proceed. | N/A (id is identity) |
| allowed_personas | N/A (part of add) | N/A (part of remove) | **Add persona**: NON_BREAKING — widens authority, all previously authorized Personas remain authorized. **Remove persona**: BREAKING — narrows authority, existing Flows using the removed Persona at this Operation's steps will fail authorization. |
| precondition | N/A (part of add) | N/A (part of remove) | REQUIRES_ANALYSIS: **Weaken** (more permissive) is NON_BREAKING — all previously valid invocations still valid. **Strengthen** (more restrictive) is BREAKING — previously valid invocations may now fail. Static determination of weaken vs. strengthen is undecidable for arbitrary predicate expressions. |
| effects | N/A (part of add) | N/A (part of remove) | **Add effect** (new entity): NON_BREAKING — does not invalidate existing effects. **Remove effect**: BREAKING — entity state transitions no longer occur, in-flight flows expecting these transitions will have incorrect entity states. **Change effect** (different from/to): BREAKING — different state transition behavior, in-flight flows at this Operation step will transition entities to unexpected states. |
| outcomes | N/A (part of add) | N/A (part of remove) | **Add outcome**: BREAKING — exhaustive handling in Flows (§11.5) means all OperationSteps referencing this Operation must handle the new outcome. Existing Flows that do not handle the new outcome are invalid. **Remove outcome**: BREAKING — Flows handling this outcome have dead routing paths. |
| error_contract | N/A (part of add) | N/A (part of remove) | **Add error type**: NON_BREAKING — new failure modes, existing handling unaffected. **Remove error type**: REQUIRES_ANALYSIS — if failure handlers reference specific errors, removing an error type may leave dead handler code. |
| provenance | N/A (part of add) | N/A (part of remove) | NON_BREAKING: Debugging metadata. |
| kind | N/A | N/A | N/A (discriminator constant). |
| tenor | N/A (part of add) | N/A (part of remove) | NON_BREAKING: Version annotation. |

#### 17.2.6 Flow Changes

| Field | Add Construct | Remove Construct | Change Value |
|-------|--------------|-----------------|-------------|
| id | NON_BREAKING: No existing construct references a new Flow (unless SubFlowStep). | BREAKING: SubFlowSteps referencing this Flow are invalid; in-flight instances of this Flow are orphaned by the contract. | N/A (id is identity) |
| entry | N/A (part of add) | N/A (part of remove) | BREAKING: Different execution path for new flow initiations. In-flight flows are unaffected (they already passed entry). |
| steps | N/A (part of add) | N/A (part of remove) | **Add step**: REQUIRES_ANALYSIS — depends on whether existing routing paths are modified. If the new step is reachable only via new routing, NON_BREAKING. If inserted into existing paths, BREAKING. **Remove step**: BREAKING — references to the removed step from other steps' outcome routing or branch targets are invalid. In-flight flows currently at or routing through the removed step cannot proceed. **Change step** (routing, persona, operation): BREAKING — different execution paths, authority, or Operation invocation for Flows reaching this step. |
| snapshot | N/A (part of add) | N/A (part of remove) | BREAKING: Changes when verdicts are frozen. Currently always `at_initiation` in v1.0, so any change violates the v1.0 spec. |
| provenance | N/A (part of add) | N/A (part of remove) | NON_BREAKING: Debugging metadata. |
| kind | N/A | N/A | N/A (discriminator constant). |
| tenor | N/A (part of add) | N/A (part of remove) | NON_BREAKING: Version annotation. |

### 17.3 Executor Migration Obligations

The following obligations parallel E1-E9 in §16.2 but are specific to contract version transitions. An executor that supports deploying updated contract versions must satisfy these obligations.

- **M1 — Breaking change detection.** An executor MUST be able to determine whether a contract version transition contains breaking changes by applying the taxonomy from §17.2 to the structural diff between two interchange bundles. The diff compares every field of every construct in both bundles, keyed by `(kind, id)` — not by array position (MI1, MI2).

- **M2 — Migration policy declaration.** An executor deploying a new contract version with any BREAKING changes (as classified by §17.2) MUST declare an in-flight flow migration policy per §17.4 (MI5).

- **M3 — No silent breaking deployment.** An executor MUST NOT silently deploy a contract version with BREAKING changes without migration policy declaration. Deployment of a breaking change without a declared policy is a conformance violation (MI5).

- **M4 — Entity state migration.** An executor MUST validate that all entities in states removed by the new contract version have been migrated before completing the transition. Entities in removed states are orphaned — no Operation in the new contract can transition them, and no Flow step can reference them.

- **M5 — In-flight flow coverage.** An executor MUST validate that all in-flight flows referencing removed or changed constructs are handled per the declared migration policy. No in-flight flow may be silently abandoned or left in an inconsistent state (MI7).

- **M6 — Conservative REQUIRES_ANALYSIS handling.** Changes classified as REQUIRES_ANALYSIS MUST be treated as BREAKING unless static analysis (S1-S7 per §15) proves them non-breaking for the specific contract pair. An executor that cannot perform S1-S7 analysis must treat all REQUIRES_ANALYSIS classifications as BREAKING (MI3).

- **M7 — Diff noise field exclusion.** The structural diff used for breaking change detection MUST exclude `provenance` and `line` fields from comparison. These fields are debugging metadata and do not affect evaluation semantics. Changes to provenance or line numbers alone do not constitute a contract change.

- **M8 — Set-semantic comparison for primitive arrays.** Fields that represent unordered sets (`states`, `allowed_personas`) MUST be compared as sets, not as ordered arrays. Reordering elements within these fields is not a change.

### 17.4 In-Flight Flow Migration Policy

When a contract version transition contains BREAKING changes (per §17.2, classified by M1), the executor MUST declare one of three migration policies before the new version becomes active:

**1. Blue-Green.** New flow initiations use the new contract version. In-flight flows (initiated under the old version) complete execution under the old version. No in-flight flow is affected by the breaking changes. The executor must maintain both contract versions simultaneously until all in-flight flows under the old version reach terminal states. This policy provides the strongest isolation but requires the highest resource overhead (two concurrent contract version environments).

**2. Force-Migrate.** All in-flight flows transition to the new contract version. The executor MUST handle the consequences of breaking changes: entities in removed states must be migrated to valid states, Operations with changed effects must be re-evaluated, Flows at removed steps must be routed to valid steps. The executor bears full responsibility for data consistency during the transition. This is the most complex policy and places the highest burden on the executor. See §17.6 for the formal conditions under which force-migration is safe for a specific flow instance.

**3. Abort.** In-flight flows that traverse any construct affected by a breaking change are terminated with a `migration_aborted` failure outcome. Flows that do not traverse affected constructs continue under the new version. The executor must identify affected flows and terminate them gracefully. This is the simplest policy but may lose in-progress work.

**Policy constraints:**

- The policy is a **deployment concern**, not a contract concern. It does not appear in `.tenor` source files or interchange JSON. It does not affect evaluation semantics, verdict resolution, or any contract-level behavior (MI6). The policy is consumed by the executor's deployment infrastructure, not by the evaluation model.
- The policy must address **all** in-flight flows, not a subset. Per-flow-type policies are permitted (e.g., abort Flow type A, blue-green Flow type B) but every active flow type must have a declared policy (MI7).
- The spec does not prescribe which policy to use — only that one MUST be declared when BREAKING changes exist (MI5).
- Frozen verdict semantics (§14) provide natural isolation at the verdict layer: in-flight flow snapshot verdicts are immutable, so Rule and Fact changes do NOT affect in-flight flow verdicts. However, Operation execution against live entity state IS affected by Entity state changes and Operation definition changes. The Blue-Green policy leverages verdict isolation fully; the Force-Migrate and Abort policies must account for entity state and Operation definition changes.

### 17.5 Migration Contract Representation

The migration output between two contract versions is expressed as a hybrid representation (selected via CFFP, Candidate C — see `docs/cffp/migration-semantics.json`):

**Primary output — DiffEntry JSON** (`tenor diff`): The authoritative diff output is structured JSON. Each change is a `DiffEntry` keyed by `(construct_kind, construct_id)` with field-level before/after values. The DiffEntry format is produced by `tenor diff` and is always complete, deterministic, and correct (MI1, MI2). The breaking change classification is a pure function applied to each DiffEntry field: `classify(kind, field, change_type)` returns the taxonomy classification from §17.2.

**Secondary output — Tenor migration contract** (`tenor diff --migration`): An optional supplementary output generates a valid Tenor contract from the diff. The migration contract uses existing v1.0 constructs with conventionalized naming:

- **Facts** encode diff data — each changed construct becomes a set of typed Facts (construct_kind, construct_id, change_type, field before/after values) with conventionalized source bindings (`system: "tenor-diff"`).
- **Rules** encode taxonomy classifications — each taxonomy entry becomes a Rule that matches on the appropriate `(construct_kind, field, change_type)` pattern and produces a classification verdict.

The migration contract is **classification-only** in v1.0. Migration orchestration (Operations that execute migration actions, Flows that sequence multi-step migrations) would require meta-level constructs not present in the v1.0 spec and is deferred to v2.

**Authoritative relationship:** The DiffEntry JSON is always authoritative. The migration contract is derived from the DiffEntry output (one-way dependency) and is supplementary. Both representations produce equivalent breaking change classifications; when the DiffEntry JSON and migration contract disagree, the DiffEntry JSON is canonical.

**Acknowledged limitations of migration contracts:**

- Cannot express arbitrary type changes involving complex types (Record, TaggedUnion, nested List). Only simple type parameter changes (Int min/max, Decimal precision/scale, Enum value addition/removal) are faithfully representable as Tenor Facts.
- Cannot perform predicate strength comparison. All precondition and predicate expression changes are conservatively classified as REQUIRES_ANALYSIS.
- Are self-contained — they do not import the contracts they migrate. All diff data is encoded as internal Facts with conventionalized source bindings (MI1 completeness via self-containedness rather than cross-contract referencing).
- Are not composable in v1.0. Transitive migration (v1-to-v3 via v1-to-v2 + v2-to-v3) requires direct diffing of endpoint versions. This is a v1.0 implementation constraint, not a fundamental design limitation: classification-only contracts do not carry enough state information to compose. Once migration contracts gain orchestration capabilities in v2 (Operations executing migration actions, Flows sequencing steps), composition becomes tractable — a completed v1-to-v2 migration produces a known state, which can be diffed against v3 to yield v2-to-v3. The composition model is sequential execution, not algebraic contract combination. See AL40.
- Generation must follow canonical ordering rules (alphabetical Fact ids, deterministic Rule ids, canonical construct ordering) to ensure deterministic output across different generators (MI2).

### 17.6 Flow Migration Compatibility

Flows are long-lived stateful processes that may outlive deployment cycles. A flow initiated under contract v1 may still be executing when contract v2 is deployed. When an executor uses the Force-Migrate policy (§17.4), it must determine per-flow-instance whether migration is safe. This section defines the formal conditions.

#### 17.6.1 Position and Reachable Paths

A flow instance's **position** is the step where it is currently waiting:

```
Position(flow_instance) = step_id
```

The **reachable paths** from a position are all execution paths from that step to a terminal state, computed using v2's step graph:

```
ReachablePaths(v2_flow, position) = { path | path is a sequence of steps
    from position to Terminal(success) or Terminal(failure)
    following v2's routing edges }
```

Reachable paths are computed per S6 (§15). The v1 step graph determines the current position; all forward analysis uses v2's definitions, since the migrated flow executes under v2.

For each step type, the path computation branches as follows:
- **OperationStep:** one successor per declared outcome in v2's operation definition.
- **BranchStep:** two successors (if_true, if_false).
- **SubFlowStep:** success path + failure handler path. The referenced sub-flow's reachable paths are computed recursively.
- **ParallelStep:** Cartesian product of branch paths, plus the join step.

#### 17.6.2 Step Equivalence

A v1 step at the current position has a v2 equivalent if and only if:

1. v2 contains a step with the same step id.
2. The v2 step references the same operation (by operation id).
3. The v2 step's persona is authorized for the operation under v2's definitions (the persona exists in v2's persona declarations and is in the operation's `allowed_personas` in v2).

Routing is NOT part of step equivalence. The migrated flow uses v2's routing definitions at and after the current position.

#### 17.6.3 Compatibility Conditions

A flow instance at position p in contract v1 is **force-migratable** to contract v2 if and only if ALL three conditions hold:

**Condition 1 — Forward Path Existence (FMC1).** For every step in ReachablePaths(v2, p), there exists a step in v2 with equivalent semantics per §17.6.2. This ensures the flow can reach a terminal state under v2. Note that reachable paths are computed from the current position using **v2's** step graph, not v1's (§17.6.1). Routing changes at or after the current position are evaluated under v2's semantics — a step removed in v2 is not reachable; a step added in v2 may introduce new dependencies checked by FMC2.

**Condition 2 — Data Dependency Satisfaction (FMC2).** For every step s in ReachablePaths(v2, p), all data dependencies of s's operation under v2 are satisfiable from the execution context established by v1's partial execution:

- **(a) Fact dependencies:** Every `fact_ref` in the operation's precondition must have a value in the frozen snapshot (taken at v1 flow initiation per §14), or the fact must have a declared default in v2.
- **(b) Verdict dependencies:** Satisfied by construction. The frozen verdict snapshot is immutable (§14). Changes to Rule definitions, Fact types affecting verdict production, or Rule removal in v2 do not affect verdicts already frozen in the snapshot. Verdict-layer changes are categorically safe for in-flight flows (see §17.6.5).
- **(c) Entity state dependencies:** The entity state required as a transition source by the operation's effects must be the current state of the entity (as established by v1 execution) or reachable from the current state via v2's declared transitions. Checked by Layer 2 (§17.6.6).
- **(d) Persona authorization:** The step's persona must be in the operation's `allowed_personas` under v2's definitions.

**Condition 3 — Entity State Equivalence (FMC3).** For every entity e referenced by any step in ReachablePaths(v2, p):

- The current state of e (as established by v1 operations executed before position p) must be a member of v2's entity declaration for e.
- All entity states that are transition targets of v2 operations in the reachable path must be declared in v2's entity definition.
- Transitions from the current state to those targets must exist in v2's transition declarations.

#### 17.6.4 Directional Asymmetry (FMC4)

The compatibility function must account for a structural asymmetry between v1's executed path and v2's expected path. When v2 introduces new steps between existing steps, or changes existing steps to have stronger preconditions, the new data dependencies may reference state (facts, verdicts, entity states) that v1's execution path never established because v1 never executed the steps that would produce them.

Forward path existence (FMC1) alone is insufficient. A path may exist structurally in v2 but be unexecutable because its data dependencies assume a v2-specific execution history that the v1 flow does not have. FMC2 captures this: the data dependency check evaluates v2's dependencies against v1's actual execution context (frozen snapshot plus current entity states), not against v2's assumed execution context.

**Example:** v1 flow: step_confirm -> step_check_threshold -> step_auto_release -> Terminal(success). v2 inserts step_compliance_check between step_confirm and step_check_threshold, and step_auto_release in v2 now requires `verdict_present(compliance_cleared)`. A v1 flow at step_check_threshold has a compatible forward path (step_check_threshold and step_auto_release exist in v2). However, FMC2 fails: the compliance_cleared verdict was never produced because the v1 flow never executed step_compliance_check. The directional asymmetry makes FMC2 the hardest condition to verify — it requires analyzing what WOULD have been produced by steps the flow has already passed.

#### 17.6.5 Frozen Verdict Layer Isolation

The frozen verdict snapshot (§14) provides natural isolation at the verdict layer. Changes to Rules and Facts do NOT affect the frozen snapshot of an in-flight flow — verdicts were computed at initiation time and are immutable. This means Rule/Fact changes never cause FMC2 failures for verdict dependencies.

However, entity state changes (live) and operation definition changes (evaluated at step execution time) ARE affected and must be checked. This insight means the compatibility analysis can skip the verdict layer entirely.

#### 17.6.6 Three-Layer Analysis Model

The compatibility check decomposes into three analysis layers corresponding to Tenor's isolation properties:

**Layer 1 — Verdict isolation.** ALWAYS PASSES for in-flight flows. The frozen verdict snapshot (§14) is immutable. Changes to Rule definitions, Fact types, or Rule removal in v2 do not affect verdicts already in the snapshot. This is a theorem of Tenor's evaluation model, not an assumption.

**Layer 2 — Entity state equivalence.** Checks FMC3. Entity states are live (mutable by operations), not frozen. For each entity e referenced in ReachablePaths(v2, p):

- Verify `current_state(e)` is in `v2.entities[e].states`.
- Verify all transition targets needed by v2 operations in the reachable path are declared in `v2.entities[e].transitions`.
- Verify transitions from `current_state(e)` to required target states exist in v2.

If any check fails: return `incompatible(layer=2, entity=e, reason)`.

**Layer 3 — Operation/flow structure.** Checks FMC1 + FMC2 (minus verdict dependencies, which are handled by Layer 1). For each step s in ReachablePaths(v2, p):

- **Step equivalence:** v2 has step s with the same operation id. `s.persona` is in v2's `operation.allowed_personas`.
- **Fact dependencies:** Every `fact_ref` in v2's operation precondition is present in `frozen_snapshot.facts` OR has a declared default in v2.
- **Persona authorization:** `s.persona` exists in v2's persona declarations.
- **SubFlowStep:** Recursively check the referenced sub-flow at its entry point.
- **ParallelStep:** Check all branches independently; all must pass.

If any check fails: return `incompatible(layer=3, step=s, reason)`.

Evaluation order: Layer 1 (trivial — no computation), Layer 2 (entity state — set membership checks), Layer 3 (structure and dependencies — graph traversal). Short-circuit on first failure.

#### 17.6.7 Position Sensitivity (FMC5)

Flow compatibility is a function of:

```
compatible(v1_flow, v2_flow, position, entity_states, snapshot)
    -> Compatible | Incompatible(layer, location, reasons)
```

It is NOT a static property of two flow definitions. A flow may be compatible at one position and incompatible at another. Compatibility analysis must be performed per-flow-instance, not per-flow-type, because each instance has a specific position and entity state context.

**Example:** v2 removes entity state `cancelled` from an Order entity. A flow instance at step_submit_order (Order in state `draft`, no future path transitions to `cancelled`) is compatible. A flow instance at step_cancel_order (which transitions Order to `cancelled`) is incompatible. Same flow definition pair, different results.

#### 17.6.8 Recursive Sub-Flow Compatibility (FMC6)

If the reachable path from position p includes a SubFlowStep referencing sub-flow F, then F must be compatibility-checked at its entry point under the same conditions (same frozen snapshot, same entity state context). Compatibility checking is transitive through the flow reference DAG. The DAG is acyclic (Tenor spec constraint, §11.5), so the transitive check terminates.

#### 17.6.9 Semantic Non-Interference (FMC7)

The compatibility analysis is a deployment-time static check with no side effects. It does not modify flow execution semantics, entity states, snapshots, or verdicts under either v1 or v2. A flow that passes the compatibility check executes under v2 semantics exactly as if it had been initiated under v2. A flow that fails the compatibility check continues under v1 semantics (or is aborted, depending on executor policy) with no change to its evaluation model.

#### 17.6.10 Flow-Level Refinement of Breaking Changes

Flow-level compatibility refines the construct-level breaking change taxonomy (§17.2). A construct-level BREAKING change (e.g., a new operation outcome per §17.2.5) may be flow-level COMPATIBLE if v2's flow definition already handles the change. The construct-level taxonomy provides the initial signal; the flow-level check provides per-instance refinement.

#### 17.6.11 Conservative and Aggressive Analysis

**Conservative analysis (REQUIRED):** Data dependency satisfaction considers only the frozen snapshot and current entity states. Dependencies not present in these sources fail the check. This may reject migrations that are actually safe (false negatives).

**Aggressive analysis (OPTIONAL):** Additionally considers dependencies satisfiable from v2 steps that MUST execute before the dependent step (path dominance analysis). If step A always executes before step B on every path from the current position, and step A produces a verdict that step B requires, the dependency is satisfied by intra-path production. Aggressive analysis reduces false negatives at the cost of implementation complexity.

#### 17.6.12 Coexistence Layer Pattern

When an executor declares Force-Migrate policy and some flow instances fail the compatibility check, the executor MAY implement a coexistence layer (informally called v1.5):

- New flow initiations execute under v2.
- Compatible in-flight flow instances are force-migrated to v2.
- Incompatible in-flight flow instances continue executing under v1.
- When a v1-retained flow reaches a terminal state, its results are translated to v2's output format if needed.

The coexistence layer is an **executor implementation strategy**, not a spec-level obligation. The spec defines the compatibility conditions; how the executor handles incompatible instances is an executor choice (blue-green, abort, coexistence, or other strategies). The coexistence pattern generalizes blue-green (all on v1) and force-migrate (all on v2) by allowing mixed assignment based on per-instance compatibility analysis.

---

## 18. Contract Discovery & Agent Orientation

This section specifies how executors expose contracts to agents and how agents
orient themselves against a running executor. These are executor obligations and
interchange metadata additions — no new language constructs are introduced and
no existing construct semantics are modified.

The design constraint driving this section: the preamble's core semantic
requirement ("any agent that can read this specification can fully understand a
system described in it, without reading any implementation code") implies the
agent must first be able to *find* the interchange bundle. Without a prescribed
discovery mechanism, every executor invents its own, and agents cannot
generalize across executors. This section closes that gap.

---

### 18.1 The Contract Manifest

The contract manifest is a JSON document that exposes a Tenor interchange bundle
at a well-known location. It is the entry point for agent cold-start and the
source of truth for change detection.

```
TenorManifest = {
  tenor:          string,               // manifest schema version, e.g. "1.1"
  etag:           string,               // SHA-256 hex digest of canonical bundle bytes
  bundle:         TenorInterchange,     // the full interchange bundle, inlined
  capabilities?:  ExecutorCapabilities  // optional executor capability advertisement
}

ExecutorCapabilities = {
  migration_analysis_mode: "conservative" | "aggressive"
}
```

The `capabilities` field is optional. Static file servers and pre-v1.1 manifests
omit it. Dynamic executors that evaluate Operations, execute Flows, and apply
entity state transitions include it. The `ExecutorCapabilities` object is
explicitly extensible — future executor capability signals land here.

The `capabilities` field is excluded from etag computation (§18.2). Capability
changes do not constitute contract changes and do not invalidate cached bundles.

**Manifest schema version:** The manifest's `tenor` field tracks the manifest
schema version, not the interchange format version and not the Tenor language
spec version. These are three independent version axes:

- `tenor` on the manifest: manifest schema version (`"1.0"` = original, `"1.1"` = with capabilities)
- `tenor` on each construct: per-construct interchange format version (`"1.0"`)
- `tenor_version` on the bundle: interchange format semver (`"1.1.0"`)

The manifest is a static artifact. It requires no live server to serve. A
conforming executor may serve it from a CDN, object storage, or a local file
system. Dynamic executors may generate it on request, but must produce output
identical to what a static file would contain for the same bundle.

**Canonical form:** The manifest is serialized as JSON with all top-level keys
sorted lexicographically: `bundle`, `capabilities` (if present), `etag`,
`tenor`. The `bundle` field contains the interchange bundle exactly as produced
by the elaborator — no fields added, no fields removed. The `etag` field is
computed after the bundle is serialized to its canonical form.

**Elaborator integration:** `tenor elaborate --manifest <file.tenor>` produces
the manifest with the interchange bundle inlined. The manifest is a
transformation of the elaborator output, not a separate pipeline. The elaborator
computes the etag as part of manifest generation.

**JSON Schema:** The manifest validates against `docs/manifest-schema.json`,
a separate schema from the interchange schema (`docs/interchange-schema.json`).
The interchange schema is embedded in the manifest schema as the type of the
`bundle` field.

---

### 18.2 Etag Semantics

The etag is a SHA-256 hex digest of the canonical interchange bundle bytes.

```
etag(bundle) = lowercase_hex(SHA-256(canonical_json_bytes(bundle)))
```

Where `canonical_json_bytes` is the deterministic JSON serialization produced
by the elaborator's Pass 6. The elaborator is already required to be
deterministic (§13.1): identical DSL input produces byte-for-byte identical
interchange output. The etag inherits this determinism — identical contracts
produce identical etags across all conforming elaborators.

**Change detection:** An etag changes if and only if the interchange bundle
changes. An etag does not change when deployment metadata, timestamps, or
other non-bundle state changes. This is a structural guarantee, not a
convention — the etag is a pure function of bundle content.

**HTTP integration:** Executors that serve the manifest over HTTP MUST set the
`ETag` response header to the value of the manifest's `etag` field. This
enables standard HTTP conditional GET (`If-None-Match`) for efficient change
detection. An agent that has previously fetched the manifest may send
`If-None-Match: <etag>` and receive `304 Not Modified` if the contract has not
changed, without re-fetching the full bundle.

The `capabilities` field (§18.1) is excluded from etag computation. Executor
capability changes do not constitute contract changes and do not invalidate
cached bundles.

---

### 18.3 Discovery Endpoint

A conforming executor MUST serve the contract manifest at:

```
/.well-known/tenor
```

The resource MUST be served with `Content-Type: application/json`. No
file extension is appended to the path. This path is prescribed exactly —
executors MUST NOT serve the manifest only at implementation-specific paths.
Executors MAY additionally serve the manifest at other paths.

For static file deployments, the manifest file is placed at the path
`/.well-known/tenor` relative to the document root. For dynamic deployments,
the endpoint is a route handler that returns the manifest.

**Executor obligation E10:** A conforming executor MUST serve a valid
TenorManifest at `/.well-known/tenor` with `Content-Type: application/json`
and an `ETag` response header matching the manifest's `etag` field value.

---

### 18.4 Cold-Start Protocol

Agent cold-start is the sequence an agent follows from a bare URL to a complete
understanding of the system. The protocol requires at most one network fetch.

```
cold_start(base_url) =
  manifest = GET base_url + "/.well-known/tenor"
  bundle   = manifest.bundle
  // Agent now has the complete interchange bundle.
  // All contract semantics are derivable from bundle alone.
  return (bundle, manifest.etag)
```

The bundle is inlined in the manifest, so no second fetch is required. An agent
that has completed cold-start has:

- The complete set of declared Facts, Entities, Rules, Personas, Operations,
  and Flows
- The complete state space and all reachable states (via S1 analysis)
- The complete authority topology — which Personas can invoke which Operations
  in which states (via S4 analysis)
- The complete set of possible verdicts and their derivation conditions
- The etag for subsequent change detection

Persona resolution is not part of cold-start. An agent learns its effective
Persona when it first invokes an Operation — the executor returns
`persona_rejected` (§9.2) if the presented credential does not map to a
declared Persona. The set of declared Personas is statically enumerable from
the bundle (§8.3), so an agent may reason about the authority model before
any execution.

**Executor obligation E11:** A conforming executor MUST ensure that an agent
which has fetched the manifest at `/.well-known/tenor` has all information
necessary to understand the contract without any additional out-of-band
documentation. The manifest's inlined bundle MUST be complete — no required
fields may be omitted, no construct references may be unresolved.

---

### 18.5 Change Detection

An agent that has previously fetched the manifest detects contract changes by
comparing the current etag to its cached etag.

```
check_for_changes(base_url, cached_etag) =
  response = GET base_url + "/.well-known/tenor"
             with header If-None-Match: cached_etag
  if response.status == 304:
    return no_change
  if response.status == 200:
    new_manifest = response.body
    if new_manifest.etag != cached_etag:
      return changed(new_manifest)
    else:
      return no_change  // defensive: etag matched despite 200
```

A contract change requires the agent to re-run cold-start against the new
manifest. The agent MUST NOT apply partial updates — a changed etag means the
full bundle has changed and must be re-fetched and re-processed.

**Executor obligation E12:** A conforming executor MUST update the manifest's
`etag` field whenever the interchange bundle changes, and MUST NOT change the
`etag` field when the bundle has not changed. The etag MUST be a pure function
of bundle content as specified in §18.2.

---

### 18.6 Dry-Run Evaluation

A dry-run is a read-only evaluation of an Operation against the current
ResolvedVerdictSet. It executes the full evaluation sequence up to but not
including effect application. It is used by agents to preflight operations
before committing side effects.

```
dry_run : Operation × PersonaId × ResolvedVerdictSet × EntityState
        → (SimulatedOutcome | SimulatedError)

dry_run(op, persona, verdict_set, entity_state) =
  if persona ∉ op.allowed_personas:
    return SimulatedError("persona_rejected", simulation: true)
  if ¬eval_pred(op.precondition, FactSet, verdict_set):
    return SimulatedError("precondition_failed", simulation: true)
  outcome = determine_outcome(op, entity_state)
  emit_simulated_provenance(op, persona, verdict_set, entity_state, outcome)
  return SimulatedOutcome(outcome, simulation: true)
```

Steps (1), (2), and (3) of the execution sequence (§9.3) are performed.
Step (4) — atomic effect application — is not performed. Step (5) — provenance
emission — emits a SimulatedProvenance record, not a real provenance record.

**Simulation flag:** Every dry-run response MUST carry `"simulation": true` at
the top level of the response body. This flag is not optional metadata — it is
a required field that distinguishes simulated results from real execution
results. An agent MUST check this flag before treating a response as authoritative.

**SimulatedProvenance:** Structurally identical to a real provenance record
(§9.5) with one additional field: `simulation: true`. SimulatedProvenance
records carry the same derivation information as real provenance records —
facts used, verdicts used, state before, would-be outcome — but are tagged
as simulations.

**Audit log exclusion:** SimulatedProvenance records MUST NOT be written to
the authoritative audit log. A dry-run produces no durable state change of any
kind. An executor that writes SimulatedProvenance to the audit log is
non-conforming.

**HTTP integration:** Dry-run requests are distinguished by the request, not
the response status code. The executor uses the same status codes for dry-run
as for real execution — a dry-run that would succeed returns 200, a dry-run
that would fail precondition returns the same error status as a real failure.
Agents use the same response handling code for both paths. The `simulation: true`
field in the response body is the sole distinguishing signal.

**Executor obligation E13:** A conforming executor MUST support dry-run
evaluation for all Operations. Dry-run requests MUST execute steps (1)-(3) of
the execution sequence (§9.3) and MUST NOT execute step (4). Dry-run responses
MUST carry `"simulation": true`. SimulatedProvenance records MUST NOT be written
to the authoritative audit log.

---

### 18.7 Executor Obligation Summary (E10-E14)

| Obligation | Description |
|------------|-------------|
| **E10** | Serve a valid TenorManifest at `/.well-known/tenor` with `Content-Type: application/json` and an `ETag` response header matching the manifest's `etag` field. |
| **E11** | Ensure the manifest's inlined bundle is complete — no required fields omitted, no construct references unresolved. The manifest alone must be sufficient for agent cold-start. |
| **E12** | Update the manifest etag when and only when the interchange bundle changes. The etag is a pure function of bundle content: `lowercase_hex(SHA-256(canonical_json_bytes(bundle)))`. |
| **E13** | Support dry-run evaluation for all Operations. Execute steps (1)-(3) of §9.3. Do not execute step (4). Carry `"simulation": true` in all dry-run responses. Never write SimulatedProvenance to the authoritative audit log. |
| **E14** | **Capability advertisement.** A dynamic executor (one that evaluates Operations, executes Flows, and applies entity state transitions) MUST include a `capabilities` object in the manifest it serves at `/.well-known/tenor`. The `capabilities` object MUST accurately reflect the executor's actual analysis behavior. Static file deployments (serving the manifest from a CDN, object storage, or file system with no live executor) are exempt from E14. A manifest served without a `capabilities` field is interpreted as a static deployment or a pre-v1.1 executor; agents MUST assume conservative defaults for all capability dimensions. |

---

## 19. Appendix A — Acknowledged Limitations

These are conscious design decisions, not oversights.

**AL1 — Fact ground property boundary** _(Fact 1.0)_
Facts are ground within the evaluation model. Whether the source populating them is itself derived is outside the language's enforcement scope. Conforming executors must not populate Facts from internal evaluations.

**AL5 — TaggedUnion absence semantics** _(BaseType)_
Mismatched tag access produces a typed absence value, which evaluates to false in predicate context. Contract authors must account for this: a negated predicate over a TaggedUnion field evaluates to true for all non-matching variants (§4.4).

**AL8 — List max is a conservative static bound** _(Fact extension)_
Runtime lists may be smaller. Static complexity analysis uses the declared max.

**AL17 — Branch decision provenance** _(Flow)_
Branch decisions are recorded in Flow provenance but not in the Operation provenance chain.

**AL18 — Duration calendar independence** _(Duration)_
Duration "day" means exactly 86,400 seconds. DST transitions, leap seconds, and calendar month/year spans are not representable as Duration values. Adapters must handle calendar-to-Duration conversion before Fact assertion.

**AL22 — Post-parallel verdict re-evaluation requires new Flow** _(ParallelStep)_
Frozen verdict semantics apply within parallel blocks. If parallel branch results must feed into verdict evaluation, a new Flow must be initiated after the parallel block completes.

**AL24 — Persona declaration is mandatory in v1.0** _(Persona)_
Contracts written against v0.3 that use bare persona strings in Operation `allowed_personas` and Flow step `persona` fields must add explicit `persona` declarations when migrating to v1.0. This is a breaking change covered by the v0.3 to v1.0 major version bump.

**AL28 — Outcome labels carry no typed payload** _(Operation)_
Outcome labels are bare strings with no associated payload data. If outcome-specific data is needed, it must be conveyed through entity state changes or separate Facts. Typed outcome payloads were rejected because payload values have no derivation chain within the closed-world evaluation model, violating C7 (provenance as semantics).

**AL30 — Operation outcome declarations are mandatory in v1.0** _(Operation)_
Contracts written against v0.3 that use ad-hoc outcome labels in Flow OperationSteps must add corresponding `outcomes` declarations to their Operations when migrating to v1.0. The migration path is additive: existing outcome labels become the declared outcome set on the referenced Operation.

**AL31 — Module federation deferred to v2** _(P5 Shared Type Library)_
Inter-organization type sharing (type registries, versioned type packages, cross-repository type distribution) is explicitly out of scope for v1.0. The shared type library mechanism supports only direct file import within a single project.

**AL32 — Generic type parameters deferred to v2** _(P5 Shared Type Library)_
Shared type libraries cannot define parameterized types (e.g., `GenericList<T>`). Each concrete type variant must be declared separately.

**AL33 — Type library files may not import other files** _(P5 Shared Type Library)_
Type library files are self-contained leaf files in the import graph. This restriction prevents transitive type propagation and eliminates the need for a module visibility system.

**AL34 — TypeDecl flat namespace across imports** _(P5 Shared Type Library)_
TypeDecl names occupy a flat namespace. Two imported type libraries that declare the same TypeDecl id cause an elaboration error. Namespace prefixing, aliasing, or selective imports are not supported in v1.0.

**AL35 — No type extension or inheritance across libraries** _(P5 Shared Type Library)_
A contract cannot import a type from a library and extend it (add fields). Type extension and inheritance are not supported in v1.0.

**AL36 — No selective type import** _(P5 Shared Type Library)_
Importing a type library file loads all its TypeDecl definitions into the type environment, even if the contract uses only a subset.

**AL37 — Migration contracts cannot express complex type changes** _(Migration)_
Migration contracts (§17.5) cannot represent arbitrary type changes involving complex types (Record, TaggedUnion, nested List). Only simple type parameter changes are faithfully representable as Tenor Facts.

**AL38 — Migration contracts cannot compare predicate strength** _(Migration)_
All precondition and predicate expression changes are conservatively classified as REQUIRES_ANALYSIS. Refinement requires S3a/S3b static analysis tooling (§15).

**AL39 — Migration contracts are self-contained** _(Migration)_
Migration contracts do not import the contracts they migrate. All diff data is encoded as internal Facts with conventionalized source bindings (`system: "tenor-diff"`). This restriction is due to Pass 1 import resolution merging all constructs into a single namespace, which would cause id collisions.

**AL40 — Migration contracts are not composable in v1.0** _(Migration)_
Transitive migration requires directly diffing the endpoint versions' interchange bundles. Classification-only contracts encode what changed but not the resulting state, so they lack the information needed for composition. See §17.5.

**AL41 — Migration contracts are classification-only in v1.0** _(Migration)_
Migration contracts express diff classification but NOT migration orchestration. Migration orchestration (Operations + Flows for multi-step migrations) requires meta-level constructs deferred to v2. See §17.5.

**AL42 — Migration contract source bindings are conventionalized** _(Migration)_
Migration contract Facts use conventionalized source bindings (`system: "tenor-diff"`) that do not correspond to real external systems.

**AL43 — Migration contract determinism requires canonical ordering** _(Migration)_
Migration contract generation must follow canonical ordering rules to ensure deterministic output across different generators.

**AL44 — Flow compatibility does not model time-based constraints** _(Flow Migration, §17.6)_
The compatibility analysis does not account for timeout changes between contract versions. If v2 changes step-level or flow-level timeout values, the analysis treats timeouts as executor-level concerns outside the formal compatibility conditions.

**AL45 — Recursive sub-flow compatibility depth is unbounded** _(Flow Migration, §17.6)_
The formal definition permits arbitrary recursion depth through SubFlowStep references. Implementations may impose a practical depth limit. The flow reference DAG is acyclic (§11.5), so recursion terminates, but deeply nested sub-flow chains may be expensive to analyze.

**AL46 — ParallelStep compatibility requires all branches compatible** _(Flow Migration, §17.6)_
A ParallelStep is compatible only if ALL branches pass the compatibility check independently. Partial migration of parallel branches (some branches on v2, some on v1) is not supported. If any branch fails compatibility, the entire ParallelStep is incompatible.

**AL47 — Conservative data dependency analysis may produce false negatives** _(Flow Migration, §17.6)_
The required conservative analysis considers only the frozen snapshot and current entity states. It does not account for verdicts or state that v2 steps would produce during execution before the dependent step. This may reject migrations that are actually safe. Aggressive analysis with path dominance (§17.6.11) is optional and reduces false negatives at the cost of implementation complexity.

**AL48 — Entity parent changes require transitive analysis** _(Flow Migration, §17.6)_
Entity parent field changes are detected through Layer 2 (entity state) and Layer 3 (operation effects), but the full impact of a parent change on state propagation chains may require transitive analysis not specified in v1.0. Parent changes are conservatively treated as compatibility failures.

**AL49 — Reachable path computation uses v2's step graph** _(Flow Migration, §17.6)_
The compatibility analysis computes reachable paths from the current position using v2's step graph, not v1's. The v1 step graph is only used to identify the current position. This means routing changes at or after the current position are evaluated under v2's semantics. A step removed in v2 is simply not reachable; a step added in v2 may introduce new dependencies.

**AL50 — User-defined verdict precedence deferred to v2** _(Rule, §7)_
v1 contracts must use distinct VerdictType names for each verdict-producing rule (S8). User-defined verdict precedence, dominance relations, and contract-specified resolution strategies — which would allow multiple rules to produce the same VerdictType with an explicit conflict resolution mechanism — are deferred to a future version. Contracts requiring conditional same-verdict production should use distinct VerdictType names and a higher-stratum aggregation rule.

**AL51 — Single contract per discovery endpoint** _(Contract Discovery, §18)_
The `/.well-known/tenor` endpoint serves a single TenorManifest. Hosts exposing multiple independent contracts must use separate subdomains or path-scoped endpoints (served via the MAY clause in §18.3). A registry mechanism for multi-contract discovery is deferred to a future version.

**AL52 — Concurrent Operation isolation unspecified** _(Executor, §16)_
E3 requires atomicity for a single Operation's effect set but does not specify isolation semantics for concurrent invocation of multiple Operations against the same entity. Two Operations that concurrently read the same entity state, compute valid transitions, and write may produce conflicting final states. Executors must document their concurrency model. Serializable isolation is recommended but not required by v1.

**AL53 — Text equality uses byte-exact comparison** _(BaseType, §4)_
Text equality (`=`, `≠`) compares UTF-8 byte sequences exactly. Two strings that are canonically equivalent under Unicode normalization (e.g., NFC vs. NFD) but differ in byte representation compare as unequal. Contract authors are responsible for ensuring consistent normalization of Text fact values before FactSet assembly.

**AL54 — Sub-flow cross-version invocation unspecified** _(Flow Migration, §17.6)_
§17.6 covers in-flight flow migration for flows within a single contract version transition. The case where a sub-flow is defined in a different contract version than its parent flow — for example, a v1 parent flow invoking a sub-flow that has been independently updated to v2 — is not addressed. Sub-flow compatibility analysis assumes parent and sub-flow are migrated together as part of the same version transition.

**AL55 — Per-flow-type capability advertisement not supported in v1.1** _(Contract Discovery, §18)_
The `migration_analysis_mode` field in `ExecutorCapabilities` is per-executor. An executor uses one analysis mode for all flow migration decisions. Per-flow-type capability granularity (e.g., aggressive analysis for some flows, conservative for others) is deferred to v2.

---

## 20. Appendix C — Worked Example: Escrow Release Contract

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

### D.2.1 Persona Declarations

```
persona buyer
persona seller
persona compliance_officer
persona escrow_agent
```

---

### D.3 Facts

```
fact escrow_amount {
  type:   Money("USD")
  source: "escrow_service.current_balance"
}

fact delivery_status {
  type:   Enum(["pending", "confirmed", "failed"])
  source: "delivery_service.status"
}

fact line_items {
  type:   List(element_type: LineItemRecord, max: 100)
  source: "order_service.line_items"
}

fact compliance_threshold {
  type:    Money("USD")
  source:  "compliance_service.release_threshold"
  default: Money { amount: Decimal(10000.00), currency: "USD" }
}

fact buyer_requested_refund {
  type:    Bool
  source:  "buyer_portal.refund_requested"
  default: false
}
```

---

### D.4 Entities

```
entity EscrowAccount {
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

entity DeliveryRecord {
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
rule all_line_items_valid {
  stratum: 0
  when: ∀ item ∈ line_items . item.valid = true
  produce: verdict line_items_validated { payload: Bool = true }
}

rule delivery_confirmed {
  stratum: 0
  when: delivery_status = "confirmed"
  produce: verdict delivery_confirmed { payload: Bool = true }
}

rule delivery_failed {
  stratum: 0
  when: delivery_status = "failed"
  produce: verdict delivery_failed { payload: Bool = true }
}

rule amount_within_threshold {
  stratum: 0
  when: escrow_amount ≤ compliance_threshold
  produce: verdict within_threshold { payload: Bool = true }
}

rule refund_requested {
  stratum: 0
  when: buyer_requested_refund = true
  produce: verdict refund_requested { payload: Bool = true }
}
```

**Stratum 1 — Composite verdicts:**

```
rule can_release_without_compliance {
  stratum: 1
  when: verdict_present(line_items_validated)
      ∧ verdict_present(delivery_confirmed)
      ∧ verdict_present(within_threshold)
  produce: verdict release_approved { payload: Text = "auto" }
}

rule requires_compliance_review {
  stratum: 1
  when: verdict_present(line_items_validated)
      ∧ verdict_present(delivery_confirmed)
      ∧ ¬verdict_present(within_threshold)
  produce: verdict compliance_review_required { payload: Bool = true }
}

rule can_refund {
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
operation release_escrow {
  allowed_personas: [escrow_agent]
  precondition:     verdict_present(release_approved)
  effects:          [(EscrowAccount, held, released)]
  outcomes:         [released]
  error_contract:   [precondition_failed, persona_rejected]
}

operation release_escrow_with_compliance {
  allowed_personas: [compliance_officer]
  precondition:     verdict_present(compliance_review_required)
  effects:          [(EscrowAccount, held, released)]
  outcomes:         [released]
  error_contract:   [precondition_failed, persona_rejected]
}

operation refund_escrow {
  allowed_personas: [escrow_agent]
  precondition:     verdict_present(refund_approved)
  effects:          [(EscrowAccount, held, refunded)]
  outcomes:         [refunded]
  error_contract:   [precondition_failed, persona_rejected]
}

operation flag_dispute {
  allowed_personas: [buyer, seller]
  precondition:     verdict_present(delivery_confirmed)
                  ∨ verdict_present(delivery_failed)
  effects:          [(EscrowAccount, held, disputed)]
  outcomes:         [disputed]
  error_contract:   [precondition_failed, persona_rejected]
}

operation confirm_delivery {
  allowed_personas: [seller]
  precondition:     ∀ item ∈ line_items . item.valid = true
  effects:          [(DeliveryRecord, pending, confirmed)]
  outcomes:         [confirmed]
  error_contract:   [precondition_failed, persona_rejected]
}

operation record_delivery_failure {
  allowed_personas: [escrow_agent]
  precondition:     verdict_present(delivery_failed)
  effects:          [(DeliveryRecord, pending, failed)]
  outcomes:         [failed]
  error_contract:   [precondition_failed, persona_rejected]
}

// Compensation operation — used in failure recovery
operation revert_delivery_confirmation {
  allowed_personas: [escrow_agent]
  precondition:     verdict_present(delivery_confirmed)
  effects:          [(DeliveryRecord, confirmed, pending)]
  outcomes:         [reverted]
  error_contract:   [precondition_failed, persona_rejected]
}
```

---

### D.7 Flows

**Standard release flow:**

```
flow standard_release {
  snapshot: at_initiation
  entry:    step_confirm

  steps: {
    step_confirm: OperationStep {
      op:      confirm_delivery
      persona: seller
      outcomes: {
        confirmed: step_check_threshold
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
        released: Terminal(success)
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
        released: Terminal(success)
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
flow refund_flow {
  snapshot: at_initiation
  entry:    step_refund

  steps: {
    step_refund: OperationStep {
      op:      refund_escrow
      persona: escrow_agent
      outcomes: {
        refunded: Terminal(success)
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
Path 1: confirm_delivery (confirmed) → within_threshold (true) → release_escrow (released) → Terminal(success)
Path 2: confirm_delivery (confirmed) → within_threshold (false) → handoff → release_escrow_with_compliance (released) → Terminal(success)
Path 3: confirm_delivery (confirmed) → within_threshold (true) → release_escrow (failure) → revert_delivery_confirmation → Terminal(failure)
Path 4: confirm_delivery (confirmed) → within_threshold (false) → handoff → release_escrow_with_compliance (failure) → revert_delivery_confirmation → Terminal(failure)
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
  outcome: confirmed → step_check_threshold

step_check_threshold:
  NOTE: verdict_present(within_threshold) evaluated against FROZEN snapshot verdict set
        Entity state is now DeliveryRecord.confirmed — but verdicts are NOT recomputed
        within_threshold is present in frozen verdict set → condition true
  → step_auto_release

step_auto_release:
  persona escrow_agent ∈ release_escrow.allowed_personas → pass
  precondition: verdict_present(release_approved) → true (frozen verdict set)
  effects: (EscrowAccount, held, released) — executor validates held matches current state → apply
  outcome: released → Terminal(success)
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

---

## 21. Appendix D — Glossary

| Term | Definition |
|------|------------|
| **ADP** | [Adversarial Design Protocol](https://github.com/riverline-labs/iap). Design space mapping method used when the solution space is unknown and must be explored before formalization. Part of the [Consensus](https://github.com/riverline-labs/iap) protocol suite. |
| **Agent** | Any software component that reads a Tenor contract to understand a system's behavior. Agents discover contracts via the manifest (§18) and reason about the contract without reading implementation code. |
| **BaseType** | One of twelve primitive types in Tenor's type system: Bool, Int, Decimal, Money, Text, Date, DateTime, Duration, Enum, List, Record, TaggedUnion (§4). |
| **Bundle** | The top-level interchange document produced by the elaborator. Contains all constructs from a contract and its imports, serialized as canonical JSON (§13). |
| **CFFP** | [Constraint-First Formalization Protocol](https://github.com/riverline-labs/iap). The design method used for all Tenor construct additions: invariant declaration, candidate formalisms, pressure testing via counterexamples, canonical form selection. Part of the [Consensus](https://github.com/riverline-labs/iap) protocol suite. |
| **Cold-Start** | The sequence an agent follows from a bare URL to complete understanding of a system. Requires one fetch of the manifest at `/.well-known/tenor` (§18.4). |
| **Conformance Suite** | The set of test fixtures (`conformance/`) that validate elaborator behavior. Positive tests verify correct output; negative tests verify correct error reporting. |
| **Construct** | A top-level declaration in Tenor: Fact, Entity, Rule, Persona, Operation, or Flow (§3). |
| **Contract** | A complete Tenor specification of a system's behavior, comprising one or more `.tenor` source files that elaborate into a single interchange bundle. |
| **Dry-Run** | A read-only evaluation of an Operation that executes steps (1)-(3) of the execution sequence without applying effects. Responses carry `"simulation": true` (§18.6). |
| **Effect** | An entity state transition produced by an Operation. Effects are atomic — all effects of an Operation are applied together or none are (§9). |
| **Elaboration** | The six-pass transformation from `.tenor` source text to canonical JSON interchange: lex, parse, bundle, index, type-check, validate, serialize (§13). |
| **Elaborator** | The tool that performs elaboration. The reference elaborator is `tenor-core`. |
| **Entity** | A finite state machine representing a domain object. Declares states, an initial state, and permitted transitions (§6). |
| **Etag** | A SHA-256 hex digest of the canonical interchange bundle bytes. Used for change detection. Changes if and only if the bundle changes (§18.2). |
| **Executor** | A runtime system that evaluates Tenor contracts against live data. Subject to executor obligations E1-E14 (§16, §18.7). |
| **Fact** | A ground truth value sourced from an external system. Facts are inputs to the evaluation model — they are asserted, not derived (§5). |
| **FactSet** | The complete set of Fact values assembled for a single evaluation. Each Fact has exactly one value (asserted or default). |
| **Flow** | A directed acyclic graph of steps orchestrating Operations, with snapshot isolation and persona handoffs (§11). |
| **Frozen Verdict Semantics** | The guarantee that a Flow's verdict set is computed once at initiation and never recomputed mid-Flow. All predicate evaluations within a Flow use the snapshot (§11). |
| **Interchange Format** | The canonical JSON representation of a Tenor contract, produced by the elaborator. Defined by `docs/interchange-schema.json` (§13). |
| **Manifest** | A JSON document (TenorManifest) that wraps an interchange bundle with an etag and spec version. Served at `/.well-known/tenor` (§18.1). |
| **NumericModel** | The specification of arithmetic behavior: fixed-point decimal arithmetic with the bounds specified in §12.6, `MidpointNearestEven` rounding, no floating-point anywhere in the evaluation path (§12). |
| **Operation** | A persona-gated, precondition-guarded unit of work that produces entity state transitions. Declares allowed personas, preconditions, effects, outcomes, and error contracts (§9). |
| **Outcome** | A named result label declared on an Operation. The outcome set is finite, closed, and exhaustively handled in Flow routing (§9.1). |
| **Pass** | One stage of the six-pass elaboration pipeline. Passes are numbered 0-6: lex/parse (0), bundle (1), index (2), types (3), typecheck (4), validate (5), serialize (6) (§13). |
| **Persona** | A declared identity token representing an actor class. Pure identity with no metadata. Operations declare which Personas may invoke them (§8). |
| **Precondition** | A predicate expression on an Operation that must evaluate to true for the Operation to execute. Evaluated against the FactSet and frozen VerdictSet (§9). |
| **PredicateExpression** | A quantifier-free first-order logic formula over ground terms. The expression language for preconditions, rule conditions, and branch conditions (§10). |
| **Provenance** | The complete derivation chain for a verdict or operation result. Every verdict records which Facts and Rules produced it. Provenance is part of the evaluation relation, not a runtime feature (§14). |
| **RCP** | [Reconciliation Protocol](https://github.com/riverline-labs/iap). Verifies consistency across multiple protocol run outputs. Part of the [Consensus](https://github.com/riverline-labs/iap) protocol suite. |
| **ResolvedVerdictSet** | The set of all verdicts produced by evaluating all Rules against the current FactSet. Each verdict carries its payload and provenance. |
| **Rule** | A verdict-producing declaration with a `when` predicate and a `produce` clause. Rules are stratified — higher strata can reference verdicts from lower strata but not the same or higher (§7). |
| **Snapshot** | The frozen state captured at Flow initiation: the FactSet and ResolvedVerdictSet at that point in time. Immutable for the duration of the Flow (§11). |
| **Stratum** | The stratification level of a Rule. Rules at stratum N can only reference verdicts produced at strata < N. Guarantees termination (§7). |
| **TenorManifest** | The JSON document format for contract discovery: `{ bundle, etag, tenor }` with keys sorted lexicographically (§18.1). |
| **Transition** | A permitted state change in an Entity, expressed as a (from, to) pair (§6). |
| **TypeDecl** | A named type declaration (Record or TaggedUnion) that can be used as a Fact type or nested within other types (§4). |
| **TypeEnv** | The type environment built during Pass 3 of elaboration. Maps type names to their resolved definitions (§13). |
| **Verdict** | A derived value produced by a Rule. Verdicts have a VerdictType (label) and a typed payload. They are the outputs of the evaluation model (§7). |
| **VerdictType** | A named category of verdict. Declared implicitly by Rule `produce` clauses. Multiple Rules may produce the same VerdictType (§7). |
