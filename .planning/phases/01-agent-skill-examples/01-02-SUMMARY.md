---
phase: 01-agent-skill-examples
plan: 02
subsystem: examples
tags: [express, middleware, rest, typescript, sdk]

requires:
  - phase: prior
    provides: TenorClient SDK, tenor serve HTTP API
provides:
  - "Express middleware that generates REST routes from contract operations"
  - "Example server demonstrating middleware usage"
  - "README with working curl examples"
affects: [sdk-examples, documentation]

tech-stack:
  added: [express]
  patterns: ["SDK-backed middleware route generation", "Error type to HTTP status mapping"]

key-files:
  created:
    - examples/express-middleware/src/middleware.ts
    - examples/express-middleware/src/server.ts
    - examples/express-middleware/README.md
    - examples/express-middleware/package.json
    - examples/express-middleware/tsconfig.json

key-decisions:
  - "SDK imported via relative path (not npm) since it is in the same repo"
  - "Default route prefix /tenor to avoid conflicts with app routes"
  - "Operation endpoint validates persona authorization before evaluation"

patterns-established:
  - "SDK error to HTTP status mapping: ContractNotFoundError->404, EvaluationError->422, ConnectionError->502"

requirements-completed: [SKEX-02]

duration: 5min
completed: 2026-02-23
---

# Plan 01-02: Express Middleware Summary

**Express middleware that auto-generates REST routes from Tenor contract operations with SDK error mapping and operation persona validation**

## Performance

- **Duration:** 5 min
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- `tenorMiddleware()` function generates Express Router from contract operations
- Five REST endpoints: list contracts, explain, list operations, evaluate, execute operation
- SDK error types mapped to HTTP status codes (404, 422, 502)
- Operation endpoint validates persona authorization before evaluation
- README provides complete quickstart with working curl examples

## Task Commits

1. **Task 1: Create Express middleware** - `015ee50` (feat)
2. **Task 2: Create example server and README** - included in `015ee50`

## Files Created/Modified
- `examples/express-middleware/src/middleware.ts` - Middleware generating routes from operations
- `examples/express-middleware/src/server.ts` - Example server using the middleware
- `examples/express-middleware/README.md` - Usage instructions with curl examples
- `examples/express-middleware/package.json` - npm package configuration
- `examples/express-middleware/tsconfig.json` - TypeScript configuration

## Decisions Made
- SDK imported via relative path since it is in the same repo
- Default route prefix `/tenor` to avoid conflicts with application routes
- Operation endpoint validates persona authorization before running evaluation

## Deviations from Plan
None - plan executed as specified

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Express middleware pattern established for web framework integration
- SDK integration pattern reusable for other Node.js frameworks

---
*Phase: 01-agent-skill-examples*
*Completed: 2026-02-23*
