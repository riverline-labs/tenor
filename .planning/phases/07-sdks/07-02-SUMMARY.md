---
phase: 07-sdks
plan: 02
subsystem: sdk
tags: [python, pyo3, maturin, pyO3, native-extension, abi3, cdylib]

# Dependency graph
requires:
  - phase: 03-eval
    provides: "tenor-eval crate with evaluate, compute_action_space, flow::execute_flow, operation::init_entity_states"
provides:
  - "Python SDK as a native PyO3 extension module built with maturin"
  - "TenorEvaluator class with evaluate(), compute_action_space(), execute_flow()"
  - "Python type stubs (types.py, py.typed) for IDE/mypy support"
  - "22 pytest tests proving cross-SDK consistency with Rust evaluator"
  - "Installable wheel via maturin build"
affects: [phase-07-sdks, platform-integration]

# Tech tracking
tech-stack:
  added: [pyo3 0.23, maturin 1.x, abi3-py39 stable ABI]
  patterns:
    - "PyO3 cdylib with [workspace] table to exclude from root Cargo workspace"
    - "abi3-py39 feature for Python 3.9+ stable ABI compatibility"
    - "module-name = tenor._native in pyproject.toml for clean public API wrapper"
    - "init_entity_states() + overlay pattern for execute_flow entity state initialization"
    - "parse_entity_states() helper for flat/nested entity state format auto-detection"

key-files:
  created:
    - sdks/python/Cargo.toml
    - sdks/python/pyproject.toml
    - sdks/python/.gitignore
    - sdks/python/src/lib.rs
    - sdks/python/src/evaluator.rs
    - sdks/python/src/types.rs
    - sdks/python/python/tenor/__init__.py
    - sdks/python/python/tenor/types.py
    - sdks/python/python/tenor/py.typed
    - sdks/python/tests/test_evaluator.py
    - sdks/python/tests/__init__.py
    - sdks/python/README.md
  modified: []

key-decisions:
  - "pyo3 0.23 with abi3-py39 feature: supports Python 3.9+ via stable ABI (Python 3.14 available on dev machine but pyo3 0.22-0.23 only supports 3.13 without abi3)"
  - "[workspace] table required in Cargo.toml to exclude the PyO3 crate from root workspace (same pattern as tenor-eval-wasm)"
  - "execute_flow uses low-level flow::execute_flow API with init_entity_states() merge, not evaluate_flow() top-level fn — mirrors WASM implementation to handle empty entity states correctly"
  - "bundle field removed from TenorEvaluator struct — not needed after switching to low-level API"
  - "PyO3 0.23 bool conversion: bool::into_pyobject returns Borrowed, must call .to_owned() before .into_any()"

patterns-established:
  - "Python SDK pattern: wrap Rust evaluator as PyO3 cdylib, expose via tenor._native, re-export from tenor package"
  - "Entity state init: always call init_entity_states() then overlay provided states (never pass raw empty map)"

requirements-completed: [SDK-PY-01, SDK-PY-02, SDK-PY-03, SDK-PY-04, SDK-PY-05]

# Metrics
duration: 33min
completed: 2026-02-27
---

# Phase 7 Plan 02: Python SDK Summary

**PyO3 native extension module via maturin with TenorEvaluator, 22 passing pytest tests, and stable abi3-py39 wheel for Python 3.9+**

## Performance

- **Duration:** 33 min
- **Started:** 2026-02-27T18:25:03Z
- **Completed:** 2026-02-27T18:58:20Z
- **Tasks:** 8
- **Files modified:** 12

## Accomplishments

- Built working PyO3 native extension module compiling Rust evaluator directly into Python extension
- Implemented TenorEvaluator with evaluate(), compute_action_space(), and execute_flow() methods
- All 22 pytest tests pass, proving cross-SDK consistency with WASM and Rust evaluator
- maturin build produces installable wheel (abi3, Python 3.9+)
- Workspace tests unaffected (96/96 conformance, all unit tests pass)

## Task Commits

1. **Task 1: Create PyO3 crate structure** - `b5ddf03` (chore)
2. **Task 2: Implement type conversions** - `8e30ed4` (feat)
3. **Task 3: Implement evaluator wrapper** - `a4bdc10` (feat)
4. **Task 4: Create module definition** - `f562271` (feat)
5. **Task 5: Create Python package wrapper** - `1545c5c` (feat)
6. **Task 6: Build PyO3 module** - `ed36b30` (feat)
7. **Task 7: Write pytest test suite** - `850bb2b` (feat)
8. **Task 8: README and maturin build** - `55b33b1` (docs)

## Files Created/Modified

- `sdks/python/Cargo.toml` - PyO3 cdylib crate with abi3-py39 and tenor-eval dependency
- `sdks/python/pyproject.toml` - maturin build config, module-name = tenor._native
- `sdks/python/src/lib.rs` - PyO3 module definition (#[pymodule])
- `sdks/python/src/evaluator.rs` - TenorEvaluator wrapper with 3 methods + parse_entity_states
- `sdks/python/src/types.rs` - json_to_py / py_to_json conversion helpers
- `sdks/python/python/tenor/__init__.py` - Public API re-export from _native
- `sdks/python/python/tenor/types.py` - TypedDict stubs for IDE/mypy support
- `sdks/python/python/tenor/py.typed` - PEP 561 marker
- `sdks/python/tests/test_evaluator.py` - 22 pytest tests
- `sdks/python/README.md` - API reference and build instructions

## Decisions Made

- Used pyo3 0.23 with `abi3-py39` feature to support Python 3.14 on dev machine (pyo3 0.22-0.23 maxes at Python 3.13 without stable ABI mode)
- `[workspace]` table required in `sdks/python/Cargo.toml` to exclude from root workspace (same pattern as `crates/tenor-eval-wasm`)
- `execute_flow()` uses low-level API (`flow::execute_flow` + `operation::init_entity_states`) not the top-level `evaluate_flow()` — this mirrors the WASM implementation and correctly handles empty entity state dicts by initializing from contract defaults
- Removed `bundle: serde_json::Value` field from `TenorEvaluator` struct (not needed with low-level API)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed PyO3 0.22 API mismatch in types.rs**
- **Found during:** Task 6 (Build PyO3 module)
- **Issue:** Plan specified `into_pyobject(py)?.into_any().unbind()` which is PyO3 0.23+ API; PyO3 0.22 has different method names (`into_py`, `PyList::empty_bound`, `PyDict::new_bound`)
- **Fix:** Upgraded to pyo3 0.23 with `abi3-py39` feature; fixed bool conversion to use `PyBool::new(py, b).to_owned()` because `bool::into_pyobject` returns `Borrowed` not `Bound`
- **Files modified:** `sdks/python/Cargo.toml`, `sdks/python/src/types.rs`
- **Committed in:** `ed36b30` (Task 6 commit)

**2. [Rule 3 - Blocking] Added `[workspace]` table to exclude from root workspace**
- **Found during:** Task 6 (Build PyO3 module)
- **Issue:** maturin failed because Cargo thought the crate should be in the root workspace
- **Fix:** Added empty `[workspace]` table to `sdks/python/Cargo.toml`
- **Files modified:** `sdks/python/Cargo.toml`
- **Committed in:** `ed36b30` (Task 6 commit)

**3. [Rule 1 - Bug] Fixed execute_flow entity state initialization**
- **Found during:** Task 7 (Write pytest test suite)
- **Issue:** Passing empty `{}` entity states caused "entity not found" error because `evaluate_flow()` top-level function passes the provided map directly without merging with contract defaults
- **Fix:** Rewrote `execute_flow()` to use lower-level API: call `init_entity_states()` to get contract defaults, then overlay provided states before calling `flow::execute_flow()` — mirrors the WASM implementation
- **Files modified:** `sdks/python/src/evaluator.rs`
- **Committed in:** `850bb2b` (Task 7 commit)

**4. [Rule 3 - Blocking] Removed stale cpython-specific .so file**
- **Found during:** Task 7 (debugging)
- **Issue:** An old `_native.cpython-314-darwin.so` from an early build attempt took precedence over the new `_native.abi3.so`, causing tests to load old code without fixes
- **Fix:** Deleted the old `_native.cpython-314-darwin.so` file; updated `.gitignore` already covers `*.so`
- **Files modified:** (deleted file)
- **Verification:** Debug output confirmed new code was loaded

---

**Total deviations:** 4 auto-fixed (2 blocking, 2 bugs)
**Impact on plan:** All auto-fixes necessary for buildability and correctness. PyO3 version difference from plan was expected (plan written without codebase access). No scope creep.

## Issues Encountered

- Python 3.14 on dev machine is newer than PyO3 0.22/0.23 max supported version (3.13). Resolved with `abi3-py39` stable ABI feature which works across all Python 3.9+ versions including 3.14.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Python SDK complete and installable via `maturin develop` or `maturin build`
- Type stubs provide IDE/mypy support
- 22 tests prove cross-SDK consistency
- Ready for PyPI publishing once project reaches that stage

## Self-Check: PASSED

All created files verified on disk. All task commits verified in git history.
