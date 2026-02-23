---
phase: 18-platform-hardening
plan: 02
subsystem: core
tags: [rust, panic-free, hashset, serialization, wasm, performance]

# Dependency graph
requires:
  - phase: 18-platform-hardening
    provides: "Core elaborator pipeline (passes 1-6)"
provides:
  - "Panic-free validation and type checking passes"
  - "O(1) import cycle detection via HashSet"
  - "Reduced string allocations in JSON serialization"
affects: [eval, codegen, lsp, wasm]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Static key constants for repeated JSON field names"
    - "Inline ins() helper to centralize Map insert pattern"
    - "Parallel HashSet for O(1) membership with Vec for ordered reporting"

key-files:
  created: []
  modified:
    - "crates/core/src/pass5_validate.rs"
    - "crates/core/src/pass3_types.rs"
    - "crates/core/src/pass4_typecheck.rs"
    - "crates/core/src/pass1_bundle.rs"
    - "crates/core/src/pass6_serialize.rs"

key-decisions:
  - "Used unwrap_or(0) for pass4 min/max on fixed-size array (mathematically safe, defensive fallback)"
  - "Extracted 7 static key constants for most-used JSON keys (kind, id, value, tenor, provenance, base, op)"
  - "Used inline ins() helper rather than Cow<str> for string reduction (simpler, same perf benefit)"

patterns-established:
  - "ins() helper pattern: centralized map insertion reducing .to_owned() noise"
  - "Static key constants for high-frequency JSON keys (K_BASE, K_ID, K_KIND, K_OP, K_PROVENANCE, K_TENOR, K_VALUE)"

requirements-completed: [HARD-03, HARD-22, HARD-23]

# Metrics
duration: 35min
completed: 2026-02-23
---

# Phase 18 Plan 02: Core Hardening Summary

**Panic-free pass3/4/5 with ElabError propagation, O(1) HashSet cycle detection in pass1, and static key constants reducing pass6 string allocations**

## Performance

- **Duration:** 35 min
- **Started:** 2026-02-23T14:30:00Z
- **Completed:** 2026-02-23T15:05:00Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Eliminated all expect()/unwrap() calls from pass5_validate.rs (2 sites), pass3_types.rs (3 sites), and pass4_typecheck.rs (2 sites) with proper ElabError returns
- Import cycle detection upgraded from O(n) Vec::contains to O(1) HashSet lookup via parallel stack_set
- pass6_serialize.rs: 7 static key constants and inline ins() helper replace 103+ direct m.insert("key".to_owned(), ...) calls
- All 73 conformance tests pass with identical output (serialization behavior unchanged)
- All 464+ workspace tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Replace all expect() calls with proper ElabError returns** - `e770057` (fix)
2. **Task 2: HashSet cycle detection + pass6 string allocation reduction** - `67793a9` (refactor)

**Plan metadata:** TBD (docs: complete plan)

## Files Created/Modified
- `crates/core/src/pass5_validate.rs` - Replaced 2 expect() calls in cycle detection DFS (dfs_flow_refs, trigger_dfs) with ok_or_else returning ElabError
- `crates/core/src/pass3_types.rs` - Replaced 3 expect() calls in TypeDecl cycle detection with ok_or_else returning ElabError
- `crates/core/src/pass4_typecheck.rs` - Replaced 2 min()/max() expect() calls with unwrap_or(0) (fixed-size array, defensive fallback)
- `crates/core/src/pass1_bundle.rs` - Added parallel HashSet (stack_set) for O(1) cycle detection alongside Vec stack for ordered error messages
- `crates/core/src/pass6_serialize.rs` - Added 7 static key constants and ins() helper, replaced 103+ insert patterns

## Decisions Made
- Used `unwrap_or(&0)` for pass4_typecheck min/max instead of full ElabError since the array is always exactly 4 elements (mathematically safe) - added safety comment
- Chose static `&str` constants + `ins()` helper over `Cow<str>` for pass6 optimization: simpler, equally effective since serde_json::Map requires owned String keys regardless
- Kept `json!({})` macro calls unchanged for simple literal objects (Bool, Date, DateTime, Enum, TypeRef) - only optimized the explicit Map::new() + insert patterns

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Concurrent HEAD change required rebasing pass1_bundle.rs changes**
- **Found during:** Task 2 (pass1_bundle.rs HashSet changes)
- **Issue:** Between session start and Task 2 execution, commit 2446816 landed on main adding SourceProvider infrastructure to pass1_bundle.rs. The committed code already included the HashSet stack_set changes as part of that work.
- **Fix:** Recognized that the current HEAD already contained the pass1_bundle.rs HashSet changes. Restored pass1_bundle.rs to HEAD (which includes both SourceProvider and HashSet work). Only pass6_serialize.rs required a new commit.
- **Files modified:** None additional (pass1_bundle.rs changes already in HEAD)
- **Verification:** grep confirms stack_set usage, all tests pass
- **Committed in:** Already part of 2446816

---

**Total deviations:** 1 auto-fixed (1 blocking - concurrent HEAD change)
**Impact on plan:** HashSet changes were already committed by a concurrent process. Only pass6 string optimization required a new commit from this plan.

## Issues Encountered
- Concurrent agent committed SourceProvider refactoring (2446816) to pass1_bundle.rs while this plan was executing, which included the HashSet changes. This caused persistent "linter" interference that was actually rust-analyzer restoring to the new HEAD. Resolved by recognizing the HEAD had moved and working with the current committed code.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Core passes are now panic-free on user input (HARD-03)
- Import cycle detection is O(1) (HARD-22)
- Serialization has reduced allocations (HARD-23)
- Ready for further hardening passes (fuzzing, error recovery improvements)

---
*Phase: 18-platform-hardening*
*Completed: 2026-02-23*
