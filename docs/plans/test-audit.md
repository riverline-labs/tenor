# Test Integrity Audit

Scan both repos for testing anti-patterns. Do NOT fix anything. Report only. Output goes to `.planning/TEST-INTEGRITY-AUDIT.md`.

**The core question:** Do our tests actually test the implementation, or do they test themselves?

---

## Anti-patterns to scan for

### 1. Shadow implementations in test files

The most dangerous anti-pattern. A test file defines its own version of a function and tests against that instead of the real implementation.

**Scan:** In every test file (`*_test.rs`, `tests/*.rs`, `#[cfg(test)]` modules):

- Find all `fn` definitions that are NOT test functions (not `#[test]`)
- For each non-test function in a test file:
  - Does a function with the same name exist in the implementation?
  - If yes: **RED FLAG** — the test may be testing its own copy, not the real one
  - If no: is it a test helper (builder, fixture factory, assertion helper)? Helpers are fine. Reimplementations are not.

**Report format:**

```
File: crates/eval/src/migration/tests.rs
  helper: build_test_contract() — OK (fixture factory)
  helper: assert_diff_contains() — OK (assertion helper)
  SHADOW: classify_change() — SAME NAME as migration::classify::classify_change — INVESTIGATE
```

### 2. Tests that only test happy paths

**Scan:** For each module with tests:

- Count tests that assert success/Ok/Some
- Count tests that assert failure/Err/None/specific errors
- If the ratio is heavily skewed toward happy paths (>80% success-only), flag it

**Report:** Module-by-module ratio of positive vs negative tests.

### 3. Tests with no assertions

**Scan:** Find test functions that:

- Have no `assert!`, `assert_eq!`, `assert_ne!`, `assert_matches!`, `should_panic`, or `unwrap()` on a Result
- Only call functions without checking their return values
- These tests prove the code doesn't panic, but nothing else

**Report:** List every assertion-free test.

### 4. Tests that test the test framework

**Scan:** Find tests that:

- Construct data, transform it, and assert properties of the transformation — without ever calling implementation code
- Example: build a HashMap, insert values, assert the HashMap contains them. This tests HashMap, not your code.

### 5. Tautological tests

**Scan:** Find tests where:

- The expected value is computed by the same code path as the actual value
- Example: `assert_eq!(classify(x), classify(x))` — always passes, tests nothing
- Example: `let expected = my_function(input); assert_eq!(my_function(input), expected);` — same thing

### 6. Tests with hardcoded wrong expectations

**Scan:** Find tests where the expected value looks suspicious:

- Expected error messages that don't match any error string in the implementation
- Expected enum variants that don't exist in the implementation's enum
- Expected struct fields that don't match the implementation's struct

This catches tests that were written against a planned API that changed, and the test was updated to match the test expectation rather than the actual implementation.

### 7. Mock overuse hiding integration gaps

**Scan:** In the private repo:

- How many tests use mocks vs real implementations?
- Are there critical paths (flow execution, entity state transitions, provenance recording) that are ONLY tested with mocks and never with the real Postgres implementation?
- List every critical path and whether it has a real integration test (not just a mock test)

### 8. Test isolation failures

**Scan:** Find tests that:

- Depend on execution order (test B only passes if test A runs first)
- Share mutable state between tests (static variables, shared files)
- Use the same database state without cleanup

### 9. Conformance test coverage gaps

**Scan:** The conformance suite tests the elaborator. Check:

- Are all 14 construct types covered by at least one positive conformance test?
- Are all constraint violations (C-SRC-01 through C-SRC-06, plus older constraints) covered by negative conformance tests?
- Are there interchange output shapes that are never tested (construct kinds that appear in the schema but have no fixture)?

### 10. Dead tests

**Scan:** Find tests that:

- Are `#[ignore]`d with no explanation
- Are commented out
- Are behind `#[cfg]` flags that are never enabled
- Have names suggesting they're obsolete (`test_old_behavior`, `test_v03_compat`)

---

## Scope

### Public repo (~/src/riverline/tenor)

Scan all of:

- `crates/core/` — elaborator tests
- `crates/eval/` — evaluator tests, migration tests, adapter tests
- `crates/cli/` — CLI tests
- `crates/interchange/` — interchange format tests
- `crates/analyze/` — static analysis tests
- `crates/storage/` — storage conformance tests
- `crates/tenor-eval-wasm/` — WASM evaluator tests
- `conformance/` — conformance test fixtures

### Private repo (~/src/riverline/tenor-platform)

Scan all of:

- `crates/executor/` — executor tests
- `crates/storage-postgres/` — Postgres storage tests
- `crates/platform-serve/` — HTTP endpoint tests
- `crates/platform/` — CLI tests
- `crates/agent-runtime/` — agent runtime tests

---

## Output format

```markdown
# Test Integrity Audit

**Date:** [date]
**Public repo tests scanned:** [N]
**Private repo tests scanned:** [N]

## Critical Issues (tests that may not test what they claim)

### Shadow implementations

[list every non-helper function defined in test files with same name as impl function]

### Tautological tests

[list every test where expected == actual by construction]

### Assertion-free tests

[list every test with no assertions]

## Coverage Gaps

### Happy path bias

| Module | Success tests | Failure tests | Ratio |
| ------ | ------------- | ------------- | ----- |
| ...    | ...           | ...           | ...   |

### Mock-only critical paths (private repo)

[list critical paths with no integration test]

### Conformance gaps

[list construct types or constraints without test coverage]

## Minor Issues

### Dead tests

[list ignored, commented-out, or obsolete tests]

### Test isolation concerns

[list any shared state or order dependencies]

## Summary

- Critical issues: [N]
- Coverage gaps: [N]
- Minor issues: [N]
- Overall assessment: [one sentence]
```

Do NOT fix anything. Report only. The human decides what to act on.
