---
phase: 07-sdks
plan: 04
subsystem: testing
tags: [conformance, cross-sdk, wasm, rust, typescript, python, go, json-fixtures]

# Dependency graph
requires:
  - phase: 07-01
    provides: TypeScript SDK (TenorEvaluator wrapping WASM)
  - phase: 07-02
    provides: Python SDK (PyO3 native extension)
  - phase: 07-03
    provides: Go SDK (wazero WASM runtime)

provides:
  - Cross-SDK conformance suite in sdks/conformance/
  - Fixture generator (Rust binary) producing canonical expected JSON from Rust evaluator
  - Per-SDK runner scripts (TypeScript, Python, Go)
  - run-all.sh master script reporting ALL SDKs CONFORM
  - README.md documenting the suite

affects: [08-publishing, future-sdk-additions]

# Tech tracking
tech-stack:
  added:
    - tenor-conformance-gen (standalone Rust binary in sdks/conformance/fixture-gen/)
    - tsx (via npx for TypeScript runner)
  patterns:
    - Fixture-driven conformance: canonical expected output generated from source-of-truth Rust evaluator
    - JSON sorted-keys comparison for deterministic cross-language equality checks
    - WASM output format is canonical: all SDKs must match exactly including null/empty field handling

key-files:
  created:
    - sdks/conformance/fixtures/escrow-bundle.json
    - sdks/conformance/fixtures/escrow-facts.json
    - sdks/conformance/fixtures/escrow-entity-states.json
    - sdks/conformance/fixtures/escrow-facts-inactive.json
    - sdks/conformance/fixtures/expected-verdicts.json
    - sdks/conformance/fixtures/expected-verdicts-inactive.json
    - sdks/conformance/fixtures/expected-action-space.json
    - sdks/conformance/fixtures/expected-action-space-blocked.json
    - sdks/conformance/fixtures/expected-flow-result.json
    - sdks/conformance/fixture-gen/Cargo.toml
    - sdks/conformance/fixture-gen/src/main.rs
    - sdks/conformance/generate-fixtures.sh
    - sdks/conformance/run-all.sh
    - sdks/conformance/run-typescript.sh
    - sdks/conformance/run-python.sh
    - sdks/conformance/run-go.sh
    - sdks/conformance/runners/typescript-runner.ts
    - sdks/conformance/runners/python_runner.py
    - sdks/conformance/runners/go-runner/main.go
    - sdks/conformance/runners/go-runner/go.mod
    - sdks/conformance/README.md
  modified:
    - sdks/python/src/evaluator.rs (bug fix: add simulation/instance_bindings to flow result)
    - sdks/go/types.go (bug fix: remove omitempty from always-present fields)

key-decisions:
  - "WASM simulate_flow output format is canonical; Python SDK execute_flow must match exactly (simulation:true, instance_bindings:{})"
  - "Go SDK omitempty removed from VerdictProvenance.VerdictsUsed, BlockedAction.InstanceBindings, FlowResult.InstanceBindings — Rust always emits these fields even when empty"
  - "fixture-gen Cargo.toml uses [workspace] table to exclude from root workspace (same pattern as Python SDK)"
  - "Python runner uses .venv if available (maturin develop), falls back to PYTHONPATH for pre-built .so"
  - "Go runner is a standalone module (go.mod with replace directive) to avoid adding test code to the SDK module"

requirements-completed: [SDK-CONF-01, SDK-CONF-02, SDK-CONF-03, SDK-CONF-04, SDK-CONF-05]

# Metrics
duration: 11min
completed: 2026-02-27
---

# Phase 7 Plan 4: Cross-SDK Conformance Suite Summary

**Rust-driven fixture suite verifying TypeScript, Python, and Go SDKs produce identical JSON output to the reference evaluator for evaluate, compute_action_space, and execute_flow**

## Performance

- **Duration:** 11 min
- **Started:** 2026-02-27T14:03:43Z
- **Completed:** 2026-02-27T14:14:47Z
- **Tasks:** 6
- **Files modified:** 23

## Accomplishments

- Created fixture generator (Rust binary) that produces canonical expected JSON directly from the `tenor-eval` crate — the unambiguous source of truth
- All 5 conformance tests pass for all three SDKs: evaluate (active), evaluate (inactive), compute_action_space, compute_action_space (blocked), execute_flow
- Fixed two bugs found during execution: Python SDK missing `simulation`/`instance_bindings` fields; Go SDK omitting empty-but-always-present fields via `omitempty`
- `run-all.sh` exits 0 with "ALL SDKs CONFORM" on current codebase

## Task Commits

1. **Task 1: Create conformance fixture bundle** - `76ce00e` (feat)
2. **Task 2: Generate expected output fixtures from Rust evaluator** - `d7470ac` (feat)
3. **Task 3: Create per-SDK conformance runner scripts** - `9100d2f` (feat)
   - Bug fixes included: `e8ea801` (fix: Python and Go output format discrepancies)
4. **Task 4: Create run-all.sh master script** - `42197c0` (feat)
5. **Task 5: Run conformance suite and fix discrepancies** - (no additional commit needed; fixes in Task 3)
6. **Task 6: Write README and final verification** - `8b4e261` (docs)

## Files Created/Modified

- `sdks/conformance/fixtures/` - 9 JSON fixture files (4 input, 5 expected output)
- `sdks/conformance/fixture-gen/` - Rust binary generating expected output from `tenor-eval`
- `sdks/conformance/generate-fixtures.sh` - Regenerate fixtures script
- `sdks/conformance/run-all.sh` - Master conformance runner
- `sdks/conformance/run-{typescript,python,go}.sh` - Per-SDK runners
- `sdks/conformance/runners/` - TypeScript, Python, Go runner scripts
- `sdks/conformance/README.md` - Documentation
- `sdks/python/src/evaluator.rs` - Bug fix: execute_flow output format
- `sdks/go/types.go` - Bug fix: omitempty on always-present fields

## Decisions Made

- The WASM module's `simulate_flow_with_bindings` output is the canonical flow result format: always includes `simulation: true` and `instance_bindings: {}` even when empty. Python SDK fixed to match.
- Go SDK's `omitempty` tags were incorrect for fields that Rust always serializes (even when empty slices/maps). Removed omitempty from `VerdictsUsed`, `BlockedAction.InstanceBindings`, `FlowResult.InstanceBindings`.
- The fixture generator uses a `[workspace]` stub in its Cargo.toml to avoid workspace membership conflicts (same pattern as the Python SDK).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Python SDK execute_flow missing simulation and instance_bindings fields**
- **Found during:** Task 3 (creating Python runner) / Task 5 (running conformance)
- **Issue:** Python SDK's `execute_flow` output was missing `"simulation": true` and `"instance_bindings": {}` that the WASM-based SDKs (TypeScript, Go) always include
- **Fix:** Updated `sdks/python/src/evaluator.rs` to include both fields and per-step instance_bindings when non-empty, matching the WASM `simulate_flow_with_bindings` output exactly
- **Files modified:** `sdks/python/src/evaluator.rs`
- **Verification:** Python runner 5/5 pass; existing 22 pytest tests still pass
- **Committed in:** `e8ea801`

**2. [Rule 1 - Bug] Go SDK omitempty on always-present JSON fields causes mismatch**
- **Found during:** Task 5 (running conformance)
- **Issue:** Three fields in Go types used `omitempty` but Rust always emits them even when empty: `VerdictProvenance.VerdictsUsed` (`"verdicts_used": []`), `BlockedAction.InstanceBindings` (`"instance_bindings": {}`), `FlowResult.InstanceBindings` (`"instance_bindings": {}`)
- **Fix:** Removed `omitempty` from all three fields in `sdks/go/types.go`
- **Files modified:** `sdks/go/types.go`
- **Verification:** Go runner 5/5 pass; existing 14 Go tests still pass
- **Committed in:** `e8ea801`

---

**Total deviations:** 2 auto-fixed (both Rule 1 - Bug)
**Impact on plan:** Both fixes were correctness bugs; SDK output now identical to Rust evaluator. No scope creep.

## Issues Encountered

- maturin not installed on dev machine; installed via `pip install maturin --break-system-packages` to rebuild Python SDK after fix. This created a `.venv` at `sdks/python/.venv/` which the `run-python.sh` script now detects and uses automatically.
- fixture-gen Cargo.toml needed `[workspace]` stub to avoid workspace conflict (same issue as the Python SDK in 07-02).

## Next Phase Readiness

- All three SDKs pass cross-SDK conformance
- Conformance suite ready to run as a CI check: `cd sdks/conformance && ./run-all.sh`
- Fixtures can be regenerated after evaluator changes via `./generate-fixtures.sh`
- Phase 7 (SDKs) is complete: TypeScript + Python + Go SDKs + conformance suite all done

---
*Phase: 07-sdks*
*Completed: 2026-02-27*
