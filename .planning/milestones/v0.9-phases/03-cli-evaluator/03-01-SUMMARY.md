---
phase: 03-cli-evaluator
plan: 01
subsystem: cli
tags: [clap, jsonschema, cli, subcommands, derive-api]

# Dependency graph
requires:
  - phase: 02-foundation
    provides: "tenor-core elaborate pipeline, conformance suite, interchange schema"
provides:
  - "clap 4.5 derive-based CLI with 9 subcommands"
  - "validate subcommand with embedded JSON Schema"
  - "stub subcommands for eval, diff, check, explain, generate (exit code 2)"
  - "global --output and --quiet flags"
  - "exit code convention: 0=success, 1=error, 2=not-implemented"
affects: [03-02, 03-03, 03-04, 03-05, 03-06, phase-04, phase-05, phase-06]

# Tech tracking
tech-stack:
  added: [clap 4.5, jsonschema 0.42]
  patterns: [clap-derive subcommand dispatch, include_str! schema embedding, exit-code convention]

key-files:
  created: []
  modified:
    - crates/cli/src/main.rs
    - crates/cli/Cargo.toml
    - Cargo.toml
    - .github/workflows/ci.yml
    - CLAUDE.md

key-decisions:
  - "jsonschema 0.42 with iter_errors API for validation error collection"
  - "include_str! to embed interchange schema at compile time (ships with binary)"
  - "Exit code convention: 0=success, 1=error, 2=not-implemented"
  - "CI updated from `run conformance` to `test conformance` to match new subcommand"

patterns-established:
  - "Subcommand dispatch: each command has its own cmd_* handler function"
  - "Stub pattern: stub_not_implemented() prints message to stderr, exits 2"
  - "Global flags: --output (text|json) and --quiet available to all subcommands"

requirements-completed: [CLI-01, CLI-02, CLI-03, CLI-07, CLI-09]

# Metrics
duration: 6min
completed: 2026-02-21
---

# Phase 3 Plan 1: CLI Migration Summary

**Clap 4.5 derive-based CLI with 9 subcommands, embedded JSON Schema validation, global output/quiet flags, and exit code convention**

## Performance

- **Duration:** 6 min
- **Started:** 2026-02-21T21:19:18Z
- **Completed:** 2026-02-21T21:25:57Z
- **Tasks:** 1
- **Files modified:** 6

## Accomplishments
- Replaced hand-rolled std::env::args() parsing with clap 4.5 derive API
- Registered 9 subcommands: elaborate, validate, eval, test, diff, check, explain, generate, ambiguity
- Elaborate, validate, test, and ambiguity are fully functional; stub commands exit with code 2
- Added global --output (text|json) and --quiet flags
- Embedded interchange JSON Schema via include_str! for the validate command
- 55/55 conformance tests still pass

## Task Commits

Each task was committed atomically:

1. **Task 1: Migrate CLI to clap derive with all subcommands and global flags** - `5672cd7` (feat)

**Plan metadata:** `a0233ed` (docs: complete plan)

## Files Created/Modified
- `crates/cli/src/main.rs` - Complete rewrite: clap Parser/Subcommand derive structs, dispatch handlers for elaborate/validate/test/ambiguity, stubs for eval/diff/check/explain/generate
- `crates/cli/Cargo.toml` - Added clap (workspace) and jsonschema 0.42 dependencies
- `Cargo.toml` - Added clap 4.5 to workspace dependencies
- `.github/workflows/ci.yml` - Updated conformance command from `run conformance` to `test conformance`
- `CLAUDE.md` - Updated CLI command reference (run -> test, added validate and --help)
- `Cargo.lock` - Updated lockfile with new dependencies

## Decisions Made
- Used jsonschema 0.42 with `iter_errors` API (not `validate` which returns single error) for collecting all schema validation errors
- Embedded interchange schema via `include_str!` so the binary is self-contained (no external file needed at runtime)
- Established exit code convention: 0=success, 1=error, 2=not-implemented for stub subcommands
- Updated CI workflow proactively to prevent breakage from the `run` -> `test` subcommand rename

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed jsonschema API usage (validate -> iter_errors)**
- **Found during:** Task 1 (compilation)
- **Issue:** Plan specified `validator.validate(&bundle)` returning an iterator of errors, but jsonschema 0.42 returns `Result<(), ValidationError>` (single error). The `iter_errors` method provides the multi-error collection pattern.
- **Fix:** Used `validator.iter_errors(&bundle)` instead of `validator.validate(&bundle)` to collect all validation errors
- **Files modified:** crates/cli/src/main.rs
- **Verification:** `cargo build -p tenor-cli` succeeds, validate command works correctly
- **Committed in:** 5672cd7 (Task 1 commit)

**2. [Rule 3 - Blocking] Updated CI workflow for renamed subcommand**
- **Found during:** Task 1 (post-implementation review)
- **Issue:** CI workflow used `cargo run -p tenor-cli -- run conformance` but the `run` command was removed in favor of `test`
- **Fix:** Updated `.github/workflows/ci.yml` to use `test conformance`
- **Files modified:** .github/workflows/ci.yml
- **Verification:** New command verified locally with 55/55 passing
- **Committed in:** 5672cd7 (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking)
**Impact on plan:** Both auto-fixes necessary for correctness. No scope creep.

## Issues Encountered
- Pre-existing `tenor-eval` crate has missing module files and unresolved imports causing `cargo test --workspace` to fail. This is NOT caused by our changes and is out of scope. Tests for `tenor-core` and `tenor-cli` pass cleanly.
- Pre-existing clippy warnings in `tenor-core` (result_large_err, for_kv_map) are not from our changes.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- CLI shell is complete and ready for subsequent plans to wire in functionality
- Plan 03-02 (Contract Evaluator) can wire `eval` subcommand into the existing stub
- Plan 03-03 (Diff) can wire `diff` subcommand into the existing stub
- All future phases have clear entry points via the subcommand dispatch pattern

## Self-Check: PASSED

All files exist. All commits verified.

---
*Phase: 03-cli-evaluator*
*Completed: 2026-02-21*
