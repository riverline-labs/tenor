# Roadmap: Tenor

## Milestones

- ✅ **v0.9 Core** — Phases 1-5.1 (shipped 2026-02-22) — [archive](milestones/v0.9-ROADMAP.md)
- ✅ **v1.0 System Construct + Documentation** — Phases 12-14 (shipped 2026-02-22) — [archive](milestones/v1.0-ROADMAP.md)
- ✅ **Agent Tooling** — Phases 14.1-16 (shipped 2026-02-23)
- 🚧 **Platform & Ecosystem** — Phases 17-24 (current)

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

4 phases, 17 plans. Full details: [v1.0-ROADMAP.md](milestones/v1.0-ROADMAP.md)

</details>

<details>
<summary>Agent Tooling (Phases 14.1-16) — SHIPPED 2026-02-23</summary>

- [x] Phase 14.1: Tech Debt, Bugs & Hardening (5/5 plans)
- [x] Phase 15: TypeScript Agent SDK (3/3 plans)
- [x] Phase 16: TypeScript Code Generation (2/2 plans)

3 phases, 10 plans.

</details>

### 🚧 Platform & Ecosystem (Phases 17-24)

- [x] **Phase 17: VS Code Extension** - Syntax highlighting, LSP, Preview Agent Capabilities panel (completed 2026-02-23)
- [ ] **Phase 18: Agent Skill Examples** - Reference implementations (CLI tool, Express middleware, Slack bot, audit agent)
- [ ] **Phase 19: AI Authoring Assistant** - Prompt templates and workflows for AI-assisted contract authoring (docs, not software)
- [ ] **Phase 20: Embedded Evaluator** - WASM-compiled Rust evaluator for browser, Node, and edge environments
- [ ] **Phase 21: Hosted Evaluator Service** - Managed API endpoint, removes infrastructure barrier
- [ ] **Phase 22: Domain Contract Library** - Curated contracts, community contribution framework
- [ ] **Phase 23: Rust and Go Agent SDKs** - Same skills, same trust model, Rust for systems, Go for backend
- [ ] **Phase 24: Multi-party Contract Execution** - Independent verification, full trust model in production (capstone)

## Phase Details

### Phase 14.1: Tech Debt, Bugs & Hardening (INSERTED)
**Goal**: Resolve all critical tech debt, known bugs, fragile areas, and security gaps identified in CONCERNS.md before starting the Agent Tooling SDK work
**Depends on**: v1.0 milestone (complete)
**Why now**: These issues range from "will bite you immediately" (duplicated manifest logic, hardcoded versions, unwrap panics) to "catastrophic if embedded in a server" (unsandboxed imports, date validation). Fixing them now prevents compounding tech debt as the SDK builds on top of tenor-core and tenor-eval.

**Scope by urgency tier:**

**Fix immediately** (will bite you):
- Duplicated manifest/etag logic — conformance tests can silently diverge from CLI output
- Hardcoded version strings in 9+ locations — next version bump is a debugging nightmare
- Stale `"tenor_version": "1.0.0"` in test helpers — silent inconsistency
- `pass5_validate.rs` unwrap() calls — elaborator panics on user input if invariants shift
- `explain.rs` untyped JSON traversal — silently drops output sections if interchange format changes

**Fix before Phase 15 ships:**
- Import path traversal not sandboxed — catastrophic if tenor-core gets embedded in a server
- Date/DateTime validation bugs — accepts garbage dates
- `generate` stub exits with code 2 — runtime error with no explanation
- O(n) operation lookup per flow step — will hurt once real contracts run through the SDK

**Fix before Phase 19:**
- Flow step limit of 1000 is unconfigurable — legitimate complex flows fail silently
- S6 path enumeration caps at 10,000 — truncation means incomplete analysis
- ureq error string parsing for HTTP status codes — retry logic silently breaks if ureq changes formatting

**Address but not blocking:**
- No unit tests in tenor-core source files — regressions hard to isolate
- Thin negative test coverage for passes 0-4
- No eval conformance fixtures for flow error paths
- AmbiguityRunResult fields all dead code — stats computed, never surfaced
- Hardcoded model name in ambiguity testing — use a non-dated alias

**Low priority / track:**
- Excessive string allocations in pass6_serialize — irrelevant for CLI, matters if embedded
- Linear import cycle detection — only hurts on pathologically deep import trees
- Markdown format output untested in explain.rs
- `tenor diff` not covered by CLI integration tests

**Plans**: 5 plans

Plans:
- [ ] 14.1-01-PLAN.md — Centralize version constants + extract manifest/etag module
- [ ] 14.1-02-PLAN.md — Convert unwrap() sites to proper error handling (passes 3/4/5)
- [ ] 14.1-03-PLAN.md — Import path sandboxing + date/datetime validation fix
- [ ] 14.1-04-PLAN.md — Harden explain.rs with typed ExplainBundle struct
- [ ] 14.1-05-PLAN.md — O(1) operation lookup + configurable limits + cleanup

### Phase 15: TypeScript Agent SDK (Client to Rust Evaluator)
**Goal**: A TypeScript SDK that connects to the Rust evaluator (running as a service) and exposes the core agent skills — getOperations, invoke, explain — without reimplementing trust-critical logic in a new language
**Depends on**: v1.0 milestone (complete)
**Why this first**: Ships fast, reuses the proven Rust evaluator, and the runtime dependency is explicit and manageable. The SDK is a client. The evaluator is the trusted core.
**Getting started options**:
  - Option A: `docker run tenor/evaluator` for local development
  - Option B: `tenor serve` CLI command that starts a local evaluator process
  - Option C: Hosted evaluator (future) for teams that don't want to manage infrastructure
**Plans**: 3 plans

Plans:
- [x] 15-01-PLAN.md — `tenor serve` HTTP JSON API server (Rust evaluator as a service)
- [x] 15-02-PLAN.md — TypeScript SDK package (`@tenor-lang/sdk`) with agent skill methods
- [ ] 15-03-PLAN.md — Docker image, SDK documentation, and example script

### Phase 16: TypeScript Code Generation (Optional Optimization)
**Goal**: Generate typed interfaces and client bindings directly from the contract, providing better IDE support and compile-time checking on top of the SDK
**Depends on**: Phase 15
**Why optional**: The SDK already works. Codegen adds better IDE support, compile-time checking, and a smoother developer experience. It is not required to use Tenor.
**Plans**: 2 plans

Plans:
- [ ] 16-01-PLAN.md — Implement tenor-codegen TypeScript type/schema emitters + wire CLI generate command
- [ ] 16-02-PLAN.md — Add typed client wrapper generator, barrel exports, and integration tests

### Phase 17: VS Code Extension
**Goal**: Syntax highlighting, LSP with check-on-save, and a Preview Agent Capabilities panel that shows what an agent would see when reading the contract — what an AI would know about your contract before you deploy it
**Depends on**: Phase 15 (needs tenor-core + tenor-analyze)
**Plans**: 5 plans

Plans:
- [ ] 17-01-PLAN.md — VS Code extension scaffolding + TextMate grammar for syntax highlighting
- [ ] 17-02-PLAN.md — LSP server with diagnostics and semantic tokens
- [ ] 17-03-PLAN.md — LSP navigation: go-to-definition, find-all-references, hover, completions
- [ ] 17-04-PLAN.md — Agent Capabilities webview panel with SVG state diagrams
- [ ] 17-05-PLAN.md — Command palette, status bar, snippets, and end-to-end polish

### Phase 18: Agent Skill Examples
**Goal**: Reference implementations showing what's possible with the SDK — tenor agent CLI, Express middleware, Slack bot, regulatory audit agent
**Plans**: TBD

Plans:
- [ ] 18-01: TBD

### Phase 19: AI Authoring Assistant
**Goal**: A Tenor contract authoring guide designed to be used with any AI assistant. Includes prompt templates that give an AI assistant sufficient context to help author Tenor contracts, example conversations demonstrating three suggested workflows (autonomous: describe and walk away; collaborative: human steers, AI executes; reviewed: AI summarizes contract in plain language for business approval), and guidance on when to ask questions versus make decisions. Deliverable is documentation in the repo — markdown files, not software.
**Depends on**: Phase 17
**Plans**: TBD

Plans:
- [ ] 19-01: TBD

### Phase 20: Embedded Evaluator
**Goal**: WASM-compiled Rust evaluator for browser, Node, and edge environments. Primary use cases are air-gapped and regulated industry deployments — healthcare, defense — where calling a remote service is not an option. Planned phase, not contingent on demand.
**Plans**: TBD

Plans:
- [ ] 20-01: TBD

### Phase 21: Hosted Evaluator Service
**Goal**: Managed API endpoint that removes the last infrastructure barrier — monetization path if desired
**Plans**: TBD

Plans:
- [ ] 21-01: TBD

### Phase 22: Domain Contract Library
**Goal**: Curated contracts for common industries, community contribution framework, the five existing domains as seed
**Depends on**: Phase 21
**Plans**: TBD

Plans:
- [ ] 22-01: TBD

### Phase 23: Rust and Go Agent SDKs
**Goal**: Same skills, same executor API, same trust model. Rust for embedded and systems teams, Go for backend and infrastructure teams. Both matter for healthcare and defense use cases.
**Plans**: TBD

Plans:
- [ ] 23-01: TBD

### Phase 24: Multi-party Contract Execution
**Goal**: Two or more parties, one contract, independent verification — no "our records vs their records." The full trust model realized in production. Capstone phase.
**Depends on**: Phase 21, Phase 23
**Plans**: TBD

Plans:
- [ ] 24-01: TBD

## Progress

**Execution Order:**
Phase 14.1 (tech debt/bugs) -> 15 (SDK) -> 16 (codegen) -> 17 (VS Code) -> 18 (examples) -> 19 (AI authoring) -> 20 (embedded evaluator) -> 21 (hosted evaluator) -> 22 (domain library) -> 23 (Rust/Go SDKs) -> 24 (multi-party)

### Agent Tooling (SHIPPED)

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 14.1 Tech Debt, Bugs & Hardening | 5/5 | Complete | 2026-02-23 |
| 15. TypeScript Agent SDK | 3/3 | Complete | 2026-02-23 |
| 16. TypeScript Code Generation | 2/2 | Complete | 2026-02-23 |

### Platform & Ecosystem

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 17. VS Code Extension | 5/5 | Complete   | 2026-02-23 |
| 18. Agent Skill Examples | 0/1 | Not started | - |
| 19. AI Authoring Assistant | 0/1 | Not started | - |
| 20. Embedded Evaluator | 0/1 | Not started | - |
| 21. Hosted Evaluator Service | 0/1 | Not started | - |
| 22. Domain Contract Library | 0/1 | Not started | - |
| 23. Rust and Go Agent SDKs | 0/1 | Not started | - |
| 24. Multi-party Contract Execution | 0/1 | Not started | - |
