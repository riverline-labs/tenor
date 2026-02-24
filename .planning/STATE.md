# Project State

## Current Position

Phase: 2 (Execution Kernel)
Status: Not started
Last activity: 2026-02-24 — Two-repo split: tenor (public) + tenor-platform (private)
Repository: riverline-labs/tenor-platform

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-23)

**Core value:** A contract authored in TenorDSL must be statically verifiable, evaluable against facts, and usable by agents and developers — the full lifecycle from specification to execution with provenance at every step.
**Current focus:** Platform & Ecosystem — phases 1-4
**Repo split:** riverline-labs/tenor (public) + riverline-labs/tenor-platform (private)

## Completed

All prior milestones shipped (v0.9 Core, v1.0 System Construct, Agent Tooling, Platform Hardening).
Phase 1: Agent Skill Examples (SKEX-01 through SKEX-04) — 4 reference implementations shipped.

## Pending Todos

None.

## Blockers/Concerns

None.

## Accumulated Context

- Spec frozen at v1.0 including System construct
- Trust boundary: Rust evaluator is trusted core, SDKs are clients
- 73 conformance tests, ~508 Rust tests
- 5 domain contracts (6,441 LOC) validated
- TypeScript SDK ships `tenor serve` + `@tenor-lang/sdk`
- VS Code extension with LSP and agent capabilities panel
- Codegen produces TypeScript behavioral skeleton from contracts
- Dead code annotations cleaned from LSP semantic tokens and navigation modules
- spec_sections field removed from ambiguity module (not wired through)
- All version strings verified to use TENOR_VERSION/TENOR_BUNDLE_VERSION constants
- FlowPathConfig provides configurable S6 analysis limits
- SourceProvider trait abstracts file I/O for WASM-ready elaboration (source.rs)
- Parser multi-error recovery available via parse_recovering() (opt-in)
- InMemoryProvider enables filesystem-free elaboration for WASM and testing
- All expect()/unwrap() removed from pass3/4/5 (ElabError propagation)
- Import cycle detection uses O(1) HashSet (parallel to Vec for error reporting)
- pass6_serialize uses static key constants and ins() helper for reduced allocations
- tenor-interchange crate provides single-source typed deserialization for interchange JSON bundles
- eval, analyze, codegen all delegate to tenor_interchange::from_interchange() for initial parsing
- Contract type has HashMap indexes for O(1) lookups; use Contract::new() and get_* methods
- Stratum evaluation uses BTreeMap index (O(n) vs previous O(k*n))
- Flow failure handling uses std::mem::take() instead of deep clones
- HTTP server uses axum + tokio (replaced tiny_http + libc); optional TLS via axum-server + rustls
- libc removed from CLI crate -- tenor-core and tenor-eval have zero libc in production builds
- explain.rs uses typed tenor-interchange structs (from_interchange + InterchangeConstruct variants) instead of raw JSON traversal
- HTTP server hardened: input validation (size, null bytes, import escape, filename), CORS via tower-http, per-IP rate limiting (60 req/min), optional API key auth via TENOR_API_KEY env var
- `tenor agent <file>` interactive REPL with fact management, evaluation, flow execution, and explain integration
- Express middleware generates REST routes from contract operations with SDK error-to-HTTP-status mapping
- Slack bot uses Bolt Socket Mode for slash commands (/tenor-list, /tenor-eval with modal, /tenor-explain)
- Audit agent walks verdict provenance chains, builds fact coverage matrices, detects compliance gaps (orphan_fact, shallow_provenance, single_rule_dependency)
- LSP unit tests cover navigation (goto-definition, find-references, document-symbols), completion, and hover via direct function calls
- diff CLI has 6 e2e tests (identical bundles, additions, breaking changes, error handling)
- Flow error-path conformance fixture exercises FailureHandler::Escalate with 4-step escalation trace
- S3a admissibility negative tests verify dead states have no admissible operations
- SystemContract coordinator design documented (docs/system-contract-coordinator.md) targeting Phase 7
