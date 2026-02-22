# Roadmap: Tenor v0.9 → v1.0

## Milestones

- **v0.9 Core** — Phases 1-5.1 (shipped 2026-02-22) — [archive](milestones/v0.9-ROADMAP.md)
- **v1.0 System Construct + Documentation** — Phases 12, 12.1, 13, 14 (next)
- **Developer Experience** — Phases 15-19 (planned, depends on v1.0)

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

### v1.0 — System Construct + Documentation

- [ ] **Phase 12: System Construct** - CFFP-driven design and full-stack implementation of the System construct for multi-contract composition
- [ ] **Phase 12.1: AAP Spec Audit** - Assumption Audit Protocol on complete v1.0 spec; gates freeze
- [ ] **Phase 13: Domain Re-validation** - All 5 domain contracts re-implemented for v1.0 spec with System scenarios
- [ ] **Phase 14: Documentation** - Language reference, authoring guide, executor guide covering full v1.0 spec

### Developer Experience (depends on v1.0)

- [ ] **Phase 15: TypeScript Code Generation** - Ports-and-adapters TypeScript target
- [ ] **Phase 16: Rust Code Generation** - Rust code generation target
- [ ] **Phase 17: Go Code Generation** - Go code generation target
- [ ] **Phase 18: Code Generation Guide** - Ports-and-adapters pattern documentation
- [ ] **Phase 19: VS Code Extension** - Syntax highlighting, inline errors, check-on-save

## Phase Details

### Phase 12: System Construct
**Goal**: Multi-contract composition is formally specified and fully implemented -- a System construct that declares member contracts, enables shared persona identity, cross-contract flow triggers, and cross-contract entity relationships, with elaboration, static analysis, and executor obligations all in place
**Depends on**: v0.9 milestone (complete)
**Requirements**: SYS-01, SYS-02, SYS-03, SYS-04, SYS-05, SYS-06, SYS-07, SYS-08, ANLZ-09, ANLZ-10, ANLZ-11, EXEC-01, EXEC-02
**Constraints:**
  - CFFP protocol required: invariant declaration, candidate formalisms, pressure testing, canonical form
  - System is a new construct touching the full stack: spec section, elaborator passes, interchange representation, static analysis, executor obligations, conformance tests
  - v0.9 spec is frozen -- System is the only additive change permitted before v1.0 freeze
**Success Criteria** (what must be TRUE):
  1. A user can author a System construct in `.tenor` that declares member contracts, and `tenor elaborate` produces valid interchange JSON containing the System
  2. Shared persona identity is expressible within a System -- the same actor name across member contracts is formally declared and the elaborator validates the persona exists in each referenced contract
  3. Cross-contract flow triggers are expressible -- a flow completion in one member contract can trigger a flow in another member contract, and the elaborator validates both flows exist
  4. `tenor check` on a System reports cross-contract authority topology findings (S4 extended) and cross-contract flow path findings (S6 extended)
  5. The interchange JSON Schema validates System construct documents, and conformance tests cover both positive and negative System elaboration cases
**Plans**: TBD

Plans:
- [ ] TBD (CFFP run required before planning)

### Phase 12.1: AAP Spec Audit
**Goal**: The complete v1.0 spec (including System construct) has been audited for hidden assumptions and fragility -- all findings resolved or documented as acknowledged limitations, gating the v1.0 freeze
**Depends on**: Phase 12
**Requirements**: SPEC-09, SPEC-10
**Success Criteria** (what must be TRUE):
  1. An AAP run has been executed against the complete v1.0 `docs/TENOR.md` and all hidden assumptions have been surfaced with fragility characterization
  2. Every AAP finding is either resolved (spec text amended) or documented as an acknowledged limitation in Appendix A with rationale for deferral
  3. The spec can be declared frozen at v1.0 with no uncharacterized assumptions remaining
**Plans**: TBD

Plans:
- [ ] 12.1-01: AAP run on complete v1.0 spec
- [ ] 12.1-02: Resolve AAP findings and freeze spec

### Phase 13: Domain Re-validation
**Goal**: All five domain contracts are re-implemented against the v1.0 spec and at least one multi-contract System scenario is validated end-to-end -- confirming the System construct works in realistic domain contexts
**Depends on**: Phase 12.1 (spec frozen at v1.0)
**Requirements**: DOMN-10, DOMN-11, DOMN-12, DOMN-13, DOMN-14, DOMN-15
**Success Criteria** (what must be TRUE):
  1. Each of the five domain contracts (SaaS, healthcare, supply chain, energy, trade finance) elaborates cleanly under the v1.0 spec and uses System construct features where applicable
  2. At least one multi-contract System scenario (e.g., two domain contracts composed into a System with shared personas and cross-contract flow triggers) validates end-to-end through elaboration, static analysis, and evaluation
  3. `tenor check` produces clean results (no new warnings beyond acknowledged limitations) for all re-implemented domain contracts
  4. Any spec gaps discovered during re-validation are documented (as in Phase 5's gap report pattern) and either resolved or acknowledged before v1.0 ships
**Plans**: TBD

Plans:
- [ ] 13-01: SaaS subscription contract re-implementation for v1.0
- [ ] 13-02: Healthcare prior auth contract re-implementation for v1.0
- [ ] 13-03: Supply chain inspection contract re-implementation for v1.0
- [ ] 13-04: Energy procurement contract re-implementation for v1.0
- [ ] 13-05: Trade finance contract re-implementation for v1.0
- [ ] 13-06: Multi-contract System scenario end-to-end validation

### Phase 14: Documentation
**Goal**: Tenor authors and executor implementers each have dedicated documentation covering the full v1.0 spec -- language reference, authoring guide, and executor guide ship with v1.0
**Depends on**: Phase 13 (documentation covers validated v1.0 spec including System construct and domain examples)
**Requirements**: DEVX-05, DEVX-06, DEVX-07
**Success Criteria** (what must be TRUE):
  1. Language reference documents every construct (Fact, Entity, Rule, Operation, Flow, TypeDecl, Persona, System) with author-facing examples distinct from the implementer spec
  2. Authoring guide walks through complete worked examples across multiple domains, including at least one System composition scenario
  3. Executor implementation guide explains how to build a runtime that correctly evaluates Tenor contracts including System composition, cross-contract snapshot coordination, and persona resolution
**Plans**: TBD

Plans:
- [ ] 14-01: Language reference (author-facing, mdBook)
- [ ] 14-02: Authoring guide with worked domain examples
- [ ] 14-03: Executor implementation guide

### Phase 15: TypeScript Code Generation
**Goal**: Users can generate TypeScript code from interchange bundles using a ports-and-adapters pattern, with generated code producing identical verdicts to the reference evaluator
**Depends on**: Phase 14
**Requirements**: CGEN-01, CGEN-02, CGEN-03, CGEN-04, CGEN-05, CGEN-07, CLI-08, TEST-04
**Success Criteria** (what must be TRUE):
  1. `tenor generate <bundle.json> --target typescript` produces compilable TypeScript
  2. Generated code exposes port interfaces for developer-supplied adapters
  3. `@tenor/adapters-local` package provides in-memory adapter implementations
  4. Generated TypeScript uses fixed-point decimal for Money and Decimal types
  5. Generated code produces the same verdicts as the reference evaluator
**Plans**: TBD

Plans:
- [ ] 15-01: Codegen crate with tera templates and ports-and-adapters TypeScript skeleton
- [ ] 15-02: Entity store, rule engine, and operation handler generation
- [ ] 15-03: Flow orchestrator, provenance collector, and port interfaces
- [ ] 15-04: Fixed-point decimal handling and numeric type mapping
- [ ] 15-05: `@tenor/adapters-local` package with in-memory implementations
- [ ] 15-06: `tenor generate` CLI subcommand and integration tests

### Phase 16: Rust Code Generation
**Goal**: Rust code generation target works end-to-end with conformance parity
**Depends on**: Phase 15
**Requirements**: CGEN-06, TEST-05
**Plans**: TBD

Plans:
- [ ] 16-01: Rust code generation templates and target implementation
- [ ] 16-02: Rust codegen integration tests (conformance parity)

### Phase 17: Go Code Generation
**Goal**: Go code generation target works end-to-end with conformance parity
**Depends on**: Phase 15
**Requirements**: CGEN-08, CGEN-09, TEST-06, TEST-10
**Plans**: TBD

Plans:
- [ ] 17-01: Go code generation templates and target implementation
- [ ] 17-02: Go codegen integration tests (conformance parity)

### Phase 18: Code Generation Guide
**Goal**: Documentation for the ports-and-adapters pattern and custom adapters
**Depends on**: Phase 15
**Requirements**: DEVX-08
**Plans**: TBD

Plans:
- [ ] 18-01: Code generation guide

### Phase 19: VS Code Extension
**Goal**: Tenor authors get real-time feedback in VS Code
**Depends on**: Phase 4 (needs tenor-core + tenor-analyze)
**Requirements**: DEVX-01, DEVX-02, DEVX-03, DEVX-04
**Plans**: TBD

Plans:
- [ ] 19-01: TextMate grammar and VS Code extension scaffold
- [ ] 19-02: LSP server with inline error diagnostics
- [ ] 19-03: Check-on-save and go-to-definition

## Progress

**Execution Order:**
12 (CFFP + implementation) -> 12.1 (AAP audit) -> 13 (domain re-validation) -> 14 (documentation)
Note: Phase 14 depends on Phase 13 so documentation covers validated v1.0 spec.
Note: Developer Experience phases (15-19) depend on v1.0 completion.

### v1.0 — System Construct + Documentation

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 12. System Construct | 0/TBD | Not started | - |
| 12.1. AAP Spec Audit | 0/2 | Not started | - |
| 13. Domain Re-validation | 0/6 | Not started | - |
| 14. Documentation | 0/3 | Not started | - |

### Developer Experience

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 15. TypeScript Code Generation | 0/6 | Not started | - |
| 16. Rust Code Generation | 0/2 | Not started | - |
| 17. Go Code Generation | 0/2 | Not started | - |
| 18. Code Generation Guide | 0/1 | Not started | - |
| 19. VS Code Extension | 0/3 | Not started | - |
