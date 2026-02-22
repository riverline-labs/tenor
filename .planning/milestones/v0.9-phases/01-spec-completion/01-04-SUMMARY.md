---
phase: 01-spec-completion
plan: 04
subsystem: spec
tags: [interchange, json-schema, versioning, semver, tenor-spec, interchange-format]

# Dependency graph
requires:
  - phase: 01-01
    provides: "Persona construct CFFP + spec section (Persona schema in interchange)"
  - phase: 01-02
    provides: "P7 Operation outcome typing (outcomes field, effect-to-outcome association)"
  - phase: 01-03
    provides: "P5 Shared type library (type library import semantics in spec)"
provides:
  - "Formal JSON Schema (docs/interchange-schema.json) for TenorInterchange v1.0"
  - "Interchange versioning semantics (Section 13.2.1) with semver tenor_version field"
  - "Versioning contract for producers and consumers"
  - "v0.3 to v1.0 breaking change documentation"
affects: [01-05-PLAN, 02-01-PLAN, 02-06-PLAN]

# Tech tracking
tech-stack:
  added: [json-schema-draft-2020-12]
  patterns: [interchange-versioning, schema-from-spec-not-implementation]

key-files:
  created:
    - docs/interchange-schema.json
  modified:
    - docs/TENOR.md

key-decisions:
  - "Schema authored from spec (not reverse-engineered from elaborator output) per user decision"
  - "Single schema file with $defs (not split per construct kind) for simplicity and self-containedness"
  - "Bundle-level tenor_version (semver) is canonical; per-construct tenor field is short version"
  - "Schema uses JSON Schema Draft 2020-12 ($id: https://tenor-lang.org/schemas/interchange/v1.0.0)"
  - "Operations outcomes field not marked required in schema (v0.3 tests lack it; v1.0 will require it)"

patterns-established:
  - "Interchange schema defines canonical structure; conformance tests validate elaborator output against it"
  - "Versioning contract: producers emit tenor_version, consumers check before processing"

requirements-completed: [SPEC-04]

# Metrics
duration: 5min
completed: 2026-02-21
---

# Phase 1 Plan 4: Interchange Versioning and JSON Schema Summary

**JSON Schema Draft 2020-12 for TenorInterchange v1.0 covering all 6 construct kinds, 12+ base types, predicate expressions, and flow step types, plus semver versioning semantics in the spec**

## Performance

- **Duration:** 5 min
- **Started:** 2026-02-21T16:23:55Z
- **Completed:** 2026-02-21T16:29:03Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Interchange versioning semantics defined in Section 13.2.1 of docs/TENOR.md: tenor_version field (semver), major/minor/patch semantics, producer/consumer contract, v0.3-to-v1.0 breaking changes
- Formal JSON Schema (836 lines) covering the complete TenorInterchange v1.0 format: Bundle, Fact, Entity, Rule, Persona, Operation (with P7 outcomes), Flow (with all 5 step types), all 12 base types plus Duration/List/Record/TaggedUnion, all predicate expression variants, DecimalValue/MoneyValue structured numerics, failure handlers
- Schema validated against all conformance test expected outputs (positive, numeric, promotion, shorthand, cross_file) -- zero structural inconsistencies

## Task Commits

Each task was committed atomically:

1. **Task 1: Define interchange versioning in docs/TENOR.md** - `64fa0fe` (feat)
2. **Task 2: Create formal JSON Schema for interchange format** - `7213409` (feat)

## Files Created/Modified
- `docs/interchange-schema.json` - Complete JSON Schema Draft 2020-12 (836 lines) with $defs for all construct kinds, base types, predicate expressions, flow step types, failure handlers, and structured numeric representations
- `docs/TENOR.md` - New Section 13.2.1 (Interchange Format Versioning) with tenor_version field definition, semver semantics, producer/consumer contract, and v0.3-to-v1.0 transition documentation

## Decisions Made

1. **Schema authored from spec, not implementation** -- Per the user decision in 01-CONTEXT.md, the schema was authored by reading the formal specification and conformance tests, not by inspecting elaborator serialization code. This ensures the schema is a normative artifact independent of any particular implementation.

2. **Single file with $defs** -- All construct schemas, type schemas, and supporting schemas are in a single `interchange-schema.json` with `$defs` references. This matches the standard JSON Schema pattern and keeps the schema self-contained (no external $ref resolution needed).

3. **Bundle-level tenor_version is canonical** -- The bundle-level `tenor_version` field (semver string, e.g. "1.0.0") is the canonical version. The per-construct `tenor` field (short version, e.g. "1.0") provides a quick check but is not the authoritative version identifier. Both fields are required at their respective levels.

4. **Operation outcomes not required in schema** -- The `outcomes` field on Operation is defined in the schema but not in the `required` array, because existing v0.3 conformance tests do not include this field. In v1.0 elaborator output, this field will be present (P7 mandates it). The schema captures the v1.0 target format while remaining structurally compatible with the v0.3 test corpus for validation purposes.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- JSON Schema complete -- plan 01-05 (spec freeze) can proceed with all four spec completion plans done
- Schema can be used in Phase 2 for elaborator conformance testing (validate output against schema)
- Versioning semantics defined -- elaborator implementation in Phase 2 knows exactly what tenor_version format to emit
- All CFFP constructs (Persona, P7, P5) are covered in the schema

## Self-Check: PASSED

- docs/interchange-schema.json: FOUND
- docs/TENOR.md: FOUND
- 01-04-SUMMARY.md: FOUND
- Commit 64fa0fe (Task 1): FOUND
- Commit 7213409 (Task 2): FOUND

---
*Phase: 01-spec-completion*
*Completed: 2026-02-21*
