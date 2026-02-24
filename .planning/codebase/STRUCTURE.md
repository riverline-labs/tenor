# Codebase Structure

**Analysis Date:** 2026-02-23

## Directory Layout

```
tenor/                          # Workspace root
├── Cargo.toml                  # Workspace manifest (6 member crates)
├── Cargo.lock                  # Lockfile (committed)
├── Dockerfile                  # Multi-stage Docker build (rust:1.93-slim → debian:trixie-slim)
├── docker-compose.yml          # Evaluator service definition
├── CLAUDE.md                   # Agent context and build instructions
├── README.md                   # Public documentation
├── CONTRIBUTING.md             # Contribution guidelines
├── STABILITY.md                # Stability policy
├── LICENSE / NOTICE            # Apache 2.0
├── crates/                     # All Rust crate source
│   ├── core/                   # tenor-core: elaboration pipeline (library)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs          # Public API re-exports, version constants
│   │   │   ├── ast.rs          # Shared AST types (RawConstruct, RawExpr, etc.)
│   │   │   ├── elaborate.rs    # 6-pass orchestrator (entry: elaborate())
│   │   │   ├── error.rs        # ElabError type
│   │   │   ├── lexer.rs        # Tokenizer
│   │   │   ├── parser.rs       # DSL → raw AST (1,598 LOC)
│   │   │   ├── pass1_bundle.rs # Pass 0+1: import resolution, bundle assembly
│   │   │   ├── pass2_index.rs  # Pass 2: construct indexing
│   │   │   ├── pass3_types.rs  # Pass 3: type environment
│   │   │   ├── pass4_typecheck.rs # Pass 4: type resolution, expression checking
│   │   │   ├── pass5_validate.rs  # Pass 5: structural validation (1,506 LOC)
│   │   │   └── pass6_serialize.rs # Pass 6: JSON interchange serialization (1,044 LOC)
│   │   └── tests/
│   │       └── schema_validation.rs  # JSON schema validation tests
│   ├── cli/                    # tenor-cli: binary (command-line tool)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs         # CLI entry point, clap subcommand dispatch (1,187 LOC)
│   │   │   ├── runner.rs       # Conformance suite runner
│   │   │   ├── tap.rs          # TAP v14 output formatter
│   │   │   ├── diff.rs         # Bundle diff and breaking change classification (1,449 LOC)
│   │   │   ├── explain.rs      # Human-readable contract summary (1,478 LOC)
│   │   │   ├── manifest.rs     # TenorManifest envelope generation
│   │   │   ├── serve.rs        # HTTP JSON API server (tiny_http)
│   │   │   └── ambiguity/      # AI ambiguity testing module
│   │   │       ├── mod.rs
│   │   │       ├── api.rs      # LLM API client (Anthropic)
│   │   │       ├── compare.rs  # Output comparison
│   │   │       ├── fixtures.rs # Fixture loading
│   │   │       ├── prompt.rs   # Prompt construction
│   │   │       └── report.rs   # Ambiguity test report
│   │   └── tests/
│   │       ├── cli_integration.rs    # End-to-end CLI tests (assert_cmd)
│   │       ├── serve_integration.rs  # HTTP server tests
│   │       └── fixtures/             # CLI test fixture files
│   ├── eval/                   # tenor-eval: contract evaluator (library)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs          # Public API: evaluate(), evaluate_flow()
│   │   │   ├── types.rs        # Runtime types: Contract, FactSet, VerdictSet, EvalError (1,842 LOC)
│   │   │   ├── assemble.rs     # Fact assembly and validation
│   │   │   ├── rules.rs        # Stratified rule evaluation
│   │   │   ├── flow.rs         # Flow execution engine, frozen snapshot semantics (1,510 LOC)
│   │   │   ├── operation.rs    # Operation execution and entity state management
│   │   │   ├── predicate.rs    # Predicate expression evaluator
│   │   │   ├── numeric.rs      # Decimal/money arithmetic (rust_decimal)
│   │   │   └── provenance.rs   # Verdict provenance tracking
│   │   └── tests/
│   │       ├── conformance.rs        # Evaluator conformance tests
│   │       └── numeric_regression.rs # Decimal/money precision regression tests (1,254 LOC)
│   ├── analyze/                # tenor-analyze: static analysis (library)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs          # Public API: analyze(), analyze_selected()
│   │   │   ├── bundle.rs       # Interchange JSON deserializer (AnalysisBundle) (774 LOC)
│   │   │   ├── report.rs       # AnalysisReport, Finding, FindingSeverity
│   │   │   ├── s1_state_space.rs    # S1: Entity state space enumeration
│   │   │   ├── s2_reachability.rs   # S2: Dead state detection
│   │   │   ├── s3a_admissibility.rs # S3a: Operation admissibility (615 LOC)
│   │   │   ├── s4_authority.rs      # S4: Persona authority mapping
│   │   │   ├── s5_verdicts.rs       # S5: Verdict space analysis
│   │   │   ├── s6_flow_paths.rs     # S6: Flow path enumeration (730 LOC)
│   │   │   ├── s7_complexity.rs     # S7: Predicate and flow complexity
│   │   │   └── s8_verdict_uniqueness.rs # S8: Verdict uniqueness confirmation
│   │   └── tests/
│   │       └── analysis_tests.rs    # Analysis integration tests
│   ├── codegen/                # tenor-codegen: TypeScript code generation
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs              # Public API: generate_typescript()
│   │   │   ├── bundle.rs           # CodegenBundle deserializer
│   │   │   ├── typescript.rs       # types.ts emission
│   │   │   ├── typescript_client.rs # client.ts emission
│   │   │   └── typescript_schemas.rs # schemas.ts emission
│   │   └── tests/
│   │       └── codegen_integration.rs # Code generation integration tests
│   └── lsp/                    # tenor-lsp: Language Server Protocol
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs               # Public API: run()
│           ├── server.rs            # LSP main loop (stdio transport)
│           ├── diagnostics.rs       # Error → LSP diagnostic conversion
│           ├── completion.rs        # Keyword/construct completion
│           ├── hover.rs             # Hover information
│           ├── navigation.rs        # Go-to-definition, references (809 LOC)
│           ├── semantic_tokens.rs   # Semantic token highlighting
│           ├── document.rs          # Open document state management
│           └── agent_capabilities.rs # Agent capabilities extraction
├── sdk/                        # Language SDKs
│   └── typescript/             # TypeScript SDK (@tenor-lang/sdk)
│       ├── package.json        # npm package config (ESM + CJS dual-package)
│       ├── src/
│       │   ├── index.ts        # Barrel re-export
│       │   ├── client.ts       # TenorClient: HTTP client to evaluator API
│       │   ├── types.ts        # SDK type definitions (Contract, Fact, Verdict, etc.)
│       │   └── errors.ts       # SDK error classes
│       ├── tests/
│       │   └── client.test.ts  # Client unit tests (Node.js --test runner)
│       └── examples/           # SDK usage examples
├── editors/                    # Editor integrations
│   └── vscode/                 # VS Code extension (tenor-lang)
│       ├── package.json        # Extension manifest (contributes: language, grammar, commands)
│       ├── src/
│       │   ├── extension.ts    # Extension entry point (LSP client setup)
│       │   ├── commands.ts     # Command implementations (elaborate, validate, etc.)
│       │   ├── agentPanel.ts   # Agent capabilities webview panel
│       │   ├── statusBar.ts    # Status bar item
│       │   └── svgRenderer.ts  # SVG rendering for agent capabilities
│       ├── syntaxes/
│       │   └── tenor.tmLanguage.json # TextMate grammar for syntax highlighting
│       ├── snippets/
│       │   └── tenor.json      # Code snippets
│       └── language-configuration.json # Bracket/comment configuration
├── conformance/                # Conformance test suite
│   ├── positive/               # Valid DSL → expected interchange JSON
│   ├── negative/               # Invalid DSL → expected error JSON
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
│   ├── TENOR.md                # Full formal specification (v1.0, ~190K)
│   ├── interchange-schema.json # TenorInterchange JSON Schema (~34K)
│   ├── manifest-schema.json    # TenorManifest JSON Schema
│   ├── index.md                # Documentation index
│   ├── cffp.cue                # CFFP migration protocol definition
│   ├── aap/                    # Assumption Audit Protocol docs
│   ├── cffp/                   # CFFP constraint documents
│   └── guide/                  # Authoring guides
├── domains/                    # Real-world domain examples (~4,480 LOC Tenor)
│   ├── saas/                   # SaaS subscription contracts
│   ├── healthcare/             # Healthcare domain contracts
│   ├── trade_finance/          # Trade finance contracts
│   ├── supply_chain/           # Supply chain contracts
│   ├── energy_procurement/     # Energy procurement contracts
│   └── system_scenario/        # Multi-contract System construct examples
├── .github/
│   └── workflows/
│       └── ci.yml              # GitHub Actions CI pipeline
└── .planning/                  # GSD planning artifacts
    ├── PROJECT.md              # Project description and context
    ├── REQUIREMENTS.md         # Active requirements
    ├── MILESTONES.md           # Milestone tracker
    ├── ROADMAP.md              # Phase roadmap
    ├── STATE.md                # Current project state
    ├── config.json             # GSD configuration
    └── codebase/               # Codebase analysis documents (THIS directory)
```

## Directory Purposes

**`crates/core/src/`:**
- Purpose: Elaboration pipeline core. The only place where `.tenor` DSL is parsed.
- Key files: `elaborate.rs` (pipeline orchestrator), `ast.rs` (all shared types), `pass1_bundle.rs` through `pass6_serialize.rs` (pipeline passes)
- Rule: This crate compiles standalone. No dependency on eval, analyze, codegen, or lsp.

**`crates/cli/src/`:**
- Purpose: All user-facing toolchain operations. Integration point for all library crates.
- Key files: `main.rs` (CLI command dispatch + handlers), `runner.rs` (conformance suite), `serve.rs` (HTTP server)
- Dependencies: tenor-core, tenor-eval, tenor-analyze, tenor-codegen, tenor-lsp
- The schemas are embedded via `include_str!("../../../docs/interchange-schema.json")`

**`crates/eval/src/`:**
- Purpose: Runtime contract evaluation against facts. Operates on interchange JSON, not DSL AST.
- Key files: `lib.rs` (public API), `types.rs` (all eval-internal types including Contract deserialization), `flow.rs` (flow state machine)
- Dependencies: tenor-core (for AST types only), serde_json, rust_decimal, time

**`crates/analyze/src/`:**
- Purpose: Static analysis of compiled contracts. Operates on interchange JSON.
- Key files: `lib.rs` (public API + dependency ordering), `bundle.rs` (interchange deserializer)
- Each `sN_*.rs` module is a self-contained analysis pass.
- Dependencies: tenor-core (for AST types), serde, serde_json

**`crates/codegen/src/`:**
- Purpose: TypeScript code generation from interchange JSON bundles.
- Generates: `types.ts`, `schemas.ts`, `client.ts`, `index.ts` per contract
- Dependencies: serde, serde_json (no dependency on tenor-core)

**`crates/lsp/src/`:**
- Purpose: Language Server Protocol server for IDE integration.
- Features: diagnostics on save, semantic tokens, completion, hover, go-to-definition, agent capabilities
- Dependencies: tenor-core (for elaboration), tenor-analyze (for diagnostics), lsp-server, lsp-types

**`sdk/typescript/`:**
- Purpose: TypeScript client SDK for the Tenor evaluator HTTP API.
- Key files: `src/client.ts` (TenorClient), `src/types.ts` (SDK type definitions)
- Published as `@tenor-lang/sdk`, dual ESM + CJS package
- No dependency on Rust crates — pure HTTP client

**`editors/vscode/`:**
- Purpose: VS Code extension for Tenor language support.
- Features: syntax highlighting (TextMate grammar), LSP client, commands (elaborate, validate), agent capabilities panel, snippets
- Key files: `src/extension.ts` (entry point, LSP client setup), `src/commands.ts` (command implementations)
- Dependencies: `vscode-languageclient` (connects to `tenor lsp` over stdio)

**`conformance/`:**
- Purpose: Golden-file test suite. The suite runner (`runner.rs`) discovers all fixture pairs automatically.
- Convention: `.tenor` file + `.expected.json` (positive), `.tenor` + `.expected-error.json` (negative)
- 112 `.tenor` fixture files across all subdirectories

**`docs/`:**
- Purpose: Formal specification. `TENOR.md` is the authoritative language spec (frozen at v1.0). JSON schemas are normative.
- The schemas are embedded in the `tenor-cli` binary and used for `tenor validate`.

**`domains/`:**
- Purpose: Reference contracts demonstrating Tenor in real domains (6 domains, ~4,480 LOC). Not part of the test suite but validated by CI.

## Key File Locations

**Entry Points:**
- `crates/cli/src/main.rs`: CLI binary entry point and all command handler functions
- `crates/core/src/elaborate.rs`: `elaborate()` function — top-level library entry for elaboration
- `crates/eval/src/lib.rs`: `evaluate()` and `evaluate_flow()` — evaluator entry points
- `crates/analyze/src/lib.rs`: `analyze()` and `analyze_selected()` — analyzer entry points
- `crates/codegen/src/lib.rs`: `generate_typescript()` — code generation entry point
- `crates/lsp/src/lib.rs`: `run()` — LSP server entry point
- `crates/cli/src/serve.rs`: `start_server()` — HTTP API server entry point

**Configuration:**
- `Cargo.toml`: Workspace manifest; all shared dependency versions declared under `[workspace.dependencies]`
- `.github/workflows/ci.yml`: CI pipeline definition (5 stages)
- `Dockerfile`: Multi-stage Docker build for evaluator service
- `docker-compose.yml`: Docker Compose service definition

**Core Logic:**
- `crates/core/src/ast.rs`: All shared AST types — start here when modifying the DSL
- `crates/core/src/pass1_bundle.rs`: Import resolution and multi-file assembly
- `crates/core/src/pass6_serialize.rs`: Interchange JSON format — controls output shape
- `crates/eval/src/types.rs`: All eval-internal types including Contract deserialization from interchange JSON
- `crates/eval/src/flow.rs`: Flow execution engine with frozen snapshot semantics
- `crates/analyze/src/bundle.rs`: Interchange deserializer for analysis — parallel to eval's `types.rs`

**Schemas (Normative):**
- `docs/interchange-schema.json`: TenorInterchange JSON Schema (embedded in CLI binary)
- `docs/manifest-schema.json`: TenorManifest envelope schema (embedded in CLI binary)

**Specification:**
- `docs/TENOR.md`: Full formal Tenor language specification (v1.0, ~190K)

## Naming Conventions

**Files:**
- Pass modules: `pass{N}_{name}.rs` (e.g., `pass3_types.rs`, `pass4_typecheck.rs`)
- Analysis modules: `s{N}_{name}.rs` (e.g., `s1_state_space.rs`, `s3a_admissibility.rs`)
- Test modules: descriptive snake_case (e.g., `schema_validation.rs`, `cli_integration.rs`)
- Conformance fixtures: `{name}.tenor` + `{name}.expected.json` or `{name}.expected-error.json`
- Codegen output: kebab-case directories, `types.ts`/`schemas.ts`/`client.ts`/`index.ts`

**Types:**
- AST types: `Raw` prefix (e.g., `RawConstruct`, `RawExpr`, `RawType`)
- Eval-internal types: no prefix, plain names (e.g., `Contract`, `FactSet`, `VerdictSet`)
- Analysis-internal types: `Analysis` prefix (e.g., `AnalysisBundle`, `AnalysisEntity`)
- Codegen-internal types: `Codegen` prefix (e.g., `CodegenBundle`, `CodegenError`)
- Error types: `ElabError`, `EvalError`, `AnalysisError`, `CodegenError` — one per crate boundary

**Functions:**
- Pass entry points: verb phrase (`build_index`, `build_type_env`, `resolve_types`, `serialize`)
- Public library entries: bare verb (`elaborate`, `evaluate`, `analyze`, `generate_typescript`)
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

**New HTTP API endpoint:**
- Add route match in `crates/cli/src/serve.rs` request dispatch loop
- Follow existing pattern: parse request, call library function, return JSON response

**New static analysis pass:**
- Create `crates/analyze/src/s{N}_{name}.rs` following the pattern of existing passes
- Add module declaration and pub use in `crates/analyze/src/lib.rs`
- Wire into `analyze()` and `analyze_selected()` in `crates/analyze/src/lib.rs`
- Add field to `AnalysisReport` in `crates/analyze/src/report.rs`

**New eval capability:**
- Add support in `crates/eval/src/types.rs` (Contract deserialization)
- Implement in appropriate module under `crates/eval/src/`
- Expose in `crates/eval/src/lib.rs` public API if needed

**New codegen target:**
- Create new module in `crates/codegen/src/{target}.rs`
- Add public entry function in `crates/codegen/src/lib.rs`
- Wire into CLI via new `GenerateCommands` variant in `crates/cli/src/main.rs`

**New LSP feature:**
- Add capability module in `crates/lsp/src/`
- Register handler in `crates/lsp/src/server.rs`

**New conformance fixture:**
- Positive: create `conformance/positive/{name}.tenor` + `conformance/positive/{name}.expected.json`
- Negative: create `conformance/negative/pass{N}/{name}.tenor` + `conformance/negative/pass{N}/{name}.expected-error.json`
- The runner discovers fixtures automatically — no registration required

**New domain example:**
- Create directory under `domains/{domain_name}/`
- Use lowercase `.tenor` keywords in all source files

**New TypeScript SDK feature:**
- Add types in `sdk/typescript/src/types.ts`
- Add client methods in `sdk/typescript/src/client.ts`
- Add tests in `sdk/typescript/tests/`
- Re-export from `sdk/typescript/src/index.ts`

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

**`sdk/typescript/dist/`:**
- Purpose: Compiled TypeScript SDK output (ESM + CJS + type declarations)
- Generated: Yes (by `npm run build`)
- Committed: Currently yes (should be in `.gitignore`)

**`editors/vscode/out/`:**
- Purpose: Compiled VS Code extension JavaScript
- Generated: Yes (by `npm run compile`)
- Committed: Currently yes (should be in `.gitignore`)

**`editors/vscode/node_modules/`, `sdk/typescript/node_modules/`:**
- Purpose: npm dependencies
- Generated: Yes
- Committed: Should not be (`.gitignore` has `node_modules/`)

---

*Structure analysis: 2026-02-23*
