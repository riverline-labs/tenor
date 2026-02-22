# Roadmap: Tenor v0.9 → v1.0

## Milestones

- **v0.9 Core** — Phases 1-5.1 (shipped 2026-02-22) — [archive](milestones/v0.9-ROADMAP.md)
- **v1.0 System Construct + Documentation** — Phases 12, 6 (next)
- **Developer Experience** — Phases 7-11 (planned, depends on M2)

## Phases

<details>
<summary>v0.9 Core (Phases 1-5.1) — SHIPPED 2026-02-22</summary>

- [x] Phase 1: Spec Completion (5/5 plans)
- [x] Phase 01.1: Spec CI — AI Ambiguity Testing (2/2 plans)
- [x] Phase 2: Foundation (4/4 plans)
- [x] Phase 3: CLI + Evaluator (7/7 plans)
- [x] Phase 3.1: CFFP — Migration Semantics (2/2 plans)
- [x] Phase 3.2: Technical Debt & Bug Fixes (3/3 plans)
- [x] Phase 3.3: Flow Migration Compatibility (2/2 plans)
- [x] Phase 3.4: Contract Discovery (2/2 plans)
- [x] Phase 4: Static Analysis (8/8 plans)
- [x] Phase 5: Domain Validation (8/8 plans)
- [x] Phase 5.1: Fix Critical DSL Gaps (3/3 plans)

14 phases, 46 plans. Full details: [v0.9-ROADMAP.md](milestones/v0.9-ROADMAP.md)

</details>

### v1.0 — System Construct + Documentation

- [ ] **Phase 12: System Construct** - CFFP-driven design of System construct for multi-contract composition
- [ ] **Phase 6: Documentation** - Language reference, authoring guide, executor guide (covers full v1.0 spec)

### Developer Experience (depends on v1.0)

- [ ] **Phase 7: TypeScript Code Generation** - Ports-and-adapters TypeScript target
- [ ] **Phase 8: Rust Code Generation** - Rust code generation target
- [ ] **Phase 9: Go Code Generation** - Go code generation target
- [ ] **Phase 10: Code Generation Guide** - Ports-and-adapters pattern documentation
- [ ] **Phase 11: VS Code Extension** - Syntax highlighting, inline errors, check-on-save

## Phase Details

### Phase 12: System Construct
**Goal**: Multi-contract composition — a System construct that formally relates contracts to each other, enabling shared persona identity, cross-contract flow triggers, and cross-contract entity relationships. Gates v1.0 spec freeze.
**Depends on**: v0.9 milestone (complete)
**Requirements**: TBD (CFFP run will derive requirements)
**Constraints:**
  - CFFP protocol required: invariant declaration, candidate formalisms, pressure testing, canonical form
  - System is a new construct touching the full stack: spec section, elaborator passes, interchange representation, static analysis, executor obligations, conformance tests
  - v0.9 spec is frozen — System is the only additive change permitted before v1.0 freeze
**Success Criteria** (what must be TRUE):
  1. System construct has formal syntax, semantics, and interchange representation in `docs/TENOR.md`
  2. Member contracts can be declared within a System
  3. Shared persona identity is expressible (same actor across contracts)
  4. Cross-contract flow triggers are expressible (completion of flow A initiates flow B)
  5. Cross-contract entity relationships are expressible (entity in contract A is the same entity in contract B)
  6. Elaborator validates System constructs (Pass 5) and serializes to interchange (Pass 6)
  7. Static analysis extended for cross-contract authority topology and flow path enumeration
  8. Executor obligations defined for cross-contract snapshot coordination and persona resolution
  9. Conformance tests cover System construct elaboration
**Plans**: TBD (run /gsd:plan-phase 12 after CFFP)

Plans:
- [ ] TBD (CFFP run required before planning)

### Phase 6: Documentation
**Goal**: Tenor authors and executor implementers each have dedicated documentation covering their use case — language reference, authoring guide, and executor guide ship with v1.0
**Depends on**: Phase 12 (documentation covers full v1.0 spec including System construct)
**Requirements**: DEVX-05, DEVX-06, DEVX-07
**Success Criteria** (what must be TRUE):
  1. Language reference documents every construct, type, and expression form with author-facing examples (distinct from the implementer spec)
  2. Authoring guide walks through complete worked examples across multiple domains
  3. Executor implementation guide explains how to build a runtime that correctly evaluates Tenor contracts
**Plans**: TBD

Plans:
- [ ] 06-01: Language reference (author-facing, mdBook)
- [ ] 06-02: Authoring guide with worked domain examples
- [ ] 06-03: Executor implementation guide

### Phase 7: TypeScript Code Generation
**Goal**: Users can generate TypeScript code from interchange bundles using a ports-and-adapters pattern, with generated code producing identical verdicts to the reference evaluator
**Depends on**: Phase 6
**Requirements**: CGEN-01, CGEN-02, CGEN-03, CGEN-04, CGEN-05, CGEN-07, CLI-08, TEST-04
**Success Criteria** (what must be TRUE):
  1. `tenor generate <bundle.json> --target typescript` produces compilable TypeScript
  2. Generated code exposes port interfaces for developer-supplied adapters
  3. `@tenor/adapters-local` package provides in-memory adapter implementations
  4. Generated TypeScript uses fixed-point decimal for Money and Decimal types
  5. Generated code produces the same verdicts as the reference evaluator
**Plans**: TBD

Plans:
- [ ] 07-01: Codegen crate with tera templates and ports-and-adapters TypeScript skeleton
- [ ] 07-02: Entity store, rule engine, and operation handler generation
- [ ] 07-03: Flow orchestrator, provenance collector, and port interfaces
- [ ] 07-04: Fixed-point decimal handling and numeric type mapping
- [ ] 07-05: `@tenor/adapters-local` package with in-memory implementations
- [ ] 07-06: `tenor generate` CLI subcommand and integration tests

### Phase 8: Rust Code Generation
**Goal**: Rust code generation target works end-to-end with conformance parity
**Depends on**: Phase 7
**Requirements**: CGEN-06, TEST-05
**Plans**: TBD

Plans:
- [ ] 08-01: Rust code generation templates and target implementation
- [ ] 08-02: Rust codegen integration tests (conformance parity)

### Phase 9: Go Code Generation
**Goal**: Go code generation target works end-to-end with conformance parity
**Depends on**: Phase 7
**Requirements**: CGEN-08, CGEN-09, TEST-06, TEST-10
**Plans**: TBD

Plans:
- [ ] 09-01: Go code generation templates and target implementation
- [ ] 09-02: Go codegen integration tests (conformance parity)

### Phase 10: Code Generation Guide
**Goal**: Documentation for the ports-and-adapters pattern and custom adapters
**Depends on**: Phase 7
**Requirements**: DEVX-08
**Plans**: TBD

Plans:
- [ ] 10-01: Code generation guide

### Phase 11: VS Code Extension
**Goal**: Tenor authors get real-time feedback in VS Code
**Depends on**: Phase 4 (needs tenor-core + tenor-analyze)
**Requirements**: DEVX-01, DEVX-02, DEVX-03, DEVX-04
**Plans**: TBD

Plans:
- [ ] 11-01: TextMate grammar and VS Code extension scaffold
- [ ] 11-02: LSP server with inline error diagnostics
- [ ] 11-03: Check-on-save and go-to-definition

## Progress

**Execution Order:**
12 (CFFP) -> 6 -> 7 -> 8, 9 (parallel) -> 10 -> 11
Note: Phase 8 (Rust) and Phase 9 (Go) both depend on Phase 7 (TypeScript) and can execute in parallel.
Note: Phase 11 (VS Code) depends on Phase 4 (already complete) and could start earlier if needed.

### v1.0 — System Construct + Documentation

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 12. System Construct (CFFP) | 0/TBD | Not started | - |
| 6. Documentation | 0/3 | Not started | - |

### Developer Experience

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 7. TypeScript Code Generation | 0/6 | Not started | - |
| 8. Rust Code Generation | 0/2 | Not started | - |
| 9. Go Code Generation | 0/2 | Not started | - |
| 10. Code Generation Guide | 0/1 | Not started | - |
| 11. VS Code Extension | 0/3 | Not started | - |
