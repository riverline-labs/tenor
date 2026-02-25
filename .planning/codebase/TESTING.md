# Testing Patterns

**Analysis Date:** 2026-02-25

## Test Framework

**Runner:**
- Rust built-in `#[test]` macro with `cargo test`
- Config: No custom test configuration; uses Cargo defaults
- Integration tests in separate `tests/` directories per crate (Rust convention)

**Assertion Library:**
- Standard Rust `assert!()`, `assert_eq!()`, `assert!()` macros
- External crate `predicates` for predicate-based assertions in CLI tests
- `assert_cmd` crate for spawning and testing binary output (exit codes, stdout, stderr)
- `serde_json` equality assertions with pretty-printed diff output for debugging

**Run Commands:**
```bash
cargo test --workspace              # Run all tests
cargo test --workspace -- --test-threads=1  # Single-threaded (if needed)
cargo run -p tenor-cli -- test conformance  # Run conformance suite
cargo test --workspace -- --nocapture       # Show println! output
```

## Test File Organization

**Location:**
- **Unit/integration tests:** Separate `tests/` directory per crate
  - `crates/core/tests/schema_validation.rs`
  - `crates/cli/tests/cli_integration.rs`
  - `crates/cli/tests/serve_integration.rs`
  - `crates/eval/tests/conformance.rs`
  - `crates/eval/tests/numeric_regression.rs`
  - `crates/analyze/tests/analysis_tests.rs`
  - `crates/codegen/tests/codegen_integration.rs`
  - `crates/lsp/tests/lsp_tests.rs`
- **Conformance fixtures:** Separate filesystem directory with `.tenor`, `.expected.json`, `.facts.json`, `.verdicts.json` files
  - `conformance/positive/` - Valid elaboration tests
  - `conformance/negative/` - Error case tests
  - `conformance/numeric/` - Numeric precision tests
  - `conformance/promotion/` - Type promotion tests
  - `conformance/shorthand/` - DSL shorthand expansion tests
  - `conformance/cross_file/` - Multi-file import tests
  - `conformance/parallel/` - Parallel entity conflict tests

**Naming:**
- Test files: `<crate>_<type>.rs` (e.g., `cli_integration.rs`, `schema_validation.rs`)
- Test functions: `test_<what_is_tested>()` or `<functionality>_<expected_outcome>()` (e.g., `elaborate_valid_file_exits_0()`, `validate_all_positive_conformance_outputs_against_schema()`)
- Conformance fixtures: `<name>.tenor` + `<name>.expected.json` (positive) or `<name>.expected-error.json` (negative)
- Evaluator fixtures: `<name>.tenor` + `<name>.facts.json` + `<name>.verdicts.json`

**Structure:**
```
crates/
├── core/
│   └── tests/
│       └── schema_validation.rs          # JSON schema validation
├── cli/
│   └── tests/
│       ├── cli_integration.rs            # All CLI subcommand tests
│       ├── serve_integration.rs          # Server integration tests
│       └── fixtures/                     # Test data files
├── eval/
│   └── tests/
│       ├── conformance.rs                # Evaluator fixture runner
│       └── numeric_regression.rs         # Numeric precision tests
└── analyze/
    └── tests/
        └── analysis_tests.rs             # Static analysis tests
```

## Test Structure

**Suite Organization:**
Tests are organized in logical groups with ASCII dividers:

```rust
// ──────────────────────────────────────────────
// 1. Help and version
// ──────────────────────────────────────────────

#[test]
fn help_exits_0_with_description() { ... }

#[test]
fn version_exits_0() { ... }

// ──────────────────────────────────────────────
// 2. Elaborate subcommand
// ──────────────────────────────────────────────

#[test]
fn elaborate_valid_file_exits_0() { ... }
```

**Setup Patterns:**
- Helper functions for test setup: `workspace_root()`, `tenor()`, `read_fixture()`
- Temporary directories via `tempfile::TempDir` for file I/O tests
- Workspace root located via `env!("CARGO_MANIFEST_DIR")` walking up directory tree
- CLI commands set `current_dir()` to workspace root for relative path resolution

**Example setup from `crates/cli/tests/cli_integration.rs`:**
```rust
fn workspace_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root")
        .to_path_buf()
}

fn tenor() -> Command {
    let mut cmd = cargo_bin_cmd!("tenor");
    cmd.current_dir(workspace_root());
    cmd
}
```

**Teardown Pattern:**
- No explicit teardown; `TempDir` and `Command` cleanup handled by RAII
- Test isolation via separate command invocations and separate temp directories

## Mocking

**Framework:** `assert_cmd` for binary mocking (no mock library for internal functions)

**Patterns:**
No internal mocking framework detected. Tests use:
- Direct function calls to elaboration pipeline
- File-based input via conformance fixtures
- `assert_cmd::Command` to spawn the `tenor` binary and assert output

Example from `crates/cli/tests/cli_integration.rs`:
```rust
#[test]
fn elaborate_valid_file_exits_0() {
    tenor()
        .args(["elaborate", "conformance/positive/fact_basic.tenor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"kind\": \"Bundle\""));
}
```

**What to Mock:**
- `SourceProvider` trait has two implementations: `FileSystemProvider` and `InMemoryProvider`
- Tests use `InMemoryProvider` for filesystem-independent testing (e.g., WASM)
- External `SourceProvider` implementations can be created for custom test scenarios

**What NOT to Mock:**
- Parser/Lexer: always run actual lexer/parser to catch syntactic errors
- Type system: always run actual type checking (core to contract correctness)
- Elaboration passes: test via full pipeline or individual pass entry points
- Error handling: test actual error construction and serialization

## Fixtures and Factories

**Test Data:**
Conformance fixtures are the primary test data approach:

```rust
// Positive test fixture
fact_basic.tenor               // DSL source
fact_basic.expected.json       // Expected interchange JSON output

// Negative test fixture
unresolved_fact_ref.tenor               // DSL source with error
unresolved_fact_ref.expected-error.json // Expected error JSON
```

**Factory patterns from evaluator tests** (`crates/eval/tests/numeric_regression.rs`):

```rust
fn comparison_bundle(
    fact_id: &str,
    fact_type: serde_json::Value,
    op: &str,
    right: serde_json::Value,
    comparison_type: Option<serde_json::Value>,
) -> serde_json::Value {
    // Constructs a minimal bundle for testing
    json!({
        "id": "numeric_test",
        "kind": "Bundle",
        // ...
    })
}

fn two_fact_comparison_bundle(
    fact1_id: &str,
    fact1_type: serde_json::Value,
    // ...
) -> serde_json::Value { ... }
```

**Location:**
- Conformance fixtures in `conformance/` directory tree at workspace root
- Integration test fixtures in `crates/<crate>/tests/fixtures/`
- Inline JSON builders in test functions when fixtures would be overkill

**Factory approach:**
- Builder pattern used inline (e.g., `json!` macro from serde_json)
- Parameterized constructors for test variants (e.g., `comparison_bundle()` with different operators)
- No trait-based factory pattern; direct function-based builders

## Coverage

**Requirements:** No enforced coverage targets detected in CI

**View Coverage:**
No coverage reporting configuration found. To generate coverage:
```bash
cargo tarpaulin --workspace --out Html  # If tarpaulin is installed
```

## Test Types

**Unit Tests:**
Confined to test functions within implementation crates. Examples:
- `crates/core/tests/schema_validation.rs` - JSON schema validation
- Individual elaboration pass tests (if any) embedded in pass module docs

**Integration Tests:**
Primary test approach; test the full pipeline or major subsystems:

- **CLI integration** (`crates/cli/tests/cli_integration.rs`): Full command execution
  - Tests help, version, elaborate, validate, eval, test, diff, check subcommands
  - Verifies exit codes (0 for success, 1 for failure, 2 for usage errors)
  - Asserts stdout/stderr content via predicates
  - Tests both happy path and error cases

- **Server integration** (`crates/cli/tests/serve_integration.rs`): HTTP server endpoint tests
  - Axum server started and tested with HTTP requests
  - Endpoint tests for elaboration, evaluation, health checks

- **Evaluator conformance** (`crates/eval/tests/conformance.rs`): Fixture-based pipeline testing
  - Elaborates `.tenor` files via `tenor-core`
  - Evaluates with `.facts.json` via `tenor-eval`
  - Compares verdicts against `.verdicts.json`
  - Tests error cases with `run_eval_fixture_error()`
  - Tests flow evaluation with `run_eval_flow_fixture()`

- **Numeric regression** (`crates/eval/tests/numeric_regression.rs`): Arithmetic correctness
  - Constructs inline JSON bundles
  - Tests Int, Decimal, Money arithmetic and comparisons
  - Tests cross-type comparisons (Int × Decimal)
  - Tests numeric edge cases
  - Complements elaborator's conformance/numeric/ fixture tests

- **Codegen integration** (`crates/codegen/tests/codegen_integration.rs`): Code generation tests
  - Loads conformance fixtures
  - Calls `generate_typescript()` with config
  - Asserts output directory structure
  - Verifies TypeScript file content

- **LSP tests** (`crates/lsp/tests/lsp_tests.rs`): Language Server Protocol tests

- **Analysis tests** (`crates/analyze/tests/analysis_tests.rs`): Static analysis tests

**E2E Tests:**
Conformance suite acts as E2E tests:
- Runner in `crates/cli/src/runner.rs` iterates conformance fixtures
- CLI subcommand `tenor test conformance` executes full suite
- Elaboration -> validation -> comparison pipeline tested end-to-end

## Common Patterns

**Async Testing:**
No async tests detected. All tests are synchronous Rust.

**Error Testing:**

Negative tests use fixture files + expected error JSON:

```rust
fn validate_all_positive_conformance_outputs_against_schema() {
    // ...
    for dir_name in &["positive", "numeric", "promotion", "shorthand"] {
        let dir = conformance_root.join(dir_name);
        for path in collect_expected_json_files(&dir) {
            validate_file(&validator, &path, &mut failures, &mut tested);
        }
    }
    assert!(tested > 0, "No conformance expected.json files found");
    assert!(failures.is_empty(),
        "Schema validation failed for {} of {} files:\n{}",
        failures.len(), tested, failures.join("\n"));
}
```

Evaluator error testing:

```rust
fn run_eval_fixture_error(fixture_dir: &Path, name: &str) {
    let bundle = tenor_core::elaborate::elaborate(&tenor_path)
        .unwrap_or_else(|e| panic!("Failed to elaborate {}: {:?}", name, e));
    let facts = load_facts(&facts_path);
    let result = tenor_eval::evaluate(&bundle, &facts);
    assert!(result.is_err(), "Expected evaluation error for {}, but got success", name);
}
```

CLI error testing uses `assert()` on command failure:

```rust
#[test]
fn elaborate_nonexistent_file_exits_1() {
    tenor()
        .args(["elaborate", "nonexistent_file_xyz.tenor"])
        .assert()
        .failure()
        .code(1);
}
```

**Comparison Testing:**

JSON equality assertions with pretty-printed diffs:

```rust
assert_eq!(
    actual,
    expected,
    "Verdict mismatch for {}\n\nActual:\n{}\n\nExpected:\n{}",
    name,
    serde_json::to_string_pretty(&actual).unwrap(),
    serde_json::to_string_pretty(&expected).unwrap(),
);
```

---

*Testing analysis: 2026-02-25*
