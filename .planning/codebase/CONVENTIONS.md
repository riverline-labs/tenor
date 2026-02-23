# Coding Conventions

**Analysis Date:** 2026-02-22

## Naming Patterns

**Files:**
- Snake case for all Rust source files: `pass1_bundle.rs`, `pass4_typecheck.rs`, `s3a_admissibility.rs`
- Prefix with pass number for elaborator passes: `pass1_bundle.rs` ... `pass6_serialize.rs`
- Prefix with stage number for analysis modules: `s1_state_space.rs` ... `s8_verdict_uniqueness.rs`
- Integration test files named by purpose: `schema_validation.rs`, `cli_integration.rs`, `conformance.rs`, `numeric_regression.rs`

**Types and Structs:**
- PascalCase for types, structs, enums: `ElabError`, `RawConstruct`, `RawType`, `EvalError`, `VerdictSet`
- `Raw` prefix for AST types before elaboration: `RawConstruct`, `RawExpr`, `RawType`, `RawLiteral`, `RawTerm`
- `Raw` prefix also for sub-types: `RawStep`, `RawStepTarget`, `RawFailureHandler`, `RawBranch`, `RawTrigger`

**Functions:**
- Snake case for all functions and methods: `load_bundle`, `build_index`, `build_type_env`, `resolve_types`
- Pass entry points named `build_*` or `load_*` or `analyze_*` or `validate_*`: `build_index()`, `build_type_env()`, `load_bundle()`, `validate()`, `analyze_state_space()`
- Helper predicates named `is_*` or `has_*`: `is_word()`, `has_verdict()`
- Constructors use `new()` or `from_interchange()`: `Parser::new()`, `Tap::new()`, `Contract::from_interchange()`

**Variables:**
- Snake case throughout: `root_path`, `bundle_id`, `type_env`, `fact_set`
- Abbreviations used consistently: `prov` for `Provenance`, `c` for construct loop variables, `e` for errors in closures

**Enum Variants:**
- PascalCase: `Token::Word`, `RawConstruct::Fact`, `EvalError::MissingFact`, `OutputFormat::Json`
- Struct-like variants for complex payloads with named fields:
  ```rust
  EvalError::MissingFact { fact_id: String }
  EvalError::TypeMismatch { fact_id, expected, got }
  RawConstruct::Fact { id, type_, source, default, prov }
  ```
- Field named `type_` (not `type`) to avoid keyword clash: `Fact { type_: RawType, .. }`

## Code Style

**Formatting:**
- `cargo fmt --all` required before every commit (CI enforces this)
- Standard rustfmt configuration (no `rustfmt.toml` found - uses defaults)

**Linting:**
- `cargo clippy --workspace -- -D warnings` required before every commit
- Warnings promoted to errors in CI: `-D warnings`
- One workspace-level allow in `crates/core/src/lib.rs`:
  ```rust
  #![allow(clippy::result_large_err)]
  ```

## Import Organization

**Order:**
1. `crate::` imports (local crate modules)
2. `std::` imports (standard library)
3. Third-party crate imports (serde, serde_json, etc.)

**Pattern examples from production code:**
```rust
use crate::ast::*;
use crate::error::ElabError;
use crate::pass3_types::TypeEnv;
use std::collections::{BTreeMap, HashMap, HashSet};
```

```rust
use crate::error::ElabError;
use crate::pass1_bundle;
use crate::pass2_index;
use serde_json::Value;
use std::path::Path;
```

**Glob imports:**
- `use crate::ast::*` is used in pass modules to import all AST types at once
- Avoid glob imports from external crates

**Re-exports:**
- `crates/core/src/lib.rs` re-exports key public types and pass entry functions for consumers
- `parser.rs` re-exports AST types from `ast.rs` for backward compatibility:
  ```rust
  pub use crate::ast::{Provenance, RawBranch, ...};
  ```

## Error Handling

**Pattern:**
- All fallible operations return `Result<T, ElabError>` or `Result<T, EvalError>`
- Use `?` operator for propagation throughout
- Errors constructed with specific constructors:
  ```rust
  ElabError::new(pass, construct_kind, construct_id, field, file, line, message)
  ElabError::lex(file, line, message)
  ElabError::parse(file, line, message)
  ```
- Error messages are human-readable strings; format strings used inline:
  ```rust
  format!("cannot open file: {}", e)
  format!("duplicate {} id '{}': first declared in {}", kind, id, first.file)
  ```
- `.map_err(|e| ElabError::new(...))` used at I/O boundaries (file reads, canonicalize)

**`unwrap()` policy:**
- `unwrap()` is allowed only when invariants have been explicitly proven in code comments
- SAFETY comments document why `unwrap()` cannot panic:
  ```rust
  // SAFETY: name was just detected in in_stack by the contains() check above
  let pos = in_stack.iter().position(|x| x == name).unwrap();
  ```
- In tests, `unwrap()` and `unwrap_or_else(|e| panic!(...))` are standard

**`unwrap_or_else` with panic in tests:**
```rust
let bundle = tenor_core::elaborate::elaborate(&tenor_path)
    .unwrap_or_else(|e| panic!("Failed to elaborate {}: {:?}", name, e));
```

## Module Design

**Structure:**
- Each elaborator pass is a self-contained module in `crates/core/src/`
- Public entry function at module top level (`pub fn load_bundle(...)`, `pub fn build_index(...)`)
- Private helper functions within the module
- `lib.rs` declares all modules public and re-exports key symbols

**Exports:**
- `pub mod` for all modules
- Selective `pub use` re-exports in `lib.rs` for the crate's public API
- Each crate's `lib.rs` defines the external API surface

**`mod.rs` pattern:**
- Not used; each module is a single `.rs` file or directory module with explicit `mod.rs`
- The `ambiguity/` subdirectory uses `mod.rs`: `crates/cli/src/ambiguity/mod.rs`

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

**Inline comments:**
- Used for non-obvious logic, SAFETY invariants, and spec references
- Spec section references noted inline: `// Per spec Section 11.4:`

**Doc comments on public items:**
- Public functions and structs carry `///` doc comments explaining purpose and arguments
- Full rustdoc on `lib.rs` public API with `#`, `#Arguments`, `#Returns` sections

## Logging

**No logging framework** - the codebase uses no logging crate (`log`, `tracing`, etc.).
Output goes to stdout/stderr directly:
- `println!` for TAP test output (`crates/cli/src/tap.rs`)
- `eprintln!` for user-facing diagnostics in CLI
- `eprint!` for error output on failure

## Function Design

**Size:** Functions are kept focused; long match arms extract into named helpers
- `validate()` in `pass5_validate.rs` dispatches to `validate_entity()`, `validate_rule()`, etc.
- `run_suite()` in `runner.rs` dispatches to `run_positive_dir()`, `run_negative_tests()`, etc.

**Parameters:**
- Prefer `&str` over `&String` for string inputs
- `impl Into<String>` for message parameters (allows both `&str` and `String` callers)
- `&Path` over `&PathBuf` at function boundaries
- Pass large structs/collections by reference

**Return values:**
- `Result<T, Error>` for fallible operations
- Unit `()` for infallible mutations (accumulator patterns)

## BTreeMap vs HashMap

- `BTreeMap` used for ordered collections where iteration order matters (AST fields, construct indexes, fixture output)
- `HashMap` used for unordered lookups (type environments, duplicate detection)
- `HashSet` used for visited sets in graph traversals

---

*Convention analysis: 2026-02-22*
