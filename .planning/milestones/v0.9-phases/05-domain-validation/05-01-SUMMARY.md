---
phase: 05-domain-validation
plan: 01
subsystem: domain-validation
tags: [tenor-dsl, saas, eval-fixtures, entity-states, record-facts, domain-contracts]

# Dependency graph
requires:
  - phase: 03-cli-evaluator
    provides: "tenor-eval flow evaluation, run_eval_flow_fixture test harness"
  - phase: 04-static-analysis
    provides: "tenor check static analysis (S1-S8 suite)"
provides:
  - "SaaS subscription domain contract (SMALL) with entity lifecycle, Record facts, multi-stratum rules"
  - "Two eval fixture sets (activate happy path, suspend failure path)"
  - "domains_dir() helper and domain test registration pattern for Phase 5 contracts"
  - "Gap log initialized with structured format for Phase 5 findings"
  - "Evaluator FieldRef fix: Record-typed fact field access outside bounded quantifiers"
affects: [05-domain-validation, 05-07-gap-report]

# Tech tracking
tech-stack:
  added: []
  patterns: ["domain contract fixture triplet with shared .tenor files per eval path", "Record fact field access in evaluator predicates"]

key-files:
  created:
    - domains/saas/saas_subscription.tenor
    - domains/saas/saas_activate.tenor
    - domains/saas/saas_activate.facts.json
    - domains/saas/saas_activate.verdicts.json
    - domains/saas/saas_suspend.tenor
    - domains/saas/saas_suspend.facts.json
    - domains/saas/saas_suspend.verdicts.json
    - .planning/phases/05-domain-validation/gap-log.md
  modified:
    - crates/eval/src/predicate.rs
    - crates/eval/src/types.rs
    - crates/eval/tests/conformance.rs

key-decisions:
  - "Activate operation precondition uses verdict_present(seats_ok) alone, not activation_approved, to allow suspend path reuse"
  - "Flow redesigned: activate first (trial->active), then branch on payment -- enables both paths from initial entity state"
  - "Fixture .tenor files are copies of canonical contract (escrow pattern) to match run_eval_flow_fixture naming convention"
  - "FieldRef evaluator fix: fall back to facts when binding not found, enabling Record-typed fact field access"

patterns-established:
  - "Domain contract authoring pattern: .tenor + .facts.json + .verdicts.json triplets in domains/{name}/ directory"
  - "Fixture .tenor files share contract content with different headers (same pattern as escrow conformance fixtures)"
  - "domains_dir() helper in conformance.rs for domain contract path resolution"

requirements-completed: [DOMN-01, DOMN-06, DOMN-07, DOMN-08]

# Metrics
duration: 12min
completed: 2026-02-22
---

# Phase 5 Plan 1: SaaS Subscription Domain Contract Summary

**SaaS subscription lifecycle contract with TypeDecl Record, 3 personas, entity state machine (4 states, 6 transitions), multi-stratum rules, and 2 eval fixture paths (activate/suspend) -- plus evaluator fix for Record-typed fact field access**

## Performance

- **Duration:** 12 min
- **Started:** 2026-02-22T15:04:01Z
- **Completed:** 2026-02-22T15:16:01Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments

- SaaS subscription contract (~140 lines) exercising TypeDecl Record, Enum fact, Bool fact with default, Int facts, entity state machine, 6 rules across 2 strata, 4 persona-restricted operations, 1 flow with BranchStep
- Two eval fixture sets validated end-to-end: activate (trial->active, payment ok) and suspend (trial->active->suspended, payment failed)
- Evaluator bug fixed: FieldRef resolution now falls back to facts when binding not found (Record-typed fact field access was broken)
- Gap log initialized with 2 findings (GAP-001 FieldRef bug, GAP-002 entity state constraint)
- Domain test registration pattern established for subsequent Phase 5 contracts

## Task Commits

Each task was committed atomically:

1. **Task 1: Author SaaS subscription contract and eval fixtures** - `da50c0a` (feat)
2. **Task 2: Register SaaS eval conformance tests** - `545a732` (feat)

## Files Created/Modified

- `domains/saas/saas_subscription.tenor` - Canonical SaaS subscription lifecycle contract
- `domains/saas/saas_activate.tenor` - Activation path fixture contract (same content, different header)
- `domains/saas/saas_activate.facts.json` - Happy path facts: seats within limit, payment ok
- `domains/saas/saas_activate.verdicts.json` - Expected verdicts: 4 verdicts, flow outcome "activated"
- `domains/saas/saas_suspend.tenor` - Suspension path fixture contract (same content, different header)
- `domains/saas/saas_suspend.facts.json` - Failure path facts: payment failed
- `domains/saas/saas_suspend.verdicts.json` - Expected verdicts: 3 verdicts, flow outcome "suspended"
- `crates/eval/src/predicate.rs` - FieldRef resolution falls back to facts for Record-typed fact field access
- `crates/eval/src/types.rs` - int_literal parsing and flexible Money format handling
- `crates/eval/tests/conformance.rs` - domains_dir() helper and 2 SaaS domain test registrations
- `.planning/phases/05-domain-validation/gap-log.md` - Phase 5 gap log initialized with 2 findings

## Decisions Made

- **Activate precondition uses seats_ok alone:** The activate_subscription operation precondition is `verdict_present(seats_ok)` rather than `verdict_present(activation_approved)` (which requires both seats_ok AND payment_current). This allows the activate step to succeed even when payment has failed, enabling the suspend path to work from the same flow.
- **Flow starts with activation, then branches:** Entity always starts at initial state (trial). The flow first activates (trial->active), then branches on payment status. This enables both the activate path (stays active) and suspend path (active->suspended) from the same flow. More realistic than a branch-first design.
- **Fixture .tenor files are copies:** Following the escrow conformance pattern, each fixture has its own .tenor file (identical contract content) because `run_eval_flow_fixture` builds the tenor path from the fixture name parameter.
- **FieldRef evaluator fix:** The evaluator's FieldRef resolution only checked `ctx.bindings` (loop variables). Record-typed facts accessed via `plan_features.max_seats` produced UnboundVariable errors. Fixed to fall back to `facts.get(var)` with proper provenance recording.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed FieldRef resolution for Record-typed facts**
- **Found during:** Task 1 (SaaS contract evaluation)
- **Issue:** Evaluator's `eval_pred` for `Predicate::FieldRef` only checked `ctx.bindings` (loop variables from bounded quantifiers), not the `facts` FactSet. Direct field access on Record-typed facts like `plan_features.max_seats` produced `UnboundVariable { name: "plan_features" }` even though the fact was correctly provided.
- **Fix:** Modified FieldRef resolution in `crates/eval/src/predicate.rs` to fall back to `facts.get(var)` when `ctx.bindings.get(var)` returns None. Also records the fact in provenance via `collector.record_fact(var)`.
- **Files modified:** `crates/eval/src/predicate.rs`
- **Verification:** Both SaaS eval tests pass; full workspace 360 tests pass
- **Committed in:** da50c0a (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 bug fix)
**Impact on plan:** Essential for Record-typed fact field access in evaluator. No scope creep -- the fix is 4 lines.

## Issues Encountered

- **Entity state constraint in flow design:** The original flow design (branch first, then either activate or suspend) couldn't work because the entity starts at `trial` and the suspend operation expects `active->suspended`. Redesigned the flow to activate first (trial->active), then branch on payment. This is documented as GAP-002 in the gap log but is actually a more realistic flow design.
- **Verdict ordering in fixtures:** Initial expected verdicts were alphabetically sorted by type, but the evaluator outputs verdicts in stratum order (0 first, then 1). Corrected to match actual output.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Domain contract authoring pattern fully established (directory structure, fixture naming, test registration)
- domains_dir() helper available for subsequent domain contracts
- Gap log initialized and ready for appendix from plans 05-02 through 05-05
- Evaluator FieldRef fix enables Record-typed fact field access for all future domain contracts

## Self-Check: PASSED

All 11 files verified present. Both task commits (da50c0a, 545a732) verified in git log.

---
*Phase: 05-domain-validation*
*Completed: 2026-02-22*
