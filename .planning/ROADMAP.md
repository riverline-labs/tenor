# Roadmap: Tenor v1.0

## Overview

Tenor goes from a working v0.3 elaborator to a complete 1.0 language toolchain in nine phases. The first two phases stabilize what exists (spec completion via CFFP, then extracting `tenor-core` from the monolith). Phases 3-4 build the evaluation and analysis engines. Phase 5 is the hard gate: real contracts across five domains must validate the spec before any code generation. Phases 6-7 deliver TypeScript and Rust code generation with CI integration. Phases 8-9 round out developer experience with IDE tooling and documentation.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Spec Completion** - CFFP-driven formalization of persona, outcome typing, shared types, and interchange versioning
- [ ] **Phase 2: Foundation** - Extract tenor-core library crate, establish Cargo workspace, extend conformance suite
- [ ] **Phase 3: CLI + Evaluator** - Unified `tenor` binary with core subcommands and provenance-traced evaluator
- [ ] **Phase 4: Static Analysis** - S1-S7 analysis suite with structured output and `tenor check` integration
- [ ] **Phase 5: Domain Validation** - Five real contracts across distinct domains proving spec completeness
- [ ] **Phase 6: TypeScript Code Generation** - Ports-and-adapters TypeScript target with local adapter package
- [ ] **Phase 7: Rust + Go Code Generation** - Rust and Go code generation targets with integration tests
- [ ] **Phase 8: VS Code Extension** - Syntax highlighting, inline errors, check-on-save, go-to-definition
- [ ] **Phase 9: Documentation** - Language reference, authoring guide, executor guide, codegen guide

## Phase Details

### Phase 1: Spec Completion
**Goal**: The Tenor v1.0 language specification is complete and frozen -- persona, outcome typing, and shared types are formally designed through CFFP, and the interchange format is versioned with a JSON Schema
**Depends on**: Nothing (first phase)
**Requirements**: SPEC-01, SPEC-02, SPEC-03, SPEC-04, SPEC-05
**Success Criteria** (what must be TRUE):
  1. Persona construct has formal syntax, semantics, and interchange representation in `docs/TENOR.md`
  2. Operation outcome types are statically enumerable with named variants specified in the spec
  3. Shared type library has import semantics for cross-contract Record and TaggedUnion reuse
  4. Interchange JSON includes `tenor_version` field and validates against a published JSON Schema
  5. Each of SPEC-01, SPEC-02, SPEC-03 has a completed CFFP artifact (invariants declared, candidates tested, canonical form chosen)
**Plans:** 5 plans

Plans:
- [x] 01-01-PLAN.md — CFFP run for Persona construct + spec section
- [ ] 01-02-PLAN.md — CFFP run for P7 Operation outcome typing + spec updates
- [ ] 01-03-PLAN.md — CFFP run for P5 Shared type library + spec section
- [ ] 01-04-PLAN.md — Interchange versioning semantics + JSON Schema
- [ ] 01-05-PLAN.md — Spec consistency review and v1.0 freeze

### Phase 2: Foundation
**Goal**: The monolithic elaborator is refactored into a Cargo workspace with `tenor-core` exposing typed pass outputs as public API, all existing tests pass, and conformance suite covers new v1.0 constructs
**Depends on**: Phase 1
**Requirements**: FNDN-01, FNDN-02, FNDN-03, FNDN-04, TEST-01, TEST-02, TEST-08
**Success Criteria** (what must be TRUE):
  1. `elaborate.rs` is decomposed into per-pass modules within a `tenor-core` library crate
  2. Cargo workspace contains separate crates for core, cli, eval, analyze, codegen, and lsp
  3. All 47 original conformance tests continue to pass without modification
  4. Downstream crates can import and use typed AST, Index, and TypeEnv from `tenor-core`
  5. Conformance suite includes tests for persona declaration, P7 outcome typing, P5 shared types, and interchange schema validation
  6. CI pipeline runs all conformance suites on every commit
**Plans**: TBD

Plans:
- [ ] 02-01: Extract per-pass modules from elaborate.rs into tenor-core
- [ ] 02-02: Cargo workspace setup with crate boundaries
- [ ] 02-03: Public API design for intermediate pass outputs
- [ ] 02-04: Implement spec additions (persona, P7, P5) in elaborator
- [ ] 02-05: Conformance suite extension and interchange schema tests
- [ ] 02-06: CI pipeline setup (runs all conformance suites on every commit)

### Phase 3: CLI + Evaluator
**Goal**: Users can elaborate, validate, evaluate, and test contracts through a unified `tenor` command-line tool, with the evaluator producing provenance-traced verdicts against fact sets
**Depends on**: Phase 2
**Requirements**: CLI-01, CLI-02, CLI-03, CLI-05, CLI-07, CLI-09, EVAL-01, EVAL-02, EVAL-03, EVAL-04, EVAL-05, EVAL-06, EVAL-07, TEST-07, TEST-09
**Success Criteria** (what must be TRUE):
  1. `tenor elaborate <file>` produces interchange JSON to stdout identical to current elaborator output
  2. `tenor validate <bundle.json>` validates interchange against the formal JSON Schema
  3. `tenor eval <bundle.json> --facts <facts.json>` produces a verdict set where every verdict carries its complete derivation chain
  4. `tenor test` runs the full conformance suite and reports results
  5. Evaluator correctly implements frozen verdict semantics and fixed-point numeric arithmetic matching the spec NumericModel
**Plans**: TBD

Plans:
- [ ] 03-01: CLI shell with clap (elaborate, validate, test subcommands)
- [ ] 03-02: Evaluator core -- bundle + facts to verdicts with provenance
- [ ] 03-03: Frozen verdict semantics and numeric model implementation
- [ ] 03-04: Evaluator conformance suite (including frozen verdicts and numeric precision)
- [ ] 03-05: CLI integration tests and eval subcommand wiring
- [ ] 03-06: Numeric precision regression suite (shared across elaborator and evaluator)

### Phase 4: Static Analysis
**Goal**: Users can run `tenor check` to get comprehensive static analysis (S1-S7) of their contracts, with structured output consumable by both CLI and future LSP
**Depends on**: Phase 3
**Requirements**: ANLZ-01, ANLZ-02, ANLZ-03, ANLZ-04, ANLZ-05, ANLZ-06, ANLZ-07, ANLZ-08, CLI-04, TEST-03
**Success Criteria** (what must be TRUE):
  1. `tenor check <file>` runs elaboration followed by all seven static analyses and reports findings
  2. Entity state space enumeration (S1), rule stratum reachability (S2), and domain coverage (S3a) produce correct results on known contracts
  3. Authority topology (S4), operation effect analysis (S5), flow path enumeration (S6), and complexity bounds (S7) produce correct results on known contracts
  4. Analyzer output is structured (not just text) and suitable for programmatic consumption by CLI and LSP
  5. Each S1-S7 analysis has test coverage with known-good and known-bad contracts
**Plans**: TBD

Plans:
- [ ] 04-01: Analyzer crate structure and S1 entity state space enumeration
- [ ] 04-02: S2 rule stratum reachability and S3a domain coverage
- [ ] 04-03: S4 authority topology and S5 operation effect analysis
- [ ] 04-04: S6 flow path enumeration and S7 complexity bounds
- [ ] 04-05: Structured output format and `tenor check` CLI integration
- [ ] 04-06: Analyzer test suite (known-good and known-bad contracts per analysis)

### Phase 5: Domain Validation
**Goal**: Five real contracts across distinct business domains elaborate, pass static analysis, and evaluate correctly -- proving the spec handles real-world complexity before code generation begins
**Depends on**: Phase 4
**Requirements**: DOMN-01, DOMN-02, DOMN-03, DOMN-04, DOMN-05, DOMN-06, DOMN-07, DOMN-08, DOMN-09, CLI-06
**Success Criteria** (what must be TRUE):
  1. Multi-tenant SaaS contract (seats, feature flags, subscriptions) elaborates, checks, and evaluates with correct provenance
  2. Healthcare prior auth contract (policy rules, peer review, appeals) elaborates, checks, and evaluates with correct provenance
  3. Supply chain contract (customs, inspection, release gates) elaborates, checks, and evaluates with correct provenance
  4. Internal procurement contract (approval tiers, delegation, budget) and financial domain contract each elaborate, check, and evaluate correctly
  5. `tenor explain <bundle.json>` produces a human-readable contract summary, and a spec gap report documents all findings from domain validation
**Plans**: TBD

Plans:
- [ ] 05-01: SaaS contract (seat limits, feature flags, subscription state)
- [ ] 05-02: Healthcare prior auth contract (policy rules, peer review, appeals)
- [ ] 05-03: Supply chain contract (customs, inspection, release gates)
- [ ] 05-04: Procurement contract (approval tiers, delegation, budget)
- [ ] 05-05: Financial domain contract (lending, escrow, or compliance)
- [ ] 05-06: `tenor explain` subcommand implementation
- [ ] 05-07: Spec gap report and findings synthesis

### Phase 6: TypeScript Code Generation
**Goal**: Users can generate TypeScript code from interchange bundles using a ports-and-adapters pattern, with generated code producing identical verdicts to the reference evaluator
**Depends on**: Phase 5
**Requirements**: CGEN-01, CGEN-02, CGEN-03, CGEN-04, CGEN-05, CGEN-07, CLI-08, TEST-04
**Success Criteria** (what must be TRUE):
  1. `tenor generate <bundle.json> --target typescript` produces compilable TypeScript implementing entity store, rule engine, operation handlers, flow orchestrator, and provenance collector
  2. Generated code exposes port interfaces (fact sources, persona resolver, state store, provenance repo) for developer-supplied adapters
  3. `@tenor/adapters-local` package provides in-memory adapter implementations for dev/test use
  4. Generated TypeScript uses fixed-point decimal (not native `number`) for Money and Decimal types
  5. Generated code produces the same verdicts as the reference evaluator when run against the evaluator conformance suite
**Plans**: TBD

Plans:
- [ ] 06-01: Codegen crate with tera templates and ports-and-adapters TypeScript skeleton
- [ ] 06-02: Entity store, rule engine, and operation handler generation
- [ ] 06-03: Flow orchestrator, provenance collector, and port interfaces
- [ ] 06-04: Fixed-point decimal handling and numeric type mapping
- [ ] 06-05: `@tenor/adapters-local` package with in-memory implementations
- [ ] 06-06: `tenor generate` CLI subcommand and integration tests

### Phase 7: Rust + Go Code Generation
**Goal**: Rust and Go code generation targets work end-to-end, with generated code producing identical verdicts to the reference evaluator, and domain validation contracts serving as integration tests
**Depends on**: Phase 6
**Requirements**: CGEN-06, CGEN-08, CGEN-09, TEST-05, TEST-06, TEST-10
**Success Criteria** (what must be TRUE):
  1. `tenor generate <bundle.json> --target rust` produces compilable Rust code using the same ports-and-adapters pattern as TypeScript
  2. Generated Rust code produces the same verdicts as the reference evaluator against the conformance suite
  3. `tenor generate <bundle.json> --target go` produces compilable Go code using the same ports-and-adapters pattern
  4. Generated Go code produces the same verdicts as the reference evaluator against the conformance suite
  5. Domain validation contracts serve as end-to-end integration tests (elaborate, check, eval, generate, run) across all three targets
**Plans**: TBD

Plans:
- [ ] 07-01: Rust code generation templates and target implementation
- [ ] 07-02: Rust codegen integration tests (conformance parity)
- [ ] 07-03: Go code generation templates and target implementation
- [ ] 07-04: Go codegen integration tests (conformance parity)
- [ ] 07-05: Domain contracts as end-to-end integration tests across all targets

### Phase 8: VS Code Extension
**Goal**: Tenor authors get real-time feedback in VS Code with syntax highlighting, inline errors, check-on-save, and go-to-definition for construct references
**Depends on**: Phase 4 (needs tenor-core + tenor-analyze)
**Requirements**: DEVX-01, DEVX-02, DEVX-03, DEVX-04
**Success Criteria** (what must be TRUE):
  1. `.tenor` files have syntax highlighting via TextMate grammar in VS Code
  2. Elaboration errors appear as inline diagnostics at the correct file and line
  3. Saving a `.tenor` file automatically runs `tenor check` and displays results
  4. Go-to-definition navigates from construct references to their declarations
**Plans**: TBD

Plans:
- [ ] 08-01: TextMate grammar and VS Code extension scaffold
- [ ] 08-02: LSP server with inline error diagnostics
- [ ] 08-03: Check-on-save and go-to-definition

### Phase 9: Documentation
**Goal**: Tenor authors, executor implementers, and code generation consumers each have dedicated documentation covering their use case
**Depends on**: Phase 6 (needs codegen examples)
**Requirements**: DEVX-05, DEVX-06, DEVX-07, DEVX-08
**Success Criteria** (what must be TRUE):
  1. Language reference documents every construct, type, and expression form with author-facing examples (distinct from the implementer spec)
  2. Authoring guide walks through complete worked examples across multiple domains
  3. Executor implementation guide explains how to build a runtime that correctly evaluates Tenor contracts
  4. Code generation guide explains the ports-and-adapters pattern and how to write custom adapters
**Plans**: TBD

Plans:
- [ ] 09-01: Language reference (author-facing, mdBook)
- [ ] 09-02: Authoring guide with worked domain examples
- [ ] 09-03: Executor implementation guide
- [ ] 09-04: Code generation guide

## Progress

**Execution Order:**
Phases execute in numeric order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8 -> 9
Note: Phase 8 (VS Code) depends on Phase 4 (not Phase 7) and could execute in parallel with Phases 5-7 if desired.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Spec Completion | 1/5 | In progress | - |
| 2. Foundation | 0/6 | Not started | - |
| 3. CLI + Evaluator | 0/6 | Not started | - |
| 4. Static Analysis | 0/6 | Not started | - |
| 5. Domain Validation | 0/7 | Not started | - |
| 6. TypeScript Code Generation | 0/6 | Not started | - |
| 7. Rust + Go Code Generation | 0/5 | Not started | - |
| 8. VS Code Extension | 0/3 | Not started | - |
| 9. Documentation | 0/4 | Not started | - |
