# Testing Patterns

**Analysis Date:** 2026-02-21

## Test Framework

**Runner:**
- Custom TAP (Test Anything Protocol) v14 implementation in `elaborator/src/tap.rs`
- No external test framework (no pytest, cargo test, vitest); conformance suite is primary test mechanism
- Test execution: `cd elaborator && cargo run -- run ../conformance`
- Expected: 47/47 passing tests (as documented in CLAUDE.md)

**Assertion Library:**
- Custom JSON comparison: `json_equal()` and `json_diff()` in `elaborator/src/runner.rs`
- Serde JSON for parsing and serialization
- Manual error JSON construction for comparison

**Run Commands:**
```bash
# Run all conformance tests (from elaborator directory)
cargo run -- run ../conformance          # Uses default ../conformance path
cargo run -- run /path/to/conformance    # Specify path explicitly

# Elaborate single file
cargo run -- elaborate path/to/file.tenor

# Build only (from elaborator directory)
cargo build
```

## Test File Organization

**Location:**
- Co-located with source: DSL fixtures in `conformance/` directory at repo root
- Organized by test category and elaboration pass
- Not co-located with Rust source files

**Naming:**
- DSL source: `<test_name>.tenor`
- Expected output (positive): `<test_name>.expected.json`
- Expected error (negative): `<test_name>.expected-error.json`
- Multi-file tests: suffix pattern `_a.tenor` (root) and `_b.tenor` (leaf)

**Structure:**
```
conformance/
├── positive/              # Valid DSL → expected.json (no error expected)
├── negative/
│   ├── pass0/            # Lex/parse errors
│   ├── pass1/            # Import resolution errors
│   ├── pass2/            # Construct indexing errors
│   ├── pass3/            # Type environment errors
│   ├── pass4/            # Type-checking errors
│   └── pass5/            # Validation errors
├── cross_file/           # Multi-file import tests (rules.tenor imports facts.tenor)
├── parallel/             # Pass 5 parallel entity conflict tests
├── numeric/              # Decimal/money precision fixtures (positive tests)
├── promotion/            # Numeric type promotion fixtures (positive tests)
└── shorthand/            # DSL shorthand expansion fixtures (positive tests)
```

## Test Structure

**Suite Organization:**
```rust
// From runner.rs lines 17-47
pub fn run_suite(suite_dir: &Path) -> RunResult {
    let mut tap = Tap::new();

    // Positive tests
    run_positive_dir(suite_dir, "positive", &mut tap);

    // Negative tests by pass
    for pass in 0..=6 {
        run_negative_tests(suite_dir, pass, &mut tap);
    }

    // Cross-file tests
    run_cross_file_tests(suite_dir, &mut tap);

    // Parallel entity conflict tests
    run_parallel_tests(suite_dir, &mut tap);

    // Numeric precision tests
    run_positive_dir(suite_dir, "numeric", &mut tap);

    // Type promotion tests
    run_positive_dir(suite_dir, "promotion", &mut tap);

    // DSL shorthand expansion tests
    run_positive_dir(suite_dir, "shorthand", &mut tap);

    let failed = tap.failure_count();
    tap.finish();

    RunResult { failed }
}
```

**Patterns:**

*Positive tests (expected success):*
```rust
// From runner.rs lines 135-168
fn run_positive_test(
    tenor_path: &Path,
    expected_path: &Path,
    name: &str,
    category: &str,
    tap: &mut Tap,
) {
    let test_name = format!("{}/{}", category, name);

    let expected_json = match read_json(expected_path) {
        Ok(v) => v,
        Err(e) => {
            tap.not_ok(&test_name, format!("failed to read expected file: {}", e));
            return;
        }
    };

    match elaborate::elaborate(tenor_path) {
        Ok(got) => {
            if json_equal(&got, &expected_json) {
                tap.ok(&test_name);
            } else {
                let diff = json_diff(&expected_json, &got);
                tap.not_ok(&test_name, format!("output mismatch:\n{}", diff));
            }
        }
        Err(e) => {
            tap.not_ok(&test_name, format!(
                "unexpected elaboration error (pass {}): {}",
                e.pass, e.message
            ));
        }
    }
}
```

*Negative tests (expected error):*
```rust
// From runner.rs lines 170-204
fn run_negative_test(
    tenor_path: &Path,
    expected_error_path: &Path,
    name: &str,
    pass: u8,
    tap: &mut Tap,
) {
    let test_name = format!("negative/pass{}/{}", pass, name);

    let expected_error = match read_json(expected_error_path) {
        Ok(v) => v,
        Err(e) => {
            tap.not_ok(&test_name, format!("failed to read expected-error file: {}", e));
            return;
        }
    };

    match elaborate::elaborate(tenor_path) {
        Err(got_error) => {
            let got_json = got_error.to_json_value();
            if json_equal(&got_json, &expected_error) {
                tap.ok(&test_name);
            } else {
                let diff = json_diff(&expected_error, &got_json);
                tap.not_ok(&test_name, format!("error mismatch:\n{}", diff));
            }
        }
        Ok(_) => {
            tap.not_ok(&test_name, format!(
                "expected pass {} elaboration error but elaboration succeeded",
                pass
            ));
        }
    }
}
```

## Mocking

**Framework:** None — No mocking library used

**Patterns:**
- File I/O tested through actual file reads: `std::fs::read_to_string(path)`
- No dependency injection; elaboration takes `&Path` directly
- Test data (fixtures) are literal `.tenor` files in `conformance/`

**What to Mock:**
- Nothing. All external interactions (file system) are tested with real files in conformance suite.

**What NOT to Mock:**
- File I/O: test with actual `.tenor` source files
- JSON output: test with actual expected.json files
- Error generation: test with expected-error.json files

## Fixtures and Factories

**Test Data:**
- DSL fixtures stored as `.tenor` files in `conformance/` subdirectories
- Expected JSON output stored as `.expected.json` files (literal JSON, exact match required)
- Expected errors stored as `.expected-error.json` files with pass/construct/field/message fields

**Fixture example: `conformance/positive/fact_basic.tenor`**
```
// Positive test: basic Fact declarations covering all scalar BaseTypes
// Expected: elaborates without error, produces fact_basic.expected.json

fact is_active {
  type:   Bool
  source: "account_service.active"
}

fact item_count {
  type:   Int(min: 0, max: 1000)
  source: "inventory_service.item_count"
}
// ... more facts
```

**Expected output structure: `fact_basic.expected.json`**
```json
{
  "constructs": [
    {
      "id": "balance",
      "kind": "Fact",
      "provenance": { "file": "fact_basic.tenor", "line": 39 },
      "source": { "field": "balance", "system": "account_service" },
      "tenor": "0.3",
      "type": { "base": "Money", "currency": "USD" },
      "default": { "kind": "decimal_value", "precision": 10, "scale": 2, "value": "0.00" }
    }
    // ... more constructs (alphabetically sorted by id)
  ],
  "id": "fact_basic",
  "kind": "Bundle",
  "tenor": "0.3"
}
```

**Error fixture example: `conformance/negative/pass0/bad_token.tenor`**
```
// Negative test — Pass 0
// An unrecognized character '@' appears in the source. The lexer must reject it.
// Expected error: pass 0, unexpected character

fact foo @ {
  type:   Bool
  source: "x.y"
}
```

**Expected error: `bad_token.expected-error.json`**
```json
{
  "pass": 0,
  "construct_kind": null,
  "construct_id": null,
  "field": null,
  "file": "bad_token.tenor",
  "line": 5,
  "message": "unexpected character '@'"
}
```

**Location:**
- Fixtures: `conformance/` directory (organized by test type)
- Read by `run_suite()` which globs `*.tenor` files and matches with `.expected.json` or `.expected-error.json`

## Coverage

**Requirements:** None enforced (no coverage tool configured)

**View Coverage:** Not available — no coverage reporting in build setup

**Test scope:** 47 tests cover all 6 elaboration passes plus cross-file imports, numeric precision, type promotion, and shorthand expansion

## Test Types

**Unit Tests:**
- None. Codebase uses integration/conformance testing only.
- Each elaboration pass tested via conformance fixtures that exercise the pass boundaries

**Integration Tests:**
- All tests are integration tests: `.tenor` → elaboration → JSON comparison
- Multi-file tests in `conformance/cross_file/`: tests import resolution (Pass 1) end-to-end
- Parallel entity tests in `conformance/parallel/`: tests entity conflict detection (Pass 5)

**E2E Tests:**
- Not used. Conformance tests are de facto end-to-end (lex → parse → elaborate → serialize → JSON output).

## Common Patterns

**Async Testing:**
Not applicable — no async code in codebase

**Error Testing:**
```rust
// From runner.rs: construct expected error from JSON, elaborate file, compare to_json_value()
match elaborate::elaborate(tenor_path) {
    Err(got_error) => {
        let got_json = got_error.to_json_value();
        if json_equal(&got_json, &expected_error) {
            tap.ok(&test_name);
        } else {
            let diff = json_diff(&expected_error, &got_json);
            tap.not_ok(&test_name, format!("error mismatch:\n{}", diff));
        }
    }
    Ok(_) => {
        tap.not_ok(&test_name, format!(
            "expected pass {} elaboration error but elaboration succeeded",
            pass
        ));
    }
}
```

**Test output format:**
- TAP v14 (Test Anything Protocol) sent to stdout
- Format: `ok N - test/name` or `not ok N - test/name`
- Diagnostics prefixed with `  # ` on following lines
- Summary: `# tests N`, `# pass P`, `# fail F`

**Example TAP output:**
```
TAP version 14
1..47
ok 1 - positive/fact_basic
ok 2 - positive/entity_basic
ok 3 - positive/rule_basic
...
not ok 45 - negative/pass0/bad_token
  # expected pass 0 elaboration error but elaboration succeeded
# tests 47
# pass 46
# fail 1
```

## Test Execution Flow

1. **Run conformance suite:** `cargo run -- run ../conformance`
2. **Runner discovers tests:**
   - Glob `*.tenor` files in each subdirectory
   - Match with `.expected.json` (positive) or `.expected-error.json` (negative)
3. **For each test:**
   - Call `elaborate::elaborate(tenor_path)`
   - For positive: compare `Ok(bundle)` JSON with expected output
   - For negative: compare `Err(error).to_json_value()` with expected error JSON
4. **Record result:** `tap.ok(test_name)` or `tap.not_ok(test_name, diagnostics)`
5. **Output:** Print TAP v14 format with test count and pass/fail summary
6. **Exit code:** 0 if all pass, 1 if any fail (via `process::exit(1)`)

---

*Testing analysis: 2026-02-21*
