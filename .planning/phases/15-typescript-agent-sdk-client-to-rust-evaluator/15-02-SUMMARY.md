---
phase: 15-typescript-agent-sdk-client-to-rust-evaluator
plan: 02
subsystem: sdk
tags: [typescript, npm, fetch, node22, http-client, esm, cjs]

# Dependency graph
requires:
  - phase: 15-typescript-agent-sdk-client-to-rust-evaluator
    plan: 01
    provides: "tenor serve HTTP API with 6 JSON endpoints"
provides:
  - "@tenor-lang/sdk npm package with TenorClient class"
  - "Typed methods: health, listContracts, getOperations, invoke, explain, elaborate"
  - "Error hierarchy: ConnectionError, EvaluationError, ElaborationError, ContractNotFoundError"
  - "TypeScript interfaces mirroring all server API response shapes"
  - "Dual ESM/CJS build output with type declarations"
affects: [15-03-PLAN, agent-tooling, typescript-consumers]

# Tech tracking
tech-stack:
  added: [typescript 5.9, "@types/node ^22"]
  patterns: [zero-runtime-dependency SDK, Node 22 built-in fetch, .ts import extensions with rewriteRelativeImportExtensions, dual ESM/CJS via separate tsconfigs]

key-files:
  created:
    - sdk/typescript/package.json
    - sdk/typescript/tsconfig.json
    - sdk/typescript/tsconfig.build.json
    - sdk/typescript/tsconfig.cjs.json
    - sdk/typescript/src/index.ts
    - sdk/typescript/src/types.ts
    - sdk/typescript/src/errors.ts
    - sdk/typescript/src/client.ts
    - sdk/typescript/tests/client.test.ts
  modified: []

key-decisions:
  - "Zero runtime dependencies: uses Node 22+ built-in fetch and AbortSignal.timeout, no axios or node-fetch"
  - ".ts import extensions in source with rewriteRelativeImportExtensions in build tsconfig for ESM/CJS output"
  - "Separate tsconfig.cjs.json for CommonJS build (Node16 moduleResolution requires module=Node16, incompatible with --module commonjs override)"
  - "404 extraction from error message pattern for /evaluate and /explain paths where contract ID is in POST body not URL"
  - "Node built-in test runner (node:test) with --experimental-strip-types for zero-dep test execution"
  - "Integration tests gated by TENOR_SERVE_URL environment variable for CI/local isolation"

patterns-established:
  - "SDK client pattern: private request() helper with error classification by HTTP status and path"
  - "Dual-build pattern: tsconfig.build.json (ESM) + tsconfig.cjs.json (CJS) with shared source"
  - "Integration test pattern: gated by env var, full API round-trip against running server"

requirements-completed: [SDK-01, SDK-02]

# Metrics
duration: 9min
completed: 2026-02-23
---

# Phase 15 Plan 02: TypeScript SDK Client Summary

**Zero-dependency TypeScript SDK (`@tenor-lang/sdk`) with typed TenorClient wrapping all 6 tenor serve HTTP endpoints, dual ESM/CJS build, 22 passing tests**

## Performance

- **Duration:** 9 min
- **Started:** 2026-02-23T01:44:07Z
- **Completed:** 2026-02-23T01:53:29Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments
- Complete TypeScript SDK at `sdk/typescript/` with `TenorClient` class exposing all agent skills
- TypeScript interfaces accurately mirroring the server API: Verdict, EvalResult, FlowEvalResult, OperationInfo, ExplainResult, etc.
- Error hierarchy with ConnectionError, EvaluationError, ElaborationError, ContractNotFoundError
- 13 unit tests (constructor, errors, connection) + 10 integration tests (all endpoints + error cases) = 22 total
- Dual ESM/CJS output with type declarations, zero runtime dependencies

## Task Commits

Each task was committed atomically:

1. **Task 1: Initialize TypeScript package with types, errors, and client skeleton** - `016dc22` (feat)
2. **Task 2: Implement client methods and write tests** - `0558b57` (test)

## Files Created/Modified
- `sdk/typescript/package.json` - npm package config with ESM/CJS dual exports, Node 22+ engine requirement
- `sdk/typescript/tsconfig.json` - Development/typecheck config with allowImportingTsExtensions
- `sdk/typescript/tsconfig.build.json` - ESM build config with rewriteRelativeImportExtensions
- `sdk/typescript/tsconfig.cjs.json` - CommonJS build config
- `sdk/typescript/src/types.ts` - All TypeScript interfaces matching server API shapes
- `sdk/typescript/src/errors.ts` - Error class hierarchy (TenorError base, 4 specific subclasses)
- `sdk/typescript/src/client.ts` - TenorClient with 6 methods and private request() helper
- `sdk/typescript/src/index.ts` - Public API re-exports
- `sdk/typescript/tests/client.test.ts` - 22 tests using node:test and node:assert

## Decisions Made
- Used Node 22+ built-in `fetch` instead of axios/node-fetch to achieve zero runtime dependencies
- Used `.ts` import extensions in source (not `.js`) with `rewriteRelativeImportExtensions` in build configs -- this allows running tests directly from source via `--experimental-strip-types` while producing correct `.js` imports in compiled output
- Created separate `tsconfig.cjs.json` because TypeScript 5.9 with `moduleResolution: Node16` requires `module: Node16` and cannot be overridden to `commonjs` via CLI flag
- 404 detection for POST endpoints (`/evaluate`, `/explain`) extracts contract ID from the error message pattern `"contract 'xxx' not found"` since the contract ID is in the POST body, not the URL path
- Integration tests gated by `TENOR_SERVE_URL` env var so unit tests run cleanly in CI without a server

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Node 22 fetch types not available without @types/node**
- **Found during:** Task 1 (typecheck)
- **Issue:** TypeScript compilation failed -- `fetch`, `Response`, `RequestInit`, `AbortSignal` not in scope with `lib: ["ES2022"]` alone
- **Fix:** Added `@types/node: ^22` to devDependencies, which includes Node 22's global fetch type declarations
- **Files modified:** sdk/typescript/package.json
- **Verification:** `npm run typecheck` passes
- **Committed in:** 016dc22

**2. [Rule 3 - Blocking] .js import extensions fail with --experimental-strip-types**
- **Found during:** Task 2 (test execution)
- **Issue:** `import from './types.js'` fails at runtime -- Node strip-types mode doesn't rewrite `.js` to `.ts`
- **Fix:** Changed all source imports to use `.ts` extensions, added `allowImportingTsExtensions` to dev tsconfig, and `rewriteRelativeImportExtensions` to build tsconfigs to produce `.js` in compiled output
- **Files modified:** src/client.ts, src/index.ts, tsconfig.json, tsconfig.build.json
- **Verification:** Tests pass with strip-types, build produces correct .js imports
- **Committed in:** 0558b57

**3. [Rule 3 - Blocking] CJS build fails with --module commonjs override**
- **Found during:** Task 2 (build verification)
- **Issue:** `tsc -p tsconfig.build.json --module commonjs` fails because `moduleResolution: Node16` requires `module: Node16`
- **Fix:** Created separate `tsconfig.cjs.json` with `module: CommonJS` and `moduleResolution: Node10`
- **Files modified:** sdk/typescript/tsconfig.cjs.json, package.json build script
- **Verification:** `npm run build` produces both dist/esm and dist/cjs
- **Committed in:** 0558b57

**4. [Rule 1 - Bug] 404 not detected for POST /evaluate and /explain**
- **Found during:** Task 2 (integration tests)
- **Issue:** ContractNotFoundError not thrown for unknown contract on evaluate/explain -- the 404 handler only checked URL path pattern `/contracts/{id}`, but POST endpoints have the contract ID in the body
- **Fix:** Added fallback extraction from error message pattern `contract '...' not found`
- **Files modified:** sdk/typescript/src/client.ts
- **Verification:** Integration tests for ContractNotFoundError on evaluate/explain pass
- **Committed in:** 0558b57

---

**Total deviations:** 4 auto-fixed (1 bug, 3 blocking issues)
**Impact on plan:** All fixes necessary for correct TypeScript compilation, test execution, and error handling. No scope creep.

## Issues Encountered
- Test `.tenor` source syntax required correction: facts need block syntax with `source` field (not shorthand), rules need `stratum` and `produce: verdict name { ... }` syntax, transitions use tuple `(from, to)` not arrow `from -> to`

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- SDK package is fully functional and tested -- ready for end-to-end integration testing (Plan 15-03)
- All three agent skills (getOperations, invoke, explain) work from TypeScript
- Build produces publishable npm package with ESM, CJS, and type declarations

---
*Phase: 15-typescript-agent-sdk-client-to-rust-evaluator*
*Plan: 02*
*Completed: 2026-02-23*
