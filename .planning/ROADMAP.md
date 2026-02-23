# Roadmap: Tenor

## Platform & Ecosystem (Phases 1-7)

| Phase | Status |
|-------|--------|
| 1. Agent Skill Examples | Not started |
| 2. AI Authoring Assistant | Not started |
| 3. Embedded Evaluator | Not started |
| 4. Hosted Evaluator Service | Not started |
| 5. Domain Contract Library | Not started |
| 6. Rust and Go Agent SDKs | Not started |
| 7. Multi-party Contract Execution | Not started |

## Phase Details

### Phase 1: Agent Skill Examples
**Goal**: Reference implementations showing what's possible with the SDK — tenor agent CLI, Express middleware, Slack bot, regulatory audit agent
**Requirements**: SKEX-01 through SKEX-04

### Phase 2: AI Authoring Assistant
**Goal**: A Tenor contract authoring guide designed to be used with any AI assistant. Includes prompt templates that give an AI assistant sufficient context to help author Tenor contracts, example conversations demonstrating three suggested workflows (autonomous: describe and walk away; collaborative: human steers, AI executes; reviewed: AI summarizes contract in plain language for business approval), and guidance on when to ask questions versus make decisions. Deliverable is documentation in the repo — markdown files, not software.
**Requirements**: AUTH-01 through AUTH-03

### Phase 3: Embedded Evaluator
**Goal**: WASM-compiled Rust evaluator for browser, Node, and edge environments. Primary use cases are air-gapped and regulated industry deployments — healthcare, defense — where calling a remote service is not an option.
**Requirements**: WASM-01 through WASM-05

### Phase 4: Hosted Evaluator Service
**Goal**: Managed API endpoint that removes the last infrastructure barrier — monetization path if desired.
**Requirements**: HOST-01 through HOST-03
**Depends on**: Phase 3

### Phase 5: Domain Contract Library
**Goal**: Curated contracts for common industries, community contribution framework, the five existing domains as seed.
**Requirements**: LIB-01 through LIB-03
**Depends on**: Phase 4

### Phase 6: Rust and Go Agent SDKs
**Goal**: Same skills, same executor API, same trust model. Rust for embedded and systems teams, Go for backend and infrastructure teams. Both matter for healthcare and defense use cases.
**Requirements**: RSDK-01 through RSDK-03
**Depends on**: Phase 5

### Phase 7: Multi-party Contract Execution
**Goal**: Two or more parties, one contract, independent verification — no "our records vs their records." The full trust model realized in production. Capstone phase.
**Requirements**: MPTY-01 through MPTY-03
**Depends on**: Phase 4, Phase 6
