# Tenor SDK Cross-Conformance Suite

Verifies that all three Tenor SDKs (TypeScript, Python, Go) produce **identical JSON output** to the Rust evaluator (the source of truth) for the same inputs.

## Purpose

The Rust evaluator is the reference implementation. All SDKs wrap the same evaluation logic (either via WASM or native bindings), but each has its own serialisation layer. This conformance suite:

1. Generates expected output fixtures from the Rust evaluator directly
2. Runs each SDK against those same fixtures
3. Reports PASS/FAIL for each SDK

If any SDK diverges from the Rust evaluator's output format, this suite will catch it.

## Prerequisites

All three SDKs must be built before running conformance:

**TypeScript** — the wasm/ directory must exist (built from wasm-pack):

```bash
cd sdks/typescript
npm install
# wasm/ is pre-built — rebuild with: cd crates/tenor-eval-wasm && wasm-pack build --target nodejs
```

**Python** — requires maturin and a .venv:

```bash
cd sdks/python
pip install maturin
maturin develop
```

**Go** — the internal/wasm/tenor_eval.wasm must exist:

```bash
# WASM is pre-built — rebuild with:
cd sdks/go && bash scripts/build-wasm.sh
```

## Running

### All SDKs

```bash
cd sdks/conformance
./run-all.sh
```

Expected output on success:

```
========================================
  Tenor SDK Cross-Conformance Suite
========================================

--- TypeScript ---
=== TypeScript SDK conformance ===
PASS: evaluate (active)
...
TypeScript SDK: 5 passed, 0 failed

--- Python ---
...
Python SDK: 5 passed, 0 failed

--- Go ---
...
Go SDK: 5 passed, 0 failed

========================================
  Summary
========================================
  TypeScript: PASS
  Python: PASS
  Go: PASS

Total: 3 passed, 0 failed
========================================
ALL SDKs CONFORM
```

### Individual SDKs

```bash
./run-typescript.sh
./run-python.sh
./run-go.sh
```

## Re-generating Fixtures

Fixtures are generated from the Rust evaluator and committed to the repository. To regenerate them after evaluator changes:

```bash
./generate-fixtures.sh
```

This runs `sdks/conformance/fixture-gen/` (a small Rust binary that calls `tenor-eval` directly) and overwrites all `fixtures/expected-*.json` files.

After regenerating, re-run `./run-all.sh` to confirm all SDKs still conform.

## Fixture Format

All JSON fixture files use **sorted keys** for deterministic comparison.

### Input fixtures

| File | Purpose |
|------|---------|
| `escrow-bundle.json` | Interchange bundle (entity_operation_basic contract) |
| `escrow-facts.json` | Active facts (`is_active: true`) |
| `escrow-entity-states.json` | Entity states (`Order: pending`) |
| `escrow-facts-inactive.json` | Inactive facts (`is_active: false`) |

### Expected output fixtures

| File | API | Input |
|------|-----|-------|
| `expected-verdicts.json` | `evaluate` | active facts |
| `expected-verdicts-inactive.json` | `evaluate` | inactive facts |
| `expected-action-space.json` | `compute_action_space` | active, admin persona |
| `expected-action-space-blocked.json` | `compute_action_space` | inactive, admin persona |
| `expected-flow-result.json` | `execute_flow` / `simulate_flow` | active, admin, approval_flow |

## Test Cases

Each SDK runner executes 5 tests:

| # | Name | API | Facts | Expected |
|---|------|-----|-------|---------|
| 1 | evaluate (active) | evaluate | is_active=true | 1 verdict: account_active |
| 2 | evaluate (inactive) | evaluate | is_active=false | 0 verdicts |
| 3 | computeActionSpace | compute_action_space | is_active=true, Order=pending, admin | 1 action: approval_flow |
| 4 | computeActionSpace (blocked) | compute_action_space | is_active=false, Order=pending, admin | 0 actions, 1 blocked: PreconditionNotMet |
| 5 | executeFlow | execute_flow / simulate_flow | is_active=true, Order=pending, admin | outcome: order_approved |

## Adding New Test Cases

To add a new fixture set:

1. Add input fixture files to `fixtures/` (e.g., `my-facts.json`)
2. Update `fixture-gen/src/main.rs` to generate the expected output
3. Run `./generate-fixtures.sh` to produce the expected files
4. Update all three SDK runners (`runners/typescript-runner.ts`, `runners/python_runner.py`, `runners/go-runner/main.go`) to include the new test
5. Run `./run-all.sh` to confirm all SDKs pass
