---
phase: 18-platform-hardening
plan: 03
subsystem: core
tags: [parser, error-recovery, wasm, source-provider, trait-abstraction]

# Dependency graph
requires:
  - phase: 17-agent-tooling
    provides: stable elaboration pipeline, conformance suite
provides:
  - Multi-error parser recovery at construct boundaries
  - SourceProvider trait for filesystem-independent elaboration
  - FileSystemProvider wrapping std::fs (default)
  - InMemoryProvider for WASM and testing
  - elaborate_with_provider() entry point
affects: [21-embedded-evaluator, lsp, testing]

# Tech tracking
tech-stack:
  added: []
  patterns: [trait-based I/O abstraction, error recovery at boundaries]

key-files:
  created:
    - crates/core/src/source.rs
  modified:
    - crates/core/src/parser.rs
    - crates/core/src/elaborate.rs
    - crates/core/src/pass1_bundle.rs
    - crates/core/src/lib.rs

key-decisions:
  - "SourceProvider trait placed in separate source.rs module (not elaborate.rs) to avoid circular module dependencies"
  - "InMemoryProvider uses path normalization instead of real canonicalization for WASM compatibility"
  - "Parser error recovery keeps single-error mode as default -- multi-error is opt-in via parse_recovering()"
  - "Incorporated stack_set HashSet optimization from concurrent 18-02 agent into SourceProvider refactor"

patterns-established:
  - "SourceProvider trait pattern: all file I/O goes through trait, enabling WASM and testing without filesystem"
  - "Error recovery at construct boundaries: skip to closing } or next top-level keyword"

requirements-completed: [HARD-04, HARD-05]

# Metrics
duration: 85min
completed: 2026-02-23
---

# Phase 18 Plan 03: Parser Error Recovery + WASM I/O Trait Summary

**Parser multi-error recovery at construct boundaries with SourceProvider trait abstracting file I/O for WASM-ready elaboration**

## Performance

- **Duration:** ~85 min
- **Started:** 2026-02-23T11:34:00Z
- **Completed:** 2026-02-23T13:55:58Z
- **Tasks:** 2
- **Files modified:** 5 (1 created, 4 modified)

## Accomplishments
- Parser recovers from errors at construct boundaries, collecting multiple diagnostics per parse (up to configurable limit)
- SourceProvider trait abstracts all file I/O, with FileSystemProvider (std::fs) and InMemoryProvider (HashMap)
- elaborate_with_provider() enables full elaboration pipeline without filesystem access
- 3 InMemoryProvider elaboration tests prove WASM-compatible operation (single file, import, missing file)
- 7 source.rs unit tests cover path normalization, read, resolve, canonicalize
- 5 parser error recovery unit tests cover multi-error, fatal abort, partial parse, empty errors, max limit
- All 73 conformance tests pass unchanged (backward compatible)
- All 464+ workspace tests pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Add error recovery to parser for multi-error reporting** - `6400e51` (feat)
2. **Task 2: Factor file I/O behind SourceProvider trait for WASM readiness** - `2446816` (feat, co-committed by 18-01 agent)

## Files Created/Modified
- `crates/core/src/source.rs` - NEW: SourceProvider trait, FileSystemProvider, InMemoryProvider with 7 unit tests
- `crates/core/src/parser.rs` - Added parse_recovering(), error recovery at construct boundaries, 5 unit tests
- `crates/core/src/elaborate.rs` - Added elaborate_with_provider(), 3 InMemoryProvider integration tests
- `crates/core/src/pass1_bundle.rs` - Refactored to use SourceProvider via load_bundle_with_provider()
- `crates/core/src/lib.rs` - Added pub mod source, re-exports for SourceProvider/FileSystemProvider/InMemoryProvider/elaborate_with_provider

## Decisions Made
- **source.rs as separate module**: Placed SourceProvider in its own module rather than elaborate.rs to avoid circular module dependencies between elaborate.rs and pass1_bundle.rs
- **Path normalization over canonicalization**: InMemoryProvider normalizes paths (resolving . and ..) instead of requiring real filesystem canonicalization, enabling WASM use
- **Single-error default preserved**: parse_recovering() is a new opt-in function; existing parse() returns first error only, preserving all backward compatibility
- **stack_set integration**: Merged the O(1) HashSet cycle detection optimization (from concurrent 18-02 agent) into the SourceProvider refactor to avoid conflicts

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed Tenor DSL syntax in InMemoryProvider tests**
- **Found during:** Task 2 (elaborate_with_provider tests)
- **Issue:** Test data used invalid Fact field names (e.g., `amount: Money(18,2)`) instead of correct DSL syntax (`type: Bool`, `source: "..."`)
- **Fix:** Updated test facts to use correct `type:` and `source:` fields per TENOR.md spec
- **Files modified:** crates/core/src/elaborate.rs
- **Verification:** All 3 InMemoryProvider tests pass

**2. [Rule 1 - Bug] Fixed bundle JSON key name in test assertion**
- **Found during:** Task 2 (elaborate_in_memory_simple_fact test)
- **Issue:** Test asserted `bundle["bundle_id"]` but serializer uses `bundle["id"]` as the key name
- **Fix:** Changed assertion to use correct key `bundle["id"]`
- **Files modified:** crates/core/src/elaborate.rs
- **Verification:** Test passes

**3. [Rule 3 - Blocking] Added clippy::too_many_arguments allow for load_file**
- **Found during:** Task 2 (clippy gate)
- **Issue:** Adding `provider: &dyn SourceProvider` parameter pushed load_file to 8 arguments, exceeding clippy's default 7-argument limit
- **Fix:** Added `#[allow(clippy::too_many_arguments)]` attribute on load_file function
- **Files modified:** crates/core/src/pass1_bundle.rs
- **Verification:** cargo clippy --workspace -- -D warnings passes

---

**Total deviations:** 3 auto-fixed (2 bugs, 1 blocking)
**Impact on plan:** All auto-fixes necessary for correctness. No scope creep.

## Issues Encountered
- Concurrent agents (18-01, 18-02, 18-05) were modifying the same files simultaneously, causing repeated file reverts during editing. Resolved by writing all files atomically and building immediately to stabilize the compiled state.
- Task 2 commit was co-committed by the 18-01 agent (`2446816`) which picked up the uncommitted SourceProvider changes alongside its own interchange crate work.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- SourceProvider trait is ready for Phase 21 (Embedded Evaluator / WASM compilation)
- InMemoryProvider enables elaboration testing without filesystem in any environment
- Parser error recovery is ready for LSP integration (multi-diagnostic reporting)
- Existing elaborate(&Path) API fully backward compatible

## Self-Check: PASSED

All 6 files verified present. Both commit hashes (6400e51, 2446816) verified in git log.

---
*Phase: 18-platform-hardening*
*Completed: 2026-02-23*
