# Requirements: Tenor v1.0

**Defined:** 2026-02-21
**Core Value:** A contract authored in TenorDSL must be statically verifiable, evaluable against facts, and generatable into working code — the full lifecycle from specification to execution with provenance at every step.

## v1 Requirements

Requirements for 1.0 release. Each maps to roadmap phases.

### Spec Completion

- [ ] **SPEC-01**: Persona declared as first-class construct with id and optional metadata in spec and elaborator
- [ ] **SPEC-02**: Operation outcome typing — named outcome types on Operations, statically enumerable, specified via CFFP
- [ ] **SPEC-03**: Shared type library — cross-contract type reuse for Record and TaggedUnion with import semantics, specified via CFFP
- [ ] **SPEC-04**: Interchange format versioned with `tenor_version` field and formal JSON Schema
- [ ] **SPEC-05**: Each spec change (SPEC-01, SPEC-02, SPEC-03) run through CFFP with invariant declaration, candidate formalisms, pressure testing, and canonicalization before implementation

### Foundation

- [ ] **FNDN-01**: Monolithic `elaborate.rs` refactored into typed per-pass modules within `tenor-core` library crate
- [ ] **FNDN-02**: Cargo workspace with separate crates: `tenor-core`, `tenor-cli`, `tenor-eval`, `tenor-analyze`, `tenor-codegen`, `tenor-lsp`
- [ ] **FNDN-03**: Existing 47 conformance tests continue to pass after refactoring (backward compatibility)
- [ ] **FNDN-04**: Intermediate pass outputs (typed AST, Index, TypeEnv) exposed as public API from `tenor-core`

### CLI

- [ ] **CLI-01**: Unified `tenor` binary with subcommands: elaborate, validate, check, eval, explain, test, generate
- [ ] **CLI-02**: `tenor elaborate <file.tenor>` produces interchange JSON to stdout
- [ ] **CLI-03**: `tenor validate <bundle.json>` validates interchange against formal JSON Schema
- [ ] **CLI-04**: `tenor check <file.tenor>` runs elaboration + S1-S7 static analysis
- [ ] **CLI-05**: `tenor eval <bundle.json> --facts <facts.json>` evaluates contract against provided facts
- [ ] **CLI-06**: `tenor explain <bundle.json>` produces human-readable contract summary
- [ ] **CLI-07**: `tenor test` runs conformance suite
- [ ] **CLI-08**: `tenor generate <bundle.json> --target typescript` generates code from interchange
- [ ] **CLI-09**: CLI supports `--output` format flags, `--quiet` for CI, and meaningful exit codes for scripting

### Evaluator

- [ ] **EVAL-01**: Evaluator accepts interchange bundle + facts JSON and produces verdict set with provenance
- [ ] **EVAL-02**: Every verdict carries complete derivation chain (provenance-traced evaluation)
- [ ] **EVAL-03**: Evaluator correctly implements frozen verdict semantics (Flow snapshots are immutable)
- [ ] **EVAL-04**: Evaluator handles numeric types with fixed-point arithmetic matching spec NumericModel
- [ ] **EVAL-05**: Evaluator conformance suite with dedicated test fixtures (separate from elaborator conformance)
- [ ] **EVAL-06**: Evaluator conformance suite includes frozen verdict semantics edge cases
- [ ] **EVAL-07**: Evaluator conformance suite includes numeric precision edge cases (50+ cases)

### Static Analysis

- [ ] **ANLZ-01**: S1 — Entity state space enumeration
- [ ] **ANLZ-02**: S2 — Rule stratum reachability analysis
- [ ] **ANLZ-03**: S3a — Domain coverage analysis
- [ ] **ANLZ-04**: S4 — Authority topology mapping
- [ ] **ANLZ-05**: S5 — Operation effect analysis
- [ ] **ANLZ-06**: S6 — Flow path enumeration
- [ ] **ANLZ-07**: S7 — Complexity bounds computation
- [ ] **ANLZ-08**: Static analyzer reports structured output suitable for CLI and LSP consumption

### Domain Validation

- [ ] **DOMN-01**: Multi-tenant SaaS contract (seat limits, feature flags, subscription state)
- [ ] **DOMN-02**: Healthcare prior auth contract (policy rules, peer review, appeals)
- [ ] **DOMN-03**: Supply chain contract (customs, inspection, release gates)
- [ ] **DOMN-04**: Internal procurement contract (approval tiers, delegation, budget)
- [ ] **DOMN-05**: Financial domain contract (lending, escrow, or compliance)
- [ ] **DOMN-06**: Each contract elaborates without error
- [ ] **DOMN-07**: Each contract passes `tenor check`
- [ ] **DOMN-08**: Each contract evaluates against sample facts via `tenor eval` with correct provenance
- [ ] **DOMN-09**: Spec gap report produced from domain validation findings (informs P5 scope)

### Code Generation

- [ ] **CGEN-01**: TypeScript code generator using ports-and-adapters pattern
- [ ] **CGEN-02**: Generated code includes: entity store, rule engine, operation handlers, flow orchestrator, provenance collector
- [ ] **CGEN-03**: Generated code exposes port interfaces for developer-supplied adapters (fact sources, persona resolver, state store, provenance repo)
- [ ] **CGEN-04**: `@tenor/adapters-local` package with in-memory adapter implementations for dev/test
- [ ] **CGEN-05**: Generated TypeScript uses fixed-point decimal (not native `number`) for Money/Decimal types
- [ ] **CGEN-06**: Rust code generator as second target
- [ ] **CGEN-07**: Generated code passes evaluation conformance suite (same verdicts as reference evaluator)

### Developer Experience

- [ ] **DEVX-01**: VS Code extension with TextMate grammar for syntax highlighting
- [ ] **DEVX-02**: VS Code extension with inline elaboration error display
- [ ] **DEVX-03**: VS Code extension runs `tenor check` on save
- [ ] **DEVX-04**: VS Code extension supports go-to-definition for construct references
- [ ] **DEVX-05**: Language reference documentation (author-facing, distinct from implementer spec)
- [ ] **DEVX-06**: Authoring guide with worked examples across multiple domains
- [ ] **DEVX-07**: Executor implementation guide
- [ ] **DEVX-08**: Code generation guide

## v2 Requirements

Deferred to post-1.0. Tracked but not in current roadmap.

### Extended Analysis

- **ANLZ-09**: S3b — Domain satisfiability (bounded model checking, needs benchmarking)
- **ANLZ-10**: S6b — Flow path analysis with probabilistic complexity estimation

### Extended Code Generation

- **CGEN-08**: Additional code generation targets beyond TypeScript + Rust
- **CGEN-09**: Code generation template customization API

### Extended Type System

- **SPEC-06**: P5 module federation (inter-org type sharing)
- **SPEC-07**: Generic type parameters for Records

### Extended DX

- **DEVX-09**: IntelliJ/Neovim language server integration
- **DEVX-10**: `tenor fmt` formatter (needs authoring patterns established first)
- **DEVX-11**: `tenor bench` benchmarking for contract evaluation performance

## Out of Scope

| Feature | Reason |
|---------|--------|
| REPL | Contradicts Tenor's batch evaluation model; contracts are evaluated against complete fact sets, not interactively |
| Runtime monitoring / enforcement | Separate operational concern; 1.0 focuses on authoring and static tooling |
| GUI contract editor | Premature without established authoring patterns from CLI experience |
| Formal proof of soundness | Separate research track; not blocking 1.0 practical use |
| Aggregates in DSL | Explicitly prohibited by spec design (decidability constraint) |
| Turing completeness | Violates core language invariant (termination guarantee) |
| Async runtime in toolchain | Pure file-to-file transformation; async adds complexity for zero benefit |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| SPEC-01 | — | Pending |
| SPEC-02 | — | Pending |
| SPEC-03 | — | Pending |
| SPEC-04 | — | Pending |
| SPEC-05 | — | Pending |
| FNDN-01 | — | Pending |
| FNDN-02 | — | Pending |
| FNDN-03 | — | Pending |
| FNDN-04 | — | Pending |
| CLI-01 | — | Pending |
| CLI-02 | — | Pending |
| CLI-03 | — | Pending |
| CLI-04 | — | Pending |
| CLI-05 | — | Pending |
| CLI-06 | — | Pending |
| CLI-07 | — | Pending |
| CLI-08 | — | Pending |
| CLI-09 | — | Pending |
| EVAL-01 | — | Pending |
| EVAL-02 | — | Pending |
| EVAL-03 | — | Pending |
| EVAL-04 | — | Pending |
| EVAL-05 | — | Pending |
| EVAL-06 | — | Pending |
| EVAL-07 | — | Pending |
| ANLZ-01 | — | Pending |
| ANLZ-02 | — | Pending |
| ANLZ-03 | — | Pending |
| ANLZ-04 | — | Pending |
| ANLZ-05 | — | Pending |
| ANLZ-06 | — | Pending |
| ANLZ-07 | — | Pending |
| ANLZ-08 | — | Pending |
| DOMN-01 | — | Pending |
| DOMN-02 | — | Pending |
| DOMN-03 | — | Pending |
| DOMN-04 | — | Pending |
| DOMN-05 | — | Pending |
| DOMN-06 | — | Pending |
| DOMN-07 | — | Pending |
| DOMN-08 | — | Pending |
| DOMN-09 | — | Pending |
| CGEN-01 | — | Pending |
| CGEN-02 | — | Pending |
| CGEN-03 | — | Pending |
| CGEN-04 | — | Pending |
| CGEN-05 | — | Pending |
| CGEN-06 | — | Pending |
| CGEN-07 | — | Pending |
| DEVX-01 | — | Pending |
| DEVX-02 | — | Pending |
| DEVX-03 | — | Pending |
| DEVX-04 | — | Pending |
| DEVX-05 | — | Pending |
| DEVX-06 | — | Pending |
| DEVX-07 | — | Pending |
| DEVX-08 | — | Pending |

**Coverage:**
- v1 requirements: 53 total
- Mapped to phases: 0
- Unmapped: 53 (pending roadmap creation)

---
*Requirements defined: 2026-02-21*
*Last updated: 2026-02-21 after initial definition*
