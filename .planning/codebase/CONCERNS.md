# Codebase Concerns

**Analysis Date:** 2026-02-23

## Tech Debt

**Triplicated interchange JSON deserialization across crates:**
- Issue: Three crates independently deserialize the same interchange JSON into different Rust struct hierarchies: `tenor-eval` (`crates/eval/src/types.rs` -- `Contract`, `TypeSpec`, `Predicate`, etc.), `tenor-analyze` (`crates/analyze/src/bundle.rs` -- `AnalysisEntity`, `AnalysisFact`, etc.), and `tenor-codegen` (`crates/codegen/src/bundle.rs` -- `CodegenFact`, `CodegenEntity`, `TypeInfo`, etc.). Each has its own `from_interchange()` parser with its own error handling. Any interchange format change must be synchronized across all three.
- Files: `crates/eval/src/types.rs:314-361`, `crates/analyze/src/bundle.rs:1-774`, `crates/codegen/src/bundle.rs:126-365`
- Impact: Tripled maintenance burden on interchange schema changes. Drift between crates means one crate may silently handle a field that another ignores. The `explain.rs` module is even worse -- it uses raw `serde_json::Value` traversal with no typed deserialization at all.
- Fix approach: Extract a shared `tenor-interchange` crate (or add shared types to `tenor-core`) that provides typed deserialization from interchange JSON. All downstream crates consume these shared types.

**Duplicated manifest/etag logic:**
- Issue: `build_manifest` and `compute_etag` exist in `crates/cli/src/manifest.rs` and are also inlined in `crates/cli/src/runner.rs`. The runner reimplements etag computation for conformance tests.
- Files: `crates/cli/src/manifest.rs`, `crates/cli/src/runner.rs`
- Impact: Manifest format changes must be made in two places or conformance tests diverge from CLI output.
- Fix approach: Have `runner.rs` import from `manifest.rs` directly.

**Hardcoded version strings scattered across codebase:**
- Issue: `"1.0"` and `"1.1.0"` are hard-coded as string literals in `pass6_serialize.rs`, and the manifest `"tenor"` field is separately hard-coded in `main.rs` and `runner.rs`. When the protocol version advances, every site must be found and updated manually.
- Files: `crates/core/src/pass6_serialize.rs` (multiple locations), `crates/cli/src/main.rs`, `crates/cli/src/runner.rs`
- Impact: Version bumps are error-prone.
- Fix approach: A `TENOR_BUNDLE_VERSION` constant exists in `crates/core/src/lib.rs`. Verify all serialization sites reference it.

**`explain.rs` uses untyped JSON traversal with 75+ silent fallbacks:**
- Issue: `crates/cli/src/explain.rs` (1,478 lines) traverses interchange JSON using raw `.get()`, `.as_str()`, `.as_array()` etc. with `.unwrap_or("")` or `.unwrap_or_default()` fallbacks throughout. If the interchange format changes a key name, sections silently disappear from explain output with no error.
- Files: `crates/cli/src/explain.rs`
- Impact: Any interchange format evolution silently breaks the explain feature.
- Fix approach: Use the same typed deserialization as `tenor-eval` or a shared interchange crate.

**`spec_sections` field loaded but never consumed in ambiguity testing:**
- Issue: `AmbiguityTestCase.spec_sections` in `crates/cli/src/ambiguity/mod.rs:27-28` is annotated `#[allow(dead_code)]` and never read after population.
- Files: `crates/cli/src/ambiguity/mod.rs:27`, `crates/cli/src/ambiguity/fixtures.rs`
- Impact: Spec-targeted prompting cannot function until wired through.
- Fix approach: Either remove the field or implement spec-section injection into prompts.

**Multiple `#[allow(dead_code)]` annotations in LSP crate:**
- Issue: `crates/lsp/src/semantic_tokens.rs` has 12 `#[allow(dead_code)]` annotations on semantic token type constants. `crates/lsp/src/navigation.rs` has 3 `#[allow(dead_code)]` on struct fields. This suggests features that were defined but not fully integrated.
- Files: `crates/lsp/src/semantic_tokens.rs:15-37`, `crates/lsp/src/navigation.rs:714-767`
- Impact: Dead code adds cognitive overhead and may mask actual integration gaps.
- Fix approach: Wire unused token types into the semantic token provider or remove them.

---

## Known Bugs

**Date format validation accepts semantically invalid dates:**
- Symptoms: `"2024-99-99"` passes `validate_date_format` in the evaluator because the check only tests digit positions and separators, not calendar correctness.
- Files: `crates/eval/src/assemble.rs` (comment says "Does NOT validate actual date correctness")
- Trigger: Provide `"2024-13-45"` as a `Date` fact; it is accepted without error.
- Workaround: The `time` crate is already a dependency of `tenor-eval` and can parse dates properly.

**DateTime validation only checks the `T` separator:**
- Symptoms: `validate_datetime_format` accepts any string matching `YYYY-MM-DDT` followed by arbitrary characters, including `"2024-01-15Tgarbage"`.
- Files: `crates/eval/src/assemble.rs`
- Trigger: Provide any Date-valid prefix plus `T` plus any suffix.
- Workaround: Use `time` crate's ISO 8601 parsing to validate the full datetime.

---

## Security Considerations

**`unsafe` block in serve.rs for signal handling:**
- Risk: `crates/cli/src/serve.rs:96-106` uses an `unsafe` block to install C signal handlers via `libc::signal()`. The signal handler writes to an `AtomicBool` which is safe, but the `unsafe` block itself is the only one in the codebase and uses raw function pointer casts.
- Files: `crates/cli/src/serve.rs:96-110`
- Current mitigation: The signal handler only performs an atomic store, which is async-signal-safe.
- Recommendations: Consider using the `ctrlc` crate or `signal-hook` for safe signal handling, especially before the hosted service milestone.

**HTTP server has no authentication, rate limiting, or CORS:**
- Risk: `tenor serve` exposes `/elaborate`, `/evaluate`, and `/explain` endpoints on `0.0.0.0` with no authentication, no rate limiting, and no CORS headers. Any network-accessible caller can elaborate arbitrary `.tenor` source, evaluate arbitrary contracts, and consume CPU.
- Files: `crates/cli/src/serve.rs:33-90`
- Current mitigation: Designed as a local development tool. Docker mounts contracts read-only.
- Recommendations: For the Hosted Evaluator Service milestone, this server architecture needs complete rework: add API key authentication, request rate limiting, CORS configuration, and input size validation beyond the 10MB body limit.

**Single-threaded request handling in serve.rs:**
- Risk: The HTTP server handles one request at a time in a loop (`handle_request(request, &state)` on line 86). A slow elaboration or evaluation blocks all other requests. A malicious client sending a large contract with deeply nested constructs could cause a denial of service.
- Files: `crates/cli/src/serve.rs:74-87`
- Current mitigation: Only designed for local use.
- Recommendations: For hosted service, switch to an async runtime (tokio + axum/actix-web) or at minimum spawn requests on threads.

**Elaborate endpoint writes user-supplied content to temp files:**
- Risk: The `/elaborate` POST handler writes user-supplied `source` content to a temp file on disk (`crates/cli/src/serve.rs:340-357`), then calls the elaborator on it. The elaborator's import resolution could follow `import` directives in the user-supplied content to read other files on the server.
- Files: `crates/cli/src/serve.rs:324-378`
- Current mitigation: The import sandbox in `pass1_bundle.rs:202` restricts imports to the contract root directory, which is the temp directory in this case.
- Recommendations: Validate that the sandbox restriction actually prevents escape from the temp directory. Add explicit input sanitization.

**Import path sandbox relies on canonicalization:**
- Risk: Import path traversal protection in `crates/core/src/pass1_bundle.rs:202` uses `canon_import.starts_with(sandbox_root)` after canonicalization. This is sound on Unix but symlink resolution behavior can vary across platforms.
- Files: `crates/core/src/pass1_bundle.rs:185-215`
- Current mitigation: Canonicalization resolves symlinks before the check, which is correct behavior.
- Recommendations: Add conformance tests specifically exercising import escape attempts (the `import_escape` fixture appears to exist).

---

## Performance Bottlenecks

**O(k * n) stratum rule evaluation:**
- Problem: `eval_strata` in `crates/eval/src/rules.rs` iterates all rules once to find `max_stratum`, then performs a full linear scan per stratum.
- Files: `crates/eval/src/rules.rs`
- Cause: `contract.rules` is a `Vec<Rule>` with no stratum index.
- Improvement path: Build a `BTreeMap<u32, Vec<&Rule>>` once at the start of evaluation.

**Frequent deep clones in flow execution:**
- Problem: `handle_failure` in `crates/eval/src/flow.rs:101-106` clones entire `steps_executed` and `entity_changes_all` vectors when producing intermediate `FlowResult` values.
- Files: `crates/eval/src/flow.rs:101-106`
- Cause: `FlowResult` owns its vectors, so intermediate returns must clone.
- Improvement path: Collect results in a shared accumulator and only build `FlowResult` at return sites.

**Linear import cycle detection using `Vec::contains`:**
- Problem: `stack.contains(&canon)` in `pass1_bundle.rs:120,217` is O(n) where n is import stack depth.
- Files: `crates/core/src/pass1_bundle.rs:120`, `crates/core/src/pass1_bundle.rs:217`
- Improvement path: Maintain a parallel `HashSet<PathBuf>` for O(1) membership tests.

**Excessive string allocations in pass6_serialize.rs:**
- Problem: The serializer reconstructs owned `String` values for every field at every construct on every elaboration invocation. The 1,044-line file is allocation-heavy.
- Files: `crates/core/src/pass6_serialize.rs`
- Improvement path: Low priority for CLI use. For WASM embedding, consider `Cow<'_, str>` or string interning.

---

## Fragile Areas

**`pass5_validate.rs` -- `expect()` calls that depend on caller-maintained invariants:**
- Files: `crates/core/src/pass5_validate.rs:842`, `crates/core/src/pass5_validate.rs:1343`
- Why fragile: Each `.expect()` has a safety comment stating the precondition. If a refactor changes the data structure construction, the elaborator panics on user input instead of returning an error.
- Safe modification: Replace with proper `ElabError` returns.
- Test coverage: Covered by negative conformance tests for cycles, but no unit tests directly exercise the expect sites.

**`pass3_types.rs` -- `expect()` on `in_stack.iter().position()` in cycle detection:**
- Files: `crates/core/src/pass3_types.rs:49`, `crates/core/src/pass3_types.rs:55`, `crates/core/src/pass3_types.rs:98`
- Why fragile: These expect calls depend on the recursive DFS only adding items that are in `decls`. The invariant is currently upheld but not enforced by types.
- Safe modification: Replace with error returns.

**`pass4_typecheck.rs` -- `expect()` on min/max of a hardcoded 4-element array:**
- Files: `crates/core/src/pass4_typecheck.rs:179`, `crates/core/src/pass4_typecheck.rs:183`
- Why fragile: `products.iter().min().expect(...)` depends on `products` being non-empty. Safe today because the array literal has exactly 4 elements, but would break if the logic is refactored.
- Safe modification: Use `.copied().min().expect("products is non-empty")` or return an error.

**Flow step limit of 1000 is a hardcoded constant:**
- Files: `crates/eval/src/flow.rs:234`
- Why fragile: `let max_steps = max_steps.unwrap_or(1000)` -- the function accepts `Option<usize>` but the default is a magic number. Legitimate multi-step flows could hit this limit unexpectedly.
- Safe modification: Make the default configurable or document it clearly.

**Parser has no error recovery -- first error aborts:**
- Files: `crates/core/src/parser.rs:1-1598`
- Why fragile: The parser is a single-pass recursive descent parser that returns `Result<_, ElabError>` on the first error. Users get one error message per invocation, making iterative fixing painful. The LSP diagnostics module has to work around this.
- Safe modification: Add error recovery at construct boundaries (skip to next `}` or next keyword after an error). This is important for LSP quality.

---

## Scaling Limits

**Path enumeration in S6 analysis caps at 10,000 paths:**
- Current capacity: Up to 10,000 distinct paths, 1,000 step depth per flow.
- Limit: At `MAX_PATHS = 10_000` the enumeration truncates and `FlowPathResult.truncated` is set.
- Files: `crates/analyze/src/s6_flow_paths.rs:15-17`
- Scaling path: Make limits configurable via analysis API or CLI flags.

**`Contract` uses `Vec` for all collections -- no indexed access:**
- Current capacity: Works well for contracts with tens to low hundreds of constructs.
- Limit: Lookup by ID degrades to O(n) for operations, flows, and rules.
- Files: `crates/eval/src/types.rs:300-307`
- Scaling path: Add `HashMap` indexes in `Contract`, populated in `from_interchange`.

---

## Dependencies at Risk

**`ureq` v3 error format parsing is fragile:**
- Risk: `crates/cli/src/ambiguity/api.rs` parses ureq error strings by scanning for 3-digit HTTP status codes in the error message text. This parsing breaks if ureq changes its error formatting.
- Impact: Retry logic silently stops working.
- Files: `crates/cli/src/ambiguity/api.rs`
- Migration plan: Switch to structured error type when available.

**`libc` dependency limits WASM portability:**
- Risk: `libc` is used only in `crates/cli/src/serve.rs` for signal handling. It is a CLI-only dependency. However, `tenor-cli` depends on all other crates via `Cargo.toml`. If the crate graph is not carefully managed, the `libc` dependency could leak into WASM compilation paths.
- Impact: Blocks the Embedded Evaluator (WASM) milestone if not isolated.
- Files: `crates/cli/Cargo.toml:19` (libc dependency), `crates/cli/src/serve.rs:96-106`
- Migration plan: Ensure `tenor-core` and `tenor-eval` remain free of libc/OS dependencies. They currently are clean -- `tenor-core` uses only `serde`/`serde_json` and `std::path`/`std::fs` (only in `pass1_bundle.rs`). `tenor-eval` has no filesystem or OS calls.

**`tiny_http` is synchronous and single-threaded:**
- Risk: `tiny_http` v0.12 is a simple synchronous HTTP server. It does not support concurrent request handling, TLS, or HTTP/2. It is suitable for local development but not for the Hosted Evaluator Service.
- Impact: The hosted service milestone requires replacing the entire HTTP stack.
- Files: `crates/cli/src/serve.rs` (entire file, 521 lines)
- Migration plan: Replace with `axum` or `actix-web` for the hosted service. Keep `tiny_http` for the local `tenor serve` command if desired.

---

## Missing Critical Features

**No WASM compilation target tested or configured:**
- Problem: The Platform & Ecosystem milestone targets an Embedded Evaluator compiled to WASM. Currently, no `wasm32` target is configured, no WASM-specific feature flags exist, and the build has not been tested under `wasm32-unknown-unknown` or `wasm32-wasi`. The `tenor-core` crate uses `std::fs::read_to_string` and `std::path::Path::canonicalize` in `pass1_bundle.rs`, which are not available in browser WASM.
- Files: `crates/core/src/pass1_bundle.rs:154` (filesystem read), `crates/core/src/elaborate.rs:18` (takes `&Path`)
- Blocks: Embedded Evaluator (WASM for browser/Node/edge).
- Fix approach: Factor `pass1_bundle.rs` file I/O behind a trait or feature flag. The evaluator (`tenor-eval`) already consumes interchange JSON without filesystem access, so it is closer to WASM-ready. The elaborator would need a `source_provider` abstraction.

**No multi-party execution runtime:**
- Problem: The System construct is elaborated and validated, but there is no runtime that can execute cross-contract triggers, shared entity state, or shared persona identity across multiple evaluator instances.
- Files: System elaboration in `crates/core/src/pass5_validate.rs` (validation only), no runtime in `crates/eval/`
- Blocks: Multi-party Contract Execution milestone.
- Fix approach: Design a System runtime coordinator that manages trigger dispatch and shared state.

**No Rust or Go SDK:**
- Problem: Only a TypeScript SDK exists (`sdk/typescript/`). The Platform & Ecosystem milestone targets Rust and Go Agent SDKs.
- Files: `sdk/typescript/` (existing), no `sdk/rust/` or `sdk/go/`
- Blocks: Rust and Go Agent SDK milestones.

---

## Test Coverage Gaps

**`crates/core/src/` -- no inline unit tests for parser or lexer:**
- What's not tested: The `crates/core/src/` directory has only 2 `#[cfg(test)]` modules (in `pass3_types.rs` and `pass5_validate.rs`). The parser (1,598 lines), lexer (419 lines), pass1_bundle (258 lines), pass2_index (190 lines), pass4_typecheck (403 lines), and pass6_serialize (1,044 lines) have zero inline unit tests. All testing is done via file-based conformance tests.
- Files: `crates/core/src/parser.rs`, `crates/core/src/lexer.rs`, `crates/core/src/pass1_bundle.rs`, `crates/core/src/pass2_index.rs`, `crates/core/src/pass4_typecheck.rs`, `crates/core/src/pass6_serialize.rs`
- Risk: Debugging individual pass regressions requires running the full conformance suite.
- Priority: Medium. The conformance suite provides good coverage, but unit tests would improve development velocity.

**`crates/lsp/` -- zero unit tests:**
- What's not tested: The entire LSP crate (2,711 lines across 8 files) has no `#[cfg(test)]` modules and no integration tests. Navigation (809 lines), agent capabilities (543 lines), semantic tokens (480 lines), completion (267 lines), and hover (80 lines) are untested.
- Files: `crates/lsp/src/navigation.rs`, `crates/lsp/src/agent_capabilities.rs`, `crates/lsp/src/semantic_tokens.rs`, `crates/lsp/src/completion.rs`, `crates/lsp/src/hover.rs`, `crates/lsp/src/server.rs`
- Risk: LSP features can silently break. Navigation and completion correctness is not verified.
- Priority: High -- LSP is user-facing and affects developer experience.

**`crates/cli/src/diff.rs` -- CLI integration not tested:**
- What's not tested: The `tenor diff` subcommand is not covered by `crates/cli/tests/cli_integration.rs`. Internal unit tests exist but CLI argument parsing, output format, and `--breaking` flag are untested end-to-end.
- Files: `crates/cli/src/diff.rs`, `crates/cli/tests/cli_integration.rs`
- Risk: CLI surface changes go undetected.
- Priority: Medium.

**`crates/cli/src/explain.rs` -- Markdown format output not tested:**
- What's not tested: `ExplainFormat::Markdown` produces different syntax than `ExplainFormat::Terminal`. No tests assert Markdown-specific output.
- Files: `crates/cli/src/explain.rs`
- Risk: Silent formatting breakage.
- Priority: Low.

**`crates/analyze/` -- S3a admissibility has no negative test cases:**
- What's not tested: Contracts where admissibility checks should fire (unreachable state predicates, missing fact references).
- Files: `crates/analyze/src/s3a_admissibility.rs`, `crates/analyze/tests/analysis_tests.rs`
- Risk: False-negative findings in admissibility analysis.
- Priority: Medium.

**Flow error-path conformance tests missing:**
- What's not tested: No file-based conformance fixtures exercise `FlowError`, `OperationError::EntityNotFound`, or `FailureHandler::Escalate`. These paths are only covered by inline unit tests that use `panic!` assertions.
- Files: `crates/eval/src/flow.rs`, `crates/eval/src/operation.rs`
- Risk: Error handling path regressions go undetected by the conformance suite.
- Priority: Medium.

---

## Platform & Ecosystem Readiness

**WASM compilation blockers in `tenor-core`:**
- The elaborator entry point `elaborate()` in `crates/core/src/elaborate.rs:18` takes a `&Path` and `pass1_bundle.rs` reads files from disk. These are incompatible with `wasm32-unknown-unknown`.
- `tenor-eval` is WASM-ready: it has no `std::fs`, no `std::path`, no `libc`, and no OS-specific code. It consumes `serde_json::Value` input.
- `tenor-analyze` is also WASM-candidate: no filesystem dependencies.
- Fix: For browser WASM, expose `tenor-eval` and `tenor-analyze` directly. For elaboration in WASM, factor file I/O in `pass1_bundle.rs` behind a trait that can accept in-memory sources.

**Hosted service requires complete HTTP stack replacement:**
- The current `serve.rs` (521 lines) uses `tiny_http` -- single-threaded, no TLS, no auth, no rate limiting. The Hosted Evaluator Service milestone requires: async runtime, concurrent request handling, authentication, rate limiting, TLS termination, structured logging, and metrics.
- The `libc`-based signal handling and the `Mutex<ServeState>` pattern would need to be replaced entirely.
- The evaluate endpoint clones the entire bundle JSON from the Mutex on every request (`crates/cli/src/serve.rs:402`), which is wasteful.

**Multi-party execution has no runtime foundation:**
- System construct elaboration and validation exist, but the evaluator has no concept of multi-contract execution, cross-contract triggers, or shared entity state synchronization.
- The `Contract` type in `crates/eval/src/types.rs` represents a single contract. A `SystemContract` coordinator would need to manage multiple `Contract` instances, trigger dispatch, and entity state consistency.

**SDK generation requires stable interchange schema:**
- The interchange JSON schema at `docs/interchange-schema.json` is the contract between the Rust evaluator and client SDKs. Currently only TypeScript SDK exists. Rust and Go SDKs will need generated or hand-written types matching this schema.
- The triplicated deserialization pattern (eval, analyze, codegen each parsing interchange independently) suggests the interchange types are not formalized as a shared library, which would be the natural base for SDK generation.

---

*Concerns audit: 2026-02-23*
