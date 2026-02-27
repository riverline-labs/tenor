---
phase: 07-sdks
plan: 01
subsystem: sdk-typescript
tags: [typescript, sdk, wasm, npm, evaluator]
dependency_graph:
  requires: [crates/tenor-eval-wasm]
  provides: [sdks/typescript]
  affects: []
tech_stack:
  added: [TypeScript 5.x, vitest 1.x, wasm-pack --target nodejs]
  patterns: [CommonJS WASM wrapping, discriminated union types, singleton WASM module]
key_files:
  created:
    - sdks/typescript/package.json
    - sdks/typescript/tsconfig.json
    - sdks/typescript/vitest.config.ts
    - sdks/typescript/scripts/build-wasm.sh
    - sdks/typescript/.gitignore
    - sdks/typescript/src/types.ts
    - sdks/typescript/src/evaluator.ts
    - sdks/typescript/src/action-space.ts
    - sdks/typescript/src/index.ts
    - sdks/typescript/tests/evaluator.test.ts
    - sdks/typescript/tests/action-space.test.ts
    - sdks/typescript/README.md
  modified: []
decisions:
  - "TenorEvaluator uses synchronous fromJson()/fromBundle() (not async) — wasm-pack --target nodejs produces synchronous CommonJS module that loads WASM via readFileSync at require-time"
  - "FactValue uses interface-based recursion (FactRecord, FactList interfaces) to avoid TypeScript circular type alias error"
  - "executeFlowWithBindings() added as bonus method exposing simulate_flow_with_bindings WASM export for multi-instance use cases"
  - "wasm/ directory excluded from git via .gitignore; prepublishOnly script builds it before publish"
metrics:
  duration_seconds: 1393
  completed_date: "2026-02-27"
  tasks_completed: 8
  files_created: 12
---

# Phase 7 Plan 1: TypeScript SDK Summary

TypeScript/JavaScript npm package `@tenor/sdk` wrapping WASM evaluator with `TenorEvaluator` class, full type definitions, and 46 tests proving identical results to the Rust evaluator.

## What Was Built

Created the complete TypeScript SDK at `sdks/typescript/` as an npm package (`@tenor/sdk`). The package wraps the WASM evaluator from `crates/tenor-eval-wasm/` (built with `wasm-pack --target nodejs`) and exposes a TypeScript-first API.

### Key deliverables

- **`TenorEvaluator` class** — synchronous factory methods `fromJson()`/`fromBundle()`, instance methods `evaluate()`, `computeActionSpace()`, `executeFlow()`, `executeFlowWithBindings()`, `inspect()`, and `free()` lifecycle management
- **TypeScript type definitions** — full type coverage of the WASM JSON interface: `FactSet`, `VerdictSet`, `ActionSpace`, `Action`, `BlockedAction`, `BlockedReason`, `FlowResult`, `InspectResult`, `InterchangeBundle`, and more
- **Action-space helpers** — 7 pure helper functions: `actionsForFlow`, `isFlowAvailable`, `isFlowBlocked`, `getBlockReason`, `getBlockedAction`, `availableFlowIds`, `blockedFlowIds`, `hasVerdict`
- **46 tests passing** — 17 evaluator integration tests (mirror `wasm.rs` Rust tests with identical inputs/expectations) + 29 action-space unit tests
- **Build tooling** — `scripts/build-wasm.sh`, `vitest.config.ts`, `tsconfig.json`, `package.json` with proper scripts and `prepublishOnly` lifecycle
- **README** — installation, quick start, full API reference, key types, build from source instructions

## Tests

- `tests/evaluator.test.ts`: 17 tests — load contract, evaluate, executeFlow, computeActionSpace, inspect, free/reuse, and an explicit Rust-parity test (Test 16)
- `tests/action-space.test.ts`: 29 tests — pure unit tests with mock ActionSpace objects, no WASM required

All 46 tests pass. Existing workspace: 96/96 conformance tests pass.

## Deviations from Plan

### Auto-additions (Rule 2)

**1. [Rule 2 - Enhancement] Added executeFlowWithBindings() method**
- **Found during:** Task 3
- **Issue:** The WASM module exports `simulate_flow_with_bindings()` for multi-instance use cases; the plan only mentions `executeFlow()` wrapping `simulate_flow()`
- **Fix:** Added `executeFlowWithBindings()` as an additional method. This is not a deviation from correctness — it exposes the full WASM API surface without removing anything from the plan
- **Files modified:** `sdks/typescript/src/evaluator.ts`

**2. [Rule 1 - Bug] Fixed FactValue circular type alias**
- **Found during:** Task 2
- **Issue:** TypeScript does not allow `type FactValue = ... | Record<string, FactValue> | FactValue[]` as direct recursive type aliases
- **Fix:** Replaced with interface-based recursion: `FactRecord` and `FactList` interfaces that TypeScript resolves without the circular alias error
- **Files modified:** `sdks/typescript/src/types.ts`

**3. [Rule 2 - Enhancement] Added getBlockedAction() helper**
- **Found during:** Task 4
- **Issue:** `getBlockReason()` returns only the `BlockedReason`; callers may also need the full `BlockedAction` (which includes `instance_bindings`)
- **Fix:** Added `getBlockedAction()` returning the full `BlockedAction | undefined`
- **Files modified:** `sdks/typescript/src/action-space.ts`, `sdks/typescript/src/index.ts`

**4. [Rule 2 - Correct API shape] `fromJson`/`fromBundle` made synchronous**
- **Found during:** Task 3
- **Issue:** The plan shows `static async fromBundle()` / `static async fromJson()` returning `Promise<TenorEvaluator>`. The wasm-pack `--target nodejs` build is synchronous (loads WASM via `readFileSync` at `require()` time), so there is no async operation
- **Fix:** Made constructors synchronous — `static fromJson(json: string): TenorEvaluator`. This is strictly better API ergonomics and correct per the actual WASM implementation
- **Files modified:** `sdks/typescript/src/evaluator.ts`

## Self-Check

### Created files exist

All 11 source files verified: FOUND (sdks/typescript/package.json, tsconfig.json, vitest.config.ts, scripts/build-wasm.sh, src/types.ts, src/evaluator.ts, src/action-space.ts, src/index.ts, tests/evaluator.test.ts, tests/action-space.test.ts, README.md)

### Commits exist

All 7 task commits verified: FOUND (7b82e83, 0e132ea, 819018f, 5c292b2, efd7c09, 53d59e8, ca5595b)

## Self-Check: PASSED
