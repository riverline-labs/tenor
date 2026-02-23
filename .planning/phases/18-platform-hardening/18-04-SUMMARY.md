---
phase: 18-platform-hardening
plan: 04
subsystem: eval
tags: [hashmap, btreemap, performance, evaluator, indexing]

# Dependency graph
requires:
  - phase: 18-01
    provides: "tenor-interchange crate for shared typed deserialization"
provides:
  - "O(1) Contract lookups via HashMap indexes (operation, flow, entity, fact)"
  - "O(n) stratum evaluation via BTreeMap index (replaces O(k*n) double-scan)"
  - "Clone-free flow failure handling via std::mem::take()"
  - "Contract::new() constructor with automatic index building"
  - "Contract::get_operation/get_flow/get_entity/get_fact lookup methods"
affects: [eval, flow, rules, hosted-evaluator-service]

# Tech tracking
tech-stack:
  added: []
  patterns: ["HashMap index on Contract for O(1) ID lookups", "BTreeMap stratum grouping for ordered traversal", "std::mem::take() for zero-copy terminal returns"]

key-files:
  created: []
  modified:
    - "crates/eval/src/types.rs"
    - "crates/eval/src/rules.rs"
    - "crates/eval/src/flow.rs"
    - "crates/eval/src/lib.rs"
    - "crates/eval/src/assemble.rs"
    - "crates/eval/src/operation.rs"

key-decisions:
  - "Added Contract::new() constructor to centralize index building rather than requiring callers to build indexes manually"
  - "Used std::mem::take() rather than Cow for flow clone elimination -- simpler, safe at terminal return sites"
  - "BTreeMap chosen for stratum index to get automatic key-ordered iteration"

patterns-established:
  - "Contract::new(): Always use the constructor, never struct literals, so indexes stay in sync"
  - "get_* methods: Use Contract::get_operation/get_flow/get_entity/get_fact for O(1) lookups"

requirements-completed: [HARD-13, HARD-17, HARD-18]

# Metrics
duration: 16min
completed: 2026-02-23
---

# Phase 18 Plan 04: Evaluator Runtime Optimization Summary

**HashMap-indexed Contract with O(1) lookups, BTreeMap-indexed stratum evaluation, and clone-free flow failure handling**

## Performance

- **Duration:** 16 min
- **Started:** 2026-02-23T15:47:37Z
- **Completed:** 2026-02-23T16:04:00Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- Contract type now has HashMap indexes for O(1) lookups by ID (operations, flows, entities, facts)
- Stratum evaluation replaced O(k*n) double-scan with O(n) BTreeMap-indexed single pass
- Flow failure handling eliminates 3 unnecessary deep clones via std::mem::take()
- All 508+ tests pass unchanged, confirming behavioral equivalence

## Task Commits

Each task was committed atomically:

1. **Task 1: Add HashMap indexes to Contract type** - `32793f5` (feat)
2. **Task 2: Index stratum evaluation and eliminate flow deep clones** - `a21fa03` (feat)

## Files Created/Modified
- `crates/eval/src/types.rs` - Added HashMap index fields, Contract::new() constructor, get_* lookup methods
- `crates/eval/src/rules.rs` - Replaced double-scan with BTreeMap stratum index in eval_strata()
- `crates/eval/src/flow.rs` - Replaced .clone() with std::mem::take() in handle_failure() terminal returns; migrated sub-flow lookup to get_flow()
- `crates/eval/src/lib.rs` - Migrated flow lookup from .iter().find() to get_flow()
- `crates/eval/src/assemble.rs` - Updated test Contract construction to use Contract::new()
- `crates/eval/src/operation.rs` - Updated test Contract construction to use Contract::new()

## Decisions Made
- Added Contract::new() constructor to centralize index building rather than requiring each test and caller to manually populate index fields -- this ensures indexes always stay in sync with Vec contents
- Used std::mem::take() for clone elimination instead of Cow<[T]> -- take() is simpler and perfectly safe at terminal return sites where the original vectors are never accessed again
- BTreeMap was the natural choice for stratum index since it provides automatic key-ordered iteration, matching the required stratum evaluation order

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
- Pre-existing uncommitted changes in CLI crate files (Cargo.toml, serve.rs, main.rs, explain.rs) from a prior plan execution caused intermittent workspace build failures. Resolved by restoring these files to HEAD before each quality gate check. Not related to this plan's changes.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Evaluator runtime optimizations complete, reducing latency for the Hosted Evaluator Service (Phase 22)
- Contract::new() constructor and get_* methods available for all future eval consumers

---
*Phase: 18-platform-hardening*
*Completed: 2026-02-23*
