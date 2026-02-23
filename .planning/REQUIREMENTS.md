# Requirements: Tenor — Platform & Ecosystem

**Milestone:** Platform & Ecosystem
**Defined:** 2026-02-22
**Core Value:** A contract authored in TenorDSL must be statically verifiable, evaluable against facts, and usable by agents and developers — the full lifecycle from specification to execution with provenance at every step.

## Platform Hardening

- [x] **HARD-01**: Shared interchange deserialization library replaces triplicated parsing in eval, analyze, and codegen
- [ ] **HARD-02**: explain.rs uses typed interchange structs instead of untyped JSON traversal with silent fallbacks
- [x] **HARD-03**: All expect() calls in pass5_validate.rs and pass3_types.rs replaced with proper error propagation
- [x] **HARD-04**: Parser recovers from first error and reports multiple diagnostics per parse
- [x] **HARD-05**: tenor-core file I/O factored behind a trait so elaborate() works without filesystem (WASM prerequisite)
- [ ] **HARD-06**: libc dependency in serve.rs isolated from tenor-core and tenor-eval crate graph
- [x] **HARD-07**: spec_sections dead code in ambiguity testing removed or wired through
- [ ] **HARD-08**: HTTP stack replaced with production-grade framework (axum or actix-web) with TLS and concurrent request handling
- [ ] **HARD-09**: Unsafe signal handling in serve.rs replaced with ctrlc or signal-hook
- [ ] **HARD-10**: Elaborate endpoint validates user content before writing to temp files
- [ ] **HARD-11**: Auth, CORS, and rate limiting added to all HTTP endpoints
- [ ] **HARD-12**: SystemContract coordinator designed for cross-contract triggers and shared entity state
- [ ] **HARD-13**: Contract type uses HashMap indexes instead of Vec for O(1) lookups
- [ ] **HARD-14**: LSP crate has unit tests covering navigation and completion correctness
- [x] **HARD-15**: Duplicate manifest/etag logic in runner.rs consolidated to import from manifest.rs
- [x] **HARD-16**: Dead code annotations in LSP crate (semantic_tokens.rs, navigation.rs) resolved
- [ ] **HARD-17**: Stratum rule evaluation uses indexed lookup instead of O(k*n) scan
- [ ] **HARD-18**: Flow execution eliminates unnecessary deep clones (handle_failure vector cloning)
- [x] **HARD-19**: S6 path enumeration MAX_PATHS and MAX_DEPTH made configurable
- [ ] **HARD-20**: Flow error-path conformance fixtures added (replacing inline unit tests with panic! asserts)
- [x] **HARD-21**: All hardcoded version strings reference TENOR_BUNDLE_VERSION constant
- [x] **HARD-22**: Import cycle detection uses HashSet instead of linear Vec::contains
- [x] **HARD-23**: Excessive string allocations in pass6_serialize.rs reduced (matters for WASM embedding)
- [ ] **HARD-24**: tenor diff CLI integration tested end-to-end
- [ ] **HARD-25**: explain.rs Markdown format output tested
- [ ] **HARD-26**: S3a admissibility has negative test cases
- [x] **HARD-27**: Interchange schema formalized as shared library (prerequisite for Rust and Go SDKs in Phase 24)

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
| HARD-01 through HARD-27 | Phase 18 | Not started |
| SKEX-01 through SKEX-04 | Phase 19 | Not started |
| AUTH-01 through AUTH-03 | Phase 20 | Not started |
| WASM-01 through WASM-05 | Phase 21 | Not started |
| HOST-01 through HOST-03 | Phase 22 | Not started |
| LIB-01 through LIB-03 | Phase 23 | Not started |
| RSDK-01 through RSDK-03 | Phase 24 | Not started |
| MPTY-01 through MPTY-03 | Phase 25 | Not started |

**Coverage:** 51 requirements, 0 complete

---
*Requirements defined: 2026-02-22*
*Last updated: 2026-02-23 after Platform Hardening phase added*
