---
phase: 03-cli-evaluator
plan: 02
subsystem: evaluator
tags: [rust_decimal, evaluation, stratified-rules, provenance, fixed-point-arithmetic]

# Dependency graph
requires:
  - phase: 02-foundation
    provides: "Cargo workspace, tenor-core library crate, interchange format"
provides:
  - "Runtime Value enum covering all spec BaseTypes"
  - "NumericModel with rust_decimal and MidpointNearestEven rounding"
  - "Contract deserialization from interchange JSON"
  - "FactSet assembly with type checking and default fallback"
  - "PredicateExpression evaluator (Compare, And, Or, Not, Forall, FieldRef, VerdictPresent, Mul)"
  - "Stratified rule evaluation with provenance tracking"
  - "Public evaluate(bundle, facts) API returning provenance-traced verdicts"
affects: [03-03-PLAN, 03-05-PLAN, 03-06-PLAN]

# Tech tracking
tech-stack:
  added: [rust_decimal 1.36]
  patterns: [tree-walker evaluation over interchange JSON, stratified evaluation, provenance collector]

key-files:
  created:
    - crates/eval/src/types.rs
    - crates/eval/src/numeric.rs
    - crates/eval/src/provenance.rs
    - crates/eval/src/assemble.rs
    - crates/eval/src/predicate.rs
    - crates/eval/src/rules.rs
  modified:
    - crates/eval/Cargo.toml
    - crates/eval/src/lib.rs
    - Cargo.toml

key-decisions:
  - "Evaluator types are DISTINCT from tenor-core AST types -- evaluator consumes interchange JSON, not raw DSL"
  - "All numeric arithmetic uses rust_decimal::Decimal with MidpointNearestEven rounding -- no f64 in evaluation paths"
  - "Predicate expressions parsed directly from interchange JSON into evaluator's own Predicate enum"
  - "ProvenanceCollector passed through eval_pred to track all fact/verdict references"
  - "Short-circuit evaluation for And/Or (left-to-right, stop early on false/true)"

patterns-established:
  - "Tree-walker evaluation: recursive eval_pred over Predicate enum nodes"
  - "Type-directed comparison: comparison_type field drives Int-to-Decimal promotion"
  - "Stratum loop: evaluate rules at stratum 0, then 1, etc., accumulating verdicts"
  - "ProvenanceCollector: mutable collector threaded through evaluation for provenance chain construction"

requirements-completed: [EVAL-01, EVAL-04]

# Metrics
duration: 14min
completed: 2026-02-21
---

# Phase 3 Plan 02: Evaluator Core Summary

**Stratified rule evaluator with fixed-point arithmetic, fact assembly, predicate evaluation, and provenance-traced verdicts using rust_decimal**

## Performance

- **Duration:** 14 min
- **Started:** 2026-02-21T21:19:36Z
- **Completed:** 2026-02-21T21:33:44Z
- **Tasks:** 3
- **Files modified:** 9

## Accomplishments
- Runtime Value type covering all 12 spec BaseTypes with equality and type-safe operations
- NumericModel using rust_decimal exclusively with MidpointNearestEven rounding, Int-to-Decimal promotion, overflow detection
- Contract deserialization from interchange JSON (all construct kinds: Fact, Entity, Rule, Operation, Flow, Persona)
- FactSet assembly implementing spec Section 5.2 semantics (type checking, defaults, missing fact errors)
- PredicateExpression evaluator handling all node types (Compare, And, Or, Not, FactRef, FieldRef, Literal, VerdictPresent, Forall, Mul)
- Stratified rule evaluation following spec Section 7.4 with correct stratum ordering
- Provenance tracking recording facts_used and verdicts_used per verdict
- Public evaluate(bundle, facts) API for end-to-end rules-only evaluation
- 79 unit and integration tests, zero f64 in evaluation code

## Task Commits

Each task was committed atomically:

1. **Task 1: Define evaluator types and numeric model** - `ffdb82a` (feat)
2. **Task 2: Implement fact assembly and predicate evaluation** - `a088614` (feat)
3. **Task 3: Implement stratified rule evaluation** - `b7ebc57` (feat)

## Files Created/Modified
- `Cargo.toml` - Added rust_decimal to workspace dependencies
- `crates/eval/Cargo.toml` - Added serde, serde_json, rust_decimal dependencies
- `crates/eval/src/types.rs` - Value enum, TypeSpec, Contract, FactSet, VerdictSet, Predicate, EvalError, Contract::from_interchange(), predicate parsing
- `crates/eval/src/numeric.rs` - promote_int_to_decimal, eval_mul, eval_int_mul, compare_values with type-directed promotion
- `crates/eval/src/provenance.rs` - VerdictProvenance, ProvenanceCollector
- `crates/eval/src/assemble.rs` - assemble_facts with type checking, default fallback, validation
- `crates/eval/src/predicate.rs` - eval_pred recursive evaluator with EvalContext for bound variables
- `crates/eval/src/rules.rs` - eval_strata stratified evaluation, eval_rule, eval_payload
- `crates/eval/src/lib.rs` - Module declarations, public evaluate() API, EvalResult type, integration tests

## Decisions Made
- Evaluator types are completely separate from tenor-core AST types (no import of RawConstruct etc.)
- Used Decimal::from_str in tests rather than adding rust_decimal_macros as a dev-dependency
- Predicate expressions parsed from interchange JSON into evaluator's own Predicate enum (not tenor-core's RawExpr)
- Facts JSON format: flat object with fact IDs as keys, values using interchange type encoding (Money as {amount, currency}, etc.)
- Short-circuit evaluation for And/Or operators (matches typical predicate logic semantics)
- Forall quantification uses cloned EvalContext with bound variable inserted per iteration

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] rust_decimal_macros not available**
- **Found during:** Task 1 (unit tests)
- **Issue:** Tests used `dec!()` macro from `rust_decimal_macros` crate which was not a dependency
- **Fix:** Replaced `dec!()` macro with helper function using `Decimal::from_str()` -- avoids extra dependency
- **Files modified:** crates/eval/src/types.rs, crates/eval/src/numeric.rs
- **Verification:** All tests compile and pass
- **Committed in:** ffdb82a (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Minor build fix, no scope change.

## Issues Encountered
None beyond the macro dependency fix above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Evaluator core ready for Operation and Flow execution (plan 03-03)
- Public evaluate() API can be wired to CLI eval subcommand (plan 03-05)
- Evaluator conformance fixtures can be built against evaluate() (plan 03-06)
- Operations and Flows are parsed but not yet executed -- deferred to plan 03-03

## Self-Check: PASSED

All 9 files verified present. All 3 task commits verified in git log.

---
*Phase: 03-cli-evaluator*
*Completed: 2026-02-21*
