# Codebase Concerns

**Analysis Date:** 2026-02-22

## Tech Debt

**Stub CLI command — `generate` is unimplemented:**
- Issue: The `generate` subcommand is dispatched via `stub_not_implemented("generate", ...)`, exits with code 2, and produces no output.
- Files: `crates/cli/src/main.rs:177-178`, `crates/codegen/src/lib.rs` (placeholder doc comment only)
- Impact: Codegen (Phase 6) is not functional. Any caller of `tenor generate` receives a runtime error.
- Fix approach: Implement `tenor-codegen` crate and wire up `cmd_generate` in `main.rs`.

**Placeholder crates with no implementation:**
- Issue: `crates/codegen/src/lib.rs` and `crates/lsp/src/lib.rs` contain only a doc comment pointing to a future phase. No structs, functions, or public API exist.
- Files: `crates/codegen/src/lib.rs`, `crates/lsp/src/lib.rs`
- Impact: Phases 6 (code generation) and 8 (LSP) cannot be used.
- Fix approach: Implement per published phase plans.

**Duplicated manifest/etag logic in two places:**
- Issue: `build_manifest` and `compute_etag` are defined in `crates/cli/src/main.rs:191-208` and then reimplemented inline in `crates/cli/src/runner.rs:257-265`. Any change to manifest format must be made in both places.
- Files: `crates/cli/src/main.rs:191`, `crates/cli/src/runner.rs:257`
- Impact: Conformance tests can diverge from CLI output if the two implementations drift.
- Fix approach: Extract `build_manifest` and `compute_etag` into a shared module (e.g., `crates/cli/src/manifest.rs`) and use it from both `main.rs` and `runner.rs`.

**Hardcoded version strings scattered across codebase:**
- Issue: `"1.0"` and `"1.1.0"` are hard-coded as string literals in 9+ locations in `pass6_serialize.rs`, and the manifest `"tenor"` field is separately hard-coded to `"1.1"` in `main.rs` and `runner.rs`. When the protocol version advances, every site must be found and updated manually.
- Files: `crates/core/src/pass6_serialize.rs:78,80,128,150,188,228,249,257,1014`, `crates/cli/src/main.rs:206`, `crates/cli/src/runner.rs:265`
- Impact: Version bumps are error-prone; tests checking `"tenor_version": "1.0.0"` in `crates/analyze/src/bundle.rs:567` and `crates/analyze/src/lib.rs:188` may break on a protocol update.
- Fix approach: Define a single `TENOR_BUNDLE_VERSION` and `TENOR_MANIFEST_VERSION` constant in `crates/core` and reference them from all serialization and test fixture helpers.

**Stale `"tenor_version": "1.0.0"` in analyze-crate test helpers:**
- Issue: `make_bundle()` in `crates/analyze/src/bundle.rs:567` and `crates/analyze/src/lib.rs:188` creates test bundles with `"tenor_version": "1.0.0"`, while the elaborator now emits `"1.1.0"`. If schema validation is ever enabled in those tests, they will fail.
- Files: `crates/analyze/src/bundle.rs:561-568`, `crates/analyze/src/lib.rs:185-194`
- Impact: Silent inconsistency; likely to cause failures when schema validation is tightened.
- Fix approach: Update test helpers to use the current version constant.

**`spec_sections` field loaded but never consumed:**
- Issue: `AmbiguityTestCase.spec_sections` in `crates/cli/src/ambiguity/mod.rs:27-28` is annotated `#[allow(dead_code)] // Loaded for future use` and is never read after population. This is future-intent dead code.
- Files: `crates/cli/src/ambiguity/mod.rs:27`, `crates/cli/src/ambiguity/fixtures.rs`
- Impact: Spec-targeted prompting cannot function until this is wired through.
- Fix approach: Either remove the field until the feature is ready, or implement spec-section injection into prompts.

**`AmbiguityRunResult` public fields are all `#[allow(dead_code)]`:**
- Issue: Three of four fields on `AmbiguityRunResult` (`total`, `matches`, `mismatches`) are suppressed with dead-code allows. The struct is returned from `run_ambiguity_suite` but the caller in `cmd_ambiguity` only checks `hard_errors`.
- Files: `crates/cli/src/ambiguity/mod.rs:43-51`, `crates/cli/src/main.rs:758-778`
- Impact: Match/mismatch stats are computed but never surfaced in the CLI exit code or summary.
- Fix approach: Use the counts to set a non-zero exit code when mismatches exceed a threshold, or at minimum print a summary line.

---

## Known Bugs

**Date format validation accepts structurally valid but semantically invalid dates:**
- Symptoms: `"2024-99-99"` passes `validate_date_format` in the evaluator because the check only tests digit positions and separators, not calendar correctness (month 1-12, day 1-31).
- Files: `crates/eval/src/assemble.rs:71-81` (comment says "Does NOT validate actual date correctness")
- Trigger: Provide `"2024-13-45"` as a `Date` fact; it will be accepted without error.
- Workaround: None currently; validation would require adding a date-parsing dependency or inline calendar logic.

**DateTime validation only checks the `T` separator, not the time portion:**
- Symptoms: `validate_datetime_format` at `crates/eval/src/assemble.rs:87-91` accepts any string matching `YYYY-MM-DDT` followed by arbitrary characters, including `"2024-01-15Tgarbage"`.
- Files: `crates/eval/src/assemble.rs:84-92`
- Trigger: Provide any Date-valid prefix plus `T` plus any suffix.
- Workaround: None.

---

## Security Considerations

**Anthropic API key transmitted in HTTP request headers:**
- Risk: The API key from `ANTHROPIC_API_KEY` is sent as an `x-api-key` header in cleartext HTTP calls to `api.anthropic.com`. No TLS pinning is applied — ureq defaults to system roots.
- Files: `crates/cli/src/ambiguity/api.rs:104-108`
- Current mitigation: ureq uses HTTPS by default; system TLS is in use.
- Recommendations: This is acceptable for a developer CLI tool. Document that the key must be scoped to ambiguity testing and never used in production pipelines.

**Import path traversal is not restricted to a sandbox directory:**
- Risk: A `.tenor` file can contain `import "../../../etc/passwd"` (or any absolute or `..`-relative path). The elaborator will attempt to read and parse any file the invoking user can access.
- Files: `crates/core/src/pass1_bundle.rs:167` (`base_dir.join(import_path)` with no prefix check)
- Current mitigation: Import paths are limited by OS filesystem permissions of the invoking process. The tool is a CLI intended to be run by the contract author, so this is low risk in the current use case.
- Recommendations: If tenor-core is ever embedded in a server or multi-tenant environment, add a `jail_to_dir` check that rejects any resolved path outside the root contract's directory.

**Hardcoded model name in ambiguity testing:**
- Risk: `DEFAULT_MODEL = "claude-sonnet-4-5-20250514"` in `crates/cli/src/ambiguity/api.rs:12` is a snapshot model version. If Anthropic deprecates this model, the ambiguity suite silently breaks for anyone not passing `--model`.
- Files: `crates/cli/src/ambiguity/api.rs:12`
- Current mitigation: `--model` flag allows override.
- Recommendations: Use a non-dated alias like `claude-sonnet-4-5` as default, or document the specific model requirement in the CLI help text.

---

## Performance Bottlenecks

**O(k * n) stratum rule evaluation:**
- Problem: `eval_strata` in `crates/eval/src/rules.rs:29-41` iterates all rules once to find `max_stratum`, then performs a full linear scan of all rules per stratum. For k strata and n rules, this is O(k * n) total rule traversals.
- Files: `crates/eval/src/rules.rs:29-41`
- Cause: `contract.rules` is a `Vec<Rule>` with no stratum index. A `BTreeMap<u32, Vec<&Rule>>` precomputed once would eliminate the repeated scans.
- Improvement path: Build the stratum map once in `eval_strata` (same approach `pass6_serialize.rs` already uses).

**O(n) operation lookup per flow step:**
- Problem: Inside `execute_flow`'s hot loop, every `OperationStep` performs a linear scan of `contract.operations` via `.iter().find(|o| o.id == *op)`.
- Files: `crates/eval/src/flow.rs:258-264`
- Cause: `Contract.operations` is a plain `Vec<Operation>`. Contracts with many operations and long flows will do O(steps * operations) work.
- Improvement path: Pre-index `contract.operations` into a `HashMap<&str, &Operation>` before entering the step loop — the same pattern already used for `step_index` at `flow.rs:218-222`.

**O(n) operation lookup during compensation:**
- Problem: Compensation steps in `handle_failure` also call `contract.operations.iter().find(...)` at `crates/eval/src/flow.rs:111-113`, outside the main step loop but on a cold path without the step-level index.
- Files: `crates/eval/src/flow.rs:110-113`
- Improvement path: Share the same operation index built for the main step loop.

**Frequent deep clones in flow execution:**
- Problem: `handle_failure` clones the entire `steps_executed` and `entity_changes_all` vectors when producing intermediate `FlowResult` values (e.g., `flow.rs:103-104`). This is called on every `Terminate` failure handler.
- Files: `crates/eval/src/flow.rs:101-106`
- Cause: `FlowResult` owns its vectors, so intermediate returns must clone.
- Improvement path: Collect results in a shared accumulator and only build `FlowResult` at return sites, passing references or using `Arc`.

**Linear import cycle detection using `Vec::contains`:**
- Problem: `stack.contains(&canon)` in `pass1_bundle.rs` at lines 104 and 181 is O(n) where n is the current import stack depth. For deep import trees this is called on each import, making cycle detection O(depth^2).
- Files: `crates/core/src/pass1_bundle.rs:104`, `crates/core/src/pass1_bundle.rs:181`
- Cause: `stack` is a `Vec<PathBuf>` used as a set.
- Improvement path: Maintain a parallel `HashSet<PathBuf>` for O(1) membership tests alongside the existing `visited` set.

**Excessive string allocations in `pass6_serialize.rs`:**
- Problem: `pass6_serialize.rs` contains 184 `.to_owned()` or `.to_string()` calls in production (not test) code; the serializer reconstructs owned `String` values for every field at every construct on every elaboration invocation.
- Files: `crates/core/src/pass6_serialize.rs`
- Cause: `serde_json::Map` requires `String` keys; no caching or intern strategy is used.
- Improvement path: Low priority for CLI use. For library embedding, consider `Arc<str>` or Cow-based keys.

---

## Fragile Areas

**`pass5_validate.rs` — multiple `unwrap()` calls with SAFETY comments that depend on invariants maintained by caller:**
- Files: `crates/core/src/pass5_validate.rs:716`, `crates/core/src/pass5_validate.rs:828`, `crates/core/src/pass5_validate.rs:1326`
- Why fragile: Each `unwrap()` has a `// SAFETY:` comment stating the precondition (e.g., "all neighbors were inserted into `in_degree`"). If a future refactor changes the construction of `in_degree` or `path`, the assumption silently breaks and the elaborator panics on user input.
- Safe modification: Replace each with an `expect("...")` that includes the invariant text, or better, return a proper `ElabError` rather than panicking.
- Test coverage: Covered by negative conformance tests for cycles, but no unit tests directly exercise the unwrap sites.

**`pass3_types.rs` — `unwrap()` on `in_stack.iter().position()` assumes cycle detection is always reliable:**
- Files: `crates/core/src/pass3_types.rs:47`, `crates/core/src/pass3_types.rs:52`, `crates/core/src/pass3_types.rs:54`, `crates/core/src/pass3_types.rs:84`
- Why fragile: `decls.get(back_edge_name.as_str()).unwrap()` will panic if the back-edge name computed from `in_stack` is not present in `decls`. This invariant holds as long as the recursive DFS only adds items that are in `decls`, but would break if the entry logic changes.
- Safe modification: Replace with a proper error return.

**`pass4_typecheck.rs` — `unwrap()` on min/max of an always-4-element array:**
- Files: `crates/core/src/pass4_typecheck.rs:177-178`
- Why fragile: `products.iter().min().unwrap()` depends on `products` being non-empty. The array literal at line 175 has exactly 4 elements, making this safe today. Any refactor that extracts that code into a helper without preserving the invariant will panic.
- Safe modification: Use `products.iter().copied().min().expect("products is non-empty array")`.

**`explain.rs` — untyped JSON traversal with silent `unwrap_or` fallbacks:**
- Files: `crates/cli/src/explain.rs`
- Why fragile: 75 `.as_str()`, `.as_array()`, `.as_object()` etc. calls return `Option` and fall back to empty strings or empty vecs via `.unwrap_or(...)` or `.unwrap_or_default()`. If the interchange format changes a key name or type, the explain output silently drops entire sections without error.
- Safe modification: Deserialize the interchange bundle into a typed Rust struct before passing to `explain`, using the same typed deserialization already used by `tenor-eval`.
- Test coverage: CLI integration tests check for non-empty output; they will not catch dropped sections.

**Flow step limit of 1000 is a magic number with no configuration:**
- Files: `crates/eval/src/flow.rs:228`
- Why fragile: The limit is a local variable initialized at execution time (`let max_steps = 1000`). There is no way for callers to configure a different limit. Legitimate multi-step flows with many loop-backs would fail unexpectedly.
- Safe modification: Accept an optional `max_steps: Option<usize>` parameter in `execute_flow`, defaulting to 1000.

---

## Scaling Limits

**Path enumeration in S6 analysis caps at 10,000 paths and 1,000 depth:**
- Current capacity: Up to 10,000 distinct paths, 1,000 step depth per flow.
- Limit: At `MAX_PATHS = 10_000` the enumeration truncates; `FlowPathResult.truncated` is set true. Analysis results for complex flows are incomplete.
- Files: `crates/analyze/src/s6_flow_paths.rs:15-17`
- Scaling path: Make `MAX_PATHS` and `MAX_DEPTH` configurable via the analysis API or CLI flags.

**`Contract` uses `Vec` for all collections — no indexed access:**
- Current capacity: Works well for contracts with tens to low hundreds of constructs.
- Limit: For very large contracts (hundreds of rules, operations, flows), any pattern requiring lookup by ID (e.g., operation lookup per flow step) degrades to O(n).
- Files: `crates/eval/src/types.rs:300-307`
- Scaling path: Add pre-built `HashMap`s in `Contract` for operations, flows, and rules indexed by ID; populate them in `from_interchange`.

---

## Dependencies at Risk

**`ureq` v3 with non-standard error format parsing:**
- Risk: `crates/cli/src/ambiguity/api.rs:163-178` parses ureq error strings by scanning for 3-digit HTTP status codes in the error message text. This parsing is fragile against any ureq version that changes its error formatting.
- Impact: Retry logic silently stops working if ureq reformats errors — all API failures would appear as non-retryable.
- Files: `crates/cli/src/ambiguity/api.rs:159-195`
- Migration plan: Once ureq provides a structured error type with a `status()` method (or if switching to `reqwest`), replace the string scan with a typed check.

**`jsonschema` v0.42 compiled into CLI binary for runtime schema validation:**
- Risk: `jsonschema` is a large dependency used only for `tenor validate` and the schema validation test suite. Version pinned at `0.42` with no flexibility.
- Impact: If `jsonschema` releases a breaking API change, schema validation tests fail without changes to caller code.
- Files: `Cargo.toml` (workspace), `crates/cli/src/main.rs`, `crates/core/tests/schema_validation.rs`
- Migration plan: Low urgency. Monitor for API changes; the `validate` command is not performance-critical.

---

## Missing Critical Features

**No unit tests in `tenor-core` source files:**
- Problem: The `crates/core/src/` directory has zero `#[test]` functions. All testing of the elaborator pipeline is done via file-based conformance tests in `conformance/`. Individual pass functions have no isolated unit tests.
- Files: `crates/core/src/pass1_bundle.rs`, `crates/core/src/pass2_index.rs`, `crates/core/src/pass3_types.rs`, `crates/core/src/pass4_typecheck.rs`, `crates/core/src/pass5_validate.rs`, `crates/core/src/pass6_serialize.rs`
- Blocks: Debugging regressions in individual pass logic requires running the full conformance suite.

**Negative test coverage is thin for most passes:**
- Problem: Pass 0 has 2 negative tests, Pass 1 has 5, Pass 2 has 4, Pass 3 has 2, Pass 4 has 6. Pass 5 has 24 — the most complete. Edge cases in the lexer, import resolver, type-checker, and serializer are exercised only through the positive conformance suite.
- Files: `conformance/negative/pass0/`, `conformance/negative/pass1/`, `conformance/negative/pass2/`, `conformance/negative/pass3/`, `conformance/negative/pass4/`
- Risk: A change to pass logic that introduces a new error case or removes an existing one may go undetected.

**`tenor-eval` has no error-path conformance fixtures for flow execution:**
- Problem: The eval conformance suite (`conformance/eval/`) covers positive outcomes and numeric regressions. There are no fixtures that exercise `FlowError`, `OperationError::EntityNotFound`, or `FailureHandler::Escalate` paths from a file-based test.
- Files: `conformance/eval/positive/`, `conformance/eval/numeric/`
- Risk: Flow error-handling paths are tested only through in-source unit tests (`crates/eval/src/flow.rs`, `crates/eval/src/operation.rs`), which use `panic!` assertions that would hide error type changes.

---

## Test Coverage Gaps

**`crates/core/src/lexer.rs` — no dedicated lexer tests:**
- What's not tested: Unicode input, edge-case token sequences (e.g., consecutive operators, unterminated strings beyond the existing 2 pass-0 negative tests), all escape sequences.
- Files: `crates/core/src/lexer.rs`
- Risk: Lexer regressions surface only when a conformance test exercises the broken token path.
- Priority: Medium.

**`crates/cli/src/diff.rs` — diff logic tested internally but not via CLI integration:**
- What's not tested: The `tenor diff` subcommand is not covered by `crates/cli/tests/cli_integration.rs`. Unit tests in `diff.rs` cover the logic, but CLI argument parsing, output format, and `--breaking` flag are untested end-to-end.
- Files: `crates/cli/src/diff.rs`, `crates/cli/tests/cli_integration.rs`
- Risk: CLI surface changes (e.g., wrong exit code on breaking change) go undetected.
- Priority: Medium.

**`crates/cli/src/explain.rs` — no tests for Markdown format output:**
- What's not tested: `ExplainFormat::Markdown` produces different heading and bullet syntax than `ExplainFormat::Terminal`. There are no tests asserting Markdown-specific output structure.
- Files: `crates/cli/src/explain.rs`
- Risk: Silent formatting breakage in Markdown mode.
- Priority: Low.

**`crates/analyze/` — S3a admissibility analysis has no negative test cases:**
- What's not tested: Contracts where admissibility checks should fire (unreachable state predicates, missing fact references in preconditions).
- Files: `crates/analyze/src/s3a_admissibility.rs`, `crates/analyze/tests/analysis_tests.rs`
- Risk: False-negative findings in admissibility analysis are not caught.
- Priority: Medium.

---

*Concerns audit: 2026-02-22*
