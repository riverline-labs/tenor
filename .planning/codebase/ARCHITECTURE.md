# Architecture

**Analysis Date:** 2026-02-22

## Pattern Overview

**Overall:** Multi-crate pipeline compiler with a strict two-boundary design

**Key Characteristics:**
- The `.tenor` DSL is compiled to a canonical TenorInterchange JSON bundle (the "interchange boundary"). All downstream consumers (evaluator, analyzer, codegen, LSP) operate on interchange JSON, never on raw AST.
- The elaboration pipeline is a six-pass sequential compiler with hard pass-number attribution on every error.
- Each crate is a separate library with a clean public API; `tenor-cli` is the only binary crate and acts as the integration layer.
- Downstream crates (`tenor-eval`, `tenor-analyze`) deserialize interchange JSON into their own typed structs — they do NOT import from `tenor-core`.

## Crates

**`crates/core` (tenor-core):**
- Purpose: The elaboration pipeline — `.tenor` source text to TenorInterchange JSON
- Location: `crates/core/src/`
- Contains: Lexer, parser, AST types, six pass modules, `elaborate()` top-level function
- Depends on: `serde_json`, `serde`, `rust_decimal`
- Used by: `tenor-cli`, `tenor-analyze` (via CLI only at runtime), `tenor-eval` (via CLI)

**`crates/cli` (tenor-cli):**
- Purpose: Binary entry point — CLI command dispatch for all toolchain operations
- Location: `crates/cli/src/`
- Contains: `main.rs` (clap-based subcommand dispatch), `runner.rs` (conformance suite), `tap.rs` (TAP v14 output), `diff.rs`, `explain.rs`, ambiguity testing module
- Depends on: `tenor-core`, `tenor-eval`, `tenor-analyze`, `clap`, `sha2`, `jsonschema`

**`crates/eval` (tenor-eval):**
- Purpose: Contract runtime evaluation — facts + interchange bundle → verdicts + flow execution
- Location: `crates/eval/src/`
- Contains: `types.rs` (runtime values, Contract, FactSet, VerdictSet), `assemble.rs` (fact assembly), `rules.rs` (stratified rule evaluation), `flow.rs` (flow execution engine), `operation.rs`, `predicate.rs`, `numeric.rs`, `provenance.rs`
- Depends on: `serde_json`, `rust_decimal`
- Used by: `tenor-cli`

**`crates/analyze` (tenor-analyze):**
- Purpose: Static analysis — eight analysis passes (S1-S8) on interchange bundles
- Location: `crates/analyze/src/`
- Contains: `bundle.rs` (interchange deserializer), `report.rs` (AnalysisReport), `s1_state_space.rs` through `s8_verdict_uniqueness.rs`
- Depends on: `serde_json`, `serde`
- Used by: `tenor-cli`

**`crates/codegen` (tenor-codegen):**
- Purpose: Code generation for TypeScript, Rust, Go targets (Phase 6, stub only)
- Location: `crates/codegen/src/lib.rs`

**`crates/lsp` (tenor-lsp):**
- Purpose: Language Server Protocol implementation for IDE integration (Phase 8, stub only)
- Location: `crates/lsp/src/lib.rs`

## Core Elaboration Pipeline

**Six-Pass Elaborator (`crates/core/src/elaborate.rs`):**

| Pass | Module | Input → Output |
|------|--------|----------------|
| 0+1 | `pass1_bundle.rs` | Source text → parsed `Vec<RawConstruct>` + bundle ID; resolves imports, detects cycles, checks cross-file duplicate IDs |
| 2 | `pass2_index.rs` | `Vec<RawConstruct>` → `Index`; construct lookup map by kind, duplicate ID detection within file |
| 3 | `pass3_types.rs` | `Vec<RawConstruct>` + `Index` → `TypeEnv`; resolves `TypeDecl` records, detects type cycles |
| 4 | `pass4_typecheck.rs` | `Vec<RawConstruct>` + `TypeEnv` → typed `Vec<RawConstruct>`; resolves `TypeRef` → concrete `BaseType`, type-checks expressions |
| 5 | `pass5_validate.rs` | Typed constructs + `Index` → validation; structural checks on Entity, Rule, Operation, Flow, System |
| 6 | `pass6_serialize.rs` | Validated constructs → `serde_json::Value`; canonical interchange JSON with sorted keys |

Entry point: `tenor_core::elaborate::elaborate(root_path: &Path) -> Result<Value, ElabError>`

## Data Flow

**DSL Compilation Flow:**

1. CLI invokes `tenor_core::elaborate::elaborate(path)`
2. Pass 0+1: `lexer::lex()` tokenizes each file; `parser::parse()` builds `Vec<RawConstruct>`; `pass1_bundle::load_bundle()` walks import graph recursively
3. Pass 2: `pass2_index::build_index()` produces `Index` with per-kind `HashMap<String, Provenance>`
4. Pass 3: `pass3_types::build_type_env()` produces `TypeEnv` = `HashMap<String, RawType>`
5. Pass 4: `pass4_typecheck::resolve_types()` rewrites `TypeRef` nodes; `type_check_rules()` validates expression types
6. Pass 5: `pass5_validate::validate()` + `validate_operation_transitions()` check structural invariants
7. Pass 6: `pass6_serialize::serialize()` emits sorted-key JSON with structured numeric values
8. CLI optionally wraps in TenorManifest envelope (SHA-256 etag in `main.rs::build_manifest()`)

**Evaluation Flow:**

1. CLI reads interchange JSON bundle + facts JSON
2. `tenor_eval::evaluate(bundle, facts)` or `tenor_eval::evaluate_flow(bundle, facts, flow_id, persona)`
3. `Contract::from_interchange(bundle)` deserializes into eval-internal typed structs
4. `assemble::assemble_facts()` validates and coerces fact values against declared types
5. `rules::eval_strata()` evaluates rules in stratum order, accumulating `VerdictSet`
6. (Flow mode) `Snapshot` is frozen; `flow::execute_flow()` walks the flow state machine using the frozen snapshot

**Analysis Flow:**

1. CLI elaborates `.tenor` → interchange JSON
2. `tenor_analyze::analyze(bundle)` or `analyze_selected(bundle, analyses)`
3. `AnalysisBundle::from_interchange(bundle)` deserializes into analysis-internal typed structs
4. S1-S8 analyses run in dependency order: S4 requires S3a; S6 requires S5; S7 requires S6
5. `AnalysisReport::extract_findings()` aggregates warnings/infos from all analyses

**State Management:**
- No global mutable state. All pipeline state is passed explicitly as function arguments.
- `RawConstruct` is cloned through passes (derived `Clone`). Pass 4 consumes and returns the construct list.
- Flow execution: `Snapshot` (facts + verdicts) is immutable. `EntityStateMap` is separately mutable.

## Key Abstractions

**`RawConstruct` (`crates/core/src/ast.rs`):**
- Purpose: Unified enum of all DSL construct variants produced by the parser
- Variants: `Import`, `TypeDecl`, `Fact`, `Entity`, `Rule`, `Operation`, `Persona`, `Flow`, `System`
- Pattern: All passes consume `&[RawConstruct]` or `Vec<RawConstruct>`

**`ElabError` (`crates/core/src/error.rs`):**
- Purpose: Structured elaboration error matching expected-error.json conformance format
- Fields: `pass: u8`, `construct_kind`, `construct_id`, `field`, `file`, `line`, `message`
- Constructor helpers: `ElabError::lex()`, `ElabError::parse()`, `ElabError::new()`

**`Index` (`crates/core/src/pass2_index.rs`):**
- Purpose: Fast construct lookup by kind and ID; maps verdict types to producing rules
- Fields: per-kind `HashMap<String, Provenance>`, `rule_verdicts`, `verdict_strata`

**`Contract` (`crates/eval/src/types.rs`):**
- Purpose: Eval-internal deserialized view of an interchange bundle
- Completely distinct from `tenor-core` types — deserialized from JSON, not from AST

**`AnalysisBundle` (`crates/analyze/src/bundle.rs`):**
- Purpose: Analyze-internal deserialized view of an interchange bundle
- Completely distinct from both `tenor-core` and `tenor-eval` types

**Interchange JSON (TenorInterchange):**
- Schema: `docs/interchange-schema.json`; manifest schema: `docs/manifest-schema.json`
- Both embedded in `tenor-cli` binary via `include_str!`
- All JSON keys sorted lexicographically within each object (enforced in Pass 6)
- Bundle kind field values use `PascalCase` (`"Fact"`, `"Rule"`, `"Entity"`, etc.)

## Entry Points

**CLI Binary (`crates/cli/src/main.rs`):**
- Subcommands: `elaborate`, `validate`, `eval`, `test`, `diff`, `check`, `explain`, `generate` (stub), `ambiguity`
- Global flags: `--output [text|json]`, `--quiet`
- Error handling: all commands call `process::exit(1)` on failure

**Library Public API (`crates/core/src/lib.rs`):**
- `elaborate::elaborate(root_path)` — full 6-pass pipeline
- `pass1_bundle::load_bundle(root)` — parse and bundle (selective execution)
- `pass2_index::build_index(constructs)` — indexing (selective execution)
- `pass3_types::build_type_env(constructs, index)` — type env (selective execution)
- `pass4_typecheck::resolve_types(constructs, type_env)` — type resolution (selective execution)

**Eval Library (`crates/eval/src/lib.rs`):**
- `evaluate(bundle, facts) -> Result<EvalResult, EvalError>`
- `evaluate_flow(bundle, facts, flow_id, persona) -> Result<FlowEvalResult, EvalError>`

**Analyze Library (`crates/analyze/src/lib.rs`):**
- `analyze(bundle) -> Result<AnalysisReport, AnalysisError>`
- `analyze_selected(bundle, analyses) -> Result<AnalysisReport, AnalysisError>`

## Error Handling

**Strategy:** Result-returning functions throughout; `process::exit(1)` only at the CLI boundary.

**Patterns:**
- Elaboration: `Result<T, ElabError>` — first-error-wins; pipeline aborts on first failure
- Evaluation: `Result<T, EvalError>` — enum of typed errors (MissingFact, TypeMismatch, Overflow, etc.)
- Analysis: `Result<T, AnalysisError>` — InvalidBundle or MissingField
- CLI: errors formatted as text or JSON depending on `--output` flag; stderr for errors, stdout for results

## Cross-Cutting Concerns

**Serialization:** `serde_json` with `BTreeMap` used in AST and serialization for deterministic key ordering. Pass 6 sorts all construct arrays and JSON object keys explicitly.

**Numeric Precision:** `rust_decimal` crate for all Decimal/Money types; literals preserved as strings through the pipeline until evaluated.

**Import Resolution:** Multi-file bundles assembled via recursive file walking in `pass1_bundle.rs`. Import cycle detection uses a DFS stack. Paths resolved relative to importing file's directory.

**Conformance Testing:** TAP v14 protocol output via `crates/cli/src/tap.rs`. Suite runner in `crates/cli/src/runner.rs` discovers fixture pairs by convention (`.tenor` + `.expected.json` or `.expected-error.json`).

**AI Ambiguity Testing:** `crates/cli/src/ambiguity/` module calls an external LLM API with the spec + DSL sample and checks whether the model correctly elaborates. Invoked via `tenor ambiguity` subcommand.

---

*Architecture analysis: 2026-02-22*
