# Elaborator Fix Plan — All 31 Failing Conformance Tests

Generated: 2026-02-20

## Overview

Running `cargo run -- run ../conformance` yields 8/39 passing.
The 31 failures fall into 6 categories:

| # | Category | Tests | Files |
|---|----------|-------|-------|
| A | Lexer error message | 1 | lexer.rs |
| B | Pass 1 import/dup error messages | 3 | elaborate.rs |
| C | Pass 2 duplicate-id messages | 2 | elaborate.rs |
| D | Pass 3 typedecl cycle | 1 | elaborate.rs |
| E | Pass 4 type-checking (missing) | 6 | elaborate.rs |
| F | Pass 5 validation messages/lines | 14 | elaborate.rs |
| G | Serialization (missing fields) | 2 | elaborate.rs |
| H | Parser: ParallelStep/SubFlowStep | 2 | parser.rs + elaborate.rs |
| I | Parser: payload Text bare type | 1 | parser.rs |

Total: 32 individual fixes covering 31 tests (rule_basic covers both G items).

---

## Category A — Lexer error message (1 test)

### Test: `negative/pass0/unterminated_string`

**Expected error:**
```json
{"pass":0,"construct_kind":null,"construct_id":null,"field":null,
 "file":"unterminated_string.tenor","line":7,"message":"unterminated string literal"}
```

**File:** `src/lexer.rs`

**Change:** Find the error produced when a newline is encountered inside a string and change the message from `"newline in string literal"` to `"unterminated string literal"`.

---

## Category B — Pass 1 import/dup-across-files error messages (3 tests)

### Test: `negative/pass1/missing_import`

**Expected:**
```json
{"pass":1,"construct_kind":null,"construct_id":null,"field":"import",
 "file":"missing_import.tenor","line":4,
 "message":"import resolution failed: file not found: nonexistent_module.tenor"}
```

**Change in elaborate.rs** (pass 1 import resolution):
- Error must set `field = Some("import".to_owned())`
- Message: `format!("import resolution failed: file not found: {}", filename)`
- `file` must be the importing file (the one that contains the `import` statement), line of the `import` statement
- Current code likely emits different text and omits `field`

### Test: `negative/pass1/import_cycle_a`

**Expected:**
```json
{"pass":1,"construct_kind":null,"construct_id":null,"field":"import",
 "file":"import_cycle_b.tenor","line":4,
 "message":"import cycle detected: import_cycle_a.tenor → import_cycle_b.tenor → import_cycle_a.tenor"}
```

**Change:**
- Error file must be the file that *closes* the cycle (import_cycle_b.tenor), at the line of its `import` statement
- `field = Some("import".to_owned())`
- Message: `format!("import cycle detected: {}", cycle_path_with_arrows)` where arrow is ` → `

### Test: `negative/pass1/dup_across_files_a`

**Expected:**
```json
{"pass":1,"construct_kind":"Fact","construct_id":"shared_id","field":"id",
 "file":"dup_across_files_b.tenor","line":4,
 "message":"duplicate Fact id 'shared_id': first declared in dup_across_files_a.tenor"}
```

**Change:**
- `pass = 1` (not 2 — cross-file dups detected during import resolution/bundle assembly)
- `construct_kind = Some("Fact".to_owned())`
- `construct_id = Some("shared_id".to_owned())`
- `field = Some("id".to_owned())`
- `file` = the second file (where the duplicate appears)
- Message: `format!("duplicate {} id '{}': first declared in {}", kind, id, first_file)`

---

## Category C — Pass 2 duplicate-id messages (2 tests)

### Test: `negative/pass2/duplicate_fact_id`

**Expected:**
```json
{"pass":2,"construct_kind":"Fact","construct_id":"foo","field":"id",
 "file":"duplicate_fact_id.tenor","line":12,
 "message":"duplicate Fact id 'foo': first declared at line 6"}
```

**Change in elaborate.rs** (pass 2 indexing):
- Message: `format!("duplicate {} id '{}': first declared at line {}", kind, id, first_line)`
- `construct_kind`, `construct_id`, `field` must all be set

### Test: `negative/pass2/duplicate_entity_id`

**Expected:**
```json
{"pass":2,"construct_kind":"Entity","construct_id":"Order","field":"id",
 "file":"duplicate_entity_id.tenor","line":10,
 "message":"duplicate Entity id 'Order': first declared at line 4"}
```

Same pattern as duplicate_fact_id but for Entity kind.

---

## Category D — Pass 3 TypeDecl cycle (1 test)

### Test: `negative/pass3/typedecl_mutual_cycle`

**Expected:**
```json
{"pass":3,"construct_kind":"TypeDecl","construct_id":"TypeB","field":"type.fields.ref",
 "file":"typedecl_mutual_cycle.tenor","line":15,
 "message":"TypeDecl cycle detected: TypeA → TypeB → TypeA"}
```

**Changes in elaborate.rs:**
- `construct_id` must be `"TypeB"` — the node that *closes* the cycle (currently reports TypeA, the entry point)
- `field = Some("type.fields.ref".to_owned())`
- `line` = 15 (the line of the TypeDecl that closes the cycle)
- Message: `format!("TypeDecl cycle detected: {}", cycle_with_arrows)` using ` → `
- Cycle path should start from the first node and end by returning to it (TypeA → TypeB → TypeA)

---

## Category E — Pass 4 type-checking (6 tests, currently missing)

Pass 4 currently does TypeRef resolution but no actual expression type-checking. All 6 tests below currently produce *no error* when they should fail.

### Test: `negative/pass4/unresolved_fact_ref`

**Expected:**
```json
{"pass":4,"construct_kind":"Rule","construct_id":"bad_ref","field":"body.when",
 "file":"unresolved_fact_ref.tenor","line":7,
 "message":"unresolved fact reference: 'nonexistent_fact' is not declared in this contract"}
```

**Implementation:** During Pass 4 type-checking of rule bodies, when a `fact_ref` is encountered, verify it names a declared Fact. If not:
- `pass = 4`, `construct_kind = "Rule"`, `construct_id = rule.id`
- `field = "body.when"`
- Message: `format!("unresolved fact reference: '{}' is not declared in this contract", name)`

### Test: `negative/pass4/bool_int_comparison`

**Expected:**
```json
{"pass":4,"construct_kind":"Rule","construct_id":"bad_comparison","field":"body.when",
 "file":"bool_int_comparison.tenor","line":13,
 "message":"type error: operator '>' not defined for Bool; Bool supports only = and ≠"}
```

**Implementation:** When type-checking a `Compare` expression, if the operand type is `Bool`, only `=` and `!=`/`≠` are permitted. Any other operator → type error.

### Test: `negative/pass4/cross_currency_compare`

**Expected:**
```json
{"pass":4,"construct_kind":"Rule","construct_id":"cross_currency","field":"body.when",
 "file":"cross_currency_compare.tenor","line":20,
 "message":"type error: cannot compare Money(currency: USD) with Money(currency: EUR); Money comparisons require identical currency codes"}
```

**Implementation:** When type-checking a `Compare` where both sides are Money typed, verify currency codes match. If not → type error with the format above.

### Test: `negative/pass4/quantifier_scalar_domain`

**Expected:**
```json
{"pass":4,"construct_kind":"Rule","construct_id":"bad_quantifier","field":"body.when",
 "file":"quantifier_scalar_domain.tenor","line":14,
 "message":"type error: quantifier domain 'flag' has type Bool; domain must be List-typed"}
```

**Implementation:** In a `Forall`/`Exists` expression, look up the domain fact's type. If it is not `List<_>` → type error.
- Message: `format!("type error: quantifier domain '{}' has type {}; domain must be List-typed", fact_name, type_display)`

### Test: `negative/pass4/var_var_in_predexpr`

**Expected:**
```json
{"pass":4,"construct_kind":"Rule","construct_id":"bad_var_var","field":"body.when",
 "file":"var_var_in_predexpr.tenor","line":20,
 "message":"type error: variable × variable multiplication is not permitted in PredicateExpression; only variable × literal_numeric is allowed"}
```

**Implementation:** In a `Multiply` or arithmetic expression inside a quantifier body, detect `var × var` (two `FieldRef` terms or two variable references). → type error with exact message above.

### Test: `negative/pass4/rule_body_var_var_range_exceeded`

**Expected:**
```json
{"pass":4,"construct_kind":"Rule","construct_id":"product_overflow","field":"body.produce.payload",
 "file":"rule_body_var_var_range_exceeded.tenor","line":26,
 "message":"type error: product range Int(min: 0, max: 10000) is not contained in declared verdict payload type Int(min: 0, max: 100)"}
```

**Implementation:** When the produce payload type is a bounded integer (e.g., `Int(min: 0, max: 100)`) and the computed value or expression result range can exceed that bound, report this error.
- `field = "body.produce.payload"`
- Message: `format!("type error: product range {} is not contained in declared verdict payload type {}", computed_range, declared_type)`

---

## Category F — Pass 5 validation messages and line numbers (14 tests)

### Test: `negative/pass5/entity_initial_not_in_states`

**Expected:**
```json
{"pass":5,"construct_kind":"Entity","construct_id":"Order","field":"initial",
 "file":"entity_initial_not_in_states.tenor","line":7,
 "message":"initial state 'draft' is not declared in states: [submitted, approved, rejected]"}
```

**Current message (approx):** `"initial state '{}' is not in states"` — missing states list
**Fix:** `format!("initial state '{}' is not declared in states: [{}]", initial, states.join(", "))`
**Line:** must be the line of the `initial:` field (line 7), not the `entity` keyword line.

### Test: `negative/pass5/entity_transition_unknown_endpoint`

**Expected:**
```json
{"pass":5,"construct_kind":"Entity","construct_id":"Order","field":"transitions",
 "file":"entity_transition_unknown_endpoint.tenor","line":12,
 "message":"transition endpoint 'cancelled' is not declared in states: [draft, submitted, approved]"}
```

**Fix:** `format!("transition endpoint '{}' is not declared in states: [{}]", endpoint, states.join(", "))`
**Line:** line of the offending transition (line 12).

### Test: `negative/pass5/entity_hierarchy_cycle`

**Expected:**
```json
{"pass":5,"construct_kind":"Entity","construct_id":"Beta","field":"parent",
 "file":"entity_hierarchy_cycle.tenor","line":14,
 "message":"entity hierarchy cycle detected: Beta → Alpha → Beta"}
```

**Fix:**
- `construct_id` = the entity that *closes* the cycle ("Beta")
- `field = Some("parent".to_owned())`
- `line` = line of the `parent:` field in "Beta" entity (line 14)
- Message: `format!("entity hierarchy cycle detected: {}", cycle_with_arrows)`

### Test: `negative/pass5/rule_negative_stratum`

**Expected:**
```json
{"pass":5,"construct_kind":"Rule","construct_id":"bad_stratum","field":"stratum",
 "file":"rule_negative_stratum.tenor","line":11,
 "message":"stratum must be a non-negative integer; got -1"}
```

**Fix:** Message = `format!("stratum must be a non-negative integer; got {}", stratum)`
**Line:** line of the `stratum:` field (line 11), not the `rule` keyword line.

### Test: `negative/pass5/rule_forward_stratum_ref`

**Expected:**
```json
{"pass":5,"construct_kind":"Rule","construct_id":"base_rule","field":"body.when",
 "file":"rule_forward_stratum_ref.tenor","line":17,
 "message":"stratum violation: rule 'base_rule' at stratum 0 references verdict 'high_verdict' produced by rule 'higher_rule' at stratum 1; verdict_refs must reference strata strictly less than the referencing rule's stratum"}
```

**Fix:** Update `validate_verdict_refs_in_expr` to emit exact message format above.
**Line:** line of the `verdict_present` reference (line 17), not the rule keyword.

### Test: `negative/pass5/operation_empty_personas`

**Expected:**
```json
{"pass":5,"construct_kind":"Operation","construct_id":"submit_order","field":"allowed_personas",
 "file":"operation_empty_personas.tenor","line":24,
 "message":"allowed_personas must be non-empty; an Operation with no allowed personas can never be invoked"}
```

**Fix:** Exact message as above. `field = "allowed_personas"`. Line = line of `allowed_personas:` field.

### Test: `negative/pass5/operation_effect_unknown_entity`

**Expected:**
```json
{"pass":5,"construct_kind":"Operation","construct_id":"bad_effect","field":"effects",
 "file":"operation_effect_unknown_entity.tenor","line":19,
 "message":"effect references undeclared entity 'NonexistentEntity'"}
```

**Fix:** Message = `format!("effect references undeclared entity '{}'", entity_id)`
**Line:** line of the offending `effect` entry (line 19).

### Test: `negative/pass5/operation_effect_unknown_transition`

**Expected:**
```json
{"pass":5,"construct_kind":"Operation","construct_id":"skip_to_approved","field":"effects",
 "file":"operation_effect_unknown_transition.tenor","line":30,
 "message":"effect (Order, draft, approved) is not a declared transition in entity Order; declared transitions are: [(draft, submitted), (submitted, approved)]"}
```

**Fix:** Message = `format!("effect ({}, {}, {}) is not a declared transition in entity {}; declared transitions are: [{}]", entity_id, from, to, entity_id, transitions_display)`
where transitions_display = `(from, to)` pairs joined by `, `

### Test: `negative/pass5/flow_missing_entry`

**Expected:**
```json
{"pass":5,"construct_kind":"Flow","construct_id":"bad_flow","field":"entry",
 "file":"flow_missing_entry.tenor","line":31,
 "message":"entry step 'nonexistent_step' is not declared in steps"}
```

**Fix:** Message = `format!("entry step '{}' is not declared in steps", entry_id)`
**Line:** line of the `entry:` field (line 31).

### Test: `negative/pass5/flow_unresolved_step_ref`

**Expected:**
```json
{"pass":5,"construct_kind":"Flow","construct_id":"bad_ref_flow","field":"steps.step_one.outcomes.success",
 "file":"flow_unresolved_step_ref.tenor","line":36,
 "message":"step reference 'step_two_does_not_exist' is not declared in steps"}
```

**Fix:**
- `field = "steps.<step_id>.outcomes.success"` (or `.outcomes.failure`, `.if_true`, `.next` etc.)
- Message = `format!("step reference '{}' is not declared in steps", target_id)`
- **Line:** line of the outcome/reference that contains the bad step id

### Test: `negative/pass5/flow_step_cycle`

**Expected:**
```json
{"pass":5,"construct_kind":"Flow","construct_id":"cyclic_flow","field":"steps",
 "file":"flow_step_cycle.tenor","line":23,
 "message":"flow step graph is not acyclic: cycle detected involving steps [step_a, step_b]"}
```

**Fix:** Message = `format!("flow step graph is not acyclic: cycle detected involving steps [{}]", cycle_steps.join(", "))`
**Line:** line of the first step in the detected cycle (line 23 = step_a's line).

### Test: `negative/pass5/flow_reference_cycle_a`

**Expected:**
```json
{"pass":5,"construct_kind":"Flow","construct_id":"flow_b","field":"steps.call_a.flow",
 "file":"flow_reference_cycle_b.tenor","line":9,
 "message":"flow reference cycle detected: flow_a → flow_b → flow_a"}
```

**Fix:**
- Error is reported in the file/line that *closes* the cycle (flow_reference_cycle_b.tenor, line 9)
- `construct_id = "flow_b"` (the flow that closes the cycle)
- `field = "steps.call_a.flow"` (the step that references flow_a)
- Message = `format!("flow reference cycle detected: {}", cycle_with_arrows)`

### Test: `negative/pass5/flow_missing_failure_handler`

**Expected:**
```json
{"pass":5,"construct_kind":"Flow","construct_id":"no_handler_flow","field":"steps.step_one.on_failure",
 "file":"flow_missing_failure_handler.tenor","line":35,
 "message":"OperationStep 'step_one' must declare a FailureHandler"}
```

**Fix:**
- `field = format!("steps.{}.on_failure", step_id)`
- Message = `format!("OperationStep '{}' must declare a FailureHandler", step_id)`
- **Line:** line of the step declaration (line 35).

### Test: `negative/pass5/unresolved_verdict_ref` (currently passes but double-check message)

Already passing — no change needed.

---

## Category G — Serialization missing fields (2 tests)

Both failures manifest as the serialized bundle missing fields that the expected JSON has.

### `comparison_type` in Compare serialization

**Affected tests:** `rule_basic`, `cross_file/bundle` (and any rule using Money/Int comparison)

In `serialize_expr` for `RawExpr::Compare`:
- When the comparison is NOT a Bool comparison, emit `"comparison_type"` key with the resolved type.
- The type to emit is the type of the left operand (or the common resolved type after promotion).

```rust
// In serialize_expr, Compare arm:
// After resolving types, if comparison_type != Bool:
if comparison_type != Type::Bool {
    m.insert("comparison_type".to_owned(), serialize_type(&comparison_type));
}
```

This requires tracking resolved types through Pass 4.

### `variable_type` in Forall serialization

**Affected test:** `rule_basic`

In `serialize_expr` for `RawExpr::Forall`:
- Emit `"variable_type"` = element type of the domain fact's List type.

```rust
// In serialize_expr, Forall arm:
// domain fact type is List<element_type>
// emit "variable_type": serialize_type(&element_type)
json!({
    "body": serialize_expr(body),
    "domain": {"fact_ref": domain},
    "quantifier": "forall",
    "variable": var,
    "variable_type": serialize_type(&element_type)
})
```

---

## Category H — Parser: ParallelStep and SubFlowStep (2 tests)

### Tests: `parallel/conflict_direct`, `parallel/conflict_transitive`

**Both tests fail because:**
1. Parser doesn't recognize `ParallelStep` or `SubFlowStep` step kinds → parse error at Pass 0
2. Even after parsing, elaborate.rs Pass 5 needs parallel branch conflict detection

#### H1. Parser changes (`src/parser.rs`)

Add to `parse_step()`:

**ParallelStep:**
```
par_step: ParallelStep {
  branches: [
    Branch {
      id:    <ident>
      entry: <ident>
      steps: { <step_id>: <StepKind> { ... }, ... }
    },
    ...
  ]
  join: JoinPolicy {
    on_all_success:  <outcome>
    on_any_failure:  <outcome>
    on_all_complete: null | <outcome>
  }
}
```

**SubFlowStep:**
```
call_sub: SubFlowStep {
  flow:       <ident>
  persona:    <ident>
  on_success: <outcome>
  on_failure: <outcome>
}
```

Add AST variants:
```rust
RawStep::Parallel {
    id: String,
    branches: Vec<RawBranch>,
    join: RawJoinPolicy,
    line: u32,
}

RawStep::SubFlow {
    id: String,
    flow: String,
    persona: String,
    on_success: RawOutcome,
    on_failure: RawOutcome,
    line: u32,
}

struct RawBranch {
    id: String,
    entry: String,
    steps: Vec<RawStep>,
}

struct RawJoinPolicy {
    on_all_success: Option<RawOutcome>,
    on_any_failure: Option<RawOutcome>,
    on_all_complete: Option<RawOutcome>,
}
```

#### H2. Elaborator changes (`src/elaborate.rs`)

**Serialization:** Serialize ParallelStep and SubFlowStep in `serialize_step()`.

**Parallel branch conflict detection (Pass 5):**

For `ParallelStep`, collect the set of entity IDs affected by each branch (directly via OperationStep effects, and transitively via SubFlowStep → referenced Flow's operations). If any two branches affect the same entity → error:

```json
{
  "pass": 5,
  "construct_kind": "Flow",
  "construct_id": "<flow_id>",
  "field": "steps.<par_step_id>.branches",
  "file": "<file>",
  "line": <line_of_par_step>,
  "message": "parallel branches '<branch_a>' and '<branch_b>' both declare effects on entity '<entity>'; parallel branch entity effect sets must be disjoint"
}
```

For transitive conflicts through SubFlowStep:
```
"message": "parallel branches '<branch_a>' and '<branch_b>' both affect entity '<entity>' (branch_a transitively through SubFlowStep → <flow_id> → <op_id>); parallel branch entity effect sets must be disjoint"
```

---

## Category I — Parser: bare `Text` type in payload context (1 test)

### Test: `positive/integration_escrow`

**File:** `integration_escrow.tenor`, line 106: `payload: Text = "auto"`

The parser handles bare `Text` as a type but the payload parsing context expects `payload: <type> = <value>` where `<type>` can be bare `Text`. The issue is that after parsing `Text` as the type, the `=` sign is either:
- Being parsed as part of a type expression (e.g., expecting `(` for `Text(max: N)`)
- Or the type-then-`=` sequence isn't handled in the payload field parser

**Fix:** In `parse_payload_field()` (or wherever operation/flow payload fields are parsed), after successfully parsing a type (even a parameterless one like bare `Text`), allow `=` as the value separator. Ensure `RawType::Text { max_length: 0 }` (unlimited) is the result for bare `Text`.

---

## Implementation Order

Implement in this order to maximize early test gains:

1. **Lexer fix** (A) — 1 line change, unblocks pass0 test
2. **Serialization fixes** (G) — unblocks rule_basic, cross_file/bundle
3. **Pass 2 message fixes** (C) — straightforward text changes
4. **Pass 3 typedecl cycle** (D) — fix construct_id and line reported
5. **Pass 1 import error fixes** (B) — requires tracing import path in elaborate.rs
6. **Pass 5 message fixes** (F) — 13 message/line adjustments, all in elaborate.rs
7. **Pass 4 type-checking** (E) — new logic, largest change block
8. **Parser: bare Text payload** (I) — parser.rs targeted fix
9. **Parser + elaborator: ParallelStep/SubFlowStep** (H) — largest new feature

---

## Files Modified

| File | Categories |
|------|-----------|
| `src/lexer.rs` | A |
| `src/parser.rs` | H, I |
| `src/elaborate.rs` | B, C, D, E, F, G, H |

No changes needed to `src/runner.rs`, `src/tap.rs`, or `src/main.rs`.
