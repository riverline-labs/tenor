---
phase: 03-cli-evaluator
plan: 05
subsystem: cli
tags: [eval-subcommand, assert-cmd, integration-tests, cli, evaluator-wiring]

# Dependency graph
requires:
  - phase: 03-cli-evaluator
    plan: 01
    provides: "clap CLI shell with eval stub subcommand"
  - phase: 03-cli-evaluator
    plan: 03
    provides: "tenor-eval evaluate() API with rules, operations, flows"
provides:
  - "Working `tenor eval <bundle> --facts <facts>` subcommand producing verdict JSON"
  - "25 CLI integration tests covering all 8 subcommand categories"
  - "Eval test fixtures: .tenor source, elaborated bundle, facts, expected verdicts"
affects: [03-06-PLAN, phase-04, phase-05]

# Tech tracking
tech-stack:
  added: [assert_cmd 2, predicates 3, tempfile 3]
  patterns: [workspace_root() helper for integration test fixture paths, assert_cmd spawn + assert pattern]

key-files:
  created:
    - crates/cli/tests/cli_integration.rs
    - crates/cli/tests/fixtures/eval_basic.tenor
    - crates/cli/tests/fixtures/eval_basic_bundle.json
    - crates/cli/tests/fixtures/eval_basic.facts.json
    - crates/cli/tests/fixtures/eval_basic.expected_verdicts.json
  modified:
    - crates/cli/src/main.rs
    - crates/cli/Cargo.toml
    - Cargo.lock

key-decisions:
  - "workspace_root() helper navigates from CARGO_MANIFEST_DIR to workspace root for fixture paths in integration tests"
  - "Static fixture files preferred over tempfile-generated fixtures for eval tests (reproducibility)"
  - "Eval text output shows verdict type, payload, rule, and stratum in human-readable format"

patterns-established:
  - "Integration test pattern: tenor() helper with current_dir(workspace_root()) for all assert_cmd tests"
  - "Eval fixture triplet: .tenor source + elaborated bundle JSON + facts JSON for end-to-end eval testing"

requirements-completed: [CLI-05, TEST-07]

# Metrics
duration: 5min
completed: 2026-02-21
---

# Phase 3 Plan 05: Eval Subcommand and CLI Integration Tests Summary

**Working `tenor eval` subcommand wired to evaluator library, plus 25 integration tests covering all CLI subcommands with assert_cmd**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-21T21:47:59Z
- **Completed:** 2026-02-21T21:53:51Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Wired `tenor eval` subcommand to `tenor_eval::evaluate()` with full file I/O, JSON/text output formats, and proper error handling
- Created 25 CLI integration tests covering: help/version (3), elaborate (3), validate (2), test (2), eval (6), diff (2), stubs (3), global flags (4)
- All integration tests verify exit codes (0=success, 1=error, 2=not-implemented) and stdout/stderr content
- Eval test fixtures: minimal .tenor contract, elaborated bundle JSON, facts JSON, expected verdicts JSON

## Task Commits

Each task was committed atomically:

1. **Task 1: Wire eval subcommand to tenor-eval library** - `7cf008b` (feat)
2. **Task 2: Create CLI integration tests with assert_cmd** - `36b3a8f` (feat)

## Files Created/Modified
- `crates/cli/src/main.rs` - Replaced eval stub with full implementation: bundle/facts I/O, evaluate(), verdict output (JSON and text), error handling
- `crates/cli/Cargo.toml` - Added tenor-eval dependency and assert_cmd/predicates/tempfile dev-dependencies
- `crates/cli/tests/cli_integration.rs` - 25 integration tests using assert_cmd::Command with workspace_root() helper
- `crates/cli/tests/fixtures/eval_basic.tenor` - Minimal contract: 1 fact (Bool), 1 rule producing verdict
- `crates/cli/tests/fixtures/eval_basic_bundle.json` - Elaborated interchange JSON from eval_basic.tenor
- `crates/cli/tests/fixtures/eval_basic.facts.json` - Facts input: `{"is_active": true}`
- `crates/cli/tests/fixtures/eval_basic.expected_verdicts.json` - Expected verdict output with provenance
- `Cargo.lock` - Updated lockfile with new dependencies

## Decisions Made
- Used `workspace_root()` helper (CARGO_MANIFEST_DIR -> parent -> parent) to resolve fixture paths from integration tests -- assert_cmd runs from the crate root, not workspace root
- Static fixture files chosen over tempfile-generated fixtures for eval tests -- ensures reproducible, inspectable test data
- Eval text output format shows `[verdict_type] payload (rule: id, stratum: N)` for human readability
- format_verdict_payload() helper handles all evaluator Value types for text display

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed .tenor fixture syntax for produce clause**
- **Found during:** Task 1 (fixture elaboration)
- **Issue:** Initial eval_basic.tenor used `produce: verdict active_confirmed { type: Bool, value: true }` which the parser rejects -- correct syntax is `produce: verdict active_confirmed { payload: Bool = true }`
- **Fix:** Updated fixture to use correct DSL syntax matching conformance suite convention; also added missing `stratum: 0` field
- **Files modified:** crates/cli/tests/fixtures/eval_basic.tenor
- **Verification:** `cargo run -p tenor-cli -- elaborate crates/cli/tests/fixtures/eval_basic.tenor` succeeds
- **Committed in:** 7cf008b (Task 1 commit)

**2. [Rule 3 - Blocking] Fixed integration test working directory for fixture path resolution**
- **Found during:** Task 2 (integration test execution)
- **Issue:** assert_cmd spawns the binary with the test runner's working directory, causing all relative paths to conformance fixtures and eval fixtures to fail with "No such file or directory"
- **Fix:** Added `workspace_root()` helper using `CARGO_MANIFEST_DIR` env var, set `.current_dir(workspace_root())` on all Command instances
- **Files modified:** crates/cli/tests/cli_integration.rs
- **Verification:** All 25 integration tests pass
- **Committed in:** 36b3a8f (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking)
**Impact on plan:** Both auto-fixes were necessary for correct operation. No scope change.

## Issues Encountered
- Pre-existing clippy warnings in ambiguity module (dead_code for spec_sections, total, matches, mismatches) -- not from our changes, out of scope
- Deprecated `Command::cargo_bin` warning from assert_cmd 2 -- functional, no impact

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Complete CLI: all subcommands (elaborate, validate, test, eval, diff) are fully functional; check/explain/generate remain as stubs (exit 2)
- Integration test infrastructure established -- future CLI features can add tests to cli_integration.rs
- Plan 03-06 (evaluator conformance) can use the eval subcommand and fixture pattern established here

## Self-Check: PASSED

All files verified present. All 2 task commits verified in git log.

---
*Phase: 03-cli-evaluator*
*Completed: 2026-02-21*
