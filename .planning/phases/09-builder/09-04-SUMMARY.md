---
phase: 09-builder
plan: "04"
subsystem: builder-spa
tags: [flow-editor, dag-visualization, system-editor, svg, typescript, react]
dependency_graph:
  requires: [09-03]
  provides: [flow-dag-viz, flow-editor, system-editor]
  affects: [builder/src/App.tsx, builder/src/components/editors/, builder/src/components/visualizations/]
tech_stack:
  added: []
  patterns: [SVG DAG rendering, topological layout, pan/zoom/drag interaction, recursive step editing]
key_files:
  created:
    - builder/src/components/visualizations/FlowDag.tsx
    - builder/src/components/editors/FlowEditor.tsx
    - builder/src/components/editors/SystemEditor.tsx
  modified:
    - builder/src/App.tsx
decisions:
  - "FlowDag uses SVG with viewBox manipulation for pan/zoom — avoids external graph library dependency"
  - "Step detail panel shown as right sidebar when step selected in DAG — mirrors VS Code side panel UX"
  - "StepTargetEditor uses step/terminal mode selector — maps directly to StepTarget discriminated union"
  - "ParallelStep branches rendered as swim lanes within a single node (not full sub-DAGs)"
  - "Validation runs on every render via useMemo — immediate feedback without explicit trigger"
  - "FlowConstruct.steps is array (not Record<string,FlowStep>) — matches actual interchange type"
metrics:
  duration_seconds: 474
  completed_date: "2026-02-27"
  tasks_completed: 5
  files_changed: 4
---

# Phase 9 Plan 4: Flow Editor and System Editor Summary

Interactive flow DAG editor with SVG visualization supporting all five step types, outcome routing, compensation steps, and acyclicity validation; plus system composition editor.

## What Was Built

### FlowDag.tsx (visualizations)
SVG-based interactive flow DAG renderer with:
- **All 5 step type shapes**: OperationStep (blue rectangle), BranchStep (orange diamond), HandoffStep (green pill), SubFlowStep (purple double-border), ParallelStep (swim lane)
- **Terminal nodes**: rounded end caps derived from step targets (success=green, failure=red)
- **Directed edges**: cubic bezier curves with outcome labels; failure edges are dashed red
- **Entry indicator**: filled circle with arrow pointing to entry step
- **Pan/zoom**: SVG viewBox manipulation via mouse drag and scroll wheel
- **Drag to reorder**: node position overrides via mouse drag (editable mode)
- **Highlighted step**: glow/pulse effect for simulation use
- Uses `layoutFlowDag()` and `extractFlowStepInfos()` from `utils/layout.ts` for topological positioning

### FlowEditor.tsx (editors)
Full-featured flow editor with:
- **Flow list sidebar**: add/delete flows, click to select
- **Flow metadata bar**: ID (rename), snapshot mode (at_initiation/live), entry step selector
- **Step toolbar**: Add Step dropdown (5 types), Delete, Set Entry
- **DAG visualization**: embedded FlowDag with step click to select
- **Step detail panel** (right sidebar, shown when step selected):
  - OperationStep: operation/persona dropdowns, outcome routing (label -> step/terminal), failure handler
  - BranchStep: condition (PredicateBuilder), persona, if_true/if_false routing
  - HandoffStep: from_persona, to_persona, next step
  - SubFlowStep: flow selector, persona, on_success/on_failure routing
  - ParallelStep: branch list editor, on_all_success routing
- **FailureHandler editor**: Terminate/Compensate/Escalate with sub-editors (CompensateHandler has ordered steps)
- **StepTargetEditor**: step vs terminal selector for all outcome targets
- **Validation**: entry required, acyclicity (DFS cycle detection), reachability from entry, operation/persona existence

### SystemEditor.tsx (editors)
System composition editor with:
- **System list sidebar**: add/delete systems
- **System detail editor**: ID (rename with duplicate check)
- **MemberListEditor**: contract ID + file path pairs
- **SharedPersonasEditor**: persona + contract multi-checkbox (which contracts share this persona)
- **SharedEntitiesEditor**: entity + contract multi-checkbox
- **TriggersEditor**: cross-contract triggers (on/source_contract/source_flow/target_contract/target_flow/persona)
- **Validation**: empty ID, unknown contract references (warnings), missing data

### App.tsx update
- `/flows` route now renders `FlowEditor`
- `/systems` route now renders `SystemEditor`
- All 9 sidebar sections have functional editors (no more placeholder pages for flows/systems)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed JSX closing tag typo in FlowEditor**
- **Found during:** Task 3 TypeScript compilation
- **Issue:** First draft of `StepTargetEditor` had `</g>` closing tag (SVG) instead of `</div>` (HTML div)
- **Fix:** Corrected to `</div>`, removed duplicate `StepTargetEditorImpl` function
- **Files modified:** `builder/src/components/editors/FlowEditor.tsx`
- **Commit:** e5f91c3

**2. [Rule 1 - Bug] Fixed StepNodeProps.onClick type signature**
- **Found during:** Task 1 TypeScript compilation
- **Issue:** `onClick` in `StepNodeProps` typed as `() => void` but callers pass `(e: React.MouseEvent) => void`
- **Fix:** Changed to `(e: React.MouseEvent) => void` to match actual usage
- **Files modified:** `builder/src/components/visualizations/FlowDag.tsx`
- **Commit:** 241d751

### Plan Divergence (flagged)

The plan described `steps: Record<string, FlowStep>` for FlowConstruct but the actual interchange type in `types/interchange.ts` uses `steps: FlowStep[]` (an array). The implementation uses the actual array type. This is a code-level assumption in the PM's plan that diverged from the codebase.

## Self-Check: PASSED

| Check | Result |
|-------|--------|
| builder/src/components/visualizations/FlowDag.tsx | FOUND |
| builder/src/components/editors/FlowEditor.tsx | FOUND |
| builder/src/components/editors/SystemEditor.tsx | FOUND |
| Commit 241d751 (FlowDag) | FOUND |
| Commit e5f91c3 (FlowEditor) | FOUND |
| Commit fbf9a73 (SystemEditor) | FOUND |
| Commit 3b2ec74 (routing) | FOUND |
| npm run build | PASSED |
| npx tsc --noEmit | PASSED (0 errors) |
