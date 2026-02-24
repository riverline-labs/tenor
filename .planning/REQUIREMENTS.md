# Requirements: Tenor — Platform & Ecosystem

**Milestone:** Platform & Ecosystem
**Defined:** 2026-02-22
**Core Value:** A contract authored in TenorDSL must be statically verifiable, evaluable against facts, and usable by agents and developers — the full lifecycle from specification to execution with provenance at every step.

## Agent Skill Examples

- [x] **SKEX-01**: `tenor agent` CLI tool turns any contract into an interactive shell
- [x] **SKEX-02**: Express middleware reference implementation generates routes from operations
- [x] **SKEX-03**: Slack bot reference implementation for contract interaction via chat
- [x] **SKEX-04**: Audit agent reference implementation generates compliance reports from provenance chains

## Execution Kernel

- [ ] **EXEC-01**: Atomic flow execution — begin Postgres transaction, read entity states with version numbers, call evaluator, version-validate, commit state transitions + provenance in one transaction, rollback on mismatch
- [ ] **EXEC-02**: Optimistic concurrency control — concurrent executions against the same entity produce one success and one typed `ConcurrentConflict` error, never corrupt state, never silent retry
- [ ] **EXEC-03**: Provenance coupling — every successful state transition has a provenance record atomically committed in the same transaction (C7 compliance); if provenance insert fails, the commit rolls back
- [ ] **EXEC-04**: Entity state management — initialize entities, track state + version, validate transition sources match current state before applying effects (E2), enforce persona authorization (E3 atomicity)
- [ ] **EXEC-05**: Integration tests — conflict detection, rollback on version mismatch, persona rejection, precondition failure, transition source mismatch, provenance atomicity

## Embedded Evaluator

- [ ] **WASM-01**: Rust evaluator compiles to WASM
- [ ] **WASM-02**: WASM evaluator runs in browser environments
- [ ] **WASM-03**: WASM evaluator runs in Node.js
- [ ] **WASM-04**: WASM evaluator runs in edge environments (Cloudflare Workers, Deno Deploy)
- [ ] **WASM-05**: Embedded evaluator produces identical results to native Rust evaluator

## Hosted Evaluator Service

- [ ] **HOST-01**: Managed API endpoint for contract evaluation
- [ ] **HOST-02**: Authentication and rate limiting
- [ ] **HOST-03**: Contract upload and management

## Deferred

| Feature | Reason |
|---------|--------|
| AI Authoring Assistant (AUTH-01–03) | Deferred — documentation, not core infrastructure |
| Domain Contract Library (LIB-01–03) | Deferred — needs hosted evaluator first |
| Rust and Go Agent SDKs (RSDK-01–03) | Deferred — extract abstraction when second backend needed |
| Multi-party Contract Execution (MPTY-01–03) | Deferred — capstone, depends on executor + hosted service |
| P5 module federation (inter-org type sharing) | Complexity explosion, defer to post-1.0 |
| Runtime monitoring / contract enforcement | Separate operational concern |
| GUI contract editor | Premature; need CLI and authoring experience first |
| UI annotation layer on Tenor contracts | Codegen produces behavioral skeleton, not full UI |
| Formal proof of soundness | Separate research track |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| SKEX-01 through SKEX-04 | Phase 1 | Complete |
| EXEC-01 through EXEC-05 | Phase 2 | Not started |
| WASM-01 through WASM-05 | Phase 3 | Not started |
| HOST-01 through HOST-03 | Phase 4 | Not started |

**Coverage:** 17 requirements, 4 complete

---
*Last updated: 2026-02-24*
