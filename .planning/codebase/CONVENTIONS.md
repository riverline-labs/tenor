# Coding Conventions

**Analysis Date:** 2026-02-21

## Naming Patterns

**Files:**
- Lowercase snake_case with `.rs` extension
- Match module/struct names exactly: `lexer.rs` contains the `Spanned` and `lex()` function; `elaborate.rs` contains elaboration passes
- Test/fixture files use `.tenor` extension for DSL source and `.expected.json` for expected output

**Functions:**
- Lowercase snake_case: `load_bundle`, `check_cross_file_dups`, `build_type_env`
- Public functions prefixed with `pub fn`
- Helper functions (private) use same naming: no distinction between public/private except `pub` keyword
- Pass-related functions explicit: `type_check_rules`, `validate_rule`, `serialize_construct`

**Variables:**
- Lowercase snake_case: `all_constructs`, `fact_types`, `bundle_id`
- Single-letter or abbreviated names only for loop counters and very local scope: `pos`, `c` (for construct)
- Meaningful names even in loops: `for c in constructs` instead of `for item in constructs`

**Types:**
- PascalCase for structs: `Parser`, `Provenance`, `RawConstruct`, `ElabError`, `Tap`
- PascalCase for enums: `Token`, `RawType`, `RawExpr`, `RawTerm`, `RawStep`
- Enum variants: PascalCase when compound, snake_case for simple: `TypeRef(String)`, `Fact { ... }`, `lex` command keyword
- Type aliases use full words: `TypeEnv = HashMap<String, RawType>`

## Code Style

**Formatting:**
- Standard Rust conventions (no explicit formatter referenced)
- Indentation: 4 spaces (observed throughout)
- Lines: generally under 100 characters; longer lines appear in complex match statements and error messages
- Braces: Allman style for structs/enums (opening brace on same line)

**Linting:**
- No explicit linting config found in repo (.eslintrc, .clippy.toml absent)
- Code follows standard Rust idioms: result types, pattern matching, no unwraps in library code

## Import Organization

**Order:**
1. External crate imports (`use serde::...`, `use serde_json::...`)
2. Standard library (`use std::...`)
3. Local crate modules (`use crate::...`)

**Path Aliases:**
- No path aliases used (`@` shortcuts absent in code)
- Imports explicit and full: `use crate::elaborate`, `use crate::error::ElabError`

**Examples:**
```rust
// elaborate.rs imports
use crate::error::ElabError;
use crate::lexer;
use crate::parser::{self, Provenance, RawBranch, RawCompStep, RawConstruct, RawExpr, ...};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
```

## Error Handling

**Patterns:**
- All fallible operations return `Result<T, ElabError>`
- `ElabError::new(pass, kind, id, field, file, line, message)` factory used throughout
- Errors never unwrap; always propagate with `?` operator or explicit `Err` return
- Error context includes elaboration pass number (0-6), construct metadata, file location, line number
- Error messages are user-facing, descriptive, and actionable

**Error examples:**
```rust
// From elaborate.rs line 220-224
if let Some(first) = idx.facts.get(id) {
    return Err(ElabError::new(
        2, Some("Fact"), Some(id), Some("id"),
        &prov.file, prov.line,
        format!("duplicate Fact id '{}': first declared at line {}", id, first.line),
    ));
}

// From elaborate.rs line 344-349
return Err(ElabError::new(
    3, Some("TypeDecl"), Some(back_edge_name),
    Some(&format!("type.fields.{}", field_name)),
    &prov.file, prov.line,
    format!("TypeDecl cycle detected: {}", cycle_str),
));
```

## Logging

**Framework:** Built-in Rust `eprintln!` macro for stderr, `println!` for stdout

**Patterns:**
- Error output: `eprintln!("error: ...")` to stderr
- Help text: `eprintln!("Usage: ...")` to stderr
- Normal output: `println!("{}", pretty)` for successful elaboration, `println!("ok ...")` for TAP test output
- No structured logging framework; messages are plain text

**Examples from `main.rs`:**
```rust
eprintln!("error: conformance suite directory not found: {}", suite_dir.display());
eprintln!("unknown command '{}'; use 'run' or 'elaborate'", cmd);
```

## Comments

**When to Comment:**
- Module-level documentation above module declarations (e.g., `/// Six-pass elaborator: ...`)
- Function-level documentation for public entry points only: `/// Elaborate the given root file...`
- Inline comments for non-obvious algorithm steps or complex pass transitions
- Inline comments explaining why, not what: "// cross-file duplicate check (Pass 1)" not "// check duplicates"
- Block separators for major pass boundaries: `// ──────────────────────────────────────────────...`

**Doc Comments (///)**
- Used only for high-level, public APIs: `pub fn elaborate()`, module overview
- Not used for internal helper functions or struct fields (seen in parser.rs for RawConstruct variants only)
- Include pass number context for elaboration functions: `/// Pass 4: check produce clause...`

**Test Comments:**
- DSL test files (.tenor) include descriptive comments: `// Positive test: basic Fact declarations...` explaining test intent and expected behavior
- Comments reference error location if applicable: `// An unrecognized character '@' appears in the source.`

**Examples:**
```rust
// From elaborate.rs module header (lines 1-9)
/// Six-pass elaborator: Tenor → TenorInterchange JSON bundle.
///
/// Pass 0 — Lex and parse
/// Pass 1 — Import resolution and bundle assembly
...

// From elaborate.rs function (lines 75-77)
/// Detect constructs with the same (kind, id) coming from different files.
/// Scans in reverse so that root-file constructs (appended last) are treated as
/// "first declared", and imports with clashing ids get the Pass 1 error.

// From elaborate.rs inline (line 67)
// all_constructs is in imports-first order (depth-first); scanning in reverse
// means root-file constructs are encountered first, so they are "first declared".
```

## Function Design

**Size:** Functions average 20-40 lines; longest functions handle complex validation (80-120 lines like `validate_flow`)
- Helper functions are extracted for readability: `references_type`, `type_refs`, `resolve_typedecl`
- Pass-specific logic grouped in dedicated functions matching pass name

**Parameters:**
- Explicit struct passing preferred over references to primitive collections
- Input references immutable unless mutating collection: `&mut Tap`, `&mut in_stack`
- File provenance (`prov: &Provenance` or `file: &str, line: u32`) always explicit
- Fact/type lookups passed as `&HashMap` for read-only access

**Return Values:**
- All potentially-failing operations return `Result<T, ElabError>`
- Success returns inner value without wrapper: `Ok(bundle)` not `Ok(Ok(...))`
- Helper builders return inner type then wrap caller: `build_type_env() -> Result<TypeEnv, ...>` not `Option`
- Void operations return `()`: `validate()`, `check_cross_file_dups()`

**Examples:**
```rust
// From elaborate.rs line 204
fn build_index(constructs: &[RawConstruct]) -> Result<Index, ElabError> {
    let mut idx = Index { ... };
    for c in constructs { ... }
    Ok(idx)
}

// From elaborate.rs line 321-326
fn detect_typedecl_cycle(
    name: &str,
    decls: &BTreeMap<String, (BTreeMap<String, RawType>, Provenance)>,
    visited: &mut HashSet<String>,
    in_stack: &mut Vec<String>,
) -> Result<(), ElabError> {
```

## Module Design

**Exports:**
- `pub fn` for entry points used outside module: `pub fn lex()`, `pub fn parse()`, `pub fn elaborate()`, `pub fn run_suite()`
- Type exports via `pub enum`/`pub struct` only where used externally: `pub struct ElabError`, `pub enum Token`
- Internal helpers remain private: `fn build_index()`, `fn serialize_construct()` not exported

**Barrel Files:**
- No barrel file pattern observed (no `mod.rs` with re-exports)
- Each module self-contained: `lexer.rs` has all lexing logic, `parser.rs` has all parsing logic
- Main module (`main.rs`) imports and orchestrates: `mod elaborate; mod error; mod lexer; mod parser; ...`

**No re-exports:** Each module boundary respected; clients import from specific modules: `use crate::elaborate::elaborate`, `use crate::error::ElabError`

## Cross-Cutting Patterns

**Provenance tracking:**
All constructs carry exact source location (`file: String, line: u32`). Used in every error to pinpoint DSL source.

**Pass numbering:**
Every error includes pass number (0-6) as first field. Errors reported to users and in tests with pass number prefix.

**Collections over iteration:**
Prefer `BTreeMap` for stable ordering (enum variant iteration, typeDecl field iteration).
Use `HashMap` for O(1) lookups (construct index, type env).
Use `HashSet` for membership tests (visited tracking, duplicate detection).
Use `Vec` for ordered sequences (constructs, transitions, steps).

**JSON serialization:**
- All output via serde_json with explicit field ordering (Map construction, not derive)
- Keys always alphabetically sorted per Tenor interchange spec
- Numeric values wrapped in structured types: `{"kind": "decimal_value", "precision": P, "scale": S, "value": "..."}`

---

*Convention analysis: 2026-02-21*
