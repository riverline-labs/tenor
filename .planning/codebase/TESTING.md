# Testing Patterns

**Analysis Date:** 2026-02-21

## Test Framework

**Runner:**
- Rust built-in test harness (`cargo test`) — no third-party test runner
- Edition 2021, Rust stable toolchain (pinned via `dtolnay/rust-toolchain@stable` in CI)

**Assertion Library:**
- Built-in `assert!`, `assert_eq!`, `assert_ne!` — no third-party assertion crate
- CLI integration tests use `predicates` crate for fluent stdout/stderr matching

**CLI Integration Test Dependencies:**
- `assert_cmd = "2"` — spawn binary as subprocess, check exit codes
- `predicates = "3"` — fluent string matching on stdout/stderr
- `tempfile = "3"` — temporary files/directories for invalid-input tests
- Schema validation tests use `jsonschema = "0.42"`

**Run Commands:**
```bash
cargo test --workspace              # Run all unit and integration tests
cargo run -p tenor-cli -- test conformance  # Run conformance suite (TAP output)
cargo test -p tenor-core            # Core library tests only
cargo test -p tenor-eval            # Evaluator tests only
cargo test -p tenor-cli             # CLI integration tests only
cargo test --workspace 2>&1 | grep "test result"  # Summary only
```

## Test File Organization

**Location:** Tests are separated into two locations depending on type:
- **External test files**: `crates/{name}/tests/` directory — integration and conformance tests
- **Inline test modules**: `#[cfg(test)] mod tests { ... }` at the bottom of source files — unit tests

**Naming:**
- External test files: `tests/cli_integration.rs`, `tests/schema_validation.rs`, `tests/conformance.rs`, `tests/numeric_regression.rs`
- Inline test modules: named `tests` by convention — `mod tests { ... }`
- Individual test functions: snake_case describing behavior — `int_compare_equal`, `elaborate_valid_file_exits_0`, `evaluate_simple_contract`

**Structure:**
```
crates/
  core/
    src/           # Source with inline #[cfg(test)] modules
    tests/
      schema_validation.rs  # Validates all expected JSONs against formal schema
  cli/
    src/           # Inline unit tests in ambiguity/*, diff.rs
    tests/
      cli_integration.rs    # Subprocess tests for all subcommands
  eval/
    src/           # Inline unit tests in all modules
    tests/
      conformance.rs         # Fixture-driven evaluator conformance
      numeric_regression.rs  # 50+ numeric precision regression cases
```

## Test Structure

**Suite Organization:**
```rust
// Section comments separate test categories in large files
// ──────────────────────────────────────────────
// A. Int arithmetic (5 cases)
// ──────────────────────────────────────────────

#[test]
fn int_compare_equal() {
    let bundle = comparison_bundle(/* ... */);
    assert_verdict_produced(&bundle, &json!({"x": 42}));
}
```

**Inline module pattern:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Local helper functions
    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn make_rule(id: &str, stratum: u32, /* ... */) -> Rule { /* ... */ }

    #[test]
    fn test_name_describes_behavior() {
        // Arrange
        let value = Value::Bool(true);
        // Act + Assert
        assert_eq!(value.type_name(), "Bool");
    }
}
```

**Patterns:**
- No `before_each` / setup/teardown — each test is fully self-contained
- Helper functions in the same file construct test data and assert patterns
- Long tests use comments to mark `// Arrange`, `// Act`, `// Assert` sections implicitly through naming
- Error path tests use `assert!(result.is_err())` or pattern match on the error variant

## Mocking

**Framework:** None — no mock crate (e.g., mockall) is used.

**Patterns:**
- Integration tests construct real interchange JSON bundles directly using `serde_json::json!()` macro
- No filesystem mocking — tests write temp files via `tempfile::TempDir`
- No network mocking — the ambiguity test module (which calls an LLM API) is tested for structure, not live calls
- External collaborators are replaced by constructing their inputs directly (bundle JSON, facts JSON)

**What to Mock:**
- Not applicable — the codebase avoids mocking by testing at boundaries (JSON in, JSON out)

**What NOT to Mock:**
- The elaborator (tenor-core) in evaluator tests — `tenor_core::elaborate::elaborate()` is called as-is
- The filesystem — tests use real fixture files from `conformance/` directory

## Fixtures and Factories

**Conformance Fixtures (DSL tests):**
```
conformance/
  positive/
    fact_basic.tenor           # DSL source input
    fact_basic.expected.json   # Expected interchange JSON output
  negative/
    pass{N}/
      unresolved_fact_ref.tenor
      unresolved_fact_ref.expected-error.json  # Expected error fields
  numeric/   # Decimal/money precision fixtures
  promotion/ # Type promotion fixtures
  shorthand/ # DSL shorthand expansion fixtures
  cross_file/ # Multi-file import fixtures
  parallel/  # Parallel entity conflict fixtures
  eval/
    positive/  # .tenor + .facts.json + .verdicts.json triplets
    frozen/    # Frozen verdict edge cases
    numeric/   # Numeric evaluation precision fixtures
```

**In-Code Bundle Builders (numeric_regression.rs):**
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
        "tenor_version": "1.0.0",
        "constructs": [/* ... */]
    })
}

fn two_fact_comparison_bundle(/* ... */) -> serde_json::Value { /* ... */ }
fn mul_payload_bundle(/* ... */) -> serde_json::Value { /* ... */ }
```

**Assertion Helpers:**
```rust
fn assert_verdict_produced(bundle: &serde_json::Value, facts: &serde_json::Value) {
    let result = tenor_eval::evaluate(bundle, facts).expect("evaluation should succeed");
    assert!(result.verdicts.has_verdict("result"), "expected verdict 'result'");
}

fn assert_no_verdict(bundle: &serde_json::Value, facts: &serde_json::Value) { /* ... */ }
fn assert_eval_error(bundle: &serde_json::Value, facts: &serde_json::Value) { /* ... */ }
fn assert_mul_int(bundle: &serde_json::Value, facts: &serde_json::Value, expected: i64) { /* ... */ }
```

**Location:**
- Conformance fixture files: `/Users/bwb/src/rll/tenor/conformance/`
- Evaluator fixture files: `/Users/bwb/src/rll/tenor/conformance/eval/`
- CLI fixture files: `/Users/bwb/src/rll/tenor/crates/cli/tests/fixtures/`
- In-code helper functions: defined at the top of each test file before test functions

## Coverage

**Requirements:** No enforced coverage threshold — no `cargo-tarpaulin` or coverage tooling in CI.

**View Coverage:**
```bash
# Not configured. To add coverage manually:
cargo install cargo-tarpaulin
cargo tarpaulin --workspace
```

**CI enforces correctness instead of coverage metrics:**
- 55/55 conformance tests passing (conformance suite via TAP)
- All cargo test passing
- Schema validation: all `*.expected.json` files validate against formal JSON Schema
- Zero clippy warnings

## Test Types

**Conformance Suite (primary correctness mechanism):**
- Scope: Positive tests (DSL → expected JSON), negative tests (DSL → expected error), cross-file, parallel, numeric, promotion, shorthand
- Runner: Custom TAP v14 runner in `crates/cli/src/runner.rs`, invoked via `cargo run -p tenor-cli -- test conformance`
- Output: TAP format printed to stdout; CI checks exit code
- Location: Fixture files in `conformance/`, runner code in `crates/cli/src/runner.rs`

**Unit Tests:**
- Scope: Individual types, functions, value operations, numeric arithmetic
- Location: `#[cfg(test)] mod tests { ... }` at the bottom of each source file
- Key files: `crates/eval/src/types.rs`, `crates/eval/src/numeric.rs`, `crates/eval/src/rules.rs`, `crates/eval/src/operation.rs`, `crates/cli/src/diff.rs`

**Integration Tests (CLI subprocess):**
- Scope: All CLI subcommands — elaborate, validate, eval, test, diff, check, explain, generate
- Location: `crates/cli/tests/cli_integration.rs`
- Framework: `assert_cmd` spawns real binary subprocess; tests exit codes, stdout content, stderr content
- Fixture dependency: CLI integration tests reference files in `conformance/` and `crates/cli/tests/fixtures/`

**Schema Validation Tests:**
- Scope: All positive `*.expected.json` conformance fixtures against the formal interchange JSON schema
- Location: `crates/core/tests/schema_validation.rs`
- Schema file: `docs/interchange-schema.json`
- Runs as a single `#[test]` that validates every file and collects failures before asserting

**Evaluator Conformance Tests:**
- Scope: Full pipeline (elaborate DSL → evaluate with facts → compare verdicts)
- Location: `crates/eval/tests/conformance.rs`
- Pattern: Each fixture is a named `#[test]` function calling `run_eval_fixture()` or `run_eval_flow_fixture()`
- Fixture triplets: `<name>.tenor` + `<name>.facts.json` + `<name>.verdicts.json`

**Numeric Regression Tests:**
- Scope: 50+ value-level arithmetic and comparison cases for all numeric types (Int, Decimal, Money, cross-type, edge cases)
- Location: `crates/eval/tests/numeric_regression.rs`
- Pattern: Construct minimal bundles in code using builder helpers; run evaluator; assert verdict presence/absence
- Organized in categories A–F with section comment headers

## Common Patterns

**Fixture-Driven Test:**
```rust
fn run_eval_fixture(fixture_dir: &Path, name: &str) {
    let tenor_path = fixture_dir.join(format!("{}.tenor", name));
    let facts_path = fixture_dir.join(format!("{}.facts.json", name));
    let expected_path = fixture_dir.join(format!("{}.verdicts.json", name));

    let bundle = tenor_core::elaborate::elaborate(&tenor_path)
        .unwrap_or_else(|e| panic!("Failed to elaborate {}: {:?}", name, e));
    // ... load, evaluate, compare
    assert_eq!(actual, expected, "Verdict mismatch for {}\n\nActual:\n{}\n\nExpected:\n{}", ...);
}

#[test]
fn fact_bool_basic() {
    run_eval_fixture(&positive_dir(), "fact_bool_basic");
}
```

**CLI Integration Test:**
```rust
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
```

**Error Path Testing:**
```rust
#[test]
fn evaluate_missing_required_fact() {
    let bundle = serde_json::json!({ /* ... */ });
    let facts = serde_json::json!({});
    let result = evaluate(&bundle, &facts);
    assert!(result.is_err());
    if let Err(EvalError::MissingFact { fact_id }) = result {
        assert_eq!(fact_id, "required");
    } else {
        panic!("expected MissingFact error");
    }
}
```

**Both True and False Cases:**
Numeric regression tests routinely assert both the positive and negative case in a single test:
```rust
#[test]
fn int_compare_less() {
    let bundle = comparison_bundle(/* ... */);
    assert_verdict_produced(&bundle, &json!({"x": 42}));   // true case
    assert_no_verdict(&bundle, &json!({"x": 100}));         // false case
}
```

---

*Testing analysis: 2026-02-21*
