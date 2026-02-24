# Roadmap: Tenor

## Platform & Ecosystem (Phases 1-4)

| Phase | Status |
|-------|--------|
| 1. Agent Skill Examples | Complete |
| 2. Execution Kernel | Not started |
| 3. Embedded Evaluator | Not started |
| 4. Hosted Evaluator Service | Not started |

## Phase Details

### Phase 1: Agent Skill Examples
**Goal**: Reference implementations showing what's possible with the SDK — tenor agent CLI, Express middleware, Slack bot, regulatory audit agent
**Requirements**: SKEX-01 through SKEX-04
**Plans:** 4 plans

Plans:
- [x] 01-01-PLAN.md — `tenor agent` interactive CLI REPL (SKEX-01)
- [x] 01-02-PLAN.md — Express middleware reference implementation (SKEX-02)
- [x] 01-03-PLAN.md — Slack bot reference implementation (SKEX-03)
- [x] 01-04-PLAN.md — Audit agent reference implementation (SKEX-04)

### Phase 2: Execution Kernel
**Goal**: `tenor-executor` crate — a thin transactional wrapper around the existing evaluator that enforces Tenor's executor obligations mechanically against Postgres. Atomic, version-validated, provenance-coupled state commits. Not a new evaluation engine, not a pluggable storage abstraction — a concrete Postgres-backed execution kernel.
**Requirements**: EXEC-01 through EXEC-05

### Phase 3: Embedded Evaluator
**Goal**: WASM-compiled Rust evaluator for browser, Node, and edge environments. Primary use cases are air-gapped and regulated industry deployments — healthcare, defense — where calling a remote service is not an option.
**Requirements**: WASM-01 through WASM-05

### Phase 4: Hosted Evaluator Service
**Goal**: Managed API endpoint that removes the last infrastructure barrier — monetization path if desired.
**Requirements**: HOST-01 through HOST-03
**Depends on**: Phase 3
