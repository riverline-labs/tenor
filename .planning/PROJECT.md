# Tenor v1.0

## What This Is

Tenor is a domain-specific language for expressing operational contracts — the rules, entities, operations, and flows that govern how decisions are made in complex business domains. The elaborator transforms `.tenor` source files into a canonical JSON interchange format through a six-pass pipeline (lex, parse, bundle, index, type-check, validate, serialize). The language is currently at v0.3 spec with a working reference elaborator and 47-test conformance suite. This project takes it to 1.0.

## Core Value

A contract authored in TenorDSL must be statically verifiable, evaluable against facts, and generatable into working code — the full lifecycle from specification to execution with provenance at every step.

## Requirements

### Validated

<!-- Shipped and confirmed working in v0.3. -->

- ✓ Six-pass elaborator (lex → parse → bundle → index → type-check → validate → serialize) — v0.3
- ✓ DSL constructs: Fact, Entity, Rule, Operation, Flow, TypeDecl — v0.3
- ✓ Import resolution with cycle detection — v0.3
- ✓ TypeDecl with named type resolution and cycle detection — v0.3
- ✓ Numeric type system (Int, Decimal, Money) with promotion rules — v0.3
- ✓ Entity state machine validation (DAG acyclicity, transition validity) — v0.3
- ✓ Rule stratum ordering and verdict references — v0.3
- ✓ Operation transition and effect validation — v0.3
- ✓ Flow step graph validation — v0.3
- ✓ Canonical JSON interchange with sorted keys and structured numeric values — v0.3
- ✓ Conformance suite (47 tests: positive, negative, numeric, promotion, shorthand, cross-file, parallel) — v0.3
- ✓ Structured error reporting with file/line provenance — v0.3

### Active

<!-- Current scope: v0.3 → 1.0 -->

- [ ] Persona as first-class declared construct with id and metadata
- [ ] Operation outcome typing (named outcome types, statically enumerable)
- [ ] Shared type library (cross-contract type reuse for Record and TaggedUnion)
- [ ] Unified `tenor` CLI binary with subcommands (elaborate, validate, check, eval, explain, test)
- [ ] Evaluator: interchange bundle + facts JSON → verdict set + provenance
- [ ] Evaluator conformance suite
- [ ] Static analyzer implementing S1–S7 from spec
- [ ] Domain validation: 5–10 real contracts across distinct domains
- [ ] Code generation (ports and adapters pattern, TypeScript first, Rust second)
- [ ] Local adapter package for dev/test use
- [ ] VS Code extension (syntax highlighting, inline errors, check on save, go-to-definition)
- [ ] Language reference documentation (author-facing)
- [ ] Authoring guide with worked domain examples
- [ ] Executor implementation guide
- [ ] Code generation guide

### Out of Scope

<!-- Explicit boundaries for 1.0. -->

- P5 module federation (inter-org type sharing) — complexity explosion, defer to post-1.0
- Runtime monitoring / contract enforcement in production — separate operational concern
- GUI contract editor — premature; need CLI and authoring experience first
- Code generation targets beyond TypeScript + Rust — prove the pattern first
- Formal proof of soundness — separate research track, not blocking 1.0

## Context

- The elaborator is a single Rust binary with minimal dependencies (serde, serde_json)
- `elaborate.rs` is 2,066 lines / 58 functions — monolithic but all tests pass
- The spec (`docs/TENOR.md`) is the source of truth; the elaborator implements it
- P5 (shared types) and P7 (outcome typing) are known spec gaps documented in the v0.3 spec
- Personas are used in contract examples but lack formal declaration syntax
- No evaluator, executor, static analyzer, or code generator exists yet
- The conformance suite uses TAP output format with a custom runner
- There is an `evaluator/` directory but it is not wired into anything yet

## Constraints

- **Spec-first**: Every language change must be specified in `docs/TENOR.md` before implementation
- **Conformance-driven**: Every elaborator change must have conformance suite coverage
- **Deterministic**: Elaboration must be deterministic — identical inputs always produce identical outputs
- **Static verifiability**: No runtime type errors in valid contracts
- **Backward compatibility**: Existing valid `.tenor` files must continue to elaborate correctly through 1.0

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rust for elaborator | Performance, type safety, natural fit for compiler work | ✓ Good |
| Six-pass pipeline | Clear separation of concerns, each pass has single responsibility | ✓ Good |
| JSON interchange format | Universal, tooling-friendly, schema-validatable | ✓ Good |
| Spec before code | Prevents implementation-driven language design | ✓ Good |
| Ports and adapters for codegen | Separates generated domain from developer-supplied adapters | — Pending |
| TypeScript as first codegen target | Widest adoption, fastest iteration for domain validation | — Pending |
| Domain validation before codegen | Real contracts surface spec gaps before committing to code generation | — Pending |

---
*Last updated: 2026-02-21 after initialization*
