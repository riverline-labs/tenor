# Architecture

**Analysis Date:** 2026-02-23

## Pattern Overview

**Overall:** Multi-crate pipeline compiler with a strict interchange-boundary design

**Key Characteristics:**
- The `.tenor` DSL is compiled to a canonical TenorInterchange JSON bundle (the "interchange boundary"). All downstream consumers (evaluator, analyzer, codegen, LSP) operate on interchange JSON, never on raw AST.
- The elaboration pipeline is a six-pass sequential compiler with hard pass-number attribution on every error.
- Each crate is a separate library with a clean public API; `tenor-cli` is the only binary crate and acts as the integration layer.
- Downstream crates (`tenor-eval`, `tenor-analyze`, `tenor-codegen`) deserialize interchange JSON into their own typed structs -- they do NOT import from `tenor-core`. This enforces the interchange boundary as the single point of coupling.
- The system is entirely synchronous -- no async runtime anywhere (no tokio, no async-std).

## Crate Dependency Graph

```
tenor-cli (binary)
  ├── tenor-core       (elaboration pipeline)
  ├── tenor-eval       (contract evaluation)
  ├── tenor-analyze    (static analysis S1-S8)
  ├── tenor-codegen    (TypeScript code generation)
  └── tenor-lsp        (Language Server Protocol)
        ├── tenor-core     (for elaboration + diagnostics)
        └── tenor-analyze  (for analysis results in agent capabilities)
```

**Isolation rule:** `tenor-eval`, `tenor-analyze`, and `tenor-codegen` do NOT depend on each other or on `tenor-core`. They consume only `serde_json::Value` (interchange JSON). `tenor-lsp` is the exception: it depends on `tenor-core` (for elaboration-based diagnostics) and `tenor-analyze` (for agent capabilities).

## Crates

**`crates/core` (tenor-core):**
- Purpose: The elaboration pipeline -- `.tenor` source text to TenorInterchange JSON
- Location: `crates/core/src/`
- Contains: Lexer, parser, AST types, six pass modules, `elaborate()` top-level function
- Key types: `RawConstruct`, `RawType`, `RawExpr`, `RawTerm`, `RawLiteral`, `Provenance`, `ElabError`, `Index`, `TypeEnv`
- Depends on: `serde_json`, `serde`, `rust_decimal`
- Used by: `tenor-cli`, `tenor-lsp`

**`crates/cli` (tenor-cli):**
- Purpose: Binary entry point -- CLI command dispatch, HTTP server, conformance runner
- Location: `crates/cli/src/`
- Contains: `main.rs` (clap dispatch), `serve.rs` (HTTP API server), `runner.rs` (conformance suite), `tap.rs` (TAP v14 output), `diff.rs`, `explain.rs`, `manifest.rs`, `ambiguity/` (AI testing module)
- Depends on: all 5 library crates + `clap`, `sha2`, `jsonschema`, `tiny_http`, `ureq`, `tempfile`, `libc`

**`crates/eval` (tenor-eval):**
- Purpose: Contract runtime evaluation -- interchange bundle + facts JSON -> verdicts + flow execution
- Location: `crates/eval/src/`
- Contains: `types.rs` (runtime values, Contract, FactSet, VerdictSet), `assemble.rs` (fact assembly), `rules.rs` (stratified rule evaluation), `flow.rs` (flow execution engine), `operation.rs` (operation execution + entity state management), `predicate.rs` (predicate evaluation), `numeric.rs` (decimal arithmetic), `provenance.rs` (verdict provenance tracking)
- Depends on: `serde_json`, `serde`, `rust_decimal`, `time`
- Used by: `tenor-cli`

**`crates/analyze` (tenor-analyze):**
- Purpose: Static analysis -- eight analysis passes (S1-S8) on interchange bundles
- Location: `crates/analyze/src/`
- Contains: `bundle.rs` (interchange deserializer), `report.rs` (AnalysisReport, Finding, FindingSeverity), `s1_state_space.rs` through `s8_verdict_uniqueness.rs`
- Depends on: `serde_json`, `serde`, `tenor-core` (for `TENOR_BUNDLE_VERSION` constant only)
- Used by: `tenor-cli`, `tenor-lsp`

**`crates/codegen` (tenor-codegen):**
- Purpose: TypeScript code generation from interchange JSON
- Location: `crates/codegen/src/`
- Contains: `bundle.rs` (interchange deserializer), `typescript.rs` (types.ts generation), `typescript_schemas.rs` (schemas.ts generation), `typescript_client.rs` (client.ts generation), `lib.rs` (index.ts generation + orchestration)
- Depends on: `serde_json`, `serde` only -- does NOT depend on `tenor-core`
- Used by: `tenor-cli`
- Generates 4 files per contract: `types.ts`, `schemas.ts`, `client.ts`, `index.ts` in `{out_dir}/{kebab-bundle-id}/`

**`crates/lsp` (tenor-lsp):**
- Purpose: Language Server Protocol implementation for IDE integration
- Location: `crates/lsp/src/`
- Contains: `server.rs` (main loop + dispatch), `diagnostics.rs` (elaboration-based error diagnostics), `semantic_tokens.rs` (syntax highlighting), `navigation.rs` (go-to-definition, references, document symbols, project index), `hover.rs` (hover information), `completion.rs` (code completion), `agent_capabilities.rs` (contract capability extraction), `document.rs` (open document state tracking)
- Depends on: `tenor-core`, `tenor-analyze`, `lsp-server`, `lsp-types`, `serde_json`
- Used by: `tenor-cli` (via `tenor lsp` subcommand)

## Core Elaboration Pipeline

**Six-Pass Elaborator (`crates/core/src/elaborate.rs`):**

```rust
pub fn elaborate(root_path: &Path) -> Result<Value, ElabError> {
    let (constructs, bundle_id) = pass1_bundle::load_bundle(root_path)?;
    let index = pass2_index::build_index(&constructs)?;
    let type_env = pass3_types::build_type_env(&constructs, &index)?;
    let constructs = pass4_typecheck::resolve_types(constructs, &type_env)?;
    pass4_typecheck::type_check_rules(&constructs)?;
    pass5_validate::validate(&constructs, &index)?;
    pass5_validate::validate_operation_transitions(&constructs, &index)?;
    let bundle = pass6_serialize::serialize(&constructs, &bundle_id);
    Ok(bundle)
}
```

| Pass | Module | Input -> Output | Key Responsibilities |
|------|--------|-----------------|---------------------|
| 0+1 | `pass1_bundle.rs` | Source text -> `(Vec<RawConstruct>, String)` | Lex + parse each file; walk import graph recursively; detect import cycles; sandbox path resolution; cross-file duplicate ID check |
| 2 | `pass2_index.rs` | Constructs -> `Index` | Build per-kind `HashMap<String, Provenance>` for facts, entities, rules, operations, flows, type_decls, personas, systems; map verdict types to producing rules; within-file duplicate detection |
| 3 | `pass3_types.rs` | Constructs + Index -> `TypeEnv` | Resolve `TypeDecl` names to concrete `RawType`; detect type reference cycles; `TypeEnv = HashMap<String, RawType>` |
| 4a | `pass4_typecheck.rs` | Constructs + TypeEnv -> Typed Constructs | Rewrite `TypeRef` nodes to concrete `BaseType` in Fact type fields and Rule expressions |
| 4b | `pass4_typecheck.rs` | Typed Constructs -> () | Expression type-checking: unresolved references, Bool operator restrictions, Money currency matching, quantifier domain List requirement, Mul variable*variable prohibition |
| 5 | `pass5_validate.rs` | Typed Constructs + Index -> () | Entity DAG acyclicity + initial state; Rule stratum ordering + verdict reference strata; Operation persona/entity/effect/outcome validation; Flow step graph + entry + cycle detection; System C-SYS-01 through C-SYS-17; parallel entity conflict detection |
| 6 | `pass6_serialize.rs` | Validated Constructs -> `serde_json::Value` | Canonical interchange JSON; sorted keys; structured numeric values; constructs sorted by kind then ID; rules sorted by stratum then ID; topological flow step ordering |

**Error Model:**
Every elaboration error is an `ElabError` with fields: `pass`, `construct_kind`, `construct_id`, `field`, `file`, `line`, `message`. The pipeline is first-error-wins -- it aborts on the first failure. Factory methods: `ElabError::lex()`, `ElabError::parse()`, `ElabError::new()`.

## Evaluator Architecture

**Entry Points (`crates/eval/src/lib.rs`):**

```rust
// Rules-only evaluation
pub fn evaluate(bundle: &Value, facts: &Value) -> Result<EvalResult, EvalError>

// Full flow evaluation (rules + flow execution)
pub fn evaluate_flow(bundle: &Value, facts: &Value, flow_id: &str, persona: &str)
    -> Result<FlowEvalResult, EvalError>
```

**Evaluation Pipeline:**
1. `Contract::from_interchange(bundle)` -- deserialize interchange JSON into eval-internal typed structs (`crates/eval/src/types.rs`)
2. `assemble::assemble_facts(&contract, facts)` -- validate and coerce fact values against declared types; apply defaults for missing facts with defaults; error on missing required facts
3. `rules::eval_strata(&contract, &fact_set)` -- evaluate rules in stratum order (0, 1, 2, ...); accumulate `VerdictSet` with provenance tracking
4. (Flow mode only) Create immutable `Snapshot { facts, verdicts }`
5. (Flow mode only) `operation::init_entity_states(&contract)` -- initialize all entities to their initial state
6. (Flow mode only) `flow::execute_flow(flow, &contract, &snapshot, &mut entity_states, None)` -- walk the flow step graph, executing operations and branching on predicates

**Runtime Type System (`crates/eval/src/types.rs`):**
- `Value` enum: `Bool`, `Int`, `Decimal`, `Text`, `Date`, `DateTime`, `Money { amount, currency }`, `Duration`, `Enum`, `Record`, `List`, `TaggedUnion`
- `Contract` struct: deserialized from interchange JSON; contains `Vec<FactDecl>`, `Vec<Entity>`, `Vec<Rule>`, `Vec<Operation>`, `Vec<Flow>`, etc.
- `FactSet`: maps fact IDs to typed `Value`s
- `VerdictSet`: ordered list of `VerdictInstance` with provenance
- `Snapshot`: frozen `{ facts: FactSet, verdicts: VerdictSet }` -- immutable during flow execution

**Critical design:** The evaluator's type system is completely separate from `tenor-core`'s `RawType`/`RawExpr`/`RawTerm`. It deserializes from interchange JSON, not from the raw AST. This is the interchange boundary in action.

## Static Analysis Architecture

**Entry Points (`crates/analyze/src/lib.rs`):**

```rust
pub fn analyze(bundle: &Value) -> Result<AnalysisReport, AnalysisError>
pub fn analyze_selected(bundle: &Value, analyses: &[&str]) -> Result<AnalysisReport, AnalysisError>
```

**S1-S8 Analysis Suite:**

| Analysis | Module | Purpose | Dependencies |
|----------|--------|---------|-------------|
| S1 | `s1_state_space.rs` | Enumerate entity state spaces (state count per entity) | None |
| S2 | `s2_reachability.rs` | Detect unreachable (dead) entity states | None |
| S3a | `s3a_admissibility.rs` | Compute admissible operations per entity state | None |
| S4 | `s4_authority.rs` | Map persona authority over transitions; cross-contract authority | S3a |
| S5 | `s5_verdicts.rs` | Enumerate verdict types and operation outcomes | None |
| S6 | `s6_flow_paths.rs` | Enumerate flow execution paths; cross-contract flow paths | S5 |
| S7 | `s7_complexity.rs` | Compute predicate depth and flow depth bounds | S6 |
| S8 | `s8_verdict_uniqueness.rs` | Confirm verdict uniqueness (pre-verified by Pass 5) | None |

**Dependency Resolution:** `analyze_selected()` automatically pulls in required dependencies. If you request S4, S3a is run automatically. If you request S7, both S6 and S5 are run.

**Deserialization:** `AnalysisBundle::from_interchange(bundle)` in `crates/analyze/src/bundle.rs` -- separate from both `tenor-core` AST and `tenor-eval` Contract types. Includes `AnalysisSystem` for System construct analysis (shared personas, shared entities, triggers).

## CLI Command Dispatch

**Architecture (`crates/cli/src/main.rs`):**
- Clap derive-based `Commands` enum with 11 subcommands
- Global flags: `--output [text|json]`, `--quiet`
- Each subcommand dispatches to a `cmd_*` function or a module entry point

| Subcommand | Function/Module | Dependencies |
|------------|----------------|-------------|
| `elaborate` | `cmd_elaborate()` | `tenor-core` |
| `validate` | `cmd_validate()` | `jsonschema` (embedded schemas via `include_str!`) |
| `eval` | `cmd_eval()` | `tenor-eval` |
| `test` | `runner::run_suite()` | `tenor-core` |
| `diff` | `diff::diff_bundles()` + `diff::classify_diff()` | internal `diff` module |
| `check` | `cmd_check()` | `tenor-core` + `tenor-analyze` |
| `explain` | `explain::explain()` | `tenor-core` (optional elaboration) + internal `explain` module |
| `generate typescript` | `cmd_generate()` | `tenor-core` (optional elaboration) + `tenor-codegen` |
| `ambiguity` | `ambiguity::run_ambiguity_suite()` | `tenor-core` + `ureq` (HTTP to Anthropic API) |
| `serve` | `serve::start_server()` | `tenor-core` + `tenor-eval` + `tiny_http` |
| `lsp` | `tenor_lsp::run()` | `tenor-lsp` |

**Embedded Schemas:**
```rust
static INTERCHANGE_SCHEMA_STR: &str = include_str!("../../../docs/interchange-schema.json");
static MANIFEST_SCHEMA_STR: &str = include_str!("../../../docs/manifest-schema.json");
```
Used by `cmd_validate()` for JSON Schema validation. Auto-detects manifest vs bundle by `etag` field presence.

## HTTP API Server Architecture

**Module:** `crates/cli/src/serve.rs`

**Technology:** `tiny_http` -- synchronous, single-threaded, no async runtime. `Arc<Mutex<ServeState>>` for shared state.

**Startup:**
1. Pre-load contracts: for each `.tenor` path, run `tenor_core::elaborate::elaborate()` and store bundle in `HashMap<String, serde_json::Value>`
2. Bind to `0.0.0.0:{port}` (default 8080)
3. Install SIGINT/SIGTERM signal handlers via `libc::signal()` for graceful shutdown
4. Enter request loop with 1-second `recv_timeout` polling for shutdown flag

**Endpoints:**

| Method | Path | Handler | Purpose |
|--------|------|---------|---------|
| GET | `/health` | `handle_health()` | Server status + tenor version |
| GET | `/contracts` | `handle_list_contracts()` | List loaded contracts with construct summaries |
| GET | `/contracts/{id}/operations` | `handle_get_operations()` | Operations for a specific contract |
| POST | `/elaborate` | `handle_elaborate()` | Elaborate `.tenor` source text (writes to temp file) |
| POST | `/evaluate` | `handle_evaluate()` | Evaluate contract against facts; supports flow mode |
| POST | `/explain` | `handle_explain()` | Generate human-readable contract explanation |

**Key Design Choices:**
- `handle_elaborate()` writes source to a `tempfile` then runs `tenor_core::elaborate::elaborate()` because the elaborator expects a file path (for import resolution)
- Lock is dropped before evaluation in `handle_evaluate()` to avoid holding the mutex during potentially slow computation
- `MAX_BODY_SIZE` = 10 MB
- All responses are `application/json`

## LSP Server Architecture

**Module:** `crates/lsp/src/server.rs`

**Technology:** `lsp-server` crate (synchronous, crossbeam channel-based) over stdio. No async runtime.

**Server Capabilities:**
- `textDocumentSync`: Full sync (entire document on every change)
- `semanticTokensProvider`: Full semantic tokens (no delta support)
- `definitionProvider`: Go-to-definition
- `referencesProvider`: Find all references
- `documentSymbolProvider`: Document outline
- `hoverProvider`: Hover information
- `completionProvider`: Code completion (trigger characters: `:`, ` `)
- Custom request: `tenor/agentCapabilities` -- extract agent-usable capabilities from a contract
- Custom notification: `tenor/agentCapabilitiesUpdated` -- sent after save with updated capabilities

**State Management:**
- `DocumentState` (`crates/lsp/src/document.rs`): tracks open documents with path, version, content
- `ProjectIndex` (`crates/lsp/src/navigation.rs`): workspace-wide construct index built by scanning all `.tenor` files; rebuilt on every save

**Lifecycle:**
1. Initialize: extract workspace root from `InitializeParams` (workspace_folders -> root_uri -> root_path fallback chain)
2. Build initial `ProjectIndex` from workspace root
3. On `textDocument/didOpen`: track document, compute + publish diagnostics
4. On `textDocument/didChange`: update tracked content (full sync)
5. On `textDocument/didSave`: re-compute diagnostics, send `tenor/agentCapabilitiesUpdated`, rebuild `ProjectIndex`
6. On `textDocument/didClose`: remove from tracking, clear diagnostics

**Diagnostics (`crates/lsp/src/diagnostics.rs`):** Runs `tenor_core::elaborate::elaborate()` on the saved file. On `ElabError`, converts to LSP `Diagnostic` with file/line provenance. On success, clears diagnostics.

## TypeScript SDK Architecture

**Location:** `sdk/typescript/src/`

**Files:**
- `client.ts`: `TenorClient` class -- HTTP client for `tenor serve`
- `types.ts`: TypeScript type definitions for API request/response shapes
- `errors.ts`: Error hierarchy (`TenorError`, `ConnectionError`, `ContractNotFoundError`, `EvaluationError`, `ElaborationError`)
- `index.ts`: Barrel exports

**Design:**
- Zero runtime dependencies -- uses Node 22+ built-in `fetch`
- `AbortSignal.timeout()` for request timeouts (default 30 seconds)
- Error classification by HTTP status code and path matching:
  - 404 -> `ContractNotFoundError`
  - Error on `/evaluate` path -> `EvaluationError`
  - Error on `/elaborate` path -> `ElaborationError`
  - All others -> `TenorError`

**Client Methods:**

| Method | HTTP | Path | Purpose |
|--------|------|------|---------|
| `health()` | GET | `/health` | Check server reachability |
| `listContracts()` | GET | `/contracts` | List loaded contracts |
| `getOperations(id)` | GET | `/contracts/{id}/operations` | Get operations for a contract |
| `invoke(id, facts, options?)` | POST | `/evaluate` | Evaluate contract (rules or flow) |
| `explain(id)` | POST | `/explain` | Get human-readable explanation |
| `elaborate(source)` | POST | `/elaborate` | Elaborate .tenor source text |

## Code Generation Architecture

**Module:** `crates/codegen/src/lib.rs`

**Entry Point:**
```rust
pub fn generate_typescript(interchange_json: &Value, config: &TypeScriptConfig) -> Result<PathBuf, CodegenError>
```

**Pipeline:**
1. `CodegenBundle::from_interchange(interchange_json)` -- deserialize interchange JSON into codegen-internal types (`crates/codegen/src/bundle.rs`)
2. `typescript::emit_types(&bundle)` -> `types.ts` with TypeScript interfaces for all contract types
3. `typescript_schemas::emit_schemas(&bundle, &sdk_import)` -> `schemas.ts` with runtime validation schemas
4. `typescript_client::emit_client(&bundle, &sdk_import)` -> `client.ts` with typed contract-specific client wrapper
5. `emit_index(&bundle)` -> `index.ts` barrel export

**Output Structure:** `{out_dir}/{kebab-bundle-id}/types.ts|schemas.ts|client.ts|index.ts`

**Isolation:** `tenor-codegen` depends only on `serde_json` and `serde`. It does NOT depend on `tenor-core`, enforcing that code generation works purely from interchange JSON.

## VS Code Extension Architecture

**Location:** `editors/vscode/src/extension.ts`

**Technology:** `vscode-languageclient` (official VS Code LSP client library)

**Lifecycle:**
1. On activation: create `LanguageClient` configured to launch `tenor lsp` as a child process over stdio
2. Register custom notification handler for `tenor/agentCapabilitiesUpdated`
3. Register commands including `tenor.openAgentCapabilities`
4. On deactivation: stop the language client

**Connection:** The extension spawns the Rust `tenor` binary with `lsp` subcommand. All communication is JSON-RPC over stdin/stdout via the LSP protocol.

## Key Abstractions

**`RawConstruct` (`crates/core/src/ast.rs`):**
- Unified enum of all DSL construct variants produced by the parser
- Variants: `Import`, `TypeDecl`, `Fact`, `Entity`, `Rule`, `Operation`, `Persona`, `Flow`, `System`
- All passes consume `&[RawConstruct]` or `Vec<RawConstruct>`

**`ElabError` (`crates/core/src/error.rs`):**
- Structured elaboration error matching expected-error.json conformance format
- Fields: `pass: u8`, `construct_kind`, `construct_id`, `field`, `file`, `line`, `message`
- Constructor helpers: `ElabError::lex()`, `ElabError::parse()`, `ElabError::new()`

**`Index` (`crates/core/src/pass2_index.rs`):**
- Fast construct lookup by kind and ID
- Per-kind `HashMap<String, Provenance>` for facts, entities, rules, operations, flows, type_decls, personas, systems
- `rule_verdicts`: maps verdict types to producing rule IDs
- `verdict_strata`: maps verdict types to their stratum

**`Contract` (`crates/eval/src/types.rs`):**
- Eval-internal deserialized view of an interchange bundle
- Completely distinct from `tenor-core` types -- deserialized from JSON

**`AnalysisBundle` (`crates/analyze/src/bundle.rs`):**
- Analyze-internal deserialized view of an interchange bundle
- Includes `AnalysisSystem` for System construct analysis
- Completely distinct from both `tenor-core` and `tenor-eval` types

**`CodegenBundle` (`crates/codegen/src/bundle.rs`):**
- Codegen-internal deserialized view of an interchange bundle
- Completely distinct from all other crates' types

**Interchange JSON (TenorInterchange):**
- Schema: `docs/interchange-schema.json`; manifest schema: `docs/manifest-schema.json`
- Both embedded in `tenor-cli` binary via `include_str!`
- All JSON keys sorted lexicographically within each object (enforced in Pass 6)
- Bundle kind field values use `PascalCase` (`"Fact"`, `"Rule"`, `"Entity"`, etc.)

## Data Flow

**DSL Compilation Flow:**

```
.tenor source files
  -> lexer::lex() -> tokens
  -> parser::parse() -> Vec<RawConstruct>
  -> pass1_bundle::load_bundle() -> bundled constructs + bundle_id
  -> pass2_index::build_index() -> Index
  -> pass3_types::build_type_env() -> TypeEnv
  -> pass4_typecheck::resolve_types() -> typed constructs
  -> pass4_typecheck::type_check_rules() -> (validation only)
  -> pass5_validate::validate() -> (validation only)
  -> pass6_serialize::serialize() -> serde_json::Value (interchange JSON)
```

**Evaluation Flow:**

```
interchange JSON + facts JSON
  -> Contract::from_interchange() -> Contract
  -> assemble::assemble_facts() -> FactSet
  -> rules::eval_strata() -> VerdictSet
  -> (flow mode) Snapshot { facts, verdicts } (frozen)
  -> (flow mode) operation::init_entity_states() -> EntityStateMap
  -> (flow mode) flow::execute_flow() -> FlowResult
```

**Analysis Flow:**

```
interchange JSON
  -> AnalysisBundle::from_interchange() -> AnalysisBundle
  -> S1-S8 in dependency order -> individual results
  -> AnalysisReport::extract_findings() -> aggregated findings
```

**State Management:**
- No global mutable state. All pipeline state is passed explicitly as function arguments.
- `RawConstruct` is cloned through passes (derived `Clone`). Pass 4 consumes and returns the construct list.
- Flow execution: `Snapshot` (facts + verdicts) is immutable. `EntityStateMap` is separately mutable.
- HTTP server: `Arc<Mutex<ServeState>>` -- lock acquired per request, released before computation.
- LSP server: `DocumentState` and `ProjectIndex` are mutable, owned by the main loop.

## Error Handling

**Strategy:** Result-returning functions throughout; `process::exit(1)` only at the CLI boundary.

**Patterns:**
- Elaboration: `Result<T, ElabError>` -- first-error-wins; pipeline aborts on first failure
- Evaluation: `Result<T, EvalError>` -- enum of typed errors (MissingFact, TypeMismatch, DeserializeError, OperationError, FlowError, Overflow, etc.)
- Analysis: `Result<T, AnalysisError>` -- InvalidBundle or MissingField
- Codegen: `Result<T, CodegenError>` -- InvalidBundle or IoError
- CLI: errors formatted as text or JSON depending on `--output` flag; stderr for errors, stdout for results
- HTTP API: errors returned as JSON `{"error": "..."}` with appropriate HTTP status codes (400, 404, 500)
- TypeScript SDK: typed error hierarchy with `TenorError` base class; error classification by HTTP status code

## Cross-Cutting Concerns

**Serialization:** `serde_json` with `BTreeMap` used in AST and serialization for deterministic key ordering. Pass 6 sorts all construct arrays and JSON object keys explicitly.

**Numeric Precision:** `rust_decimal` crate for all Decimal/Money types; literals preserved as strings through the pipeline until evaluated.

**Import Resolution:** Multi-file bundles assembled via recursive file walking in `pass1_bundle.rs`. Import cycle detection uses a DFS stack. Paths resolved relative to importing file's directory. Sandbox boundary: all resolved paths must stay within the root file's directory tree.

**Conformance Testing:** TAP v14 protocol output via `crates/cli/src/tap.rs`. Suite runner in `crates/cli/src/runner.rs` discovers fixture pairs by convention (`.tenor` + `.expected.json` or `.expected-error.json`).

**AI Ambiguity Testing:** `crates/cli/src/ambiguity/` module calls Anthropic Claude API with the spec + DSL sample and checks whether the model correctly elaborates. Invoked via `tenor ambiguity` subcommand.

**Provenance:** Every interchange construct carries `provenance: { file, line }`. Verdicts carry `VerdictProvenance` (rule_id, stratum, facts_used, verdicts_used). Flow execution records `StepRecord` per step executed.

---

*Architecture analysis: 2026-02-23*
