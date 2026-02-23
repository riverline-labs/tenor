# Requirements: Tenor — Agent Tooling

**Milestone:** Agent Tooling
**Defined:** 2026-02-22
**Core Value:** A contract authored in TenorDSL must be statically verifiable, evaluable against facts, and usable by agents and developers — the full lifecycle from specification to execution with provenance at every step.

## TypeScript Agent SDK

- [x] **SDK-01**: TypeScript SDK connects to Rust evaluator running as a service
- [x] **SDK-02**: SDK exposes core agent skills: getOperations, invoke, explain
- [x] **SDK-03**: Evaluator available via `tenor serve` CLI command (local process)
- [x] **SDK-04**: Evaluator available via Docker image (`tenor/evaluator`)
- [x] **SDK-05**: SDK documentation is explicit: the SDK is a client, the evaluator is the trusted core

## TypeScript Code Generation

- [x] **CGEN-01**: Generate typed interfaces from interchange bundles
- [x] **CGEN-02**: Generate typed client bindings from interchange bundles
- [x] **CGEN-03**: Generated code provides IDE autocompletion and compile-time type checking

## VS Code Extension

- [x] **DEVX-01**: VS Code syntax highlighting for .tenor files
- [x] **DEVX-02**: Inline error diagnostics via LSP
- [x] **DEVX-03**: Check-on-save
- [x] **DEVX-04**: Preview Agent Capabilities panel — shows what an agent would see when reading the contract

## Agent Skill Examples

- [ ] **SKEX-01**: `tenor agent` CLI tool turns any contract into an interactive shell
- [ ] **SKEX-02**: Express middleware reference implementation generates routes from operations
- [ ] **SKEX-03**: Slack bot reference implementation for contract interaction via chat
- [ ] **SKEX-04**: Audit agent reference implementation generates compliance reports from provenance chains

## Embedded Evaluator

- [ ] **WASM-01**: Rust evaluator compiles to WASM
- [ ] **WASM-02**: WASM evaluator runs in browser environments
- [ ] **WASM-03**: WASM evaluator runs in Node.js
- [ ] **WASM-04**: WASM evaluator runs in edge environments (Cloudflare Workers, Deno Deploy)
- [ ] **WASM-05**: Embedded evaluator produces identical results to native Rust evaluator

## Deferred

| Feature | Reason |
|---------|--------|
| Rust code generation target | TypeScript is sufficient for v1 tooling |
| Go code generation target | TypeScript is sufficient for v1 tooling |
| P5 module federation (inter-org type sharing) | Complexity explosion, defer to post-1.0 |
| Runtime monitoring / contract enforcement | Separate operational concern |
| GUI contract editor | Premature; need CLI and authoring experience first |
| UI annotation layer on Tenor contracts | Codegen produces behavioral skeleton, not full UI |
| Formal proof of soundness | Separate research track |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| SDK-01 through SDK-05 | Phase 15 | Not started |
| CGEN-01 through CGEN-03 | Phase 16 | Not started |
| DEVX-01 through DEVX-04 | Phase 17 | Not started |
| SKEX-01 through SKEX-04 | Phase 18 | Not started |
| WASM-01 through WASM-05 | Phase 19 | Not started |

**Coverage:** 22 requirements, 0 complete

---
*Requirements defined: 2026-02-22*
*Last updated: 2026-02-22 after v1.0 milestone archival*
