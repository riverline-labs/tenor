# Tenor — Full Product

## What This Is

Tenor is a contract-based business process runtime. You write formal contracts in a DSL that declares entities, states, transitions, rules, personas, operations, and flows. The elaborator compiles contracts to interchange JSON. The evaluator computes action spaces — what each persona can do given the current state of facts and entities. The platform executes actions atomically with full provenance. Tenor turns business processes into verifiable, auditable, formally specified systems.

## Core Value

Given a contract and the current state of the world, Tenor tells you exactly what actions are available, why blocked actions are blocked, and executes chosen actions with full provenance — no ambiguity, no hidden logic.

## Requirements

### Validated

- ✓ DSL parser and 6-pass elaborator — existing
- ✓ Conformance suite (positive, negative, numeric, promotion, shorthand, cross-file, parallel) — existing
- ✓ Interchange JSON schema and validation — existing
- ✓ CLI (elaborate, validate, test conformance, diff) — existing
- ✓ WASM evaluator (tenor-eval-wasm, Node target) — existing
- ✓ Eval crate with FactProvider trait, StaticFactProvider, action space computation — existing
- ✓ Storage crate with OCC conformance tests — existing
- ✓ Interchange crate with typed bundle representation — existing
- ✓ Spec v1.0 (docs/TENOR.md) — existing
- ✓ Migration spec audit (AL items resolved) — existing
- ✓ Migration spec amendment (M1-M8 defined) — existing

### Active

- [ ] Complete migration analysis (check_flow_compatibility, MigrationPlan type)
- [ ] Migration executor (atomic bulk entity transition with rollback)
- [ ] Migration CLI command
- [ ] Ingestion DSL spec amendment
- [ ] Ingestion elaborator changes
- [ ] Fact assembly with source verification
- [ ] Ingestion adapters (trait + reference implementations)
- [ ] Provenance enrichment for ingestion
- [ ] tenor connect (environment introspection, fact-to-source matching, adapter generation)
- [ ] Multi-instance entity support in evaluator
- [ ] WASM evaluator instance update
- [ ] Contract signing (Ed25519)
- [ ] WASM bundle signing
- [ ] Executor conformance suite
- [ ] Human-in-the-loop agent policy
- [ ] LLM agent policy
- [ ] Composite agent policy
- [ ] Multi-language SDKs (Rust, Go, TypeScript, Python) with conformance suite
- [ ] Automatic UI (schema derivation, component library, theme system, CLI generation)
- [ ] Tenor Builder (conversational authoring, visual editor, live preview, simulation)
- [ ] Hosted platform (multi-tenant, provisioning, auth, API gateway, billing)
- [ ] Tenor Marketplace (registry, templates, one-click deploy, community contributions)

### Out of Scope

- Reimplementing the evaluator in other languages — SDKs wrap WASM, one source of truth
- Mobile-native apps — web-first for automatic UI
- Custom storage backends beyond what the storage trait supports — users implement the trait
- Rewriting the spec — v1.0 is closed, extensions only

## Context

- Elaborator is a 6-pass pipeline (lex/parse → bundle → index → types → typecheck → validate → serialize)
- Evaluator computes action spaces from contracts + facts + entity states
- Storage uses OCC (optimistic concurrency control) with version-based conflict detection
- WASM crate is excluded from workspace — needs separate build/test commands
- Phase 1 (Migration) is in progress: spec audit + amendment done, migration analysis underway
- diff() and DiffEntry types exist in crates/cli/src/diff.rs
- The codebase has both public (open-source) and private (platform) components; this project tracks both but execution is currently scoped to the public repo
- ELv2 license on platform code requires commercial licensing for third-party hosted use

## Constraints

- **Spec-first**: Phases 1 (Migration) and 2 (Ingestion) require spec amendments before implementation
- **Ordering**: Phase 1 → 2 sequential; Phase 3 follows 2 (degraded mode possible earlier); Phase 4+6 parallel with 1; Phase 5 after 1-4; Growth phases 7→8→9→10→11 sequential
- **WASM parity**: Any evaluator change must be reflected in the WASM build and pass wasm-pack tests
- **Pre-commit gates**: cargo fmt, build, test, conformance, clippy must all pass before every commit
- **Single evaluator**: SDKs wrap WASM — no reimplementation in other languages
- **Contract as source of truth**: Automatic UI, Builder, and Marketplace all derive from contracts

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Spec extensions, not rewrites | v1.0 is stable; migration and ingestion extend rather than modify | — Pending |
| SDKs wrap WASM evaluator | One source of truth for evaluation logic; no divergence across languages | — Pending |
| Automatic UI derives from contracts | Contract defines both behavior and presentation; regenerate on change | — Pending |
| Builder uses LLM + evaluator feedback loop | Spec is LLM-authorable; evaluator explain output enables self-correction | — Pending |
| Hosted platform runs same code as self-hosted | No fork, no feature divergence; hosting is convenience, not lock-in | — Pending |
| Public + private repo split | Core language/evaluator open-source; platform/hosting commercial (ELv2) | — Pending |

---
*Last updated: 2026-02-25 after initialization*
