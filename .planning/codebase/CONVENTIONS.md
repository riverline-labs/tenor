# Coding Conventions

**Analysis Date:** 2026-02-21

## Naming Patterns

**Files:**
- Pass modules: `pass{N}_{name}.rs` — e.g., `pass1_bundle.rs`, `pass4_typecheck.rs`
- Test files in `tests/` directory: snake_case — e.g., `cli_integration.rs`, `schema_validation.rs`
- Source modules: snake_case — e.g., `ast.rs`, `elaborate.rs`, `runner.rs`

**Functions:**
- Public API entry points: snake_case verbs — `elaborate()`, `load_bundle()`, `build_index()`, `build_type_env()`, `resolve_types()`
- Helper functions: snake_case — `detect_typedecl_cycle()`, `check_cross_file_dups()`, `glob_tenor_files()`
- Test helper functions: descriptive snake_case — `comparison_bundle()`, `assert_verdict_produced()`, `workspace_root()`
- Constructor methods: `new()`, `from_interchange()`, `from_json()`

**Variables:**
- snake_case throughout — `fact_types`, `bundle_id`, `root_dir`, `verdict_strata`
- Short abbreviations for local iteration — `c` for construct, `t` for type, `p` for path, `e` for entry
- Underscore prefix for intentionally unused params — `_persona`, `_index`

**Types:**
- Structs: PascalCase — `ElabError`, `Provenance`, `Index`, `TypeEnv`, `RunResult`, `Tap`
- Enums: PascalCase — `RawConstruct`, `RawType`, `RawExpr`, `RawTerm`, `Token`, `EvalError`, `Value`
- Enum variants: PascalCase — `Fact`, `Entity`, `Rule`, `Operation`, `Flow`, `Bool`, `Int`, `Decimal`
- Type aliases: PascalCase — `TypeEnv = HashMap<String, RawType>`

**DSL Construct Names in Code:**
- Construct kind strings in interchange JSON use PascalCase: `"Fact"`, `"Entity"`, `"Rule"`, `"Operation"`, `"Flow"`
- DSL source keywords are lowercase (`fact`, `entity`, `rule`) — not in Rust source, only in `.tenor` files

## Code Style

**Formatting:**
- `cargo fmt` (rustfmt) with default settings — enforced in CI
- No `rustfmt.toml` customization detected — standard Rust formatting rules apply

**Linting:**
- `cargo clippy --workspace -- -D warnings` — all warnings treated as errors in CI
- One explicit allow at crate level: `#![allow(clippy::result_large_err)]` in `crates/core/src/lib.rs`
- Two targeted allows on specific types: `#[allow(clippy::large_enum_variant)]` on `PayloadValue` and `FlowStep` in `crates/eval/src/types.rs`

## Import Organization

**Order (within each file):**
1. External crate imports: `use serde_json::Value;`, `use std::collections::HashMap;`
2. Crate-local imports: `use crate::ast::*;`, `use crate::error::ElabError;`
3. No enforced blank-line grouping observed — files typically group by std then crate

**Pattern:**
- `use crate::ast::*;` glob imports are used for AST types within pass modules (acceptable because ast.rs is a shared type module within the same crate)
- All other imports are explicit — `use crate::pass2_index::Index;` not `use crate::pass2_index::*;`
- Cross-crate imports use full paths — `use tenor_core::elaborate;`

**Re-exports:**
- `crates/core/src/lib.rs` re-exports key public types and entry points at crate root for consumer convenience
- Pattern: group by "types" then "entry points" with labeled section comments

## Error Handling

**Primary Pattern:**
- All elaboration errors return `Result<T, ElabError>` — never panics in production code paths
- `ElabError::new()` with explicit pass number, construct kind, construct id, field, file, line, message
- Constructor helpers for common cases: `ElabError::lex()`, `ElabError::parse()`
- Errors propagated with `?` operator — no `unwrap()` in non-test production code except where invariants are guaranteed (e.g., after cycle detection that proved membership)

**Evaluation Errors:**
- `EvalError` enum with named struct variants — `MissingFact { fact_id }`, `TypeMismatch { fact_id, expected, got }`
- `impl fmt::Display for EvalError` — all variants produce human-readable messages
- `impl std::error::Error for EvalError`

**CLI Error Handling:**
- Pattern: `match result { Ok(v) => { ... } Err(e) => { report_error(...); process::exit(1); } }`
- Exit codes: 0 = success, 1 = runtime error, 2 = not-yet-implemented stub
- `report_error()` helper respects `--quiet` and `--output` flags before printing

**Invariant Unwraps:**
- `unwrap()` used only after guards that logically guarantee presence — e.g., after `if stack.contains(&name)` then `.position().unwrap()`
- Tests use `.unwrap()` and `.unwrap_or_else(|e| panic!(...))` freely

## Logging

**Framework:** None — no logging crate detected.

**Patterns:**
- Diagnostic output goes to stderr via `eprintln!`
- Success output goes to stdout via `println!`
- `--quiet` flag suppresses non-essential output at CLI layer; errors on stderr may still be suppressed
- No structured logging in library crates — errors are returned as typed values, not logged

## Comments

**Module-Level Doc Comments:**
- Every `.rs` file begins with a `//!` doc comment describing the module's purpose and pass role
- Example: `//! Pass 0+1: Lex, parse, import resolution, cycle detection, bundle assembly.`
- Top-level crate lib.rs has full `//!` doc block listing public API items with rustdoc links

**Section Dividers:**
- Horizontal rule style: `// ── Section Name ─────────────────────────────────────────────` (box-drawing chars)
- Used consistently in large files to separate logical sections — `ast.rs`, `pass4_typecheck.rs`, `pass5_validate.rs`, `types.rs`
- Shorter variant: `// ── Short label ────────────────────────────────────` for lib.rs re-export groups

**Inline Comments:**
- `///` doc comments on public items (structs, enums, functions, fields) explaining purpose
- `//` inline for clarifying non-obvious choices: field semantics, algorithm steps, invariant notes
- Example from `ast.rs`: `/// Line of the `initial:` field keyword`
- Test category labels: `// ──── A. Int arithmetic (5 cases) ────────────────────`

**When to Comment:**
- All public items: required
- Algorithm steps in passes: one comment per major phase within a function
- Tricky ordering decisions: document the "why" — see `parse_predicate()` comment about Mul nodes

## Function Design

**Size:** Functions are kept focused — pass entry points delegate to private helpers immediately. Large match arms in `types.rs` and `pass5_validate.rs` are the exception where exhaustive matching requires length.

**Parameters:**
- Pass functions accept slices `&[RawConstruct]` not owned Vecs (except where ownership transfer is needed)
- Error context propagated through parameters: `file: &str`, `line: u32` passed to sub-functions
- Mutable state threaded via `&mut` parameters in the bundle loader: `visited`, `stack`, `out`

**Return Values:**
- `Result<T, ElabError>` for all fallible operations in the elaborator
- `Result<T, EvalError>` for all fallible evaluator operations
- Infallible helpers return `T` directly — no wrapping in `Ok()`

## Module Design

**Exports:**
- Library crates expose a minimal public API; internal helpers are `fn` (private by default)
- `pub` on types and functions only when consumed outside the module
- `pub use` in `lib.rs` creates flat re-export surface — callers use `tenor_core::ElabError` not `tenor_core::error::ElabError`

**Barrel Files:**
- `lib.rs` acts as a barrel for re-exports in `tenor-core`
- Individual pass modules are all `pub mod` — fully accessible if needed for partial pipeline execution

---

*Convention analysis: 2026-02-21*
