# Codebase Concerns

**Analysis Date:** 2026-02-21

## Tech Debt

**Stub crates for future phases:**
- Issue: Three crates exist as placeholder stubs with no implementation
- Files: `crates/analyze/src/lib.rs`, `crates/codegen/src/lib.rs`, `crates/lsp/src/lib.rs`
- Impact: The `check`, `explain`, and `generate` CLI subcommands exit with code 2 and print "not yet implemented". Any caller depending on these commands will fail at runtime.
- Fix approach: Phase 4 (analyze), Phase 6 (codegen), Phase 8 (LSP) per stub docs

**Parallel step execution deferred in evaluator:**
- Issue: `FlowStep::ParallelStep` variant is fully parsed and validated by the elaborator but immediately returns a `TypeError` at runtime
- Files: `crates/eval/src/flow.rs:335-339`
- Impact: Any contract that uses parallel flow steps will elaborate successfully (pass all conformance tests) but fail at evaluation time. Silent discoverability gap.
- Fix approach: Implement parallel branch execution in `flow.rs::execute_flow`; add conformance fixtures under `conformance/eval/positive/`

**Compensation and escalation failure handlers not implemented:**
- Issue: `FailureHandler::Compensate` and `FailureHandler::Escalate` arms in `execute_flow` terminate the flow with a `TypeError` rather than executing compensation logic
- Files: `crates/eval/src/flow.rs:189-207`
- Impact: Contracts declaring `on_failure: compensate` or `on_failure: escalate` steps will fail at eval time with an opaque type error rather than the intended behavior
- Fix approach: Implement compensation rollback and escalation routing in `flow.rs`

**`_persona` parameter is accepted but unused in `evaluate_flow`:**
- Issue: The public API `evaluate_flow(bundle, facts, flow_id, persona)` accepts a `persona` argument but silently ignores it (variable is prefixed `_persona`). Each flow step carries its own `persona` field but branch steps and handoff steps also have their persona fields discarded (`persona: _`)
- Files: `crates/eval/src/lib.rs:75`, `crates/eval/src/flow.rs:215, 256, 322-323`
- Impact: Callers believe they are authorizing a flow initiator persona, but no authorization check is performed at the flow level; persona enforcement only occurs inside individual operation steps
- Fix approach: Either document that flow-level persona is intentionally unused or implement flow-level persona validation per spec Section 11

**Multi-outcome operation routing falls back to first outcome:**
- Issue: In `execute_operation`, when multiple outcomes are declared but no effect carries an `outcome` tag, the code silently returns `op.outcomes[0]` as a default rather than returning an error or requiring explicit routing
- Files: `crates/eval/src/operation.rs:210-213`
- Impact: Incorrectly authored contracts silently produce the wrong outcome instead of failing with a validation error
- Fix approach: Return an `OperationError` when no effect-to-outcome mapping exists and multiple outcomes are declared; or validate this at elaboration time in pass 5

**TypeDecl resolution uses `unwrap()` on pre-validated keys:**
- Issue: `pass3_types.rs` calls `.unwrap()` on `decls.get()` in three places inside cycle detection and resolution after a prior check has confirmed the key exists. The logic is correct, but any future refactoring that separates the check from the unwrap could introduce a panic.
- Files: `crates/core/src/pass3_types.rs:50-52, 79, 113`
- Impact: Low immediate risk but fragile under refactoring
- Fix approach: Replace with `.expect("invariant: key exists after cycle check passed")` or restructure to use `if let`

**`products.iter().min().unwrap()` in type-check pass:**
- Issue: In `type_check_produce`, a fixed-size 4-element array is created then `.min()` and `.max()` are unwrapped. The array is always non-empty so this cannot panic, but the intent is not obvious.
- Files: `crates/core/src/pass4_typecheck.rs:175-177`
- Impact: None currently; maintenance clarity issue
- Fix approach: Document the invariant or use `[a, b, c, d].iter().copied().reduce(i64::min).unwrap_or_default()`

**`10i64.pow(max_int_digits)` can panic on large precision:**
- Issue: `check_precision` computes `Decimal::from(10i64.pow(max_int_digits))` where `max_int_digits = precision - scale`. For `precision > 18`, `i64::pow` will overflow and panic in debug mode (wraps in release)
- Files: `crates/eval/src/numeric.rs:83`
- Impact: A contract with `Decimal(precision: 20, scale: 0)` would panic at runtime during numeric validation
- Fix approach: Use `Decimal::from_str("1").unwrap() * Decimal::TEN.powi(max_int_digits as i64)` or `Decimal::from(10u64).powi(max_int_digits as i64)` which stays within `Decimal` arithmetic

**Ambiguity test API retry logic uses string-match heuristics:**
- Issue: `is_retryable` in the Anthropic API client detects retryable errors by substring-matching the error message string for "429", "500", "503", etc. This is fragile and could false-positive on any error message that contains these digit sequences.
- Files: `crates/cli/src/ambiguity/api.rs:162-171`
- Impact: Low frequency in practice; could retry non-retryable errors unnecessarily
- Fix approach: Parse HTTP status codes from structured `ureq::Error` variants instead of string matching

**`AmbiguityTestCase` and `AmbiguityRunResult` structs have `#[allow(dead_code)]`:**
- Issue: Two structs in the ambiguity module require dead_code suppression
- Files: `crates/cli/src/ambiguity/mod.rs:17, 41`
- Impact: Fields are defined but not consumed; indicates API was designed speculatively ahead of full usage
- Fix approach: Either use all fields or remove them until needed

**`pass5_validate.rs` has two `#[allow(clippy::too_many_arguments)]` suppressions:**
- Issue: `validate_entity` and `validate_operation` have enough parameters to trigger clippy's too-many-arguments lint (suppressed)
- Files: `crates/core/src/pass5_validate.rs:145, 368`
- Impact: Functions are hard to call and easy to mis-order arguments; potential maintainability issue as features are added
- Fix approach: Group related parameters into structs (e.g., `EntityDecl`, `OperationDecl`) matching the AST types

## Known Bugs

**Date/DateTime comparison uses lexicographic string ordering:**
- Symptoms: Date values like `"2025-12-01"` compare correctly against `"2025-01-15"` because ISO 8601 dates sort lexicographically. However, this breaks for non-ISO formats and provides no format validation.
- Files: `crates/eval/src/numeric.rs:137-138`
- Trigger: Any `Date` or `DateTime` fact with a value that is not zero-padded ISO 8601 format (e.g., `"1/1/2025"`) will sort incorrectly
- Workaround: All conformance fixtures use ISO 8601 strings; no validation enforces this

**Money currency fallback silently uses empty string:**
- Symptoms: When parsing a `Money` value from facts JSON, if neither the value's `currency` field nor the type spec's `currency` field is present, the currency defaults to `""` (empty string)
- Files: `crates/eval/src/types.rs:485`
- Trigger: Malformed facts JSON with a `Money` fact missing the `currency` key
- Workaround: Contracts with proper type declarations always carry the currency in the TypeSpec

**Duration unit fallback is `"seconds"` when unit is absent:**
- Symptoms: `Duration` values with no `unit` field in the JSON or type spec default silently to `"seconds"`
- Files: `crates/eval/src/types.rs:516`
- Trigger: Malformed `Duration` facts missing the `unit` field
- Workaround: No meaningful workaround; silently incorrect behavior

**LLM response parser assumes JSON is not embedded in surrounding text:**
- Symptoms: If the LLM returns JSON embedded between non-JSON text (outside of code fences), `parse_llm_response` will fail to parse it
- Files: `crates/cli/src/ambiguity/compare.rs:49-57`
- Trigger: LLM response like `"Here is the result: {...}"` without code fences
- Workaround: The prompt instructs the LLM to return only JSON, but compliance is not guaranteed

## Security Considerations

**API key in environment variable (acceptable for toolchain):**
- Risk: `ANTHROPIC_API_KEY` is read from the environment; exposure in CI logs or process lists
- Files: `crates/cli/src/ambiguity/api.rs:54-59`
- Current mitigation: Standard environment variable approach; key not logged
- Recommendations: Ensure CI systems mask the variable in build logs; the key is only required for the optional `ambiguity` subcommand

**No input size limits on `.tenor` source files:**
- Risk: A malicious or malformed `.tenor` file could cause excessive memory allocation during lexing/parsing
- Files: `crates/core/src/pass1_bundle.rs:137` (reads entire file into String)
- Current mitigation: None; `std::fs::read_to_string` reads the full file
- Recommendations: For production use, add a file-size limit check before reading (e.g., 10MB cap)

**No input size limits on facts JSON:**
- Risk: Excessively large facts JSON objects could cause excessive memory use during evaluation
- Files: `crates/eval/src/lib.rs:41-51`
- Current mitigation: None
- Recommendations: Add size checks or streaming JSON parsing for production deployment

**Interchange JSON schema not validated before evaluation:**
- Risk: `evaluate()` and `evaluate_flow()` accept arbitrary `serde_json::Value` bundles and attempt to parse them. Malformed bundles produce `DeserializeError` but no schema validation occurs before evaluation begins.
- Files: `crates/eval/src/lib.rs:45`, `crates/eval/src/types.rs:309-356`
- Current mitigation: The `validate` CLI subcommand exists separately but is not called by `evaluate`
- Recommendations: Optionally validate against the embedded JSON schema before evaluation, or document the caller responsibility clearly

## Performance Bottlenecks

**`Contract::from_interchange` walks the full constructs array on every evaluation:**
- Problem: Every call to `evaluate()` or `evaluate_flow()` fully deserializes the interchange bundle from `serde_json::Value` into `Contract` structs. For repeated evaluations against the same contract, this is wasteful.
- Files: `crates/eval/src/lib.rs:45`, `crates/eval/src/types.rs:304-356`
- Cause: No caching or pre-parsed contract representation exists
- Improvement path: Add a `Contract::from_interchange` cache or expose `Contract` as a pre-parsed type in the public API so callers can deserialize once and evaluate many times

**`eval_strata` iterates all rules for each stratum level:**
- Problem: For each stratum from 0 to `max_stratum`, the full rules list is filtered. For contracts with many strata and many rules, this is O(strata × rules).
- Files: `crates/eval/src/rules.rs:31-41`
- Cause: Rules are not pre-indexed by stratum at deserialization time
- Improvement path: Pre-group rules by stratum in `Contract::from_interchange` into a `BTreeMap<u32, Vec<Rule>>`

**Operation lookup in flow execution is linear search:**
- Problem: `execute_flow` finds an operation by calling `contract.operations.iter().find(|o| o.id == *op)` on every operation step execution
- Files: `crates/eval/src/flow.rs:127-133`
- Cause: No operation index; operations are stored as a `Vec`
- Improvement path: Index operations by ID in `Contract` as `BTreeMap<String, Operation>`

**Forall quantifier clones the entire `EvalContext` for each list element:**
- Problem: `eval_pred` for `Predicate::Forall` clones the full `EvalContext` (including all bindings map) for each element in the list
- Files: `crates/eval/src/predicate.rs:165`
- Cause: Context ownership model requires cloning rather than extending in-place
- Improvement path: Pass context by mutable reference and undo the binding after each iteration

## Fragile Areas

**Conformance fixture JSON exact-match comparison:**
- Files: `crates/cli/src/runner.rs:152-158`
- Why fragile: The test runner compares elaborated output against `*.expected.json` using a custom `json_equal` function that normalizes numbers but performs byte-for-byte key order comparison otherwise. Any addition of a new optional field to the interchange format that is omitted for some constructs will break all existing positive fixtures until they are regenerated.
- Safe modification: Run `cargo run -p tenor-cli -- elaborate <file>` to regenerate expected files after serialization changes; check `pass6_serialize.rs` for sort order changes
- Test coverage: 55 conformance tests; all currently passing

**Import resolution uses filesystem `canonicalize()` for cycle detection:**
- Files: `crates/core/src/pass1_bundle.rs:103, 180`
- Why fragile: Cycle detection relies on `PathBuf::canonicalize()` which resolves symlinks. If `.tenor` files are accessed via symlinks pointing to the same physical file, they may appear as different paths and bypass cycle detection.
- Safe modification: Test cross-file import scenarios with actual files (not symlinks)
- Test coverage: `conformance/cross_file/` covers the basic import case; no symlink tests exist

**`flow.rs` step execution uses a hardcoded limit of 1000 steps:**
- Files: `crates/eval/src/flow.rs:94`
- Why fragile: The magic constant `1000` is not derived from any spec requirement; it prevents infinite loops in DAG execution. A legitimate complex flow with many steps could hit this limit. The error returned uses `EvalError::TypeError`, which is semantically incorrect (this is not a type error).
- Safe modification: Make the limit configurable or increase it; change the error variant to a dedicated `FlowLimitExceeded` variant
- Test coverage: No test exercises flows near the limit

**`pass5_validate.rs` parallel branch conflict detection uses `b2_trace.as_ref().unwrap()`:**
- Files: `crates/core/src/pass5_validate.rs:861`
- Why fragile: Inside the parallel conflict detection loop, `b2_trace.as_ref().unwrap()` is called after a check that at least one of `b1_trace` or `b2_trace` is `Some`. If `b1_trace` is `Some` and `b2_trace` is `None`, the code takes the `b2_id` branch and then unwraps `b2_trace`. The logic is correct (the branch only runs when `b2_trace` is `Some`), but the invariant is implicit and would panic if the control flow were refactored incorrectly.
- Safe modification: Restructure to `if let Some(t) = b2_trace { ... }` pattern
- Test coverage: `conformance/parallel/` covers conflict scenarios

**`check_precision` panics on `precision - scale > 18` due to `i64::pow`:**
- Files: `crates/eval/src/numeric.rs:83`
- Why fragile: `10i64.pow(max_int_digits)` overflows when `max_int_digits > 18`. In release mode this wraps silently to a negative number, causing a false positive (values incorrectly accepted as within bounds). In debug mode it panics.
- Safe modification: Use `Decimal` arithmetic for the bound calculation; see fix approach in Tech Debt section
- Test coverage: `conformance/numeric/` does not test precision values > 18

## Scaling Limits

**Conformance suite is file-system glob-based:**
- Current capacity: ~55 tests across ~8 subdirectories
- Limit: No known hard limit, but discovery is `std::fs::read_dir` with no parallelism; adding hundreds of fixtures will increase test time linearly
- Scaling path: Add parallel test execution using `rayon` or split suite into independent test binaries

**Ambiguity suite makes serial blocking HTTP calls:**
- Current capacity: Each test case makes one synchronous `ureq` HTTP call (blocking); the suite runs sequentially
- Limit: With 3-retry exponential backoff peaking at 4 seconds per case, a suite of 10 cases can take ~40+ seconds under rate limiting
- Scaling path: Use async HTTP or a thread pool for parallel API calls; add a `--concurrency` flag

## Dependencies at Risk

**`jsonschema = "0.42"` is used in two places with a direct version pin:**
- Risk: Pinned in both `crates/cli/Cargo.toml` and `crates/core/Cargo.toml` as dev-dependencies; not declared in `[workspace.dependencies]`. Versions may drift if updated independently.
- Impact: Subtle schema validation differences between the CLI and the schema validation test
- Migration plan: Promote `jsonschema` to a workspace dependency

**`ureq = "3"` is a blocking HTTP client in the CLI binary:**
- Risk: `ureq` v3 uses a blocking synchronous model. Adding any async feature (streaming, parallel API calls) would require migrating to `reqwest` or `hyper`.
- Impact: Currently acceptable for the optional `ambiguity` command
- Migration plan: If parallel ambiguity tests are needed, migrate to `reqwest` with `tokio`

## Missing Critical Features

**No runtime validation of Date/DateTime format:**
- Problem: Date values are stored and compared as raw strings with no ISO 8601 format validation at fact assembly time
- Blocks: Reliable date comparisons in contracts; spec compliance for temporal types
- Files affected: `crates/eval/src/assemble.rs`, `crates/eval/src/types.rs:495-505`

**No flow-level persona authorization:**
- Problem: The `evaluate_flow` API accepts a `persona` parameter but never enforces it; the spec intends the initiating persona to be checked against the flow's authorized personas
- Blocks: Multi-persona contract security enforcement at the flow level
- Files affected: `crates/eval/src/lib.rs:71-106`

**`check`, `explain`, and `generate` CLI subcommands are unimplemented stubs:**
- Problem: Three CLI commands (static analysis, natural language explanation, code generation) are registered in the CLI but immediately exit with code 2
- Blocks: Any tooling or CI that depends on these commands
- Files affected: `crates/cli/src/main.rs:125-133`

## Test Coverage Gaps

**No eval conformance tests for `Duration` or `TaggedUnion` fact types:**
- What's not tested: `Duration` and `TaggedUnion` values in facts, comparison, and rule evaluation
- Files: `crates/eval/src/types.rs:140, 144`; no corresponding fixtures in `conformance/eval/`
- Risk: Type parsing is implemented but the evaluation path for these types is untested; comparison of `Duration` values would fall through to the `_ => TypeError` arm in `numeric.rs`
- Priority: Medium

**No eval conformance tests for `ParallelStep` error path:**
- What's not tested: That contracts with parallel steps fail at evaluation with a meaningful error
- Files: `crates/eval/src/flow.rs:335-339`; no fixtures in `conformance/eval/`
- Risk: The error message ("parallel step execution not yet implemented") is not tested; a change to the error could go undetected
- Priority: Low

**No eval conformance tests for `Compensate`/`Escalate` failure handlers:**
- What's not tested: That failure handlers beyond `Terminate` produce the expected error behavior
- Files: `crates/eval/src/flow.rs:189-207`
- Risk: If/when these are implemented, there are no fixtures to verify correctness
- Priority: Medium

**No test for `check_precision` with `precision > 18`:**
- What's not tested: The panic/overflow behavior of `10i64.pow(max_int_digits)` for large precision values
- Files: `crates/eval/src/numeric.rs:83`
- Risk: The bug exists in production code and is undetected by any test
- Priority: High

**No symlink cycle test for import resolution:**
- What's not tested: Import cycle detection via symlinked files
- Files: `crates/core/src/pass1_bundle.rs:103`
- Risk: Symlink-based cycles bypass detection; low frequency in practice
- Priority: Low

**No test for money with missing currency in facts JSON:**
- What's not tested: The `unwrap_or("")` fallback in money currency resolution
- Files: `crates/eval/src/types.rs:485`
- Risk: Silent empty-string currency values could produce incorrect comparisons
- Priority: Low

---

*Concerns audit: 2026-02-21*
