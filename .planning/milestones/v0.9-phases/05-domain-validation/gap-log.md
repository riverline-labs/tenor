# Phase 5: Domain Validation -- Gap Log

[Running log of spec issues encountered during domain contract authoring]

## SaaS Subscription Domain (Plan 05-01)

### GAP-001: FieldRef on Record-typed facts not resolved by evaluator

- **Domain:** SaaS
- **Scenario:** Rule `seats_within_limit` compares `current_seat_count <= plan_features.max_seats` where `plan_features` is a Record-typed Fact
- **Attempted:** Standard field access syntax `plan_features.max_seats` in rule predicate
- **Result:** Evaluator's `FieldRef` resolution only checked `ctx.bindings` (loop variables from bounded quantifiers), not the `facts` FactSet. Record-typed facts accessed via field ref produced `UnboundVariable` error at eval time despite correct elaboration.
- **Severity:** BLOCKER (fixed inline as evaluator bug -- Rule 1 deviation)
- **Fix Applied:** Modified `eval_pred` in `crates/eval/src/predicate.rs` to fall back to `facts.get(var)` when `ctx.bindings.get(var)` returns None. Also records the fact in provenance when resolved from facts.
- **Suggested Spec Clarification:** None needed -- the spec is clear that Record field access should work on facts. This was purely an evaluator implementation gap.

### GAP-002: Entity state constraint in flow design

- **Domain:** SaaS
- **Scenario:** Wanted to test both activation (trial -> active) and suspension (active -> suspended) paths through the same flow
- **Attempted:** Single flow with a branch: payment ok -> activate, payment failed -> suspend
- **Result:** Entity always starts at initial state (`trial`). A branch-first flow routing to `suspend_subscription` (which expects `active -> suspended`) would fail because the entity is still in `trial` state. Flow had to be redesigned to first activate (trial -> active), then branch on payment for the suspend path.
- **Severity:** FRICTION (design workaround exists)
- **Suggested Fix:** Not needed -- this is inherent to the closed-world entity model. Entities always start at their declared initial state. The redesigned flow (activate first, then branch) is actually more realistic: you activate a subscription, then check ongoing payment status.

## Supply Chain Inspection Domain (Plan 05-03)

### GAP-003: No bounded existential quantification (exists / there-exists)

- **Domain:** Supply Chain
- **Scenario:** Checking if any item in a cargo inspection list has a defect (e.g., `exists item in inspection_items: item.compliant = false`)
- **Attempted:** The plan specified bounded existential quantification `exists item in inspection_items`, but the lexer/parser only supports `forall` (universal quantification, U+2200)
- **Result:** Cannot express "there exists an item with property X" directly. Must use negated universal: `not (forall item in list . item.prop = true)` to express "exists item where prop is not true", which is semantically equivalent but less readable.
- **Severity:** FRICTION (workaround exists via De Morgan's law: not-forall-not = exists)
- **Suggested Fix:** Add `exists` / `there_exists` / U+2203 as a lexer token and parser production for bounded existential quantification. Implementation: parse like `forall` but serialize with `"quantifier": "exists"` in interchange; evaluator short-circuits on first match.

### GAP-004: Parallel branch disjoint entity constraint prevents natural entity hierarchy modeling

- **Domain:** Supply Chain
- **Scenario:** Modeling concurrent quality and compliance inspections on the same InspectionLot entity
- **Attempted:** Two parallel branches both operating on InspectionLot (quality branch records quality pass, compliance branch records compliance pass)
- **Result:** Pass 5 validation error: "parallel branches must have disjoint entity effect sets". This is correct per spec Section 11.5 (prevents concurrent state conflicts), but it forces splitting what is conceptually one entity (InspectionLot) into two (QualityLot, ComplianceLot).
- **Severity:** FRICTION (workaround exists by splitting entities)
- **Suggested Fix:** Not needed per se -- the constraint is sound. Document the pattern: when parallel branches need to independently track progress on the same logical concept, model as separate entity types (one per branch). This is actually a good modeling practice that makes the state machine more precise.

## Healthcare Prior Auth Domain (Plan 05-02)

### GAP-005: No TaggedUnion type construct in DSL

- **Domain:** Healthcare
- **Scenario:** Modeling denial reasons as a discriminated union (medical_necessity, experimental_treatment, out_of_network, documentation_insufficient) where each variant could carry variant-specific payload data
- **Attempted:** The feature coverage matrix in research specified TypeDecl (TaggedUnion) for DenialReason. The AST and parser only support TypeDecl as Record (struct with named fields) and Enum (flat string set).
- **Result:** Used Enum instead of TaggedUnion. This works for the denial reason use case (just selecting a reason category) but cannot carry variant-specific payload data (e.g., experimental_treatment might want a clinical_trial_id field).
- **Severity:** FRICTION (workaround exists -- use Enum for simple discrimination, separate Record facts for variant payloads)
- **Suggested Fix:** Consider adding TaggedUnion as a TypeDecl variant in a future spec revision. For v1.0, Enum covers the common case. Document the Enum + separate Record pattern as idiomatic Tenor.

### GAP-006: No effect-to-outcome mapping syntax in DSL

- **Domain:** Healthcare
- **Scenario:** Modeling decide_auth as a multi-outcome operation where the outcome (approved/denied) is determined by which entity state transition fires. The plan called for a multi-outcome operation with approve/conditional_approve outcomes.
- **Attempted:** The spec describes multi-outcome operations with effect-to-outcome mapping. The evaluator supports it (effects carry an optional `outcome` field). However, the DSL parser only supports `effects: [(Entity, from, to)]` -- there is no syntax for attaching an outcome label to an individual effect tuple.
- **Result:** Used separate single-outcome operations (approve_auth, deny_auth) instead of one multi-outcome operation. This is functionally equivalent but less elegant -- the branching between approve and deny happens at the flow level (BranchStep) rather than the operation level.
- **Severity:** FRICTION (workaround exists -- use separate operations with flow-level branching)
- **Suggested Fix:** Add optional outcome label to effect tuples in DSL syntax: `effects: [(Entity, from, to, outcome_label)]` or `effects: [(Entity, from, to) => outcome_label]`. The evaluator already handles this via the `outcome` field on Effect structs; only the parser needs the syntax.

### GAP-007: Static analysis reports escalation targets as unreachable steps

- **Domain:** Healthcare
- **Scenario:** Flow auth_review_flow has an Escalate handler on step_deny that routes to step_director_review. Static analysis (S6) reports step_director_review, step_director_approve, and step_director_deny as unreachable.
- **Attempted:** These steps ARE reachable but only through the Escalate failure handler path, which the static path tracer does not follow.
- **Result:** Informational finding `[s6/INFO]: Flow 'auth_review_flow' has 3 unreachable step(s)`. Not a blocker but misleading -- the steps are actually reachable through escalation.
- **Severity:** COSMETIC (the finding is informational, not a warning or error, but could confuse contract authors)
- **Suggested Fix:** Enhance S6 path analysis to trace Escalate handler targets in addition to normal outcome-based routing. This would eliminate false positive unreachable step reports for escalation paths.

### GAP-008: No flow-level eval in CLI (only rule eval)

- **Domain:** Healthcare
- **Scenario:** Wanted to run `tenor eval --flow auth_review_flow --persona requesting_physician` to test flow execution from the CLI.
- **Attempted:** The `tenor eval` CLI subcommand only evaluates rules (no --flow or --persona flags). Flow evaluation is only available through the Rust API `tenor_eval::evaluate_flow()`.
- **Result:** Had to manually trace flow execution to construct expected verdict files. Correct verdict files can only be verified by running the Rust conformance tests.
- **Severity:** FRICTION (flow evaluation works through the API but not the CLI)
- **Suggested Fix:** Add `--flow <flow_id>` and `--persona <persona>` flags to `tenor eval` CLI to support flow evaluation. This is a CLI ergonomics improvement, not a spec issue.

## Trade Finance LC Domain (Plan 05-05)

### GAP-009: Frozen snapshot prevents sequential fact-based preconditions in flows

- **Domain:** Trade Finance
- **Scenario:** Sequential operations in a flow where each operation's precondition depends on a state change from the previous operation. E.g., `present_documents` requires `lc_status = "issued"`, then `examine_documents` requires `lc_status = "presented"`.
- **Attempted:** Using Enum fact `lc_status` as a precondition for operations in a sequential flow. Since the snapshot is frozen at initiation, the fact `lc_status` never changes during flow execution -- it remains "issued" throughout.
- **Result:** Cannot use sequential fact-value preconditions in a flow (e.g., status must be "issued" for step 1, then "presented" for step 2). Entity state transitions (effects) are tracked separately from fact values. Redesigned operations to use verdict-based or simple fact preconditions that are all satisfiable from the initial snapshot.
- **Severity:** FRICTION (design workaround exists -- use verdict_present() preconditions instead of evolving fact values)
- **Suggested Fix:** Not needed -- this is inherent to the frozen snapshot model (spec Section 11.2). Document the pattern: operation preconditions in flows should use verdict_present() or initial-snapshot-compatible fact checks, not facts that would need to evolve. Entity state evolution is handled by effects, not by fact mutation.

### GAP-010: Multi-outcome operations require effect-to-outcome mapping absent from DSL

- **Domain:** Trade Finance
- **Scenario:** Modeling `examine_documents` as a multi-outcome operation with outcomes `[accept, reject]` where the outcome routes to different flow steps.
- **Attempted:** Declared operation with `outcomes: [accept, reject]` and effects `[(Document, submitted, examined)]`. The evaluator requires effects to carry an `outcome` field for multi-outcome routing, but the DSL parser and serializer do not emit `outcome` on individual effect tuples.
- **Result:** Same issue as GAP-006 (Healthcare). Multi-outcome operations cannot be fully utilized because the elaborator does not produce effect-to-outcome mapping. Redesigned to single-outcome operations with BranchStep-based routing.
- **Severity:** FRICTION (same as GAP-006; reinforces that finding)
- **Suggested Fix:** Same as GAP-006 -- add optional outcome label to effect tuples in DSL syntax.

## Energy Procurement RFP Domain (Plan 05-04)

### GAP-011: Missing int_literal handling in evaluator default value parser

- **Domain:** Energy Procurement
- **Scenario:** Fact `minimum_score_threshold` with default value `180` (Int type)
- **Attempted:** Evaluating a contract with Int-typed facts that have default values
- **Result:** Evaluator crashed with "expected integer" error. The elaborator serializes Int defaults as `{"kind": "int_literal", "value": 180}` but `parse_default_value()` only handled `bool_literal`, `decimal_value`, and `money_value` -- the `int_literal` kind fell through to `parse_plain_value()` which tried `as_i64()` on the entire JSON object.
- **Severity:** BLOCKER (auto-fixed inline as evaluator bug)
- **Fix Applied:** Added `"int_literal"` case to `parse_default_value()` in `crates/eval/src/types.rs`

### GAP-012: Money literal parsing fails for interchange format in rule conditions

- **Domain:** Energy Procurement
- **Scenario:** Rule conditions comparing `rfp_amount` against Money literals like `Money { amount: "50000.00", currency: "USD" }`
- **Attempted:** Evaluating rules with Money comparison predicates
- **Result:** Evaluator crashed with "Money value missing 'amount' string". The interchange serializes Money literals as `{"amount": {"kind": "decimal_value", "value": "50000.00", ...}, "currency": "USD"}` -- the `amount` field is a structured object, not a plain string. But `parse_plain_value()` for Money called `v.get("amount").and_then(|a| a.as_str())` which fails on the structured object.
- **Severity:** BLOCKER (auto-fixed inline as evaluator bug)
- **Fix Applied:** Updated Money parsing in `parse_plain_value()` to handle both plain string format (facts) and structured decimal_value format (interchange literals)

### GAP-013: Multi-outcome operations with conflicting entity effects cannot be expressed (same as GAP-006, GAP-010)

- **Domain:** Energy Procurement
- **Scenario:** `award_contract` operation with outcomes `[award, reject]` where `award` transitions RFP to `awarded` and `reject` transitions RFP to `cancelled` -- both from `shortlisted`
- **Attempted:** `effects: [(RFP, shortlisted, awarded), (PurchaseOrder, pending, approved), (RFP, shortlisted, cancelled)]`
- **Result:** At eval time the evaluator applies ALL effects sequentially. After the first effect moves RFP to `awarded`, the third effect tries `(RFP, shortlisted, cancelled)` but finds RFP in `awarded` state -- entity state mismatch. The DSL has no syntax for mapping effects to specific outcomes. Same root cause as GAP-006 and GAP-010.
- **Severity:** FRICTION (workaround: split into separate `award_rfp` and `reject_rfp` operations, use BranchStep in flow to route)
- **Suggested Fix:** Same as GAP-006 -- add effect-to-outcome mapping syntax in DSL. Three domains have now independently hit this limitation.
