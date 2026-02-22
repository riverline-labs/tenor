# Requirements: Tenor v1.0

**Defined:** 2026-02-22
**Core Value:** A contract authored in TenorDSL must be statically verifiable, evaluable against facts, and generatable into working code — the full lifecycle from specification to execution with provenance at every step.

## v1.0 Requirements

Requirements for v1.0 release. Each maps to roadmap phases.

### System Construct

- [ ] **SYS-01**: System construct declares member contracts that compose together
- [ ] **SYS-02**: Shared persona identity is expressible — the same actor across contracts is formally declared (customs_officer in contract A is the same actor as in contract B)
- [ ] **SYS-03**: Cross-contract flow triggers are expressible — completion of a flow in one contract can initiate a flow in another contract
- [ ] **SYS-04**: Cross-contract entity relationships are expressible — an entity in one contract can be declared as the same entity in another contract
- [ ] **SYS-05**: System construct has formal syntax, semantics, and interchange representation in docs/TENOR.md (CFFP-derived)
- [ ] **SYS-06**: Elaborator validates System constructs (Pass 5) and serializes to interchange JSON (Pass 6)
- [ ] **SYS-07**: Interchange JSON Schema extended to validate System construct documents
- [ ] **SYS-08**: Conformance suite covers System construct elaboration (positive and negative tests)

### Static Analysis Extensions

- [ ] **ANLZ-09**: S4 authority topology extended for cross-contract persona analysis within a System
- [ ] **ANLZ-10**: S6 flow path enumeration extended for cross-contract flow trigger analysis within a System
- [ ] **ANLZ-11**: `tenor check` reports cross-contract analysis findings for System constructs

### Executor Obligations

- [ ] **EXEC-01**: Executor obligations defined for cross-contract snapshot coordination within a System
- [ ] **EXEC-02**: Executor obligations defined for cross-contract persona resolution within a System

### Spec Quality

- [ ] **SPEC-09**: AAP (Assumption Audit Protocol) run on the complete v1.0 spec — all hidden assumptions surfaced and fragility characterized
- [ ] **SPEC-10**: AAP findings resolved or documented as acknowledged limitations before v1.0 freeze

### Domain Re-validation

- [ ] **DOMN-10**: SaaS subscription contract re-implemented for v1.0 spec (including System construct where applicable)
- [ ] **DOMN-11**: Healthcare prior auth contract re-implemented for v1.0 spec
- [ ] **DOMN-12**: Supply chain inspection contract re-implemented for v1.0 spec
- [ ] **DOMN-13**: Energy procurement contract re-implemented for v1.0 spec
- [ ] **DOMN-14**: Trade finance contract re-implemented for v1.0 spec
- [ ] **DOMN-15**: At least one multi-contract System scenario validated end-to-end across domain contracts

### Documentation

- [ ] **DEVX-05**: Language reference documents every construct including System, with author-facing examples
- [ ] **DEVX-06**: Authoring guide walks through complete worked examples across multiple domains
- [ ] **DEVX-07**: Executor implementation guide explains how to build a runtime that correctly evaluates Tenor contracts including System composition

## v2 Requirements

Deferred to future release.

### Code Generation

- **CGEN-01**: TypeScript code generation from interchange bundles (ports-and-adapters pattern)
- **CGEN-06**: Rust code generation target with conformance parity
- **CGEN-08**: Go code generation target with conformance parity

### IDE Tooling

- **DEVX-01**: VS Code syntax highlighting for .tenor files
- **DEVX-02**: Inline error diagnostics in VS Code
- **DEVX-03**: Check-on-save in VS Code
- **DEVX-04**: Go-to-definition for construct references

## Out of Scope

| Feature | Reason |
|---------|--------|
| P5 module federation (inter-org type sharing) | Complexity explosion, defer to post-1.0 |
| Runtime monitoring / contract enforcement | Separate operational concern |
| GUI contract editor | Premature; need CLI and authoring experience first |
| UI annotation layer on Tenor contracts | Codegen produces behavioral skeleton, not full UI |
| Code generation targets | Deferred to Milestone 3 (depends on v1.0 interchange) |
| Formal proof of soundness | Separate research track |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| SYS-01 | Phase 12 | Pending |
| SYS-02 | Phase 12 | Pending |
| SYS-03 | Phase 12 | Pending |
| SYS-04 | Phase 12 | Pending |
| SYS-05 | Phase 12 | Pending |
| SYS-06 | Phase 12 | Pending |
| SYS-07 | Phase 12 | Pending |
| SYS-08 | Phase 12 | Pending |
| ANLZ-09 | Phase 12 | Pending |
| ANLZ-10 | Phase 12 | Pending |
| ANLZ-11 | Phase 12 | Pending |
| EXEC-01 | Phase 12 | Pending |
| EXEC-02 | Phase 12 | Pending |
| SPEC-09 | Phase 12.1 | Pending |
| SPEC-10 | Phase 12.1 | Pending |
| DOMN-10 | Phase 13 | Pending |
| DOMN-11 | Phase 13 | Pending |
| DOMN-12 | Phase 13 | Pending |
| DOMN-13 | Phase 13 | Pending |
| DOMN-14 | Phase 13 | Pending |
| DOMN-15 | Phase 13 | Pending |
| DEVX-05 | Phase 14 | Pending |
| DEVX-06 | Phase 14 | Pending |
| DEVX-07 | Phase 14 | Pending |

**Coverage:**
- v1.0 requirements: 24 total
- Mapped to phases: 24
- Unmapped: 0

---
*Requirements defined: 2026-02-22*
*Last updated: 2026-02-22 after roadmap creation*
