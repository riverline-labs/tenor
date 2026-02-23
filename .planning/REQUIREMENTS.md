# Requirements: Tenor — Platform & Ecosystem

**Milestone:** Platform & Ecosystem
**Defined:** 2026-02-22
**Core Value:** A contract authored in TenorDSL must be statically verifiable, evaluable against facts, and usable by agents and developers — the full lifecycle from specification to execution with provenance at every step.

## Agent Skill Examples

- [ ] **SKEX-01**: `tenor agent` CLI tool turns any contract into an interactive shell
- [ ] **SKEX-02**: Express middleware reference implementation generates routes from operations
- [ ] **SKEX-03**: Slack bot reference implementation for contract interaction via chat
- [ ] **SKEX-04**: Audit agent reference implementation generates compliance reports from provenance chains

## AI Authoring Assistant

- [ ] **AUTH-01**: Prompt templates give AI assistants sufficient context to author Tenor contracts
- [ ] **AUTH-02**: Example conversations demonstrate autonomous, collaborative, and reviewed workflows
- [ ] **AUTH-03**: Guidance on when to ask questions versus make decisions

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

## Domain Contract Library

- [ ] **LIB-01**: Curated contracts for common industries
- [ ] **LIB-02**: Community contribution framework
- [ ] **LIB-03**: Five existing domain contracts published as seed library

## Rust and Go Agent SDKs

- [ ] **RSDK-01**: Rust SDK with same agent skills as TypeScript SDK
- [ ] **RSDK-02**: Go SDK with same agent skills as TypeScript SDK
- [ ] **RSDK-03**: Same trust model — SDKs are clients, evaluator is trusted core

## Multi-party Contract Execution

- [ ] **MPTY-01**: Two or more parties execute against one contract
- [ ] **MPTY-02**: Independent verification — no single party controls the truth
- [ ] **MPTY-03**: Full trust model realized in production

## Deferred

| Feature | Reason |
|---------|--------|
| P5 module federation (inter-org type sharing) | Complexity explosion, defer to post-1.0 |
| Runtime monitoring / contract enforcement | Separate operational concern |
| GUI contract editor | Premature; need CLI and authoring experience first |
| UI annotation layer on Tenor contracts | Codegen produces behavioral skeleton, not full UI |
| Formal proof of soundness | Separate research track |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| SKEX-01 through SKEX-04 | Phase 1 | Not started |
| AUTH-01 through AUTH-03 | Phase 2 | Not started |
| WASM-01 through WASM-05 | Phase 3 | Not started |
| HOST-01 through HOST-03 | Phase 4 | Not started |
| LIB-01 through LIB-03 | Phase 5 | Not started |
| RSDK-01 through RSDK-03 | Phase 6 | Not started |
| MPTY-01 through MPTY-03 | Phase 7 | Not started |

**Coverage:** 24 requirements, 0 complete

---
*Last updated: 2026-02-23*
