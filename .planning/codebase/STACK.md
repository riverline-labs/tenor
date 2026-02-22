# Technology Stack

**Analysis Date:** 2026-02-21

## Languages

**Primary:**
- Rust (stable, 1.93.1 in dev) - All crates: core library, CLI binary, eval, analyze, codegen, lsp

**Secondary:**
- JSON - Interchange format, conformance fixtures, JSON Schema definition at `docs/interchange-schema.json`

## Runtime

**Environment:**
- Rust native binary (`tenor` CLI), no VM or interpreter

**Package Manager:**
- Cargo 1.93.1
- Lockfile: `Cargo.lock` present and committed

## Frameworks

**Core:**
- No application framework — pure library and CLI crates

**CLI Argument Parsing:**
- `clap` 4.5.60 (derive feature) - CLI subcommand dispatch in `crates/cli/src/main.rs`

**Testing:**
- Built-in Rust test harness (`cargo test`) - unit tests within each crate
- Custom TAP v14 conformance runner at `crates/cli/src/runner.rs` and `crates/cli/src/tap.rs`
- `assert_cmd` 2.1.2 - CLI integration testing (dev dependency in `crates/cli`)
- `predicates` 3.1.4 - Assertion helpers for `assert_cmd` tests
- `tempfile` 3.25.0 - Temporary file management in tests
- `jsonschema` 0.42.1 - JSON Schema validation used both in tests (`crates/core` dev-dep) and production CLI (`crates/cli`)

**Build/Dev:**
- `cargo fmt` - Code formatting (enforced in CI)
- `cargo clippy` - Linting (enforced in CI with `-D warnings`)
- `Swatinem/rust-cache@v2` - GitHub Actions CI build caching

## Key Dependencies

**Critical:**
- `serde` 1.0.228 (derive feature) - Serialization/deserialization framework; used in every crate
- `serde_json` 1.0.149 - JSON parsing and serialization; central to interchange format output
- `rust_decimal` 1.40.0 (serde-with-str feature) - Arbitrary-precision decimal arithmetic for Money and Decimal types; used in `crates/core` and `crates/eval`

**Infrastructure:**
- `ureq` 3.2.0 (json feature) - Synchronous HTTP client; used exclusively in `crates/cli/src/ambiguity/api.rs` for Anthropic API calls
- `jsonschema` 0.42.1 - JSON Schema validation; used in `crates/cli` for the `validate` subcommand and in `crates/core` dev-tests

## Configuration

**Environment:**
- `ANTHROPIC_API_KEY` - Required for `tenor ambiguity` subcommand; read at runtime via `std::env::var`
- No `.env` files or environment loading libraries; env vars read directly

**Build:**
- `Cargo.toml` (workspace root) - Workspace members and shared dependency versions
- `crates/*/Cargo.toml` - Per-crate dependencies
- `Cargo.lock` - Pinned dependency tree

**Embedded Assets:**
- `docs/interchange-schema.json` is embedded at compile time via `include_str!` in `crates/cli/src/main.rs` (used by `validate` subcommand)

## Platform Requirements

**Development:**
- Rust stable toolchain (no `rust-toolchain.toml` pinning; CI uses `dtolnay/rust-toolchain@stable`)
- Cargo

**Production:**
- Single compiled binary `tenor` (cross-platform; CI tests on `ubuntu-latest`)
- No database, no server, no runtime dependencies beyond the binary itself

---

*Stack analysis: 2026-02-21*
