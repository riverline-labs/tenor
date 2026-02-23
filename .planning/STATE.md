# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-23)

**Core value:** A contract authored in TenorDSL must be statically verifiable, evaluable against facts, and usable by agents and developers -- the full lifecycle from specification to execution with provenance at every step.
**Current focus:** Platform & Ecosystem milestone -- Phase 17 complete

## Current Position

Milestone: Platform & Ecosystem
Phase: 17 (VS Code Extension)
Plan: 05 of 5 (complete)
Status: Phase complete
Last activity: 2026-02-23 -- Completed 17-05-PLAN.md (Commands, status bar, snippets, and end-to-end polish)

Progress: [#░░░░░░░░░] 12.5% (1/8 phases in Platform & Ecosystem)

## Performance Metrics

**Velocity (v0.9 + v1.0):**
- Total plans completed: 71 (46 v0.9 + 17 v1.0 + 6 agent tooling + 2 platform)
- Average duration: ~8.0min
- Total execution time: ~5.5 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1. Spec Completion | 5 | 40min | 8.0min |
| 1.1. Spec CI: AI Ambiguity Testing | 2 | 7min | 3.5min |
| 2. Foundation | 4 | 58min | 14.5min |
| 3. CLI + Evaluator | 7 | ~57min | ~8.1min |
| 3.1. CFFP Migration Semantics | 2 | 13min | 6.5min |
| 3.3. Flow Migration Compatibility | 2 | 21min | 10.5min |
| 3.4. Contract Discovery | 2 | 13min | 6.5min |
| 4. Static Analysis | 8 | ~65min | ~8.1min |
| 5. Domain Validation | 8 | ~100min | ~12.5min |
| 5.1. Fix Critical DSL Gaps | 3 | ~15min | ~5min |
| 12. System Construct | 6 | 58min | ~10min |
| 12.1. AAP Spec Audit | 2 | ~10min | ~5min |
| 13. Domain Re-validation | 7 | ~22min | ~3min |
| 14. Documentation | 3 | ~13min | ~4min |
| 14.1. Tech Debt & Hardening | 5 | 32min | ~6.4min |
| 15. TS Agent SDK: Client to Rust Eval | 3 | 30min | ~10min |
| 16. TypeScript Code Generation | 2 | 13min | ~6.5min |
| 17. VS Code Extension | 2 | 14min | 7min |
| Phase 17 P04 | 10min | 2 tasks | 9 files |
| Phase 17 P03 | 13 | 2 tasks | 5 files |
| Phase 17 P05 | 3min | 2 tasks | 5 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Key decisions affecting current work:

- v1.0 spec frozen including System construct -- no breaking changes without CFFP
- SDK-first over codegen-first: client to proven Rust evaluator ships fast without reimplementing trust-critical logic
- Rust/Go codegen deferred: TypeScript alone is sufficient for v1 tooling
- Embedded evaluator is a planned phase (not contingent) -- air-gapped/regulated deployments need it
- Trust boundary preservation: the Rust evaluator is the trusted core, the TypeScript SDK is a client
- Version constants in tenor-core lib.rs: single source for TENOR_VERSION and TENOR_BUNDLE_VERSION
- Manifest version separate from bundle version: "1.1" for manifests lives in manifest.rs
- Unwrap elimination pattern: expect() for algorithmic invariants, ok_or_else with ElabError for potentially-fallible lookups
- Import sandbox: canonicalize + starts_with for all import paths, fail closed on canonicalize failure
- Date/datetime validation via time crate (calendar-correct, not format-only)
- Typed deserialization over untyped traversal: define structs covering exactly what a consumer reads, use serde tagged enum for heterogeneous arrays, let serde ignore extra JSON fields
- HashMap index pattern for hot-loop lookups: build index from Vec before loop, use O(1) get() in loop body
- Config struct with Default impl for configurable limits: FlowPathConfig, max_steps Option<usize>
- tiny_http for HTTP server: synchronous, no async runtime, minimal dependency tree
- Signal handling via libc + AtomicBool: SIGINT/SIGTERM set flag, recv_timeout polling loop checks it
- Zero-dependency SDK: Node 22+ built-in fetch, no axios/node-fetch
- .ts import extensions with rewriteRelativeImportExtensions for dual ESM/CJS build
- Docker image: rust:1.93-slim build + debian:trixie-slim runtime (glibc 2.38 match required)
- Trust boundary documentation: SDK README leads with architecture section explaining client vs trusted core
- [Phase 15]: Docker image: rust:1.93-slim build + debian:trixie-slim runtime for glibc compatibility
- [Phase 16]: Codegen crate reads interchange JSON only (no tenor-core dep) -- same pattern as eval and analyze
- [Phase 16]: CLI `tenor generate <language>` subcommand pattern (not --target flag)
- [Phase 16]: PascalCase entity IDs preserved as-is in generated TypeScript type names
- [Phase 16]: Client class uses composition (not inheritance); single-persona ops hardcode persona, multi-persona use union type
- [Phase 17]: VS Code extension at editors/vscode/ with standard TextMate scopes (no custom theme required)
- [Phase 17]: Synchronous LSP server using lsp-server crate (no async runtime), lsp-types 0.97 Uri-based API
- [Phase 17]: Semantic tokens: 12 types, best-effort from lexer + load_bundle, degrade gracefully on parse errors
- [Phase 17]: Agent capabilities extracted from interchange JSON (same pattern as tenor-analyze/eval), custom LSP request/notification for webview
- [Phase 17]: ProjectIndex rebuilt on every file save for simplicity over incremental updates
- [Phase 17]: Hover uses markdown code blocks with tenor language tag; keyword hover provides DSL reference docs
- [Phase 17]: Status bar driven by vscode.languages.onDidChangeDiagnostics (standard API) not custom LSP notification
- [Phase 17]: Commands use child_process.exec for tenor CLI rather than LSP requests for operations outside LSP session
- [Phase 17]: Three template tiers for New Tenor File: empty, entity+operation, full contract skeleton

### Roadmap Evolution

- v0.9 Core shipped (14 phases, 46 plans)
- v1.0 System Construct + Documentation shipped (4 phases, 17 plans) -- archived
- Agent Tooling milestone shipped: Phases 14.1-16 (10 plans)
- Platform & Ecosystem milestone: Phases 17-24 (requirements and roadmap preserved from prior definition)
- Phase 14.1 inserted after Phase 14: Tech Debt, Bugs & Hardening (URGENT) — resolve all critical CONCERNS.md items before SDK work

### Pending Todos

None yet.

### Blockers/Concerns

None yet.

## Session Continuity

Last session: 2026-02-23
Stopped at: Completed 17-05-PLAN.md (Phase 17 complete)
Resume file: None
