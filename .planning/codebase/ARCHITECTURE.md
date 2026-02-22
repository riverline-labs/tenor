# Architecture

**Analysis Date:** 2026-02-21

## Pattern Overview

**Overall:** Multi-stage compiler pipeline with a separate runtime evaluator

**Key Characteristics:**
- The elaboration pipeline (tenor-core) is a pure transformation: `.tenor` DSL source → TenorInterchange JSON bundle. No runtime concerns enter the elaborator.
- The evaluator (tenor-eval) is a separate crate that consumes only interchange JSON, never raw DSL. This enforces a strict boundary: the interchange format is the contract between compiler and runtime.
- All crates are organized as a Cargo workspace. The CLI (`tenor-cli`) is the public surface that orchestrates both subsystems.
- Four future crates (`analyze`, `codegen`, `lsp`) are registered in the workspace with stub `lib.rs` files; none contain implementation yet.

## Layers

**DSL Parsing Layer (tenor-core, passes 0+1):**
- Purpose: Convert `.tenor` source text to a raw construct list
- Location: `crates/core/src/lexer.rs`, `crates/core/src/parser.rs`, `crates/core/src/pass1_bundle.rs`
- Contains: `Token` enum, `Spanned` struct, `RawConstruct` AST enum tree
- Depends on: filesystem (reads `.tenor` files), nothing else
- Used by: elaboration orchestrator `crates/core/src/elaborate.rs`

**Elaboration Pipeline (tenor-core, passes 2-6):**
- Purpose: Validate and transform the raw construct list into a canonical JSON bundle
- Location: `crates/core/src/pass2_index.rs` through `crates/core/src/pass6_serialize.rs`
- Contains: `Index`, `TypeEnv`, typed AST transformations, structural validation, JSON serialization
- Depends on: AST types from `crates/core/src/ast.rs`, error type from `crates/core/src/error.rs`
- Used by: `elaborate()` in `crates/core/src/elaborate.rs`, directly by conformance runner

**Interchange Boundary:**
- Purpose: The canonical JSON interchange format is the stable contract between elaborator and all downstream consumers
- Location: `docs/interchange-schema.json` (JSON Schema), embedded via `include_str!` in `crates/cli/src/main.rs`
- Constructs in the bundle are sorted: Facts → Entities → Personas → Rules (by stratum) → Operations → Flows

**Evaluation Layer (tenor-eval):**
- Purpose: Execute contract logic against a set of facts, producing verdicts with provenance
- Location: `crates/eval/src/`
- Contains: `Contract` (deserialized from interchange), `FactSet`, `VerdictSet`, `VerdictInstance`, `Snapshot`, flow execution state machine
- Depends on: interchange JSON only (not tenor-core types); `rust_decimal` for arithmetic
- Used by: `cmd_eval` in `crates/cli/src/main.rs`

**CLI Layer (tenor-cli):**
- Purpose: User-facing command dispatch; orchestrates elaboration, evaluation, diff, validation, conformance testing, AI ambiguity testing
- Location: `crates/cli/src/main.rs`
- Contains: clap-based `Cli` / `Commands` structs; `cmd_*` functions; `runner`, `diff`, `tap`, `ambiguity` submodules
- Depends on: `tenor_core`, `tenor_eval`

**Stub Crates (future phases):**
- `crates/analyze/src/lib.rs` — static analysis (Phase 4, not implemented)
- `crates/codegen/src/lib.rs` — code generation (Phase 6, not implemented)
- `crates/lsp/src/lib.rs` — Language Server Protocol (Phase 8, not implemented)

## Data Flow

**Elaboration (DSL → Interchange JSON):**

1. CLI calls `tenor_core::elaborate::elaborate(root_path)` (`crates/core/src/elaborate.rs`)
2. Pass 0+1 (`pass1_bundle::load_bundle`): reads root `.tenor` file, resolves `import` directives recursively (DFS), detects cycles, concatenates all `RawConstruct` values into a flat `Vec<RawConstruct>`
3. Pass 2 (`pass2_index::build_index`): scans construct list, builds `Index` (per-kind HashMaps of id → Provenance), detects duplicate ids within and across files
4. Pass 3 (`pass3_types::build_type_env`): resolves `TypeDecl` constructs into a `TypeEnv` (name → `RawType`), detects `TypeDecl` cycles
5. Pass 4 (`pass4_typecheck::resolve_types` + `type_check_rules`): replaces all `RawType::TypeRef` nodes with concrete types; validates rule predicate expression types
6. Pass 5 (`pass5_validate::validate` + `validate_operation_transitions`): structural checks — entity state machine validity, operation effect references, flow step graph validity, flow cycles, parallel branch entity conflicts
7. Pass 6 (`pass6_serialize::serialize`): serializes the validated construct list to a `serde_json::Value` with lexicographically sorted keys and structured numeric values
8. CLI prints the bundle as pretty-printed JSON to stdout

**Evaluation (Interchange JSON + Facts → Verdicts):**

1. CLI calls `tenor_eval::evaluate(bundle, facts)` (`crates/eval/src/lib.rs`)
2. `Contract::from_interchange(bundle)`: deserializes interchange JSON into typed `Contract` struct
3. `assemble::assemble_facts(contract, facts)`: validates and coerces each fact value against its declared type; applies defaults for missing optional facts
4. `rules::eval_strata(contract, fact_set)`: iterates strata 0..max, evaluates each rule's predicate against `FactSet + VerdictSet`, accumulates `VerdictInstance` values with provenance
5. Returns `EvalResult { verdicts: VerdictSet }`

**Flow Execution (extends Evaluation):**

1. CLI calls `tenor_eval::evaluate_flow(bundle, facts, flow_id, persona)`
2. Runs steps 1-4 above to produce rules verdicts
3. Creates an immutable `Snapshot { facts, verdicts }` — never mutated during flow execution (per spec Section 11.4)
4. Initializes a mutable `EntityStateMap` from contract initial states
5. `flow::execute_flow` walks the flow as a state machine: OperationStep, BranchStep, HandoffStep, SubFlowStep, ParallelStep, each updating `EntityStateMap` and recording `StepRecord` values
6. Returns `FlowEvalResult { verdicts, flow_result }`

**Conformance Testing:**

1. CLI `cmd_test` calls `runner::run_suite(suite_dir)` (`crates/cli/src/runner.rs`)
2. Runner discovers positive and negative fixture files, calls `elaborate::elaborate` on each `.tenor` file
3. Positive tests: compares JSON output to `*.expected.json` using `json_equal` (normalizes number types)
4. Negative tests: compares `ElabError::to_json_value()` to `*.expected-error.json`
5. Results emitted in TAP v14 format via `crates/cli/src/tap.rs`

**State Management:**
- All elaboration pass functions are stateless transformations: they take inputs and return outputs or errors
- `ElabError` is the single error type across all passes; carries `pass` number, location, and message
- Evaluation maintains mutable `EntityStateMap` during flow execution only; all other state is immutable

## Key Abstractions

**RawConstruct:**
- Purpose: The unified AST node type; a Rust enum covering all Tenor language constructs
- Location: `crates/core/src/ast.rs`
- Pattern: One `Vec<RawConstruct>` threaded through all elaboration passes; passes destructure it with `match`

**Index:**
- Purpose: Fast O(1) construct lookup by kind and id; also carries `rule_verdicts` and `verdict_strata` maps
- Location: `crates/core/src/pass2_index.rs`
- Pattern: Built once in Pass 2, passed by reference to Passes 4 and 5

**TypeEnv:**
- Purpose: Maps `TypeDecl` names to fully-resolved `RawType` values; a `HashMap<String, RawType>`
- Location: `crates/core/src/pass3_types.rs`
- Pattern: Built in Pass 3, used in Pass 4 to rewrite `TypeRef` nodes

**ElabError:**
- Purpose: Uniform error representation for all elaboration passes; serializes to the expected-error JSON format used in negative conformance fixtures
- Location: `crates/core/src/error.rs`
- Pattern: Carry `pass` (u8), `construct_kind`, `construct_id`, `field`, `file`, `line`, `message`; `to_json_value()` always includes all fields (nulls for missing)

**Contract (eval):**
- Purpose: Strongly-typed Rust representation of an interchange bundle for evaluation
- Location: `crates/eval/src/types.rs`
- Pattern: Deserialized from `serde_json::Value` via `Contract::from_interchange()`; entirely independent of tenor-core AST types

**Snapshot (eval):**
- Purpose: Immutable freeze of `FactSet + VerdictSet` taken at flow initiation
- Location: `crates/eval/src/flow.rs`
- Pattern: Passed by reference throughout flow execution; entity state changes use a separate mutable `EntityStateMap`

## Entry Points

**`elaborate()` function:**
- Location: `crates/core/src/elaborate.rs`
- Triggers: CLI `elaborate` subcommand, conformance runner, tests
- Responsibilities: Orchestrates passes 0-6; returns `Result<serde_json::Value, ElabError>`

**`evaluate()` function:**
- Location: `crates/eval/src/lib.rs`
- Triggers: CLI `eval` subcommand; integration tests in `crates/eval/src/lib.rs`
- Responsibilities: Deserializes bundle, assembles facts, evaluates rules, returns `EvalResult`

**`evaluate_flow()` function:**
- Location: `crates/eval/src/lib.rs`
- Triggers: Test infrastructure, future CLI integration
- Responsibilities: Full evaluation pipeline plus flow state machine execution

**CLI `main()`:**
- Location: `crates/cli/src/main.rs`
- Triggers: `cargo run -p tenor-cli -- <subcommand>`
- Responsibilities: Parses arguments via clap, dispatches to `cmd_elaborate`, `cmd_validate`, `cmd_eval`, `cmd_test`, `cmd_diff`, `cmd_ambiguity`; stub exits for `check`, `explain`, `generate`

**`runner::run_suite()`:**
- Location: `crates/cli/src/runner.rs`
- Triggers: CLI `test` subcommand; CI pipeline
- Responsibilities: Discovers conformance fixture files across all subdirectories, runs elaboration, compares output, emits TAP

## Error Handling

**Strategy:** All errors propagate as `Result<T, ElabError>` through the elaboration pipeline. The evaluator uses its own `EvalError` enum. The CLI maps errors to exit code 1 and prints structured JSON or text.

**Patterns:**
- Elaboration: Early return `Err(ElabError)` from any pass; `elaborate()` unwraps each pass with `?`
- `ElabError::new()` constructor takes pass number, optional construct context fields, file/line, message
- `ElabError::lex()` and `ElabError::parse()` are convenience constructors for pass 0
- Evaluation: `EvalError` enum with named fields per variant; `fmt::Display` implemented for human-readable messages
- CLI: `process::exit(1)` on error; `process::exit(2)` for stub (not yet implemented) subcommands
- Conformance: TAP `not ok` lines carry diff/mismatch details; test runner always runs all tests (no abort on first failure)

## Cross-Cutting Concerns

**Logging:** None. No logging framework. Diagnostic output goes to `eprintln!` in the CLI layer only.

**Validation:** Structural validation is entirely the responsibility of Pass 5 (`pass5_validate.rs`). Type validation is Pass 4. The evaluator performs runtime type-checking during fact assembly.

**Authentication:** Not applicable. No network auth in the elaborator or evaluator. The `ambiguity` subcommand reads `ANTHROPIC_API_KEY` from the environment via `api::get_api_key()` in `crates/cli/src/ambiguity/api.rs`.

**Serialization:** `serde_json` is used throughout. The interchange format enforces lexicographically sorted JSON keys (implemented manually in Pass 6 using `serde_json::Map`). Numeric values use the structured `decimal_value` / `money_value` interchange format, not JSON numbers, for precision.

---

*Architecture analysis: 2026-02-21*
