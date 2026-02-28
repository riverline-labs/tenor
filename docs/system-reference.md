# Tenor — System Reference

## 1. Summary

Tenor is a formal contract language for encoding multi-party business agreements as machine-verifiable specifications. A Tenor contract declares facts (external inputs), entities (finite state machines), rules (stratified verdict producers), operations (persona-gated state transitions), and flows (orchestration DAGs) — then a six-pass elaborator transforms the DSL into a canonical JSON interchange format that any conforming evaluator or executor can consume. The language is deliberately non-Turing-complete: no loops, no aggregation, no unbounded computation. Every execution path is statically enumerable, every authority boundary is derivable, and every verdict is traceable to its source facts. The system includes SDKs for TypeScript, Python, and Go; a static analyzer (S1–S8 properties); a WASM-compiled evaluator; a visual contract builder; automatic UI generation; contract signing and verification; and a hosted multi-tenant platform with full provenance tracking.

---

## 2. Architecture Overview

```
                              ┌─────────────────────────────────────┐
                              │         PUBLIC REPO (tenor)          │
                              │                                     │
                              │  ┌───────────┐   ┌──────────────┐  │
                              │  │ tenor-core │   │ tenor-eval   │  │
                              │  │ (elaborator│   │ (evaluator,  │  │
                              │  │  6-pass)   │   │  migration,  │  │
                              │  └───────────┘   │  policies,   │  │
                              │                   │  adapters)   │  │
                              │  ┌───────────┐   └──────────────┘  │
                              │  │ tenor-cli  │   ┌──────────────┐  │
                              │  │ (26 cmds)  │   │tenor-analyze │  │
                              │  └───────────┘   │ (S1–S8)      │  │
                              │                   └──────────────┘  │
                              │  ┌───────────┐   ┌──────────────┐  │
                              │  │tenor-codegen   │ tenor-lsp    │  │
                              │  │(TypeScript)│   │ (editor)     │  │
                              │  └───────────┘   └──────────────┘  │
                              │                                     │
                              │  ┌──────────────────────────────┐  │
                              │  │     tenor-interchange         │  │
                              │  │ (shared types + deser)        │  │
                              │  └──────────────────────────────┘  │
                              │  ┌──────────────────────────────┐  │
                              │  │     tenor-storage (trait)      │  │
                              │  └──────────────────────────────┘  │
                              │  ┌──────────────────────────────┐  │
                              │  │  executor-conformance (E1-E20)│  │
                              │  └──────────────────────────────┘  │
                              │                                     │
                              │  ┌─────────┐ ┌─────┐ ┌──────────┐ │
                              │  │ tenor-  │ │ Go  │ │  Python  │ │
                              │  │eval-wasm│ │ SDK │ │   SDK    │ │
                              │  │ (WASM)  │ │(wazero)│ (PyO3)  │ │
                              │  └────┬────┘ └──┬──┘ └────┬─────┘ │
                              │       │         │         │        │
                              │  ┌────┴─────────┴─────────┴─────┐  │
                              │  │    TypeScript SDK (@tenor/sdk) │  │
                              │  └──────────────────────────────┘  │
                              │                                     │
                              │  ┌──────────┐   ┌───────────────┐  │
                              │  │  Builder  │   │ Auto UI gen   │  │
                              │  │ (React)   │   │ (React)       │  │
                              │  └──────────┘   └───────────────┘  │
                              │                                     │
                              │  ┌──────────┐   ┌───────────────┐  │
                              │  │ VS Code  │   │ Conformance   │  │
                              │  │Extension │   │ Suite (127)   │  │
                              │  └──────────┘   └───────────────┘  │
                              └──────────────────┬──────────────────┘
                                                 │
                                    depends on (never reverse)
                                                 │
                              ┌──────────────────┴──────────────────┐
                              │       PRIVATE REPO (platform)        │
                              │                                      │
                              │  Execution kernel, Postgres storage, │
                              │  HTTP API server, agent runtime,     │
                              │  multi-tenant hosting, admin UI      │
                              │                                      │
                              │  Implements executor obligations     │
                              │  E1–E20 from the public spec.        │
                              └──────────────────────────────────────┘
```

**Dependency direction:** Private repo depends on public repo crates (`tenor-eval`, `tenor-interchange`, `tenor-storage`, `tenor-executor-conformance`). The public repo has zero knowledge of the private repo. This is a hard invariant — the public repo can be updated independently.

**WASM evaluator:** `tenor-eval` compiles to WebAssembly via `wasm-pack`. The WASM module powers the TypeScript SDK (Node.js), Go SDK (via wazero), and the Builder SPA (browser). The WASM crate is excluded from the Cargo workspace and built separately.

**SDKs:** TypeScript wraps the WASM module directly. Python uses PyO3 native bindings (no WASM). Go uses a Rust-to-WASM bridge compiled with `wasm32-wasi` target, loaded by wazero at runtime.

---

## 3. Technology Stack

### Languages

| Language   | Version               | Where Used                                               |
| ---------- | --------------------- | -------------------------------------------------------- |
| Rust       | 2021 edition (stable) | All crates — elaborator, evaluator, CLI, SDKs, WASM      |
| TypeScript | 5.9.3                 | TypeScript SDK, Builder SPA, VS Code extension, examples |
| Python     | 3.10+                 | Python SDK (PyO3 bindings)                               |
| Go         | 1.25.5                | Go SDK (wazero WASM bridge)                              |

### Rust Crate Dependencies

#### Workspace-Level (shared by all crates)

| Crate           | Version | Purpose                                                       |
| --------------- | ------- | ------------------------------------------------------------- |
| `async-trait`   | 0.1     | Async trait method support                                    |
| `axum`          | 0.8     | HTTP framework (CLI serve command)                            |
| `axum-server`   | 0.8     | TLS support for axum (optional, behind `tls` feature)         |
| `clap`          | 4.5     | CLI argument parsing (derive macros)                          |
| `jsonschema`    | 0.42    | JSON Schema validation for interchange                        |
| `serde`         | 1       | Serialization/deserialization (derive)                        |
| `serde_json`    | 1       | JSON processing                                               |
| `rust_decimal`  | 1.40    | Fixed-point decimal arithmetic (serde-with-str)               |
| `sha2`          | 0.10    | SHA-256 hashing (etag computation, WASM signing)              |
| `time`          | 0.3     | Date/DateTime/Duration handling (parsing, formatting, macros) |
| `tokio`         | 1       | Async runtime (full features)                                 |
| `tower-http`    | 0.6     | HTTP middleware (CORS)                                        |
| `ureq`          | 3       | HTTP client (adapter framework, LLM policy)                   |
| `lsp-server`    | 0.7     | Language Server Protocol transport                            |
| `lsp-types`     | 0.97    | LSP type definitions                                          |
| `ed25519-dalek` | 2.2     | Ed25519 signing/verification (rand_core)                      |
| `base64`        | 0.22    | Base64 encoding for signatures                                |
| `rand`          | 0.8     | Random number generation (interactive policy)                 |
| `thiserror`     | 2       | Error derive macros                                           |

#### Per-Crate Dependencies

| Crate               | Version | Used By                    | Purpose                                         |
| ------------------- | ------- | -------------------------- | ----------------------------------------------- |
| `flate2`            | 1       | tenor-cli                  | Gzip compression (template packaging)           |
| `tar`               | 0.4     | tenor-cli                  | Tar archive creation (template packaging)       |
| `toml`              | 1       | tenor-cli                  | TOML config parsing (adapter config, templates) |
| `tempfile`          | 3       | tenor-cli (dev)            | Temporary files for testing                     |
| `assert_cmd`        | 2       | tenor-cli (dev)            | CLI integration testing                         |
| `predicates`        | 3       | tenor-cli (dev)            | Assertion predicates for CLI tests              |
| `pyo3`              | 0.28    | Python SDK                 | Python-Rust FFI (abi3-py39)                     |
| `wasm-bindgen`      | 0.2     | tenor-eval-wasm            | Rust-to-WASM bindings                           |
| `wasm-bindgen-test` | 0.3     | tenor-eval-wasm (dev)      | WASM test harness                               |
| `slab`              | 0.4     | tenor-eval-wasm, Go bridge | Arena allocator for WASM contract handles       |

### Node.js / TypeScript Dependencies

#### TypeScript SDK (`@tenor/sdk`)

| Package      | Version | Purpose       |
| ------------ | ------- | ------------- |
| `typescript` | 5.9.3   | Type checking |
| `vitest`     | 4.0.18  | Test runner   |

#### Builder SPA

| Package                       | Version | Purpose                      |
| ----------------------------- | ------- | ---------------------------- |
| `react`                       | 19.2.4  | UI framework                 |
| `react-dom`                   | 19.2.4  | DOM rendering                |
| `react-router`                | 7.13.1  | Client-side routing          |
| `zustand`                     | 5.0.11  | State management             |
| `zundo`                       | 2.3.0   | Undo/redo for zustand        |
| `tailwindcss`                 | 4.2.1   | Utility CSS                  |
| `vite`                        | 7.3.1   | Build tool                   |
| `vite-plugin-wasm`            | 3.5.0   | WASM loading in Vite         |
| `vite-plugin-top-level-await` | 1.6.0   | Top-level await support      |
| `@testing-library/react`      | 16.3.2  | Component testing            |
| `happy-dom`                   | 20.7.0  | DOM implementation for tests |
| `@vitest/coverage-v8`         | 4.0.18  | Code coverage                |

#### VS Code Extension

| Package                 | Version | Purpose    |
| ----------------------- | ------- | ---------- |
| `vscode-languageclient` | 9.0.1   | LSP client |

### Go Dependencies

| Module                          | Version | Purpose               |
| ------------------------------- | ------- | --------------------- |
| `github.com/tetratelabs/wazero` | 1.11.0  | WASM runtime (no CGo) |

### Python Build

| Tool      | Version        | Purpose                  |
| --------- | -------------- | ------------------------ |
| `maturin` | >= 1.12, < 2.0 | Rust-Python build system |

### Build Tools

| Tool        | Purpose                                              |
| ----------- | ---------------------------------------------------- |
| `cargo`     | Rust workspace build                                 |
| `wasm-pack` | WASM compilation and packaging                       |
| `maturin`   | Python native extension build                        |
| `vite`      | TypeScript/React bundling (Builder, admin dashboard) |
| `tsc`       | TypeScript compilation                               |
| `npm`       | Node.js package management                           |

---

## 4. Repository Structure

### Public Repo (`tenor`)

```
tenor/
├── Cargo.toml                    Workspace root (9 members + 1 excluded)
├── Cargo.lock
├── CLAUDE.md                     Project instructions
├── README.md                     Project documentation
├── STABILITY.md                  Stability guarantees
├── LICENSE                       MIT license
├── Dockerfile                    Multi-stage build (rust:1.93 → debian:trixie-slim)
├── docker-compose.yml            Evaluator service + volume mount
│
├── crates/
│   ├── core/                     tenor-core: 6-pass elaborator
│   │   └── src/
│   │       ├── lib.rs            Public API (elaborate, TENOR_VERSION, TENOR_BUNDLE_VERSION)
│   │       ├── ast.rs            AST types (RawConstruct, RawType, RawExpr, RawTerm)
│   │       ├── elaborate.rs      Pipeline orchestrator
│   │       ├── error.rs          ElabError type
│   │       ├── lexer.rs          Tokenizer
│   │       ├── source.rs         FileProvider trait (filesystem, WASM, in-memory)
│   │       ├── pass1_bundle.rs   Import resolution, bundle assembly
│   │       ├── pass2_index.rs    Construct indexing
│   │       ├── pass3_types.rs    Type environment
│   │       ├── pass4_typecheck.rs Type resolution and expression checking
│   │       ├── pass5_validate/   Structural validation (entity, rule, operation, flow, source, system, parallel)
│   │       ├── pass6_serialize.rs JSON interchange serialization
│   │       └── parser/           DSL parser (constructs, expressions, types, flow, system)
│   │
│   ├── cli/                      tenor-cli: 26-subcommand CLI binary
│   │   └── src/
│   │       ├── main.rs           Clap dispatcher
│   │       ├── runner.rs         Conformance suite runner
│   │       ├── tap.rs            TAP v14 output formatter
│   │       ├── explain.rs        Natural language contract explanation
│   │       ├── agent.rs          Interactive agent shell
│   │       ├── builder.rs        Builder SPA dev server
│   │       ├── migrate.rs        Contract migration analysis
│   │       ├── connect.rs        LLM-powered source wiring
│   │       ├── ui.rs             React UI generation
│   │       ├── manifest.rs       TenorManifest generation
│   │       ├── trust.rs          Ed25519 signing/verification
│   │       ├── serve/            HTTP API server (elaborate, evaluate, simulate)
│   │       ├── commands/         Subcommand implementations
│   │       ├── template/         Registry: pack, publish, search, install, deploy
│   │       └── ambiguity/        AI ambiguity testing
│   │
│   ├── eval/                     tenor-eval: contract evaluator
│   │   └── src/
│   │       ├── lib.rs            evaluate(), evaluate_flow()
│   │       ├── rules.rs          Stratified rule evaluation
│   │       ├── predicate.rs      Predicate expression evaluation
│   │       ├── operation.rs      Operation execution, entity state management
│   │       ├── numeric.rs        Fixed-point decimal arithmetic
│   │       ├── assemble.rs       FactSet assembly from JSON
│   │       ├── action_space.rs   Action space computation
│   │       ├── provenance.rs     Provenance recording
│   │       ├── fact_provider.rs  FactProvider trait
│   │       ├── flow/             Flow execution engine (snapshot, routing, parallel, sub-flow)
│   │       ├── migration/        Contract version migration (diff, classify, plan, execute)
│   │       ├── adapter/          Fact adapters (HTTP, database, static, manual)
│   │       ├── policy/           Agent policies (random, priority, LLM, composite, human-in-the-loop)
│   │       └── types/            Runtime types (Contract, Value, FactSet, VerdictSet)
│   │
│   ├── analyze/                  tenor-analyze: static analysis (S1–S8)
│   │   └── src/
│   │       ├── lib.rs            analyze(), analyze_selected()
│   │       ├── bundle.rs         Analysis bundle deserialization
│   │       ├── report.rs         AnalysisReport, Finding extraction
│   │       ├── s1_state_space.rs     S1: complete state space enumeration
│   │       ├── s2_reachability.rs    S2: reachable/dead state detection
│   │       ├── s3a_admissibility.rs  S3a: structural admissibility per state
│   │       ├── s4_authority.rs       S4: authority topology derivation
│   │       ├── s5_verdicts.rs        S5: verdict and outcome space
│   │       ├── s6_flow_paths.rs      S6: flow path enumeration
│   │       ├── s7_complexity.rs      S7: evaluation complexity bounds
│   │       └── s8_verdict_uniqueness.rs S8: verdict uniqueness (pre-verified)
│   │
│   ├── codegen/                  tenor-codegen: TypeScript code generation
│   │   └── src/
│   │       ├── lib.rs            generate_typescript()
│   │       ├── bundle.rs         CodegenBundle deserialization
│   │       ├── typescript.rs     Type definitions emission
│   │       ├── typescript_client.rs  Client class emission
│   │       └── typescript_schemas.rs Zod schema emission
│   │
│   ├── interchange/              tenor-interchange: shared types
│   │   └── src/
│   │       ├── lib.rs            from_interchange()
│   │       ├── types.rs          Construct types (Fact, Entity, Rule, Operation, Flow, etc.)
│   │       └── deserialize.rs    JSON deserialization
│   │
│   ├── lsp/                      tenor-lsp: Language Server Protocol
│   │   └── src/
│   │       ├── server.rs         LSP main loop
│   │       ├── diagnostics.rs    Error reporting
│   │       ├── completion.rs     Autocomplete
│   │       ├── navigation.rs     Go-to-definition, references
│   │       ├── semantic_tokens.rs Syntax highlighting
│   │       ├── hover.rs          Hover information
│   │       ├── agent_capabilities.rs Agent capabilities preview
│   │       └── document.rs       Document management
│   │
│   ├── storage/                  tenor-storage: abstract storage trait
│   │   └── src/
│   │       ├── traits.rs         TenorStorage async trait
│   │       ├── record.rs         Record types
│   │       └── error.rs          StorageError
│   │
│   ├── executor-conformance/     E1–E20 test fixtures
│   │   └── src/
│   │       ├── suite.rs          Test runner
│   │       ├── traits.rs         Executor test trait
│   │       └── fixtures.rs       E1–E20 test cases
│   │
│   └── tenor-eval-wasm/          WASM evaluator (excluded from workspace)
│       └── src/
│           ├── lib.rs            wasm-bindgen entry point
│           └── inspect.rs        Contract introspection
│
├── sdks/
│   ├── typescript/               @tenor/sdk — WASM-powered evaluator + HTTP client
│   │   ├── src/
│   │   │   ├── index.ts          SDK exports
│   │   │   ├── evaluator.ts      WASM evaluator wrapper
│   │   │   ├── client.ts         HTTP client
│   │   │   ├── client-types.ts   Client type definitions
│   │   │   ├── types.ts          TypeScript types
│   │   │   ├── action-space.ts   Action space types
│   │   │   └── errors.ts         Error types
│   │   └── wasm/                 WASM module output
│   │
│   ├── go/                       Go SDK — wazero WASM bridge
│   │   ├── tenor.go              Evaluator struct
│   │   ├── types.go              Go types
│   │   └── wasm-bridge/          Rust WASM bridge (wasm32-wasi target)
│   │
│   ├── python/                   Python SDK — PyO3 native bindings
│   │   ├── python/tenor/         Python package
│   │   └── src/evaluator.rs      PyO3 bindings
│   │
│   └── conformance/              Cross-SDK conformance
│       ├── fixture-gen/          Rust fixture generator
│       └── runners/go-runner/    Go conformance runner
│
├── builder/                      Tenor Builder — React SPA
│   ├── src/                      Visual contract editor
│   └── package.json
│
├── editors/
│   └── vscode/                   VS Code extension (LSP client)
│
├── examples/
│   ├── express-middleware/        Express.js integration example
│   ├── audit-agent/              Audit agent example
│   └── slack-bot/                Slack bot example
│
├── conformance/                  Elaborator conformance suite
│   ├── positive/                 Valid DSL → expected JSON (41 tests)
│   ├── negative/                 Invalid DSL → expected error (pass0–pass5)
│   ├── numeric/                  Decimal/Money precision (10 tests)
│   ├── promotion/                Numeric type promotion (2 tests)
│   ├── shorthand/                DSL shorthand expansion (2 tests)
│   ├── cross_file/               Multi-file import (2 tests)
│   ├── parallel/                 Parallel entity conflicts (2 tests)
│   ├── analysis/                 Static analysis (5 tests)
│   ├── manifest/                 Manifest generation (1 test)
│   └── eval/                     Evaluator tests (positive, numeric, frozen)
│
├── docs/
│   ├── tenor-language-specification.md  Formal specification v1.0
│   ├── agent-orientation.md      Architecture, constraints, trust model (agent briefing)
│   ├── author-guide.md           Contract authoring guide
│   ├── narrative.md              Design narrative
│   ├── what-is-tenor.md          High-level introduction
│   └── minimal-kernel.md         Minimal language subset
│
├── schema/
│   ├── interchange-schema.json   JSON Schema for interchange format
│   └── manifest-schema.json      JSON Schema for TenorManifest
└── domains/                      Example domain contracts
```

### Private Repo (architectural summary only)

The private repo contains five Rust crates that implement the commercial execution platform:

- **Execution kernel** — Implements executor obligations E1–E20 with atomic flow execution against Postgres, optimistic concurrency, and full provenance tracking.
- **Storage backend** — Postgres implementation of the `TenorStorage` trait defined in the public repo. Owns all database migrations.
- **HTTP server** — API server for contract execution, multi-tenant management, and marketplace operations.
- **CLI binary** — Platform-specific commands (serve, deploy, execute, migrate, entity management, agent runtime).
- **Agent runtime** — Autonomous observe-evaluate-choose-execute loop with pluggable action-selection policies.
- **Admin dashboard** — React SPA for platform operators.

All private crates depend on public repo crates via git dependency. A local `.cargo/config.toml` (gitignored) patches these to local paths for development.

---

## 5. Spec Summary

The Tenor specification (`docs/tenor-language-specification.md`) is a formal v1.0 document defining the complete language semantics.

### §4 BaseType

Twelve primitive value types form a closed set: `Bool`, `Int(min, max)`, `Decimal(precision, scale)`, `Text(max_length)`, `Enum(values)`, `Date`, `DateTime`, `Money(currency)`, `Record(fields)`, `TaggedUnion(variants)`, `List(element_type, max)`, and `Duration(unit, min, max)`. Each type has a defined operator set (Bool: `= != and or not`; Int/Decimal: `= != < <= > >= + - * literal`; Money: `= != < <= > >=` same-currency; Text/Enum: `= !=`; Date/DateTime: `= != < <= > >=`; Record/TaggedUnion: `= !=` field-wise; List: `len()`, element access). Named type aliases (`TypeDecl`) are permitted only for Record and TaggedUnion, resolved during Pass 3 and inlined at all use sites — TypeDecl does not appear in interchange output. All numeric values are fixed-point (never floating-point). DateTime values are normalized to UTC. Duration "day" means exactly 86,400 seconds.

### §5 Fact (including §5A Source Declarations)

A Fact is a named, typed, sourced ground value representing external input. Facts are asserted or defaulted at FactSet assembly time and never derived by any rule, operation, or internal computation. Each Fact declares an id, a BaseType, a source (freetext string or structured reference to a declared Source), and optionally a default value. Sources are named declarations of external systems carrying protocol identity (`http`, `database`, `graphql`, `grpc`, `static`, `manual`, or `x_*` extensions) and connection metadata. Sources are infrastructure metadata with zero impact on evaluation — they are consumed by adapters, provenance enrichment, and automated tooling. Aggregate computation (sum, count, average) is not permitted in the contract; aggregates must arrive as Facts from external systems.

### §6 Entity (including §6.5 Multi-Instance)

An Entity is a finite state machine with a declared state set, initial state, and transition relation. The entity hierarchy (via parent pointers) must be acyclic. State is never derived — it is stored and updated solely by Operations. Multiple runtime instances of the same entity type may coexist, identified by `(EntityId, InstanceId)` composite keys. The `EntityStateMap` maps these pairs to current state values. Single-instance operation uses the degenerate `"_default"` instance ID. Instance creation is an executor concern (E15); new instances start in the declared initial state.

### §7 Rule

A Rule is a stratified, verdict-producing, side-effect-free evaluation function. Rules are assigned to explicit numbered strata; a rule in stratum N may only reference verdicts from strata strictly below N. `eval_strata` is a fold from stratum 0 to max, accumulating verdicts. Each VerdictType is produced by exactly one rule (S8, enforced by elaboration). Variable-by-variable multiplication is permitted only in Rule bodies (not predicates); the product range is verified against the declared payload type.

### §8 Persona

A Persona is an opaque identity token representing an actor class. Personas carry no metadata — their sole purpose is making the authority namespace explicit and checkable. All persona references in Operations, Flows, and Systems must resolve to declared Personas. Unreferenced personas are not errors. Personas occupy a distinct namespace from other construct kinds.

### §9 Operation

An Operation is the sole construct producing entity state transitions. Each Operation declares `allowed_personas` (non-empty), a precondition, effects (entity transitions), error contract, and outcomes. The execution sequence is invariant: (1) persona check, (2) precondition evaluation, (3) outcome determination, (4) atomic effect application, (5) provenance emission. For multi-outcome Operations, each effect is associated with exactly one outcome. No wildcard transitions — every effect must name explicit source and target states.

### §10 PredicateExpression

A quantifier-free first-order logic formula over ground terms from the FactSet, ResolvedVerdictSet, and literal constants. Supports comparison, arithmetic (literal multiplication only — no variable-by-variable), logical connectives, and bounded quantification over List-typed Facts. No implicit type coercions. Entity state is not a predicate term — state constraints are enforced through effect declarations.

### §11 Flow

A Flow is a finite DAG of steps: OperationStep, BranchStep, HandoffStep, ParallelStep, SubFlowStep, and Terminal. The snapshot (FactSet + VerdictSet) is frozen at initiation and never recomputed during execution. OperationStep outcome routing must be exhaustive (outcome map keys = Operation's declared outcomes). Every OperationStep and SubFlowStep must declare a FailureHandler. No two parallel branches may have overlapping entity effect sets. SubFlows inherit the parent's snapshot and instance bindings.

### §12 System

A System declares cross-contract relationships: shared personas (identity equivalence), triggers (flow-to-flow activation on terminal outcome), and shared entities (identical state sets across members). Systems are elaborated independently; validation occurs in Pass 5. Triggers are asynchronous (at-most-once delivery).

### §13 NumericModel

All numeric computation uses fixed-point decimal arithmetic — no floating-point anywhere. Rounding mode is round-half-to-even (IEEE 754). Implementation bounds: 28 maximum significant digits, 0–28 scale range. Overflow produces typed abort (no silent wraparound). The promotion function is total and commutative, with rules for Int+Int, Decimal+Decimal, Int×Decimal cross-type promotion.

### §14 Elaborator (Six Passes)

Pass 0 (Lex/Parse): tokenize and parse per grammar. Pass 1 (Bundle): import resolution, cycle detection, sandbox verification, duplicate id check. Pass 2 (Index): build `(kind, id)` index. Pass 3 (Type environment): resolve TypeDecl definitions, detect cycles, build named type lookup. Pass 4 (Typecheck): type-check expressions, resolve references, apply promotion rules. Pass 5 (Validate): structural validation of all constructs. Pass 6 (Serialize): canonical JSON with sorted keys, structured numeric values, deterministic construct ordering. The conformance suite validates all passes.

### §15 Evaluation Model

Contract evaluation proceeds in stages: `assemble_facts` → `eval_strata` → `take_snapshot` → `execute_flow`. Frozen verdict semantics guarantee the verdict set is computed once at Flow initiation and never recomputed — entity state changes mid-Flow do not affect verdict evaluations. The action space is computable: `compute_action_space(contract, facts, entity_state_map, persona)` returns available and blocked actions. No built-in functions — all time-varying values enter as Facts.

### §16 Static Analysis (S1–S8)

Eight properties derivable from a contract alone: S1 (complete state space), S2 (reachable states), S3a (structural admissibility per state), S3b (domain satisfiability, qualified), S4 (authority topology), S5 (verdict and outcome space), S6 (flow path enumeration), S7 (evaluation complexity bounds), S8 (verdict uniqueness, enforced by Pass 5).

### §17 Executor Obligations (E1–E20)

Logic conformance: same bundle + FactSet → same verdicts and transitions. E1 (external source integrity), E2 (transition source validation), E3 (atomicity), E4 (snapshot isolation), E5 (sub-flow snapshot inheritance), E6 (UTC normalization), E7 (numeric conformance), E8 (branch isolation), E9 (join after completion), E10 (serve manifest at `/.well-known/tenor`), E11 (complete manifest bundle), E12 (etag iff bundle changes), E13 (dry-run support), E14 (capability advertisement), E15 (instance creation in initial state), E16 (instance identity stability), E17 (instance enumeration completeness). Trust obligations: E18 (artifact integrity attestation, capability), E19 (provenance authenticity, capability), E20 (trust domain identification, optional).

### §18 Migration (M1–M8)

Breaking change taxonomy: BREAKING, NON_BREAKING, REQUIRES_ANALYSIS, INFRASTRUCTURE. Every `(construct_kind, field, change_type)` triple has a defined classification. Executor obligations: M1 (detect via structural diff), M2 (declare in-flight flow migration policy), M3 (never silently deploy BREAKING), M4 (validate orphaned entity state), M5 (validate in-flight flow coverage), M6 (treat REQUIRES_ANALYSIS as BREAKING unless proven safe), M7 (exclude provenance/line from diff), M8 (compare unordered sets as sets). In-flight policies: Blue-Green, Force-Migrate, Abort.

### §19 Contract Discovery

TenorManifest wraps the interchange bundle with etag (SHA-256) and optional capabilities/trust fields. Cold-start: one fetch of `/.well-known/tenor` provides everything. Change detection via `If-None-Match`. Dry-run: steps 1–3 only (no effects), all responses carry `"simulation": true`.

### §20 Acknowledged Limitations

84 deliberate design decisions limiting scope. Key examples: no aggregation in contracts (AL1), mandatory persona declarations (AL24), no outcome payloads (AL28), no shared type federation (AL31), no concurrent operation isolation specification (AL52), byte-exact text comparison (AL53), attestation format not standardized (AL81).

---

## 6. Elaboration Pipeline

### Pass 0+1: Lex, Parse, Bundle Assembly

**Source files:** `crates/core/src/lexer.rs`, `crates/core/src/parser/`, `crates/core/src/pass1_bundle.rs`

**Input:** Source text file paths.
**Output:** Flat `Vec<RawConstruct>` with provenance + bundle ID (root filename).

**Token types:** Word, Str, Int, Float (kept as string), braces, brackets, parens, colon, comma, dot. **Operators:** `= != < <= > >= *` and logical `and or not forall exists in`. **Arrow tokens:** `→` and `->` are the same token. Comments: `//` line, `/* */` block.

**Parser structure:** Recursive descent with submodules for constructs, expressions, types, flow steps, and system declarations.

**Bundle assembly:**

- Recursive file loading with import resolution
- Import cycle detection (parallel HashSet + Vec stack, O(1) checks)
- Sandbox verification: all imports must stay within contract root directory
- Type library constraint: files with only TypeDecl cannot have import statements
- Cross-file duplicate detection: same `(kind, id)` across different files is an error

**Error messages (Pass 0/1):**

- `"cannot open file '...'"` — file read failure
- `"import cycle detected"` — cyclic import graph
- `"cannot resolve import '...'"` — path doesn't exist
- `"import '...' escapes the contract root directory"` — sandbox violation
- `"duplicate {} id '{}': first declared in {}"` — construct redefined
- `"type library files may not contain import declarations"` — constraint violation
- `"unterminated block comment"` — lexer error

### Pass 2: Construct Indexing

**Source file:** `crates/core/src/pass2_index.rs`

**Input:** Flat construct list.
**Output:** `Index` struct with HashMaps for fast lookup.

**Index contents:**

- Per-kind maps: facts, entities, rules, operations, flows, type_decls, personas, systems, sources → Provenance
- `rule_verdicts`: rule_id → verdict_type
- `verdict_strata`: verdict_type → (rule_id, stratum)
- `operation_outcomes`: operation_id → Vec of outcome labels
- `operation_allowed_personas`: operation_id → Vec of persona ids

**Error messages (Pass 2):**

- `"duplicate {} id '{}': first declared at line {}"` — same-kind duplicate

### Pass 3: Type Environment

**Source file:** `crates/core/src/pass3_types.rs`

**Input:** Constructs + Index.
**Output:** `TypeEnv = HashMap<String, RawType>` (name → resolved type).

**Operations:**

- Extract all TypeDecl constructs
- Detect TypeDecl cycles via DFS (circular type references)
- Resolve named types (TypeRef → concrete BaseType)
- Build lookup table for all named types

**Error messages (Pass 3):**

- `"TypeDecl cycle detected: A → B → C → A"` — circular type references (includes full cycle path)

### Pass 4: Type Resolution and Expression Checking

**Source file:** `crates/core/src/pass4_typecheck.rs`

**Input:** Constructs + TypeEnv.
**Output:** Constructs with all TypeRef nodes resolved; expression type errors caught.

**Two phases:**

**Pass 4a — Type Resolution:** Replace all `TypeRef(name)` with concrete RawType. Handles nested types (Records, Lists, TaggedUnions). Resolves types in Fact declarations and Rule payload types.

**Pass 4b — Expression Type-Checking:** Validate rule predicates and produce clauses. Fact reference resolution. Operator compatibility (`Bool` only supports `= !=`; `Text` and `Enum` reject ordering operators). Numeric type promotion rules. Multiplication safety: `var × var` forbidden, only `var × literal`. Product range validation for Int payloads.

**Error messages (Pass 4):**

- `"unknown type reference '...'"` — TypeRef not in environment
- `"unresolved fact reference: '...' is not declared"` — fact doesn't exist
- `"operator '...' not defined for Bool"` — type mismatch
- `"variable × variable multiplication is not permitted"` — forbidden operation
- `"type error: product range Int(...) is not contained in declared verdict payload type Int(...)"` — overflow check

### Pass 5: Structural Validation

**Source files:** `crates/core/src/pass5_validate/` (mod.rs, entity.rs, rule.rs, operation.rs, flow.rs, source.rs, system.rs, parallel.rs)

**Input:** Constructs + Index.
**Output:** Validation report (all constructs valid or first error).

**Entity validation:** Initial state ∈ declared states. All transition endpoints ∈ declared states. Entity DAG acyclic (parent pointers).

**Rule validation:** Stratum ≥ 0. Verdict references resolve to produced verdicts. Stratum constraint: rule at stratum S can only reference verdicts from strata < S. Verdict uniqueness (S8): no two rules produce the same verdict type.

**Operation validation:** `allowed_personas` non-empty. Personas resolve to declarations. Effects reference declared entities. Multi-outcome: each effect associated with an outcome label. Outcome labels unique.

**Flow validation:** Entry step exists. All step references resolve. OperationStep must declare FailureHandler. OperationStep outcome routing is exhaustive.

**Source validation:** Core protocol required fields (C-SRC-03). Extension tag format (C-SRC-04). Structured source references resolve (C-SRC-06).

**System validation:** Member id uniqueness (C-SYS-01). Shared persona existence in members (C-SYS-06). Trigger references valid (C-SYS-07–12). Shared entity state set equality (C-SYS-14). Trigger graph acyclic (C-SYS-15).

**Parallel validation:** Non-overlapping entity effect sets across branches.

### Pass 6: Interchange Serialization

**Source file:** `crates/core/src/pass6_serialize.rs`

**Input:** Constructs + bundle ID.
**Output:** `serde_json::Value` — canonical interchange JSON.

**Serialization rules:**

- Constructs grouped by kind: Personas, Sources, Facts, Entities, Rules (by stratum), Operations, Flows, Systems
- Within each kind, sorted by ID (deterministic ordering)
- All JSON keys sorted lexicographically within each object
- Decimal/Money defaults: `{"kind": "decimal_value", "precision": P, "scale": S, "value": "..."}` using **declared type's** P/S (not inferred from literal)
- Multiplication: `{"left": {...}, "literal": N, "op": "*", "result_type": {...}}`
- `comparison_type` emitted on Compare nodes for: Money (always), Int × Decimal (cross-type), Mul × Int

### Error Type

**Source file:** `crates/core/src/error.rs`

```rust
pub struct ElabError {
    pub pass: u8,              // 0–6 identifying which pass failed
    pub construct_kind: Option<String>,  // e.g., "Fact", "Rule"
    pub construct_id: Option<String>,    // e.g., "score", "approve_order"
    pub field: Option<String>,           // e.g., "type", "initial", "stratum"
    pub file: String,
    pub line: u32,
    pub message: String,
}
```

Serializes to JSON matching the conformance suite `expected-error.json` format.

---

## 7. Evaluation Model

**Source files:** `crates/eval/src/lib.rs`, `crates/eval/src/rules.rs`, `crates/eval/src/predicate.rs`, `crates/eval/src/operation.rs`, `crates/eval/src/assemble.rs`, `crates/eval/src/flow/`, `crates/eval/src/numeric.rs`, `crates/eval/src/action_space.rs`

### Top-Level API

- `evaluate(bundle, facts)` → `Result<EvalResult, EvalError>` — rules only
- `evaluate_flow(bundle, facts, flow_id, persona, entity_states, instance_bindings)` → `Result<FlowEvalResult, EvalError>` — full execution pipeline

### FactSet Assembly (`assemble.rs`)

Validates all provided values against declared types, applies defaults where values are missing, aborts if required facts lack both value and default. Type validation covers all 12 base types including nested Records, Lists, TaggedUnions with range/length/enum checking.

### Stratified Rule Evaluation (`rules.rs`)

BTreeMap stratum index for O(n) build + O(n) evaluate. For each stratum in order, evaluate all rules' conditions against facts + lower-stratum verdicts. True conditions produce VerdictInstances with provenance (rule id, stratum, facts_used, verdicts_used).

### Predicate Evaluation (`predicate.rs`)

Recursive tree walk over Predicate enum. Handles FactRef (lookup in FactSet), FieldRef (record field access), Literal (constant), VerdictPresent (set membership), Compare (numeric::compare_values with cross-type promotion), And/Or (short-circuit), Not (negation), Forall/Exists (bounded quantification over List facts), Mul (multiplication).

### Operation Execution (`operation.rs`)

**Types:**

- `EntityStateMap = BTreeMap<(String, String), String>` — `(entity_id, instance_id) → state`
- `InstanceBindingMap = BTreeMap<String, String>` — `entity_id → instance_id`
- `DEFAULT_INSTANCE_ID = "_default"` — single-instance degenerate case

**Execution sequence:**

1. Persona check (set membership in `allowed_personas`)
2. Precondition evaluation (eval_pred against frozen snapshot)
3. Effect application (entity state transitions per instance)
4. Outcome determination and routing
5. Provenance recording (per-instance before/after snapshots)

**Error types:** `PersonaRejected`, `PreconditionFailed`, `InvalidEntityState`, `EntityNotFound`, `EvalError`

### Flow Execution (`flow/`)

**Frozen snapshot semantics:** FactSet + VerdictSet created at flow initiation, NEVER mutated during execution. Entity state changes tracked separately in mutable EntityStateMap.

**Step types:**

- OperationStep: execute operation, route by outcome
- BranchStep: evaluate predicate, route true/false
- HandoffStep: pause for user input
- ParallelStep: execute branches concurrently, merge results (all branches complete before join)
- SubFlowStep: nested flow execution (inherits parent snapshot)

**Failure handling:** TerminateHandler or CompensateHandler (cascade compensation steps).

**Flow result:** outcome (success/failure/escalation), steps_executed, entity_state_changes, initiating_persona.

### Numeric Operations (`numeric.rs`)

Fixed-point decimal only. Int, Decimal, Money comparisons with type promotion. Cross-type comparisons (Int × Decimal). Arithmetic with overflow checking. Currency validation for Money comparisons. Round-half-to-even rounding.

### Action Space (`action_space.rs`)

`compute_action_space(contract, facts, entity_state_map, persona)` → available flows with eligible instance bindings, plus blocked actions with reasons. Size is O(|flows| × product of |instances|).

---

## 8. Migration System

**Source files:** `crates/eval/src/migration/` (mod.rs, diff.rs, classify.rs, analysis.rs, plan.rs, executor.rs, error.rs)

### Diff Computation (`diff.rs`)

Compares two interchange bundles. Identifies added/removed constructs and field-level changes with before/after values. Normalized comparison (primitive arrays sorted, objects preserve order). Provenance and line fields excluded from diff (M7).

### Breaking Change Classification (`classify.rs`)

Every `(construct_kind, field, change_type)` triple has a defined severity:

- **BREAKING:** schema incompatible, verdict type removed, required field removed, state removed
- **NON_BREAKING:** new construct, additive changes, new state/transition
- **REQUIRES_ANALYSIS:** predicate change, default value change, complex field modifications
- **INFRASTRUCTURE:** Source construct changes, adapter wiring

Construct-level classification is the supremum of field-level classifications.

### Three-Layer Flow Compatibility (`analysis.rs`)

In-flight flows are force-migratable if: (1) **Forward path existence** — every reachable step has v2 equivalent, (2) **Data dependency satisfaction** — fact/verdict references satisfied by frozen snapshot or v2 defaults, (3) **Entity state equivalence** — current state is member of v2 state set and transitions exist.

### Migration Plan (`plan.rs`)

Entity state mappings (from_state → to_state per entity/instance). Flow compatibility check. Migration policy selection (allow breaking, require analysis, strict).

### Migration Execution (`executor.rs`)

Atomic via TenorStorage backend. Per entity: read state, validate expected, update state, record provenance. All in single storage snapshot (transaction) — all-or-nothing semantics.

### CLI Usage

```bash
tenor diff v1.json v2.json                # Structural diff
tenor diff v1.json v2.json --breaking     # Breaking change classification
tenor migrate v1.json v2.json             # Full migration analysis
tenor migrate v1.json v2.json --yes       # Skip confirmation
```

---

## 9. Source Declarations and Adapters

**Source files:** `crates/core/src/pass5_validate/source.rs` (elaboration), `crates/eval/src/adapter/` (runtime)

### Source Construct

A Source declares an external system's protocol and connection metadata. Six core protocols with required fields:

| Protocol   | Required Fields | Description             |
| ---------- | --------------- | ----------------------- |
| `http`     | `base_url`      | REST/HTTP API           |
| `database` | `dialect`       | Database query          |
| `graphql`  | `endpoint`      | GraphQL API             |
| `grpc`     | `proto_ref`     | gRPC service            |
| `static`   | (none)          | Static/hardcoded values |
| `manual`   | (none)          | Human-provided input    |

Extension protocols use `x_` prefix (e.g., `x_internal.event_bus`). Extension tags bypass required-field validation (C-SRC-04).

### Elaboration Constraints

- **C-SRC-01:** Source id uniqueness within contract
- **C-SRC-03:** Core protocol required fields validated (Pass 5)
- **C-SRC-04:** Extension tag format: `x_[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*`
- **C-SRC-05:** All field values are strings
- **C-SRC-06:** Structured source references on Facts resolve to declared Sources

### Adapter Framework (`crates/eval/src/adapter/`)

- **FactAdapter trait** — async fetch interface for each protocol
- **Reference implementations:** HTTP (GET with bearer auth), Database (Postgres query), Static (in-memory), Manual (prompt)
- **AdapterRegistry** — maps source IDs to configured adapters
- **AdapterFactProvider** — implements FactProvider using registry lookup
- **AdapterConfig** — TOML-based source-to-connection mapping

### Enriched Fact Provenance

`EnrichedFactProvenance` records: fact_id, source_id, path, fetched value, adapter_id, fetch_timestamp, raw source response. This is an executor capability (not obligation).

### `tenor connect`

LLM-powered source wiring tool. Reads contract's Source declarations and an environment schema (OpenAPI, GraphQL SDL, SQL), then proposes fact-to-endpoint mappings. Supports heuristic mode (pattern matching, no LLM) and batch mode (review file for human approval).

```bash
tenor connect contract.tenor --environment api-spec.yaml
tenor connect contract.tenor --heuristic --verbose
tenor connect contract.tenor --batch review.json
tenor connect contract.tenor --apply reviewed.json --out ./adapters
```

---

## 10. Multi-Instance Entities

**Source files:** `crates/eval/src/operation.rs`, `crates/eval/src/flow/`, `crates/eval/src/action_space.rs`

### EntityStateMap

Runtime state is keyed by `(entity_id, instance_id)` composite:

```rust
pub type EntityStateMap = BTreeMap<(String, String), String>;
```

Single-instance entities use `DEFAULT_INSTANCE_ID = "_default"`.

### InstanceBindingMap

At flow invocation, the caller specifies which entity instances the flow targets:

```rust
pub type InstanceBindingMap = BTreeMap<String, String>; // entity_id → instance_id
```

Missing bindings fall back to `"_default"`.

### Per-Instance Action Space

`compute_action_space` returns per-instance action availability. For each flow, it computes which entity instances are in states satisfying the flow's effect source states. The action space size scales with O(|flows| × product of |instances|).

### Executor Obligations

- **E15:** New instances initialized in declared initial state
- **E16:** Instance IDs remain stable for instance lifetime; reuse only after deletion
- **E17:** EntityStateMap provided to evaluator must be complete (every active instance present)

---

## 11. Trust and Security

**Source files:** `crates/cli/src/trust.rs`, `crates/cli/src/main.rs` (keygen, sign, verify, sign-wasm, verify-wasm commands)

### Ed25519 Signing

```bash
tenor keygen                              # Generate keypair (tenor-key.secret, tenor-key.public)
tenor sign bundle.json --key key.secret   # Sign bundle → bundle.signed.json
tenor verify bundle.signed.json           # Verify signature
```

Signing produces a detached attestation with: signer_public_key, signature (base64), algorithm ("ed25519"), signed_at timestamp, signed_etag (SHA-256 of bundle). The attestation is embedded in the interchange JSON under a top-level `attestation` key.

### WASM Bundle Signing

```bash
tenor sign-wasm evaluator.wasm --key key.secret --bundle-etag <etag>
tenor verify-wasm evaluator.wasm --sig evaluator.wasm.sig --pubkey key.public
```

Binds a WASM evaluator binary to a specific contract bundle via the etag. Prevents substitution attacks where a signed WASM module is used with a different contract.

### Manifest Trust Field

The TenorManifest includes an optional `trust` object:

```json
{
  "bundle_attestation": "<base64 signature>",
  "trust_domain": "<opaque identifier>",
  "attestation_format": "ed25519-detached"
}
```

The trust field is non-evaluating (ignored by the evaluator, consumed by auditors and operators).

### Provenance Trust Fields

Operation and flow provenance records can carry trust metadata:

- `trust_domain`: opaque string identifying the deployment boundary
- `attestation`: tamper-evident authenticity claim

### Executor Obligations

- **E18:** Executor capable of producing cryptographic attestation (activation optional)
- **E19:** Executor capable of associating provenance with tamper-evident authenticity claim
- **E20:** Executor MAY declare trust domain identifier (optional)

---

## 12. Agent Policies

**Source files:** `crates/eval/src/policy/`

### AgentPolicy Trait

Defines how an autonomous agent selects which flow to execute when multiple actions are available.

### Reference Policies

| Policy                 | Behavior                                                    |
| ---------------------- | ----------------------------------------------------------- |
| `AlwaysApprove`        | Approve every proposed action                               |
| `NeverApprove`         | Reject every proposed action                                |
| `FirstAvailablePolicy` | Select the first available flow in declaration order        |
| `PriorityPolicy`       | Select based on a priority ordering of flow IDs             |
| `RandomPolicy`         | Select uniformly at random (requires `interactive` feature) |

### HumanInTheLoopPolicy

Interactive approval via `ApprovalChannel` trait. Reference implementation reads from stdin. Supports timeout (returns rejection on expiry). `CallbackApprovalChannel` allows programmatic approval via message passing.

### LlmPolicy

AI-powered action selection. Requires `anthropic` feature flag. Components:

- `LlmClient` trait — abstract LLM interface
- `AnthropicClient` — Claude API implementation (via ureq HTTP client)
- Prompt construction: contract context, available actions, entity states → LLM → selected action
- Response parsing: extract flow_id and persona from LLM output
- Retry logic for malformed responses

### CompositePolicy

Three-stage pipeline: proposer (selects candidate), predicate (filters), approver (final gate). Allows combining policies — e.g., LLM proposes, rule-based predicate filters, human approves.

---

## 13. SDKs

### TypeScript SDK

**Location:** `sdks/typescript/`
**Package:** `@tenor/sdk`
**Mechanism:** WASM evaluator (wasm-bindgen, Node.js target)

**API surface:**

- `TenorEvaluator` — load contract, evaluate rules, execute flows, compute action space
- `TenorClient` — HTTP client for remote executor (evaluate, execute, simulate, action space, entity CRUD, history)
- Full TypeScript type definitions for all contract constructs, values, results, errors, action space

**Installation:**

```bash
npm install @tenor/sdk
```

**Usage:**

```typescript
import { TenorEvaluator } from "@tenor/sdk";

const evaluator = await TenorEvaluator.fromBundle(bundleJson);
const result = evaluator.evaluate({ cargo_weight_kg: 500 });
const flowResult = evaluator.executeFlow(
  "release_flow",
  "escrow_agent",
  facts,
  entityStates,
);
const actions = evaluator.computeActionSpace("buyer", facts, entityStates);
```

**Tests:** Via vitest

### Python SDK

**Location:** `sdks/python/`
**Package:** `tenor`
**Mechanism:** PyO3 native bindings (no WASM, compiled Rust extension)

**API surface:**

- `TenorEvaluator` — load contract, evaluate rules, execute flows
- Native Python types (dict, list, str, int, float, bool)

**Installation:**

```bash
pip install tenor  # or: maturin develop (for development)
```

**Usage:**

```python
from tenor import TenorEvaluator

evaluator = TenorEvaluator(bundle_json)
result = evaluator.evaluate({"cargo_weight_kg": 500})
flow_result = evaluator.execute_flow("release_flow", "escrow_agent", facts, entity_states)
```

**Build:** `maturin build` (requires Rust toolchain)

### Go SDK

**Location:** `sdks/go/`
**Module:** `github.com/riverline-labs/tenor-go`
**Mechanism:** Rust WASM bridge compiled to `wasm32-wasi`, loaded by wazero at runtime (no CGo)

**API surface:**

- `NewEvaluatorFromBundle(bundleJSON)` — create evaluator
- `Evaluate(facts)` — evaluate rules
- `ExecuteFlow(flowID, persona, facts, entityStates, instanceBindings)` — execute flow
- `ComputeActionSpace(persona, facts, entityStates)` — compute actions
- Go-native types and error handling

**Installation:**

```bash
go get github.com/riverline-labs/tenor-go
```

**Usage:**

```go
evaluator, err := tenor.NewEvaluatorFromBundle(bundleJSON)
result, err := evaluator.Evaluate(facts)
flowResult, err := evaluator.ExecuteFlow("release_flow", "escrow_agent", facts, states, bindings)
```

**Build:** WASM bridge built with `cargo build --target wasm32-wasi --release`

### Cross-SDK Conformance Suite

**Location:** `sdks/conformance/`
**Generator:** `sdks/conformance/fixture-gen/` (Rust binary)
**Runners:** `sdks/conformance/runners/go-runner/` (Go)

Generates contract + facts + expected output triples. Each SDK runner loads the contract, evaluates against facts, and asserts matching output. Ensures all SDK implementations produce identical results.

---

## 14. Automatic UI

**Source file:** `crates/cli/src/ui.rs`

`tenor ui` generates a complete React application from a contract's interchange JSON.

**Generated components:**

- Entity state viewers with transition history
- Fact input forms (type-aware: numeric inputs, date pickers, enum dropdowns, boolean toggles)
- Flow execution panels with persona selection
- Verdict display with provenance
- Action space dashboard
- Contract-derived theming (colors derived from contract structure)

**Generated files:**

- `src/` — React components, pages, hooks
- `src/api/` — API client (targeting spec-defined endpoints)
- `src/types/` — TypeScript types from contract
- Configuration files (package.json, tsconfig.json, vite.config.ts, tailwind)

**Usage:**

```bash
tenor ui contract.tenor --out ./my-app --api-url http://localhost:8080
cd my-app && npm install && npm run dev
```

**Flags:** `--contract-id`, `--theme` (custom theme file), `--title` (app title)

---

## 15. Builder

**Location:** `builder/`
**Architecture:** React 19 SPA with WASM evaluator (zustand state management, zundo undo/redo)

### Visual Editors

- **Entity state machine editor** — drag-and-drop states and transitions
- **Flow DAG editor** — visual step graph with outcome routing
- **Predicate builder** — structured expression construction (no raw syntax)
- **Rule editor** — stratum assignment, condition/produce clause editing
- **Operation editor** — persona assignment, effect declaration, outcome definition
- **Fact/Persona/Source editors** — form-based construct declaration
- **System editor** — multi-contract composition

### Simulation Mode

Live contract evaluation within the Builder. Enter fact values, see verdicts update in real time, step through flow execution, inspect entity state changes — all powered by the WASM evaluator running in the browser.

### Import/Export

- Import from `.tenor` DSL files or interchange JSON
- Export to `.tenor` DSL, interchange JSON, or WASM evaluator binary
- URL-based sharing (contract state encoded in URL)

### CLI

```bash
tenor builder dev                        # Start dev server (default: http://localhost:5173)
tenor builder build                      # Production build
tenor builder dev --contract file.tenor  # Pre-load contract
tenor builder dev --port 3000 --open     # Custom port, open browser
```

---

## 16. Hosted Platform

The private repo implements a multi-tenant hosted platform for Tenor contract execution. This section describes what the platform does at an architectural level; implementation details are not public.

### Multi-Tenancy

Organizations are isolated tenants. Each organization has API keys, contract deployments, and persona-to-identity mappings. Tenant isolation is enforced at the database level. Plan tiers (free, pro, enterprise) govern rate limits and usage quotas.

### Execution

The platform executor implements all 20 executor obligations (E1–E20) from the spec. Flow execution is atomic — all entity state transitions succeed or all fail. Every operation, transition, and rule evaluation is recorded with full provenance.

### Management

Organizations can create API keys, deploy contracts, map personas to API key identities, and manage contract lifecycle (provisioning → active → archived).

### Metering and Billing

Usage is tracked per organization per day across categories: evaluations, flow executions, simulations, entity instances, storage. Plan-tier limits are enforced (hard limits for free tier, soft limits for pro).

### Admin

A separate admin API and dashboard provide platform operators with system health monitoring, organization management, usage reporting, and deployment statistics.

### Contract Discovery

The platform serves TenorManifest at `/.well-known/tenor` per E10. Supports etag-based change detection (E12) and dry-run evaluation (E13).

---

## 17. Marketplace

### Template Format

A Tenor template is a packaged contract with metadata (`tenor-template.toml`):

- Contract name, version, description, category, tags
- Author organization
- Contract files (`.tenor` source + interchange JSON)

### CLI Commands

```bash
tenor pack                                # Package template from current directory
tenor publish                             # Publish to registry
tenor search "escrow"                     # Search templates
tenor search "escrow" --category finance  # Filter by category
tenor install escrow-release              # Install template locally
tenor deploy escrow-release --org my-org  # Deploy to hosted platform
```

### Registry

Templates are published to a registry API. Supports versioning, category/tag filtering, download tracking, and ratings. Published templates go through a review workflow before becoming publicly available.

### One-Click Deploy

The marketplace web UI provides browse, detail view, and a deployment wizard that provisions a contract on the hosted platform in a single step.

---

## 18. Infrastructure

The system is designed for a serverless deployment pattern:

- **Compute:** Serverless container instances that scale to zero when idle. The public repo's `tenor serve` runs as a stateless evaluator (no database required). The private platform requires a Postgres connection.
- **Database:** Serverless Postgres with branch-based environments (production, staging). Scales to zero when inactive.
- **CDN:** DNS, SSL/TLS termination, and static asset caching via edge network. HSTS with includeSubDomains, minimum TLS 1.2.
- **Container registry:** Container images built in CI and pushed to a registry for deployment.
- **Secrets:** Managed secret storage for database connection strings and API key hashing secrets.
- **CI/CD:** GitHub Actions with OIDC-based authentication to the cloud provider (no static service account keys).
- **Static hosting:** Builder SPA and documentation served from object storage behind CDN.

**Cost profile:** Near-zero at idle (serverless compute scales to zero, serverless database scales to zero, CDN free tier). Low single-digit dollars per month under light use.

---

## 19. CI/CD Pipeline

### Public Repo (`.github/workflows/ci.yml`)

**Triggers:** Push to `main`, pull requests to `main`
**Runner:** ubuntu-latest

**Steps (sequential):**

1. Checkout (actions/checkout@v4)
2. Install Rust toolchain (dtolnay/rust-toolchain@stable)
3. Rust cache (Swatinem/rust-cache@v2)
4. `cargo build --workspace`
5. `cargo run -p tenor-cli -- test conformance` (127 fixtures)
6. `cargo test --workspace` (849 tests)
7. `cargo fmt --all -- --check`
8. `cargo clippy --workspace -- -D warnings`
9. Install wasm-pack
10. `wasm-pack build --target nodejs` (crates/tenor-eval-wasm)
11. `wasm-pack test --node` (27 WASM tests)

**All checks must pass.** Clippy warnings are errors (`-D warnings`).

### Pre-Commit Hooks

Local pre-commit hooks run the same quality gates before every commit:

```bash
cargo fmt --all
cargo build --workspace
cargo test --workspace
cargo run -p tenor-cli -- test conformance
cargo clippy --workspace -- -D warnings
```

If the commit touches WASM-related crates, additionally:

```bash
cd crates/tenor-eval-wasm && wasm-pack build --target nodejs && wasm-pack test --node
```

---

## 20. CLI Command Reference

**Binary:** `tenor` (26 subcommands)
**Global flags:** `--output text|json` (default: text), `--quiet` (suppress non-essential output)

### Elaboration and Validation

| Command                                | Description                                          |
| -------------------------------------- | ---------------------------------------------------- |
| `tenor elaborate FILE`                 | Elaborate `.tenor` file to interchange JSON          |
| `tenor elaborate FILE --manifest`      | Generate TenorManifest with interchange bundle       |
| `tenor validate BUNDLE`                | Validate interchange JSON against formal JSON Schema |
| `tenor check FILE`                     | Run static analysis (S1–S8)                          |
| `tenor check FILE --analysis s1,s4,s6` | Run selected analyses                                |

### Evaluation

| Command                                                           | Description                  |
| ----------------------------------------------------------------- | ---------------------------- |
| `tenor eval BUNDLE --facts PATH`                                  | Evaluate rules against facts |
| `tenor eval BUNDLE --facts PATH --flow FLOW_ID --persona PERSONA` | Execute flow                 |

### Analysis and Migration

| Command                                          | Description                                |
| ------------------------------------------------ | ------------------------------------------ |
| `tenor diff V1 V2`                               | Structural diff of two interchange bundles |
| `tenor diff V1 V2 --breaking`                    | Classify changes as breaking/non-breaking  |
| `tenor migrate V1 V2`                            | Full migration analysis                    |
| `tenor migrate V1 V2 --yes`                      | Skip confirmation prompt                   |
| `tenor explain FILE`                             | Explain contract in natural language       |
| `tenor explain FILE --verbose --format markdown` | Detailed explanation                       |

### Code Generation

| Command                                                               | Description                                |
| --------------------------------------------------------------------- | ------------------------------------------ |
| `tenor generate typescript INPUT`                                     | Generate TypeScript types, schemas, client |
| `tenor generate typescript INPUT --out ./gen --sdk-import @tenor/sdk` | Custom output                              |

### Server and Interactive

| Command                                                         | Description                                |
| --------------------------------------------------------------- | ------------------------------------------ |
| `tenor serve [contracts...]`                                    | Start HTTP API server (default port: 8080) |
| `tenor serve --port 3000 --tls-cert cert.pem --tls-key key.pem` | TLS mode                                   |
| `tenor agent FILE`                                              | Interactive agent shell                    |

### Source Wiring

| Command                                          | Description                                      |
| ------------------------------------------------ | ------------------------------------------------ |
| `tenor connect CONTRACT`                         | Introspect sources, generate adapter scaffolding |
| `tenor connect CONTRACT --environment spec.yaml` | Match against OpenAPI/GraphQL/SQL                |
| `tenor connect CONTRACT --heuristic`             | Pattern matching (no LLM)                        |
| `tenor connect CONTRACT --batch review.json`     | Generate review file                             |
| `tenor connect CONTRACT --apply reviewed.json`   | Apply reviewed mappings                          |

### UI Generation

| Command                                                         | Description                |
| --------------------------------------------------------------- | -------------------------- |
| `tenor ui CONTRACT`                                             | Generate React application |
| `tenor ui CONTRACT --out ./app --api-url http://localhost:3000` | Custom output and API      |

### Builder

| Command                                          | Description               |
| ------------------------------------------------ | ------------------------- |
| `tenor builder dev`                              | Start Builder dev server  |
| `tenor builder build`                            | Production build          |
| `tenor builder dev --contract file.tenor --open` | Pre-load and open browser |

### Template Management

| Command                              | Description               |
| ------------------------------------ | ------------------------- |
| `tenor pack`                         | Package contract template |
| `tenor publish`                      | Publish to registry       |
| `tenor search QUERY`                 | Search templates          |
| `tenor install TEMPLATE`             | Install template locally  |
| `tenor deploy TEMPLATE --org ORG_ID` | Deploy to hosted platform |

### LSP

| Command     | Description                                      |
| ----------- | ------------------------------------------------ |
| `tenor lsp` | Start Language Server Protocol server over stdio |

### Cryptography

| Command                                                    | Description                      |
| ---------------------------------------------------------- | -------------------------------- |
| `tenor keygen`                                             | Generate Ed25519 signing keypair |
| `tenor sign BUNDLE --key secret.key`                       | Sign interchange bundle          |
| `tenor verify BUNDLE`                                      | Verify signed bundle             |
| `tenor sign-wasm WASM --key secret.key --bundle-etag ETAG` | Sign WASM binary                 |
| `tenor verify-wasm WASM --sig SIG --pubkey PUB`            | Verify WASM signature            |

### Testing

| Command                                             | Description                      |
| --------------------------------------------------- | -------------------------------- |
| `tenor test conformance`                            | Run elaborator conformance suite |
| `tenor ambiguity conformance/ --spec docs/tenor-language-specification.md` | AI ambiguity testing             |

---

## 21. Configuration Reference

### Environment Variables

| Variable               | Used By                                       | Default | Description                        |
| ---------------------- | --------------------------------------------- | ------- | ---------------------------------- |
| `ANTHROPIC_API_KEY`    | `tenor connect`, `tenor ambiguity`, LlmPolicy | (none)  | Anthropic API key for Claude       |
| `TENOR_REGISTRY_TOKEN` | `tenor publish`                               | (none)  | Auth token for template registry   |
| `TENOR_REGISTRY_URL`   | `tenor publish`, `tenor deploy`               | (none)  | Registry endpoint override         |
| `TENOR_PLATFORM_TOKEN` | `tenor deploy`                                | (none)  | Auth token for hosted platform     |
| `RUST_LOG`             | All crates                                    | (none)  | Logging level (tracing-subscriber) |

### Adapter Config (TOML)

Maps Source declarations to runtime connection details:

```toml
[global]
timeout_ms = "30000"

[sources.order_service]
base_url = "https://api.example.com/v2"
auth_header = "Bearer <token>"

[sources.compliance_db]
connection_string = "postgresql://user:pass@host/db"
```

Loaded via `--adapter-config` flag on `tenor serve` or platform serve command.

### Trust Config

Trust signing uses Ed25519 keypairs generated by `tenor keygen`:

- Secret key: `tenor-key.secret` (PEM-encoded Ed25519 private key)
- Public key: `tenor-key.public` (PEM-encoded Ed25519 public key)

Custom prefix: `tenor keygen --prefix my-signer`

### Cargo Features

| Crate        | Feature       | Default | Effect                                              |
| ------------ | ------------- | ------- | --------------------------------------------------- |
| `tenor-eval` | `adapter`     | Yes     | Enables fact adapter framework (tokio, ureq)        |
| `tenor-eval` | `interactive` | Yes     | Enables RandomPolicy (rand)                         |
| `tenor-eval` | `anthropic`   | No      | Enables AnthropicClient for LlmPolicy (ureq, tokio) |
| `tenor-cli`  | `tls`         | No      | Enables TLS for `tenor serve` (axum-server)         |

WASM and Python SDK builds use `default-features = false` to exclude tokio/ureq (not available in those environments).

---

## 22. Error Reference

### Elaboration Errors (ElabError)

All errors include: pass number, construct_kind, construct_id, field, file, line, message.

#### Pass 0/1 (Lex/Parse/Bundle)

| Message Pattern                                          | Cause                     |
| -------------------------------------------------------- | ------------------------- |
| `cannot open file '{}'`                                  | File read failure         |
| `import cycle detected`                                  | Cyclic import graph       |
| `cannot resolve import '{}'`                             | Import path doesn't exist |
| `import '{}' escapes the contract root directory`        | Sandbox violation         |
| `duplicate {} id '{}': first declared in {}`             | Cross-file duplicate      |
| `type library files may not contain import declarations` | Constraint violation      |
| `unterminated block comment`                             | Lexer error               |
| `expected '{}', got '{}'`                                | Parse error               |

#### Pass 2 (Index)

| Message Pattern                                   | Cause               |
| ------------------------------------------------- | ------------------- |
| `duplicate {} id '{}': first declared at line {}` | Same-kind duplicate |

#### Pass 3 (Types)

| Message Pattern                          | Cause                   |
| ---------------------------------------- | ----------------------- |
| `TypeDecl cycle detected: A → B → C → A` | Circular type reference |

#### Pass 4 (Typecheck)

| Message Pattern                                       | Cause                      |
| ----------------------------------------------------- | -------------------------- |
| `unknown type reference '{}'`                         | TypeRef not in environment |
| `unresolved fact reference: '{}' is not declared`     | Undeclared fact            |
| `operator '{}' not defined for Bool`                  | Operator/type mismatch     |
| `operator '{}' not defined for Text`                  | Ordering on Text           |
| `operator '{}' not defined for Enum`                  | Ordering on Enum           |
| `variable × variable multiplication is not permitted` | Forbidden operation        |
| `type error: product range ... not contained in ...`  | Int overflow               |

#### Pass 5 (Validate)

| Message Pattern                                                          | Cause                 |
| ------------------------------------------------------------------------ | --------------------- |
| `initial state '{}' is not declared in states: [...]`                    | Bad initial state     |
| `transition endpoint '{}' is not declared`                               | Invalid transition    |
| `Entity cycle detected: A → B → C → A`                                   | Parent DAG cycle      |
| `stratum must be a non-negative integer; got {}`                         | Bad stratum           |
| `unresolved VerdictType reference: '{}'`                                 | Missing verdict       |
| `stratum violation: rule at stratum N references verdict from stratum N` | Cross-stratum         |
| `duplicate outcome '{}'`                                                 | Non-unique outcome    |
| `allowed_personas must be non-empty`                                     | Missing personas      |
| `undeclared persona '{}'`                                                | Unresolved persona    |
| `effect references undeclared entity '{}'`                               | Bad effect            |
| `entry step '{}' is not declared in steps`                               | Missing entry         |
| `OperationStep must declare a FailureHandler`                            | Missing handler       |
| `source '{}' with protocol '{}' is missing required field '{}'`          | Missing field         |
| `invalid extension protocol tag '{}'`                                    | Bad extension tag     |
| `unknown protocol tag '{}'`                                              | Unrecognized protocol |

### Evaluation Errors (EvalError)

| Variant           | Fields                   | Cause                                       |
| ----------------- | ------------------------ | ------------------------------------------- |
| `MissingFact`     | fact_id                  | Required fact not provided and no default   |
| `TypeMismatch`    | fact_id, expected, got   | Value doesn't match declared type           |
| `Overflow`        | message                  | Numeric computation overflow                |
| `InvalidOperator` | op                       | Unsupported operator                        |
| `UnknownFact`     | fact_id                  | Fact reference in predicate doesn't resolve |
| `UnknownVerdict`  | verdict_type             | Verdict reference doesn't resolve           |
| `TypeError`       | message                  | Expression type error at runtime            |
| `ListOverflow`    | fact_id, max, actual     | List exceeds declared max                   |
| `InvalidEnum`     | fact_id, value, variants | Enum value not in declared set              |
| `NotARecord`      | message                  | Field access on non-Record                  |
| `UnboundVariable` | name                     | Quantifier variable not in scope            |
| `FlowError`       | flow_id, message         | Flow execution failure                      |

### Operation Errors (OperationError)

| Variant                                                           | Cause                                     |
| ----------------------------------------------------------------- | ----------------------------------------- |
| `PersonaRejected { operation_id, persona }`                       | Persona not in allowed_personas           |
| `PreconditionFailed { operation_id, condition_desc }`             | Precondition evaluated false              |
| `InvalidEntityState { entity_id, instance_id, expected, actual }` | Current state doesn't match effect source |
| `EntityNotFound { entity_id, instance_id }`                       | Instance not in EntityStateMap            |

### Migration Errors (MigrationError)

| Variant                                                     | Cause                               |
| ----------------------------------------------------------- | ----------------------------------- |
| `Diff(DiffError)`                                           | Bundle comparison failure           |
| `Deserialize(String)`                                       | Bundle deserialization failure      |
| `Analysis(String)`                                          | Compatibility analysis failure      |
| `StateMismatch { entity_id, instance_id, expected, found }` | Entity state doesn't match expected |
| `Storage(String)`                                           | Storage backend failure             |
| `Incompatible(String)`                                      | Breaking change without policy      |

---

## 23. Test Suite Summary

| Suite                 | Location                       | Count      | What It Tests                                                                                                        |
| --------------------- | ------------------------------ | ---------- | -------------------------------------------------------------------------------------------------------------------- |
| Workspace unit tests  | `crates/*/src/**`              | 849        | All Rust crate internals                                                                                             |
| Conformance suite     | `conformance/`                 | 127        | Elaborator correctness (positive, negative, numeric, promotion, shorthand, cross-file, parallel, analysis, manifest) |
| WASM tests            | `crates/tenor-eval-wasm/`      | 27         | WASM evaluator (load, evaluate, flow, action space, inspect)                                                         |
| TypeScript SDK        | `sdks/typescript/`             | vitest     | WASM evaluator wrapper, HTTP client                                                                                  |
| Python SDK            | `sdks/python/`                 | pytest     | PyO3 bindings                                                                                                        |
| Go SDK                | `sdks/go/`                     | go test    | Wazero WASM bridge                                                                                                   |
| Cross-SDK conformance | `sdks/conformance/`            | fixtures   | Identical output across all SDK implementations                                                                      |
| Executor conformance  | `crates/executor-conformance/` | E1–E20     | Executor obligation compliance                                                                                       |
| Builder               | `builder/`                     | vitest     | React component tests                                                                                                |
| CLI integration       | `crates/cli/` (dev-deps)       | assert_cmd | CLI binary integration tests                                                                                         |
| Storage conformance   | `crates/storage/`              | doctest    | TenorStorage trait contract                                                                                          |

**Total: 849 workspace + 127 conformance + 27 WASM = 1,003 tests**

---

## 24. Glossary

Terms from the Tenor specification (§22) plus implementation-specific additions.

| Term                         | Definition                                                                                                                      |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| **Action Space**             | The set of flows a persona can initiate given current facts, entity states, and verdicts. Computed by `compute_action_space()`. |
| **Adapter**                  | Runtime component that fetches Fact values from external systems per Source declarations.                                       |
| **AdapterConfig**            | TOML configuration mapping Source IDs to connection details (base URLs, credentials, etc.).                                     |
| **AdapterRegistry**          | Registry mapping Source IDs to configured FactAdapter instances.                                                                |
| **Agent**                    | Software reading Tenor contracts to understand system behavior, or an autonomous execution loop.                                |
| **AgentPolicy**              | Trait defining how an autonomous agent selects which flow to execute.                                                           |
| **Attestation**              | Cryptographic claim binding content to signer (Ed25519, evaluator-ignored).                                                     |
| **BaseType**                 | One of twelve primitive value types.                                                                                            |
| **Bundle**                   | Top-level elaborator output — canonical interchange JSON.                                                                       |
| **CodegenBundle**            | Deserialized interchange JSON for code generation consumption.                                                                  |
| **Cold-Start**               | Agent discovery sequence: one fetch of `/.well-known/tenor` provides complete bundle.                                           |
| **Compensate**               | Failure handler that runs a sequence of Operations to undo partial work.                                                        |
| **Conformance Suite**        | 127 test fixtures validating elaborator correctness.                                                                            |
| **Construct**                | Top-level language declaration: Fact, Entity, Rule, Persona, Operation, Flow, Source, System.                                   |
| **Contract**                 | Complete Tenor specification in `.tenor` files.                                                                                 |
| **Dry-Run**                  | Read-only Operation evaluation (steps 1–3, no effects). Response carries `"simulation": true`.                                  |
| **Effect**                   | Entity state transition declared on an Operation.                                                                               |
| **ElabError**                | Elaboration error type with pass, construct_kind, construct_id, field, file, line, message.                                     |
| **Elaboration**              | Six-pass transformation from `.tenor` source to interchange JSON.                                                               |
| **Elaborator**               | The reference tool implementing the six-pass pipeline (`tenor-core`).                                                           |
| **EnrichedFactProvenance**   | Extended provenance tracing fact origin through adapter fetch.                                                                  |
| **Entity**                   | Finite state machine type with states, initial state, and transitions.                                                          |
| **EntityStateMap**           | Runtime mapping `(EntityId, InstanceId) → StateId`.                                                                             |
| **Etag**                     | SHA-256 hex digest of canonical bundle bytes. Changes iff bundle changes.                                                       |
| **EvalError**                | Evaluator error type (MissingFact, TypeMismatch, Overflow, etc.).                                                               |
| **Executor**                 | Runtime system that evaluates contracts and applies state transitions.                                                          |
| **Fact**                     | Ground external input value with type, source, and optional default.                                                            |
| **FactProvider**             | Trait for supplying fact values to the evaluator.                                                                               |
| **FactSet**                  | Assembled facts ready for evaluation.                                                                                           |
| **Flow**                     | Orchestration DAG of steps with frozen snapshot semantics.                                                                      |
| **FlowEvalResult**           | Combined result: verdicts + flow execution outcome.                                                                             |
| **Frozen Verdict Semantics** | Guarantee that the verdict set is computed once at Flow initiation and never recomputed.                                        |
| **Index**                    | Pass 2 output: HashMap-based lookup by (construct_kind, id).                                                                    |
| **InstanceBindingMap**       | Mapping `EntityId → InstanceId` for flow execution targeting.                                                                   |
| **InstanceId**               | Unique identifier for an entity instance (opaque UTF-8 string).                                                                 |
| **Interchange Format**       | Canonical JSON schema produced by Pass 6, consumed by evaluators and tooling.                                                   |
| **LlmPolicy**                | Agent policy using Claude API for action selection.                                                                             |
| **Manifest**                 | TenorManifest wrapping interchange bundle with etag and capabilities.                                                           |
| **NumericModel**             | Fixed-point decimal specification: 28 max digits, round-half-to-even, no floating-point.                                        |
| **Operation**                | Persona-gated, precondition-guarded unit of work producing entity state transitions.                                            |
| **OperationError**           | Operation execution error (PersonaRejected, PreconditionFailed, etc.).                                                          |
| **Outcome**                  | Named success-path result label on an Operation.                                                                                |
| **Pass**                     | One stage of the six-pass elaboration pipeline.                                                                                 |
| **Persona**                  | Opaque identity token representing an actor class.                                                                              |
| **Precondition**             | PredicateExpression on an Operation that must be true for execution.                                                            |
| **PredicateExpression**      | Quantifier-free first-order logic formula over ground terms.                                                                    |
| **ProtocolTag**              | Source protocol identifier: `http`, `database`, `graphql`, `grpc`, `static`, `manual`, or `x_*`.                                |
| **Provenance**               | Complete derivation chain for a verdict or operation result.                                                                    |
| **RawConstruct**             | AST node representing a parsed construct before type resolution.                                                                |
| **RawType**                  | AST type node before TypeRef resolution.                                                                                        |
| **ResolvedVerdictSet**       | Set of all verdicts produced by evaluating all rules.                                                                           |
| **Rule**                     | Stratified verdict-producing declaration with `when` predicate and `produce` clause.                                            |
| **Snapshot**                 | Frozen FactSet + VerdictSet captured at Flow initiation.                                                                        |
| **Source**                   | Named declaration of an external system (protocol, fields, description).                                                        |
| **SourcePath**               | Syntactically validated path string identifying a location in an external system.                                               |
| **StaticFactProvider**       | FactProvider implementation that returns pre-loaded values.                                                                     |
| **Stratum**                  | Rule evaluation level (0 to max). Higher strata can reference lower-stratum verdicts.                                           |
| **StructuredSource**         | Fact source reference to a declared Source construct with path.                                                                 |
| **System**                   | Multi-contract composition construct with triggers and shared personas.                                                         |
| **TenorClient**              | TypeScript SDK HTTP client for remote executor communication.                                                                   |
| **TenorEvaluator**           | SDK class wrapping the WASM evaluator for local contract evaluation.                                                            |
| **TenorManifest**            | Discovery document: `{ tenor, bundle, etag, capabilities?, trust? }`.                                                           |
| **TenorStorage**             | Async trait for storage backends (entity state, flow history, provenance).                                                      |
| **Transition**               | Permitted state change in an entity's transition relation.                                                                      |
| **Trust Domain**             | Opaque deployment boundary identifier (E20, optional).                                                                          |
| **TypeDecl**                 | Named type alias for Record or TaggedUnion (resolved and inlined during elaboration).                                           |
| **TypeEnv**                  | Pass 3 output: `HashMap<String, RawType>` mapping type names to resolved types.                                                 |
| **Verdict**                  | Rule-produced value with type, payload, and provenance.                                                                         |
| **VerdictType**              | Verdict category/name (produced by exactly one rule per S8).                                                                    |

---

_Key constants: `TENOR_VERSION = "1.0"`, `TENOR_BUNDLE_VERSION = "1.0.0"`_

_Test totals: 849 workspace + 127 conformance + 27 WASM = 1,003_
