# Phase 5: Domain Validation -- Spec Gap Report

**Synthesized from 5 domain contracts authored during plans 05-01 through 05-05**

---

## 1. Executive Summary

| Metric | Result |
|--------|--------|
| Contracts authored | 5/5 |
| Elaborated successfully | 5/5 |
| Passed `tenor check` | 5/5 (1 informational finding, 0 errors/warnings) |
| Evaluated correctly | 5/5 (12 eval fixture sets, all passing) |
| Total gaps found | **13** (3 BLOCKER, 7 FRICTION, 1 COSMETIC, 2 BLOCKER-fixed) |
| Evaluator bugs fixed inline | 3 (GAP-001 FieldRef, GAP-011 int_literal, GAP-012 Money literal) |

### Overall Assessment

**The spec is ready for code generation (Phase 6).** All five domain contracts -- spanning SaaS, healthcare, supply chain, energy procurement, and trade finance -- are expressible in Tenor and evaluate correctly end-to-end. No scenario was completely inexpressible; every gap had a viable workaround. The three evaluator bugs discovered were implementation defects (not spec gaps) and were fixed inline during validation.

The primary friction point -- effect-to-outcome mapping syntax for multi-outcome operations (GAP-006/010/013) -- was encountered independently in three domains. This is the single highest-priority improvement for a future spec revision but is not a code generation blocker because BranchStep routing provides a clean workaround.

---

## 2. Domain Coverage Summary

### 2.1 Contracts Overview

| Domain | Size | Lines | Personas | Entities | Rules | Strata | Operations | Flows | Eval Paths |
|--------|------|-------|----------|----------|-------|--------|------------|-------|------------|
| SaaS Subscription | SMALL | 175 | 3 | 1 | 6 | 2 | 4 | 1 | 2 |
| Healthcare Prior Auth | LARGE | 465 | 6 | 2 | 17 | 4 | 8 | 2 | 3 |
| Supply Chain Inspection | MEDIUM | 230 | 4 | 3 | 7 | 2 | 6 | 1 | 2 |
| Energy Procurement RFP | MED-LARGE | 361 | 5 | 2 | 12 | 3 | 7 | 1 | 3 |
| Trade Finance LC | MEDIUM | 230 | 5 | 2 | 8 | 2 | 5 | 1 | 2 |

**Totals:** 1,461 lines, 23 personas, 10 entities, 50 rules, 30 operations, 6 flows, 12 eval fixture paths

### 2.2 Spec Feature Coverage Matrix

| Feature | SaaS | Healthcare | Supply Chain | Energy | Trade Finance |
|---------|:----:|:----------:|:------------:|:------:|:-------------:|
| **Construct Types** | | | | | |
| TypeDecl (Record) | X | X | X | X | X |
| TypeDecl (Enum) -- via fact | X | X | X | X | X |
| Persona | X | X | X | X | X |
| Fact (Bool) | X | X | -- | X | X |
| Fact (Bool with default) | X | X | -- | X | X |
| Fact (Int) | X | X | X | X | -- |
| Fact (Int with default) | -- | -- | X | X | -- |
| Fact (Text) | -- | X | -- | -- | -- |
| Fact (Enum) | X | X | X | X | X |
| Fact (Enum with default) | -- | -- | -- | -- | X |
| Fact (Date) | -- | X | -- | X | X |
| Fact (Money) | -- | -- | -- | X | X |
| Fact (Record-typed) | X | X | X | X | -- |
| Fact (List) | -- | X | X | X | X |
| Entity | X | X | X | X | X |
| Rule (stratum 0) | X | X | X | X | X |
| Rule (stratum 1+) | X | X | X | X | X |
| Rule (stratum 2+) | -- | X | -- | X | -- |
| Rule (stratum 3+) | -- | X | -- | -- | -- |
| Operation | X | X | X | X | X |
| Flow | X | X | X | X | X |
| **Rule Predicates** | | | | | |
| Comparison (=, !=, <, >, <=, >=) | X | X | X | X | X |
| Logical AND | X | X | X | X | X |
| Logical OR | -- | X | X | X | X |
| Logical NOT | -- | X | X | X | X |
| verdict_present() | X | X | X | X | X |
| Negated verdict_present() | -- | X | X | X | X |
| FieldRef (Record field access) | X | X | X | X | -- |
| Bounded quantification (forall) | -- | X | X | X | X |
| Money literal comparison | -- | -- | -- | X | X |
| Date comparison | -- | -- | -- | X | X |
| **Operation Features** | | | | | |
| allowed_personas | X | X | X | X | X |
| precondition | X | X | X | X | X |
| effects (single entity) | X | X | X | X | X |
| effects (multi-entity) | -- | X | X | X | X |
| error_contract | X | X | X | X | X |
| Multi-outcome operation | -- | -- | -- | -- | -- |
| **Flow Step Types** | | | | | |
| OperationStep | X | X | X | X | X |
| BranchStep | X | X | X | X | X |
| ParallelStep | -- | -- | X | -- | -- |
| SubFlowStep | -- | X | -- | -- | -- |
| HandoffStep | -- | X | -- | -- | X |
| Terminal | X | X | X | X | X |
| **Failure Handlers** | | | | | |
| Terminate | X | X | X | X | X |
| Escalate | -- | X | -- | X | -- |
| Compensate | -- | -- | X | -- | -- |
| **Structural Features** | | | | | |
| Multi-file import | -- | -- | X | X | -- |
| JoinPolicy (on_all_success) | -- | -- | X | -- | -- |
| JoinPolicy (on_any_failure) | -- | -- | X | -- | -- |
| Multiple flows per contract | -- | X | -- | -- | -- |

### 2.3 Coverage Statistics

- **Tenor construct kinds used:** TypeDecl, Persona, Fact, Entity, Rule, Operation, Flow -- **7/7 (100%)**
- **Fact type variants used:** Bool, Int, Text, Enum, Date, Money, Record-typed, List -- **8/8 (100%)**
- **Flow step types used:** OperationStep, BranchStep, ParallelStep, SubFlowStep, HandoffStep, Terminal -- **6/6 (100%)**
- **Failure handlers used:** Terminate, Escalate, Compensate -- **3/3 (100%)**
- **Features exercised by 3+ domains:** TypeDecl Record, Persona, Entity, Rule, Operation, Flow, OperationStep, BranchStep, Terminal, Terminate, verdict_present(), Logical AND, Comparison operators
- **Features exercised by only 1 domain:** ParallelStep (Supply Chain), SubFlowStep (Healthcare), Compensate (Supply Chain), JoinPolicy (Supply Chain), Multiple flows (Healthcare)
- **Features never exercised:** Multi-outcome operations with effect-to-outcome mapping (DSL syntax does not exist -- see GAP-006/010/013)

---

## 3. Gaps by Severity

### 3.1 BLOCKER (Auto-fixed -- Evaluator Bugs)

These were implementation defects in the evaluator, not spec gaps. All three were fixed inline during domain contract authoring.

#### GAP-001: FieldRef on Record-typed facts not resolved by evaluator

- **Domain:** SaaS
- **Scenario:** Rule `seats_within_limit` compares `current_seat_count <= plan_features.max_seats` where `plan_features` is a Record-typed Fact
- **Attempted:** Standard field access syntax `plan_features.max_seats` in rule predicate
- **Result:** Evaluator's `FieldRef` resolution only checked `ctx.bindings` (loop variables from bounded quantifiers), not the `facts` FactSet. Record-typed facts accessed via field ref produced `UnboundVariable` error at eval time despite correct elaboration.
- **Severity:** BLOCKER (fixed inline -- Rule 1 deviation)
- **Fix Applied:** Modified `eval_pred` in `crates/eval/src/predicate.rs` to fall back to `facts.get(var)` when `ctx.bindings.get(var)` returns None. Also records the fact in provenance when resolved from facts.
- **Suggested Spec Clarification:** None needed -- the spec is clear that Record field access should work on facts. This was purely an evaluator implementation gap.

#### GAP-011: Missing int_literal handling in evaluator default value parser

- **Domain:** Energy Procurement
- **Scenario:** Fact `minimum_score_threshold` with default value `180` (Int type)
- **Attempted:** Evaluating a contract with Int-typed facts that have default values
- **Result:** Evaluator crashed with "expected integer" error. The elaborator serializes Int defaults as `{"kind": "int_literal", "value": 180}` but `parse_default_value()` only handled `bool_literal`, `decimal_value`, and `money_value`.
- **Severity:** BLOCKER (fixed inline -- Rule 1 deviation)
- **Fix Applied:** Added `"int_literal"` case to `parse_default_value()` in `crates/eval/src/types.rs`

#### GAP-012: Money literal parsing fails for interchange format in rule conditions

- **Domain:** Energy Procurement
- **Scenario:** Rule conditions comparing `rfp_amount` against Money literals like `Money { amount: "50000.00", currency: "USD" }`
- **Attempted:** Evaluating rules with Money comparison predicates
- **Result:** Evaluator crashed with "Money value missing 'amount' string". The interchange serializes Money literals as `{"amount": {"kind": "decimal_value", "value": "50000.00", ...}, "currency": "USD"}` -- the `amount` field is a structured object, not a plain string. But `parse_plain_value()` for Money called `v.get("amount").and_then(|a| a.as_str())` which fails on the structured object.
- **Severity:** BLOCKER (fixed inline -- Rule 1 deviation)
- **Fix Applied:** Updated Money parsing in `parse_plain_value()` to handle both plain string format (facts) and structured decimal_value format (interchange literals)

### 3.2 FRICTION (Workarounds Exist)

#### GAP-002: Entity state constraint in flow design

- **Domain:** SaaS
- **Scenario:** Wanted to test both activation (trial -> active) and suspension (active -> suspended) paths through the same flow
- **Attempted:** Single flow with a branch: payment ok -> activate, payment failed -> suspend
- **Result:** Entity always starts at initial state (`trial`). A branch-first flow routing to `suspend_subscription` (which expects `active -> suspended`) would fail because the entity is still in `trial` state.
- **Severity:** FRICTION
- **Workaround:** Flow redesigned to first activate (trial -> active), then branch on payment for the suspend path. This is actually more realistic.
- **Suggested Fix:** Not needed -- this is inherent to the closed-world entity model. Entities always start at their declared initial state. Document the pattern: always design flows with the entity initial state in mind.

#### GAP-003: No bounded existential quantification (exists / there-exists)

- **Domain:** Supply Chain
- **Scenario:** Checking if any item in a cargo inspection list has a defect (e.g., `exists item in inspection_items: item.compliant = false`)
- **Attempted:** Bounded existential quantification `exists item in inspection_items`, but the lexer/parser only supports `forall` (universal quantification)
- **Result:** Cannot express "there exists an item with property X" directly.
- **Severity:** FRICTION
- **Workaround:** Use negated universal via De Morgan's law: `not (forall item in list . item.prop = true)` to express "exists item where prop is not true"
- **Suggested Fix:** Add `exists` / U+2203 as a lexer token and parser production for bounded existential quantification. Implementation: parse like `forall` but serialize with `"quantifier": "exists"` in interchange; evaluator short-circuits on first match.

#### GAP-004: Parallel branch disjoint entity constraint prevents natural entity hierarchy modeling

- **Domain:** Supply Chain
- **Scenario:** Modeling concurrent quality and compliance inspections on the same InspectionLot entity
- **Attempted:** Two parallel branches both operating on InspectionLot
- **Result:** Pass 5 validation error: "parallel branches must have disjoint entity effect sets". Correct per spec Section 11.5.
- **Severity:** FRICTION
- **Workaround:** Split InspectionLot into QualityLot and ComplianceLot with identical state machines.
- **Suggested Fix:** Not needed -- the constraint is sound and prevents concurrent state conflicts. Document the pattern: when parallel branches need to independently track progress on the same logical concept, model as separate entity types (one per branch).

#### GAP-005: No TaggedUnion type construct in DSL

- **Domain:** Healthcare (also encountered in Trade Finance)
- **Scenario:** Modeling denial reasons as a discriminated union where each variant could carry variant-specific payload data
- **Attempted:** TypeDecl (TaggedUnion) for DenialReason
- **Result:** The AST and parser only support TypeDecl as Record (struct with named fields) and Enum (flat string set). No TaggedUnion variant.
- **Severity:** FRICTION
- **Workaround:** Use Enum for simple discrimination, separate Record facts for variant payloads
- **Suggested Fix:** Consider adding TaggedUnion as a TypeDecl variant in a future spec revision. For v1.0, Enum covers the common case. Document the Enum + separate Record pattern as idiomatic Tenor.

#### GAP-006: No effect-to-outcome mapping syntax in DSL

- **Domain:** Healthcare (also independently hit in Energy as GAP-013 and Trade Finance as GAP-010)
- **Scenario:** Modeling decide_auth as a multi-outcome operation where the outcome (approved/denied) is determined by which entity state transition fires
- **Attempted:** Multi-outcome operation with effect-to-outcome mapping
- **Result:** The spec describes multi-outcome operations with effect-to-outcome mapping, and the evaluator supports it. However, the DSL parser only supports `effects: [(Entity, from, to)]` -- there is no syntax for attaching an outcome label to an individual effect tuple.
- **Severity:** FRICTION (the single most impactful gap -- hit independently in 3 of 5 domains)
- **Workaround:** Use separate single-outcome operations with flow-level BranchStep routing instead of one multi-outcome operation
- **Suggested Fix:** Add optional outcome label to effect tuples in DSL syntax: `effects: [(Entity, from, to) => outcome_label]`. The evaluator already handles this via the `outcome` field on Effect structs; only the parser needs the syntax.

#### GAP-008: No flow-level eval in CLI (only rule eval)

- **Domain:** Healthcare
- **Scenario:** Wanted to run `tenor eval --flow auth_review_flow --persona requesting_physician` to test flow execution from the CLI
- **Attempted:** CLI-based flow evaluation
- **Result:** The `tenor eval` CLI subcommand only evaluates rules. Flow evaluation is only available through the Rust API `tenor_eval::evaluate_flow()`.
- **Severity:** FRICTION
- **Workaround:** Construct expected verdict files manually and verify through Rust conformance tests
- **Suggested Fix:** Add `--flow <flow_id>` and `--persona <persona>` flags to `tenor eval` CLI. This is a CLI ergonomics improvement, not a spec issue.

#### GAP-009: Frozen snapshot prevents sequential fact-based preconditions in flows

- **Domain:** Trade Finance
- **Scenario:** Sequential operations where each operation's precondition depends on a state change from the previous operation (e.g., `lc_status = "issued"` for step 1, then `lc_status = "presented"` for step 2)
- **Attempted:** Using Enum fact `lc_status` as preconditions that evolve through the flow
- **Result:** Snapshot is frozen at initiation. The fact `lc_status` never changes during flow execution -- it remains "issued" throughout.
- **Severity:** FRICTION
- **Workaround:** Use `verdict_present()` preconditions or simple fact checks that are all satisfiable from the initial snapshot. Entity state evolution is handled by effects, not fact mutation.
- **Suggested Fix:** Not needed -- this is inherent to the frozen snapshot model (spec Section 11.2). Document the pattern: operation preconditions in flows should use verdict_present() or initial-snapshot-compatible fact checks.

#### GAP-010: Multi-outcome operations require effect-to-outcome mapping absent from DSL (duplicate of GAP-006)

- **Domain:** Trade Finance
- **Scenario:** Modeling `examine_documents` with outcomes `[accept, reject]`
- **Attempted:** Declared operation with multiple outcomes
- **Result:** Same issue as GAP-006. Evaluator requires effects to carry an `outcome` field, but DSL parser and serializer do not emit it.
- **Severity:** FRICTION (reinforces GAP-006)
- **Suggested Fix:** Same as GAP-006

#### GAP-013: Multi-outcome operations with conflicting entity effects (duplicate of GAP-006)

- **Domain:** Energy Procurement
- **Scenario:** `award_contract` operation with outcomes `[award, reject]` where effects move the same entity to different target states
- **Attempted:** `effects: [(RFP, shortlisted, awarded), (PurchaseOrder, pending, approved), (RFP, shortlisted, cancelled)]`
- **Result:** At eval time the evaluator applies ALL effects sequentially. After the first effect moves RFP to `awarded`, the third tries `(RFP, shortlisted, cancelled)` but finds RFP in `awarded` state.
- **Severity:** FRICTION (reinforces GAP-006 with a more severe variant -- conflicting effects on the same entity)
- **Workaround:** Split into separate `award_rfp` and `reject_rfp` operations
- **Suggested Fix:** Same as GAP-006. This variant makes effect-to-outcome mapping even more critical because without it, conflicting effects on the same entity are unresolvable.

### 3.3 COSMETIC

#### GAP-007: Static analysis reports escalation targets as unreachable steps

- **Domain:** Healthcare
- **Scenario:** Flow auth_review_flow has an Escalate handler on step_deny that routes to step_director_review. S6 reports 3 steps as unreachable.
- **Attempted:** Steps reachable through Escalate failure handler path
- **Result:** Informational finding `[s6/INFO]: Flow 'auth_review_flow' has 3 unreachable step(s)`. Not a blocker but could confuse contract authors.
- **Severity:** COSMETIC
- **Suggested Fix:** Enhance S6 path analysis to trace Escalate handler targets in addition to normal outcome-based routing. This would eliminate false positive unreachable step reports for escalation paths.

---

## 4. Skipped Scenarios

No scenarios were fully skipped during domain validation. Every planned domain feature was either expressed directly or through a documented workaround. However, several scenarios were **redesigned** due to known limitations:

### 4.1 Multi-outcome Operations (3 domains affected)

- **What was attempted:** Operations with multiple possible outcomes (approve/deny, accept/reject, award/reject) where different effects fire based on which outcome occurs
- **Why the language couldn't express it:** The DSL parser has no syntax for mapping effects to specific outcomes within a multi-outcome operation. The spec describes the concept, the evaluator supports it, but the DSL-to-interchange pipeline does not emit effect-to-outcome associations.
- **What was done instead:** Split each multi-outcome operation into separate single-outcome operations; routing handled by BranchStep at the flow level
- **Spec change needed:** Add outcome label syntax to effect tuples (parser change only -- evaluator and interchange schema already support it)
- **v1.0 blocker?** No -- BranchStep workaround is clean and arguably more explicit

### 4.2 TaggedUnion Types (2 domains affected)

- **What was attempted:** Discriminated union types (e.g., DenialReason with variant-specific payloads)
- **Why the language couldn't express it:** TypeDecl only supports Record and Enum. No TaggedUnion variant exists in the AST/parser.
- **What was done instead:** Used Enum for simple variant selection; separate Record facts for variant-specific data
- **Spec change needed:** New TypeDecl variant for TaggedUnion (AST, parser, type checker, serializer, evaluator all need changes)
- **v1.0 blocker?** No -- the Enum + Record pattern covers all cases encountered. TaggedUnion is a v2.0 convenience improvement.

### 4.3 Bounded Existential Quantification (1 domain affected)

- **What was attempted:** `exists item in inspection_items : item.compliant = false`
- **Why the language couldn't express it:** The lexer/parser only recognizes `forall` / U+2200 for bounded quantification
- **What was done instead:** Used universal quantification (forall) with De Morgan negation
- **Spec change needed:** Add `exists` / U+2203 token and parser production
- **v1.0 blocker?** No -- De Morgan workaround is semantically equivalent

---

## 5. Recommendations

### 5.1 Before Phase 6 (Code Generation) -- No Blockers

**No spec or toolchain changes are required before Phase 6.** All five domain contracts elaborate, pass static analysis, and evaluate correctly. The workarounds used are idiomatic and well-documented.

### 5.2 Recommended Improvements for v1.x (Post-Code-Generation)

These should be addressed in a future iteration cycle, prioritized by impact:

| Priority | Gap | Change | Impact |
|----------|-----|--------|--------|
| **HIGH** | GAP-006/010/013 | Add effect-to-outcome mapping syntax in DSL parser | 3 of 5 domains hit this; parser-only change (evaluator already supports it) |
| **MEDIUM** | GAP-003 | Add `exists` bounded quantifier | 1 domain hit this; small lexer/parser/evaluator change |
| **MEDIUM** | GAP-008 | Add `--flow` and `--persona` flags to `tenor eval` CLI | CLI ergonomics; no spec change needed |
| **LOW** | GAP-007 | Enhance S6 path analysis to follow Escalate handler targets | Eliminates false positive unreachable step reports |

### 5.3 Acceptable for v1.0 (No Change Needed)

| Gap | Reason |
|-----|--------|
| GAP-002 (entity initial state constraint) | Inherent to closed-world model; flow redesign is more realistic |
| GAP-004 (parallel disjoint entities) | Sound constraint preventing concurrent state conflicts; entity splitting is good modeling practice |
| GAP-005 (no TaggedUnion) | Enum + Record pattern covers all encountered cases; TaggedUnion is v2.0 convenience |
| GAP-009 (frozen snapshot) | Inherent to spec Section 11.2 design; verdict-based preconditions are the correct pattern |

### 5.4 Tracked for v2.0

| Feature | Rationale |
|---------|-----------|
| TaggedUnion TypeDecl (GAP-005) | Nice-to-have for domain modeling ergonomics but not required for correctness |
| Mutable snapshot model | Would eliminate GAP-009 but fundamentally changes Flow evaluation semantics |
| Multi-outcome operation DSL syntax | Should be resolved in v1.x, but if deferred, track for v2.0 |

### 5.5 Iterative Fix-and-Retest Cycle

**Not required before Phase 6.** All contracts are working as-is. When the v1.x improvements listed in Section 5.2 are implemented, the domain contracts should be updated to exercise the new features (especially multi-outcome operations, which would simplify all 3 affected contracts).

---

## 6. Feature Coverage Statistics

### 6.1 Construct-Level Coverage

| Category | Total | Exercised | Coverage |
|----------|-------|-----------|----------|
| Top-level construct kinds (TypeDecl, Persona, Fact, Entity, Rule, Operation, Flow) | 7 | 7 | 100% |
| Fact type variants (Bool, Int, Text, Enum, Date, Money, Record, List) | 8 | 8 | 100% |
| Flow step types (OperationStep, BranchStep, ParallelStep, SubFlowStep, HandoffStep, Terminal) | 6 | 6 | 100% |
| Failure handlers (Terminate, Escalate, Compensate) | 3 | 3 | 100% |
| Predicate operators (=, !=, <, >, <=, >=, AND, OR, NOT, verdict_present, FieldRef, forall, Money literal, Date comparison) | 14 | 14 | 100% |
| Structural features (multi-file import, JoinPolicy, multiple flows) | 3 | 3 | 100% |

### 6.2 Cross-Domain Exercise Depth

| Times Exercised | Features |
|-----------------|----------|
| All 5 domains | TypeDecl Record, Persona, Entity, Rule, Operation, Flow, OperationStep, BranchStep, Terminal, Terminate, verdict_present, AND, basic comparisons, error_contract |
| 3-4 domains | Logical OR, Logical NOT, negated verdict_present, FieldRef, bounded quantification (forall), multi-entity effects, Fact with default |
| 2 domains | Money literal comparison, Date comparison, HandoffStep, Escalate, multi-file import |
| 1 domain only | ParallelStep, SubFlowStep, Compensate, JoinPolicy, multiple flows per contract |

### 6.3 Never Exercised

- **Multi-outcome operations with effect-to-outcome mapping**: DSL syntax does not exist (GAP-006/010/013). The evaluator and interchange schema support it, but no contract can exercise it until the parser adds the syntax.

---

## Appendix: Gap-to-Domain Traceability

| Gap ID | Domain(s) | Severity | Status |
|--------|-----------|----------|--------|
| GAP-001 | SaaS | BLOCKER | **Fixed** (evaluator bug) |
| GAP-002 | SaaS | FRICTION | Accepted (inherent to model) |
| GAP-003 | Supply Chain | FRICTION | v1.x improvement |
| GAP-004 | Supply Chain | FRICTION | Accepted (sound constraint) |
| GAP-005 | Healthcare, Trade Finance | FRICTION | v2.0 |
| GAP-006 | Healthcare | FRICTION | v1.x (HIGH priority) |
| GAP-007 | Healthcare | COSMETIC | v1.x (LOW priority) |
| GAP-008 | Healthcare | FRICTION | v1.x (MEDIUM priority) |
| GAP-009 | Trade Finance | FRICTION | Accepted (inherent to model) |
| GAP-010 | Trade Finance | FRICTION | Duplicate of GAP-006 |
| GAP-011 | Energy Procurement | BLOCKER | **Fixed** (evaluator bug) |
| GAP-012 | Energy Procurement | BLOCKER | **Fixed** (evaluator bug) |
| GAP-013 | Energy Procurement | FRICTION | Duplicate of GAP-006 |

---

*Phase: 05-domain-validation*
*Report synthesized: 2026-02-22*
*Source: gap-log.md + 05-01 through 05-05 SUMMARY.md files*
