# Testing Patterns

**Analysis Date:** 2026-02-22

## Test Framework

**Runner:**
- Rust built-in test harness (`cargo test`)
- No external test runner framework (no nextest config found)
- Config: none (uses Cargo defaults)

**Assertion Library:**
- Standard `assert!`, `assert_eq!`, `assert_ne!` macros
- `predicates` crate for CLI output matching (in `crates/cli/` dev-dependencies)
- `assert_cmd` crate for spawning the binary in CLI integration tests
- `jsonschema` crate for JSON schema validation

**Conformance suite runner:**
- Custom TAP v14 output via `crates/cli/src/tap.rs`
- Run via: `cargo run -p tenor-cli -- test conformance`
- This is NOT `cargo test` -- it is a separate subcommand that elaborates `.tenor` files and diffs JSON output

**Run Commands:**
```bash
# Run all cargo tests (unit + integration + schema validation)
cargo test --workspace

# Run conformance suite (TAP output, tests elaborator end-to-end)
cargo run -p tenor-cli -- test conformance

# Run tests for a specific crate
cargo test -p tenor-core
cargo test -p tenor-eval
cargo test -p tenor-cli

# Check conformance then tests (as required before every commit)
cargo fmt --all
cargo build --workspace
cargo test --workspace
cargo run -p tenor-cli -- test conformance
cargo clippy --workspace -- -D warnings
```

## Test File Organization

**Location:**
- Integration tests in dedicated `tests/` directories per crate (co-located with crate, not with src):
  - `crates/core/tests/schema_validation.rs`
  - `crates/cli/tests/cli_integration.rs`
  - `crates/eval/tests/conformance.rs`
  - `crates/eval/tests/numeric_regression.rs`
  - `crates/analyze/tests/analysis_tests.rs`
- Unit tests in `#[cfg(test)]` modules at the bottom of source files:
  - `crates/eval/src/lib.rs` contains `mod integration_tests { ... }`
  - `crates/analyze/src/lib.rs` contains `mod tests { ... }`
  - `crates/cli/src/ambiguity/compare.rs` contains inline tests
  - `crates/cli/src/diff.rs` contains inline tests

**Naming:**
- Integration test files named by scope: `cli_integration.rs`, `schema_validation.rs`, `conformance.rs`, `analysis_tests.rs`
- Individual `#[test]` functions named descriptively in snake_case:
  - `help_exits_0_with_description`, `elaborate_valid_file_exits_0`, `validate_all_positive_conformance_outputs_against_schema`
  - `fact_bool_basic`, `rule_multi_stratum`, `domain_healthcare_approve`
  - `int_compare_equal`, `decimal_round_midpoint_even_down`, `money_compare_different_currency`
  - `test_s1_entity_basic`, `test_s2_all_reachable`, `test_analyze_selected_s4_pulls_s3a`

## Test Structure

**Suite Organization (crates/eval/tests/conformance.rs):**
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

**Unit Test Module (crates/eval/src/lib.rs):**
```rust
#[cfg(test)]
mod integration_tests {
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

**Analysis test pattern (crates/analyze/tests/analysis_tests.rs):**
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
    assert_eq!(order.state_count, 6);
}
```

**Patterns:**
- Setup: helper functions build reusable fixtures, no `before_each` hooks
- Teardown: none (no persistent state; `TempDir` from `tempfile` crate for CLI tests needing temp files)
- Assertion: `assert!`, `assert_eq!`, `assert_ne!`, `.expect("message")` for Options

## Mocking

**Framework:** None - no mocking library used.

**Patterns:**
- Test bundles constructed as inline `serde_json::json!({...})` values (no mocking needed)
- Real files used for conformance tests (elaborate actual `.tenor` fixtures)
- `TempDir` for CLI tests that need temporary filesystem files:
  ```rust
  let tmp = TempDir::new().unwrap();
  let path = tmp.path().join("bad.json");
  fs::write(&path, r#"{"not": "a bundle"}"#).unwrap();
  ```

**What to Mock:**
- Nothing -- the test strategy favors real end-to-end elaboration over mocking

**What NOT to Mock:**
- The elaborator pipeline (always run real elaboration in evaluator tests)
- File I/O (use real fixture files from `conformance/` directory)

## Fixtures and Factories

**Conformance Fixtures:**
- Located in `conformance/` at workspace root
- Each fixture is a triplet of files:
  - `<name>.tenor` -- Tenor DSL source
  - `<name>.expected.json` -- expected interchange JSON (positive tests)
  - `<name>.expected-error.json` -- expected error JSON (negative tests)
- Eval fixtures add:
  - `<name>.facts.json` -- fact values for evaluation
  - `<name>.verdicts.json` -- expected verdict output
- Subdirectories: `positive/`, `negative/pass0/` ... `negative/pass5/`, `numeric/`, `promotion/`, `shorthand/`, `cross_file/`, `parallel/`, `eval/positive/`, `eval/frozen/`, `eval/numeric/`, `manifest/`

**Inline JSON bundles (numeric_regression.rs):**
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
```

**Assertion helpers:**
```rust
fn assert_verdict_produced(bundle: &serde_json::Value, facts: &serde_json::Value) {
    let result = tenor_eval::evaluate(bundle, facts).expect("evaluation should succeed");
    assert!(result.verdicts.has_verdict("result"), "expected verdict 'result'");
}

fn assert_no_verdict(bundle: &serde_json::Value, facts: &serde_json::Value) { ... }
fn assert_eval_error(bundle: &serde_json::Value, facts: &serde_json::Value) { ... }
fn assert_mul_int(bundle: &serde_json::Value, facts: &serde_json::Value, expected: i64) { ... }
```

**Location:**
- Conformance fixtures: `conformance/` (workspace root)
- Domain test data: `domains/saas/`, `domains/healthcare/`, `domains/supply_chain/`, `domains/trade_finance/`, `domains/energy_procurement/`
- CLI test fixtures: `crates/cli/tests/fixtures/` (e.g., `eval_basic_bundle.json`, `eval_basic.facts.json`)

## Coverage

**Requirements:** No explicit coverage targets enforced. No `cargo-tarpaulin` or similar configured.

**View Coverage:**
```bash
# No coverage tooling configured -- use cargo-tarpaulin if needed:
cargo install cargo-tarpaulin
cargo tarpaulin --workspace
```

## Test Types

**Conformance Tests (primary validation):**
- Scope: End-to-end elaboration pipeline correctness
- Runner: `cargo run -p tenor-cli -- test conformance` (TAP v14 output)
- How it works: For every `.tenor` file in `conformance/`, elaborate it and compare output JSON exactly against `.expected.json`, or compare error JSON against `.expected-error.json`
- Fixture directories tested: `positive/`, `negative/pass0..pass5/`, `numeric/`, `promotion/`, `shorthand/`, `cross_file/`, `parallel/`, `manifest/`

**Schema Validation Tests:**
- Location: `crates/core/tests/schema_validation.rs`
- Scope: All positive `.expected.json` conformance outputs validate against `docs/interchange-schema.json`
- Run via `cargo test -p tenor-core`

**Unit Tests (inline):**
- Location: `#[cfg(test)]` modules within source files
- Scope: Public API of the module, using constructed JSON inputs
- Examples in `crates/eval/src/lib.rs` and `crates/analyze/src/lib.rs`

**Integration Tests:**
- `crates/cli/tests/cli_integration.rs` - Binary CLI integration via `assert_cmd`
- `crates/eval/tests/conformance.rs` - Evaluator fixture tests (elaborate + evaluate + compare verdicts)
- `crates/eval/tests/numeric_regression.rs` - 50+ numeric precision cases using inline JSON bundles
- `crates/analyze/tests/analysis_tests.rs` - S1-S8 static analysis tests against real fixtures

**CLI Integration Test Pattern:**
```rust
use assert_cmd::Command;
use predicates::prelude::*;

fn tenor() -> Command {
    let mut cmd = Command::cargo_bin("tenor").expect("binary exists");
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

## Common Patterns

**Path resolution in tests:**
All test crates resolve workspace root from `env!("CARGO_MANIFEST_DIR")`:
```rust
fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    // crates/cli -> workspace root is two levels up
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
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

    // ... load and evaluate ...

    assert_eq!(
        actual,
        expected,
        "Verdict mismatch for {}\n\nActual:\n{}\n\nExpected:\n{}",
        name,
        serde_json::to_string_pretty(&actual).unwrap(),
        serde_json::to_string_pretty(&expected).unwrap(),
    );
}
```

**Error Testing:**
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

**Determinism Testing:**
```rust
// Run evaluation multiple times, verify identical output
let r1 = tenor_eval::evaluate(&bundle, &facts).expect("eval 1 failed");
let r2 = tenor_eval::evaluate(&bundle, &facts).expect("eval 2 failed");
let r3 = tenor_eval::evaluate(&bundle, &facts).expect("eval 3 failed");
assert_eq!(r1.verdicts.to_json(), r2.verdicts.to_json(), "must be deterministic (run 1 vs 2)");
assert_eq!(r2.verdicts.to_json(), r3.verdicts.to_json(), "must be deterministic (run 2 vs 3)");
```

**Async Testing:**
- Not applicable -- all code is synchronous (no async runtime)

## Conformance Suite Structure

The conformance runner (`crates/cli/src/runner.rs`) is invoked as a CLI subcommand, not as cargo tests. It:
1. Globs `.tenor` files in each subdirectory
2. Elaborates each one
3. Compares JSON output to `.expected.json` (or error JSON to `.expected-error.json`)
4. Outputs TAP v14 format to stdout
5. Exits non-zero if any test fails

CI runs both `cargo test --workspace` AND `cargo run -p tenor-cli -- test conformance` as separate steps.

---

*Testing analysis: 2026-02-22*
