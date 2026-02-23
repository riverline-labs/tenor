---
phase: 18-platform-hardening
plan: 09
subsystem: testing
tags: [lsp, diff, conformance, analysis, s3a, system-contract, escalation, markdown]

# Dependency graph
requires:
  - phase: 18-05
    provides: "LSP foundation (navigation, completion, hover modules)"
  - phase: 18-07
    provides: "diff CLI and explain Markdown format"
provides:
  - "LSP unit test coverage for navigation, completion, and hover"
  - "diff CLI e2e test coverage (4 tests)"
  - "Flow error-path conformance fixture (escalation handling)"
  - "explain Markdown format test assertions"
  - "S3a admissibility negative test cases"
  - "SystemContract coordinator design document"
affects: [25-multi-party-execution]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "LSP testing via ProjectIndex + temp files (no protocol overhead)"
    - "CLI e2e diff tests using serde_json::json! temp bundles"
    - "S3a negative testing pattern: dead states have no admissible operations"

key-files:
  created:
    - "crates/lsp/tests/lsp_tests.rs"
    - "conformance/eval/positive/flow_error_escalate.tenor"
    - "conformance/eval/positive/flow_error_escalate.facts.json"
    - "conformance/eval/positive/flow_error_escalate.verdicts.json"
    - "docs/system-contract-coordinator.md"
  modified:
    - "crates/lsp/Cargo.toml"
    - "crates/cli/tests/cli_integration.rs"
    - "crates/cli/src/explain.rs"
    - "crates/eval/tests/conformance.rs"
    - "crates/analyze/tests/analysis_tests.rs"

key-decisions:
  - "LSP tests call navigation/completion/hover functions directly rather than going through LSP protocol -- faster and more deterministic"
  - "diff CLI tests construct bundles with serde_json::json! macro rather than elaborating .tenor files -- isolates diff logic from elaborator"
  - "SystemContract coordinator design targets Phase 25 with SystemRuntime struct, trigger dispatch, shared entity state, and persona mapping"

patterns-established:
  - "LSP test helper: build_index_from_source() creates temp file, builds ProjectIndex, returns (index, uri, source)"
  - "S3a negative testing: use dead_states.tenor fixture to verify no operations are admissible from unreachable states"

requirements-completed: [HARD-12, HARD-14, HARD-20, HARD-24, HARD-25, HARD-26]

# Metrics
duration: 12min
completed: 2026-02-23
---

# Phase 18 Plan 09: Test Coverage Gaps and SystemContract Design Summary

**22 new tests across LSP, diff CLI, flow evaluation, explain formatting, and S3a analysis plus SystemContract coordinator design document**

## Performance

- **Duration:** 12 min
- **Started:** 2026-02-23T16:15:02Z
- **Completed:** 2026-02-23T16:27:10Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments
- 12 LSP unit tests covering navigation (goto-definition, find-references, document-symbols), completion (keywords, fact names, entity fields), and hover
- 4 diff CLI e2e tests covering fact addition, breaking/non-breaking change detection, and error handling
- Flow error-path conformance fixture exercising FailureHandler::Escalate with 4-step escalation flow
- 2 explain Markdown format tests verifying headings, backtick formatting, tables, and checkbox syntax
- 3 S3a admissibility negative tests verifying dead state detection and finding severity
- SystemContract coordinator design document with architecture, trigger dispatch, shared entity state, persona mapping, and Phase 25 implementation plan

## Task Commits

Each task was committed atomically:

1. **Task 1: LSP unit tests and diff CLI e2e tests** - `d4ff61f` (feat)
2. **Task 2: Flow error fixtures, explain Markdown tests, S3a negatives, SystemContract design** - `935dbe5` (feat)

## Files Created/Modified
- `crates/lsp/tests/lsp_tests.rs` - 12 LSP unit tests (339 lines) for navigation, completion, and hover
- `crates/lsp/Cargo.toml` - Added tempfile and serde_json dev-dependencies
- `crates/cli/tests/cli_integration.rs` - 4 new diff CLI e2e tests (+81 lines)
- `conformance/eval/positive/flow_error_escalate.tenor` - Escalation flow fixture with Document entity
- `conformance/eval/positive/flow_error_escalate.facts.json` - Input facts for escalation test
- `conformance/eval/positive/flow_error_escalate.verdicts.json` - Expected verdicts with 4-step escalation trace
- `crates/eval/tests/conformance.rs` - Added flow_error_escalate test function
- `crates/cli/src/explain.rs` - 2 Markdown format tests with make_rich_bundle() helper (+128 lines)
- `crates/analyze/tests/analysis_tests.rs` - 3 S3a admissibility negative tests (+81 lines)
- `docs/system-contract-coordinator.md` - SystemContract coordinator design document (278 lines)

## Decisions Made
- LSP tests use direct function calls (ProjectIndex, compute_completions, compute_hover) rather than LSP protocol messages -- avoids async server overhead and provides deterministic testing
- diff CLI tests use serde_json::json! macro to construct minimal valid bundles rather than elaborating .tenor files -- isolates diff comparison logic from the elaborator pipeline
- SystemContract coordinator design document targets Phase 25 with a SystemRuntime struct managing contract instances, trigger dispatch with cycle prevention, shared entity state with atomic transitions, and cross-contract persona mapping

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed LSP test line number for goto_definition**
- **Found during:** Task 1 (LSP unit tests)
- **Issue:** goto_definition_fact_ref_resolves_to_declaration test used Position::new(17, 8) but the `when:` keyword was actually on line 18 of the test contract
- **Fix:** Changed position to Position::new(18, 8) to match actual line offset
- **Files modified:** crates/lsp/tests/lsp_tests.rs
- **Verification:** Test passes, goto-definition resolves to fact declaration
- **Committed in:** d4ff61f (part of Task 1 commit)

**2. [Rule 1 - Bug] Fixed deprecated TempDir::into_path() API**
- **Found during:** Task 1 (LSP unit tests)
- **Issue:** tempfile 3.x deprecated `into_path()` in favor of `keep()` -- clippy warning with -D warnings would fail CI
- **Fix:** Changed `dir.into_path()` to `let _ = dir.keep()` throughout LSP tests
- **Files modified:** crates/lsp/tests/lsp_tests.rs
- **Verification:** clippy --workspace -- -D warnings passes cleanly
- **Committed in:** d4ff61f (part of Task 1 commit)

---

**Total deviations:** 2 auto-fixed (2 bugs)
**Impact on plan:** Both fixes necessary for test correctness and CI compliance. No scope creep.

## Issues Encountered
- Flow error escalation fixture required running the evaluator to capture exact expected output format, then writing the .verdicts.json from actual evaluator output. The eval flow runner produces a specific structure with step traces that needed precise matching.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- All 6 test coverage gaps closed (HARD-12, HARD-14, HARD-20, HARD-24, HARD-25, HARD-26)
- Phase 18 (Platform Hardening) fully complete -- all 9 plans executed
- SystemContract coordinator design ready for Phase 25 implementation
- Total test count now exceeds 530+ (22 new tests added)

## Self-Check: PASSED

All 11 files verified present. Both task commits (d4ff61f, 935dbe5) verified in git log. LSP test file meets min_lines requirement (339 lines >= 50).

---
*Phase: 18-platform-hardening*
*Completed: 2026-02-23*
