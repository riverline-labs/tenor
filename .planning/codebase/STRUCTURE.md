# Codebase Structure

**Analysis Date:** 2026-02-21

## Directory Layout

```
tenor/                          # Cargo workspace root
├── Cargo.toml                  # Workspace manifest; workspace-level deps
├── Cargo.lock                  # Lockfile (committed)
├── CLAUDE.md                   # Agent context: DSL casing, build commands, layout
├── README.md                   # Public project readme
├── STABILITY.md                # Stability guarantees
├── CONTRIBUTING.md             # Contributor guide
├── LICENSE / NOTICE            # Apache-2.0 license files
│
├── docs/                       # Specification and schema artifacts
│   ├── TENOR.md                # Full formal specification (v1.0)
│   ├── interchange-schema.json # JSON Schema for TenorInterchange bundles
│   ├── index.md                # Docs index
│   ├── cffp.cue                # CUE schema (CFFP format)
│   └── cffp/                   # CFFP-related documents
│
├── conformance/                # Elaborator conformance test suite
│   ├── positive/               # Valid DSL → expected interchange JSON
│   ├── negative/               # Invalid DSL → expected error JSON
│   │   ├── pass0/              # Lex/parse errors
│   │   ├── pass1/              # Import/bundle errors
│   │   ├── pass2/              # Duplicate id errors
│   │   ├── pass3/              # TypeDecl cycle errors
│   │   ├── pass4/              # Type resolution errors
│   │   └── pass5/              # Structural validation errors
│   ├── numeric/                # Decimal/Money precision fixtures
│   ├── promotion/              # Numeric type promotion fixtures
│   ├── shorthand/              # DSL shorthand expansion fixtures
│   ├── cross_file/             # Multi-file import fixtures
│   ├── parallel/               # Parallel entity conflict fixtures (pass 5)
│   ├── eval/                   # Evaluator conformance fixtures
│   │   ├── positive/           # *.tenor + *.facts.json + *.verdicts.json
│   │   ├── frozen/             # Frozen snapshot semantics tests
│   │   └── numeric/            # Numeric evaluation fixtures
│   └── ambiguity/              # AI ambiguity test cases (*.tenor + *.facts.json + *.verdicts.json)
│
├── crates/                     # Cargo workspace members
│   ├── core/                   # tenor-core: elaboration pipeline library
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs          # Public API re-exports
│   │   │   ├── ast.rs          # RawConstruct, RawType, RawExpr, RawTerm, RawLiteral, Provenance
│   │   │   ├── elaborate.rs    # 6-pass orchestrator: elaborate(path) -> Result<Value, ElabError>
│   │   │   ├── error.rs        # ElabError type
│   │   │   ├── lexer.rs        # Tokenizer: lex(src, filename) -> Result<Vec<Spanned>, ElabError>
│   │   │   ├── parser.rs       # DSL → RawConstruct AST
│   │   │   ├── pass1_bundle.rs # Import resolution, cycle detection, bundle assembly
│   │   │   ├── pass2_index.rs  # Construct indexing, duplicate id detection
│   │   │   ├── pass3_types.rs  # TypeDecl resolution, TypeEnv construction
│   │   │   ├── pass4_typecheck.rs  # TypeRef resolution, expression type checking
│   │   │   ├── pass5_validate.rs   # Structural validation (entity, operation, flow, rule)
│   │   │   └── pass6_serialize.rs  # Canonical JSON interchange serialization
│   │   └── tests/
│   │       └── schema_validation.rs  # JSON Schema conformance tests
│   │
│   ├── cli/                    # tenor-cli: command-line binary
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs         # CLI entry point (clap subcommand dispatch)
│   │   │   ├── runner.rs       # Conformance suite runner (TAP output)
│   │   │   ├── tap.rs          # TAP v14 output formatter
│   │   │   ├── diff.rs         # Bundle structural diff (diff_bundles)
│   │   │   └── ambiguity/      # AI ambiguity testing module
│   │   │       ├── mod.rs      # AmbiguityTestCase, run_ambiguity_suite
│   │   │       ├── api.rs      # Anthropic API client (ureq)
│   │   │       ├── compare.rs  # LLM response parser, verdict comparator
│   │   │       ├── fixtures.rs # Test case loader from ambiguity/ directory
│   │   │       ├── prompt.rs   # System/user prompt builder (reads TENOR.md)
│   │   │       └── report.rs   # AmbiguityReport, TAP printer
│   │   └── tests/
│   │       ├── cli_integration.rs  # Integration tests (elaborate, eval, diff subcommands)
│   │       └── fixtures/       # Test .tenor and .json files for CLI integration tests
│   │
│   ├── eval/                   # tenor-eval: contract evaluator library
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs          # evaluate(), evaluate_flow() public API
│   │   │   ├── types.rs        # Contract, FactSet, VerdictSet, EvalError, Value, TypeSpec
│   │   │   ├── assemble.rs     # FactSet assembly from facts JSON
│   │   │   ├── rules.rs        # Stratified rule evaluation
│   │   │   ├── predicate.rs    # Predicate expression evaluator
│   │   │   ├── operation.rs    # Operation execution, EntityStateMap
│   │   │   ├── flow.rs         # Flow state machine executor, Snapshot
│   │   │   ├── numeric.rs      # Decimal/Money arithmetic and comparisons
│   │   │   └── provenance.rs   # ProvenanceCollector for verdict tracing
│   │   └── tests/
│   │       ├── conformance.rs      # Eval conformance suite (eval/positive, eval/frozen)
│   │       └── numeric_regression.rs  # Numeric precision regression tests
│   │
│   ├── analyze/                # tenor-analyze: static analysis (Phase 4, stub)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs          # Empty stub
│   │
│   ├── codegen/                # tenor-codegen: code generation (Phase 6, stub)
│   │   ├── Cargo.toml
│   │   └── src/lib.rs          # Empty stub
│   │
│   └── lsp/                    # tenor-lsp: Language Server Protocol (Phase 8, stub)
│       ├── Cargo.toml
│       └── src/lib.rs          # Empty stub
│
├── .planning/                  # GSD planning artifacts
│   ├── codebase/               # Codebase analysis documents (this directory)
│   ├── phases/                 # Phase planning documents
│   └── research/               # Research notes
│
└── .github/
    └── workflows/
        └── ci.yml              # CI pipeline (build, conformance, schema, fmt, clippy)
```

## Directory Purposes

**`crates/core/src/`:**
- Purpose: The complete 6-pass elaboration pipeline as a library
- Contains: AST types, lexer, parser, 6 pass modules, orchestrator, error type
- Key files: `elaborate.rs` (entry point), `ast.rs` (shared types), `pass5_validate.rs` (largest file, ~30KB), `pass6_serialize.rs` (largest file, ~32KB)

**`crates/cli/src/`:**
- Purpose: User-facing binary; delegates to core and eval libraries
- Contains: `main.rs` with all `cmd_*` handlers, conformance runner, diff engine, ambiguity testing
- Key files: `main.rs` (subcommand dispatch), `runner.rs` (conformance suite), `diff.rs` (bundle diff)

**`crates/eval/src/`:**
- Purpose: Runtime evaluator; consumes interchange JSON, not DSL
- Contains: Contract deserialization, fact assembly, stratified rule evaluation, flow execution, numeric arithmetic
- Key files: `lib.rs` (public API + integration tests), `types.rs` (all runtime types, ~61KB), `flow.rs` (state machine), `predicate.rs` (expression evaluator)

**`conformance/`:**
- Purpose: Ground-truth fixtures for the conformance test suite
- Contains: Paired `.tenor` source and `.expected.json` / `.expected-error.json` files
- Subdirectory naming maps exactly to the pass numbers and test categories in `crates/cli/src/runner.rs`

**`docs/`:**
- Purpose: Formal specification and interchange schema
- Key files: `TENOR.md` (full spec, also used as system prompt for ambiguity testing), `interchange-schema.json` (embedded in CLI binary via `include_str!`)

**`.planning/`:**
- Purpose: GSD planning artifacts — not compiled, not part of the crate graph
- Contains: Phase plans, codebase analysis documents, research notes

## Key File Locations

**Entry Points:**
- `crates/core/src/elaborate.rs`: `elaborate(path) -> Result<Value, ElabError>` — the elaboration pipeline entry point
- `crates/eval/src/lib.rs`: `evaluate(bundle, facts)` and `evaluate_flow(bundle, facts, flow_id, persona)` — evaluator entry points
- `crates/cli/src/main.rs`: `main()` — CLI binary entry point

**Configuration:**
- `Cargo.toml`: Workspace members and shared dependency versions
- `.github/workflows/ci.yml`: CI pipeline stages and branch triggers
- `docs/interchange-schema.json`: JSON Schema embedded at compile time via `include_str!`

**Core Logic:**
- `crates/core/src/ast.rs`: `RawConstruct` enum — the central data structure for all elaboration passes
- `crates/core/src/error.rs`: `ElabError` — uniform error type serializable to expected-error JSON format
- `crates/core/src/pass5_validate.rs`: Entity, operation, rule, flow structural validation
- `crates/core/src/pass6_serialize.rs`: Canonical interchange JSON serialization with sorted keys
- `crates/eval/src/types.rs`: All evaluator runtime types (`Contract`, `FactSet`, `VerdictSet`, `Value`, `EvalError`)
- `crates/eval/src/flow.rs`: Flow execution state machine with frozen snapshot semantics

**Testing:**
- `crates/cli/src/runner.rs`: Conformance suite runner
- `crates/core/tests/schema_validation.rs`: JSON Schema validation tests
- `crates/eval/tests/conformance.rs`: Evaluator conformance tests
- `crates/eval/tests/numeric_regression.rs`: Numeric precision regression tests
- `crates/cli/tests/cli_integration.rs`: CLI subprocess integration tests
- `crates/eval/src/lib.rs` (inline): Integration tests using hand-constructed interchange bundles

## Naming Conventions

**Files:**
- Pass modules: `pass<N>_<purpose>.rs` — e.g., `pass1_bundle.rs`, `pass5_validate.rs`
- Test files: descriptive snake_case noun phrases — `schema_validation.rs`, `numeric_regression.rs`
- Conformance fixtures: `<construct_or_scenario>_<variant>.<ext>` — e.g., `entity_basic.tenor`, `rule_mul_valid.expected.json`

**Directories:**
- Conformance subdirectories: snake_case, named for the test category — `positive`, `negative`, `cross_file`, `parallel`
- Negative test subdirectories: `pass<N>` matching the elaboration pass that should fail

**Rust Types:**
- AST enums: `Raw` prefix — `RawConstruct`, `RawType`, `RawExpr`, `RawTerm`
- Error types: `*Error` suffix — `ElabError`, `EvalError`, `DiffError`
- Pass result types: descriptive — `Index`, `TypeEnv`
- CLI enums: `Commands`, `OutputFormat`

**Functions:**
- Elaboration pass entry points: `load_bundle`, `build_index`, `build_type_env`, `resolve_types`, `validate`, `serialize`
- CLI command handlers: `cmd_<subcommand>` — `cmd_elaborate`, `cmd_eval`, `cmd_diff`
- Evaluator helpers: `eval_strata`, `eval_rule`, `eval_pred`, `assemble_facts`, `execute_flow`, `execute_operation`

## Where to Add New Code

**New elaboration pass or sub-check:**
- Primary code: `crates/core/src/pass<N>_<name>.rs`
- Export from: `crates/core/src/lib.rs` (add `pub mod` and `pub use`)
- Wire into: `crates/core/src/elaborate.rs`
- Tests: `conformance/negative/pass<N>/` for new error fixtures, `conformance/positive/` for valid cases

**New DSL construct or keyword:**
- AST variant: `crates/core/src/ast.rs` in `RawConstruct` or relevant enum
- Lexer: `crates/core/src/lexer.rs` — add token to `Token` enum, handle in `lex()`
- Parser: `crates/core/src/parser.rs` — add parsing logic
- Indexing: `crates/core/src/pass2_index.rs` — add to `Index` and `build_index()`
- Validation: `crates/core/src/pass5_validate.rs` — add structural checks
- Serialization: `crates/core/src/pass6_serialize.rs` — add to `serialize_construct()`
- Conformance fixtures: `conformance/positive/` and `conformance/negative/`

**New CLI subcommand:**
- Add variant to `Commands` enum in `crates/cli/src/main.rs`
- Add `cmd_<name>()` function in `crates/cli/src/main.rs`
- Match arm in `main()` dispatch block
- If complex, create submodule `crates/cli/src/<name>.rs` or `crates/cli/src/<name>/mod.rs`

**New evaluator capability:**
- Implementation: `crates/eval/src/<module>.rs`
- Declare in: `crates/eval/src/lib.rs` as `pub mod`
- Export types: `pub use <module>::<Type>` in `crates/eval/src/lib.rs`
- Tests: `crates/eval/tests/` or inline `#[cfg(test)]` in `crates/eval/src/lib.rs`
- Eval conformance fixtures: `conformance/eval/positive/` (`.tenor` + `.facts.json` + `.verdicts.json`)

**New conformance fixture:**
- Positive: `conformance/<category>/<name>.tenor` + `conformance/<category>/<name>.expected.json`
- Negative: `conformance/negative/pass<N>/<name>.tenor` + `conformance/negative/pass<N>/<name>.expected-error.json`
- Multi-file: `conformance/cross_file/<root>.tenor` + `conformance/cross_file/<root>.expected.json` (with leaf files in same dir)

**Utilities and shared helpers:**
- Elaborator utilities: inline in the relevant pass module or in `crates/core/src/ast.rs` if truly shared across passes
- Evaluator utilities: `crates/eval/src/numeric.rs` for arithmetic; `crates/eval/src/provenance.rs` for tracing

## Special Directories

**`conformance/`:**
- Purpose: Ground-truth test fixtures; the authoritative record of what the elaborator should produce
- Generated: No — fixtures are hand-authored and version-controlled
- Committed: Yes

**`target/`:**
- Purpose: Cargo build artifacts
- Generated: Yes
- Committed: No (in `.gitignore`)

**`.planning/`:**
- Purpose: GSD planning documents; consumed by plan/execute agents
- Generated: Partially (by mapping agents)
- Committed: Yes

**`.claude/`:**
- Purpose: Claude project context files
- Generated: No
- Committed: Yes

---

*Structure analysis: 2026-02-21*
