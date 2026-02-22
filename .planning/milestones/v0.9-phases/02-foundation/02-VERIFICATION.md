---
phase: 02-foundation
verified: 2026-02-21T21:00:00Z
status: gaps_found
score: 12/13 must-haves verified
gaps:
  - truth: "README.md reflects the new crate structure with accurate conformance count"
    status: partial
    reason: "README.md documents the crate structure correctly but still shows '47/47 conformance tests passing' in two places (line 9 and line 155) instead of 55/55. CLAUDE.md correctly says 55/55."
    artifacts:
      - path: "README.md"
        issue: "Lines 9 and 155 still say '47/47 conformance tests passing'; actual count is 55/55"
    missing:
      - "Update README.md line 9: change '47/47' to '55/55'"
      - "Update README.md line 155: change '47/47 tests passing' to '55/55 tests passing'"
---

# Phase 2: Foundation Verification Report

**Phase Goal:** The monolithic elaborator is refactored into a Cargo workspace with `tenor-core` exposing typed pass outputs as public API, all existing tests pass, and conformance suite covers new v1.0 constructs
**Verified:** 2026-02-21T21:00:00Z
**Status:** gaps_found
**Re-verification:** No -- initial verification

---

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `cargo build --workspace` compiles all 6 crates | VERIFIED | `Finished dev profile` with no errors; all 6 members (core, cli, eval, analyze, codegen, lsp) compile |
| 2 | All 55 conformance tests pass (47 original + 8 new) | VERIFIED | `cargo run -p tenor-cli -- run conformance` reports 55 pass, 0 fail |
| 3 | `elaborate.rs` decomposed into 6 per-pass modules inside tenor-core | VERIFIED | `crates/core/src/pass{1..6}_*.rs` all exist; pass1_bundle.rs (138 lines), pass6_serialize.rs (701 lines) |
| 4 | tenor-core is a library crate and tenor-cli is a binary crate depending on it | VERIFIED | `crates/core/Cargo.toml` has `name = "tenor-core"` (no `[[bin]]`); `crates/cli/Cargo.toml` has `[[bin]]` and `tenor-core = { path = "../core" }` |
| 5 | Downstream code can import Index, TypeEnv, and AST types from tenor_core | VERIFIED | `lib.rs` exports `pub use pass2_index::Index`, `pub use pass3_types::TypeEnv`, and 6 AST types at crate root |
| 6 | All 6 workspace crates compile with `cargo build --workspace` | VERIFIED | Same as truth 1; `analyze`, `eval`, `codegen`, `lsp` stub crates all present and compiling |
| 7 | Stub crates exist for eval, analyze, codegen, and lsp with doc comments | VERIFIED | All 4 `lib.rs` files contain doc comments referencing their implementation phase |
| 8 | Parser accepts `persona <id>` and produces a Persona AST node | VERIFIED | `parse_persona()` at parser.rs:783; `RawConstruct::Persona` variant at ast.rs:153; `conformance/positive/persona_basic.tenor` elaborates correctly |
| 9 | Pass 6 serializes Persona constructs with kind=Persona and tenor=1.0 | VERIFIED | pass6_serialize.rs:177-182: `m.insert("kind", json!("Persona"))` and `m.insert("tenor", json!("1.0"))` |
| 10 | All conformance fixture expected JSONs use tenor: 1.0 (not 0.3) | VERIFIED | `grep -r '"0.3"' conformance/` returns 0 matches; all 13 positive expected.json files contain `"tenor_version": "1.0.0"` |
| 11 | Every positive conformance expected JSON validates against interchange schema | VERIFIED | `cargo test -p tenor-core` passes `validate_all_positive_conformance_outputs_against_schema` test |
| 12 | CI pipeline runs workspace build and conformance suite on push and PR | VERIFIED | `.github/workflows/ci.yml` triggers on push/PR to main and v1; runs build, conformance, test, fmt, clippy |
| 13 | README.md reflects the new crate structure with accurate conformance count | PARTIAL | README.md `crates/` section correctly lists all 6 crates. However, README.md line 9 and line 155 still read "47/47 conformance tests passing" -- the actual count is 55/55. CLAUDE.md correctly says "Expected: 55/55 passing". |

**Score:** 12/13 truths verified

---

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `Cargo.toml` | Workspace root with `[workspace]` | VERIFIED | Contains `[workspace]` with 6 members: core, cli, eval, analyze, codegen, lsp |
| `crates/core/src/lib.rs` | Library crate re-exporting elaboration API | VERIFIED | Declares 11 modules with `pub mod`; 9 `pub use` re-exports at crate root |
| `crates/core/src/pass1_bundle.rs` | Pass 1 bundle assembly (min 30 lines) | VERIFIED | 138 lines; contains `load_bundle()`, `load_file()`, cross-file dup check |
| `crates/core/src/pass6_serialize.rs` | Pass 6 serialization (min 100 lines) | VERIFIED | 701 lines; full serialization of all construct kinds with v1.0 version strings |
| `crates/cli/src/main.rs` | CLI binary entry point using tenor_core | VERIFIED | Line 43: `tenor_core::elaborate::elaborate(path)` |
| `crates/core/src/ast.rs` | Persona variant in RawConstruct enum | VERIFIED | `Persona { id: String, prov: Provenance }` at line 153 |
| `crates/core/src/parser.rs` | Persona parsing and outcomes field parsing | VERIFIED | `parse_persona()` at line 783; `"outcomes"` handler at line 725 |
| `crates/core/src/pass2_index.rs` | Persona indexing with duplicate detection | VERIFIED | `personas: HashMap<String, Provenance>` field; duplicate detection at lines 101-108 |
| `crates/eval/src/lib.rs` | Stub evaluator crate (contains "Phase 3") | VERIFIED | Doc comment: "Implementation: Phase 3." |
| `crates/analyze/src/lib.rs` | Stub analyzer crate (contains "Phase 4") | VERIFIED | Doc comment: "Implementation: Phase 4." |
| `crates/codegen/src/lib.rs` | Stub codegen crate (contains "Phase 6") | VERIFIED | Doc comment: "Implementation: Phase 6." |
| `crates/lsp/src/lib.rs` | Stub LSP crate (contains "Phase 8") | VERIFIED | Doc comment: "Implementation: Phase 8." |
| `README.md` | Updated README with `crates/` section | PARTIAL | `crates/` section present and correct; but "47/47" count on lines 9 and 155 should be "55/55" |
| `conformance/positive/persona_basic.tenor` | Basic persona declaration test | VERIFIED | Contains `persona admin`, operation with `allowed_personas: [admin]` |
| `conformance/positive/operation_outcomes.tenor` | Operation with outcomes field test | VERIFIED | Contains `outcomes: [approved, denied]` |
| `conformance/negative/pass2/duplicate_persona.tenor` | Duplicate persona detection test | VERIFIED | Two `persona admin` declarations |
| `crates/core/tests/schema_validation.rs` | JSON Schema validation test | VERIFIED | `validate_all_positive_conformance_outputs_against_schema` test passes |
| `.github/workflows/ci.yml` | GitHub Actions CI pipeline | VERIFIED | Contains build, conformance, test, fmt, clippy steps |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/cli/src/main.rs` | `crates/core/src/elaborate.rs` | `tenor_core::elaborate::elaborate` | WIRED | Line 43: `tenor_core::elaborate::elaborate(path)` |
| `crates/core/src/elaborate.rs` | `crates/core/src/pass1_bundle.rs` | `pass1_bundle::` module calls | WIRED | Line 20: `pass1_bundle::load_bundle(root_path)` |
| `crates/core/src/lib.rs` | `crates/core/src/pass2_index.rs` | `pub use` re-export of Index | WIRED | Line 38: `pub use pass2_index::Index` |
| `crates/core/src/lib.rs` | `crates/core/src/pass3_types.rs` | `pub use` re-export of TypeEnv | WIRED | Line 39: `pub use pass3_types::TypeEnv` |
| `crates/core/src/parser.rs` | `crates/core/src/ast.rs` | creates `RawConstruct::Persona` variant | WIRED | Line 786: `Ok(RawConstruct::Persona { ... })` |
| `crates/core/tests/schema_validation.rs` | `docs/interchange-schema.json` | loads and compiles schema | WIRED | Line 41: `.join("../../docs/interchange-schema.json")`; test passes |
| `.github/workflows/ci.yml` | `conformance/` | runs conformance suite | WIRED | Line 23: `cargo run -p tenor-cli -- run conformance` |
| `crates/cli/src/runner.rs` | `crates/core/src/elaborate.rs` | `use tenor_core::elaborate` | WIRED | Line 8: `use tenor_core::elaborate`; lines 147,182: `elaborate::elaborate(tenor_path)` |

---

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| FNDN-01 | 02-01-PLAN.md | Monolithic `elaborate.rs` refactored into typed per-pass modules within `tenor-core` | SATISFIED | 6 pass modules in `crates/core/src/pass{1..6}_*.rs`; thin orchestrator in `elaborate.rs` |
| FNDN-02 | 02-01-PLAN.md | Cargo workspace with separate crates: core, cli, eval, analyze, codegen, lsp | SATISFIED | `Cargo.toml` workspace has all 6 members; all compile |
| FNDN-03 | 02-01-PLAN.md | Existing 47 conformance tests continue to pass after refactoring | SATISFIED | 55/55 pass (superset of original 47); no test regressions |
| FNDN-04 | 02-02-PLAN.md | Intermediate pass outputs (typed AST, Index, TypeEnv) exposed as public API from `tenor-core` | SATISFIED | `lib.rs` exports `Index`, `TypeEnv`, `RawConstruct`, `RawType`, `RawExpr`, `RawTerm`, `RawLiteral`, `Provenance` + 4 pass entry functions |
| TEST-01 | 02-04-PLAN.md | CI pipeline runs all conformance suites on every commit | SATISFIED | `.github/workflows/ci.yml` triggers on push/PR to main/v1; runs workspace build + conformance suite + all tests |
| TEST-02 | 02-03-PLAN.md, 02-04-PLAN.md | Elaborator conformance suite extended to cover persona, P7 outcome typing, P5 shared types | SATISFIED | persona_basic, persona_multiple, operation_outcomes, shared_types positive tests + 3 negative tests; 55 total |
| TEST-08 | 02-04-PLAN.md | Interchange JSON Schema validation test -- every elaborator output validates against the formal schema | SATISFIED | `crates/core/tests/schema_validation.rs` passes; validates all positive conformance expected.json files against `docs/interchange-schema.json` |

**No orphaned requirements.** All 7 requirements declared in plan frontmatter have evidence. All 7 appear in REQUIREMENTS.md traceability table as "Complete" for Phase 2.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `README.md` | 9, 155 | Stale conformance count ("47/47" instead of "55/55") | Warning | Documentation inconsistency only; code and CLAUDE.md are correct. Does not block any goal. |

No stub implementations, empty handlers, or TODO markers found in `crates/core/src/` or `crates/cli/src/`.

---

### Human Verification Required

None. All automated checks passed or failed programmatically. The single gap (README count) is a textual discrepancy verifiable by inspection.

---

### Gaps Summary

One gap blocking the "README reflects new crate structure" truth:

**README.md stale conformance count.** The conformance suite was extended from 47 to 55 tests during Plan 02-04. CLAUDE.md was correctly updated to "55/55" but README.md was not. Two occurrences on lines 9 and 155 still read "47/47 conformance tests passing". This is a cosmetic documentation inconsistency, not a functional regression -- the workspace compiles, the conformance suite passes 55/55, and CLAUDE.md is accurate.

The core phase goal is otherwise achieved:
- Workspace refactoring: complete (6 crates, all compile)
- Public API: complete (Index, TypeEnv, AST types re-exported)
- Conformance backward compatibility: complete (55/55 pass)
- v1.0 constructs in conformance suite: complete (persona, outcomes, shared types)
- CI: complete (GitHub Actions pipeline)
- Schema validation: complete (integration test passes)

The README fix requires two single-line edits.

---

_Verified: 2026-02-21T21:00:00Z_
_Verifier: Claude (gsd-verifier)_
