# Coding Conventions

**Analysis Date:** 2026-02-25

## Naming Patterns

**Files:**
- Snake case for file names: `pass1_bundle.rs`, `pass2_index.rs`, `pass4_typecheck.rs`
- Pass modules use `passN_description` pattern: `pass1_bundle.rs`, `pass2_index.rs`, etc.
- Test files use crate name + `_tests.rs` or `_integration.rs`: `cli_integration.rs`, `schema_validation.rs`, `conformance.rs`
- Helper modules use descriptive names: `runner.rs`, `tap.rs`, `manifest.rs`, `serve.rs`, `agent.rs`, `diff.rs`, `explain.rs`

**Functions:**
- Snake case for all function names: `build_index()`, `elaborate()`, `load_bundle()`, `validate_all_positive_conformance_outputs_against_schema()`
- Functions starting with `cmd_` for CLI command handlers: `cmd_elaborate()`, `cmd_validate()`, `cmd_test()`
- Functions prefixed with `run_` for test runners: `run_eval_fixture()`, `run_eval_fixture_error()`, `run_eval_flow_fixture()`
- Helper functions prefixed with `workspace_root()`, `tenor()` in test utilities
- Internal helper functions use simple descriptive names: `validate_file()`, `collect_expected_json_files()`, `check_cross_file_dups()`

**Variables:**
- Snake case throughout: `bundle_id`, `root_dir`, `sandbox_root`, `all_constructs`, `visited`, `stack`, `stack_set`
- Short loop variables: `i`, `e`, `c`, `p` used in iterators
- Abbreviated names for internal state: `idx`, `prov`, `msg`, `w`, `s`, `n`, `ct`

**Types:**
- PascalCase for structs and enums: `Parser`, `Index`, `ElabError`, `Provenance`, `RawConstruct`, `RawType`, `RawExpr`, `OutputFormat`, `Commands`, `Cli`
- PascalCase for trait names and impl blocks
- Enum variants in PascalCase: `Bool`, `Int`, `Decimal`, `Money`, `Duration`, `Text`, `Date`, `DateTime`, `Enum`, `Record`, `List`, `TypeRef`
- Wrapper types with descriptive names: `TypeEnv`, `SourceProvider`

**Constants:**
- Uppercase with underscores: `TENOR_VERSION`, `TENOR_BUNDLE_VERSION`, `INTERCHANGE_SCHEMA_STR`, `MANIFEST_SCHEMA_STR`
- Module-level string constants documented with `///` docs

## Code Style

**Formatting:**
- Uses default `cargo fmt` settings (rustfmt default configuration)
- 4-space indentation (Rust standard)
- No custom rustfmt.toml configuration detected; uses Rust defaults
- Line length: follows rustfmt defaults (~100 chars, configurable)
- Brace style: opening braces on same line for structs and impl blocks

**Linting:**
- Uses Clippy with `-D warnings` flag (warnings treated as errors)
- Allows specific clippy lints where needed: `#![allow(clippy::result_large_err)]` on `crates/core/src/lib.rs`
- No `.clippy.toml` or `clippy.toml` configuration; uses workspace defaults

**Module Documentation:**
- Markdown-style module docs with `//!` at file start: `//! Pass 0+1: Lex, parse, import resolution...`
- Module docs explain purpose, inputs, outputs, and key invariants
- Example: `crates/core/src/elaborate.rs` documents the 6-pass orchestration pattern
- Section headers in docs use `// ──────────────────────────────────────────────` ASCII dividers

## Import Organization

**Order:**
1. External crates: `use serde::{...}`, `use clap::{...}`
2. Standard library: `use std::collections::{...}`, `use std::path::*`
3. Crate modules: `use crate::error::*`, `use crate::ast::*`
4. Re-exports: `pub use ast::{...}` for public API surface

**Path Aliases:**
- No path aliases in use statements; all imports are fully qualified
- Re-exports used at module root level to provide convenience API
- Example in `crates/core/src/lib.rs`: `pub use elaborate::{elaborate, elaborate_with_provider}`

**Module Structure:**
- Parent module re-exports child AST types for consistency: `pub use crate::ast::{Provenance, RawConstruct, ...}`
- Barrel files used minimally; main re-export is at `lib.rs`

## Error Handling

**Patterns:**
- `Result<T, ElabError>` is the error type for elaboration pipeline
- Errors flow as `Err()` through pass functions
- Use `map_err()` to convert system errors to `ElabError`:
  ```rust
  provider.canonicalize(root).map_err(|e| {
      ElabError::new(1, None, None, None, &root.to_string_lossy(), 0, format!("cannot open file: {}", e))
  })
  ```
- Use `?` operator to propagate errors early
- CLI commands use match expressions to handle `Result` and report errors with `process::exit(1)`
- Test fixtures use `unwrap_or_else()` with detailed panic messages for clarity
- Error JSON serialization via `ElabError::to_json_value()` for structured error output

**ElabError Structure:**
- Fields: `pass` (u8), `construct_kind` (Option), `construct_id` (Option), `field` (Option), `file`, `line`, `message`
- Pass numbers map to elaboration stages: 0 (lex/parse), 1 (bundle), 2 (index), 3 (types), 4 (typecheck), 5 (validate), 6 (serialize)
- Constructor: `ElabError::new(pass, kind, id, field, file, line, message)`
- Convenience constructors: `ElabError::lex()`, `ElabError::parse()`

## Logging

**Framework:** `eprintln!()` for error output, `println!()` for standard output

**Patterns:**
- Errors logged to stderr via `eprintln!()`: `eprintln!("Server error: {}", e)`
- Success output to stdout via `println!()`
- Test fixtures may log to stderr for debugging: `eprintln!("Schema validation passed for {} expected.json files", tested)`
- JSON output structured via `serde_json::to_string_pretty()`
- Quiet mode support: when `quiet` flag is set, non-essential errors are suppressed
- No logging framework (tracing, log, env_logger); uses direct eprintln! for simplicity

## Comments

**When to Comment:**
- Module-level comments explain pass invariants and data flow
- `// ──────────────` dividers used to separate logical sections within files
- Inline comments explain non-obvious logic, particularly in parser lookahead and error construction
- Comments added before complex validation checks explaining what is being validated

**JSDoc/TSDoc:**
- Uses Rust doc comments (`///` for public items)
- Doc comments include purpose, arguments (if complex), return type, and error conditions
- Doc links use backticks and module paths: `` [`elaborate()`] ``, `` [`FileSystemProvider`](crate::source::FileSystemProvider) ``
- Example from elaborate.rs:
  ```rust
  /// Elaborate the given root `.tenor` file and return the interchange bundle,
  /// or the first elaboration error encountered.
  ///
  /// Uses the default [`FileSystemProvider`](crate::source::FileSystemProvider)
  /// for file I/O. For filesystem-independent elaboration (e.g., WASM),
  /// use [`elaborate_with_provider`] instead.
  ```

## Function Design

**Size:**
- Functions generally 30-80 lines
- Large orchestrator functions like `load_file()` up to 150+ lines when looping through recursive structures
- Test fixtures use helper functions to keep individual test cases concise

**Parameters:**
- Use references (`&T`) for read-only access to collections and complex types
- Pass paths as `&Path` for filesystem operations
- Error context provided inline in function signatures, not via Result types (Error type handles context)
- Generics used sparingly; mostly confined to trait bounds on `SourceProvider`

**Return Values:**
- Always use `Result<T, ElabError>` for fallible operations
- Directly return `Value` (serde_json) for successful elaboration
- Test helpers return `serde_json::Value` for JSON fixtures
- No custom Result type wrapper; directly use `Result` with `ElabError`

## Module Design

**Exports:**
- Public functions at module level: `pub fn elaborate()`, `pub fn build_index()`
- Struct fields made public when needed for external passes to read: `pub facts: HashMap<String, Provenance>`
- Types re-exported in parent module lib.rs for public API: `pub use error::ElabError`
- Internal helper functions (not in public API) left as private `fn`

**Barrel Files:**
- Single barrel file at `crates/core/src/lib.rs` re-exports key types and entry points
- Pattern: imports and re-exports provide convenient access without requiring callers to know internal structure
- Example: `pub use elaborate::{elaborate, elaborate_with_provider}` makes pipeline entry point easily discoverable

---

*Convention analysis: 2026-02-25*
