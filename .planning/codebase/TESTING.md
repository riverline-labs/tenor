# Testing Patterns

**Analysis Date:** 2026-02-23

## Test Framework

**Runner:**
- Rust built-in test harness (`cargo test`)
- No external test runner framework (no nextest config found)
- Config: uses Cargo defaults (no test-specific config files)

**Assertion Library:**
- Standard `assert!`, `assert_eq!`, `assert_ne!` macros
- `predicates` crate for CLI output matching (`crates/cli/` dev-dependencies)
- `assert_cmd` crate for spawning the binary in CLI integration tests
- `jsonschema` crate for JSON schema validation in `crates/core/tests/schema_validation.rs`

**Conformance Suite Runner:**
- Custom TAP v14 output via `crates/cli/src/tap.rs`
- Run via: `cargo run -p tenor-cli -- test conformance`
- This is NOT `cargo test` -- it is a separate CLI subcommand that elaborates `.tenor` files and diffs JSON output
- Runner implementation: `crates/cli/src/runner.rs`

**Run Commands:**
```bash
# Full pre-commit quality gate (all 5 steps, in order)
cargo fmt --all
cargo build --workspace
cargo test --workspace
cargo run -p tenor-cli -- test conformance
cargo clippy --workspace -- -D warnings

# Run all cargo tests (unit + integration + schema validation)
cargo test --workspace

# Run conformance suite only (TAP output, 73 tests)
cargo run -p tenor-cli -- test conformance

# Run tests for a specific crate
cargo test -p tenor-core
cargo test -p tenor-eval
cargo test -p tenor-cli
cargo test -p tenor-analyze
cargo test -p tenor-codegen

# No coverage tooling configured; use cargo-tarpaulin if needed:
cargo install cargo-tarpaulin
cargo tarpaulin --workspace
```

## Test Counts (as of analysis date)

| Category | Count | Location |
|----------|-------|----------|
| Conformance suite tests | 73 | `conformance/` (TAP runner) |
| Cargo unit + integration tests | 435 | `crates/*/src/` and `crates/*/tests/` |
| **Total** | **508** | |

**By crate (cargo test):**
- `tenor-eval`: ~125 tests (unit + conformance + numeric regression)
- `tenor-cli`: ~53 tests (CLI integration + serve integration)
- `tenor-analyze`: ~17 tests (S1-S8 analysis integration)
- `tenor-core`: ~8 tests (schema validation + unit)
- `tenor-codegen`: ~4 tests (TypeScript generation)
- Remaining: inline `#[cfg(test)]` module tests in source files across all crates

## Test File Organization

**Location:**
- Integration tests in dedicated `tests/` directories per crate:
  - `crates/core/tests/schema_validation.rs` -- validates all conformance `.expected.json` against `docs/interchange-schema.json`
  - `crates/cli/tests/cli_integration.rs` -- 42 tests for the `tenor` binary via `assert_cmd`
  - `crates/cli/tests/serve_integration.rs` -- 11 tests for the HTTP API server
  - `crates/eval/tests/conformance.rs` -- 45 tests: evaluator fixture tests across 5 domains
  - `crates/eval/tests/numeric_regression.rs` -- 61 tests: numeric precision and arithmetic
  - `crates/analyze/tests/analysis_tests.rs` -- 17 tests: S1-S8 static analysis
  - `crates/codegen/tests/codegen_integration.rs` -- 4 tests: TypeScript code generation
- Unit tests in `#[cfg(test)]` modules at the bottom of source files (31 files contain `#[cfg(test)]`)
  - `crates/eval/src/lib.rs` -- `mod integration_tests` with hand-constructed bundles
  - `crates/eval/src/predicate.rs` -- 18 predicate evaluation tests
  - `crates/eval/src/assemble.rs` -- 21 fact assembly tests
  - `crates/eval/src/numeric.rs` -- 27 numeric model tests
  - `crates/eval/src/types.rs` -- 17 type system tests
  - `crates/eval/src/operation.rs` -- 15 operation evaluation tests
  - `crates/eval/src/flow.rs` -- 9 flow execution tests
  - `crates/eval/src/rules.rs` -- 9 rule evaluation tests
  - `crates/eval/src/provenance.rs` -- 5 provenance tracking tests
  - `crates/cli/src/diff.rs` -- 23 bundle diff tests
  - `crates/cli/src/explain.rs` -- 3 explain formatter tests
  - `crates/cli/src/ambiguity/compare.rs` -- 8 ambiguity comparison tests
  - `crates/cli/src/ambiguity/prompt.rs` -- 6 prompt template tests
  - `crates/cli/src/ambiguity/fixtures.rs` -- 2 fixture loading tests
  - `crates/analyze/src/*.rs` -- tests across all 8 analysis modules
  - `crates/codegen/src/*.rs` -- tests across all codegen modules
  - `crates/core/src/pass3_types.rs` -- 3 type resolution tests
  - `crates/core/src/pass5_validate.rs` -- 4 validation tests

**Naming:**
- Integration test files named by scope: `cli_integration.rs`, `schema_validation.rs`, `conformance.rs`, `analysis_tests.rs`
- Individual `#[test]` functions named descriptively in snake_case:
  - CLI: `help_exits_0_with_description`, `elaborate_valid_file_exits_0`, `e10_manifest_valid_schema`
  - Eval: `fact_bool_basic`, `rule_multi_stratum`, `domain_healthcare_approve`
  - Numeric: `int_compare_equal`, `decimal_round_midpoint_even_down`, `money_compare_different_currency`
  - Analysis: `test_s1_entity_basic`, `test_s2_all_reachable`, `test_analyze_selected`

## Conformance Suite Structure

**Runner:** `crates/cli/src/runner.rs` (invoked via `cargo run -p tenor-cli -- test conformance`)

**How it works:**
1. Globs `.tenor` files in each subdirectory of `conformance/`
2. Elaborates each via `tenor_core::elaborate::elaborate()`
3. For positive tests: compares output JSON exactly against `.expected.json` using deep JSON equality (normalizing number types)
4. For negative tests: compares error JSON against `.expected-error.json`
5. Outputs TAP v14 format to stdout
6. Exits non-zero if any test fails

**Fixture directories (73 tests total):**

| Directory | Count | Test Type | Description |
|-----------|-------|-----------|-------------|
| `conformance/positive/` | 21 | Positive | Valid DSL produces expected interchange JSON |
| `conformance/negative/pass0/` | 2 | Negative | Lexer errors (bad tokens, unterminated strings) |
| `conformance/negative/pass1/` | 5 | Negative | Import resolution errors (cycles, duplicates, escapes) |
| `conformance/negative/pass2/` | 4 | Negative | Duplicate construct ID detection |
| `conformance/negative/pass3/` | 2 | Negative | TypeDecl cycle detection |
| `conformance/negative/pass4/` | 6 | Negative | Type checking errors (unresolved refs, type mismatches) |
| `conformance/negative/pass5/` | 22 | Negative | Structural validation (entities, flows, operations, rules) |
| `conformance/numeric/` | 3 | Positive | Decimal/money precision serialization |
| `conformance/promotion/` | 2 | Positive | Numeric type promotion |
| `conformance/shorthand/` | 2 | Positive | DSL shorthand expansion |
| `conformance/cross_file/` | 1 | Positive | Multi-file import bundle assembly |
| `conformance/parallel/` | 2 | Negative | Parallel entity conflict detection |
| `conformance/manifest/` | 1 | Positive | Manifest envelope generation |

**Positive test fixture format:**
```
conformance/positive/
  fact_basic.tenor              -- Tenor DSL source
  fact_basic.expected.json      -- Expected interchange JSON output
```

**Negative test fixture format:**
```
conformance/negative/pass5/
  entity_initial_not_in_states.tenor              -- Tenor DSL with error
  entity_initial_not_in_states.expected-error.json -- Expected error JSON
```

**Error JSON format:**
```json
{
  "pass": 5,
  "construct_kind": "Entity",
  "construct_id": "Order",
  "field": "initial",
  "file": "entity_initial_not_in_states.tenor",
  "line": 7,
  "message": "initial state 'draft' is not declared in states: [submitted, approved, rejected]"
}
```

**Multi-file negative tests:** Some negative tests use multiple `.tenor` files (e.g., `dup_across_files_a.tenor` + `dup_across_files_b.tenor`). The runner skips files ending in `_b.tenor` as roots and only uses them as imports.

## Evaluator Conformance Tests

**Location:** `crates/eval/tests/conformance.rs` (45 tests, run via `cargo test -p tenor-eval`)

**Fixture format (three files per test):**
```
conformance/eval/positive/
  fact_bool_basic.tenor         -- Tenor DSL source
  fact_bool_basic.facts.json    -- Fact values for evaluation
  fact_bool_basic.verdicts.json -- Expected verdict output
```

**Three fixture directories:**
- `conformance/eval/positive/` -- 34 fixtures per file trio (rules, operations, flows, domain contracts)
- `conformance/eval/frozen/` -- 3 fixtures for frozen verdict/fact edge cases
- `conformance/eval/numeric/` -- 4 fixtures for numeric precision evaluation

**Domain validation fixtures (run as eval conformance tests):**
- `domains/saas/saas_activate.facts.json` / `saas_activate.verdicts.json`
- `domains/healthcare/prior_auth_approve.facts.json` / `prior_auth_approve.verdicts.json`
- `domains/supply_chain/inspection_pass.facts.json` / `inspection_pass.verdicts.json`
- `domains/trade_finance/lc_present.facts.json` / `lc_present.verdicts.json`
- `domains/energy_procurement/rfp_approve.facts.json` / `rfp_approve.verdicts.json`

**Executor conformance tests (E10-E13):** Inline in `crates/cli/tests/cli_integration.rs` and `crates/eval/tests/conformance.rs`:
- `e10_manifest_valid_schema` -- validates manifest structure
- `e11_cold_start_completeness` -- validates bundle self-containment
- `e12_etag_determinism` -- validates etag determinism
- `e12_etag_change_detection` -- validates different contracts produce different etags
- `e13_dry_run_rule_evaluation` -- validates determinism and side-effect-free rule evaluation
- `e13_dry_run_healthcare_determinism` -- validates multi-stratum determinism

## Analysis Tests

**Location:** `crates/analyze/tests/analysis_tests.rs` (17 tests)

**Analysis-specific fixtures:**
- `conformance/analysis/dead_states.tenor` -- S2 dead state detection
- `conformance/analysis/authority_basic.tenor` -- S4 authority topology
- `conformance/analysis/flow_branching.tenor` -- S6 flow path analysis
- `conformance/analysis/system_authority.tenor` -- S4 cross-contract authority
- `conformance/analysis/system_flow_trigger.tenor` -- S6 cross-contract flow paths

## Test Structure Patterns

**Suite Organization Pattern (eval/tests/conformance.rs):**
```rust
// Helper functions at top
fn run_eval_fixture(fixture_dir: &Path, name: &str) { ... }
fn run_eval_fixture_error(fixture_dir: &Path, name: &str) { ... }
fn run_eval_flow_fixture(fixture_dir: &Path, name: &str, flow_id: &str, persona: &str) { ... }

// Directory helpers
fn positive_dir() -> PathBuf { ... }
fn frozen_dir() -> PathBuf { ... }
fn numeric_dir() -> PathBuf { ... }

// Section comments group related tests
// ──────────────────────────────────────────────
// Positive evaluation fixtures (15+)
// ──────────────────────────────────────────────
#[test]
fn fact_bool_basic() {
    run_eval_fixture(&positive_dir(), "fact_bool_basic");
}
```

**Unit Test Module Pattern:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluate_simple_contract() {
        let bundle = serde_json::json!({ ... });
        let facts = serde_json::json!({ ... });
        let result = evaluate(&bundle, &facts).unwrap();
        assert_eq!(result.verdicts.0.len(), 3);
        assert!(result.verdicts.has_verdict("account_active"));
    }
}
```

**Analysis test pattern:**
```rust
fn elaborate_and_analyze(fixture: &str) -> tenor_analyze::AnalysisReport {
    let bundle = elaborate_fixture(fixture);
    tenor_analyze::analyze(&bundle).unwrap_or_else(|e| {
        panic!("analysis failed for {}: {}", fixture, e);
    })
}

#[test]
fn test_s1_entity_basic() {
    let report = elaborate_and_analyze("conformance/positive/entity_basic.tenor");
    let s1 = report.s1_state_space.expect("S1 should be populated");
    assert!(s1.entities.contains_key("Order"));
    assert_eq!(s1.entities["Order"].state_count, 6);
}
```

## Mocking

**Framework:** None -- no mocking library used anywhere in the codebase.

**Strategy:**
- Test bundles constructed as inline `serde_json::json!({...})` values (no mocking needed)
- Real files used for conformance tests (elaborate actual `.tenor` fixtures)
- `TempDir` from `tempfile` crate for CLI tests that need temporary filesystem files
- The test strategy favors real end-to-end elaboration over mocking

**TempDir pattern for CLI tests:**
```rust
let tmp = TempDir::new().unwrap();
let path = tmp.path().join("bad.json");
fs::write(&path, r#"{"not": "a bundle"}"#).unwrap();
```

**What NOT to Mock:**
- The elaborator pipeline (always run real elaboration in evaluator tests)
- File I/O (use real fixture files from `conformance/` directory)

## Fixtures and Factories

**Inline JSON bundle builders (crates/eval/tests/numeric_regression.rs):**
```rust
fn comparison_bundle(
    fact_id: &str,
    fact_type: serde_json::Value,
    op: &str,
    right: serde_json::Value,
    comparison_type: Option<serde_json::Value>,
) -> serde_json::Value {
    json!({
        "id": "numeric_test",
        "kind": "Bundle",
        "tenor": "1.0",
        "tenor_version": "1.1.0",
        "constructs": [ ... ]
    })
}

fn two_fact_comparison_bundle(...) -> serde_json::Value { ... }
fn mul_payload_bundle(...) -> serde_json::Value { ... }
```

**Assertion helpers (crates/eval/tests/numeric_regression.rs):**
```rust
fn assert_verdict_produced(bundle: &serde_json::Value, facts: &serde_json::Value) {
    let result = tenor_eval::evaluate(bundle, facts).expect("evaluation should succeed");
    assert!(result.verdicts.has_verdict("result"), "expected verdict 'result'");
}
fn assert_no_verdict(bundle: &serde_json::Value, facts: &serde_json::Value) { ... }
fn assert_eval_error(bundle: &serde_json::Value, facts: &serde_json::Value) { ... }
fn assert_mul_int(bundle: &serde_json::Value, facts: &serde_json::Value, expected: i64) { ... }
```

**Fixture locations:**
| Category | Path |
|----------|------|
| Elaborator conformance | `conformance/positive/`, `conformance/negative/`, etc. |
| Evaluator conformance | `conformance/eval/positive/`, `conformance/eval/frozen/`, `conformance/eval/numeric/` |
| Analysis fixtures | `conformance/analysis/` |
| Ambiguity fixtures | `conformance/ambiguity/` |
| Domain contracts | `domains/saas/`, `domains/healthcare/`, `domains/supply_chain/`, `domains/trade_finance/`, `domains/energy_procurement/` |
| CLI test fixtures | `crates/cli/tests/fixtures/` |

## CLI Integration Tests

**Framework:** `assert_cmd` + `predicates` crates

**Location:** `crates/cli/tests/cli_integration.rs` (42 tests)

**Pattern:**
```rust
use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use predicates::prelude::*;

fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().and_then(|p| p.parent()).expect("workspace root").to_path_buf()
}

fn tenor() -> Command {
    let mut cmd = cargo_bin_cmd!("tenor");
    cmd.current_dir(workspace_root());
    cmd
}

#[test]
fn elaborate_valid_file_exits_0() {
    tenor()
        .args(["elaborate", "conformance/positive/fact_basic.tenor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"kind\": \"Bundle\""));
}

#[test]
fn elaborate_nonexistent_file_exits_1() {
    tenor()
        .args(["elaborate", "nonexistent_file_xyz.tenor"])
        .assert()
        .failure()
        .code(1);
}
```

**All subcommands tested:** elaborate, validate, eval, test, diff, check, explain, generate

## Serve Integration Tests

**Location:** `crates/cli/tests/serve_integration.rs` (11 tests)

**Pattern:** Spawns the `tenor serve` process, makes raw TCP HTTP requests, verifies responses.

```rust
static NEXT_PORT: AtomicU16 = AtomicU16::new(0);

fn start_server(port: u16, contracts: &[&str]) -> Child { ... }
fn http_get(port: u16, path: &str) -> (u16, String) { ... }
fn http_post(port: u16, path: &str, body: &str) -> (u16, String) { ... }

#[test]
fn health_returns_200_with_version() {
    let port = next_port();
    let mut child = start_server(port, &[]);
    let (status, body) = http_get(port, "/health");
    child.kill().ok();
    assert_eq!(status, 200);
}
```

**Port conflict avoidance:** Uses `AtomicU16` starting from `20000 + (pid % 20000)` so parallel test runs don't collide.

## Common Patterns

**Path resolution in tests:**
```rust
fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().and_then(|p| p.parent()).expect("workspace root").to_path_buf()
}
```

**Fixture runner pattern:**
```rust
fn run_eval_fixture(fixture_dir: &Path, name: &str) {
    let tenor_path = fixture_dir.join(format!("{}.tenor", name));
    let facts_path = fixture_dir.join(format!("{}.facts.json", name));
    let expected_path = fixture_dir.join(format!("{}.verdicts.json", name));

    let bundle = tenor_core::elaborate::elaborate(&tenor_path)
        .unwrap_or_else(|e| panic!("Failed to elaborate {}: {:?}", name, e));
    // ... load facts, evaluate, compare ...
    assert_eq!(actual, expected, "Verdict mismatch for {}\n...", name);
}
```

**Error testing:**
```rust
#[test]
fn evaluate_missing_required_fact() {
    let result = evaluate(&bundle, &facts);
    assert!(result.is_err());
    if let Err(EvalError::MissingFact { fact_id }) = result {
        assert_eq!(fact_id, "required");
    } else {
        panic!("expected MissingFact error");
    }
}
```

**Determinism testing:**
```rust
let r1 = tenor_eval::evaluate(&bundle, &facts).expect("eval 1 failed");
let r2 = tenor_eval::evaluate(&bundle, &facts).expect("eval 2 failed");
let r3 = tenor_eval::evaluate(&bundle, &facts).expect("eval 3 failed");
assert_eq!(r1.verdicts.to_json(), r2.verdicts.to_json());
assert_eq!(r2.verdicts.to_json(), r3.verdicts.to_json());
```

**Async testing:** Not applicable -- all code is synchronous (no async runtime).

## CI/CD Pipeline

**Config:** `.github/workflows/ci.yml`

**Triggers:** Push and PR to `main` and `v1` branches.

**Steps (in order):**
1. `cargo build --workspace` -- compile check
2. `cargo run -p tenor-cli -- test conformance` -- 73 elaborator conformance tests
3. `cargo test --workspace` -- 435 unit + integration tests
4. `cargo fmt --all -- --check` -- formatting check
5. `cargo clippy --workspace -- -D warnings` -- lint check (warnings are errors)

**Toolchain:** Uses `dtolnay/rust-toolchain@stable` with `Swatinem/rust-cache@v2` for dependency caching.

## Coverage

**Requirements:** No explicit coverage targets enforced. No coverage tooling configured.

**Well-tested areas:**
- Elaborator pipeline: Full conformance suite (73 tests) covers all 6 passes
- Evaluator: 125+ tests covering rules, operations, flows, domain contracts, numeric precision
- Static analysis: S1-S8 all have dedicated integration tests
- CLI: All subcommands tested for success/failure exit codes and output content
- HTTP API: All endpoints tested (health, contracts, operations, elaborate, evaluate, explain)
- Numeric model: 61 dedicated regression tests for Int/Decimal/Money arithmetic

**Under-tested areas:**
- `crates/lsp/` -- no dedicated test files found; LSP server has no tests
- `crates/core/src/parser.rs` (1,598 lines) -- tested only through conformance suite, no unit tests
- `crates/core/src/pass5_validate.rs` (1,506 lines) -- only 4 inline tests
- `crates/core/src/lexer.rs` (419 lines) -- tested only through conformance suite, no unit tests

## Adding New Tests

**New conformance test (elaborator):**
1. Create `conformance/positive/<name>.tenor` with valid DSL
2. Run `cargo run -p tenor-cli -- elaborate conformance/positive/<name>.tenor > conformance/positive/<name>.expected.json`
3. Verify the `.expected.json` is correct
4. Run `cargo run -p tenor-cli -- test conformance` to verify it passes

**New negative conformance test:**
1. Create `conformance/negative/pass<N>/<name>.tenor` with invalid DSL
2. Run `cargo run -p tenor-cli -- elaborate conformance/negative/pass<N>/<name>.tenor 2> conformance/negative/pass<N>/<name>.expected-error.json` (the error goes to stderr in JSON format)
3. Verify the `.expected-error.json` is correct
4. Run `cargo run -p tenor-cli -- test conformance` to verify it passes

**New evaluator conformance test:**
1. Create the `.tenor` source in `conformance/eval/positive/`
2. Create `.facts.json` with test fact values
3. Create `.verdicts.json` with expected verdict output
4. Add a `#[test]` function in `crates/eval/tests/conformance.rs` calling `run_eval_fixture()`

**New unit test:**
1. Add a `#[test]` function inside the `#[cfg(test)] mod tests { ... }` block in the relevant source file
2. Use `serde_json::json!({...})` for inline test data construction

---

*Testing analysis: 2026-02-23*
