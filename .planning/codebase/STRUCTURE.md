# Codebase Structure

**Analysis Date:** 2026-02-25

## Directory Layout

```
/Users/bwb/src/riverline/tenor/
├── crates/                    # Rust workspace (8 crates)
│   ├── core/                  # Tenor elaborator pipeline
│   ├── cli/                   # Command-line toolchain
│   ├── eval/                  # Contract evaluator engine
│   ├── analyze/               # Static analysis suite (S1-S8)
│   ├── codegen/               # TypeScript code generator
│   ├── interchange/           # Shared interchange types
│   ├── lsp/                   # Language Server Protocol
│   ├── storage/               # Storage abstraction traits
│   └── tenor-eval-wasm/       # WASM bindings (excluded from workspace)
├── conformance/               # Test suite fixtures
│   ├── positive/              # Valid DSL → expected interchange JSON
│   ├── negative/              # Invalid DSL → expected error JSON
│   ├── numeric/               # Decimal/money precision fixtures
│   ├── promotion/             # Numeric type promotion fixtures
│   ├── shorthand/             # DSL shorthand expansion fixtures
│   ├── cross_file/            # Multi-file import fixtures
│   ├── parallel/              # Parallel entity conflict fixtures
│   ├── eval/                  # Evaluation test cases
│   ├── analysis/              # Static analysis test cases
│   ├── manifest/              # Manifest format test cases
│   └── ambiguity/             # AI ambiguity testing fixtures
├── docs/                      # Formal specification and guides
│   ├── TENOR.md               # v1.0 specification (comprehensive)
│   ├── index.md               # Documentation index
│   └── guide/                 # User guides
├── domains/                   # Example contract domains
│   ├── healthcare/            # Prior authorization contracts
│   ├── supply_chain/          # Inspection contracts
│   ├── energy_procurement/    # Procurement workflows
│   └── system_scenario/       # System-level examples
├── examples/                  # Integration examples
│   ├── express-middleware/    # Express.js HTTP middleware
│   ├── slack-bot/             # Slack bot integration
│   └── audit-agent/           # Audit reporting tool
├── sdk/                       # SDK definitions and stubs
├── editors/                   # Editor integrations
├── .github/workflows/         # CI pipeline (ci.yml)
├── .planning/                 # GSD working directory
├── Cargo.toml                 # Workspace configuration
├── Cargo.lock                 # Dependency lock file
├── CLAUDE.md                  # Agent instructions (DSL, pre-commit, build)
├── README.md                  # Project overview
└── LICENSE                    # Apache 2.0
```

## Directory Purposes

**crates/core:**
- Purpose: Tenor elaborator — parser and 6-pass compilation pipeline
- Contains: Lexer, parser, raw AST, error types, all elaboration passes, source provider abstraction
- Key files: `lib.rs` (public API), `elaborate.rs` (6-pass orchestrator), `ast.rs` (shared types), `error.rs` (ElabError)

**crates/cli:**
- Purpose: Command-line toolchain with subcommands for elaborate, eval, validate, test, check, diff, explain, serve, manifest, agent
- Contains: Clap argument parser, subcommand handlers, TAP output formatter, conformance test runner, AI ambiguity testing
- Key files: `main.rs` (Clap CLI struct), `runner.rs` (conformance test execution), `serve.rs` (LSP server launcher)

**crates/eval:**
- Purpose: Contract evaluator — executes rules, produces verdicts, handles flow execution and entity state
- Contains: Fact assembly, stratified rule evaluation, verdict computation with provenance, flow execution, operation effects
- Key files: `lib.rs` (public API), `types.rs` (Contract/FactSet/VerdictSet), `rules.rs` (rule evaluation), `flow.rs` (flow execution), `operation.rs` (operation effects)

**crates/analyze:**
- Purpose: Static analysis suite (S1-S8) for contract properties
- Contains: 8 independent analysis modules (s1_state_space through s8_verdict_uniqueness), dependency resolver, finding aggregator
- Key files: `lib.rs` (public API with analyze/analyze_selected), `report.rs` (AnalysisReport), `bundle.rs` (deserialization wrapper)

**crates/codegen:**
- Purpose: Generate TypeScript types, JSON schemas, and client bindings from contracts
- Contains: Interchange deserialization, AST-to-TypeScript emitters, case converters, barrel file generation
- Key files: `lib.rs` (public API), `typescript.rs` (type/case converters), `typescript_schemas.rs` (JSON schema emitter), `typescript_client.ts` (client stub emitter)

**crates/interchange:**
- Purpose: Shared interchange JSON type definitions
- Contains: Typed structs for all Tenor construct kinds (Fact, Entity, Rule, Operation, Flow, Persona, System, TypeDecl), deserialization entry point
- Key files: `lib.rs` (public API), `types.rs` (construct type definitions), `deserialize.rs` (from_interchange function)

**crates/lsp:**
- Purpose: Language Server Protocol implementation for IDE support
- Contains: Diagnostic generation from elaboration errors, hover information, code completion stubs
- Key files: `lib.rs` (public API), `diagnostics.rs` (LSP diagnostic mapper)

**crates/storage:**
- Purpose: Trait-based contract for persistence of execution state
- Contains: Record type definitions (EntityStateRecord, EntityTransitionRecord, FlowExecutionRecord, OperationExecutionRecord, ProvenanceRecord), TenorStorage trait
- Key files: `traits.rs` (TenorStorage trait), `record.rs` (record types)

**conformance/:**
- Purpose: Test fixtures for elaboration, evaluation, analysis, and manifest format
- Organization: Subdirectories by test category, each containing `.tenor` + `.expected.json` or `.tenor` + `.expected-error.json` pairs
- Pattern: Positive tests must elaborate without error and match JSON exactly; negative tests must fail with specified error
- Example fixture: `conformance/positive/fact_basic.tenor` + `conformance/positive/fact_basic.expected.json`

**docs/:**
- Purpose: Formal specification and user documentation
- Key files:
  - `TENOR.md` — 1.0 specification with complete grammar, type system, operation semantics, analysis taxonomy
  - `guide/what-is-tenor.md` — Introduction for new users
  - `guide/author-guide.md` — Contract authoring guide
  - `guide/minimal-kernel.md` — Core concepts for agent understanding

**domains/:**
- Purpose: Complete example contracts demonstrating real-world use cases
- Examples:
  - `healthcare/prior_auth.tenor` — Prior authorization workflow with multiple approval types
  - `supply_chain/inspection.tenor` — Inspection hold/pass decisions
  - `energy_procurement/` — Procurement request workflow
- Pattern: Multi-file contracts with imports, types, entities, rules, operations, flows

**examples/:**
- Purpose: Integration examples showing how to embed Tenor in applications
- Examples:
  - `express-middleware/` — HTTP middleware for Express.js
  - `slack-bot/` — Slack bot showing flow execution
  - `audit-agent/` — Report generation tool

## Key File Locations

**Entry Points:**
- `crates/cli/src/main.rs` — CLI application entry point (Clap command dispatch)
- `crates/core/src/elaborate.rs` — `elaborate()` and `elaborate_with_provider()` functions
- `crates/eval/src/lib.rs` — `evaluate()` and `evaluate_flow()` functions
- `crates/analyze/src/lib.rs` — `analyze()` and `analyze_selected()` functions
- `crates/codegen/src/lib.rs` — `generate_typescript()` function

**Configuration:**
- `Cargo.toml` — Workspace configuration with 8 crates and shared dependencies
- `CLAUDE.md` — Agent instructions, pre-commit checks, build commands
- `.github/workflows/ci.yml` — CI pipeline (runs workspace build, conformance suite, schema validation, formatting, clippy)

**Core Logic:**
- `crates/core/src/lexer.rs` — Tokenizer with `lex()` function
- `crates/core/src/parser.rs` — Recursive descent parser, produces RawConstruct list
- `crates/core/src/pass1_bundle.rs` — Import resolution, duplicate detection, bundle assembly
- `crates/core/src/pass2_index.rs` — Construct indexing by (kind, id)
- `crates/core/src/pass3_types.rs` — Type environment building, TypeRef resolution
- `crates/core/src/pass4_typecheck.rs` — Type checking, expression validation, numeric promotion
- `crates/core/src/pass5_validate.rs` — Structural validation (entity/operation/rule/flow constraints)
- `crates/core/src/pass6_serialize.rs` — JSON interchange serialization with canonical key ordering
- `crates/eval/src/rules.rs` — Stratified rule evaluation with verdict computation
- `crates/eval/src/flow.rs` — Flow step execution, entity state management
- `crates/eval/src/operation.rs` — Operation execution, effect recording
- `crates/analyze/src/s1_state_space.rs` through `s8_verdict_uniqueness.rs` — Individual analyses

**Testing:**
- `conformance/` — Fixture pairs (`.tenor` + `.expected.json` or `.expected-error.json`)
- `crates/core/tests/schema_validation.rs` — JSON schema validation tests
- `crates/eval/tests/` — Evaluation tests
- `crates/analyze/tests/` — Analysis tests
- `crates/codegen/tests/` — Code generation tests
- `crates/cli/tests/` — CLI tests with fixture files in `crates/cli/tests/fixtures/`

## Naming Conventions

**Files:**
- Rust source files: snake_case (e.g., `pass4_typecheck.rs`, `type_check_rules()`)
- Elaboration passes: `passN_description.rs` (e.g., `pass1_bundle.rs`, `pass6_serialize.rs`)
- Analysis modules: `sN_description.rs` (e.g., `s1_state_space.rs`, `s8_verdict_uniqueness.rs`)
- Tests: `description.rs` (inline #[test]), integration tests in `tests/` subdirectories
- Example contracts: kebab-case (e.g., `prior_auth.tenor`, `inspection_hold.tenor`)

**Directories:**
- Crates: kebab-case (e.g., `tenor-eval-wasm`)
- Logical groupings: snake_case (e.g., `cross_file`, `prior_auth`)
- Conformance categories: snake_case (e.g., `positive`, `numeric`, `cross_file`)

**Functions:**
- Public API functions: verb + noun (e.g., `elaborate()`, `evaluate()`, `analyze()`, `build_index()`, `load_bundle()`)
- Pass orchestrators: `elaborate()` in pass modules (e.g., `pass3_types::build_type_env()`)
- Helper functions: descriptive (e.g., `check_cross_file_dups()`, `resolve_import()`)

**Types:**
- Construct types: PascalCase noun (e.g., `RawConstruct`, `ElabError`, `Contract`, `FactSet`)
- Enum variants: PascalCase (e.g., `RawConstruct::Fact`, `RawType::Decimal`)
- Error types: suffix `Error` (e.g., `ElabError`, `EvalError`, `AnalysisError`)
- Trait types: suffix with behavior (e.g., `SourceProvider`, `TenorStorage`)

## Where to Add New Code

**New Elaboration Pass (unlikely - spec is complete):**
- File: `crates/core/src/passN_description.rs`
- Entry function: `pub fn orchestrate_pass(constructs: &[RawConstruct]) -> Result<..., ElabError>`
- Integration: Add to `elaborate.rs` orchestrator, add to workspace test suite
- Tests: Add conformance fixtures to `conformance/` directory

**New Evaluation Feature (e.g., new expression type):**
- Core logic: `crates/eval/src/` (new module or extend existing `rules.rs`, `operation.rs`)
- Type definition: Update `crates/eval/src/types.rs`
- Interchange representation: Update `crates/interchange/src/types.rs` to support new construct
- Tests: Add to `crates/eval/tests/` and conformance fixtures
- Example: New operation type would go in `operation.rs`, need interchange support, need evaluation tests

**New Analysis (Phase 4 extension):**
- Module: `crates/analyze/src/sN_description.rs`
- Entry function: `pub fn analyze_topic(bundle: &AnalysisBundle) -> Result<TopicResult, AnalysisError>`
- Integration: Add to `analyze()` and `analyze_selected()` in `lib.rs`, resolve dependencies
- Report type: Add to `crates/analyze/src/report.rs`
- Tests: Add to `crates/analyze/tests/` and conformance fixtures

**New Code Generator Output (Phase 6 extension):**
- Module: `crates/codegen/src/language_name.rs` (e.g., `python.rs`, `go.rs`)
- Entry function: `pub fn generate_language(bundle: &CodegenBundle) -> Result<String, CodegenError>`
- Integration: Add to `lib.rs` as `pub fn generate_language(...)`
- Emitters: Create helper functions for each construct type
- Tests: Add to `crates/codegen/tests/`

**Shared Utilities:**
- Type definitions: `crates/interchange/src/types.rs` (shared structs)
- Error types: `crates/eval/src/lib.rs` (EvalError) or `crates/analyze/src/bundle.rs` (AnalysisError)
- Test helpers: Create in respective `tests/` directory

## Special Directories

**target/:**
- Purpose: Cargo build artifacts
- Generated: Yes (by `cargo build`)
- Committed: No (.gitignore)

**.planning/:**
- Purpose: GSD working directory for analysis and phase planning
- Generated: Yes (by agent commands)
- Committed: No (on separate branch)

**.github/workflows/:**
- Purpose: CI pipeline definitions
- Generated: No
- Committed: Yes
- Key file: `ci.yml` runs cargo fmt, build, test, conformance, schema validation, clippy on main/v1 branches

**conformance/:**
- Purpose: Test fixture source of truth
- Generated: No
- Committed: Yes (all `.tenor` and `.expected*.json` files)

---

*Structure analysis: 2026-02-25*
