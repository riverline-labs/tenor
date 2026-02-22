---
phase: 03-cli-evaluator
plan: 04
subsystem: cli
tags: [diff, structural-diff, interchange, construct-comparison]

# Dependency graph
requires:
  - phase: 03-cli-evaluator
    provides: "clap 4.5 CLI shell with stub subcommands (03-01)"
provides:
  - "tenor diff subcommand for construct-level interchange bundle diffing"
  - "BundleDiff with added/removed/changed construct sets keyed by (kind, id)"
  - "Field-level diffs for changed constructs excluding provenance noise"
  - "JSON and human-readable text output formats"
  - "Exit code 0 (identical) / 1 (different) convention"
affects: [phase-03.1-cffp-migration, phase-04-analyze]

# Tech tracking
tech-stack:
  added: []
  patterns: [construct-keyed diff by (kind, id), normalized set comparison for primitive arrays, provenance/line exclusion from diff]

key-files:
  created:
    - crates/cli/src/diff.rs
  modified:
    - crates/cli/src/main.rs

key-decisions:
  - "Hand-built domain-aware diff (not generic JSON diff) to key by (kind, id) not array position"
  - "Provenance and line fields excluded from comparison (noise fields that change with any edit)"
  - "Primitive arrays normalized as sets for comparison (states, allowed_personas order-insensitive)"
  - "Exit code convention: 0=identical, 1=different (matches Unix diff)"

patterns-established:
  - "Diff module pattern: diff::diff_bundles returns BundleDiff with to_json/to_text methods"
  - "cmd_diff handler follows same error-handling pattern as cmd_validate"

requirements-completed: [MIGR-01]

# Metrics
duration: 4min
completed: 2026-02-21
---

# Phase 3 Plan 4: Diff Subcommand Summary

**Domain-aware construct-level diff of interchange bundles keyed by (kind, id) with JSON/text output and Unix exit code convention**

## Performance

- **Duration:** 4 min
- **Started:** 2026-02-21T21:38:19Z
- **Completed:** 2026-02-21T21:42:13Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Implemented construct-level structural diff comparing bundles by (kind, id) key, not array position
- Field-level diffs for changed constructs with provenance/line noise excluded
- Normalized set comparison for primitive arrays (states, allowed_personas order-insensitive)
- JSON and human-readable text output formats with `--quiet` suppression
- 14 unit tests covering identical, added, removed, changed, multiple changes, set ordering, error cases
- 55/55 conformance tests still pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement construct-level bundle diff** - `f8ba45b` (feat)
2. **Task 2: Wire diff subcommand into CLI** - `849be82` (feat)

**Plan metadata:** `3131076` (docs: complete plan)

## Files Created/Modified
- `crates/cli/src/diff.rs` - BundleDiff, ConstructSummary, ConstructChange, FieldDiff types; diff_bundles algorithm; to_json/to_text output; 14 unit tests
- `crates/cli/src/main.rs` - Added mod diff, cmd_diff handler replacing stub, updated Diff subcommand help text

## Decisions Made
- Hand-built domain-aware diff rather than generic JSON diff library -- required for (kind, id) keying and provenance exclusion
- Provenance and line fields excluded from comparison as noise that changes with any edit
- Primitive arrays normalized as sorted sets for comparison to handle order-insensitive fields like states
- Exit code 0 for identical, 1 for different -- follows Unix diff convention

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- `tenor diff` is fully functional and ready for Phase 3.1 (CFFP Migration Semantics) to build on
- Phase 4 (`tenor diff --breaking`) can extend the BundleDiff type with breaking change classification
- diff::diff_bundles is the foundation for contract versioning workflows

## Self-Check: PASSED

All files exist. All commits verified (f8ba45b, 849be82).

---
*Phase: 03-cli-evaluator*
*Completed: 2026-02-21*
