# Architecture Patterns

**Domain:** DSL toolchain (elaboration, evaluation, static analysis, code generation, editor integration)
**Researched:** 2026-02-21
**Confidence:** HIGH (based on direct codebase analysis, Rust compiler dev guide, rust-analyzer architecture, and Oso/Polar extension patterns)

## Recommended Architecture

### The Problem

The current elaborator is a single Rust binary (`tenor-elaborator`) with a monolithic `elaborate.rs` (2066 lines, 58 functions) that runs all six passes sequentially and returns `serde_json::Value`. Every new consumer (evaluator, static analyzer, code generator, language server) needs access to different stages of the pipeline output. The current architecture exports only the final JSON -- there is no way to get the typed AST, the construct index, or the type environment without re-running elaboration from scratch.

### The Solution: Cargo Workspace with Shared Core Library

Convert from a single `elaborator/` crate to a Cargo workspace with a core library crate that exposes the pipeline's intermediate representations. Every consumer (CLI, evaluator, analyzer, codegen, LSP) depends on the core library and calls into specific pipeline stages.

```
tenor/
  Cargo.toml              (workspace manifest)
  crates/
    tenor-core/           (library: lexer, parser, elaborator, AST types, Index, TypeEnv)
    tenor-cli/            (binary: unified CLI dispatching to core + evaluator + analyzer)
    tenor-eval/           (library: evaluator -- bundle + facts -> verdicts + provenance)
    tenor-analyze/        (library: static analyzer -- S1-S7 derivations)
    tenor-codegen/        (library: code generator -- interchange -> TypeScript/Rust)
    tenor-lsp/            (binary: language server protocol implementation)
  conformance/            (unchanged -- elaborator conformance suite)
  conformance-eval/       (new -- evaluator conformance suite)
  docs/                   (unchanged)
```

### Why This Structure

**Single-crate library with pass-level API, not one crate per pass.** The six elaboration passes share a single AST type (`RawConstruct`) mutated through passes. Splitting each pass into its own crate would force defining intermediate types at every crate boundary -- painful, brittle, and unnecessary. The passes are coupled by design (the spec says so: Pass 4 reads Pass 3's TypeEnv; Pass 5 reads Pass 2's Index). The right boundary is between the elaboration pipeline (one crate) and its consumers (separate crates).

**Consumers as separate crates because they have distinct dependency profiles.** The evaluator needs decimal arithmetic. The code generator needs template engines or string builders. The language server needs tower-lsp or async I/O. None of these belong in the core elaboration library.

**Binary crates are thin dispatchers.** `tenor-cli` and `tenor-lsp` are just argument parsing and protocol handling. All logic lives in library crates.

---

## Component Boundaries

| Component | Responsibility | Depends On | Communicates With |
|-----------|---------------|------------|-------------------|
| **tenor-core** | Lex, parse, elaborate (6 passes), produce typed AST + Index + TypeEnv + JSON interchange | serde, serde_json | Everything depends on this |
| **tenor-eval** | Evaluate a bundle against a FactSet, produce VerdictSet + provenance chain | tenor-core (for AST types), decimal arithmetic crate | tenor-cli (invoked as library) |
| **tenor-analyze** | Derive S1-S7 static analysis reports from elaborated bundle | tenor-core (for AST types, Index) | tenor-cli, tenor-lsp |
| **tenor-codegen** | Generate TypeScript/Rust code from interchange JSON | tenor-core (for AST types or JSON Value) | tenor-cli (invoked as library) |
| **tenor-cli** | Unified `tenor` binary with subcommands | tenor-core, tenor-eval, tenor-analyze, tenor-codegen, clap | User (via CLI) |
| **tenor-lsp** | Language server: diagnostics, go-to-definition, hover | tenor-core, tenor-analyze, tower-lsp or lsp-server | VS Code extension (via stdio) |

---

## Data Flow

### Elaboration Pipeline (tenor-core)

```
Source text (.tenor files)
    |
    v
[Pass 0] Lexer + Parser
    |  produces: Vec<RawConstruct> with Provenance (file, line)
    v
[Pass 1] Bundle Assembly
    |  produces: (Vec<RawConstruct>, bundle_id) -- imports resolved, flattened
    v
[Pass 2] Construct Indexing
    |  produces: Index (facts, entities, rules, operations, flows, type_decls,
    |            rule_verdicts, verdict_strata)
    v
[Pass 3] Type Environment
    |  produces: TypeEnv (HashMap<String, RawType>) -- named types resolved
    v
[Pass 4] Type Resolution + Checking
    |  produces: Vec<RawConstruct> with TypeRef nodes resolved to concrete BaseTypes
    |            + all expressions type-checked
    v
[Pass 5] Validation
    |  produces: () (constructs validated or ElabError returned)
    v
[Pass 6] Serialization
    |  produces: serde_json::Value (interchange JSON bundle)
    v
JSON output
```

### Key Insight: Different Consumers Need Different Pipeline Stages

| Consumer | Needs | Does NOT Need |
|----------|-------|---------------|
| `tenor elaborate` | Full pipeline through Pass 6 (JSON output) | -- |
| `tenor validate` | JSON schema validation of existing interchange | Elaboration at all |
| `tenor check` | Passes 0-5 (typed AST + Index), then S1-S7 analysis | Pass 6 serialization |
| `tenor eval` | Deserialized interchange bundle (or typed AST) + facts | Raw source parsing |
| `tenor explain` | Typed AST + Index (for human-readable summary) | JSON serialization |
| `tenor generate` | Interchange JSON or typed AST | -- |
| Language server | Passes 0-4 (per-keystroke), Pass 5 on save, S1-S7 on demand | JSON serialization |

This means **tenor-core must expose intermediate results, not just the final JSON**. The public API should look approximately like:

```rust
// tenor-core public API (sketch)
pub fn parse(path: &Path) -> Result<Vec<RawConstruct>, ElabError>;
pub fn load_bundle(root: &Path) -> Result<(Vec<RawConstruct>, String), ElabError>;
pub fn build_index(constructs: &[RawConstruct]) -> Result<Index, ElabError>;
pub fn build_type_env(constructs: &[RawConstruct], index: &Index) -> Result<TypeEnv, ElabError>;
pub fn resolve_and_check(constructs: Vec<RawConstruct>, type_env: &TypeEnv) -> Result<Vec<RawConstruct>, ElabError>;
pub fn validate(constructs: &[RawConstruct], index: &Index) -> Result<(), ElabError>;
pub fn serialize(constructs: &[RawConstruct], bundle_id: &str) -> Value;

// Convenience: full pipeline
pub fn elaborate(root: &Path) -> Result<Value, ElabError>;
```

### Evaluator Data Flow

```
Interchange JSON (or typed AST)
    +
Facts JSON (external input)
    |
    v
[FactSet Assembly]
    |  validates types against declared Fact types
    |  applies defaults for missing facts
    v
[Stratum Evaluation]
    |  evaluates rules in stratum order (0, 1, 2, ...)
    |  each stratum sees verdicts from all lower strata
    v
ResolvedVerdictSet + VerdictProvenance
    |
    v
[Operation Execution] (per invocation)
    |  checks precondition against verdicts
    |  validates persona authorization
    |  applies entity state transitions atomically
    v
EntityState' + OperationProvenance
    |
    v
[Flow Orchestration]
    |  walks step graph, invokes operations, follows outcomes
    |  handles failures via declared handlers
    v
FlowOutcome + complete provenance chain
```

### Static Analyzer Data Flow

```
Typed AST + Index + TypeEnv (from tenor-core, passes 0-5)
    |
    v
[S1] Entity complete state space  ---> Report
[S2] Reachable states             ---> Report
[S3a] Structural admissibility    ---> Report
[S4] Authority topology           ---> Report
[S5] Verdict space                ---> Report
[S6] Flow path enumeration        ---> Report
[S7] Complexity bounds            ---> Report
    |
    v
AnalysisReport (structured, serializable)
```

S1, S2, S4, S5, and S7 are straightforward graph traversals over the Index. S3a is type-level satisfiability checking (O(|expression tree|) per precondition). S6 is flow path enumeration (DAG traversal with branching). S3b (domain satisfiability) is qualified -- computationally infeasible for large domains; implement S3a first and defer S3b.

### Code Generator Data Flow

```
Interchange JSON
    |
    v
[Parse interchange] -> typed in-memory model
    |
    v
[Generate per construct kind]
    |
    +-- Entity -> TypeScript/Rust entity store (state machine)
    +-- Rule   -> TypeScript/Rust rule engine (stratum evaluation)
    +-- Operation -> TypeScript/Rust operation handlers (precondition + effects)
    +-- Flow   -> TypeScript/Rust flow orchestrator (step graph walker)
    +-- Fact   -> TypeScript/Rust port interfaces (fact source contracts)
    |
    v
[Generate port interfaces] -> adapter contracts
    |
    v
[Generate wiring] -> composition root / DI container
```

The code generator reads interchange JSON, not the raw AST. This is important: it means the code generator works with any conforming elaborator's output, not just the Rust reference implementation. Interchange JSON is the contract boundary.

### Language Server Data Flow

```
File change event (from editor)
    |
    v
[Incremental re-lex + re-parse] (Pass 0)
    |
    v
[Re-elaborate affected constructs] (Passes 1-5)
    |
    v
[Publish diagnostics] -> editor (inline errors)
    |
    +-- [On hover] -> type info from TypeEnv
    +-- [Go to definition] -> provenance file/line from Index
    +-- [On save] -> full S1-S7 analysis via tenor-analyze
```

The language server does NOT need incremental compilation in the salsa/rust-analyzer sense. Tenor contracts are small (the largest example is 7.9 KB). Full re-elaboration on every keystroke is feasible at <10ms. Incremental analysis is over-engineering for this domain. If it ever becomes necessary, introduce it later -- the per-pass API boundary supports it.

---

## Patterns to Follow

### Pattern 1: Library-First, Binary-Last

**What:** All logic lives in library crates (`tenor-core`, `tenor-eval`, `tenor-analyze`, `tenor-codegen`). Binary crates (`tenor-cli`, `tenor-lsp`) are thin wrappers.

**When:** Always. This is not optional.

**Why:** Enables testing without process boundaries. The conformance suite runner, unit tests, and integration tests all call library functions directly. Binary crates are just argument parsing and I/O.

**Example:**
```rust
// tenor-cli/src/main.rs
fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::Elaborate { file } => {
            match tenor_core::elaborate(&file) {
                Ok(json) => println!("{}", serde_json::to_string_pretty(&json).unwrap()),
                Err(e) => { eprintln!("{}", e.to_json()); std::process::exit(1); }
            }
        }
        Command::Check { file } => {
            let bundle = tenor_core::elaborate_to_typed(&file)?;
            let report = tenor_analyze::check(&bundle);
            println!("{}", report.to_json());
        }
        Command::Eval { bundle, facts } => {
            let result = tenor_eval::evaluate(&bundle, &facts)?;
            println!("{}", result.to_json());
        }
        // ...
    }
}
```

### Pattern 2: Fail-Fast with Structured Errors

**What:** Continue the existing `Result<T, ElabError>` pattern. Every error carries pass, construct_kind, construct_id, field, file, line, message. First error stops processing.

**When:** All pipeline stages. The evaluator gets its own error type (`EvalError`) with analogous structure.

**Why:** The spec mandates structured error reporting (section 12.3). Fail-fast is simpler and the spec doesn't require error recovery or multi-error reporting.

### Pattern 3: Interchange JSON as Integration Boundary

**What:** The code generator and evaluator consume interchange JSON, not raw AST types. They can optionally accept typed AST for efficiency, but interchange JSON is the normative interface.

**When:** Any consumer that doesn't need to be in the same Rust process as the elaborator.

**Why:** The spec defines interchange JSON as the trust boundary (section 12.1). A code generator that only works with the Rust elaborator's internal types is not portable. Code generators in other languages must work from JSON.

**Exception:** The static analyzer and language server operate on the typed AST directly because they need Index and TypeEnv, which are not in the interchange JSON.

### Pattern 4: Conformance Suites per Component

**What:** Each major component gets its own conformance suite with the same fixture conventions (positive test = input + expected output; negative test = input + expected error).

**When:** Elaborator (existing, 47 tests), evaluator (new), static analyzer (new).

**Why:** Conformance testing is the project's quality model. Extending it to new components maintains the standard.

```
conformance/           (existing -- elaborator)
conformance-eval/      (new -- evaluator)
  positive/            (.tenor + .facts.json + .expected-verdict.json)
  negative/            (.tenor + .facts.json + .expected-error.json)
conformance-analyze/   (new -- static analyzer)
  positive/            (.tenor + .expected-analysis.json)
```

---

## Anti-Patterns to Avoid

### Anti-Pattern 1: One Crate Per Pass

**What:** Splitting elaborate.rs into `tenor-pass0`, `tenor-pass1`, ... `tenor-pass6` as separate crates.

**Why bad:** The passes share `RawConstruct`, `RawType`, `RawExpr`, `Index`, `TypeEnv` -- all defined in the parser and used through all passes. Separate crates would force these into yet another "types" crate, creating a dependency DAG with no benefit. Passes are already sequential with clear boundaries inside the file; they just need to be separate modules within `tenor-core`, not separate crates.

**Instead:** Split `elaborate.rs` into modules within `tenor-core`:

```
tenor-core/src/
  lib.rs          (public API surface)
  lexer.rs        (existing)
  parser.rs       (existing -- AST types)
  error.rs        (existing)
  elaborate/
    mod.rs        (orchestration: calls passes in order)
    bundle.rs     (Pass 1: import resolution)
    index.rs      (Pass 2: construct indexing)
    type_env.rs   (Pass 3: type environment)
    type_check.rs (Pass 4: type resolution + checking)
    validate.rs   (Pass 5: construct validation)
    serialize.rs  (Pass 6: interchange output)
```

### Anti-Pattern 2: WASM-First Language Server

**What:** Compiling the language server to WebAssembly and shipping it inside the VS Code extension.

**Why bad for Tenor:** Oso chose WASM because their DSL validation runs in ~1ms and they wanted to avoid multi-platform binaries. Tenor's elaboration is also fast, but the language server will eventually need file system access (import resolution across files), process lifecycle management (running `tenor check` on save), and access to the full type environment. WASM restricts all of these. The distribution complexity argument also doesn't apply here -- Tenor already ships a native Rust binary.

**Instead:** Ship the language server as a native binary (`tenor-lsp`). The VS Code extension is a thin TypeScript client that spawns `tenor-lsp` via stdio. This is the standard pattern used by rust-analyzer, gopls, clangd, and every other serious language server.

### Anti-Pattern 3: Incremental Compilation Infrastructure

**What:** Introducing salsa, demand-driven compilation, or incremental analysis for the elaborator.

**Why bad for Tenor:** Tenor contracts are small. The spec's design constraints (C1-C7) guarantee finite, bounded evaluation. Full re-elaboration of a multi-file contract takes single-digit milliseconds. The engineering cost of incremental compilation (query system, cache invalidation, dependency tracking) is enormous and provides no user-visible benefit for a language whose contracts are measured in hundreds of lines, not millions.

**Instead:** Re-elaborate on every change. If profiling later shows this is a bottleneck (it will not), introduce caching at the file level, not the expression level.

### Anti-Pattern 4: Code Generator Reading Raw AST

**What:** Having `tenor-codegen` depend on `tenor-core` AST types and reading the typed AST directly.

**Why bad:** Couples the code generator to the Rust implementation's internal representation. The roadmap calls for TypeScript + Rust code generation targets. A code generator in any language must work from interchange JSON.

**Instead:** The code generator reads interchange JSON. If performance matters (it does not -- code generation runs once, not per keystroke), provide an optional in-process path that accepts `serde_json::Value` from the elaborator without serialization/deserialization roundtrip.

---

## Suggested Build Order (Dependencies Between Components)

This is the critical section for the roadmap.

### Phase A: Extract tenor-core (prerequisite for everything)

1. Create workspace Cargo.toml
2. Move elaborator source into `crates/tenor-core/`
3. Split `elaborate.rs` into modules (bundle, index, type_env, type_check, validate, serialize)
4. Make pass functions `pub` with clear input/output signatures
5. Make `Index`, `TypeEnv`, `RawConstruct`, `RawType`, `RawExpr` all `pub`
6. Verify: existing conformance suite still passes (47/47)

**No new functionality. Pure refactor. Must happen first.**

### Phase B: tenor-cli (depends on Phase A)

1. Create `crates/tenor-cli/` with clap-based argument parsing
2. Implement `tenor elaborate` (delegates to `tenor_core::elaborate()`)
3. Implement `tenor validate` (JSON schema validation of interchange)
4. Implement `tenor test` (delegates to conformance runner)
5. Retire the old `tenor-elaborator` binary or keep it as an alias

### Phase C: tenor-eval (depends on Phase A)

1. Create `crates/tenor-eval/`
2. Implement FactSet assembly (type validation, defaults)
3. Implement stratum-ordered rule evaluation
4. Implement verdict resolution
5. Wire into `tenor-cli` as `tenor eval`
6. Create `conformance-eval/` test suite
7. Implement operation execution (precondition check, effect application)
8. Implement flow orchestration (step graph walker)
9. Implement provenance chain collection

### Phase D: tenor-analyze (depends on Phase A)

1. Create `crates/tenor-analyze/`
2. Implement S1 (complete state space -- trivial: enumerate Entity.states)
3. Implement S2 (reachable states -- DFS from initial over transitions)
4. Implement S5 (verdict space -- enumerate rule verdict_types)
5. Implement S7 (complexity bounds -- expression tree size + flow depth)
6. Implement S4 (authority topology -- persona x entity state x operation matrix)
7. Implement S3a (structural admissibility -- type-level satisfiability)
8. Implement S6 (flow path enumeration -- DAG path enumeration)
9. Wire into `tenor-cli` as `tenor check`

### Phase E: tenor-codegen (depends on Phase A, best after Phase C)

1. Create `crates/tenor-codegen/`
2. Define port interfaces (fact source, persona resolver, state store, provenance repo)
3. Implement TypeScript target: entity store, rule engine, operation handlers, flow orchestrator
4. Implement `@tenor/adapters-local` (in-memory adapters for dev/test)
5. Wire into `tenor-cli` as `tenor generate`
6. (Later) Implement Rust target

### Phase F: tenor-lsp (depends on Phase A + D)

1. Create `crates/tenor-lsp/`
2. Implement TextMate grammar for syntax highlighting (this is a .tmLanguage.json file, not Rust)
3. Implement LSP server with diagnostics (re-elaborate on change, publish errors)
4. Implement go-to-definition (use Index provenance)
5. Implement hover (type info from TypeEnv)
6. Implement check-on-save (invoke tenor-analyze)
7. Create VS Code extension package (TypeScript client that spawns tenor-lsp)

### Dependency Graph

```
                     tenor-core
                    /    |    \    \
                   /     |     \    \
            tenor-cli  tenor-eval  tenor-analyze  tenor-codegen
                |        |           |
                +--------+-----------+
                |
             tenor-lsp
                |
          VS Code extension (TypeScript)
```

**Phase A must come first.** After Phase A, Phases B/C/D can proceed in parallel. Phase E benefits from Phase C (evaluator tests validate code generator output). Phase F depends on A and D (needs core + analyzer).

---

## Scalability Considerations

| Concern | Current (1-5 contracts) | At 50 contracts | At 500 contracts |
|---------|------------------------|-----------------|------------------|
| Elaboration time | <10ms per contract | <10ms per contract | <10ms per contract -- contracts are bounded by design |
| Language server responsiveness | Full re-elaborate on keystroke (<10ms) | Same | Same |
| Code generation time | <1s per contract | <1s per contract | Batch generation; parallelize across contracts |
| Conformance suite runtime | <1s (47 tests) | <5s (200+ tests) | <30s -- still manageable as unit test suite |
| Workspace compile time | <10s (small crate count) | Same (crate structure is fixed) | Same |

Scalability is not a concern for Tenor. The language's design constraints (C5: finite evaluation, C6: no recursion) guarantee that individual contract processing is bounded. The toolchain's scale bottleneck is the number of contracts in a workspace, not the size of individual contracts, and that grows linearly.

---

## Sources

- Direct codebase analysis: `elaborator/src/elaborate.rs`, `parser.rs`, `lexer.rs`, `main.rs`, `error.rs`, `runner.rs`
- Tenor specification: `docs/TENOR.md` v0.3, sections 12-15 (ElaboratorSpec, Evaluation Model, Static Analysis, Executor Obligations)
- [rust-analyzer architecture documentation](https://rust-analyzer.github.io/book/contributing/architecture.html) -- crate layering, syntax independence, API boundary discipline
- [Cargo Workspaces - The Rust Programming Language](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html) -- workspace fundamentals
- [Large Rust Workspaces (matklad)](https://matklad.github.io/2021/08/22/large-rust-workspaces.html) -- flat layout, compilation unit boundaries
- [Oso: Building VS Code Extension with Rust, WASM, TypeScript](https://www.osohq.com/post/building-vs-code-extension-with-rust-wasm-typescript) -- DSL language server architecture, WASM tradeoffs
- [tower-lsp](https://github.com/ebkalderon/tower-lsp) -- Rust LSP framework
- [lsp-server (rust-analyzer)](https://github.com/rust-analyzer/lsp-server) -- synchronous LSP scaffold
- [Rust Compiler Architecture - rustc dev guide](https://rustc-dev-guide.rust-lang.org/overview.html) -- multi-pass compiler patterns, shared IR

---

*Architecture analysis: 2026-02-21*
