---
phase: 12-system-construct
plan: 04
subsystem: elaborator
tags: [system, validation, serialization, interchange, json-schema, pass5, pass6, cross-contract]

# Dependency graph
requires:
  - phase: 12-system-construct
    plan: 03
    provides: "RawConstruct::System AST variant, parser, Pass 2 indexing"
  - phase: 12-system-construct
    plan: 02
    provides: "Complete System spec section with C-SYS-01 through C-SYS-17 constraints"
provides:
  - "Pass 5 System structural validation (C-SYS-01, C-SYS-07/08, C-SYS-11, C-SYS-15, C-SYS-16, C-SYS-17)"
  - "Pass 6 System interchange JSON serialization with sorted keys per Section 12.5"
  - "Interchange JSON Schema System definition with SystemMember, SharedPersona, SystemTrigger, SharedEntity"
  - "End-to-end System elaboration: parse -> index -> validate -> serialize"
affects: [12-05 conformance, 12-06 static analysis]

# Tech tracking
tech-stack:
  added: []
  patterns: [DFS trigger graph cycle detection analogous to flow reference graph cycle detection, canonical sorted-key serialization for System sub-objects]

key-files:
  created: []
  modified: [crates/core/src/pass5_validate.rs, crates/core/src/pass6_serialize.rs, docs/interchange-schema.json]

key-decisions:
  - "System validation in Pass 5 checks structural constraints that don't require elaborated member contracts (member uniqueness, contract membership refs, outcome validity, trigger acyclicity)"
  - "Cross-contract deep validation (C-SYS-06, C-SYS-09, C-SYS-10, C-SYS-12, C-SYS-13, C-SYS-14) deferred to System-level elaboration when member contracts are loaded"
  - "System constructs placed after Flows in canonical construct ordering within the Bundle"
  - "Shared persona/entity bindings require minimum 2 contracts (binding with 1 is meaningless)"
  - "Self-referential triggers (same contract + same flow) rejected as validation error"

patterns-established:
  - "System validation function follows same pattern as validate_entity/validate_operation/validate_flow"
  - "Trigger graph acyclicity uses DFS with in_path tracking, analogous to flow reference graph cycle detection"
  - "System interchange schema uses dedicated sub-type definitions (SystemMember, SharedPersona, SystemTrigger, SharedEntity)"

requirements-completed: [SYS-06, SYS-07]

# Metrics
duration: 7min
completed: 2026-02-22
---

# Phase 12 Plan 04: Pass 5 Validation and Pass 6 Serialization Summary

**System construct validation with trigger acyclicity checking and canonical interchange JSON serialization with sorted-key output matching Section 12.5 spec**

## Performance

- **Duration:** 7 min
- **Started:** 2026-02-22T18:28:41Z
- **Completed:** 2026-02-22T18:36:02Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Pass 5 validate_system() covering 10 structural constraint checks (C-SYS-01, C-SYS-07, C-SYS-08, C-SYS-11, C-SYS-15, C-SYS-16, C-SYS-17, plus minimum member count, minimum shared binding contracts, self-referential trigger rejection)
- Pass 6 serialize_system() producing canonical interchange JSON with lexicographically sorted keys at every level (members by id, contracts sorted within shared bindings, triggers sorted by source/target)
- DFS-based trigger graph acyclicity check (C-SYS-15) detecting cross-contract trigger cycles
- Interchange JSON Schema extended with System, SystemMember, SharedPersona, SystemTrigger, SharedEntity definitions
- End-to-end verification: valid System .tenor files elaborate to correct interchange JSON that passes schema validation
- All 61 conformance tests continue passing, clippy clean

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement Pass 5 System validation** - `f32e5ed` (feat)
2. **Task 2: Implement Pass 6 System serialization and extend JSON Schema** - `5c82671` (feat)

## Files Created/Modified
- `crates/core/src/pass5_validate.rs` - Added validate_system(), validate_trigger_acyclicity(), trigger_dfs() for structural System validation
- `crates/core/src/pass6_serialize.rs` - Added serialize_system() with canonical sorted-key output, System in construct ordering
- `docs/interchange-schema.json` - Added System, SystemMember, SharedPersona, SystemTrigger, SharedEntity definitions to $defs; added System ref to Construct.oneOf

## Decisions Made
- **Structural vs. deep validation split:** Pass 5 validates constraints checkable from the System construct alone (member uniqueness, contract refs, outcome validity, trigger acyclicity). Constraints requiring elaborated member contract data (C-SYS-06 shared persona existence, C-SYS-09/10 flow existence, C-SYS-12 target persona, C-SYS-13/14 entity state sets) are deferred to System-level elaboration when member contracts are loaded.
- **Minimum binding size:** Shared persona and shared entity bindings must reference at least 2 contracts. A binding with 1 contract is meaningless (sharing requires multiple participants).
- **Self-referential trigger rejection:** A trigger where source_contract == target_contract AND source_flow == target_flow is rejected as it would create an immediate self-loop.
- **Construct ordering:** Systems placed after Flows in the canonical construct array ordering, consistent with the spec's construct dependency order.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Full elaboration pipeline works end-to-end for System constructs (parse -> index -> validate -> serialize)
- Valid System inputs produce correct interchange JSON matching Section 12.5 spec
- Invalid System inputs produce structured ElabError with pass=5, construct_kind="System"
- JSON Schema validates System interchange documents
- Ready for plan 12-05: conformance test fixtures for System constructs
- Deep cross-contract validation (requiring member contract elaboration) remains for future implementation when System-level elaboration pipeline is built

## Self-Check: PASSED

All files exist, all commits verified, all content checks passed.

---
*Phase: 12-system-construct*
*Completed: 2026-02-22*
