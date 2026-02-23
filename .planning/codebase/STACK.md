# Technology Stack

**Analysis Date:** 2026-02-22

## Languages

**Primary:**
- Rust (edition 2021, workspace version 0.1.0) - All production code across all crates

**Secondary:**
- JSON Schema (Draft 2020-12) - Interchange format schema definitions at `docs/interchange-schema.json` and `docs/manifest-schema.json`
- Tenor DSL (custom language) - Contract definition language in `.tenor` source files (parsed by `crates/core/`)

## Runtime

**Environment:**
- Native binary (no VM/runtime dependency)
- Rust std, no async runtime (all I/O is synchronous)

**Package Manager:**
- Cargo 1.93.1
- Lockfile: `Cargo.lock` — present and committed

## Frameworks

**Core:**
- No framework — pure Rust library/binary crates

**CLI:**
- `clap` 4.5.60 (derive feature) — argument parsing and subcommand dispatch in `crates/cli/src/main.rs`

**Testing:**
- `assert_cmd` 2.1.2 — spawn the `tenor` binary and assert on exit code/stdout/stderr in `crates/cli/tests/cli_integration.rs`
- `predicates` 3.1.4 — compose assertions for `assert_cmd` tests
- `tempfile` 3.25.0 — temporary file/dir creation in integration tests

**Build/Dev:**
- `rustfmt` (stable toolchain) — enforced via `cargo fmt --all -- --check` in CI
- `clippy` (stable toolchain, `-D warnings`) — enforced in CI

## Key Dependencies

**Critical:**
- `serde` 1.0.228 (derive feature) — serialization/deserialization of AST and interchange JSON; used in every crate
- `serde_json` 1.0.149 — JSON value manipulation, pretty-printing, and interchange serialization; used in every crate
- `rust_decimal` 1.40.0 (serde-with-str feature) — arbitrary-precision decimal arithmetic for Money and Decimal types; used in `crates/eval/` and `crates/core/`
- `jsonschema` 0.42.1 — validates interchange JSON bundles against embedded JSON Schema; used in `crates/cli/src/main.rs` and `crates/core/tests/schema_validation.rs`

**Infrastructure:**
- `sha2` 0.10.9 — SHA-256 etag computation for TenorManifest envelopes in `crates/cli/src/main.rs` and `crates/cli/src/runner.rs`
- `ureq` 3.2.0 (json feature) — synchronous HTTP client for Anthropic API calls in `crates/cli/src/ambiguity/api.rs`
- `indexmap` 2.13.0 — ordered map (transitive dependency of jsonschema)
- `fancy-regex` 0.17.0 — regex with lookahead (transitive dependency of jsonschema)

## Configuration

**Environment:**
- `ANTHROPIC_API_KEY` — required only for `tenor ambiguity` subcommand; read via `std::env::var` in `crates/cli/src/ambiguity/api.rs`
- No other runtime environment variables; no `.env` files present

**Build:**
- `Cargo.toml` — workspace root at `/Cargo.toml`; workspace-level `[workspace.dependencies]` pins all shared dep versions
- `Cargo.lock` — committed; ensures reproducible builds
- No build scripts (`build.rs`) in any crate

**Embedded assets:**
- `docs/interchange-schema.json` — embedded into the CLI binary at compile time via `include_str!()` in `crates/cli/src/main.rs`
- `docs/manifest-schema.json` — embedded into the CLI binary at compile time via `include_str!()`

## Platform Requirements

**Development:**
- Rust stable toolchain (tested at 1.93.1)
- No system libraries required beyond Rust std

**Production:**
- Single static-ish binary: `tenor` (compiled from `crates/cli/`)
- CI target: `ubuntu-latest` (GitHub Actions)
- No containerization, no runtime dependencies, no database

---

*Stack analysis: 2026-02-22*
