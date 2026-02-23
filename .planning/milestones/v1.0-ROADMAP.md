# Roadmap: Tenor

## Milestones

- **v0.9 Core** — Phases 1-5.1 (shipped 2026-02-22) — [archive](milestones/v0.9-ROADMAP.md)
- **v1.0 System Construct + Documentation** — Phases 12-14 (shipped 2026-02-22)
- **Agent Tooling** — Phases 15-19 (current)

## Phases

<details>
<summary>v0.9 Core (Phases 1-5.1) — SHIPPED 2026-02-22</summary>

- [x] Phase 1: Spec Completion (5/5 plans)
- [x] Phase 01.1: Spec CI — AI Ambiguity Testing (2/2 plans)
- [x] Phase 2: Foundation (4/4 plans)
- [x] Phase 3: CLI + Evaluator (7/7 plans)
- [x] Phase 3.1: CFFP Migration Semantics (2/2 plans)
- [x] Phase 3.2: Technical Debt & Bug Fixes (3/3 plans)
- [x] Phase 3.3: Flow Migration Compatibility (2/2 plans)
- [x] Phase 3.4: Contract Discovery (2/2 plans)
- [x] Phase 4: Static Analysis (8/8 plans)
- [x] Phase 5: Domain Validation (8/8 plans)
- [x] Phase 5.1: Fix Critical DSL Gaps (3/3 plans)

14 phases, 46 plans. Full details: [v0.9-ROADMAP.md](milestones/v0.9-ROADMAP.md)

</details>

<details>
<summary>v1.0 System Construct + Documentation (Phases 12-14) — SHIPPED 2026-02-22</summary>

- [x] Phase 12: System Construct (6/6 plans) — CFFP-driven design and full-stack implementation
- [x] Phase 12.1: AAP Spec Audit (2/2 plans) — Assumption Audit Protocol on complete v1.0 spec
- [x] Phase 13: Domain Re-validation (7/7 plans) — All 5 domain contracts re-validated for v1.0
- [x] Phase 14: Documentation (3/3 plans) — Author guide, one-page explainer, README update

4 phases, 17 plans.

</details>

### Agent Tooling (current)

- [ ] **Phase 15: TypeScript Agent SDK** - Client SDK to Rust evaluator exposing agent skills
- [ ] **Phase 16: TypeScript Code Generation** - Generated typed interfaces and client bindings (optional optimization)
- [ ] **Phase 17: VS Code Extension** - Syntax highlighting, LSP, Preview Agent Capabilities panel
- [ ] **Phase 18: Agent Skill Examples** - Reference implementations (CLI tool, Express middleware, Slack bot, audit agent)
- [ ] **Phase 19: Embedded Evaluator** - WASM-compiled Rust evaluator for browser, Node, and edge environments

## Phase Details

### Phase 15: TypeScript Agent SDK (Client to Rust Evaluator)
**Goal**: A TypeScript SDK that connects to the Rust evaluator (running as a service) and exposes the core agent skills — getOperations, invoke, explain — without reimplementing trust-critical logic in a new language
**Depends on**: v1.0 milestone (complete)
**Why this first**: Ships fast, reuses the proven Rust evaluator, and the runtime dependency is explicit and manageable. The SDK is a client. The evaluator is the trusted core.
**Getting started options**:
  - Option A: `docker run tenor/evaluator` for local development
  - Option B: `tenor serve` CLI command that starts a local evaluator process
  - Option C: Hosted evaluator (future) for teams that don't want to manage infrastructure
**Plans**: TBD

Plans:
- [ ] 15-01: TBD

### Phase 16: TypeScript Code Generation (Optional Optimization)
**Goal**: Generate typed interfaces and client bindings directly from the contract, providing better IDE support and compile-time checking on top of the SDK
**Depends on**: Phase 15
**Why optional**: The SDK already works. Codegen adds better IDE support, compile-time checking, and a smoother developer experience. It is not required to use Tenor.
**Plans**: TBD

Plans:
- [ ] 16-01: TBD

### Phase 17: VS Code Extension
**Goal**: Syntax highlighting, LSP with check-on-save, and a Preview Agent Capabilities panel that shows what an agent would see when reading the contract — what an AI would know about your contract before you deploy it
**Depends on**: Phase 15 (needs tenor-core + tenor-analyze)
**Plans**: TBD

Plans:
- [ ] 17-01: TBD

### Phase 18: Agent Skill Examples
**Goal**: Reference implementations showing what's possible — making the abstract agent skills concrete and showing developers what they can build
**Depends on**: Phase 15
**Examples**:
  - `tenor agent` — CLI tool that turns any contract into an interactive shell
  - Express middleware — automatically generate routes from operations
  - Slack bot — let users interact with contracts via chat
  - Audit agent — generate compliance reports from provenance chains
**Plans**: TBD

Plans:
- [ ] 18-01: TBD

### Phase 19: Embedded Evaluator
**Goal**: Compile the Rust evaluator to WASM for embedding in browser, Node, and edge environments without a separate service
**Depends on**: Phase 18
**Why this is planned, not contingent**: Air-gapped and regulated industry deployments — healthcare, defense — cannot call a remote service. This is not an edge case. These are the buyers with the highest compliance requirements and the least tolerance for external dependencies. Embedded execution is required to serve them.
**Timeline**: After Phase 18. The client-service model ships first because it gets the SDK into developers' hands fastest. Embedded execution follows as a planned phase, not a research project.
**Plans**: TBD

Plans:
- [ ] 19-01: TBD

## Progress

**Execution Order:**
Phase 15 (SDK) -> 16 (codegen) -> 17 (VS Code) -> 18 (examples) -> 19 (embedded evaluator)

### Agent Tooling

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 15. TypeScript Agent SDK | 0/1 | Not started | - |
| 16. TypeScript Code Generation | 0/1 | Not started | - |
| 17. VS Code Extension | 0/1 | Not started | - |
| 18. Agent Skill Examples | 0/1 | Not started | - |
| 19. Embedded Evaluator | 0/1 | Not started | - |
