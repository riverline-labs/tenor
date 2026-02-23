# Roadmap: Tenor

## Platform & Ecosystem (Phases 18-25)

| Phase | Status |
|-------|--------|
| 18. Platform Hardening | 7/9 | In Progress|  | Not started |
| 20. AI Authoring Assistant | Not started |
| 21. Embedded Evaluator | Not started |
| 22. Hosted Evaluator Service | Not started |
| 23. Domain Contract Library | Not started |
| 24. Rust and Go Agent SDKs | Not started |
| 25. Multi-party Contract Execution | Not started |

## Phase Details

### Phase 18: Platform Hardening
**Goal**: Fix all blocking concerns identified by codebase mapping — shared interchange library, typed explain.rs, error recovery in parser, WASM-ready I/O abstraction, production HTTP stack, security hardening, SystemContract coordinator design, indexed lookups, LSP tests, and conformance fixture gaps.
**Requirements**: HARD-01 through HARD-27
**Depends on**: Phase 17 (complete)
**Plans:** 7/9 plans executed

Plans:
- [x] 18-01-PLAN.md — Shared interchange deserialization library (HARD-01, HARD-27)
- [x] 18-02-PLAN.md — Core error hardening: expect removal, HashSet cycles, string alloc reduction (HARD-03, HARD-22, HARD-23)
- [x] 18-03-PLAN.md — Parser error recovery + WASM I/O trait (HARD-04, HARD-05)
- [x] 18-04-PLAN.md — Eval performance: HashMap indexes, stratum indexing, flow clone elimination (HARD-13, HARD-17, HARD-18)
- [x] 18-05-PLAN.md — Cleanup: dead code, duplicates, version constants, configurable limits (HARD-07, HARD-15, HARD-16, HARD-19, HARD-21)
- [x] 18-06-PLAN.md — Production HTTP stack with axum + tokio (HARD-06, HARD-08, HARD-09)
- [x] 18-07-PLAN.md — Typed explain.rs rewrite (HARD-02)
- [ ] 18-08-PLAN.md — HTTP security: input validation, CORS, rate limiting, auth (HARD-10, HARD-11)
- [ ] 18-09-PLAN.md — Test coverage gaps + SystemContract design (HARD-12, HARD-14, HARD-20, HARD-24, HARD-25, HARD-26)

### Phase 19: Agent Skill Examples
**Goal**: Reference implementations showing what's possible with the SDK — tenor agent CLI, Express middleware, Slack bot, regulatory audit agent
**Requirements**: SKEX-01 through SKEX-04
**Depends on**: Phase 18

### Phase 20: AI Authoring Assistant
**Goal**: A Tenor contract authoring guide designed to be used with any AI assistant. Includes prompt templates that give an AI assistant sufficient context to help author Tenor contracts, example conversations demonstrating three suggested workflows (autonomous: describe and walk away; collaborative: human steers, AI executes; reviewed: AI summarizes contract in plain language for business approval), and guidance on when to ask questions versus make decisions. Deliverable is documentation in the repo — markdown files, not software.
**Requirements**: AUTH-01 through AUTH-03
**Depends on**: Phase 18

### Phase 21: Embedded Evaluator
**Goal**: WASM-compiled Rust evaluator for browser, Node, and edge environments. Primary use cases are air-gapped and regulated industry deployments — healthcare, defense — where calling a remote service is not an option.
**Requirements**: WASM-01 through WASM-05
**Depends on**: Phase 18

### Phase 22: Hosted Evaluator Service
**Goal**: Managed API endpoint that removes the last infrastructure barrier — monetization path if desired.
**Requirements**: HOST-01 through HOST-03
**Depends on**: Phase 21

### Phase 23: Domain Contract Library
**Goal**: Curated contracts for common industries, community contribution framework, the five existing domains as seed.
**Requirements**: LIB-01 through LIB-03
**Depends on**: Phase 22

### Phase 24: Rust and Go Agent SDKs
**Goal**: Same skills, same executor API, same trust model. Rust for embedded and systems teams, Go for backend and infrastructure teams. Both matter for healthcare and defense use cases.
**Requirements**: RSDK-01 through RSDK-03
**Depends on**: Phase 23

### Phase 25: Multi-party Contract Execution
**Goal**: Two or more parties, one contract, independent verification — no "our records vs their records." The full trust model realized in production. Capstone phase.
**Requirements**: MPTY-01 through MPTY-03
**Depends on**: Phase 22, Phase 24
