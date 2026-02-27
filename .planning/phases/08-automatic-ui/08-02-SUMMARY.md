---
phase: 08-automatic-ui
plan: 02
subsystem: ui
tags: [react, typescript, codegen, components, hooks]

# Dependency graph
requires:
  - phase: 08-01
    provides: generate.rs orchestrator with stub components and hooks ready to replace
  - phase: 07-sdks
    provides: CodegenBundle, TypeInfo, to_pascal_case, to_camel_case patterns
provides:
  - types_gen.rs: emit_ui_types() generating contract-driven types.ts with entity unions, Facts interface, Persona, ENTITIES/OPERATIONS/FLOWS/VERDICT_TYPES consts, FACTS metadata
  - hooks.rs: three React hooks (useActionSpace with 300ms debounce, useEntities, useExecution)
  - components.rs: 11 React components (Dashboard, EntityList, EntityDetail, InstanceDetail, ActionSpace, BlockedActions, FactInput, FlowExecution, FlowHistory, ProvenanceDrill, VerdictDisplay)
  - generate.rs updated: all stubs replaced with real generators, types.ts now from types_gen
affects:
  - 08-03/04 (components serve as the complete UI implementation)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Raw string r##\"...\"## used when generated TypeScript contains \"# sequences"
    - "format!() used only when Rust variables are interpolated; otherwise raw string .to_string()"
    - "FactInput uses FACTS metadata from types.ts for runtime type dispatch (no static codegen)"
    - "ProvenanceDrill uses expandable TreeNode with useState for click-to-expand behavior"
    - "ActionSpace instance binding selectors shown when Object.keys(instances).length > 1"

key-files:
  created:
    - crates/cli/src/ui/types_gen.rs
    - crates/cli/src/ui/hooks.rs
    - crates/cli/src/ui/components.rs
  modified:
    - crates/cli/src/ui/generate.rs
    - crates/cli/src/ui/templates.rs
    - crates/cli/src/ui/mod.rs

key-decisions:
  - "[08-02] emit_action_space and emit_fact_input use r##\"...\"## raw strings (no format!()) because generated TypeScript has no Rust variable interpolation -- clippy::useless_format would fire otherwise"
  - "[08-02] FactInput dispatches on FACTS metadata at runtime (imported from types.ts) rather than statically embedding fact info in component code"
  - "[08-02] ProvenanceDrill uses color literal string for rules (#7c3aed purple) -- requires r## delimiters since \"# terminates r# raw strings"
  - "[08-02] Entity transitions field absent in CodegenEntity -- ENTITIES const emits transitions: [] as placeholder per plan guidance"
  - "[08-02] Decimal type maps to number in UI types (not string) for simpler form input handling"

patterns-established:
  - "types_gen.rs separation: UI type generation lives in its own module separate from generate.rs"
  - "Component emitters accept &CodegenBundle; emitters with no bundle data use _bundle prefix"
  - "Raw string delimiter escalation: use r## when generated content contains \"# sequences"

requirements-completed: [UI-TYPES-01, UI-DASH-01, UI-ENTITY-01, UI-ACTION-01, UI-FACT-01, UI-FLOW-01, UI-PROV-01, UI-VERDICT-01, UI-INSTANCE-01]

# Metrics
duration: 15min
completed: 2026-02-27
---

# Phase 8 Plan 2: Automatic UI — Component Implementations Summary

**All contract-driven React components generated from the interchange bundle: types.ts with entity state unions and fact interface, useActionSpace/useEntities/useExecution hooks, Dashboard, EntityList, EntityDetail, InstanceDetail, ActionSpace with type-aware FactInput and instance binding selectors, FlowExecution (execute/simulate modes), ProvenanceDrill (verdict->rule->facts tree), VerdictDisplay (stratum-organized), and FlowHistory (filterable timeline).**

## Performance

- **Duration:** 15 min
- **Started:** 2026-02-27T20:04:20Z
- **Completed:** 2026-02-27T20:19:02Z
- **Tasks:** 7 (1 types_gen, 1 hooks, 3 components, 1 wiring, 1 quality gates)
- **Files modified/created:** 6

## Accomplishments

- `types_gen.rs`: `emit_ui_types()` generates contract-specific types.ts with:
  - Entity state union types (e.g., `EscrowAccountState = "held" | "released" | ...`)
  - `EntityStates` map interface
  - `ENTITIES` const with states array and initial state
  - `Facts` interface with UI-friendly types (Money -> `{amount: number; currency: string}`)
  - `Persona` union and `PERSONAS` array
  - `OperationMeta` interface and `OPERATIONS` array
  - `FlowMeta` interface and `FLOWS` array
  - `VerdictType` union and `VERDICT_TYPES` array
  - `FactMeta` interface and `FACTS` array (recursive for List/Record types)
- `hooks.rs`: Three React hooks:
  - `useActionSpace`: debounced (300ms) API call on persona/facts change
  - `useEntities`: fetches all instances for all entity types via Promise.all
  - `useExecution`: execute/simulate with result tracking and loading state
- `components.rs`: 11 contract-driven React components, all using inline styles with theme variables
- `generate.rs`: stubs replaced with real generators; old `emit_types()` removed
- `templates.rs`: `stub_component()` and `stub_hook()` removed (superseded)
- 96/96 conformance tests still pass, clippy clean

## Task Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Contract-driven TypeScript type generation | 17759f7 | types_gen.rs, mod.rs |
| 2 | React hooks generation | c70f869 | hooks.rs |
| 3-5 | Dashboard and all components | 343a692 | components.rs |
| 6 | Wire into generate pipeline | 7c33a66 | generate.rs, templates.rs |
| 7 | Quality gates (no code changes) | — | — |

## Files Created/Modified

- `crates/cli/src/ui/types_gen.rs` — Contract-driven TypeScript type generation (368 lines)
- `crates/cli/src/ui/hooks.rs` — React hook generation: useActionSpace, useEntities, useExecution (187 lines)
- `crates/cli/src/ui/components.rs` — 11 React component generators (1579 lines)
- `crates/cli/src/ui/generate.rs` — Updated: stubs replaced, old emit_types removed, types_gen wired in
- `crates/cli/src/ui/templates.rs` — Removed stub_component/stub_hook (now superseded)
- `crates/cli/src/ui/mod.rs` — Added mod components, mod hooks, mod types_gen

## Decisions Made

- `emit_action_space` and `emit_fact_input` use `r##"..."##` raw strings (no `format!()`) — clippy's `useless_format` lint fires when `format!` is used with `{{}}` escaping but no Rust variable substitution. Switching to raw strings with direct `{`, `}` characters avoids this.
- `FactInput` dispatches on `FACTS` metadata from `types.ts` at runtime — no static codegen of type information into the component body. This keeps the component generic.
- `ProvenanceDrill` hardcodes `color="#7c3aed"` (purple) for rule nodes in the provenance chain. This required escalating the raw string delimiter to `r##"..."##` since `"#` would otherwise terminate `r#"..."#`.
- Entity `transitions` field is absent in `CodegenEntity` — ENTITIES const emits `transitions: []` as placeholder per plan instructions.
- `Decimal` maps to `number` in the UI Facts interface (plan says "UI-friendly types").

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed `format!` useless use in emit_action_space and emit_fact_input**
- **Found during:** Task 6 quality gates (clippy)
- **Issue:** Both functions used `format!(r#"..."#)` with `{{}}` escaping but no Rust variable interpolation. Clippy `-D warnings` rejects `clippy::useless_format`.
- **Fix:** Converted both to `r##"..."##.to_string()` with direct `{`, `}` characters (since raw strings don't do format escaping). Changed all `{{foo}}` to `{foo}` and `{{{{...}}}}` to `{{...}}` in the generated TypeScript.
- **Files modified:** crates/cli/src/ui/components.rs
- **Commit:** 343a692

**2. [Rule 1 - Bug] Fixed raw string delimiter conflict in emit_provenance_drill**
- **Found during:** Initial compile of components.rs
- **Issue:** `color="#7c3aed"` inside `r#"..."#` — the `"#` sequence in the hex color terminated the raw string literal early, causing Rust parse errors.
- **Fix:** Changed `r#"..."#` to `r##"..."##` for that function.
- **Files modified:** crates/cli/src/ui/components.rs
- **Commit:** 343a692

**3. [Rule 1 - Bug] Fixed incorrect format string escape in emit_action_space**
- **Found during:** Initial compile
- **Issue:** `({{"{"}}{{inst.state}}{{"}"}}{{)}})` was an incorrect attempt to produce `({inst.state})` — mixing format string escaping with literal brace characters incorrectly.
- **Fix:** Changed to `({{inst.state}})` which produces `({inst.state})` in format! output; then converted to raw string as part of fix #1.
- **Files modified:** crates/cli/src/ui/components.rs
- **Commit:** 343a692

---

**Total deviations:** 3 auto-fixed (all compilation/lint bugs caught during quality gate)
**Impact on plan:** No scope changes; all required components implemented as specified.

## Self-Check

### Files Created

- [x] `/Users/bwb/src/riverline/tenor/crates/cli/src/ui/types_gen.rs` exists
- [x] `/Users/bwb/src/riverline/tenor/crates/cli/src/ui/hooks.rs` exists
- [x] `/Users/bwb/src/riverline/tenor/crates/cli/src/ui/components.rs` exists

### Generated UI Files

- [x] `/tmp/test-ui-08-02/src/types.ts` contains `EscrowAccountState`, `Facts`, `Persona`, `ENTITIES`
- [x] `/tmp/test-ui-08-02/src/components/Dashboard.tsx` imports ENTITIES, uses useEntities
- [x] `/tmp/test-ui-08-02/src/components/FactInput.tsx` has Bool/Int/Money/Enum/List/Record dispatch
- [x] `/tmp/test-ui-08-02/src/components/ActionSpace.tsx` has persona selector, execute/simulate buttons
- [x] `/tmp/test-ui-08-02/src/components/ProvenanceDrill.tsx` renders verdict->rule->facts chain
- [x] `/tmp/test-ui-08-02/src/components/FlowExecution.tsx` handles execute and simulate modes

### Commits

- [x] 17759f7 — feat(08-02): contract-driven TypeScript type generation
- [x] c70f869 — feat(08-02): React hooks generation
- [x] 343a692 — feat(08-02): React component generation
- [x] 7c33a66 — feat(08-02): wire components and hooks into UI generation pipeline

## Self-Check: PASSED

All created files exist. All 4 commits verified. Generated UI project passes smoke test with 24 files generated from integration_escrow.tenor contract.

---
*Phase: 08-automatic-ui*
*Completed: 2026-02-27*
