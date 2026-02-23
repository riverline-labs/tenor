---
phase: 12-system-construct
plan: 05
subsystem: conformance
tags: [system, conformance, testing, positive, negative, interchange, json-schema]

# Dependency graph
requires:
  - phase: 12-system-construct
    plan: 04
    provides: "Pass 5 System validation, Pass 6 System serialization, interchange JSON schema System definition"
provides:
  - "4 positive System conformance fixtures (basic, shared persona, flow trigger, shared entity)"
  - "2 standalone member contract conformance fixtures (system_member_a, system_member_b)"
  - "4 negative System conformance fixtures (duplicate id pass 2, duplicate member pass 5, invalid persona ref pass 5, invalid flow trigger pass 5)"
  - "Schema validation coverage for all System expected JSONs"
affects: [12-06 static analysis, domain re-validation]

# Tech tracking
tech-stack:
  added: []
  patterns: [member contract reuse across multiple System fixture files, shared persona/entity identity via matching ids across contracts]

key-files:
  created:
    - conformance/positive/system_member_a.tenor
    - conformance/positive/system_member_a.expected.json
    - conformance/positive/system_member_b.tenor
    - conformance/positive/system_member_b.expected.json
    - conformance/positive/system_basic.tenor
    - conformance/positive/system_basic.expected.json
    - conformance/positive/system_shared_persona.tenor
    - conformance/positive/system_shared_persona.expected.json
    - conformance/positive/system_flow_trigger.tenor
    - conformance/positive/system_flow_trigger.expected.json
    - conformance/positive/system_shared_entity.tenor
    - conformance/positive/system_shared_entity.expected.json
    - conformance/negative/pass2/system_duplicate_id.tenor
    - conformance/negative/pass2/system_duplicate_id.expected-error.json
    - conformance/negative/pass5/system_duplicate_member.tenor
    - conformance/negative/pass5/system_duplicate_member.expected-error.json
    - conformance/negative/pass5/system_invalid_persona_ref.tenor
    - conformance/negative/pass5/system_invalid_persona_ref.expected-error.json
    - conformance/negative/pass5/system_invalid_flow_trigger.tenor
    - conformance/negative/pass5/system_invalid_flow_trigger.expected-error.json
  modified: []

key-decisions:
  - "Member contracts (system_member_a, system_member_b) designed as reusable standalone contracts with shared persona id 'applicant' and entity id 'application' for cross-contract testing"
  - "snapshot field uses 'at_initiation' per schema requirement (not entity name)"
  - "Expected JSONs generated from elaborator output (not hand-written) ensuring byte-for-byte conformance match"

patterns-established:
  - "System conformance fixtures: member contract .tenor files as standalone passing fixtures, System .tenor files referencing member ids and paths"
  - "Negative System fixtures test both pass 2 (indexing) and pass 5 (validation) error detection"

requirements-completed: [SYS-08]

# Metrics
duration: 12min
completed: 2026-02-22
---

# Phase 12 Plan 05: System Conformance Fixtures Summary

**10 System conformance fixtures (6 positive, 4 negative) covering all System features with elaborator-generated expected JSONs validated against interchange schema**

## Performance

- **Duration:** 12 min
- **Started:** 2026-02-22T18:40:34Z
- **Completed:** 2026-02-22T18:52:17Z
- **Tasks:** 2
- **Files modified:** 20

## Accomplishments
- 6 positive conformance fixtures: 2 standalone member contracts (system_member_a, system_member_b) with fact/entity/persona/rule/operation/flow constructs, plus 4 System fixtures (basic, shared persona, flow trigger, shared entity)
- 4 negative conformance fixtures: duplicate System id (pass 2), duplicate member (pass 5), invalid persona ref (pass 5), invalid flow trigger (pass 5)
- All expected JSONs generated from elaborator output for byte-for-byte conformance match
- Schema validation test auto-discovers and validates all new System expected JSONs
- Conformance suite grew from 61 to 71 passing tests

## Task Commits

Both tasks' fixtures were already committed by a concurrent 12-06 plan executor:

1. **Task 1: Positive System conformance fixtures** - `99a41e3` (feat)
2. **Task 2: Negative System conformance fixtures** - `01db2b7` (feat)

## Files Created/Modified
- `conformance/positive/system_member_a.tenor` - Standalone member contract A with applicant persona, application entity, age fact, check_age rule, submit/approve operations, application_flow
- `conformance/positive/system_member_a.expected.json` - Expected interchange JSON for member A
- `conformance/positive/system_member_b.tenor` - Standalone member contract B sharing persona id "applicant" and entity id "application"
- `conformance/positive/system_member_b.expected.json` - Expected interchange JSON for member B
- `conformance/positive/system_basic.tenor` - Minimal System with 2 members, no shared features
- `conformance/positive/system_basic.expected.json` - Expected interchange JSON for basic System
- `conformance/positive/system_shared_persona.tenor` - System with shared persona binding across 2 contracts
- `conformance/positive/system_shared_persona.expected.json` - Expected interchange JSON for shared persona System
- `conformance/positive/system_flow_trigger.tenor` - System with cross-contract flow trigger (success -> review_flow)
- `conformance/positive/system_flow_trigger.expected.json` - Expected interchange JSON for flow trigger System
- `conformance/positive/system_shared_entity.tenor` - System with shared entity relationship (application entity)
- `conformance/positive/system_shared_entity.expected.json` - Expected interchange JSON for shared entity System
- `conformance/negative/pass2/system_duplicate_id.tenor` - Two Systems with same id
- `conformance/negative/pass2/system_duplicate_id.expected-error.json` - Pass 2 duplicate id error
- `conformance/negative/pass5/system_duplicate_member.tenor` - System with duplicate member id
- `conformance/negative/pass5/system_duplicate_member.expected-error.json` - Pass 5 duplicate member error
- `conformance/negative/pass5/system_invalid_persona_ref.tenor` - Shared persona referencing non-member contract
- `conformance/negative/pass5/system_invalid_persona_ref.expected-error.json` - Pass 5 invalid persona ref error
- `conformance/negative/pass5/system_invalid_flow_trigger.tenor` - Trigger targeting non-member contract
- `conformance/negative/pass5/system_invalid_flow_trigger.expected-error.json` - Pass 5 invalid trigger error

## Decisions Made
- **Member contract design:** Created system_member_a and system_member_b as full standalone contracts with matching persona id ("applicant") and entity id ("application") to enable testing shared persona and shared entity features across contracts.
- **Snapshot field:** Used `at_initiation` (not entity name) per interchange schema requirement and existing fixture convention.
- **Expected JSON generation:** All .expected.json files generated via `cargo run -p tenor-cli -- elaborate` to ensure byte-for-byte conformance match, not hand-written.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed snapshot field in member contract .tenor files**
- **Found during:** Task 1 (positive fixtures)
- **Issue:** Initial member contracts used `snapshot: application` (entity name) instead of `snapshot: at_initiation`, causing schema validation failure
- **Fix:** Changed to `snapshot: at_initiation` and regenerated expected JSONs
- **Files modified:** system_member_a.tenor, system_member_b.tenor
- **Verification:** Schema validation test passes
- **Committed in:** 99a41e3

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Minor DSL field value correction. No scope creep.

## Issues Encountered
- Fixture files for both positive and negative tests were already committed by a concurrent 12-06 plan executor (commits 99a41e3, 01db2b7). Work was verified to be identical -- no additional commits needed for the fixture files themselves.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Full conformance suite coverage for System constructs (10 fixtures total)
- All 71 conformance tests pass (up from 61 before System construct work)
- Schema validation confirms all positive expected JSONs comply with interchange schema
- Ready for plan 12-06: static analysis extensions for System constructs
- Deep cross-contract validation fixtures (requiring member contract elaboration) remain for future phases

## Self-Check: PASSED

All files exist, all commits verified, all content checks passed.

---
*Phase: 12-system-construct*
*Completed: 2026-02-22*
