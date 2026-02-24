# Roadmap: Tenor

## Platform & Ecosystem (Phases 1-4)

| Phase | Status | Repository |
|-------|--------|------------|
| 1. Agent Skill Examples | Complete | tenor (public) |
| 2. Execution Kernel | Not started | tenor-platform (private) |
| 3. Embedded Evaluator | Not started | tenor (public) |
| 4. Hosted Evaluator Service | Not started | tenor-platform (private) |

## Repository Split

- **`riverline-labs/tenor`** (public) — elaborator, evaluator, CLI, LSP, SDK, conformance suite, spec, WASM build target
- **`riverline-labs/tenor-platform`** (private) — execution kernel (Postgres-backed), hosted evaluator service, commercial packaging

Dependency direction: `tenor-platform` depends on `tenor` crates. Never the reverse.

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
**Repository**: `riverline-labs/tenor-platform`

### Phase 3: Embedded Evaluator
**Goal**: WASM build target for tenor-eval — `cargo build --target wasm32-unknown-unknown`. The capability is open source. Polished npm packaging or CDN distribution would be commercial if ever needed.
**Requirements**: WASM-01 through WASM-05
**Repository**: `riverline-labs/tenor`

### Phase 4: Hosted Evaluator Service
**Goal**: Managed API endpoint that removes the last infrastructure barrier — monetization path if desired.
**Requirements**: HOST-01 through HOST-03
**Depends on**: Phase 3
**Repository**: `riverline-labs/tenor-platform`
