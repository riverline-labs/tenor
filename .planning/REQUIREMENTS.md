# Requirements: Tenor — Platform & Ecosystem

**Milestone:** Platform & Ecosystem
**Defined:** 2026-02-22
**Core Value:** A contract authored in TenorDSL must be statically verifiable, evaluable against facts, and usable by agents and developers — the full lifecycle from specification to execution with provenance at every step.

## Agent Skill Examples

- [x] **SKEX-01**: `tenor agent` CLI tool turns any contract into an interactive shell
- [x] **SKEX-02**: Express middleware reference implementation generates routes from operations
- [x] **SKEX-03**: Slack bot reference implementation for contract interaction via chat
- [x] **SKEX-04**: Audit agent reference implementation generates compliance reports from provenance chains

## Embedded Evaluator

- [ ] **WASM-01**: Rust evaluator compiles to WASM
- [ ] **WASM-02**: WASM evaluator runs in browser environments
- [ ] **WASM-03**: WASM evaluator runs in Node.js
- [ ] **WASM-04**: WASM evaluator runs in edge environments (Cloudflare Workers, Deno Deploy)
- [ ] **WASM-05**: Embedded evaluator produces identical results to native Rust evaluator

## Deferred

| Feature | Reason |
|---------|--------|
| AI Authoring Assistant (AUTH-01–03) | Documentation, not core infrastructure |
| Domain Contract Library (LIB-01–03) | Needs hosted evaluator first |
| Rust and Go Agent SDKs (RSDK-01–03) | Extract abstraction when second backend needed |
| Multi-party Contract Execution (MPTY-01–03) | Capstone, depends on executor + hosted service |
| P5 module federation (inter-org type sharing) | Complexity explosion, defer to post-1.0 |
| Runtime monitoring / contract enforcement | Separate operational concern |
| GUI contract editor | Premature; need CLI and authoring experience first |
| UI annotation layer on Tenor contracts | Codegen produces behavioral skeleton, not full UI |
| Formal proof of soundness | Separate research track |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| SKEX-01 through SKEX-04 | Phase 1 | Complete |
| WASM-01 through WASM-05 | Phase 2 | Not started |

**Coverage:** 9 requirements, 4 complete

---
*Last updated: 2026-02-24*
