# Roadmap: Tenor

## Platform & Ecosystem (Phases 1-2)

| Phase | Status |
|-------|--------|
| 1. Agent Skill Examples | Complete |
| 2. Embedded Evaluator | Not started |

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

### Phase 2: Embedded Evaluator
**Goal**: WASM build target for tenor-eval — `cargo build --target wasm32-unknown-unknown`. The capability is open source. Polished npm packaging or CDN distribution would be commercial if ever needed.
**Requirements**: WASM-01 through WASM-05
