# Tenor

## What This Is

Tenor is a domain-specific language for expressing operational contracts — the rules, entities, operations, and flows that govern how decisions are made in complex business domains. The elaborator transforms `.tenor` source files into a canonical JSON interchange format through a six-pass pipeline. The evaluator executes contracts against fact sets with full provenance. Static analysis (S1-S8) verifies structural properties. The language specification is frozen at v1.0 with the System construct for multi-contract composition.

## Current Milestone: Platform & Ecosystem

**Goal:** Take Tenor from a working language with agent tooling to a production platform — reference implementations, embedded evaluator, hosted service, domain library, multi-language SDKs, and multi-party execution.

**Target features:**
- Agent Skill Examples (CLI tool, Express middleware, Slack bot, audit agent)
- AI Authoring Assistant (prompt templates, example workflows)
- Embedded Evaluator (WASM for browser/Node/edge)
- Hosted Evaluator Service (managed API endpoint)
- Domain Contract Library (curated contracts, community framework)
- Rust and Go Agent SDKs
- Multi-party Contract Execution

**Prior milestones shipped:**
- v0.9 Core — spec, elaborator, evaluator, static analysis, domain validation
- v1.0 System Construct — multi-contract composition, AAP audit, documentation
- Agent Tooling — tech debt hardening, TypeScript SDK, codegen, VS Code extension

## Core Value

A contract authored in TenorDSL must be statically verifiable, evaluable against facts, and usable by agents and developers — the full lifecycle from specification to execution with provenance at every step.

## Requirements

### Validated

- ✓ Six-pass elaborator (lex → parse → bundle → index → type-check → validate → serialize) — v0.3, extended v0.9
- ✓ DSL constructs: Fact, Entity, Rule, Operation, Flow, TypeDecl — v0.3
- ✓ Persona as first-class declared construct — v0.9
- ✓ Operation outcome typing (named outcome types, statically enumerable) — v0.9
- ✓ Shared type library (cross-contract type reuse for Record and TaggedUnion) — v0.9
- ✓ Import resolution with cycle detection — v0.3
- ✓ TypeDecl with named type resolution and cycle detection — v0.3
- ✓ Numeric type system (Int, Decimal, Money) with promotion rules — v0.3
- ✓ Entity state machine validation (DAG acyclicity, transition validity) — v0.3
- ✓ Rule stratum ordering and verdict references — v0.3
- ✓ Operation transition and effect validation — v0.3
- ✓ Flow step graph validation — v0.3
- ✓ Canonical JSON interchange with sorted keys and structured numeric values — v0.3
- ✓ Conformance suite (72 tests: positive, negative, numeric, promotion, shorthand, cross-file, parallel, manifest, exists, effect-to-outcome) — v0.9
- ✓ Contract manifest: `tenor elaborate --manifest` with SHA-256 etag, manifest-aware validate — v0.9
- ✓ Structured error reporting with file/line provenance — v0.3
- ✓ Unified `tenor` CLI binary with subcommands (elaborate, validate, check, eval, explain, test, diff) — v0.9
- ✓ Evaluator: interchange bundle + facts JSON → verdict set + provenance — v0.9
- ✓ Evaluator conformance suite — v0.9
- ✓ Static analyzer implementing S1-S8 from spec — v0.9
- ✓ Domain validation: 5 real contracts across distinct domains — v0.9
- ✓ Breaking change classification via `tenor diff --breaking` — v0.9
- ✓ Migration semantics (CFFP-derived breaking change taxonomy, versioning spec section) — v0.9
- ✓ Flow migration compatibility (CFFP-derived, three-layer analysis) — v0.9
- ✓ Contract discovery (§18, manifest schema, executor obligations E10-E13) — v0.9
- ✓ Effect-to-outcome mapping syntax for multi-outcome operations — v0.9
- ✓ Exists quantifier (∃) across full pipeline — v0.9
- ✓ AI ambiguity testing harness — v0.9
- ✓ `tenor eval --flow --persona` for flow evaluation — v0.9
- ✓ CFFP for construct design — v0.9
- ✓ System construct for multi-contract composition (shared persona identity, cross-contract flows, cross-contract entities) — v1.0
- ✓ AAP spec audit (all findings resolved or documented as acknowledged limitations) — v1.0
- ✓ Language reference documentation — v1.0
- ✓ Authoring guide with worked domain examples — v1.0
- ✓ One-page explainer for decision makers — v1.0
- ✓ All 5 domain contracts re-validated for v1.0 spec — v1.0
- ✓ Multi-contract System scenario validated end-to-end — v1.0

### Active

See [REQUIREMENTS.md](REQUIREMENTS.md) — 24 requirements across phases 18-24.

### Out of Scope

- P5 module federation (inter-org type sharing) — complexity explosion, defer to post-1.0
- Runtime monitoring / contract enforcement in production — separate operational concern
- GUI contract editor — premature; need CLI and authoring experience first
- Rust/Go code generation targets — deferred; TypeScript is sufficient for v1 tooling
- Formal proof of soundness — separate research track, not blocking agent tooling
- UI annotation layer on Tenor contracts — codegen produces behavioral skeleton, not full UI (display order, field labels, help text, visual hierarchy are out of scope)

## Context

- Codebase: ~25,000 LOC Rust across 6 crates (core, cli, eval, analyze, codegen, lsp)
- Spec: docs/TENOR.md, frozen at v1.0
- Conformance: 73 tests (positive, negative, numeric, promotion, shorthand, cross-file, parallel, manifest, exists, effect-to-outcome, import_escape)
- Domain contracts: 5 domains (SaaS, healthcare, supply chain, energy, trade finance) totaling 6,441 LOC
- ~398 Rust tests passing, 73 conformance tests passing
- The spec is the source of truth; the elaborator implements it
- The evaluator handles full Rule/Operation/Flow evaluation with provenance tracking
- Static analysis covers S1-S8 (entity states, rule reachability, domain coverage, authority topology, effect analysis, flow paths, complexity bounds, verdict uniqueness)

## Constraints

- **CFFP for spec changes**: New constructs and construct modifications go through the [Constraint-First Formalization Protocol (CFFP)](https://github.com/riverline-labs/iap). System construct requires a dedicated CFFP run before any spec text.
- **Spec-first**: Every language change must be specified in `docs/TENOR.md` before implementation
- **Conformance-driven**: Every elaborator change must have conformance suite coverage
- **Deterministic**: Elaboration must be deterministic — identical inputs always produce identical outputs
- **Static verifiability**: No runtime type errors in valid contracts
- **v1.0 frozen**: Spec is frozen at v1.0 including System construct. No breaking changes without a new CFFP run.
- **Trust boundary preservation**: The Rust evaluator is the trusted core. The TypeScript SDK is a client — it does not reimplement evaluation logic.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust for elaborator | Performance, type safety, natural fit for compiler work | ✓ Good |
| Six-pass pipeline | Clear separation of concerns, each pass has single responsibility | ✓ Good |
| JSON interchange format | Universal, tooling-friendly, schema-validatable | ✓ Good |
| Spec before code | Prevents implementation-driven language design | ✓ Good |
| CFFP for construct design | Invariant-driven pressure testing prevents ad-hoc spec changes | ✓ Good |
| Domain validation before codegen | Real contracts surface spec gaps before committing to code generation | ✓ Good |
| §18 Contract Discovery | Manifest format, etag semantics, discovery endpoint, cold-start protocol | ✓ Good |
| v0.9 reframe | Spec complete for core language but lacks multi-contract composition; v1.0 requires System construct | ✓ Good |
| System construct: CFFP Candidate A | Dedicated .tenor file with centralized member declaration; clearest semantics | ✓ Good |
| AAP for spec quality | Assumption Audit Protocol surfaced hidden assumptions before v1.0 freeze | ✓ Good |
| Logic vs operational conformance | §17.1.1 distinction clarifies what elaborator guarantees vs executor responsibilities | ✓ Good |
| SDK-first over codegen-first | Client to proven Rust evaluator ships fast without reimplementing trust-critical logic | — Pending |
| Defer Rust/Go codegen | TypeScript alone is sufficient for v1 tooling; focus on agent SDK | ✓ Good |
| Embedded evaluator as planned phase | Air-gapped/regulated deployments need it; not contingent, just sequenced after SDK | — Pending |

---
*Last updated: 2026-02-23 after milestone Platform & Ecosystem started*
