---
phase: 16-typescript-code-generation
plan: 01
subsystem: codegen
tags: [typescript, zod, branded-types, code-generation, interchange-json]

# Dependency graph
requires:
  - phase: 15-typescript-agent-sdk
    provides: SDK client and trust boundary model for TypeScript consumers
provides:
  - tenor-codegen TypeScript emitter (types.ts, schemas.ts) from interchange JSON
  - CLI `tenor generate typescript --out <dir> <input>` subcommand
  - Branded string types for Money, Decimal, Date, DateTime, Duration
  - Zod validators for fact input types
  - Entity state union types
  - Operation input interfaces
  - Verdict type union
affects: [16-02-client-class, 17-vscode-extension, 23-rust-go-codegen]

# Tech tracking
tech-stack:
  added: [serde, serde_json (codegen crate)]
  patterns: [interchange-json-deserialization, branded-string-types, zod-validation]

key-files:
  created:
    - crates/codegen/src/bundle.rs
    - crates/codegen/src/typescript.rs
    - crates/codegen/src/typescript_schemas.rs
  modified:
    - crates/codegen/src/lib.rs
    - crates/codegen/Cargo.toml
    - crates/cli/src/main.rs
    - crates/cli/Cargo.toml
    - crates/cli/tests/cli_integration.rs

key-decisions:
  - "Removed tenor-core dependency from codegen crate -- codegen reads interchange JSON only, same pattern as tenor-eval and tenor-analyze"
  - "PascalCase entity IDs preserved as-is in type names rather than re-casing (DeliveryRecord stays DeliveryRecord, not Deliveryrecord)"
  - "Branded types always emitted regardless of whether contract uses them -- zero runtime cost, consistent developer experience"

patterns-established:
  - "Codegen bundle pattern: CodegenBundle::from_interchange() extracts only codegen-relevant fields, ignoring provenance/body details"
  - "Emitter pattern: emit_types(bundle) -> String, emit_schemas(bundle, sdk_import) -> String -- pure functions producing complete file content"
  - "CLI subcommand pattern: `tenor generate <language>` with per-language subcommands instead of --target flag"

requirements-completed: [CGEN-01]

# Metrics
duration: 9min
completed: 2026-02-23
---

# Phase 16 Plan 01: TypeScript Code Generation Emitters Summary

**TypeScript codegen emitters producing branded types, Zod fact validators, entity state unions, and operation input interfaces from interchange JSON bundles via `tenor generate typescript`**

## Performance

- **Duration:** 9 min
- **Started:** 2026-02-23T02:57:29Z
- **Completed:** 2026-02-23T03:06:29Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments
- Implemented full TypeScript code generation pipeline: interchange JSON -> types.ts + schemas.ts
- Wired CLI `tenor generate typescript --out <dir> <input>` accepting both .tenor and .json input
- Generated types include branded strings (TenorMoney, TenorDecimal, TenorDate, TenorDateTime, TenorDuration), entity state unions, facts interface, operation input interfaces, and verdict type union
- Generated schemas include Zod validators for all fact types with branded type constructor helpers

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement tenor-codegen TypeScript type and schema emitters** - `16693b2` (feat)
2. **Task 2: Wire CLI generate command and verify end-to-end** - `b298177` (feat)

## Files Created/Modified
- `crates/codegen/src/bundle.rs` - Interchange JSON deserialization into codegen-internal typed structs
- `crates/codegen/src/typescript.rs` - TypeScript types.ts emitter (branded types, entity unions, facts, operations, verdicts)
- `crates/codegen/src/typescript_schemas.rs` - Zod schemas.ts emitter (fact validators, branded type constructors)
- `crates/codegen/src/lib.rs` - Public API: generate_typescript() writing files to output directory
- `crates/codegen/Cargo.toml` - Replaced tenor-core dep with serde/serde_json
- `crates/cli/src/main.rs` - Replaced stub Generate with subcommand-based GenerateCommands
- `crates/cli/Cargo.toml` - Added tenor-codegen dependency
- `crates/cli/tests/cli_integration.rs` - Replaced stub test with 4 generate integration tests

## Decisions Made
- Removed tenor-core dependency from codegen crate -- codegen reads interchange JSON only, maintaining the same independent deserialization pattern as tenor-eval and tenor-analyze
- PascalCase identifiers (entity IDs like DeliveryRecord) are preserved as-is in generated type names rather than being lowercased and re-cased
- Branded types section is always emitted regardless of whether the contract uses those types -- zero runtime cost and provides consistent developer experience

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed to_pascal_case lowercasing PascalCase entity IDs**
- **Found during:** Task 2 (end-to-end verification)
- **Issue:** Entity IDs like "DeliveryRecord" were being converted to "Deliveryrecord" because to_pascal_case lowercased everything after the first character
- **Fix:** Added short-circuit path: if input has no separators, preserve original casing with only first letter uppercased
- **Files modified:** crates/codegen/src/typescript.rs
- **Verification:** Unit test added for DeliveryRecord and EscrowAccount; end-to-end output verified
- **Committed in:** b298177 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 bug fix)
**Impact on plan:** Essential for correctness of generated TypeScript type names. No scope creep.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Codegen emitters produce correct types.ts and schemas.ts for any contract
- CLI subcommand infrastructure ready for client.ts and index.ts additions in Plan 02
- CodegenBundle struct has all construct data needed for typed client class generation

---
*Phase: 16-typescript-code-generation*
*Completed: 2026-02-23*
