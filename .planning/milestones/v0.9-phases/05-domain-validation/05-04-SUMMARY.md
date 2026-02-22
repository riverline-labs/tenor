---
phase: 05-domain-validation
plan: 04
subsystem: domain-validation
tags: [energy-procurement, rfp-workflow, money-types, date-types, approval-tiers, escalate, multi-entity, bounded-quantification, multi-file-import]

# Dependency graph
requires:
  - phase: 03-cli-evaluator
    provides: "tenor-eval flow evaluation, rule evaluation, predicate evaluator"
  - phase: 04-static-analysis
    provides: "tenor check static analysis (S1-S8)"
provides:
  - "Energy procurement RFP workflow domain contract (361 lines, 2 files)"
  - "Three eval fixture sets (approve, reject, escalate paths)"
  - "run_domain_flow_fixture() helper for multi-file contract testing"
  - "Gap log entries GAP-011 through GAP-013"
affects: [05-domain-validation, 06-code-generation]

# Tech tracking
tech-stack:
  added: []
  patterns: ["run_domain_flow_fixture() for multi-file contract eval testing", "approval tier routing via BranchStep + Money comparisons"]

key-files:
  created:
    - domains/energy_procurement/types.tenor
    - domains/energy_procurement/rfp_workflow.tenor
    - domains/energy_procurement/rfp_approve.facts.json
    - domains/energy_procurement/rfp_approve.verdicts.json
    - domains/energy_procurement/rfp_reject.facts.json
    - domains/energy_procurement/rfp_reject.verdicts.json
    - domains/energy_procurement/rfp_escalate.facts.json
    - domains/energy_procurement/rfp_escalate.verdicts.json
  modified:
    - crates/eval/tests/conformance.rs
    - .planning/phases/05-domain-validation/gap-log.md

key-decisions:
  - "Split multi-outcome award_contract into separate award_rfp and reject_rfp operations due to DSL lacking effect-to-outcome mapping syntax (GAP-013)"
  - "Used run_domain_flow_fixture() helper to decouple contract file name from fixture name for multi-file contracts"
  - "Approval tier routing via BranchStep (not multi-outcome operation) models real procurement approval chains more accurately"

patterns-established:
  - "run_domain_flow_fixture(): test helper accepting separate contract path and fixture name for multi-file domain contracts"
  - "Approval tier pattern: Money comparison rules at stratum 1 produce tier-specific verdicts, BranchStep routes flow based on which tier verdict fires"

requirements-completed: [DOMN-04, DOMN-06, DOMN-07, DOMN-08]

# Metrics
duration: 25min
completed: 2026-02-22
---

# Phase 05 Plan 04: Energy Procurement RFP Workflow Summary

**Energy procurement RFP workflow contract (361 lines) with Money/Date comparisons, 4-tier approval routing via BranchStep, multi-entity effects (RFP + PurchaseOrder), bounded quantification (forall bid), Escalate handler, and multi-file import -- 3 eval paths verified**

## Performance

- **Duration:** 25 min
- **Started:** 2026-02-22T15:04:17Z
- **Completed:** 2026-02-22T15:29:45Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments
- Energy procurement RFP workflow contract modeling realistic approval tiers ($50k/$500k/$2M thresholds), supplier scoring, and purchase order generation
- Three eval fixture sets covering: direct approval (tier 1, under $50k), rejection (budget unavailable, escalation cascade), and VP tier routing ($750k high-value)
- Multi-file import: types.tenor defines SupplierScore, CostBreakdown, BidRecord shared types used across the contract
- Contract passes full static analysis (S1-S8) with no findings

## Task Commits

Each task was committed atomically:

1. **Task 1: Author energy procurement RFP workflow contract and eval fixtures** - `b5ac64c` (feat)
2. **Task 2: Register energy procurement eval conformance tests** - `f614367` (feat, included in concurrent commit)

## Files Created/Modified
- `domains/energy_procurement/types.tenor` - Shared types: SupplierScore, CostBreakdown, BidRecord
- `domains/energy_procurement/rfp_workflow.tenor` - Main contract: 5 personas, 11 facts, 2 entities, 12 rules (3 strata), 7 operations, 1 flow
- `domains/energy_procurement/rfp_approve.facts.json` - Happy path: $35k RFP, all checks pass, tier 1 direct approval
- `domains/energy_procurement/rfp_approve.verdicts.json` - Expected: 8 verdicts, flow outcome "awarded", 5 steps
- `domains/energy_procurement/rfp_reject.facts.json` - Reject path: budget not approved
- `domains/energy_procurement/rfp_reject.verdicts.json` - Expected: 6 verdicts, flow outcome "escalation_failed", 8 steps
- `domains/energy_procurement/rfp_escalate.facts.json` - High-value: $750k RFP, VP tier routing
- `domains/energy_procurement/rfp_escalate.verdicts.json` - Expected: 8 verdicts, flow outcome "awarded", 6 steps
- `crates/eval/tests/conformance.rs` - Added run_domain_flow_fixture() helper and 3 energy domain tests
- `.planning/phases/05-domain-validation/gap-log.md` - Added GAP-011, GAP-012, GAP-013

## Decisions Made
- **Split multi-outcome operation into separate award/reject operations:** The DSL lacks syntax for mapping effects to specific outcomes within a multi-outcome operation (GAP-013, same as GAP-006/GAP-010). Split `award_contract` into `award_rfp` (RFP->awarded + PO->approved) and `reject_rfp` (RFP->cancelled). Flow-level BranchStep handles the routing.
- **run_domain_flow_fixture() helper:** For multi-file contracts where the .tenor file name differs from the fixture name, created a helper that accepts both paths separately. Existing run_eval_flow_fixture() delegates to it.
- **Approval tier routing via Money comparison rules:** Rather than complex conditional logic in operations, the approval tier determination is modeled as 4 rules at stratum 1 producing tier-specific verdicts (pm_can_approve, cl_approval_required, vp_approval_required, cfo_approval_required). The flow BranchStep checks which tier verdict fired.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Missing int_literal handling in evaluator default value parser**
- **Found during:** Task 1 (energy procurement contract evaluation)
- **Issue:** Int-typed facts with defaults (e.g., `default: 180`) crash the evaluator. The elaborator serializes as `{"kind": "int_literal", "value": 180}` but `parse_default_value()` only handled `bool_literal`, `decimal_value`, and `money_value`.
- **Fix:** Added `"int_literal"` case to `parse_default_value()` in `crates/eval/src/types.rs`
- **Note:** Same fix was independently applied by the SaaS plan (05-01) executor and committed first. Both executors encountered the same bug concurrently.

**2. [Rule 1 - Bug] Money literal parsing fails for interchange format in rule conditions**
- **Found during:** Task 1 (energy procurement contract evaluation)
- **Issue:** Money literals in rule predicates serialize as `{"amount": {"kind": "decimal_value", "value": "50000.00", ...}, "currency": "USD"}` but `parse_plain_value()` for Money expected `amount` to be a plain string.
- **Fix:** Updated Money parsing to handle both plain string format (facts) and structured decimal_value format (interchange literals).
- **Note:** Same fix was independently applied by the SaaS plan (05-01) executor and committed first.

**3. [Design - Workaround] Multi-outcome operation refactored to separate operations**
- **Found during:** Task 1 (flow evaluation of award_contract)
- **Issue:** Multi-outcome `award_contract` with conflicting entity effects from same source state cannot be evaluated (GAP-013).
- **Fix:** Split into `award_rfp` and `reject_rfp` operations. Flow-level routing handles the decision.

---

**Total deviations:** 3 (2 auto-fixed bugs already resolved by parallel executor, 1 design workaround)
**Impact on plan:** Bugs were pre-existing evaluator gaps exposed by domain contract complexity. Design workaround (separate operations) is actually more realistic for procurement workflows. No scope creep.

## Issues Encountered
- Parallel execution: The SaaS plan (05-01) committed the same eval bugfixes before this plan, so the eval fixes were already in place when Task 1 committed. The conformance.rs test registration was included in the healthcare plan's (05-02) commit due to shared working tree in parallel execution.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Energy procurement domain contract complete and verified
- Three evaluation paths (approve/reject/escalate) cover the key flow branches
- Contract exercises Money types, Date types, approval tiers, Escalate handler, multi-entity effects, bounded quantification, and multi-file import
- Gap log updated with 3 findings (GAP-011 through GAP-013), reinforcing the effect-to-outcome mapping gap (now seen in 3 domains)

## Self-Check: PASSED

All 8 created files verified present. Both commits (b5ac64c, f614367) verified in git log. All 3 energy procurement eval tests passing.

---
*Phase: 05-domain-validation*
*Completed: 2026-02-22*
