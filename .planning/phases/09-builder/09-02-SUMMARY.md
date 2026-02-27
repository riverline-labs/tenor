---
phase: 09-builder
plan: 02
subsystem: ui
tags: [react, typescript, svg, zustand, state-machine, interchange]

# Dependency graph
requires:
  - phase: 09-01
    provides: Zustand contract store, interchange types, layout utilities, TypePicker component, app shell
provides:
  - Interactive SVG state machine visualization with pan/zoom/drag
  - Entity editor with full CRUD for states, transitions, initial state
  - Fact editor with all BaseType variants and type-correct default values
  - Persona editor with usage tracking (operation references)
  - Source editor with protocol and field mapping
  - All editors wired into app routing (/entities, /facts, /personas, /sources)
affects: [09-03, 09-04, 09-05]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - SVG state machine: force-directed layout + per-node drag overrides; viewport pan/zoom via transform group
    - Add-transition mode: two-click flow via local React state (addMode + addSource)
    - FactDefault serialization: type-appropriate interchange JSON (bool_literal, decimal_value, money_value, string)
    - Rename pattern: remove old ID + addConstruct new ID (since updateConstruct matches on id+kind pair)

key-files:
  created:
    - builder/src/components/visualizations/StateMachine.tsx
    - builder/src/components/editors/EntityEditor.tsx
    - builder/src/components/editors/FactEditor.tsx
    - builder/src/components/editors/PersonaEditor.tsx
    - builder/src/components/editors/SourceEditor.tsx
  modified:
    - builder/src/App.tsx

key-decisions:
  - "StateMachine nodePosOverrides reset on states/transitions changes to prevent stale drag positions"
  - "FactDefault for Text/Date/DateTime/Enum stored as plain string (interchange type has no text_literal/enum_literal variants)"
  - "Source description field reused for base_url/connection string storage"
  - "Rename operations use remove+add pattern since updateConstruct matches (id, kind) pair"

patterns-established:
  - "Editor pattern: left panel list + right panel detail, selectedId in local state"
  - "Inline validation on entity: no initial, orphan states, duplicate transitions"
  - "Usage tracking: scan operations/facts on each render, pass usageCount to row components"

requirements-completed:
  - Entity editor with interactive state machine canvas (add/delete states, drag transitions, set initial)
  - State machine visualization with force-directed layout, pan, zoom, drag
  - Fact editor with full type picker, parameterized type sub-fields, source, default value
  - Persona editor for managing persona list
  - Source editor for source declarations (protocol, fields)
  - All editors read/write to the Zustand contract store
  - Changes trigger real-time validation via elaboration store

# Metrics
duration: 7min
completed: 2026-02-27
---

# Phase 9 Plan 2: Builder Editors Summary

**Interactive SVG state machine editor + full Tenor type system fact editor + persona and source editors wired into React SPA routing**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-02-27T00:03:13Z
- **Completed:** 2026-02-27T00:10:03Z
- **Tasks:** 6
- **Files modified:** 6 (5 created, 1 modified)

## Accomplishments

- StateMachine SVG component with force-directed layout, pan/zoom/drag, self-loops, arrowheads, add-transition two-click mode
- EntityEditor: left/right panel, state CRUD with initial-state guard and transition cleanup, inline validation
- FactEditor: expandable table rows, all 12 BaseType variants via TypePicker, type-appropriate default editors, freetext/structured source modes
- PersonaEditor: inline rename (double-click), usage count badges, delete warning when referenced by operations
- SourceEditor: protocol selector (http/graphql/database/manual), field name-to-path mapping, referencing-facts display
- All editors wired into BrowserRouter routes, replacing PlaceholderPage; production build clean

## Task Commits

1. **Task 1: StateMachine visualization** - `3367a8f` (feat)
2. **Task 2: EntityEditor** - `1fab08d` (feat)
3. **Task 3: FactEditor** - `3690060` (feat)
4. **Task 4: PersonaEditor** - `d1acf32` (feat)
5. **Task 5: SourceEditor** - `3eb5b00` (feat)
6. **Task 6: Wire editors into routing** - `f9fa06c` (feat)

## Files Created/Modified

- `builder/src/components/visualizations/StateMachine.tsx` - SVG state machine with pan/zoom/drag/selection
- `builder/src/components/editors/EntityEditor.tsx` - Entity CRUD with embedded state machine
- `builder/src/components/editors/FactEditor.tsx` - Fact table with expandable type/default/source editing
- `builder/src/components/editors/PersonaEditor.tsx` - Persona list with usage tracking
- `builder/src/components/editors/SourceEditor.tsx` - Source protocol/fields editor
- `builder/src/App.tsx` - Replaced PlaceholderPage for /entities, /facts, /personas, /sources

## Decisions Made

- **FactDefault plain strings**: The `FactDefault` interchange type does not define `text_literal` or `enum_literal` variants (only `DecimalValue | MoneyValue | BoolLiteral | boolean | number | string`). Text, Date, DateTime, and Enum defaults stored as plain strings.
- **Rename pattern**: Since `updateConstruct` matches on `(id, kind)` pair, renaming an entity/source/persona requires `removeConstruct(oldId) + addConstruct({...updated})`.
- **Source description field**: The `SourceConstruct` interchange type has a `description?: string` field, used to store base URL / connection string (since there is no dedicated `base_url` field in the schema).
- **nodePosOverrides reset**: Position drag overrides are reset via `useEffect` when `states.join(",")` changes to prevent stale positions after add/delete.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed TypeScript FactDefault type mismatch**
- **Found during:** Task 3 (FactEditor)
- **Issue:** Plan described `{ kind: "text_literal", value }` and `{ kind: "enum_literal", value }` for Text/Enum defaults, but these types do not exist in `FactDefault` (which is `DecimalValue | MoneyValue | BoolLiteral | boolean | number | string`)
- **Fix:** Use plain `string` for Text/Date/DateTime/Enum defaults, which is a valid `FactDefault` member
- **Files modified:** `builder/src/components/editors/FactEditor.tsx`
- **Verification:** `npx tsc --noEmit` passes
- **Committed in:** `3690060` (Task 3 commit)

**2. [Rule 1 - Bug] Removed unused `useCallback` import from StateMachine**
- **Found during:** Task 1 cleanup
- **Issue:** `startAddMode` helper using `useCallback` was internal scaffolding with no external caller; keeping it would fire lint warnings
- **Fix:** Removed `useCallback` import and `startAddMode` function
- **Files modified:** `builder/src/components/visualizations/StateMachine.tsx`
- **Verification:** `npx tsc --noEmit` passes
- **Committed in:** `3367a8f` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (2 × Rule 1 bugs)
**Impact on plan:** Both fixes ensure correctness against actual TypeScript types. No scope creep.

## Issues Encountered

None beyond the auto-fixed TypeScript type mismatches above.

## Next Phase Readiness

- Entity, Fact, Persona, and Source editors are fully functional and wired in
- Operation, Rule, and Flow editors remain as PlaceholderPage (subsequent plans)
- Contract store, elaboration store, and layout utilities ready for remaining editor plans

---
*Phase: 09-builder*
*Completed: 2026-02-27*

## Self-Check: PASSED

All created files verified present:
- `builder/src/components/visualizations/StateMachine.tsx` — FOUND
- `builder/src/components/editors/EntityEditor.tsx` — FOUND
- `builder/src/components/editors/FactEditor.tsx` — FOUND
- `builder/src/components/editors/PersonaEditor.tsx` — FOUND
- `builder/src/components/editors/SourceEditor.tsx` — FOUND

All task commits verified in git log:
- `3367a8f` — FOUND
- `1fab08d` — FOUND
- `3690060` — FOUND
- `d1acf32` — FOUND
- `3eb5b00` — FOUND
- `f9fa06c` — FOUND
