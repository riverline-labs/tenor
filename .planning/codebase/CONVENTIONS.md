# Coding Conventions

**Analysis Date:** 2026-02-23

## Naming Patterns

**Files:**
- Snake case for all Rust source files: `pass1_bundle.rs`, `pass4_typecheck.rs`, `s3a_admissibility.rs`
- Elaborator pass modules prefixed with pass number: `pass1_bundle.rs`, `pass2_index.rs`, `pass3_types.rs`, `pass4_typecheck.rs`, `pass5_validate.rs`, `pass6_serialize.rs`
- Static analysis modules prefixed with stage number: `s1_state_space.rs`, `s2_reachability.rs`, `s3a_admissibility.rs`, `s4_authority.rs`, `s5_verdicts.rs`, `s6_flow_paths.rs`, `s7_complexity.rs`, `s8_verdict_uniqueness.rs`
- Integration test files named by purpose: `schema_validation.rs`, `cli_integration.rs`, `conformance.rs`, `numeric_regression.rs`, `analysis_tests.rs`, `codegen_integration.rs`, `serve_integration.rs`

**Types and Structs:**
- PascalCase for all types, structs, enums: `ElabError`, `RawConstruct`, `RawType`, `EvalError`, `VerdictSet`, `AnalysisReport`
- `Raw` prefix for all AST types before elaboration: `RawConstruct`, `RawExpr`, `RawType`, `RawLiteral`, `RawTerm`, `RawStep`, `RawStepTarget`, `RawFailureHandler`, `RawBranch`, `RawTrigger`, `RawJoinPolicy`, `RawCompStep`
- No prefix for elaborated/runtime types: `Contract`, `FactSet`, `VerdictSet`, `Index`, `TypeEnv`

**Functions:**
- Snake case for all functions and methods: `load_bundle`, `build_index`, `build_type_env`, `resolve_types`
- Pass entry points: `load_bundle()`, `build_index()`, `build_type_env()`, `resolve_types()`, `validate()`, `serialize()`
- Analysis entry points: `analyze()`, `analyze_selected()`, `analyze_state_space()`, `analyze_reachability()`
- Predicate functions: `is_word()`, `has_verdict()`, `has_dead_states`
- Constructors: `Parser::new()`, `Tap::new()`, `Contract::from_interchange()`
- Conversion/serialization: `to_json_value()`, `to_json()`, `to_text()`

**Variables:**
- Snake case throughout: `root_path`, `bundle_id`, `type_env`, `fact_set`, `verdict_set`
- Abbreviations used consistently: `prov` for `Provenance`, `c` for construct loop variables, `e` for errors in closures, `ct` for comparison_type, `env` for type environment
- Index variables: `idx` for `Index`, `pos` for parser position

**Enum Variants:**
- PascalCase: `Token::Word`, `RawConstruct::Fact`, `EvalError::MissingFact`, `OutputFormat::Json`
- Struct-like variants for complex payloads with named fields:
  ```rust
  EvalError::MissingFact { fact_id: String }
  EvalError::TypeMismatch { fact_id, expected, got }
  RawConstruct::Fact { id, type_, source, default, prov }
  ```
- Field named `type_` (not `type`) to avoid Rust keyword clash:
  ```rust
  RawConstruct::Fact { id: String, type_: RawType, source: String, default: Option<RawLiteral>, prov: Provenance }
  ```

**Constants:**
- SCREAMING_SNAKE_CASE: `TENOR_VERSION`, `TENOR_BUNDLE_VERSION`
- Static strings for embedded resources: `INTERCHANGE_SCHEMA_STR`, `MANIFEST_SCHEMA_STR`
- Defined in `crates/core/src/lib.rs`:
  ```rust
  pub const TENOR_VERSION: &str = "1.0";
  pub const TENOR_BUNDLE_VERSION: &str = "1.1.0";
  ```

## Code Style

**Formatting:**
- `cargo fmt --all` required before every commit (CI enforces via `cargo fmt --all -- --check`)
- Standard rustfmt configuration (no `rustfmt.toml` -- uses defaults)
- Max line width: default rustfmt (100 characters)

**Linting:**
- `cargo clippy --workspace -- -D warnings` required before every commit
- Warnings promoted to errors in CI: `-D warnings`
- One workspace-level allow in `crates/core/src/lib.rs`:
  ```rust
  #![allow(clippy::result_large_err)]
  ```
  Rationale: `ElabError` is a large error type (contains String fields), but returning it by value is intentional for the compiler pipeline.

## Import Organization

**Order (observed in production code):**
1. `crate::` imports (local crate modules)
2. External crate imports (serde, serde_json, clap, etc.)
3. `std::` imports (standard library)

**Pattern examples:**
```rust
// From crates/core/src/pass1_bundle.rs
use crate::ast::*;
use crate::error::ElabError;
use crate::lexer;
use crate::parser;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
```

```rust
// From crates/core/src/elaborate.rs
use crate::error::ElabError;
use crate::pass1_bundle;
use crate::pass2_index;
use crate::pass3_types;
use crate::pass4_typecheck;
use crate::pass5_validate;
use crate::pass6_serialize;
use serde_json::Value;
use std::path::Path;
```

**Glob imports:**
- `use crate::ast::*` is used in pass modules to import all AST types at once
- Never use glob imports from external crates
- Re-export AST types from `parser.rs` for backward compatibility:
  ```rust
  pub use crate::ast::{Provenance, RawBranch, RawCompStep, RawConstruct, ...};
  ```

**Re-exports in lib.rs:**
```rust
// crates/core/src/lib.rs
pub use ast::{Provenance, RawConstruct, RawExpr, RawLiteral, RawTerm, RawType};
pub use error::ElabError;
pub use pass2_index::Index;
pub use pass3_types::TypeEnv;
pub use elaborate::elaborate;
pub use pass1_bundle::load_bundle;
pub use pass2_index::build_index;
pub use pass3_types::build_type_env;
pub use pass4_typecheck::resolve_types;
```

## Error Handling

**Elaborator errors (`ElabError`):**
- Defined in `crates/core/src/error.rs`
- All fields required: `pass`, `construct_kind`, `construct_id`, `field`, `file`, `line`, `message`
- Constructors with specific helpers:
  ```rust
  ElabError::new(pass, construct_kind, construct_id, field, file, line, message)
  ElabError::lex(file, line, message)    // pass=0
  ElabError::parse(file, line, message)  // pass=0
  ```
- Serialization via `to_json_value()` for JSON output (always includes all fields, null for missing)
- Error messages are human-readable, use `format!()` inline:
  ```rust
  format!("cannot open file: {}", e)
  format!("duplicate {} id '{}': first declared in {}", kind, id, first.file)
  format!("initial state '{}' is not declared in states: [{}]", initial, states_list)
  ```

**Evaluator errors (`EvalError`):**
- Defined in `crates/eval/src/types.rs`
- Uses enum with struct-like variants:
  ```rust
  enum EvalError {
      MissingFact { fact_id: String },
      TypeMismatch { fact_id: String, expected: String, got: String },
      DeserializeError { message: String },
      // ...
  }
  ```

**Error propagation:**
- Use `?` operator throughout for `Result` propagation
- `.map_err(|e| ElabError::new(...))` at I/O boundaries (file reads, path canonicalization)
- In CLI `main.rs`, match on `Result` and call `process::exit(1)` on errors

**`unwrap()` policy:**
- `unwrap()` is allowed only when invariants have been explicitly proven
- Use `.expect("descriptive message")` when the invariant is documented by the message:
  ```rust
  .expect("name must be in in_stack when contains() returned true")
  .expect("workspace root")
  ```
- In tests, `unwrap()` and `unwrap_or_else(|e| panic!("...", e))` are standard:
  ```rust
  let bundle = tenor_core::elaborate::elaborate(&tenor_path)
      .unwrap_or_else(|e| panic!("Failed to elaborate {}: {:?}", name, e));
  ```

## Module Design

**Structure:**
- Each elaborator pass is a self-contained module in `crates/core/src/`
- Each static analysis is a self-contained module in `crates/analyze/src/`
- Each evaluator concern is a self-contained module in `crates/eval/src/`
- Public entry function at module top level: `pub fn load_bundle(...)`, `pub fn build_index(...)`, `pub fn analyze(...)`
- Private helper functions within the module
- `lib.rs` declares all modules `pub` and re-exports key symbols

**Exports:**
- `pub mod` for all submodules in `lib.rs`
- Selective `pub use` re-exports for the crate's public API surface
- Each crate's `lib.rs` defines the external API surface

**Directory modules:**
- `crates/cli/src/ambiguity/` uses `mod.rs` pattern: `crates/cli/src/ambiguity/mod.rs`
- All other modules are single `.rs` files

## Serialization Conventions

**JSON interchange format (canonical):**
- All JSON keys sorted lexicographically within each object
- Constructs sorted by id within kind groups
- Construct order: Facts, Entities, Personas, Rules (by stratum then id), Operations, Flows, Systems
- Bundle envelope: `{ "constructs": [...], "id": "...", "kind": "Bundle", "tenor": "1.0", "tenor_version": "1.1.0" }`

**Numeric value serialization:**
- Decimal values use structured representation:
  ```json
  { "kind": "decimal_value", "precision": 10, "scale": 2, "value": "100.50" }
  ```
- Money values use structured representation:
  ```json
  { "kind": "money_value", "currency": "USD", "amount": { "kind": "decimal_value", "precision": 10, "scale": 2, "value": "0.00" } }
  ```
- Precision and scale from the **declared type**, not inferred from the literal string

**Multiplication in interchange:**
```json
{ "left": { "fact_ref": "x" }, "literal": 10, "op": "*", "result_type": { "base": "Int", "min": 0, "max": 1000 } }
```

**`comparison_type` emission:**
- Emitted on Compare nodes for: Money (always), Int x Decimal cross-type, Mul x Int
- Tells the evaluator how to promote/compare values

**Provenance on every construct:**
```json
{ "provenance": { "file": "example.tenor", "line": 5 } }
```

**Source field decomposition:**
- Source string `"system_name.field_name"` is split into `{ "system": "system_name", "field": "field_name" }`

## DSL Conventions

**Keywords are lowercase.** The parser expects:
```
fact, entity, rule, operation, flow, type, persona, system, import
```

**Uppercase in interchange JSON `"kind"` values:**
```json
"kind": "Fact", "kind": "Entity", "kind": "Rule", "kind": "Bundle"
```

**Transition syntax accepts multiple forms:**
- `(from, to)` -- comma separator
- `(from -> to)` -- ASCII arrow
- `(from → to)` -- Unicode arrow (U+2192)

**Unicode logical operators supported:**
- `∧` (U+2227) for AND
- `∨` (U+2228) for OR
- `¬` (U+00AC) for NOT
- `∀` (U+2200) for FORALL
- `∃` (U+2203) for EXISTS
- `∈` (U+2208) for IN

## Comments

**Module-level doc comments:**
- Every module starts with a `//!` doc comment explaining purpose:
  ```rust
  //! Pass 0+1: Lex, parse, import resolution, cycle detection, bundle assembly.
  //! Six-pass elaborator: Tenor -> TenorInterchange JSON bundle.
  //! Shared AST types for the Tenor elaborator.
  ```

**Inline section separators:**
- Visual separators group related items within a file:
  ```rust
  // ──────────────────────────────────────────────
  // Provenance
  // ──────────────────────────────────────────────
  ```

**Spec references:**
- Referenced inline when implementing spec requirements:
  ```rust
  // Per spec Section 11.4: initiating_persona is recorded for provenance.
  // Per spec Section 12: NumericModel
  ```

**Doc comments on public items:**
- Public functions and structs carry `///` doc comments explaining purpose
- `#[doc]` attributes and `# Arguments`, `# Returns` sections on public API

## Logging

**No logging framework.** The codebase uses no logging crate (`log`, `tracing`, etc.).

Output goes to stdout/stderr directly:
- `println!` for TAP test output (`crates/cli/src/tap.rs`), successful CLI results
- `eprintln!` for user-facing error diagnostics in CLI
- No debug logging in library crates (`core`, `eval`, `analyze`, `codegen`)

## Function Design

**Size:** Functions are focused; long match arms extract into named helpers:
- `validate()` dispatches to `validate_entity()`, `validate_rule()`, etc.
- `run_suite()` dispatches to `run_positive_dir()`, `run_negative_tests()`, etc.
- `serialize()` dispatches to `serialize_construct()` per construct kind

**Parameters:**
- Use `&str` over `&String` for string inputs
- Use `impl Into<String>` for message parameters (allows both `&str` and `String`)
- Use `&Path` over `&PathBuf` at function boundaries
- Pass large structs/collections by reference
- Use `Option<&str>` for optional string params (not `Option<String>`)

**Return values:**
- `Result<T, Error>` for all fallible operations
- Unit `()` for infallible mutations (accumulator patterns like `Tap::ok()`)
- `Value` (serde_json) for JSON output from serialization

## Collection Choices

- `BTreeMap` for ordered collections where iteration order matters (AST fields, construct indexes, serialized output keys). Ensures deterministic JSON output.
- `HashMap` for unordered lookups (type environments, duplicate detection in indexing)
- `HashSet` for visited sets in graph traversals (import cycle detection, type cycle detection)
- `Vec` with `.sort()` for construct lists that need deterministic ordering
- `VecDeque` used in `pass6_serialize.rs` for flow step breadth-first traversal

## CLI Output Conventions

**Dual output format:** All CLI commands support `--output text` (default) and `--output json`
- Text mode: human-readable output to stdout, errors to stderr
- JSON mode: machine-parseable JSON to stdout, errors as JSON to stderr
- `--quiet` flag suppresses non-essential output

**Error reporting helper:**
```rust
fn report_error(msg: &str, output: OutputFormat, quiet: bool) {
    if quiet { return; }
    match output {
        OutputFormat::Text => eprintln!("{}", msg),
        OutputFormat::Json => eprintln!("{{\"error\": \"{}\"}}", msg.replace('"', "\\\"")),
    }
}
```

**Exit codes:**
- 0: success
- 1: elaboration/evaluation/validation/diff error
- 2: CLI argument error (from clap)

## Workspace and Crate Dependencies

**Inter-crate dependency direction:**
```
tenor-cli -> tenor-core, tenor-eval, tenor-analyze, tenor-codegen, tenor-lsp
tenor-eval -> tenor-core
tenor-analyze -> tenor-core
tenor-codegen -> tenor-core (via interchange JSON, not direct Rust type dependency)
tenor-lsp -> tenor-core
```

**Workspace dependencies:** Shared dependency versions declared in root `Cargo.toml` `[workspace.dependencies]` and consumed via `{ workspace = true }` in each crate.

---

*Convention analysis: 2026-02-23*
