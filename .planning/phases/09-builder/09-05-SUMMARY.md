---
phase: 09-builder
plan: 05
subsystem: ui
tags: [react, zustand, wasm, simulation, evaluation]

# Dependency graph
requires:
  - phase: 09-04
    provides: FlowDag visualization component with highlightedStep prop
  - phase: 09-01
    provides: simulation store skeleton, WASM evaluator API, contract/elaboration stores

provides:
  - SimulationPage with three-tab layout (Evaluate / Actions / Flows)
  - FactInputPanel with type-appropriate controls for all Tenor base types
  - VerdictPanel with stratum grouping and provenance drill-down
  - ActionSpacePanel with available/blocked/unauthorized sections per persona
  - FlowRunner with step-by-step DAG highlighting and entity transition tracking
  - ProvenanceView modal showing rule chain and fact dependencies
  - Enhanced simulation store with initFromContract, stepFlowForward, isEvaluating

affects:
  - future simulation UI enhancements
  - Phase 10 (any embedding of simulation into external workflows)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - Client-side replay: full WASM simulation stored upfront, steps revealed via stepFlowForward()
    - Type-dispatch input rendering: switch on type.base for correct control selection
    - Stratum grouping: verdicts grouped by rule stratum using contract store lookup
    - Debounced auto-evaluate: 600ms debounce on factValues change when toggle enabled

key-files:
  created:
    - builder/src/components/simulation/SimulationPage.tsx
    - builder/src/components/simulation/FactInputPanel.tsx
    - builder/src/components/simulation/VerdictPanel.tsx
    - builder/src/components/simulation/ActionSpacePanel.tsx
    - builder/src/components/simulation/FlowRunner.tsx
    - builder/src/components/simulation/ProvenanceView.tsx
  modified:
    - builder/src/store/simulation.ts
    - builder/src/App.tsx

key-decisions:
  - "Client-side step replay: WASM simulate_flow() runs full simulation at once; stepFlowForward() advances playback index — no per-step WASM calls needed"
  - "ProvenanceView created before VerdictPanel to satisfy TypeScript import order"
  - "FactInputPanel derives defaults from fact.type.base when no explicit default declared"
  - "ActionSpacePanel computes unauthorized ops client-side by diffing persona allowed_personas vs action space result"

patterns-established:
  - "TypeInput: recursive type-dispatch component for all 11 Tenor base types including nested List/Record"
  - "ProvenanceNode: recursive tree rendering with depth guard at 4 levels"
  - "SimulationPage: three-tab shell pattern for multi-mode simulation"

requirements-completed: []

# Metrics
duration: 7min
completed: 2026-02-27
---

# Phase 9 Plan 5: Simulation Mode Summary

**WASM-powered simulation UI with fact input panel, stratum-organized verdicts, action space display, step-by-step flow runner with DAG highlighting, and recursive provenance drill-down — all running in the browser with no server.**

## Performance

- **Duration:** 7 min
- **Started:** 2026-02-27T~22:53Z
- **Completed:** 2026-02-27T~23:00Z
- **Tasks:** 7
- **Files modified:** 8 (6 created, 2 modified)

## Accomplishments
- Full simulation mode wired at /simulation with three functional tabs (Evaluate, Actions, Flows)
- FactInputPanel covers all 11 Tenor base types including recursive List/Record editors
- VerdictPanel groups verdicts by rule stratum with green/red indicators and provenance drill-down
- FlowRunner uses client-side replay for step-by-step DAG highlighting without extra WASM calls
- ProvenanceView recursively traces verdict production through rule chains to source facts

## Task Commits

Each task was committed atomically:

1. **Task 1: Enhance simulation store** - `1f788e0` (feat)
2. **Task 2: Implement fact input panel** - `d669154` (feat)
3. **Task 3: Implement verdict panel** - `18a6618` (feat)
4. **Task 6: Implement provenance view** - `d23e281` (feat) *(created first to satisfy Task 3 import)*
5. **Task 4: Implement action space panel** - `dd28410` (feat)
6. **Task 5: Implement flow runner** - `9425a88` (feat)
7. **Task 7: Create simulation page + routing** - `cfd88e7` (feat)

## Files Created/Modified
- `builder/src/store/simulation.ts` - Enhanced: initFromContract, stepFlowForward/resetFlowPlayback, isEvaluating, EntityStateChange/StepResult types
- `builder/src/components/simulation/FactInputPanel.tsx` - TypeInput dispatcher for all 11 base types, entity state overrides
- `builder/src/components/simulation/VerdictPanel.tsx` - Stratum-grouped verdict display, auto-evaluate toggle, provenance trigger
- `builder/src/components/simulation/ActionSpacePanel.tsx` - Persona selector, available/blocked/unauthorized sections, simulate button
- `builder/src/components/simulation/FlowRunner.tsx` - Flow + persona selector, Step/Run to End/Reset controls, DAG + step history
- `builder/src/components/simulation/ProvenanceView.tsx` - Recursive tree: verdict -> rule -> facts/verdict_refs
- `builder/src/components/simulation/SimulationPage.tsx` - Three-tab shell, contract status bar, Init Simulation button
- `builder/src/App.tsx` - /simulation route now uses SimulationPage

## Decisions Made
- Client-side step replay: `simulateFlow()` runs full WASM simulation upfront, stores result in `fullResult`; `stepFlowForward()` advances the displayed slice. No per-step WASM calls needed. This keeps the UI responsive and avoids stateful server calls.
- ProvenanceView was created during Task 3 processing (before its plan position at Task 6) because VerdictPanel imports it. Committed as a separate atomic commit.
- `FactInputPanel` derives sensible zero-defaults from type declarations when no explicit `fact.default` is set (e.g., `""` for Text, `false` for Bool, `[]` for List).
- ActionSpacePanel computes "unauthorized" operations client-side by querying `allowed_personas` in the contract store and diffing against the WASM action space result.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed TypeScript cast error in ProvenanceView**
- **Found during:** Task 3/6 (VerdictPanel + ProvenanceView TypeScript check)
- **Issue:** `expr as Record<string, unknown>` fails because `VerdictPresentExpr` lacks an index signature — TypeScript won't allow the direct cast
- **Fix:** Changed to `expr as unknown as Record<string, unknown>` (double-cast via unknown)
- **Files modified:** builder/src/components/simulation/ProvenanceView.tsx
- **Verification:** `npx tsc --noEmit` passes
- **Committed in:** d23e281 (Task 6 commit)

**2. [Rule 1 - Bug] Fixed null assignability in ActionSpacePanel**
- **Found during:** Task 4 (ActionSpacePanel TypeScript check)
- **Issue:** `persona={selectedPersona}` passes `string | null` to `AvailableCard.persona: string` prop
- **Fix:** Changed to `persona={selectedPersona ?? ""}`
- **Files modified:** builder/src/components/simulation/ActionSpacePanel.tsx
- **Verification:** `npx tsc --noEmit` passes
- **Committed in:** dd28410 (Task 4 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 — TypeScript type bugs)
**Impact on plan:** Both fixes required for TypeScript correctness. No scope creep.

## Issues Encountered
- Task 3 (VerdictPanel) imports ProvenanceView; created Task 6 (ProvenanceView) ahead of its plan sequence and committed separately. This follows natural dependency order.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All 5 simulation panels implemented and wired into the /simulation route
- Production build succeeds with `npm run build`
- TypeScript clean with `npx tsc --noEmit`
- Phase 9 Plan 6 can proceed (if any) — simulation foundation is complete

## Self-Check: PASSED

All 8 key files verified present. All 7 task commits verified in git log.

---
*Phase: 09-builder*
*Completed: 2026-02-27*
