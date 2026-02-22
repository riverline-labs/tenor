---
phase: 05-domain-validation
plan: 02
subsystem: domain-validation
tags: [healthcare, prior-auth, flow, escalate, subflow, bounded-quantification, handoff]

# Dependency graph
requires:
  - phase: 03-cli-evaluator
    provides: "evaluate_flow API, flow step types (BranchStep, SubFlowStep, Escalate, HandoffStep)"
  - phase: 04-static-analysis
    provides: "tenor check CLI for static analysis validation"
provides:
  - "Healthcare prior auth domain contract (465 lines) -- the wow showcase for spec feature breadth"
  - "3 eval fixture sets (approve, deny, appeal) with verified verdicts"
  - "4 spec gap findings (GAP-005 through GAP-008) in running gap log"
affects: [05-domain-validation, 05-07-gap-report, 05-06-explain]

# Tech tracking
tech-stack:
  added: []
  patterns: [multi-flow-contract, appeal-subflow-pattern, escalation-handler-pattern, per-fixture-tenor-copies]

key-files:
  created:
    - domains/healthcare/prior_auth.tenor
    - domains/healthcare/prior_auth_approve.tenor
    - domains/healthcare/prior_auth_approve.facts.json
    - domains/healthcare/prior_auth_approve.verdicts.json
    - domains/healthcare/prior_auth_deny.tenor
    - domains/healthcare/prior_auth_deny.facts.json
    - domains/healthcare/prior_auth_deny.verdicts.json
    - domains/healthcare/prior_auth_appeal.tenor
    - domains/healthcare/prior_auth_appeal.facts.json
    - domains/healthcare/prior_auth_appeal.verdicts.json
  modified:
    - crates/eval/tests/conformance.rs
    - .planning/phases/05-domain-validation/gap-log.md

key-decisions:
  - "Used Enum instead of TaggedUnion for DenialReason (parser has no TaggedUnion support)"
  - "Used separate single-outcome operations (approve_auth, deny_auth) instead of multi-outcome operation due to missing effect-to-outcome DSL syntax"
  - "Per-fixture .tenor copies follow convention established by SaaS domain (one .tenor per fixture set)"
  - "Appeal sub-flow returning Ok from Terminate routes to parent SubFlowStep on_success (not on_failure)"

patterns-established:
  - "Multi-flow contract pattern: main flow with SubFlowStep referencing appeal flow"
  - "Escalation handler pattern: OperationStep on_failure Escalate routes to alternative review steps"
  - "Per-fixture .tenor copies: each eval fixture set gets its own copy of the contract with fixture-specific header comment"

requirements-completed: [DOMN-02, DOMN-06, DOMN-07, DOMN-08]

# Metrics
duration: 18min
completed: 2026-02-22
---

# Phase 5 Plan 2: Healthcare Prior Auth Summary

**Healthcare prior authorization contract (465 lines) with 6 personas, 4 rule strata, SubFlowStep appeal path, Escalate handler for peer review, and bounded quantification over medical records -- 3 eval paths verified**

## Performance

- **Duration:** 18 min
- **Started:** 2026-02-22T15:05:55Z
- **Completed:** 2026-02-22T15:24:10Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments
- Healthcare prior auth contract is the "wow" showcase: 465 lines, 6 personas, 2 entities, 17 rules across 4 strata, 8 operations, 2 flows with SubFlowStep, Escalate, BranchStep, HandoffStep, bounded quantification
- All 3 eval paths verified: approval (clinical review approve), denial (denied + appeal filing failed), appeal (denied + appeal filed + overturn granted)
- 4 spec gap findings documented: TaggedUnion absence (GAP-005), effect-to-outcome syntax gap (GAP-006), escalation path false positive unreachable (GAP-007), no CLI flow eval (GAP-008)
- Contract elaborates and passes `tenor check` with only informational finding about escalation step reachability

## Task Commits

Each task was committed atomically:

1. **Task 1: Author healthcare prior auth contract and eval fixtures** - `0dd28a9` (feat)
2. **Task 2: Register healthcare eval conformance tests** - `ec4d99a` (feat)

## Files Created/Modified
- `domains/healthcare/prior_auth.tenor` - Healthcare prior auth contract (465 lines), the showcase contract
- `domains/healthcare/prior_auth_approve.tenor` - Per-fixture copy for approval path eval
- `domains/healthcare/prior_auth_approve.facts.json` - Facts: all criteria met, routine urgency
- `domains/healthcare/prior_auth_approve.verdicts.json` - Expected: 10 verdicts, flow outcome "approved"
- `domains/healthcare/prior_auth_deny.tenor` - Per-fixture copy for denial path eval
- `domains/healthcare/prior_auth_deny.facts.json` - Facts: clinical criteria not met
- `domains/healthcare/prior_auth_deny.verdicts.json` - Expected: 9 verdicts, flow outcome "appeal_resolved"
- `domains/healthcare/prior_auth_appeal.tenor` - Per-fixture copy for appeal path eval
- `domains/healthcare/prior_auth_appeal.facts.json` - Facts: appeal filed, merit score 75, new evidence
- `domains/healthcare/prior_auth_appeal.verdicts.json` - Expected: 12 verdicts, flow outcome "appeal_resolved"
- `crates/eval/tests/conformance.rs` - 3 healthcare domain eval tests registered
- `.planning/phases/05-domain-validation/gap-log.md` - 4 new gap entries (GAP-005 through GAP-008)

## Decisions Made
- **Enum instead of TaggedUnion:** The feature coverage matrix called for a TaggedUnion type for DenialReason, but the parser/AST only supports TypeDecl as Record or Enum. Used Enum for denial reasons (flat variant selection) which covers the use case. Documented as GAP-005.
- **Separate operations for approve/deny:** Multi-outcome operations require effect-to-outcome mapping in the evaluator, but the DSL parser has no syntax for attaching outcome labels to effect tuples. Used separate approve_auth and deny_auth operations with BranchStep routing at the flow level. Documented as GAP-006.
- **Appeal sub-flow termination semantics:** When a sub-flow's Terminate handler fires (e.g., appeal_filing_failed), it returns Ok(FlowResult) to the parent. The parent SubFlowStep routes this to on_success (not on_failure), since on_failure only triggers on Err. This means the deny path flows through to appeal_resolved even when the appeal filing fails.
- **Per-fixture .tenor copies:** Following the convention established in plan 05-01 (SaaS), each eval fixture set gets its own copy of the contract .tenor file with a fixture-specific header comment.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed Unicode operators in DSL**
- **Found during:** Task 1 (contract authoring)
- **Issue:** Used ASCII `^` (caret) and `~` (tilde) for conjunction and negation instead of Unicode `∧` (U+2227) and `¬` (U+00AC)
- **Fix:** Replaced all `^` with `∧`, `~` with `¬`, `|` with `∨`, and `forall ... in ...` with `∀ ... ∈ ...`
- **Files modified:** domains/healthcare/prior_auth.tenor
- **Verification:** Elaboration succeeds after fix
- **Committed in:** 0dd28a9

**2. [Rule 1 - Bug] Fixed stratum violation in emergent_fast_track rule**
- **Found during:** Task 1 (elaboration verification)
- **Issue:** Rule `emergent_fast_track` at stratum 1 referenced verdict `documentation_ok` also produced at stratum 1 -- verdicts can only reference strata strictly less
- **Fix:** Changed emergent_fast_track to reference stratum 0 verdicts (records_complete, records_relevant) directly instead of the stratum 1 composite
- **Files modified:** domains/healthcare/prior_auth.tenor
- **Verification:** Elaboration succeeds after fix
- **Committed in:** 0dd28a9

---

**Total deviations:** 2 auto-fixed (2 bugs)
**Impact on plan:** Both were authoring errors caught by the elaborator's validation. No scope change.

## Issues Encountered
- Static analysis reports 3 unreachable steps in auth_review_flow (step_director_review, step_director_approve, step_director_deny). These steps are reachable via the Escalate handler but the S6 path tracer doesn't follow failure handler paths. Documented as GAP-007 -- informational finding, not a blocker.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Healthcare domain contract complete and verified -- the most complex showcase contract
- 4 gap findings contribute to the running gap log for the Phase 5 gap report (plan 05-07)
- Energy procurement (05-04) and trade finance (05-05) domain contracts can proceed independently
- `tenor explain` (05-06) now has a complex contract to validate output quality against

## Self-Check: PASSED

All 10 created files exist. Both task commits (0dd28a9, ec4d99a) verified. Contract is 465 lines (min 300). All 3 domain_healthcare eval tests passing.

---
*Phase: 05-domain-validation*
*Completed: 2026-02-22*
