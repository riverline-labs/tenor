---
phase: 18-platform-hardening
plan: 01
subsystem: interchange
tags: [interchange, deserialization, shared-types, deduplication]

# Dependency graph
requires: []
provides:
  - "tenor-interchange crate with shared typed deserialization of interchange JSON"
  - "Single from_interchange() entry point for bundle parsing"
  - "InterchangeConstruct enum covering all 8 construct kinds"
affects: [eval, analyze, codegen, sdk, lsp]

# Tech tracking
tech-stack:
  added: [tenor-interchange]
  patterns: [shared-interchange-types, delegation-to-shared-crate]

key-files:
  created:
    - crates/interchange/Cargo.toml
    - crates/interchange/src/lib.rs
    - crates/interchange/src/types.rs
    - crates/interchange/src/deserialize.rs
  modified:
    - Cargo.toml
    - crates/eval/Cargo.toml
    - crates/eval/src/types.rs
    - crates/analyze/Cargo.toml
    - crates/analyze/src/bundle.rs
    - crates/codegen/Cargo.toml
    - crates/codegen/src/bundle.rs

key-decisions:
  - "Use serde_json::Value for deeply nested fields (predicates, expressions, flow steps) since each consumer interprets differently"
  - "Keep crate-specific deep parsers (eval: predicates/flow steps, codegen: TypeInfo enum, analyze: shallow Value fields)"
  - "Shared types use InterchangeConstruct enum with 8 variants for kind dispatch"
  - "Helper methods on RuleConstruct (when/produce/verdict_type/produce_payload) for ergonomic access"

patterns-established:
  - "Interchange delegation: consumer crates call tenor_interchange::from_interchange() then convert shared types to domain types"
  - "Forward compatibility: unknown construct kinds are silently skipped"

requirements-completed: [HARD-01, HARD-27]

# Metrics
duration: ~45min
completed: 2026-02-23
---

# Phase 18 Plan 01: Interchange Crate Summary

**New `tenor-interchange` crate provides single-source typed deserialization for interchange JSON, replacing ~300 lines of triplicated parsing across eval, analyze, and codegen**

## Performance

- **Duration:** ~45 min (across continuation sessions)
- **Started:** 2026-02-23
- **Completed:** 2026-02-23
- **Tasks:** 2
- **Files modified:** 11 (4 created + 7 modified)

## Accomplishments

- Created `tenor-interchange` crate with typed structs for all 8 interchange construct kinds (Fact, Entity, Rule, Operation, Flow, Persona, System, TypeDecl)
- Migrated eval, analyze, and codegen to delegate initial JSON parsing to `tenor_interchange::from_interchange()`
- Eliminated ~300 lines of triplicated deserialization code while preserving all crate-specific deep parsers
- All 508+ existing tests pass unchanged, all 73 conformance tests pass, clippy clean

## Task Commits

Each task was committed atomically:

1. **Task 1: Create tenor-interchange crate with shared types and deserialization** - `2446816` (feat)
2. **Task 2: Migrate eval, analyze, and codegen to use tenor-interchange** - `a6cc84a` (refactor)

## Files Created/Modified

- `crates/interchange/Cargo.toml` - New crate manifest with serde + serde_json deps
- `crates/interchange/src/lib.rs` - Public API re-exports
- `crates/interchange/src/types.rs` - Typed structs: InterchangeBundle, InterchangeConstruct enum (8 variants), Provenance, per-construct structs with helper methods
- `crates/interchange/src/deserialize.rs` - from_interchange() function with 14 unit tests
- `Cargo.toml` - Added "crates/interchange" to workspace members
- `crates/eval/Cargo.toml` - Added tenor-interchange dependency
- `crates/eval/src/types.rs` - Contract::from_interchange() delegates to tenor_interchange; removed dead parse_fact/entity/rule/operation/flow functions
- `crates/analyze/Cargo.toml` - Added tenor-interchange dependency
- `crates/analyze/src/bundle.rs` - AnalysisBundle::from_interchange() delegates to tenor_interchange; removed dead parse_* and required_* functions
- `crates/codegen/Cargo.toml` - Added tenor-interchange dependency
- `crates/codegen/src/bundle.rs` - CodegenBundle::from_interchange() delegates to tenor_interchange; removed dead parse_* and required_str functions

## Decisions Made

- **serde_json::Value for nested fields**: Predicates, expressions, and flow steps remain as `serde_json::Value` in shared types since eval needs deep Predicate trees, analyze uses shallow Value access, and codegen ignores them entirely. Only top-level construct structure is shared.
- **Helper methods on RuleConstruct**: Added `when()`, `produce()`, `verdict_type()`, `produce_payload()` methods that navigate the body JSON, giving consumers ergonomic access without raw JSON traversal.
- **Keep crate-specific parsers**: eval retains `parse_predicate`, `parse_flow_step`, `TypeSpec::from_json`; codegen retains `parse_type_info` for its TypeInfo enum. These are domain-specific deep parsers, not duplicated interchange parsing.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Previous session experienced file reversion issues from an external process, requiring multiple re-applications. This session completed cleanly without interference.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- `tenor-interchange` is the foundation for Rust and Go SDKs (Phase 24)
- Any future interchange format changes need only one edit point
- All consumer crates verified working with shared types

---
*Phase: 18-platform-hardening*
*Completed: 2026-02-23*
