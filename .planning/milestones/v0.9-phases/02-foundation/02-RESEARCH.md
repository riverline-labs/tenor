# Phase 2: Foundation - Research

**Researched:** 2026-02-21
**Domain:** Rust Cargo workspace refactoring, elaborator decomposition, conformance extension
**Confidence:** HIGH

## Summary

Phase 2 transforms the monolithic `elaborator/` crate into a Cargo workspace with `tenor-core` as a library crate exposing typed intermediate representations (AST, Index, TypeEnv) as public API. The current elaborator is a single binary crate with all logic in six source files totaling ~5,600 lines of Rust. The main file `elaborate.rs` (2,066 lines) contains all six elaboration passes, the construct index, and the serialization layer in private functions. The refactoring is a mechanical decomposition: extract each pass into its own module, make key data structures public, then wrap the binary in a thin CLI crate that depends on `tenor-core`.

Concurrently, the elaborator must be extended to handle three v1.0 spec constructs that were formalized in Phase 1 but never implemented: (1) `persona` as a first-class parsed/elaborated/serialized construct, (2) Operation `outcomes` field with associated effect-to-outcome mapping, and (3) `tenor_version` field in the interchange bundle. The interchange schema (`docs/interchange-schema.json`, 836 lines) is already written and includes definitions for all these constructs. A conformance extension adds test fixtures for these constructs plus a schema validation test harness.

**Primary recommendation:** Do the workspace extraction first (Plans 01-03), then implement spec additions (Plan 04), then extend conformance (Plan 05), then add CI (Plan 06). This ordering ensures the 47 existing tests serve as a regression safety net throughout the refactoring.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| FNDN-01 | Monolithic `elaborate.rs` refactored into typed per-pass modules within `tenor-core` library crate | elaborate.rs has clear pass boundaries at lines 48-186 (Pass 0+1), 187-285 (Pass 2), 287-435 (Pass 3), 437-684 (Pass 4), 686-1349 (Pass 5), 1351-2066 (Pass 6). Each section is self-contained with well-defined inputs/outputs. Decomposition is mechanical. |
| FNDN-02 | Cargo workspace with separate crates: `tenor-core`, `tenor-cli`, `tenor-eval`, `tenor-analyze`, `tenor-codegen`, `tenor-lsp` | No workspace exists today. Single `elaborator/Cargo.toml` with edition 2021, three deps (serde, serde_json, ureq). Standard Cargo workspace pattern with root `Cargo.toml` + member crates. |
| FNDN-03 | Existing 47 conformance tests continue to pass after refactoring | Currently 47/47 passing. Tests exercise `elaborate::elaborate()` through `runner::run_suite()`. As long as the public `elaborate()` function signature is preserved in `tenor-core`, tests pass unchanged. |
| FNDN-04 | Intermediate pass outputs (typed AST, Index, TypeEnv) exposed as public API from `tenor-core` | Currently private: `struct Index` (line 191), `type TypeEnv = HashMap<String, RawType>` (line 291), all pass functions. Making these `pub` and re-exporting from `tenor-core` lib.rs is the core API design task. |
| TEST-01 | CI pipeline runs all conformance suites on every commit | No CI exists (no `.github/` directory). GitHub Actions workflow needed. Rust CI is well-established: `cargo build`, `cargo test`, `cargo run -- run ../conformance`. |
| TEST-02 | Elaborator conformance suite extended to cover persona, P7 outcome typing, and P5 shared types | Current parser has NO `persona` construct variant. Operation has NO `outcomes` field. Bundle emits `tenor: "0.3"` not `"1.0"`. Type libraries exist via `import` but no dedicated shared-type-only fixtures. Substantial elaborator changes needed. |
| TEST-08 | Interchange JSON Schema validation test -- every elaborator output validates against the formal schema | Schema exists at `docs/interchange-schema.json` (836 lines, JSON Schema draft 2020-12). Need a test that validates every positive conformance test's expected JSON against this schema. Rust crate `jsonschema` is the standard validator. |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde | 1.x | Serialization framework | Already in use; Rust ecosystem standard |
| serde_json | 1.x | JSON serialization | Already in use; needed for interchange format |
| ureq | 3.x | Synchronous HTTP client | Already in use (ambiguity harness); no new dependency for Phase 2 |
| jsonschema | 0.28+ | JSON Schema validation | Standard Rust crate for JSON Schema draft 2020-12 validation; needed for TEST-08 |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| clap | 4.x | CLI argument parsing | Phase 3 (not needed yet, but CLI crate can declare dep early) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| jsonschema | valico | jsonschema is actively maintained and supports draft 2020-12; valico is older |
| Manual schema validation | None | Schema is 836 lines; hand-rolling validation would be error-prone and unmaintainable |

**Installation:**
```toml
# In tenor-core/Cargo.toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# In dev-dependencies (for schema validation tests)
[dev-dependencies]
jsonschema = "0.28"
```

## Architecture Patterns

### Recommended Project Structure
```
Cargo.toml                    # Workspace root
crates/
  tenor-core/
    Cargo.toml
    src/
      lib.rs                  # Re-exports: elaborate(), Index, TypeEnv, AST types
      ast.rs                  # RawConstruct, RawType, RawExpr, etc. (from parser.rs types)
      error.rs                # ElabError (unchanged)
      lexer.rs                # Tokenizer (unchanged)
      parser.rs               # Parser (extended for persona + outcomes)
      pass1_bundle.rs         # load_bundle, import resolution, cycle detection
      pass2_index.rs          # build_index, Index struct
      pass3_types.rs          # build_type_env, TypeDecl resolution
      pass4_typecheck.rs      # resolve_types, type_check_rules
      pass5_validate.rs       # validate, all structural checks
      pass6_serialize.rs      # serialize to interchange JSON
      elaborate.rs            # Public entry point, orchestrates passes
  tenor-cli/
    Cargo.toml                # depends on tenor-core
    src/
      main.rs                 # CLI binary (current main.rs, calls tenor-core)
  tenor-eval/
    Cargo.toml                # stub — Phase 3
    src/lib.rs
  tenor-analyze/
    Cargo.toml                # stub — Phase 4
    src/lib.rs
  tenor-codegen/
    Cargo.toml                # stub — Phase 6
    src/lib.rs
  tenor-lsp/
    Cargo.toml                # stub — Phase 8
    src/lib.rs
conformance/                  # Unchanged location
docs/                         # Unchanged location
```

### Pattern 1: Workspace Root Cargo.toml
**What:** Single root `Cargo.toml` with `[workspace]` declaring all member crates.
**When to use:** Always, for multi-crate Rust projects.
**Example:**
```toml
# /Cargo.toml (workspace root)
[workspace]
members = [
    "crates/tenor-core",
    "crates/tenor-cli",
    "crates/tenor-eval",
    "crates/tenor-analyze",
    "crates/tenor-codegen",
    "crates/tenor-lsp",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

### Pattern 2: Library + Binary Split
**What:** `tenor-core` is a library crate (`lib.rs`), `tenor-cli` is a binary crate (`main.rs`) that depends on `tenor-core`.
**When to use:** When downstream crates need programmatic access to elaboration.
**Example:**
```toml
# crates/tenor-cli/Cargo.toml
[package]
name = "tenor-cli"
version.workspace = true
edition.workspace = true

[[bin]]
name = "tenor"
path = "src/main.rs"

[dependencies]
tenor-core = { path = "../tenor-core" }
serde_json.workspace = true
```

### Pattern 3: Pass-Per-Module Decomposition
**What:** Each elaboration pass becomes its own module with a public entry function and typed output.
**When to use:** When a monolithic function has clearly separated phases.
**Example:**
```rust
// crates/tenor-core/src/pass2_index.rs
use crate::ast::{RawConstruct, Provenance};
use crate::error::ElabError;
use std::collections::HashMap;

/// Construct index produced by Pass 2.
pub struct Index {
    pub facts: HashMap<String, Provenance>,
    pub entities: HashMap<String, Provenance>,
    pub rules: HashMap<String, Provenance>,
    pub operations: HashMap<String, Provenance>,
    pub flows: HashMap<String, Provenance>,
    pub type_decls: HashMap<String, Provenance>,
    pub rule_verdicts: HashMap<String, String>,
    pub verdict_strata: HashMap<String, (String, i64)>,
    pub personas: HashMap<String, Provenance>,  // NEW for v1.0
}

pub fn build_index(constructs: &[RawConstruct]) -> Result<Index, ElabError> {
    // ... existing logic, now public
}
```

### Pattern 4: Stub Crates for Future Phases
**What:** Create minimal `lib.rs` files for crates not yet implemented, so the workspace compiles.
**When to use:** When declaring all planned crates upfront aids downstream planning.
**Example:**
```rust
// crates/tenor-eval/src/lib.rs
//! Tenor contract evaluator — accepts interchange bundle + facts, produces verdicts.
//! Implementation deferred to Phase 3.
```

### Anti-Patterns to Avoid
- **Moving conformance tests into crate test directories:** Keep the conformance suite in `conformance/` at repo root. The runner reads fixture files from disk. Tests are not unit tests; they are integration-level file-based tests.
- **Breaking the `elaborate()` function signature during refactoring:** The conformance runner calls `elaborate::elaborate(path)` and checks the returned `serde_json::Value`. Any signature change breaks all 47 tests. Preserve the `pub fn elaborate(root_path: &Path) -> Result<Value, ElabError>` interface.
- **Renaming the binary from `tenor-elaborator` to `tenor` during this phase:** Phase 3 introduces the unified `tenor` binary with clap subcommands. Phase 2 keeps the existing binary name and adds `tenor-cli` as a second binary that wraps it.
- **Making all struct fields `pub` reflexively:** Only expose what downstream crates actually need. Start conservative; widen access as Phase 3/4 requirements emerge.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON Schema validation | Custom schema checker | `jsonschema` crate | Schema is 836 lines with `oneOf`, `$ref`, `$defs`; hand-rolling is error-prone |
| CI pipeline | Shell scripts | GitHub Actions YAML | Standard, well-documented, integrates with GitHub |
| Workspace dependency management | Manual version pinning | `[workspace.dependencies]` | Cargo workspace feature since Rust 1.64 |

**Key insight:** The refactoring is primarily a mechanical code motion task. The risk is in maintaining behavioral compatibility, not in novel technical challenges. Use the 47-test conformance suite as the regression gate at every step.

## Common Pitfalls

### Pitfall 1: Breaking Import Paths During Module Extraction
**What goes wrong:** Moving types between modules breaks `use` statements in the same crate, causing compile errors and confusing circular dependency issues.
**Why it happens:** Rust's module system requires explicit re-exports. Moving `RawType` from `parser.rs` to `ast.rs` means every file that did `use crate::parser::RawType` must change.
**How to avoid:** Extract types into `ast.rs` first, then re-export from `parser.rs` via `pub use crate::ast::*;` to maintain backward compatibility within the crate during transition. Remove re-exports only after all internal callers are updated.
**Warning signs:** Compile errors mentioning "cannot find type" or "private type in public interface."

### Pitfall 2: Pass 6 Serialization Sensitivity to Key Ordering
**What goes wrong:** JSON output changes subtly (key ordering, null vs absent), breaking conformance tests that do byte-for-byte comparison.
**Why it happens:** Pass 6 manually constructs `serde_json::Map` objects with `insert()` calls in lexicographic order. Moving code between modules can accidentally reorder insertions or change null-handling.
**How to avoid:** The conformance runner uses `json_equal()` which does deep structural comparison, not string comparison. However, changing serialization logic (e.g., refactoring `serialize_construct`) must preserve exactly which keys are present and what values they have.
**Warning signs:** Conformance test failures showing `output mismatch` diffs.

### Pitfall 3: Persona Implementation Touches Parser, All Passes, and Serialization
**What goes wrong:** Adding `persona` as a first-class construct requires changes across the entire pipeline: lexer (keyword), parser (new construct), pass 1 (bundle assembly), pass 2 (index), pass 5 (validation of persona references), pass 6 (serialization). Missing any one of these causes hard-to-trace bugs.
**Why it happens:** Tenor constructs are threaded through all six passes. A new construct kind must be handled in every match expression on `RawConstruct`.
**How to avoid:** Plan the persona implementation as a vertical slice touching all layers. Add the `Persona` variant to `RawConstruct` first, then handle it in every `match c { ... }` block before running tests. Use the compiler's exhaustiveness checking as a guide.
**Warning signs:** Non-exhaustive match warnings; "unreachable pattern" or "unmatched" compiler errors.

### Pitfall 4: Version String Update from "0.3" to "1.0"
**What goes wrong:** The current elaborator emits `"tenor": "0.3"` in all constructs and the bundle. The v1.0 schema requires `"tenor": "1.0"` and `"tenor_version": "1.0.0"`. Updating these breaks all 47 existing conformance tests because every `.expected.json` file contains `"tenor": "0.3"`.
**Why it happens:** Conformance tests do exact JSON matching. The version string appears in every construct output.
**How to avoid:** Update version strings AND update all `.expected.json` files in the same commit. This is a bulk find-and-replace operation. Alternatively, introduce the version bump in the same plan as the conformance extension (Plan 05), updating all fixtures at once.
**Warning signs:** Every conformance test failing with `"tenor": "0.3"` vs `"tenor": "1.0"` diffs.

### Pitfall 5: Operation `outcomes` Field Requires Parser Extension
**What goes wrong:** The current parser rejects unknown Operation fields. Adding `outcomes` requires parser changes AND new conformance fixtures, but the existing positive tests don't declare outcomes.
**Why it happens:** Parser line 953: `_ => return Err(self.err(format!("unknown Operation field '{}'", key)))`. New fields must be added to the match.
**How to avoid:** Make `outcomes` optional during parsing (default to empty or infer from effects for backward compatibility with existing fixtures). The spec says `|outcomes| >= 1` but existing v0.3 tests don't have outcomes. Either (a) update existing fixtures to add outcomes, or (b) make outcomes optional in the parser with a validation pass that enforces the requirement for v1.0 contracts. Option (a) is cleaner.
**Warning signs:** Parse errors on existing `.tenor` fixtures that lack `outcomes` fields.

### Pitfall 6: Stub Crates Must Actually Compile
**What goes wrong:** Empty stub crates that don't compile waste CI minutes and block workspace-wide `cargo build`.
**Why it happens:** A `Cargo.toml` without a valid `src/lib.rs` fails to compile.
**How to avoid:** Every stub crate gets a minimal `src/lib.rs` with a doc comment and nothing else. Verify `cargo build --workspace` succeeds after adding stubs.
**Warning signs:** CI failures on `cargo build --workspace` from empty crate directories.

## Code Examples

Verified patterns from the existing codebase:

### Workspace Root Cargo.toml
```toml
# Root /Cargo.toml
[workspace]
members = [
    "crates/tenor-core",
    "crates/tenor-cli",
    "crates/tenor-eval",
    "crates/tenor-analyze",
    "crates/tenor-codegen",
    "crates/tenor-lsp",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
ureq = { version = "3", features = ["json"] }
```

### Persona Construct in AST
```rust
// In ast.rs - new variant added to RawConstruct
pub enum RawConstruct {
    // ... existing variants ...
    Persona {
        id: String,
        prov: Provenance,
    },
}
```

### Persona in Parser
```rust
// In parser.rs - parse_construct match arm
"persona" => self.parse_persona(line),

// New function
fn parse_persona(&mut self, line: u32) -> Result<RawConstruct, ElabError> {
    self.advance(); // consume 'persona'
    let id = self.take_word()?;
    Ok(RawConstruct::Persona {
        id,
        prov: Provenance { file: self.filename.clone(), line },
    })
}
```

### Persona in Pass 2 Index
```rust
// In pass2_index.rs
RawConstruct::Persona { id, prov } => {
    if let Some(first) = idx.personas.get(id) {
        return Err(ElabError::new(
            2, Some("Persona"), Some(id), Some("id"),
            &prov.file, prov.line,
            format!("duplicate Persona id '{}': first declared at line {}", id, first.line),
        ));
    }
    idx.personas.insert(id.clone(), prov.clone());
}
```

### Persona in Pass 6 Serialization
```rust
// In pass6_serialize.rs
RawConstruct::Persona { id, prov } => {
    let mut m = Map::new();
    m.insert("id".to_owned(), json!(id));
    m.insert("kind".to_owned(), json!("Persona"));
    m.insert("provenance".to_owned(), serialize_prov(prov));
    m.insert("tenor".to_owned(), json!("1.0"));
    Value::Object(m)
}
```

### Operation Outcomes in Parser
```rust
// In parse_operation - add to match block
"outcomes" => { outcomes = self.parse_ident_array()?; }
```

### Schema Validation Test
```rust
// In tenor-core dev-dependency test or integration test
#[test]
fn validate_conformance_output_against_schema() {
    let schema_src = std::fs::read_to_string("../../docs/interchange-schema.json").unwrap();
    let schema_value: serde_json::Value = serde_json::from_str(&schema_src).unwrap();
    let compiled = jsonschema::compile(&schema_value).unwrap();

    // For each positive conformance test expected.json:
    for entry in glob::glob("../../conformance/positive/*.expected.json").unwrap() {
        let path = entry.unwrap();
        let json_src = std::fs::read_to_string(&path).unwrap();
        let instance: serde_json::Value = serde_json::from_str(&json_src).unwrap();
        let result = compiled.validate(&instance);
        assert!(result.is_ok(), "Schema validation failed for {}: {:?}", path.display(), result.err());
    }
}
```

### GitHub Actions CI
```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Build workspace
        run: cargo build --workspace
      - name: Run conformance suite
        run: cargo run -p tenor-cli -- run conformance
      - name: Run schema validation tests
        run: cargo test --workspace
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Single Cargo.toml per crate | Workspace with `[workspace.dependencies]` | Rust 1.64 (Sept 2022) | Shared dependency versions across workspace members |
| `edition = "2018"` | `edition = "2021"` | Rust 1.56 (Oct 2021) | Project already uses 2021 edition |
| Manual `resolver = "1"` | `resolver = "2"` default for workspaces | Rust 1.51 (Mar 2021) | Feature resolver v2 is workspace default; still explicit for clarity |

**Deprecated/outdated:**
- Using `path = "../elaborator"` relative dependencies: use workspace `{ path = "../tenor-core" }` instead
- Single-binary monoliths for multi-purpose tools: Cargo workspace with separate binary/library crates is the standard Rust pattern

## Codebase Analysis: What Must Change

### Files That Stay Unchanged
- `lexer.rs` (239 lines) — Pure tokenizer, no construct awareness needed for persona (persona id is parsed as a Word token, same as other identifiers)
- `tap.rs` — TAP output formatting
- `error.rs` (61 lines) — Error type (may need minor additions)

### Files That Get Decomposed
- `elaborate.rs` (2,066 lines) → 6 pass modules + orchestrator
- `parser.rs` (1,300 lines) → stays mostly intact, gains `persona` construct + Operation `outcomes` field
- `runner.rs` (260 lines) → moves to `tenor-cli`
- `main.rs` (134 lines) → moves to `tenor-cli`

### New Files Created
- `crates/tenor-core/src/ast.rs` — Shared type definitions extracted from parser.rs
- `crates/tenor-core/src/pass1_bundle.rs` through `pass6_serialize.rs`
- `crates/tenor-core/src/elaborate.rs` — Thin orchestrator calling pass functions
- `crates/tenor-core/src/lib.rs` — Public re-exports
- `crates/tenor-cli/src/main.rs` — Binary entry point
- Stub `lib.rs` for eval, analyze, codegen, lsp crates

### Version String Changes Required
- All 47 `.expected.json` fixtures: `"tenor": "0.3"` -> `"tenor": "1.0"`
- Bundle envelope: add `"tenor_version": "1.0.0"` field
- Elaborator serialization: 6 occurrences of `"0.3"` in elaborate.rs

### Spec Constructs to Implement

**Persona (SPEC-01):**
- Parser: Add `"persona"` to construct keyword match, parse as `persona <id>` (no braces, no body)
- AST: Add `RawConstruct::Persona { id, prov }`
- Pass 1: Handle in import/cross-file-dup logic (skip or check like other constructs)
- Pass 2: Index personas, check duplicate persona ids
- Pass 5: Validate persona references in `allowed_personas`, Flow step `persona` fields, HandoffStep `from_persona`/`to_persona`
- Pass 6: Serialize to `{"id": ..., "kind": "Persona", "provenance": ..., "tenor": "1.0"}`

**Operation Outcomes (SPEC-02 / P7):**
- Parser: Add `"outcomes"` field to Operation parsing (array of identifiers)
- AST: Add `outcomes: Vec<String>` to `RawConstruct::Operation`
- Effects: Extend effect tuples to support `-> outcome_label` association syntax
- Pass 5: Validate outcomes non-empty, disjoint from error_contract, effect-to-outcome completeness for multi-outcome ops
- Pass 6: Serialize `"outcomes"` array in Operation interchange; add `"outcome"` field to Effect objects for multi-outcome ops

**Shared Types (SPEC-03 / P5):**
- Already partially implemented via `import` mechanism and TypeDecl
- Need conformance test fixtures specifically exercising type-library-only files
- Validate that type library files contain only TypeDecl constructs (spec constraint)

**Interchange Versioning (SPEC-04):**
- Bundle: Add `"tenor_version": "1.0.0"` field
- All constructs: Change `"tenor": "0.3"` to `"tenor": "1.0"`

## Open Questions

1. **Crate directory: `crates/` vs flat layout?**
   - What we know: The roadmap says "Cargo workspace with separate crates." Common patterns are `crates/` subdirectory or flat top-level layout.
   - What's unclear: User preference for directory organization.
   - Recommendation: Use `crates/` subdirectory. It keeps the repo root clean and is the dominant pattern in Rust projects with 3+ crates.

2. **Should existing `.tenor` fixtures be updated for v1.0 syntax?**
   - What we know: Existing positive tests don't declare personas or outcomes. The spec makes persona references in Operations and Flows resolve against declared personas (Pass 5 validation).
   - What's unclear: Whether to update all existing fixtures to add persona declarations and outcomes, or keep them as-is and add separate new fixtures.
   - Recommendation: Update existing fixtures to include persona declarations and outcomes. This ensures full v1.0 conformance. Keeping old-format fixtures alive would mean the elaborator needs to support both v0.3 and v1.0 input syntax, which adds complexity for no benefit.

3. **Should the ambiguity module move to `tenor-cli` or stay in `tenor-core`?**
   - What we know: The ambiguity harness (`ambiguity/` submodule) uses `ureq` for HTTP and is CLI-specific functionality.
   - What's unclear: Whether future consumers of `tenor-core` would need the ambiguity module.
   - Recommendation: Move ambiguity to `tenor-cli`. It is a development/CI tool, not core elaboration logic. This also removes the `ureq` dependency from `tenor-core`.

4. **When to bump version strings: during refactoring or during conformance extension?**
   - What we know: Changing `"tenor": "0.3"` to `"tenor": "1.0"` breaks all 47 expected-JSON fixtures.
   - What's unclear: Whether to batch this change.
   - Recommendation: Do it in the conformance extension plan (Plan 05) together with adding persona/outcomes tests. This minimizes the window where tests are broken and batches all fixture modifications.

## Sources

### Primary (HIGH confidence)
- Codebase analysis: `elaborator/src/elaborate.rs` (2,066 lines, 6 clearly demarcated passes)
- Codebase analysis: `elaborator/src/parser.rs` (1,300 lines, RawConstruct enum lacks Persona/outcomes)
- Codebase analysis: `elaborator/Cargo.toml` (edition 2021, serde 1.x, serde_json 1.x, ureq 3.x)
- Codebase analysis: `docs/interchange-schema.json` (836 lines, includes Persona and outcomes definitions)
- Codebase analysis: Conformance suite 47/47 passing, 51 .tenor files across 8 categories
- Spec: `docs/TENOR.md` Section 8 (Persona), Section 9 (Operation with outcomes)
- Rust toolchain: rustc 1.93.1, cargo 1.93.1

### Secondary (MEDIUM confidence)
- Cargo workspace patterns: standard practice since Rust 1.64 (workspace.dependencies)
- GitHub Actions Rust CI: `dtolnay/rust-toolchain@stable` + `Swatinem/rust-cache@v2` are well-established
- `jsonschema` crate: standard for JSON Schema validation in Rust, supports draft 2020-12

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - pure Rust refactoring with existing well-understood dependencies
- Architecture: HIGH - clear pass boundaries in existing code make decomposition mechanical
- Pitfalls: HIGH - identified from direct codebase analysis, not external sources
- Spec implementation: HIGH - schema and spec sections provide precise requirements

**Research date:** 2026-02-21
**Valid until:** 2026-03-21 (stable domain, no fast-moving dependencies)
