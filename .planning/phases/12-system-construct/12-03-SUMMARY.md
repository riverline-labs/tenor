---
phase: 12-system-construct
plan: 03
subsystem: elaborator
tags: [system, parser, ast, lexer, indexing, multi-contract, composition]

# Dependency graph
requires:
  - phase: 12-system-construct
    plan: 02
    provides: "Complete System construct spec section in TENOR.md (Section 12) with DSL syntax"
provides:
  - "RawConstruct::System AST variant with members, shared_personas, triggers, shared_entities fields"
  - "RawTrigger struct for cross-contract flow trigger declarations"
  - "System parser (parse_system) handling full TENOR.md DSL syntax"
  - "Pass 2 System indexing with duplicate id detection"
  - "Systems field on Index struct"
  - "construct_id arm for System in Pass 6"
affects: [12-04 validation/serialization, 12-05 conformance, 12-06 static analysis]

# Tech tracking
tech-stack:
  added: []
  patterns: [System construct uses RawTrigger struct instead of tuple for readability, dot-separated contract.flow parsing for trigger source/target]

key-files:
  created: []
  modified: [crates/core/src/ast.rs, crates/core/src/parser.rs, crates/core/src/pass2_index.rs, crates/core/src/pass1_bundle.rs, crates/core/src/pass6_serialize.rs]

key-decisions:
  - "Used RawTrigger struct instead of 6-element tuple for cross-contract trigger data -- improves readability and maintainability"
  - "shared_personas and shared_entities use Vec<(String, Vec<String>)> tuple representation -- simpler than dedicated structs for 2-field structures"
  - "No separate lexer token for system keyword -- reuses Word token like all other keywords, dispatched in parser"
  - "Pass 1 cross-file dup check includes System alongside other construct kinds"

patterns-established:
  - "System sub-type structs (RawTrigger) in ast.rs follow same derive traits pattern as RawCompStep, RawBranch"
  - "System parser methods prefixed with parse_system_ for sub-field parsers"

requirements-completed: [SYS-01, SYS-02, SYS-03, SYS-04]

# Metrics
duration: 5min
completed: 2026-02-22
---

# Phase 12 Plan 03: Lexer, Parser, AST, Pass 2 Indexing Summary

**System construct AST variant with RawTrigger struct, full DSL parser for members/shared_personas/triggers/shared_entities, and Pass 2 duplicate-detecting indexer**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-22T18:19:30Z
- **Completed:** 2026-02-22T18:24:44Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- RawConstruct::System variant with all four cross-contract feature fields (members, shared_personas, triggers, shared_entities)
- RawTrigger struct for 6-field trigger declarations (source_contract, source_flow, on, target_contract, target_flow, persona)
- Full parser implementation handling TENOR.md Section 12 DSL syntax including dot-separated contract.flow references in triggers
- Pass 2 Index struct extended with `systems: HashMap<String, Provenance>` and duplicate System id detection
- All match arms updated across pass1_bundle, pass2_index, pass6_serialize to handle new System variant
- Workspace compiles cleanly, all 61 conformance tests pass, clippy clean

## Task Commits

Each task was committed atomically:

1. **Task 1: Add System AST variant and lexer token** - `79b2ec1` (feat)
2. **Task 2: Implement System parser and Pass 2 indexing** - `cca7da0` (feat)

## Files Created/Modified
- `crates/core/src/ast.rs` - Added RawConstruct::System variant and RawTrigger struct
- `crates/core/src/parser.rs` - Added parse_system() with sub-parsers, RawTrigger re-export, system keyword dispatch
- `crates/core/src/pass2_index.rs` - Added systems field to Index, duplicate System id detection
- `crates/core/src/pass1_bundle.rs` - Added System arm to cross-file duplicate check
- `crates/core/src/pass6_serialize.rs` - Added System arm to construct_id function

## Decisions Made
- **RawTrigger struct over tuple:** Used a named struct for the 6-field trigger representation instead of a tuple. The fields (source_contract, source_flow, on, target_contract, target_flow, persona) are too many for a readable tuple, and match the CFFP canonical form's named fields.
- **No lexer change needed:** The lexer uses `Token::Word(String)` for all identifiers and keywords. The parser's `parse_construct()` dispatches on word values, so adding "system" to the match suffices.
- **Compilation stub strategy:** Added minimal match arms for System in pass1_bundle, pass2_index, and pass6_serialize in Task 1 to allow workspace compilation, then replaced the pass2_index stub with full implementation in Task 2.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added System match arm to pass1_bundle.rs**
- **Found during:** Task 1 (AST variant addition)
- **Issue:** The plan mentioned only pass4/pass5/pass6 for non-exhaustive match fixes, but pass1_bundle.rs also has an explicit match on RawConstruct without a catch-all
- **Fix:** Added `RawConstruct::System { id, prov, .. } => ("System", id, prov)` to the cross-file dup check
- **Files modified:** crates/core/src/pass1_bundle.rs
- **Verification:** cargo build succeeds
- **Committed in:** 79b2ec1 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary for compilation. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- System construct can be parsed from DSL source into the AST and indexed in Pass 2
- The elaboration pipeline does not crash on System input (pass4 catch-all handles it, pass5 catch-all skips it, pass6 serializes empty constructs)
- Ready for plan 12-04: Pass 5 validation (C-SYS-01 through C-SYS-17) and Pass 6 serialization of System interchange JSON
- Trigger acyclicity checking, shared persona validation, entity state set equality all need implementation in Pass 5

## Self-Check: PASSED

All files exist, all commits verified, all content checks passed.

---
*Phase: 12-system-construct*
*Completed: 2026-02-22*
