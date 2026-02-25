# Architecture

**Analysis Date:** 2026-02-25

## Pattern Overview

**Overall:** Multi-phase Tenor contract language compiler and runtime

**Key Characteristics:**
- **Elaboration pipeline**: 6-pass compiler transforms `.tenor` DSL source files to JSON interchange format
- **Modular phases**: Each pass is a separate module with clear input/output contracts
- **Provider abstraction**: Filesystem I/O is abstracted to support WASM compilation and in-memory testing
- **Interchange-based**: All downstream tools consume JSON interchange format, not raw AST
- **Stratified evaluation**: Contract rules are executed in dependency-ordered strata for deterministic results

## Layers

**DSL Input Layer:**
- Purpose: Accept `.tenor` source files and parse them into raw AST
- Location: `crates/core/src/lexer.rs`, `crates/core/src/parser.rs`
- Contains: Tokenizer, recursive descent parser, raw AST type definitions
- Depends on: Text input via SourceProvider
- Used by: Pass 1 (bundle assembly)

**Elaboration Core (Passes 0-6):**
- Purpose: Transform raw AST through type checking, validation, and serialization
- Location: `crates/core/src/` (pass0+1 through pass6 modules)
- Contains: Import resolution, construct indexing, type environment building, expression type-checking, structural validation, JSON serialization
- Depends on: Raw AST from parser, SourceProvider for import resolution
- Used by: CLI elaborate command, external elaboration requests

**Interchange Format:**
- Purpose: Language-agnostic JSON schema for contract representation
- Location: `crates/interchange/src/types.rs`, schema in `docs/TENOR.md`
- Contains: Typed struct definitions for all construct kinds (Fact, Entity, Rule, Operation, Flow, Persona, System, TypeDecl)
- Depends on: serde for JSON serialization
- Used by: Evaluator, analyzer, code generator, LSP

**Evaluation Engine (Phase 3):**
- Purpose: Execute contracts against facts and produce verdicts with provenance
- Location: `crates/eval/src/`
- Contains: Fact assembly, rule evaluation, verdict computation, flow execution, entity state management
- Depends on: Interchange bundles, external fact inputs
- Used by: CLI eval command, application integrations

**Static Analysis (Phase 4):**
- Purpose: Analyze contract properties (S1-S8 analyses) for security and correctness
- Location: `crates/analyze/src/` (s1_state_space.rs through s8_verdict_uniqueness.rs)
- Contains: Modular analyses with dependency ordering (S4 depends on S3a, S6 depends on S5, S7 depends on S6)
- Depends on: Interchange bundles
- Used by: CLI check command

**Code Generation (Phase 6):**
- Purpose: Generate TypeScript types, schemas, and client bindings from contracts
- Location: `crates/codegen/src/` (typescript.rs, typescript_schemas.rs, typescript_client.rs)
- Contains: AST-to-TypeScript emitters, kebab/pascal case converters, barrel file generation
- Depends on: Interchange bundles
- Used by: SDK generation workflows

**Command-Line Interface:**
- Purpose: User-facing toolchain commands
- Location: `crates/cli/src/main.rs` and subcommand modules
- Contains: Clap-based command dispatch, runners for elaborate/eval/validate/test/check/diff/explain/serve/manifest/agent
- Depends on: All other crates
- Used by: Direct terminal invocations, CI pipelines

**Language Server Protocol (Phase 8):**
- Purpose: Real-time IDE support for Tenor editing
- Location: `crates/lsp/src/`
- Contains: Diagnostics generation, hover information, code completion (placeholder)
- Depends on: Elaboration pipeline, interchange format
- Used by: Editor integrations (VSCode, etc.)

**Storage Abstraction:**
- Purpose: Provide trait-based contract for persisting execution state
- Location: `crates/storage/src/`
- Contains: Record types for entity states, transitions, flow execution, operation execution, provenance
- Depends on: None (trait definitions only)
- Used by: External implementations (databases, caches)

**WASM Bridge:**
- Purpose: Expose elaboration and evaluation to JavaScript runtimes
- Location: `crates/tenor-eval-wasm/` (excluded from main workspace)
- Contains: wasm-bindgen FFI, InMemoryProvider integration
- Depends on: tenor-core, tenor-eval with WASM target
- Used by: Browser/Node.js environments

## Data Flow

**Elaboration Flow:**

1. User provides `.tenor` file path
2. CLI calls `elaborate(root_path)` from tenor-core
3. Pass 0+1: SourceProvider reads root file and transitive imports → flat construct list
   - Lexer tokenizes each file
   - Parser produces raw AST
   - Import resolution follows `import "path"` statements recursively
   - Cycle detection prevents circular dependencies
   - Duplicate ID check across files
4. Pass 2: `build_index()` creates construct lookup maps by (kind, id)
5. Pass 3: `build_type_env()` resolves named type references, builds type environment
6. Pass 4: `resolve_types()` replaces TypeRef nodes with concrete BaseType
   - `type_check_rules()` validates expression types, applies numeric promotions
7. Pass 5: `validate()` checks entity/operation/rule/flow structural constraints
8. Pass 6: `serialize()` produces canonical JSON with sorted keys
9. Result: `serde_json::Value` containing interchange bundle

**Evaluation Flow:**

1. User provides interchange JSON bundle and facts JSON object
2. CLI calls `evaluate(bundle, facts)` or `evaluate_flow(bundle, facts, flow_id, persona, override_entity_states)`
3. Deserialization: `Contract::from_interchange()` parses JSON into typed Contract struct
4. Fact assembly: `assemble_facts()` merges user facts with fact defaults
5. Rule evaluation: `eval_strata()` executes rules in stratum order, produces verdicts with provenance
6. (Flow only) Snapshot creation: combines fact set and verdict set
7. (Flow only) Entity state initialization: from contract defaults or override
8. (Flow only) Flow execution: steps run in sequence, each step applies operations with authorization
9. Result: `EvalResult` or `FlowEvalResult` with verdict provenance and operation effects

**Analysis Flow:**

1. User provides interchange JSON bundle
2. CLI calls `analyze(bundle)` or `analyze_selected(bundle, analyses)`
3. Deserialization: `AnalysisBundle::from_interchange()` parses JSON
4. Sequential analysis execution with dependency resolution:
   - S1: State space (independent)
   - S2: Reachability (independent)
   - S3a: Admissibility (independent)
   - S4: Authority (depends on S3a)
   - S5: Verdict space (independent)
   - S6: Flow paths (depends on S5)
   - S7: Complexity (depends on S6)
   - S8: Verdict uniqueness (independent)
5. Result aggregation: All results extracted into `AnalysisReport` with findings

**State Management:**

- **Elaboration**: Immutable, single-pass per module. No global state.
- **Evaluation**: Frozen snapshot pattern (section 14.2 of spec). FactSet and VerdictSet are computed once, then passed to flow execution unchanged.
- **Analysis**: Each analysis reads the same AnalysisBundle independently, no mutation.
- **Entity states**: Mutable during flow execution. Operations produce EntityStateMap diffs that are accumulated as flow progresses.

## Key Abstractions

**SourceProvider:**
- Purpose: Decouple file I/O from elaborator logic
- Examples: `FileSystemProvider` (delegates to std::fs), `InMemoryProvider` (HashMap-backed)
- Pattern: Trait with three methods: read_source, resolve_import, canonicalize
- Location: `crates/core/src/source.rs`

**RawConstruct:**
- Purpose: Represent any top-level Tenor construct (Fact, Entity, Rule, Operation, Flow, TypeDecl, Persona, System, Import) with provenance
- Examples: `RawConstruct::Fact`, `RawConstruct::Rule`, `RawConstruct::Import`
- Pattern: Enum variant per construct kind, all carrying file/line provenance
- Location: `crates/core/src/ast.rs`

**ElabError:**
- Purpose: Carry structured error information matching conformance test expected-error.json format
- Pattern: Single error type with pass number, construct_kind/id/field, file, line, message
- Location: `crates/core/src/error.rs`

**Index:**
- Purpose: Fast lookup of constructs by (kind, id) after parsing
- Pattern: HashMap per kind + special maps for rule verdicts and verdict strata
- Location: `crates/core/src/pass2_index.rs`

**TypeEnv:**
- Purpose: Named type resolution context for Pass 3/4
- Pattern: Maps type name to BaseType definition
- Location: `crates/core/src/pass3_types.rs`

**Contract:**
- Purpose: Evaluate-time representation of a contract (deserialized from interchange JSON)
- Pattern: Typed struct containing all construct instances and metadata
- Location: `crates/eval/src/types.rs`

**FactSet:**
- Purpose: Immutable collection of fact values computed from user inputs
- Pattern: HashMap<String, Value> plus type information for validation
- Location: `crates/eval/src/assemble.rs`, `crates/eval/src/types.rs`

**VerdictSet:**
- Purpose: Immutable collection of verdicts produced by rule evaluation
- Pattern: HashMap<String, VerdictInstance> with provenance traces
- Location: `crates/eval/src/rules.rs`, `crates/eval/src/types.rs`

## Entry Points

**CLI Main:**
- Location: `crates/cli/src/main.rs`
- Triggers: `tenor elaborate`, `tenor eval`, `tenor validate`, `tenor test`, `tenor check`, `tenor diff`, `tenor explain`, `tenor serve`, `tenor manifest`, `tenor agent`
- Responsibilities: Parse arguments, dispatch to subcommand handlers, format output (text or JSON)

**Elaboration Entry:**
- Location: `crates/core/src/elaborate.rs` functions `elaborate()` and `elaborate_with_provider()`
- Triggers: Direct library calls, CLI elaborate command
- Responsibilities: Orchestrate 6-pass pipeline, return interchange JSON or error

**Evaluation Entry:**
- Location: `crates/eval/src/lib.rs` functions `evaluate()` and `evaluate_flow()`
- Triggers: Direct library calls, CLI eval command
- Responsibilities: Execute contract rules/flow against facts, produce verdicts/effects

**Analysis Entry:**
- Location: `crates/analyze/src/lib.rs` functions `analyze()` and `analyze_selected()`
- Triggers: Direct library calls, CLI check command
- Responsibilities: Run analyses in dependency order, aggregate findings

**Code Generation Entry:**
- Location: `crates/codegen/src/lib.rs` function `generate_typescript()`
- Triggers: Direct library calls, CLI generate command
- Responsibilities: Parse interchange bundle, emit TypeScript files

## Error Handling

**Strategy:** All major operations return `Result<T, E>` where E is a structured error type.

**Patterns:**

1. **Elaboration errors** (`ElabError`):
   - Created with pass number, construct context, file/line, and message
   - Serialized to JSON matching conformance test format
   - First error encountered stops the pipeline (fail-fast)
   - Example: `ElabError::new(4, Some("Rule"), Some("verify_amount"), Some("body"), "contract.tenor", 15, "type mismatch: expected Int, got Text")`

2. **Evaluation errors** (`EvalError`):
   - Covers fact validation, type mismatch, missing verdicts, flow errors
   - Carries context (fact ID, rule ID, step ID) for debugging
   - Example: `EvalError::FactMissing { fact_id: "user_score" }`

3. **Analysis errors** (`AnalysisError`):
   - Used when interchange deserialization fails or analysis logic detects inconsistency
   - Findings are reportable (non-fatal) issues, not errors

4. **IO errors** (`std::io::Error`):
   - Wrapped by SourceProvider when file operations fail
   - Elevated to ElabError with pass 1 context

## Cross-Cutting Concerns

**Logging:** None formal; use of `eprintln!()` in CLI for user-facing messages, debug output for diagnostics (future: structured logging via tracing crate)

**Validation:**
- Pass 4 validates all expression types (Compare, Mul, ForAll, Exists, verdict_present references)
- Pass 5 validates structural constraints (Operation transitions, Rule body referenceability, Flow step sequencing)
- Pass 6 (serialization) encodes all values in their canonical form

**Authentication/Authorization:**
- Not a concern of elaboration, evaluation, or analysis
- Flow execution respects Operation persona constraints (evaluated at runtime)
- Stored separately from contract (application integration point)

**Provenance Tracking:**
- Every construct carries file/line information (Provenance)
- Every verdict instance carries trace chain showing which facts/rules/steps produced it
- Flow step execution records operation effects with provenance
- Example: `VerdictInstance { verdict_type: "approved", facts: [...], rules: [...], provenance: [...] }`

---

*Architecture analysis: 2026-02-25*
