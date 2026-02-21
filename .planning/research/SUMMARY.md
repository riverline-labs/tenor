# Research Summary

**Project:** Tenor v1.0
**Synthesized:** 2026-02-21
**Sources:** STACK.md, FEATURES.md, ARCHITECTURE.md, PITFALLS.md

---

## Executive Summary

Tenor is a behavioral contract DSL with a mature elaborator (47/47 conformance) but no surrounding toolchain. The path to 1.0 requires: spec completion (persona, P7 outcome typing, interchange versioning), monolithic refactoring into a Cargo workspace, then building CLI/evaluator/analyzer/codegen/IDE tooling in strict dependency order. The dominant risk is spec drift under tooling pressure; the second risk is building code generation before domain validation proves spec stability.

---

## Stack Recommendations

| Component | Recommendation | Rationale |
|-----------|---------------|-----------|
| CLI | clap 4.5 (derive API) | De facto Rust standard; maps directly to subcommand list |
| Diagnostics | ariadne 0.6 | Purpose-built for compiler source-span diagnostics |
| Code generation | tera 1.20 | Runtime template loading; codegen templates target other languages and change frequently |
| LSP | lsp-server 0.7 | Synchronous (like rust-analyzer); Tenor's elaborator is sync, no need for async |
| Decimal math | rust_decimal | Evaluator needs fixed-point arithmetic per NumericModel; f64 violates spec |
| Documentation | mdBook | Rust ecosystem standard for language references |
| Async runtime | **None** | Tenor is pure file-to-file transformation; async adds compile time and binary size for zero benefit |

---

## Feature Landscape

**Table Stakes (10):** Unified CLI, elaboration, evaluation with provenance, static analysis (S1-S7), conformance suites, error diagnostics, interchange schema, spec completion (persona/P7), domain validation, documentation.

**Differentiators (8):** Provenance-traced evaluation (every verdict carries full derivation chain), S1-S7 static analysis suite (state space + reachability + authority topology + flow paths), CFFP-driven construct design, code generation with ports-and-adapters pattern, cross-domain validation (SaaS/healthcare/supply chain/procurement/finance).

**Anti-Features (8):** REPL (contradicts batch evaluation model), formatter (premature without usage patterns), aggregates (explicitly prohibited by spec), Turing completeness, runtime monitoring, module federation, GUI editor, additional codegen targets beyond TS+Rust.

---

## Architecture Direction

**Cargo workspace with 6 crates:**

| Crate | Purpose | Dependencies |
|-------|---------|-------------|
| `tenor-core` | Library: parser, 6-pass pipeline, typed AST, Index, TypeEnv | serde, serde_json, ariadne |
| `tenor-cli` | Unified `tenor` binary | tenor-core, clap |
| `tenor-eval` | Evaluator: bundle + facts → verdicts + provenance | tenor-core, rust_decimal |
| `tenor-analyze` | Static analyzer: S1-S7 | tenor-core |
| `tenor-codegen` | Code generator: interchange → TypeScript/Rust | tenor-core, tera |
| `tenor-lsp` | Language server | tenor-core, tenor-analyze, lsp-server |

**Hard prerequisite:** Extract `tenor-core` from monolithic `elaborate.rs` before any new tooling. Every downstream tool needs access to intermediate pass outputs (typed AST, Index, TypeEnv).

**After core extraction:** CLI, evaluator, and analyzer can proceed in parallel. Code generation benefits from evaluator (conformance suite provides ground truth). LSP depends on core + analyzer.

---

## Critical Pitfalls

1. **Spec drift under tooling pressure** — Enforce spec-first discipline: spec changes before implementation, conformance suite as executable spec, interchange schema formally defined. Phase 1 must freeze spec for 1.0 constructs.

2. **Monolithic elaborate.rs** — 2,066 lines / 58 functions containing all 6 passes. Must modularize into typed pass boundaries before adding constructs or building downstream tools. Zero-functionality-change refactor.

3. **Frozen verdict semantics** — Spec defines Flow snapshots as immutable, but natural evaluator implementation re-evaluates rules at each step. Evaluator conformance suite must catch this divergence from day one.

4. **Interchange format versioning** — Currently defined only by serializer output with no schema and no version field. Adding constructs without versioning breaks all downstream consumers. Must establish before tooling begins.

5. **NumericModel conformance across targets** — JavaScript `Number`, Rust integers, and spec decimal semantics diverge at precision boundaries. Dedicated numeric conformance suite (50+ edge cases) required per target.

6. **Premature code generation** — Building codegen before domain validation is a common language toolchain mistake. Phase 3 (5-10 real contracts) is a hard gate.

---

## Suggested Phase Structure

| Phase | Focus | Key Output |
|-------|-------|------------|
| 1 | Spec completion + foundation | CFFP runs for persona/P7/P5, spec frozen, interchange versioned, `tenor-core` extracted |
| 2 | CLI + evaluator + static analysis | `tenor` binary, evaluation with provenance, S1-S7 |
| 3 | Domain validation | 5-10 real contracts across distinct domains, spec gap report |
| 4 | Code generation | TypeScript target with ports-and-adapters, local adapters package |
| 5 | Developer experience | VS Code extension (TextMate + LSP), mdBook documentation |

---

## Open Questions for Roadmapping

- P5 (shared type library) scope depends on domain validation findings — may need to be split across phases
- Evaluator conformance suite design: language-agnostic JSON fixtures vs language-specific tests (probably both)
- S3b domain satisfiability: what domain size thresholds are practical? May defer to post-1.0
- LSP incremental elaboration: assume full re-elab is fine until multi-file contracts prove otherwise
- `rust_decimal` vs `bigdecimal` for evaluator — needs Phase 2 research

---

*Synthesized: 2026-02-21*
