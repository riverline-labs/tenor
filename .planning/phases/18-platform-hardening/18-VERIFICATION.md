---
phase: 18-platform-hardening
verified: 2026-02-23T16:34:39Z
status: passed
score: 27/27 must-haves verified
re_verification: false
---

# Phase 18: Platform Hardening Verification Report

**Phase Goal:** Fix all blocking concerns identified by codebase mapping -- shared interchange library, typed explain.rs, error recovery in parser, WASM-ready I/O abstraction, production HTTP stack, security hardening, SystemContract coordinator design, indexed lookups, LSP tests, and conformance fixture gaps.
**Verified:** 2026-02-23T16:34:39Z
**Status:** passed
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | A single `tenor-interchange` crate provides typed deserialization of interchange JSON bundles | VERIFIED | `crates/interchange/src/{lib.rs,types.rs,deserialize.rs}` all exist with 1060 total lines of substantive code |
| 2 | eval, analyze, and codegen all depend on tenor-interchange instead of hand-rolling their own parsers | VERIFIED | `tenor-interchange = { path = "../interchange" }` in all three Cargo.toml files; `tenor_interchange::from_interchange()` called in eval's `Contract::from_interchange()`, analyze's `bundle.rs`, and codegen's `bundle.rs` |
| 3 | No expect() calls remain in pass5_validate.rs or pass3_types.rs | VERIFIED | `grep -n "expect("` returns zero results in both files; all use `ElabError::new()` with `?` |
| 4 | Import cycle detection uses HashSet for O(1) membership tests | VERIFIED | `crates/core/src/pass1_bundle.rs` declares `let mut stack_set: HashSet<PathBuf>` with explicit O(1) comments; `visited: HashSet<PathBuf>` also present |
| 5 | Parser reports multiple errors per parse up to a configurable limit | VERIFIED | `parser.rs` has `-> Result<(Vec<RawConstruct>, Vec<ElabError>), ElabError>` return type; unit tests `multi_error_reports_both_construct_errors`, `max_errors_limit_stops_collection`, `fatal_lexer_error_aborts_immediately` all pass |
| 6 | elaborate() can be called without filesystem access via SourceProvider trait | VERIFIED | `crates/core/src/source.rs` defines `SourceProvider` trait, `FileSystemProvider`, and `InMemoryProvider`; `elaborate_with_provider()` function in `elaborate.rs`; all re-exported from `lib.rs` line 53 |
| 7 | HTTP server uses axum with tokio async runtime for concurrent request handling | VERIFIED | `serve.rs` imports `axum`, `tokio::sync::{Mutex, RwLock}`, `tokio::net::TcpListener`; CPU-bound work runs via `tokio::task::spawn_blocking()` |
| 8 | libc dependency removed from tenor-cli, tenor-core, tenor-eval Cargo.toml | VERIFIED | No `libc` entry in any of the three Cargo.toml files; libc in tree is only from dev-dependency transitive chain (jsonschema/aws-lc) |
| 9 | Signal handling uses tokio::signal instead of unsafe libc | VERIFIED | `shutdown_signal()` in `serve.rs` uses `tokio::signal::ctrl_c()`; zero `unsafe` blocks in `serve.rs` |
| 10 | Elaborate endpoint validates user content before writing to temp files | VERIFIED | `MAX_SOURCE_SIZE = 1MB`, null-byte check, import path escape check, filename sanitization -- all happen before tempfile creation |
| 11 | CORS, rate limiting, and optional API key auth on all HTTP endpoints | VERIFIED | `CorsLayer` from `tower_http::cors` applied as a router layer; `RateLimiter` struct with per-IP HashMap; `TENOR_API_KEY` env var gates auth; `/health` exempt |
| 12 | explain.rs uses typed interchange structs from tenor-interchange | VERIFIED | `use tenor_interchange::{EntityConstruct, FactConstruct, FlowConstruct, ...}` at top; `tenor_interchange::from_interchange()` called at entry; remaining raw JSON in `walk_steps()` is for `steps: Vec<serde_json::Value>` which is intentionally `serde_json::Value` in the interchange types |
| 13 | spec_sections dead code removed from ambiguity module | VERIFIED | Zero results for `spec_sections` and `allow(dead_code)` in `ambiguity/mod.rs` |
| 14 | LSP dead code annotations resolved in semantic_tokens.rs and navigation.rs | VERIFIED | Zero `allow(dead_code)` annotations remain in either file |
| 15 | All serialization sites reference TENOR_BUNDLE_VERSION constant instead of hardcoded strings | VERIFIED | `pass6_serialize.rs` references `crate::TENOR_VERSION` and `crate::TENOR_BUNDLE_VERSION` throughout; constants defined in `lib.rs` lines 22 and 24 |
| 16 | runner.rs imports manifest/etag logic from manifest.rs | VERIFIED | `use crate::manifest` at line 1 of `runner.rs`; `manifest::build_manifest(bundle)` called at line 256 |
| 17 | S6 MAX_PATHS and MAX_DEPTH are configurable via FlowPathConfig | VERIFIED | `FlowPathConfig { max_paths: usize, max_depth: usize }` with `Default` impl in `s6_flow_paths.rs`; `analyze_flow_paths_with_config()` accepts `&FlowPathConfig`; callers use `FlowPathConfig::default()` |
| 18 | Contract type uses HashMap indexes for O(1) lookups | VERIFIED | `operation_index`, `flow_index`, `entity_index`, `fact_index` all `HashMap<String, usize>` fields; `get_operation()`, `get_flow()`, `get_entity()`, `get_fact()` methods present |
| 19 | Stratum rule evaluation uses BTreeMap index instead of O(k*n) scan | VERIFIED | `let mut stratum_index: BTreeMap<u32, Vec<&crate::types::Rule>>` in `rules.rs` with explicit O(n) comment |
| 20 | Flow execution eliminates unnecessary deep clones | VERIFIED | `std::mem::take(steps_executed)` and `std::mem::take(entity_changes_all)` used in `handle_failure()` return paths |
| 21 | Optional TLS support via axum-server with rustls | VERIFIED | `tls = ["axum-server"]` feature in CLI Cargo.toml; `--tls-cert` and `--tls-key` flags in main.rs; `#[cfg(feature = "tls")]` gate in serve.rs |
| 22 | LSP crate has unit tests covering navigation and completion | VERIFIED | `crates/lsp/tests/lsp_tests.rs` is 339 lines with go-to-definition, find-references, document-symbols, and keyword/fact completion tests |
| 23 | tenor diff CLI is tested end-to-end in cli_integration.rs | VERIFIED | 6 diff test functions: `diff_identical_files_exits_0`, `diff_different_files_exits_1`, `diff_fact_added_shows_addition`, `diff_breaking_non_breaking_changes`, `diff_invalid_file_exits_1`, `diff_nonexistent_file_exits_1` |
| 24 | Flow error-path conformance fixtures exist | VERIFIED | `conformance/eval/positive/flow_error_escalate.{tenor,facts.json,verdicts.json}` exist; `flow_error_escalate` test in `eval/tests/conformance.rs` passes |
| 25 | explain.rs Markdown format output has dedicated test assertions | VERIFIED | `markdown_format_uses_headings_and_backticks()` test at line 1288; tests headings (`##`), backtick quoting, markdown table headers |
| 26 | S3a admissibility has negative test cases that verify findings are reported | VERIFIED | `test_s3a_no_admissible_from_unreachable_state()`, `test_s3a_dead_state_findings_in_full_report()` with `FindingSeverity::Warning` assertions |
| 27 | SystemContract coordinator design is documented | VERIFIED | `docs/system-contract-coordinator.md` exists at 278 lines with problem statement, coordinator architecture, trigger dispatch, shared entity state, persona mapping, API sketch, and implementation phases |

**Score:** 27/27 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|---------|---------|--------|---------|
| `crates/interchange/src/lib.rs` | Public API for shared interchange types | VERIFIED | 17 lines, re-exports public API |
| `crates/interchange/src/types.rs` | Typed structs for all interchange constructs | VERIFIED | 232 lines, covers Fact/Entity/Rule/Operation/Flow/Persona/System/TypeDecl |
| `crates/interchange/src/deserialize.rs` | `from_interchange()` deserialization | VERIFIED | 811 lines, full typed deserialization |
| `crates/core/src/pass5_validate.rs` | Panic-free structural validation | VERIFIED | Zero `expect()` calls; returns `ElabError` throughout |
| `crates/core/src/pass3_types.rs` | Panic-free type cycle detection | VERIFIED | Zero `expect()` calls; `HashSet<String> visited` DFS |
| `crates/core/src/pass1_bundle.rs` | HashSet-based cycle detection | VERIFIED | `stack_set: HashSet<PathBuf>` with O(1) membership comments |
| `crates/core/src/parser.rs` | Multi-error parser with error recovery | VERIFIED | Returns `(Vec<RawConstruct>, Vec<ElabError>)`; 4 unit tests |
| `crates/core/src/elaborate.rs` | SourceProvider trait and elaborate_with_provider() | VERIFIED | `elaborate_with_provider()` function; `use crate::source::SourceProvider` |
| `crates/core/src/source.rs` | FileSystemProvider and InMemoryProvider | VERIFIED | Full trait + two implementations with unit tests |
| `crates/eval/src/types.rs` | HashMap-indexed Contract type | VERIFIED | 4 HashMap fields + 4 lookup methods |
| `crates/eval/src/rules.rs` | Indexed stratum evaluation | VERIFIED | `BTreeMap<u32, Vec<&Rule>> stratum_index` |
| `crates/eval/src/flow.rs` | Clone-free flow execution | VERIFIED | `std::mem::take()` at return sites |
| `crates/cli/src/serve.rs` | Production-grade HTTP server | VERIFIED | axum + tokio, CorsLayer, RateLimiter, API key auth, TLS support |
| `crates/cli/src/explain.rs` | Typed explain using tenor-interchange | VERIFIED | `use tenor_interchange::` at top; typed struct access throughout |
| `crates/analyze/src/s6_flow_paths.rs` | Configurable path enumeration limits | VERIFIED | `FlowPathConfig { max_paths, max_depth }` with Default |
| `crates/lsp/tests/lsp_tests.rs` | LSP unit tests (min 50 lines) | VERIFIED | 339 lines with navigation and completion tests |
| `crates/cli/tests/cli_integration.rs` | End-to-end diff tests | VERIFIED | 6 diff test functions present |
| `conformance/eval/positive/flow_error_escalate.tenor` | Flow error-path test fixture | VERIFIED | All 3 fixture files exist |
| `docs/system-contract-coordinator.md` | SystemContract coordinator design | VERIFIED | 278 lines, well-structured design document |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/eval/src/types.rs` | `crates/interchange/src/types.rs` | `use tenor_interchange::InterchangeConstruct` | WIRED | Line 386 in types.rs |
| `crates/analyze/src/bundle.rs` | `crates/interchange/src/types.rs` | `use tenor_interchange::InterchangeConstruct` | WIRED | Line 10 in bundle.rs |
| `crates/codegen/src/bundle.rs` | `crates/interchange/src/types.rs` | `use tenor_interchange::InterchangeConstruct` | WIRED | Line 10 in bundle.rs |
| `crates/core/src/pass5_validate.rs` | `crates/core/src/error.rs` | `ElabError::new()` replacing expect() | WIRED | Confirmed zero expect() calls |
| `crates/core/src/elaborate.rs` | `crates/core/src/pass1_bundle.rs` | `SourceProvider` passed through pipeline | WIRED | `load_bundle_with_provider()` takes `&dyn SourceProvider` |
| `crates/core/src/parser.rs` | `crates/core/src/error.rs` | `Vec<ElabError>` collected during recovery | WIRED | Function signature confirmed |
| `crates/eval/src/rules.rs` | `crates/eval/src/types.rs` | `BTreeMap<u32, Vec<&crate::types::Rule>>` stratum index | WIRED | Line 31 in rules.rs |
| `crates/eval/src/flow.rs` | `crates/eval/src/types.rs` | FlowResult via `std::mem::take()` | WIRED | Lines 107-108 in flow.rs |
| `crates/cli/src/serve.rs` | `crates/eval/src/lib.rs` | `tokio::task::spawn_blocking(|| tenor_eval::evaluate(...))` | WIRED | Line 661 in serve.rs |
| `crates/cli/src/serve.rs` | `crates/core/src/elaborate.rs` | `tokio::task::spawn_blocking(|| tenor_core::elaborate(...))` | WIRED | Line 519 in serve.rs |
| `crates/cli/src/explain.rs` | `crates/interchange/src/types.rs` | `use tenor_interchange::{...}` | WIRED | Lines 14-17 in explain.rs |
| `crates/cli/src/runner.rs` | `crates/cli/src/manifest.rs` | `use crate::manifest` | WIRED | Line 1 in runner.rs |
| `crates/core/src/pass6_serialize.rs` | `crates/core/src/lib.rs` | `crate::TENOR_BUNDLE_VERSION` constant | WIRED | Lines 97, 102, 149, 171, etc. |
| `crates/lsp/tests/lsp_tests.rs` | `crates/lsp/src/navigation.rs` | `tenor_lsp::navigation::goto_definition`, `find_references`, `document_symbols` | WIRED | Lines 76, 99, 116, 138, 157, 180 |
| `crates/eval/tests/conformance.rs` | `conformance/eval/positive/` | `flow_error_escalate` test loading fixture | WIRED | Line 320-323 in conformance.rs |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|---------|
| HARD-01 | 18-01 | Shared interchange deserialization library | SATISFIED | `crates/interchange/` crate exists and all three consumers depend on it |
| HARD-02 | 18-07 | explain.rs uses typed interchange structs | SATISFIED | `tenor_interchange::from_interchange()` at entry; typed struct field access throughout |
| HARD-03 | 18-02 | All expect() in pass5/pass3 replaced | SATISFIED | Zero grep matches for `expect(` in both files |
| HARD-04 | 18-03 | Parser multi-error reporting | SATISFIED | `Vec<ElabError>` return type; 4 unit tests; recovery confirmed |
| HARD-05 | 18-03 | WASM-ready SourceProvider trait | SATISFIED | `source.rs` with trait + two implementations; `elaborate_with_provider()` function |
| HARD-06 | 18-06 | libc removed from core/eval crate graph | SATISFIED | No `libc` in core or eval Cargo.toml; none in serve.rs production code |
| HARD-07 | 18-05 | spec_sections dead code removed | SATISFIED | Zero matches for `spec_sections` in ambiguity module |
| HARD-08 | 18-06 | Production HTTP stack with TLS | SATISFIED | axum + tokio; optional `tls` feature via axum-server + rustls |
| HARD-09 | 18-06 | Unsafe signal handling replaced | SATISFIED | `tokio::signal::ctrl_c()` for graceful shutdown; zero `unsafe` in serve.rs |
| HARD-10 | 18-08 | Elaborate endpoint input validation | SATISFIED | Size limit (1MB), null-byte check, import escape check, filename sanitization |
| HARD-11 | 18-08 | Auth, CORS, rate limiting | SATISFIED | `CorsLayer`, per-IP `RateLimiter`, `TENOR_API_KEY` optional auth |
| HARD-12 | 18-09 | SystemContract coordinator designed | SATISFIED | `docs/system-contract-coordinator.md` exists at 278 lines |
| HARD-13 | 18-04 | Contract HashMap indexes | SATISFIED | 4 HashMap index fields + 4 lookup methods in `eval/src/types.rs` |
| HARD-14 | 18-09 | LSP unit tests | SATISFIED | `crates/lsp/tests/lsp_tests.rs` with 339 lines and navigation/completion coverage |
| HARD-15 | 18-05 | Duplicate manifest/etag logic consolidated | SATISFIED | `runner.rs` uses `crate::manifest::build_manifest()` |
| HARD-16 | 18-05 | LSP dead code annotations resolved | SATISFIED | Zero `allow(dead_code)` in `semantic_tokens.rs` and `navigation.rs` |
| HARD-17 | 18-04 | Stratum BTreeMap indexed evaluation | SATISFIED | `BTreeMap<u32, Vec<&Rule>>` stratum_index built once, iterated in order |
| HARD-18 | 18-04 | Flow deep clones eliminated | SATISFIED | `std::mem::take()` used instead of `clone()` at return sites |
| HARD-19 | 18-05 | S6 limits configurable | SATISFIED | `FlowPathConfig { max_paths, max_depth }` with Default::default() |
| HARD-20 | 18-09 | Flow error-path conformance fixtures | SATISFIED | `flow_error_escalate.{tenor,facts.json,verdicts.json}` + test in conformance.rs |
| HARD-21 | 18-05 | Version string consolidation | SATISFIED | `crate::TENOR_BUNDLE_VERSION` and `crate::TENOR_VERSION` used throughout pass6_serialize.rs |
| HARD-22 | 18-02 | HashSet cycle detection | SATISFIED | `stack_set: HashSet<PathBuf>` in pass1_bundle.rs with explicit O(1) comments |
| HARD-23 | 18-02 | Reduced string allocations in pass6 | SATISFIED | Static key constants (`K_TENOR` etc.) used; `TENOR_VERSION` constant avoids repeated string construction |
| HARD-24 | 18-09 | tenor diff CLI e2e tests | SATISFIED | 6 diff test functions in cli_integration.rs |
| HARD-25 | 18-09 | explain.rs Markdown format tests | SATISFIED | `markdown_format_uses_headings_and_backticks()` test with heading and table assertions |
| HARD-26 | 18-09 | S3a admissibility negative tests | SATISFIED | Two negative tests assert findings with correct `FindingSeverity::Warning` |
| HARD-27 | 18-01 | Interchange schema as shared library | SATISFIED | `tenor-interchange` crate is prerequisite-ready for Phase 24 SDKs |

### Anti-Patterns Found

No anti-patterns detected. Specific checks:

| Check | Result |
|-------|--------|
| `expect()` calls in pass5_validate.rs | Zero |
| `expect()` calls in pass3_types.rs | Zero |
| `unsafe` blocks in serve.rs | Zero |
| `allow(dead_code)` in semantic_tokens.rs | Zero |
| `allow(dead_code)` in navigation.rs | Zero |
| `spec_sections` in ambiguity/mod.rs | Zero |
| Hardcoded `"1.0"` strings in pass6_serialize.rs | Zero (all use `crate::TENOR_VERSION`) |
| `libc` in core/eval Cargo.toml | Zero |

### Human Verification Required

**1. serve.rs concurrent request handling under load**

**Test:** Start `cargo run -p tenor-cli -- serve --port 9000` and send concurrent POST /evaluate requests
**Expected:** All requests handled concurrently without blocking; response times consistent
**Why human:** Can't test async concurrency behavior with static file inspection

**2. TLS handshake with axum-server**

**Test:** Build with `cargo build -p tenor-cli --features tls`, start server with self-signed cert via `--tls-cert` and `--tls-key`, connect with `curl --insecure https://localhost:9000/health`
**Expected:** TLS handshake completes, JSON health response returned
**Why human:** Requires actual certificate files and network connection

**3. API key authentication behavior**

**Test:** Start server with `TENOR_API_KEY=secret-key cargo run -p tenor-cli -- serve --port 9001`, send requests with and without the key
**Expected:** /health returns 200 without key; /contracts returns 401 without key, 200 with `Authorization: Bearer secret-key`
**Why human:** Requires live server process and HTTP client

**4. Rate limiting enforcement**

**Test:** Send >60 requests per minute from same IP to server with default rate limit
**Expected:** 61st+ requests return HTTP 429 with `{"error": "rate limit exceeded", "retry_after": N}`
**Why human:** Requires actual HTTP traffic at sufficient volume

### Build and Test Verification

All automated checks passed at verification time:

- `cargo build --workspace`: Finished in 0.14s (already built, all crates compile)
- `cargo run -p tenor-cli -- test conformance`: 73 pass, 0 fail
- `cargo test --workspace`: All test suites pass
  - tenor-core: 22 passed (includes parser multi-error, SourceProvider, pass3 ElabError tests)
  - tenor-eval: 125 passed (includes flow error escalate)
  - tenor-cli integration: 46 passed (includes 6 diff e2e tests)
  - tenor-analyze: 61 passed (includes S3a negative tests)
  - tenor-lsp: 14 passed (navigation + completion tests)
  - tenor-interchange: 15 passed
- `cargo clippy --workspace -- -D warnings`: Clean (finished in 0.15s)

### Gaps Summary

No gaps found. All 27 HARD requirements are implemented, substantive, and properly wired. The phase goal is fully achieved.

The only nuance worth noting: `explain.rs` retains raw `serde_json::Value` traversal for flow `steps` rendering. This is intentional and consistent with the interchange library design (`FlowConstruct.steps: Vec<serde_json::Value>` -- see `crates/interchange/src/types.rs` line 160). The surrounding constructs (facts, entities, rules, operations, flows as top-level) are all typed. The PLAN explicitly anticipated this: "Use `serde_json::Value` for deeply nested expression trees ... Only the top-level construct structure needs shared types."

---

_Verified: 2026-02-23T16:34:39Z_
_Verifier: Claude (gsd-verifier)_
