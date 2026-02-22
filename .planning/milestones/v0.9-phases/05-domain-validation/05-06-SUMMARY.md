---
phase: 05-domain-validation
plan: 06
subsystem: cli
tags: [explain, cli, human-readable, contract-summary, analysis, markdown, terminal]

requires:
  - phase: 05-01
    provides: Domain contracts (SaaS, healthcare, supply chain, energy, trade finance) as test data
  - phase: 04
    provides: tenor-analyze S1-S8 analysis API for risk/coverage section
provides:
  - Working `tenor explain` CLI subcommand with 4-section contract summary
  - Terminal styled output (ANSI) and markdown output formats
  - Verbose mode with technical details (strata, preconditions, effects, findings)
  - CLI integration tests for explain subcommand
affects: [06-codegen, 08-lsp]

tech-stack:
  added: []
  patterns: [ANSI escape code styling for terminal output, section-based contract summary generation]

key-files:
  created:
    - crates/cli/src/explain.rs
  modified:
    - crates/cli/src/main.rs
    - crates/cli/tests/cli_integration.rs

key-decisions:
  - "ANSI escape codes used directly instead of crossterm/textwrap dependencies -- simpler, no new deps"
  - "Walk-based flow narrative follows entry point through step graph for natural reading order"
  - "Fact inventory grouped by type category (numeric, boolean, enum, record, text/temporal, list)"

patterns-established:
  - "Explain module pattern: pure function returning String, format enum for output type"
  - "Step walker with visited set for cycle-safe flow traversal"

requirements-completed: [CLI-06]

duration: 7min
completed: 2026-02-22
---

# Phase 5 Plan 6: Explain CLI Subcommand Summary

**`tenor explain` subcommand producing 4-section human-readable contract summaries with terminal styling, markdown output, verbose mode, and real S1-S8 analysis integration**

## Performance

- **Duration:** 7 min
- **Started:** 2026-02-22T15:35:23Z
- **Completed:** 2026-02-22T15:43:19Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- Fully functional `tenor explain` command replacing the stub (no longer exits with code 2)
- 4-section output: Contract Summary, Decision Flow Narrative, Fact Inventory, Risk/Coverage Notes
- All flow step kinds supported: OperationStep, BranchStep, HandoffStep, SubFlowStep, ParallelStep
- Terminal format with ANSI bold headings, cyan names, green checkmarks, yellow warnings
- Markdown format with ## headings, tables, and checkbox lists
- Verbose mode adds: persona lists, entity states, strata breakdown, preconditions, effects, analysis findings
- Accepts both .tenor (elaborates internally) and .json (interchange bundle) inputs
- 6 CLI integration tests covering happy path, formats, error cases

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement tenor explain command** - `2a67b06` (feat)
2. **Task 2: Add CLI integration tests for explain** - `0b7fea8` (test)

## Files Created/Modified
- `crates/cli/src/explain.rs` - 1218-line explain module with 4-section contract summary generation
- `crates/cli/src/main.rs` - Updated CLI wiring: ExplainOutputFormat enum, cmd_explain function, mod explain
- `crates/cli/tests/cli_integration.rs` - 6 new explain integration tests replacing stub test

## Decisions Made
- Used ANSI escape codes directly instead of adding crossterm/textwrap dependencies. Terminal styling is straightforward (bold, cyan, green, yellow) and doesn't warrant additional crate dependencies. The plan suggested these deps but they are unnecessary for the actual styling needs.
- Flow narrative walks from entry point through the step graph using a visited set to prevent infinite loops in cyclic flows. Branch steps allow independent path traversal for if-true and if-false paths.
- Fact inventory grouped by type category (numeric, boolean, enum, record, text/temporal, list) for readability. Categories derived from the interchange type `base` field.
- Condition descriptions handle all interchange predicate forms: verdict_present, comparison operators, logical and/or, unary not (with `operand` field).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed `not` operator condition display**
- **Found during:** Task 1 (explain implementation)
- **Issue:** Healthcare contract's `uphold_denial` precondition uses `{"op": "not", "operand": {...}}` format, but initial describe_condition expected a `not` key at top level
- **Fix:** Added `"not"` as an op variant in describe_condition, reading the `operand` field
- **Files modified:** crates/cli/src/explain.rs
- **Verification:** Healthcare explain output shows "not (verdict 'overturn_recommended' is present)"
- **Committed in:** 2a67b06 (Task 1 commit)

**2. [Rule 1 - Bug] Fixed `or`/`and` logical operator display**
- **Found during:** Task 1 (explain implementation)
- **Issue:** Supply chain contract's begin_inspection precondition has nested `or` operators that were displayed as raw JSON
- **Fix:** Added recursive handling for `"and"` and `"or"` operators in describe_condition
- **Files modified:** crates/cli/src/explain.rs
- **Verification:** Supply chain explain output shows readable condition like "(inspection_type = "standard" or ...)"
- **Committed in:** 2a67b06 (Task 1 commit)

**3. [Rule 1 - Bug] Fixed Escalate handler display**
- **Found during:** Task 1 (explain implementation)
- **Issue:** Escalate on_failure handler uses `to_persona` field, not `target`
- **Fix:** Changed field access from `target` to `to_persona` in describe_operation_step
- **Files modified:** crates/cli/src/explain.rs
- **Verification:** Healthcare explain shows "On failure: escalate to medical_director"
- **Committed in:** 2a67b06 (Task 1 commit)

---

**Total deviations:** 3 auto-fixed (3 bugs)
**Impact on plan:** All fixes necessary for correct condition/handler display. No scope creep. Plan did not specify crossterm/textwrap deps were required; ANSI codes used instead (simpler).

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Explain subcommand is complete and production-ready
- All 5 domain contracts work as explain test data
- Ready for Phase 5 plans 07-08 (gap analysis log, cross-domain summary)

---
*Phase: 05-domain-validation*
*Completed: 2026-02-22*
