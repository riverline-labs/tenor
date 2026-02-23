# Codebase Structure

**Analysis Date:** 2026-02-22

## Directory Layout

```
tenor/                          # Workspace root
├── Cargo.toml                  # Workspace manifest (6 member crates)
├── Cargo.lock                  # Lockfile (committed)
├── CLAUDE.md                   # Agent context and build instructions
├── README.md                   # Public documentation
├── CONTRIBUTING.md             # Contribution guidelines
├── STABILITY.md                # Stability policy
├── LICENSE / NOTICE            # Apache 2.0
├── crates/                     # All Rust crate source
│   ├── core/                   # tenor-core: elaboration pipeline (library)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs          # Public API re-exports
│   │   │   ├── ast.rs          # Shared AST types (RawConstruct, RawExpr, etc.)
│   │   │   ├── elaborate.rs    # 6-pass orchestrator (entry: elaborate())
│   │   │   ├── error.rs        # ElabError type
│   │   │   ├── lexer.rs        # Tokenizer
│   │   │   ├── parser.rs       # DSL -> raw AST
│   │   │   ├── pass1_bundle.rs # Pass 0+1: import resolution, bundle assembly
│   │   │   ├── pass2_index.rs  # Pass 2: construct indexing
│   │   │   ├── pass3_types.rs  # Pass 3: type environment
│   │   │   ├── pass4_typecheck.rs # Pass 4: type resolution, expression checking
│   │   │   ├── pass5_validate.rs  # Pass 5: structural validation
│   │   │   └── pass6_serialize.rs # Pass 6: JSON interchange serialization
│   │   └── tests/
│   │       ├── schema_validation.rs  # JSON schema validation tests
│   │       ├── conformance.rs        # Elaboration conformance tests
│   │       ├── numeric_regression.rs # Decimal/money precision tests
│   │       └── analysis_tests.rs     # Analysis integration tests
│   ├── cli/                    # tenor-cli: binary (command-line tool)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs         # CLI entry point (clap subcommand dispatch)
│   │   │   ├── runner.rs       # Conformance suite runner
│   │   │   ├── tap.rs          # TAP v14 output formatter
│   │   │   ├── diff.rs         # Bundle diff logic
│   │   │   ├── explain.rs      # Human-readable contract summary
│   │   │   └── ambiguity/      # AI ambiguity testing module
│   │   │       ├── mod.rs
│   │   │       ├── api.rs      # LLM API client
│   │   │       ├── compare.rs  # Output comparison
│   │   │       ├── fixtures.rs # Fixture loading
│   │   │       ├── prompt.rs   # Prompt construction
│   │   │       └── report.rs   # Ambiguity test report
│   │   └── tests/
│   │       ├── cli_integration.rs  # End-to-end CLI tests (assert_cmd)
│   │       └── fixtures/           # CLI test fixture files
│   ├── eval/                   # tenor-eval: contract evaluator (library)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs          # Public API: evaluate(), evaluate_flow()
│   │   │   ├── types.rs        # Runtime types: Contract, FactSet, VerdictSet, EvalError
│   │   │   ├── assemble.rs     # Fact assembly and validation
│   │   │   ├── rules.rs        # Stratified rule evaluation
│   │   │   ├── flow.rs         # Flow execution engine (frozen snapshot semantics)
│   │   │   ├── operation.rs    # Operation execution and entity state management
│   │   │   ├── predicate.rs    # Predicate expression evaluator
│   │   │   ├── numeric.rs      # Numeric operations (Decimal, Money)
│   │   │   └── provenance.rs   # Verdict provenance tracking
│   │   └── tests/              # Eval integration tests
│   ├── analyze/                # tenor-analyze: static analysis (library)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs          # Public API: analyze(), analyze_selected()
│   │   │   ├── bundle.rs       # Interchange JSON deserializer (AnalysisBundle)
│   │   │   ├── report.rs       # AnalysisReport, Finding, FindingSeverity
│   │   │   ├── s1_state_space.rs    # S1: Entity state space enumeration
│   │   │   ├── s2_reachability.rs   # S2: Dead state detection
│   │   │   ├── s3a_admissibility.rs # S3a: Operation admissibility
│   │   │   ├── s4_authority.rs      # S4: Persona authority mapping
│   │   │   ├── s5_verdicts.rs       # S5: Verdict space analysis
│   │   │   ├── s6_flow_paths.rs     # S6: Flow path enumeration
│   │   │   ├── s7_complexity.rs     # S7: Predicate and flow complexity
│   │   │   └── s8_verdict_uniqueness.rs # S8: Verdict uniqueness confirmation
│   │   └── tests/              # Analysis integration tests
│   ├── codegen/                # tenor-codegen: code generation (stub, Phase 6)
│   │   └── src/lib.rs
│   └── lsp/                    # tenor-lsp: LSP implementation (stub, Phase 8)
│       └── src/lib.rs
├── conformance/                # Conformance test suite
│   ├── positive/               # Valid DSL -> expected interchange JSON
│   ├── negative/               # Invalid DSL -> expected error JSON
│   │   ├── pass0/              # Lex/parse errors
│   │   ├── pass1/              # Import/bundle errors
│   │   ├── pass2/              # Duplicate ID errors
│   │   ├── pass3/              # Type environment errors
│   │   ├── pass4/              # Type check errors
│   │   └── pass5/              # Structural validation errors
│   ├── numeric/                # Decimal/money precision fixtures
│   ├── promotion/              # Numeric type promotion fixtures
│   ├── shorthand/              # DSL shorthand expansion fixtures
│   ├── cross_file/             # Multi-file import fixtures
│   ├── parallel/               # Parallel entity conflict fixtures
│   ├── manifest/               # Manifest envelope tests
│   ├── eval/                   # Evaluator conformance fixtures
│   │   ├── positive/           # .tenor + .facts.json + .verdicts.json
│   │   ├── frozen/             # Frozen snapshot semantics tests
│   │   └── numeric/            # Numeric evaluation tests
│   ├── analysis/               # Static analysis conformance fixtures
│   └── ambiguity/              # AI ambiguity test fixtures
├── docs/                       # Formal specification and documentation
│   ├── TENOR.md                # Full formal specification (v1.0)
│   ├── interchange-schema.json # TenorInterchange JSON Schema
│   ├── manifest-schema.json    # TenorManifest JSON Schema
│   ├── index.md                # Documentation index
│   ├── aap/                    # Agent-addressable protocol docs
│   ├── cffp/                   # CFFP migration docs
│   └── guide/                  # Author guides
├── domains/                    # Real-world domain examples
│   ├── saas/                   # SaaS subscription contracts
│   ├── healthcare/             # Healthcare domain contracts
│   ├── trade_finance/          # Trade finance contracts
│   ├── supply_chain/           # Supply chain contracts
│   ├── energy_procurement/     # Energy procurement contracts
│   └── system_scenario/        # System construct examples
└── .planning/                  # GSD planning artifacts
    ├── codebase/               # Codebase analysis documents (THIS directory)
    ├── milestones/             # Milestone phase plans
    ├── phases/                 # Individual phase plans
    └── research/               # Research documents
```

## Directory Purposes

**`crates/core/src/`:**
- Purpose: Elaboration pipeline core. The only place where `.tenor` DSL is parsed.
- Key files: `elaborate.rs` (pipeline orchestrator), `ast.rs` (all shared types), `pass1_bundle.rs` through `pass6_serialize.rs` (pipeline passes)
- Rule: This crate must compile standalone. No dependency on eval or analyze.

**`crates/cli/src/`:**
- Purpose: All user-facing toolchain operations. Integration point for all library crates.
- Key files: `main.rs` (all CLI command handlers are inline here), `runner.rs` (conformance suite)
- The schemas are embedded via `include_str!("../../../docs/interchange-schema.json")`

**`crates/eval/src/`:**
- Purpose: Runtime contract evaluation against facts. Operates on interchange JSON, not DSL AST.
- Key files: `lib.rs` (public API), `types.rs` (all eval-internal types), `flow.rs` (flow state machine)

**`crates/analyze/src/`:**
- Purpose: Static analysis of compiled contracts. Operates on interchange JSON.
- Key files: `lib.rs` (public API + dependency ordering), `bundle.rs` (interchange deserializer)
- Each `sN_*.rs` module is a self-contained analysis pass.

**`conformance/`:**
- Purpose: Golden-file test suite. The suite runner (`runner.rs`) discovers all fixture pairs automatically.
- Convention: `.tenor` file + `.expected.json` (positive), `.tenor` + `.expected-error.json` (negative)

**`docs/`:**
- Purpose: Formal specification. `TENOR.md` is the authoritative language spec. JSON schemas are normative.
- The schemas are embedded in the `tenor-cli` binary and used for `tenor validate`.

**`domains/`:**
- Purpose: Reference contracts demonstrating Tenor in real domains. Not part of the test suite.

## Key File Locations

**Entry Points:**
- `crates/cli/src/main.rs`: CLI binary main function and all command handler functions
- `crates/core/src/elaborate.rs`: `elaborate()` function — top-level library entry point
- `crates/eval/src/lib.rs`: `evaluate()` and `evaluate_flow()` — evaluator entry points
- `crates/analyze/src/lib.rs`: `analyze()` and `analyze_selected()` — analyzer entry points

**Configuration:**
- `Cargo.toml`: Workspace manifest; all shared dependency versions declared here under `[workspace.dependencies]`
- `.github/workflows/ci.yml`: CI pipeline definition

**Core Logic:**
- `crates/core/src/ast.rs`: All shared types shared across all passes — start here when modifying the DSL
- `crates/core/src/pass1_bundle.rs`: Import resolution and multi-file assembly
- `crates/core/src/pass6_serialize.rs`: Interchange JSON format — controls what the output looks like
- `crates/eval/src/types.rs`: All eval-internal types including Contract deserialization
- `crates/eval/src/flow.rs`: Flow execution engine with frozen snapshot semantics
- `crates/analyze/src/bundle.rs`: Interchange deserializer for analysis — parallel to eval's `types.rs`

**Schemas (Normative):**
- `docs/interchange-schema.json`: TenorInterchange JSON Schema (embedded in CLI binary)
- `docs/manifest-schema.json`: TenorManifest envelope schema (embedded in CLI binary)

**Specification:**
- `docs/TENOR.md`: Full formal Tenor language specification

## Naming Conventions

**Files:**
- Pass modules: `pass{N}_{name}.rs` (e.g., `pass3_types.rs`, `pass4_typecheck.rs`)
- Analysis modules: `s{N}_{name}.rs` (e.g., `s1_state_space.rs`, `s3a_admissibility.rs`)
- Test modules: descriptive snake_case (e.g., `schema_validation.rs`, `cli_integration.rs`)
- Conformance fixtures: `{name}.tenor` + `{name}.expected.json` or `{name}.expected-error.json`

**Types:**
- AST types: `Raw` prefix (e.g., `RawConstruct`, `RawExpr`, `RawType`)
- Eval-internal types: no prefix, plain names (e.g., `Contract`, `FactSet`, `VerdictSet`)
- Analysis-internal types: `Analysis` prefix for deserialized constructs (e.g., `AnalysisBundle`, `AnalysisEntity`)
- Error types: `ElabError`, `EvalError`, `AnalysisError`, `DiffError` — one per crate boundary

**Functions:**
- Pass entry points: verb phrase (`build_index`, `build_type_env`, `resolve_types`, `serialize`)
- Public library entries: bare verb (`elaborate`, `evaluate`, `analyze`)
- CLI handlers: `cmd_{subcommand}` (e.g., `cmd_elaborate`, `cmd_eval`, `cmd_check`)

## Where to Add New Code

**New DSL construct:**
1. Add variant to `RawConstruct` enum in `crates/core/src/ast.rs`
2. Add lexer/parser support in `crates/core/src/lexer.rs` and `crates/core/src/parser.rs`
3. Add indexing in `crates/core/src/pass2_index.rs`
4. Add type checking in `crates/core/src/pass4_typecheck.rs`
5. Add structural validation in `crates/core/src/pass5_validate.rs`
6. Add serialization in `crates/core/src/pass6_serialize.rs`
7. Add positive conformance fixture in `conformance/positive/`
8. Add negative conformance fixtures for each new error case in `conformance/negative/pass{N}/`

**New elaboration error:**
- Return `Err(ElabError::new(pass_number, construct_kind, construct_id, field, file, line, message))`
- Add negative conformance fixture in `conformance/negative/pass{N}/{name}.tenor` + `{name}.expected-error.json`

**New CLI subcommand:**
- Add variant to `Commands` enum in `crates/cli/src/main.rs`
- Add handler function `cmd_{name}()` in `crates/cli/src/main.rs`
- Add match arm in `main()`

**New static analysis pass:**
- Create `crates/analyze/src/s{N}_{name}.rs` following the pattern of existing passes
- Add module declaration and pub use in `crates/analyze/src/lib.rs`
- Wire into `analyze()` and `analyze_selected()` in `crates/analyze/src/lib.rs`
- Add field to `AnalysisReport` in `crates/analyze/src/report.rs`

**New eval capability:**
- Add support in `crates/eval/src/types.rs` (Contract deserialization)
- Implement in appropriate module under `crates/eval/src/`
- Expose in `crates/eval/src/lib.rs` public API if needed

**New conformance fixture:**
- Positive: create `conformance/positive/{name}.tenor` + `conformance/positive/{name}.expected.json`
- Negative: create `conformance/negative/pass{N}/{name}.tenor` + `conformance/negative/pass{N}/{name}.expected-error.json`
- The runner discovers fixtures automatically — no registration required

**New domain example:**
- Create directory under `domains/{domain_name}/`
- Use lowercase `.tenor` keywords in all source files

## Special Directories

**`target/`:**
- Purpose: Cargo build output
- Generated: Yes
- Committed: No

**`.planning/`:**
- Purpose: GSD planning artifacts — milestones, phase plans, codebase analysis
- Generated: Partially (by GSD agents)
- Committed: Yes

**`.github/`:**
- Purpose: GitHub Actions CI configuration
- Key file: `.github/workflows/ci.yml`
- Committed: Yes

**`conformance/`:**
- Purpose: Golden-file test fixtures for the elaboration pipeline
- Generated: No (hand-authored, normative)
- Committed: Yes
- Note: All fixture JSON keys must be sorted lexicographically (enforced by Pass 6 and validated by CI)

---

*Structure analysis: 2026-02-22*
