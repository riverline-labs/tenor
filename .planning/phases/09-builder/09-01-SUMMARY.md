---
phase: 09-builder
plan: 01
subsystem: builder
tags: [react, vite, typescript, wasm, zustand, tailwind, dsl-generator]
dependency_graph:
  requires: [crates/tenor-eval-wasm]
  provides: [builder/src/wasm/pkg, builder/src/store, builder/src/utils, builder/src/types, builder/src/components]
  affects: []
tech_stack:
  added: [React 18, React Router 6, Zustand 5, zundo 2, Tailwind CSS 3, Vite 5, vite-plugin-wasm, vite-plugin-top-level-await, TypeScript 5]
  patterns: [interchange-JSON-as-internal-model, WASM-lazy-init, zustand-temporal-undo-redo, force-directed-layout, topological-sort-DAG]
key_files:
  created:
    - builder/package.json
    - builder/vite.config.ts
    - builder/tsconfig.json
    - builder/tsconfig.node.json
    - builder/index.html
    - builder/src/main.tsx
    - builder/src/App.tsx
    - builder/src/vite-env.d.ts
    - builder/src/types/interchange.ts
    - builder/src/wasm/evaluator.ts
    - builder/src/wasm/elaborator.ts
    - builder/src/wasm/pkg/ (WASM build artifacts)
    - builder/src/store/contract.ts
    - builder/src/store/elaboration.ts
    - builder/src/store/simulation.ts
    - builder/src/utils/dsl-generator.ts
    - builder/src/utils/layout.ts
    - builder/src/components/Layout.tsx
    - builder/src/components/ContractOverview.tsx
    - builder/src/components/shared/ErrorPanel.tsx
    - builder/src/components/shared/TypePicker.tsx
  modified: []
decisions:
  - Internal model is always interchange JSON (InterchangeBundle); DSL generated only on export
  - WASM pkg committed via git force-add (wasm-pack generates .gitignore with * that blocks tracking)
  - zundo added for undo/redo (zustand temporal middleware, 50-state history)
  - Simulation store reads contractHandle from elaboration store rather than managing WASM state independently
  - TypePicker uses max depth=4 guard for recursive Record/TaggedUnion/List types
  - App.tsx uses placeholder pages for editors — full editors implemented in plans 02-07
metrics:
  duration_seconds: 746
  tasks_completed: 10
  files_created: 21
  completed_date: 2026-02-27
---

# Phase 9 Plan 01: Builder SPA Scaffold Summary

Scaffolded the Tenor Builder React SPA with Vite + TypeScript + Tailwind, integrated the WASM evaluator for client-side contract evaluation, established Zustand state management using interchange JSON as the internal model, and implemented the DSL generator that converts the model to .tenor source at export time.

## Tasks Completed

| Task | Name | Commit |
|------|------|--------|
| 1 | Initialize Vite + React + TypeScript project | 1001db1 |
| 2 | Define TypeScript interchange types | b1e7d71 |
| 3 | Build WASM evaluator and create wrapper | f30912d + b697c34 |
| 4 | Create elaborator shim | fe1fbbd |
| 5 | Implement Zustand contract store | c786bce |
| 6 | Implement elaboration store | 1dc3490 |
| 7 | Implement simulation store | 53d26ed |
| 8 | Implement DSL generator | ecc8564 |
| 9 | Implement graph layout utilities | c79aea6 |
| 10 | Create application shell components | 480b8e5 |

## What Was Built

### Project Scaffold (Task 1)
- Vite 5 project with `@vitejs/plugin-react`, `vite-plugin-wasm`, `vite-plugin-top-level-await`
- TypeScript strict mode with `@` path alias to `src/`
- Tailwind CSS 3 configured via PostCSS
- `npm run build` produces optimized production bundle

### TypeScript Interchange Types (Task 2)
- Complete mirror of `docs/interchange-schema.json` as TypeScript types
- `InterchangeBundle`, `InterchangeConstruct` discriminated union on `kind`
- All 12 `BaseType` variants, full `PredicateExpression` type tree
- All `FlowStep` types (`OperationStep`, `BranchStep`, `HandoffStep`, `SubFlowStep`, `ParallelStep`)
- Type guard functions (`isFact`, `isEntity`, `isRule`, etc.)

### WASM Integration (Task 3)
- Built `tenor-eval-wasm` with `wasm-pack --target web` into `builder/src/wasm/pkg/`
- `evaluator.ts` wraps all WASM exports with typed TypeScript API
- Lazy async initialization via `initEvaluator()`
- All error responses `{ error: string }` converted to JavaScript exceptions
- WASM pkg force-committed past wasm-pack's generated `.gitignore`

### Elaborator Shim (Task 4)
- `quickValidate()`: synchronous structural checks (duplicate IDs, entity state refs, rule strata, operation persona refs, flow step refs)
- `validateBundle()`: WASM-based loading after quick checks pass
- `ValidationError` type with severity `"error" | "warning"`

### Zustand Stores (Tasks 5-7)
- **contract.ts**: `useContractStore` with `initContract`, `loadBundle`, CRUD (`addConstruct`, `updateConstruct`, `removeConstruct`), typed selectors, undo/redo via `zundo` temporal middleware (50-state history)
- **elaboration.ts**: `useElaborationStore` tracking WASM readiness, contract handle, validation errors, two-phase validation pipeline
- **simulation.ts**: `useSimulationStore` with fact value inputs, entity state overrides, evaluation results (`verdicts`, `actionSpace`, `flowExecution`)

### DSL Generator (Task 8)
- `generateDsl(bundle)` produces valid `.tenor` from any `InterchangeBundle`
- Canonical construct ordering per `pass6_serialize.rs`
- Full predicate expression formatting with unicode operators (`∧`, `∨`, `¬`, `∀`, `∃`)
- All flow step types with correct DSL syntax matching `integration_escrow.tenor`
- Lowercase keywords throughout per `CLAUDE.md`

### Graph Layout Utilities (Task 9)
- `layoutStateMachine()`: force-directed spring model (50 iterations), initial state anchored top-left
- `layoutFlowDag()`: topological sort + layered columns, BFS from entry step
- `extractFlowStepInfos()`: extracts outgoing step IDs from `FlowStep[]`
- `LayoutNode`, `LayoutEdge`, `LayoutResult` exported types

### Application Shell (Task 10)
- **App.tsx**: `BrowserRouter` with routes for all 9 sections + simulation, WASM init on mount
- **Layout.tsx**: sidebar with `NavLink` items + construct counts, toolbar with undo/redo + import/export
- **ContractOverview.tsx**: dashboard showing construct counts per kind, validation/WASM status badges, quick-add links
- **ErrorPanel.tsx**: collapsible bottom panel, click-to-navigate by construct kind
- **TypePicker.tsx**: recursive type selector for all `BaseType` variants with parameterized sub-fields

## Deviations from Plan

None — plan executed exactly as written.

## Verification

1. `cd builder && npm install` — all dependencies install cleanly
2. `cd builder && npm run build` — production build succeeds (480KB WASM + 347KB JS)
3. `cd builder && npx tsc --noEmit` — no TypeScript errors
4. WASM pkg exists at `builder/src/wasm/pkg/` with `.wasm` and `.js` files
5. App shell renders with sidebar navigation (manual — dev server available via `npm run dev`)
6. WASM initialization logged to browser console (manual verification)

## Self-Check: PASSED

All key files exist:
- builder/package.json — FOUND
- builder/src/store/contract.ts — FOUND
- builder/src/wasm/evaluator.ts — FOUND
- builder/src/types/interchange.ts — FOUND
- builder/src/utils/dsl-generator.ts — FOUND
- builder/src/wasm/pkg/tenor_eval_wasm_bg.wasm — FOUND
- builder/src/wasm/pkg/tenor_eval_wasm.js — FOUND

All task commits exist:
- 1001db1 Task 1: Vite project init — FOUND
- b1e7d71 Task 2: TypeScript interchange types — FOUND
- f30912d Task 3: WASM evaluator wrapper — FOUND
- b697c34 Task 3b: WASM pkg artifacts — FOUND
- fe1fbbd Task 4: Elaborator shim — FOUND
- c786bce Task 5: Contract store — FOUND
- 1dc3490 Task 6: Elaboration store — FOUND
- 53d26ed Task 7: Simulation store — FOUND
- ecc8564 Task 8: DSL generator — FOUND
- c79aea6 Task 9: Layout utilities — FOUND
- 480b8e5 Task 10: Application shell — FOUND
