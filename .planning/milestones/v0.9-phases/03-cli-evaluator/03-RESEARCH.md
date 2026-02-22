# Phase 3: CLI + Evaluator - Research

**Researched:** 2026-02-21
**Domain:** Rust CLI tooling, contract evaluation engine, fixed-point numeric arithmetic, structured JSON diffing
**Confidence:** HIGH

## Summary

Phase 3 transforms Tenor from a conformance-suite-driven elaborator into a user-facing command-line tool with evaluation capabilities. The work divides into three major streams: (1) CLI shell migration from hand-rolled argument parsing to clap derive-based subcommands, (2) a reference evaluator in `tenor-eval` that accepts interchange bundles + facts JSON and produces provenance-traced verdicts, and (3) a structural diff tool for interchange bundles.

The CLI work is straightforward -- the existing `main.rs` already implements `elaborate` and `run` commands via manual `args` matching. Migrating to clap with the derive API gives us typed argument parsing, help text, `--output`/`--quiet` flags, and meaningful exit codes essentially for free. The evaluator is the centerpiece of the phase: it must implement the spec's evaluation model (Section 14) faithfully, including frozen verdict semantics (Section 11.4), stratified rule evaluation (Section 7.4), predicate expression evaluation (Section 10.3), and the NumericModel (Section 12) with fixed-point arithmetic. The `rust_decimal` crate provides IEEE 754 roundTiesToEven (MidpointNearestEven) out of the box, which is exactly what the spec mandates.

**Primary recommendation:** Use clap 4.5 with derive macros for CLI, `rust_decimal` for all numeric evaluation, hand-build the evaluator as a tree-walker over deserialized interchange JSON, and implement `tenor diff` as construct-level structural comparison keyed by `(kind, id)`.

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| CLI-01 | Unified `tenor` binary with subcommands: elaborate, validate, check, eval, explain, test, generate, diff | clap 4.5 derive API with `#[derive(Parser)]` + `#[derive(Subcommand)]` enum. Register all subcommands now; unimplemented ones return "not yet implemented" with exit code 2. |
| CLI-02 | `tenor elaborate <file.tenor>` produces interchange JSON to stdout | Already implemented in current `main.rs`. Migrate to clap subcommand, preserve exact behavior. |
| CLI-03 | `tenor validate <bundle.json>` validates interchange against formal JSON Schema | Load `docs/interchange-schema.json`, deserialize bundle, validate with `jsonschema` 0.42 (already a dev-dep in tenor-core). Move to runtime dep. |
| CLI-05 | `tenor eval <bundle.json> --facts <facts.json>` evaluates contract against provided facts | Core evaluator work. `tenor-eval` crate implements `evaluate(bundle: &Value, facts: &Value) -> Result<VerdictSet>`. CLI wires stdin/stdout. |
| CLI-07 | `tenor test` runs conformance suite | Already implemented as `run` command. Rename to `test` subcommand, keep TAP v14 output. |
| CLI-09 | CLI supports `--output` format flags, `--quiet` for CI, and meaningful exit codes | clap global args: `--output {json,text}`, `--quiet`. Exit codes: 0=success, 1=error, 2=not-implemented. |
| EVAL-01 | Evaluator accepts interchange bundle + facts JSON and produces verdict set with provenance | Evaluator reads deserialized interchange `Value`, builds internal construct index, assembles FactSet from facts.json, runs stratified rule evaluation. Output is JSON verdict set. |
| EVAL-02 | Every verdict carries complete derivation chain (provenance-traced evaluation) | VerdictInstance includes rule_id, stratum, facts_used, verdicts_used. OperationProvenance includes state_before/after. FlowProvenance is ordered StepRecord list. |
| EVAL-03 | Evaluator correctly implements frozen verdict semantics | Snapshot taken at Flow initiation: FactSet + VerdictSet frozen. All steps within a Flow evaluate against snapshot, not recomputed state. Sub-flows inherit parent snapshot (E5). |
| EVAL-04 | Evaluator handles numeric types with fixed-point arithmetic matching spec NumericModel | `rust_decimal::Decimal` with `MidpointNearestEven` rounding. Promotion rules from Section 12.2 implemented as type-directed dispatch. Overflow aborts with typed error. |
| EVAL-05 | Evaluator conformance suite with dedicated test fixtures | New `conformance/eval/` directory with `.tenor` + `.facts.json` + `.verdicts.json` triplets. Separate from elaborator conformance. |
| EVAL-06 | Evaluator conformance suite includes frozen verdict semantics edge cases | Dedicated fixtures exercising: mid-flow entity state changes not affecting verdicts, sub-flow snapshot inheritance, parallel branch isolation. |
| EVAL-07 | Evaluator conformance suite includes numeric precision edge cases (50+ cases) | Shared numeric fixture set covering: promotion rules, overflow detection, rounding edge cases, Money arithmetic, cross-type comparisons. |
| TEST-07 | CLI integration tests for each subcommand (exit codes, output format, error handling) | `assert_cmd` crate for CLI integration testing. Test each subcommand's stdout, stderr, exit code. |
| TEST-09 | Numeric precision regression suite shared across elaborator and evaluator | Shared fixtures in `conformance/numeric/` consumed by both elaborator conformance and evaluator conformance runners. |
| MIGR-01 | `tenor diff <t1.json> <t2.json>` produces structured diff of two interchange bundles showing added, removed, and changed constructs by (kind, id) | Construct-level diff keyed by `(kind, id)`. Output categories: added, removed, changed. Changed constructs show field-level diffs. No external library needed -- hand-built over `serde_json::Value`. |
</phase_requirements>

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| [clap](https://crates.io/crates/clap) | 4.5 | CLI argument parsing with derive macros | De facto standard Rust CLI framework. Derive API gives typed subcommands, help text, shell completion, and validation. |
| [rust_decimal](https://crates.io/crates/rust_decimal) | 1.36+ | Fixed-point decimal arithmetic | 96-bit mantissa, `MidpointNearestEven` rounding matches spec's IEEE 754 roundTiesToEven requirement. Used by financial Rust projects. |
| [serde](https://crates.io/crates/serde) + [serde_json](https://crates.io/crates/serde_json) | 1.x | JSON serialization/deserialization | Already in workspace dependencies. Used for interchange format I/O. |
| [jsonschema](https://crates.io/crates/jsonschema) | 0.42 | JSON Schema validation | Already a dev-dependency in tenor-core. Needed at runtime for `tenor validate`. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| [assert_cmd](https://crates.io/crates/assert_cmd) | 2.x | CLI integration testing | TEST-07: testing subcommand exit codes, stdout/stderr content. |
| [predicates](https://crates.io/crates/predicates) | 3.x | Assertion helpers for assert_cmd | Pairs with assert_cmd for content matching in CLI tests. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| rust_decimal | fpdec / rust_fixed_point_decimal | rust_decimal is more mature, has serde support, has the exact rounding mode we need. Others are less widely used. |
| Hand-built JSON diff | serde_json_diff / sjdiff | External libs do generic JSON diff. We need construct-level diff keyed by (kind, id) -- domain-specific enough that a library would add complexity without value. |
| jsonschema 0.42 | jsonschema newer | Already validated in Phase 2 (TEST-08). No reason to change. |

### Installation

```toml
# crates/cli/Cargo.toml additions
[dependencies]
clap = { version = "4.5", features = ["derive"] }
tenor-eval = { path = "../eval" }
jsonschema = "0.42"

# crates/eval/Cargo.toml additions
[dependencies]
tenor-core = { path = "../core" }
serde = { workspace = true }
serde_json = { workspace = true }
rust_decimal = { version = "1.36", features = ["serde-with-str"] }

# dev-dependencies for integration tests
[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

## Architecture Patterns

### Recommended Project Structure

```
crates/
├── core/src/           # Unchanged -- elaboration pipeline
├── cli/src/
│   ├── main.rs          # clap Parser + Subcommand dispatch
│   ├── runner.rs        # Conformance suite runner (existing, enhanced)
│   ├── tap.rs           # TAP v14 output (existing)
│   ├── ambiguity/       # AI ambiguity testing (existing)
│   └── diff.rs          # Interchange bundle diff logic
├── eval/src/
│   ├── lib.rs           # Public API: evaluate(), EvalError, VerdictSet
│   ├── types.rs         # Runtime value types, FactSet, VerdictInstance
│   ├── assemble.rs      # FactSet assembly from facts.json
│   ├── rules.rs         # Stratified rule evaluation (eval_strata)
│   ├── predicate.rs     # PredicateExpression evaluator (eval_pred)
│   ├── operation.rs     # Operation execution (persona check, precondition, effects)
│   ├── flow.rs          # Flow execution with frozen snapshot
│   ├── numeric.rs       # NumericModel: promotion, arithmetic, overflow, rounding
│   └── provenance.rs    # Provenance chain construction
conformance/
├── eval/                # NEW: evaluator conformance fixtures
│   ├── positive/        # .tenor + .facts.json + .verdicts.json
│   ├── frozen/          # Frozen verdict edge cases
│   └── numeric/         # Shared numeric precision cases (50+)
```

### Pattern 1: clap Derive Subcommand Dispatch

**What:** Use clap's derive API to define the CLI as Rust types with subcommand enum.
**When to use:** All CLI entry point logic.
**Example:**

```rust
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "tenor", version, about = "Tenor contract language toolchain")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output format
    #[arg(long, global = true, default_value = "text")]
    output: OutputFormat,

    /// Suppress non-essential output
    #[arg(long, global = true)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Elaborate a .tenor file to interchange JSON
    Elaborate {
        /// Path to .tenor source file
        file: PathBuf,
    },
    /// Validate an interchange bundle against the JSON Schema
    Validate {
        /// Path to interchange bundle JSON
        bundle: PathBuf,
    },
    /// Evaluate a contract against provided facts
    Eval {
        /// Path to interchange bundle JSON
        bundle: PathBuf,
        /// Path to facts JSON file
        #[arg(long)]
        facts: PathBuf,
    },
    /// Run conformance test suite
    Test {
        /// Path to conformance suite directory
        #[arg(default_value = "conformance")]
        suite_dir: PathBuf,
    },
    /// Diff two interchange bundles
    Diff {
        /// First bundle
        t1: PathBuf,
        /// Second bundle
        t2: PathBuf,
    },
    // Future subcommands (Phase 4+):
    // Check, Explain, Generate -- registered now, return "not yet implemented"
}

#[derive(Clone, clap::ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}
```

### Pattern 2: Evaluator as Tree-Walker over Interchange JSON

**What:** The evaluator deserializes the interchange bundle JSON into internal types, then walks the construct tree to evaluate rules, operations, and flows.
**When to use:** All evaluation logic.
**Why:** The interchange format IS the contract -- the evaluator must consume it, not raw DSL. This is the spec's design: "TenorInterchange -- canonical JSON bundle (single source of truth for all tooling)".

```rust
// Evaluator public API
pub fn evaluate(
    bundle: &serde_json::Value,
    facts: &serde_json::Value,
) -> Result<EvalResult, EvalError> {
    let contract = Contract::from_interchange(bundle)?;
    let fact_set = assemble_facts(&contract, facts)?;
    let verdict_set = eval_strata(&contract, &fact_set)?;
    Ok(EvalResult {
        verdicts: verdict_set,
        provenance: /* built during evaluation */,
    })
}
```

### Pattern 3: Frozen Verdict Snapshot

**What:** At Flow initiation, capture the current FactSet and VerdictSet as an immutable snapshot. All steps within the Flow (including sub-flows) evaluate against this snapshot.
**When to use:** Flow execution.

```rust
struct Snapshot {
    facts: FactSet,        // Immutable after creation
    verdicts: VerdictSet,  // Immutable after creation
}

fn execute_flow(
    flow: &Flow,
    persona: &str,
    snapshot: &Snapshot,  // Borrowed immutably -- cannot be modified
    entity_states: &mut EntityStateMap,
) -> Result<FlowOutcome, EvalError> {
    let mut current = &flow.entry;
    loop {
        match flow.steps.get(current) {
            Some(Step::Operation { op, persona, outcomes, on_failure }) => {
                // Execute against FROZEN snapshot.verdicts, not recomputed
                let result = execute_op(op, persona, &snapshot.verdicts, entity_states)?;
                // Entity state changes are applied to entity_states
                // but verdicts are NOT recomputed
                current = &outcomes[&result.outcome_label];
            }
            Some(Step::Branch { condition, if_true, if_false, .. }) => {
                let val = eval_pred(condition, &snapshot.facts, &snapshot.verdicts);
                current = if val { if_true } else { if_false };
            }
            Some(Step::SubFlow { flow_ref, persona, .. }) => {
                // Sub-flow INHERITS parent snapshot (E5)
                let sub_result = execute_flow(flow_ref, persona, snapshot, entity_states)?;
                // ...
            }
            Some(Step::Terminal { outcome }) => return Ok(FlowOutcome { outcome: *outcome }),
            // ...
        }
    }
}
```

### Pattern 4: Construct-Level Bundle Diff

**What:** Diff two interchange bundles by comparing constructs keyed by `(kind, id)`.
**When to use:** `tenor diff` subcommand.

```rust
struct BundleDiff {
    added: Vec<ConstructSummary>,
    removed: Vec<ConstructSummary>,
    changed: Vec<ConstructChange>,
}

struct ConstructChange {
    kind: String,
    id: String,
    fields: Vec<FieldDiff>,
}

fn diff_bundles(t1: &Value, t2: &Value) -> BundleDiff {
    let index1 = index_by_kind_id(t1);
    let index2 = index_by_kind_id(t2);
    // added = in index2 but not index1
    // removed = in index1 but not index2
    // changed = in both but field-level diff is non-empty
}
```

### Anti-Patterns to Avoid

- **Re-parsing DSL in the evaluator:** The evaluator MUST consume interchange JSON, not `.tenor` source. The elaborator is the trust boundary.
- **Mutable snapshot during Flow execution:** The snapshot must be borrowed immutably. Entity state changes are tracked separately. Verdicts are never recomputed mid-flow.
- **Floating-point anywhere in numeric evaluation:** The spec explicitly prohibits this. All arithmetic must use `rust_decimal::Decimal`. No `f64` in the evaluation path.
- **Dynamic rule re-evaluation in flows:** Rules are evaluated ONCE at snapshot creation time. No re-evaluation after operations execute. This is the frozen verdict semantic commitment.
- **Generic JSON diff for `tenor diff`:** A generic JSON diff tool would show field-level noise (provenance line numbers, key ordering). The diff must be construct-aware and semantically meaningful.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI argument parsing | Manual `args` matching (current approach) | clap 4.5 derive | Current code is fragile, no help text, no validation, no completions |
| Fixed-point decimal arithmetic | Custom decimal type | rust_decimal | 96-bit precision, correct rounding modes, serde support, battle-tested in financial code |
| JSON Schema validation | Custom schema validator | jsonschema 0.42 | Already proven in Phase 2 tests. Correct draft-2020-12 support. |
| CLI integration testing | Custom process spawning | assert_cmd + predicates | Standard Rust pattern for testing CLI binaries |

**Key insight:** The evaluator itself MUST be hand-built -- there is no library for "evaluate a Tenor contract." But the infrastructure around it (CLI, numerics, schema validation, testing) should use standard crates. The spec's evaluation model is precise enough that the evaluator is essentially a direct transcription of spec pseudocode into Rust.

## Common Pitfalls

### Pitfall 1: Confusing Elaborator AST Types with Evaluator Types

**What goes wrong:** Trying to reuse `RawConstruct`, `RawExpr`, etc. from tenor-core directly in the evaluator, leading to type mismatches and confusion about what's resolved vs. unresolved.
**Why it happens:** The tenor-core AST types carry parse-time artifacts (line numbers, TypeRefs, raw strings) that don't belong in evaluation.
**How to avoid:** Define evaluator-specific types in `tenor-eval` that deserialize from interchange JSON. The evaluator never sees DSL-layer types. It consumes the canonical interchange format.
**Warning signs:** Import of `tenor_core::ast::*` in evaluator code beyond type reference.

### Pitfall 2: Incorrect Fact Assembly Semantics

**What goes wrong:** Not following the spec's `assemble_facts` exactly -- missing type checking, missing default fallback, missing List bounds checking.
**Why it happens:** Treating facts.json as pass-through data instead of validating against declared types.
**How to avoid:** Implement spec Section 5.2 literally. Every fact must be type-checked against its declared type. Missing facts without defaults must abort. List-typed facts must check element count against declared max.
**Warning signs:** Evaluator accepts facts.json without type validation.

### Pitfall 3: Entity State vs. Verdict State Confusion

**What goes wrong:** Entity state changes during Flow execution incorrectly triggering verdict re-evaluation.
**Why it happens:** Natural intuition says "after an Operation changes state, rules should re-fire." But the spec says the opposite.
**How to avoid:** Entity state map is mutable during flow execution (operations apply effects to it). Verdict set is immutable after snapshot creation. These are separate data structures with different mutability.
**Warning signs:** A `take_snapshot()` call appearing anywhere except at flow initiation.

### Pitfall 4: Numeric Promotion Rule Errors

**What goes wrong:** Incorrect type promotion when mixing Int and Decimal in comparisons, leading to wrong comparison results.
**Why it happens:** The promotion rules (Section 12.2) are precise but have several cases. Int promoted to Decimal requires computing `ceil(log10(max(|min|,|max|)))+1` for precision.
**How to avoid:** Implement promotion as a dedicated function with exhaustive test cases. The existing elaborator conformance fixtures in `conformance/promotion/` already test the type-level promotion; the evaluator must perform the same promotion at the value level.
**Warning signs:** Numeric comparisons passing without explicit promotion step.

### Pitfall 5: OperationStep Outcome Routing vs. Error Handling Confusion

**What goes wrong:** Conflating Operation error results (persona_rejected, precondition_failed) with outcome routing. Errors go to `on_failure`, outcomes go to the `outcomes` map.
**Why it happens:** Both are "results" of an Operation execution, but they have completely different routing paths.
**How to avoid:** Operation execution returns `Result<(EntityState, OutcomeLabel), OperationError>`. Ok variant routes through outcomes map. Err variant routes through on_failure handler. These are disjoint paths.
**Warning signs:** Error types appearing in outcome maps, or outcome labels appearing in failure handlers.

### Pitfall 6: facts.json Format Not Designed

**What goes wrong:** Starting evaluator implementation without a clear facts.json format, then retrofitting it after evaluation logic is built.
**Why it happens:** The spec defines FactSet assembly semantics but not the JSON serialization of external inputs.
**How to avoid:** Design the facts.json format upfront as part of plan 03-02. It should mirror the interchange format's type representation: `{ "fact_id": value }` where values follow the same structured type encoding as interchange defaults (e.g., Money as `{"amount": "100.00", "currency": "USD"}`).
**Warning signs:** Ad hoc fact value parsing in evaluation code.

## Code Examples

### Stratified Rule Evaluation (from spec Section 7.4)

```rust
use rust_decimal::Decimal;
use std::collections::{BTreeMap, HashSet};

fn eval_strata(
    contract: &Contract,
    facts: &FactSet,
) -> Result<VerdictSet, EvalError> {
    let mut verdicts = VerdictSet::new();
    let max_stratum = contract.rules.iter()
        .map(|r| r.stratum)
        .max()
        .unwrap_or(0);

    for n in 0..=max_stratum {
        let stratum_rules: Vec<&Rule> = contract.rules.iter()
            .filter(|r| r.stratum == n)
            .collect();
        for rule in stratum_rules {
            if let Some(verdict) = eval_rule(rule, facts, &verdicts)? {
                verdicts.insert(verdict);
            }
        }
    }
    Ok(verdicts)
}
```

### Fixed-Point Arithmetic with rust_decimal

```rust
use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy;

fn eval_decimal_comparison(
    left: &Decimal,
    right: &Decimal,
    op: &str,
    result_scale: u32,
) -> Result<bool, EvalError> {
    // Promotion: both values rounded to result_scale with MidpointNearestEven
    let left_promoted = left.round_dp_with_strategy(
        result_scale,
        RoundingStrategy::MidpointNearestEven,
    );
    let right_promoted = right.round_dp_with_strategy(
        result_scale,
        RoundingStrategy::MidpointNearestEven,
    );
    match op {
        "=" | "==" => Ok(left_promoted == right_promoted),
        "!=" | "≠" => Ok(left_promoted != right_promoted),
        "<" => Ok(left_promoted < right_promoted),
        "<=" | "≤" => Ok(left_promoted <= right_promoted),
        ">" => Ok(left_promoted > right_promoted),
        ">=" | "≥" => Ok(left_promoted >= right_promoted),
        _ => Err(EvalError::invalid_operator(op)),
    }
}

fn eval_mul(
    left: &Decimal,
    right: &Decimal,
    result_precision: u32,
    result_scale: u32,
) -> Result<Decimal, EvalError> {
    let product = left.checked_mul(*right)
        .ok_or_else(|| EvalError::overflow("multiplication overflow"))?;
    let rounded = product.round_dp_with_strategy(
        result_scale,
        RoundingStrategy::MidpointNearestEven,
    );
    // Check that result fits in declared precision
    let max_int_digits = result_precision - result_scale;
    let int_part = rounded.trunc().abs();
    let max_val = Decimal::from(10i64.pow(max_int_digits)) - Decimal::ONE;
    if int_part > max_val {
        return Err(EvalError::overflow("result exceeds declared precision"));
    }
    Ok(rounded)
}
```

### CLI Integration Test Pattern

```rust
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn elaborate_produces_json() {
    Command::cargo_bin("tenor")
        .unwrap()
        .args(&["elaborate", "conformance/positive/fact_basic.tenor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"kind\": \"Bundle\""));
}

#[test]
fn validate_rejects_invalid_bundle() {
    Command::cargo_bin("tenor")
        .unwrap()
        .args(&["validate", "tests/fixtures/invalid_bundle.json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("validation error"));
}

#[test]
fn eval_requires_facts_flag() {
    Command::cargo_bin("tenor")
        .unwrap()
        .args(&["eval", "bundle.json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--facts"));
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual `std::env::args()` parsing | clap 4.5 derive macros | clap 4.0 (2022), mature | Typed subcommands, auto-generated help, shell completions |
| f64 for decimal arithmetic | rust_decimal with MidpointNearestEven | N/A (always use fixed-point for financial) | Correct rounding, no floating-point surprises |
| Custom test harness for CLI | assert_cmd + predicates | Established pattern | Standard Rust CLI testing approach |

**Deprecated/outdated:**
- clap 2.x/3.x: Superseded by clap 4. The derive API in clap 4 is stable and recommended.
- `structopt`: Merged into clap 4 derive. Do not use structopt separately.

## Evaluator Design Deep-Dive

### Facts JSON Format Design

The spec defines `assemble_facts(contract, external_inputs) -> FactSet | Abort` (Section 5.2). The evaluator needs a `facts.json` format. Based on the interchange format's type encoding:

```json
{
  "escrow_amount": { "amount": "5000.00", "currency": "USD" },
  "delivery_status": "confirmed",
  "line_items": [
    {
      "id": "LI-001",
      "description": "Widget A",
      "amount": { "amount": "2500.00", "currency": "USD" },
      "valid": true
    },
    {
      "id": "LI-002",
      "description": "Widget B",
      "amount": { "amount": "2500.00", "currency": "USD" },
      "valid": true
    }
  ],
  "buyer_requested_refund": false
}
```

Key decisions:
- Top-level keys are fact IDs (matching contract-declared fact IDs)
- Values follow the interchange type encoding: Money as `{amount, currency}`, Enum as string, Bool as boolean, etc.
- Missing facts with defaults: omit from facts.json; evaluator uses contract default
- Missing facts without defaults: evaluator aborts with `missing_fact` error

### Verdict Output Format

```json
{
  "verdicts": [
    {
      "type": "line_items_validated",
      "payload": { "kind": "bool_value", "value": true },
      "provenance": {
        "rule": "all_line_items_valid",
        "stratum": 0,
        "facts_used": ["line_items"],
        "verdicts_used": []
      }
    },
    {
      "type": "within_threshold",
      "payload": { "kind": "bool_value", "value": true },
      "provenance": {
        "rule": "amount_within_threshold",
        "stratum": 0,
        "facts_used": ["escrow_amount", "compliance_threshold"],
        "verdicts_used": []
      }
    }
  ]
}
```

### Evaluator Internal Type System

The evaluator needs runtime value types distinct from the elaborator's AST types:

```rust
enum Value {
    Bool(bool),
    Int(i64),
    Decimal(rust_decimal::Decimal),
    Text(String),
    Date(String),       // ISO 8601 date string
    DateTime(String),   // ISO 8601 datetime string
    Money { amount: rust_decimal::Decimal, currency: String },
    Duration { value: i64, unit: String },
    Enum(String),
    Record(BTreeMap<String, Value>),
    List(Vec<Value>),
    TaggedUnion { tag: String, payload: Box<Value> },
}
```

### Construct Diff Algorithm

For `tenor diff`, the algorithm is:

1. Parse both bundles as `serde_json::Value`
2. Extract `constructs` arrays from both
3. Index each by `(kind, id)` -- both fields are always present in interchange
4. Compute set differences: added, removed, common
5. For common constructs, do field-level deep comparison (ignoring provenance by default -- provenance changes are noise)
6. Output structured diff as JSON or human-readable text

```json
{
  "added": [
    { "kind": "Fact", "id": "new_fact" }
  ],
  "removed": [
    { "kind": "Rule", "id": "old_rule" }
  ],
  "changed": [
    {
      "kind": "Entity",
      "id": "Order",
      "fields": {
        "states": {
          "before": ["draft", "submitted"],
          "after": ["draft", "submitted", "approved"]
        }
      }
    }
  ]
}
```

## Open Questions

1. **Facts JSON format formalization**
   - What we know: The spec defines FactSet assembly semantics but not the serialization format for external inputs
   - What's unclear: Should the facts.json format be documented in the spec or only in CLI docs? Should it mirror interchange type encoding exactly?
   - Recommendation: Define the format during plan 03-02. Use interchange type encoding for consistency. Document in CLI --help and a future authoring guide, not in the spec (which is frozen).

2. **Evaluator scope: Rules only vs. full Flow execution**
   - What we know: The spec defines both the read path (rules -> verdicts) and write path (operations -> state changes) and orchestration (flows)
   - What's unclear: Should Phase 3 implement full flow execution, or just rule evaluation?
   - Recommendation: Implement full evaluation including flows. The frozen verdict semantics are a key requirement (EVAL-03), and they only matter during Flow execution. Without flows, EVAL-03 is untestable.

3. **Parallel step evaluation**
   - What we know: Parallel branches must execute under the parent Flow's frozen snapshot with branch isolation (E8)
   - What's unclear: Does the reference evaluator need to actually run branches in parallel, or is sequential execution with snapshot isolation sufficient?
   - Recommendation: Sequential execution with snapshot isolation is sufficient and simpler. Branch execution order is "implementation-defined" per spec. The isolation guarantee is what matters, not actual parallelism.

4. **Operation outcome determination for multi-outcome Operations**
   - What we know: For multi-outcome Operations, the outcome is determined by the effect-to-outcome mapping and current entity state (Section 9.2)
   - What's unclear: When evaluating against a facts.json (no live entity state), how is initial entity state provided?
   - Recommendation: facts.json should include an optional `entity_states` section mapping entity IDs to their current state. If omitted, entities start in their declared initial state. This must be designed in plan 03-02.

5. **VerdictType constructs in interchange**
   - What we know: The spec mentions VerdictType with name, payload_schema, precedence_class, but the current interchange schema and elaborator only embed verdict info within Rule constructs
   - What's unclear: Whether VerdictTypes should be standalone constructs in the interchange or remain embedded in Rules
   - Recommendation: Follow the current elaborator behavior -- VerdictTypes are embedded in Rule `produce` clauses. The evaluator extracts verdict type information from Rule constructs during loading.

## Sources

### Primary (HIGH confidence)
- `docs/TENOR.md` -- Sections 5 (Fact), 7 (Rule), 9 (Operation), 10 (PredicateExpression), 11 (Flow), 12 (NumericModel), 14 (Complete Evaluation Model), 16 (Executor Obligations). All evaluation semantics directly from frozen v1.0 spec.
- `docs/interchange-schema.json` -- JSON Schema defining interchange format structure.
- `crates/cli/src/main.rs` -- Current CLI implementation (hand-rolled args parsing).
- `crates/core/src/` -- Elaborator pass modules, AST types, error types.
- `crates/eval/src/lib.rs` -- Stub crate (empty, ready for implementation).

### Secondary (MEDIUM confidence)
- [clap crates.io](https://crates.io/crates/clap) -- Version 4.5.53+ confirmed current. Derive API stable.
- [rust_decimal docs](https://docs.rs/rust_decimal/latest/rust_decimal/) -- `RoundingStrategy::MidpointNearestEven` confirmed as banker's rounding (IEEE 754 roundTiesToEven).
- [assert_cmd crates.io](https://crates.io/crates/assert-json-diff) -- Standard CLI integration testing crate.

### Tertiary (LOW confidence)
- JSON diff library survey (sjdiff, serde_json_diff, jsondiffpatch) -- evaluated but recommendation is to hand-build construct-level diff for domain-specific needs.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- clap 4.5 and rust_decimal are well-established, verified via web search and docs
- Architecture: HIGH -- evaluator architecture follows directly from spec pseudocode (Sections 7.4, 10.3, 11.4, 14)
- Pitfalls: HIGH -- identified from direct analysis of spec semantics and codebase structure
- Evaluator semantics: HIGH -- derived from frozen v1.0 spec with detailed pseudocode
- facts.json format: MEDIUM -- must be designed, no spec guidance on serialization format

**Research date:** 2026-02-21
**Valid until:** 2026-03-21 (stable domain -- spec is frozen, libraries are mature)
