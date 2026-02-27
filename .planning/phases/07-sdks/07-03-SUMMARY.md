---
phase: 07-sdks
plan: 03
subsystem: sdk
tags: [go, wazero, wasm, wasip1, cabi, evaluator]

# Dependency graph
requires:
  - phase: 04-multi-instance
    provides: EntityStateMap composite key, InstanceBindingMap, simulate_flow_with_bindings
  - phase: 05-trust
    provides: WASM evaluator crate (tenor-eval-wasm)
  - phase: 07-sdks-01
    provides: WASM module interface patterns (alloc/dealloc)
provides:
  - Go SDK module github.com/riverline-labs/tenor-go
  - wasm-bridge Rust crate (wasm32-wasip1, C-ABI exports)
  - wazero runtime wrapper with alloc/dealloc memory protocol
  - Go Evaluator with Evaluate, ComputeActionSpace, ExecuteFlow, ExecuteFlowWithBindings
  - 17 passing Go tests proving identical results to Rust evaluator
affects: [07-sdks-04, ci, distribution]

# Tech tracking
tech-stack:
  added:
    - wazero v1.9.0 (pure-Go WASM runtime)
    - Go module github.com/riverline-labs/tenor-go
    - wasm32-wasip1 target (Rust crate sdks/go/wasm-bridge)
  patterns:
    - WASM bridge with WASI target (not wasm-bindgen) for wazero compatibility
    - alloc/dealloc/get_result_ptr/get_result_len memory protocol for string passing
    - go:embed for zero-dependency WASM binary distribution
    - wasi_snapshot_preview1.MustInstantiate before module load in wazero
    - WithStartFunctions() to skip _start for library-style WASM modules

key-files:
  created:
    - sdks/go/tenor.go
    - sdks/go/types.go
    - sdks/go/tenor_test.go
    - sdks/go/go.mod
    - sdks/go/go.sum
    - sdks/go/internal/wasm/runtime.go
    - sdks/go/scripts/build-wasm.sh
    - sdks/go/README.md
    - sdks/go/wasm-bridge/Cargo.toml
    - sdks/go/wasm-bridge/src/lib.rs
    - sdks/go/.gitignore
  modified: []

key-decisions:
  - "Use wasm32-wasip1 target (not wasm32-unknown-unknown) to avoid wasm-bindgen/js-sys imports that wazero cannot satisfy"
  - "wasi_snapshot_preview1.MustInstantiate(ctx, r) must precede module load for WASI imports"
  - "WithStartFunctions() empty call prevents wazero calling _start on library-style WASM"
  - "Bridge crate has [workspace] in Cargo.toml to exclude from root workspace (same pattern as tenor-eval-wasm)"
  - "alloc/dealloc/get_result_ptr/get_result_len C-ABI protocol mirrors ptr/len pattern for passing strings without JS glue"
  - "EntityStateMapNested and ComputeActionSpaceNested expose multi-instance support to Go callers"

patterns-established:
  - "WASM bridge pattern: Rust C-ABI cdylib -> wasm32-wasip1 -> embedded via go:embed -> wazero"
  - "Per-arity CallHandleN methods (one, three, four, five args) for type-safe WASM calls"
  - "extractError() helper checks JSON error field before deserializing success response"

requirements-completed: [SDK-GO-01, SDK-GO-02, SDK-GO-03, SDK-GO-04, SDK-GO-05]

# Metrics
duration: 28min
completed: 2026-02-27
---

# Phase 7 Plan 03: Go SDK Summary

**Go SDK for Tenor using wazero WASM runtime: pure-Go Evaluator with 17 passing tests and zero CGo/native dependencies**

## Performance

- **Duration:** 28 min
- **Started:** 2026-02-27T18:25:07Z
- **Completed:** 2026-02-27T18:53:22Z
- **Tasks:** 8
- **Files modified:** 11

## Accomplishments

- Go module `github.com/riverline-labs/tenor-go` with Evaluate, ComputeActionSpace, ExecuteFlow, ExecuteFlowWithBindings
- Rust WASM bridge crate (`wasm32-wasip1`) with C-ABI exports (alloc/dealloc/get_result_ptr/get_result_len protocol)
- wazero v1.9.0 runtime wrapper with WASI support — no CGo, no native toolchain at runtime
- 17 Go tests all passing, including TestResultsMatchRustEvaluator proving identical outputs
- Multi-instance support via `ComputeActionSpaceNested` and `ExecuteFlowWithBindings`
- WASM binary embedded via `go:embed` — single-binary distribution

## Task Commits

Each task was committed atomically:

1. **Task 1: Go module structure and WASM build script** - `895b78c` (chore)
2. **Task 2: WASM bridge crate (C-ABI exports)** - `869488d` (feat)
3. **Task 3: Go type definitions** - `0cbf2eb` (feat)
4. **Task 4: wazero runtime wrapper** - `b86dadf` (feat)
5. **Task 5: Go Evaluator API** - `59b9b29` (feat)
6. **Task 6: Build WASM bridge, verify compilation** - `b164fc0` (chore)
7. **Task 7: Go test suite** - `a4a020e` (feat)
8. **Task 8: README and final verification** - `9df9085` (docs)

## Files Created/Modified

- `sdks/go/tenor.go` - Evaluator struct with Evaluate, ComputeActionSpace, ExecuteFlow methods
- `sdks/go/types.go` - Go type definitions: FactSet, ActionSpace, FlowResult, etc.
- `sdks/go/tenor_test.go` - 17 tests proving identical results to Rust evaluator
- `sdks/go/go.mod` - Go module github.com/riverline-labs/tenor-go with wazero v1.9.0
- `sdks/go/internal/wasm/runtime.go` - wazero WASM runtime wrapper with alloc/dealloc protocol
- `sdks/go/scripts/build-wasm.sh` - Build script for wasm32-wasip1 bridge
- `sdks/go/README.md` - API reference, quick start, architecture notes
- `sdks/go/wasm-bridge/Cargo.toml` - Rust bridge crate (wasm32-wasip1)
- `sdks/go/wasm-bridge/src/lib.rs` - C-ABI exports: load_contract, evaluate, compute_action_space, simulate_flow
- `sdks/go/.gitignore` - Exclude WASM binary artifacts from VCS

## Decisions Made

- Use `wasm32-wasip1` instead of `wasm32-unknown-unknown`: the unknown target with `getrandom/js` pulls in `wasm-bindgen` which creates `__wbindgen_placeholder__` imports that wazero cannot satisfy. WASIP1 uses native WASI syscalls that wazero implements natively.
- Call `wasi_snapshot_preview1.MustInstantiate(ctx, r)` before module instantiation to satisfy WASI imports
- Use `WithStartFunctions()` (empty) in `InstantiateWithConfig` to prevent wazero from calling `_start`, since the bridge is a library, not a CLI program
- Bridge crate uses `[workspace]` in Cargo.toml to exclude it from the root workspace (same pattern as `tenor-eval-wasm`)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Switch from wasm32-unknown-unknown to wasm32-wasip1**
- **Found during:** Task 6 (build WASM bridge, verify Go compilation)
- **Issue:** `wasm32-unknown-unknown` + `getrandom/js` feature pulls in `wasm-bindgen`/`js-sys`, which emits `__wbindgen_placeholder__` imports that wazero cannot satisfy at runtime
- **Fix:** Changed target to `wasm32-wasip1`, removed `getrandom = { features = ["js"] }`, added `wasi_snapshot_preview1.MustInstantiate` in Go runtime.go, used `WithStartFunctions()` to skip _start
- **Files modified:** `sdks/go/wasm-bridge/Cargo.toml`, `sdks/go/internal/wasm/runtime.go`, `sdks/go/scripts/build-wasm.sh`
- **Verification:** `go test -v ./...` — all 17 tests pass
- **Committed in:** `a4a020e` (Task 7 commit, included build script + runtime fixes)

---

**Total deviations:** 1 auto-fixed (Rule 1 — bug in WASM target selection)
**Impact on plan:** Necessary correctness fix. The plan specified "wasm32-unknown-unknown" but this target's interaction with wasm-bindgen via getrandom/js makes it incompatible with wazero. wasm32-wasip1 is the correct target for wazero consumption.

## Issues Encountered

- wasm32-unknown-unknown + getrandom/js creates browser-specific wasm-bindgen imports incompatible with wazero. Fixed by switching to wasm32-wasip1 (WASI-based target).

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Go SDK complete: 17 tests passing, `go build ./...` clean, `go vet ./...` clean
- WASM binary embedded — no Rust toolchain needed by Go users at runtime
- Ready for Phase 7 Plan 04 (final SDK integration/distribution)
- Existing workspace: 96/96 conformance, all workspace tests pass, clippy clean

---
*Phase: 07-sdks*
*Completed: 2026-02-27*
