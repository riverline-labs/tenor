---
phase: 01-spec-completion
plan: 03
subsystem: spec
tags: [cffp, p5, shared-types, type-library, tenor-spec, formalization, import, structural-typing]

# Dependency graph
requires:
  - phase: 01-01
    provides: "Persona CFFP artifact and spec section (depends_on includes Persona)"
  - phase: 01-02
    provides: "P7 CFFP artifact and spec section (depends_on includes P7 Operation Outcome Typing)"
provides:
  - "CFFP artifact for P5 Shared type library (docs/cffp/p5-shared-types.json)"
  - "Section 4.6 (Shared Type Libraries) in docs/TENOR.md with import mechanism, type identity, constraints"
  - "All three CFFP runs complete -- spec constructs ready for interchange schema (plan 01-04)"
affects: [01-04-PLAN, 01-05-PLAN, 02-01-PLAN, 02-02-PLAN, 02-04-PLAN]

# Tech tracking
tech-stack:
  added: []
  patterns: [cffp-scoped-down-canonical, type-library-leaf-constraint, structural-inlining-for-shared-types]

key-files:
  created:
    - docs/cffp/p5-shared-types.json
  modified:
    - docs/TENOR.md

key-decisions:
  - "Type identity is structural -- imported types are fully inlined, no nominal identity change (Candidate C selected)"
  - "Type library files are self-contained leaves in import graph -- no imports within type libraries"
  - "Nominal typing (Candidate B) eliminated -- incompatible with interchange self-containedness"
  - "Transitive import exposure accepted as trade-off vs module system -- resolved by restricting type libraries to leaf files"
  - "Scoped-down canonical form with 6 acknowledged limitations -- module federation, generics, import depth, flat namespace, no type extension, no selective import"
  - "Section 4.6 placed as subsection under BaseType -- shared types extend TypeDecl mechanism, not a new construct"

patterns-established:
  - "Scoped-down CFFP canonical form is a valid outcome when construct complexity warrants it"
  - "Shared type libraries use existing import syntax with no new keywords or construct kinds"

requirements-completed: [SPEC-03, SPEC-05]

# Metrics
duration: 8min
completed: 2026-02-21
---

# Phase 1 Plan 3: CFFP Run for P5 Shared Type Library + Spec Section Summary

**Scoped-down shared type library via CFFP (3 candidates, 8 counterexamples, 9 composition tests) with structural inlining, type library leaf constraint, and 6 acknowledged limitations for v1.0**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-21T16:12:23Z
- **Completed:** 2026-02-21T16:20:48Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Complete CFFP run for P5 Shared Type Library with canonical outcome -- 8 invariants, 3 candidate formalisms, genuine pressure testing with 8 counterexamples and 9 composition tests covering all 8 depends_on constructs
- Candidate B (Nominal-with-source + Qualified References) eliminated by CE2/CE3: nominal typing is fundamentally incompatible with the interchange format's structural inlining and self-containedness requirements
- Candidate C (Scoped-down minimal) selected over Candidate A (Structural + Import): satisfies all 8 invariants including I8 (explicit imports) by restricting type libraries to self-contained leaf files
- Type identity question resolved: structural identity preserved (no change to existing Tenor semantics)
- Full spec section (4.6 Shared Type Libraries) with import mechanism, type identity, DSL examples, constraints, and interchange representation
- All three CFFP runs now complete (Persona, P7, P5) -- spec ready for interchange schema formalization

## Task Commits

Each task was committed atomically:

1. **Task 1: Execute full CFFP run for P5 Shared type library** - `927b9bb` (feat)
2. **Task 2: Write shared type library spec section in docs/TENOR.md** - `b856c1d` (feat)

## Files Created/Modified
- `docs/cffp/p5-shared-types.json` - Complete CFFP instance (511 lines) with 8 invariants, 3 candidates, 8 counterexamples, 9 composition tests, canonical outcome with 6 acknowledged limitations
- `docs/TENOR.md` - New Section 4.6 (Shared Type Libraries) with import mechanism, type identity, constraints, DSL examples, interchange representation; Section 3 updated with shared type library mention; Section 4.5 TypeDecl constraints updated for imported ids; Section 13 Pass 1 updated with type library detection; Section 13 Pass 3 updated for imported TypeDecls; Section 17 P5 moved from "Deferred to v2" to "Resolved in v1.0"; Appendix A AL31-AL36 added

## Decisions Made

1. **Type identity is structural (Candidate C)** -- Imported types are fully inlined during elaboration, producing interchange identical to locally-declared types. No nominal identity change. This was the decisive factor in eliminating Candidate B: nominal typing creates an irreconcilable split between elaboration-time semantics (nominal) and interchange-time semantics (structural). The interchange format's self-containedness requirement (no external file path references) makes nominal typing impossible without a fundamental redesign.

2. **Type library files are self-contained leaves** -- Type libraries may not contain import declarations. This eliminates transitive type propagation (the I8 problem) without requiring a module visibility system. The trade-off is that type libraries cannot compose (a library cannot reference types from another library). This is acceptable for v1.0 -- module federation and composable type libraries are v2 concerns.

3. **Section 4.6 as subsection under BaseType** -- Shared type libraries extend the existing TypeDecl mechanism (Section 4.5) rather than introducing a new top-level construct. The section is placed as 4.6 immediately after TypeDecl, preserving the dependency order and keeping related content together. No section renumbering required.

4. **Scoped-down canonical form with 6 acknowledged limitations** -- This is the hardest CFFP run, and the plan explicitly anticipated a scoped-down outcome. The 6 limitations (module federation, generics, import depth, flat namespace, no type extension, no selective import) are all design decisions appropriate for v1.0 with clear forward-compatible upgrade paths to v2.

5. **Existing import mechanism reused** -- No new syntax, no new keywords, no new construct kinds. The existing `import` statement loads type library files. The only new elaboration rule is that type library files (files containing only TypeDecl constructs) may not contain import declarations.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- All three CFFP runs complete (Persona, P7, P5) -- the spec constructs are finalized
- Plan 01-04 (interchange versioning and JSON Schema) can proceed -- all construct definitions are stable
- Plan 01-05 (spec freeze) can proceed after 01-04
- The scoped-down P5 canonical form creates clear v2 work items: module federation (SPEC-06), generic type parameters (SPEC-07), composable type libraries, namespace prefixing, selective imports

## Self-Check: PASSED

- docs/cffp/p5-shared-types.json: FOUND
- docs/TENOR.md: FOUND
- 01-03-SUMMARY.md: FOUND
- Commit 927b9bb (Task 1): FOUND
- Commit b856c1d (Task 2): FOUND

---
*Phase: 01-spec-completion*
*Completed: 2026-02-21*
