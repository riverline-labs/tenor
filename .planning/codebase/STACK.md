# Technology Stack

**Analysis Date:** 2026-02-25

## Languages

**Primary:**
- Rust 2021 edition - Core elaborator, evaluator, static analyzer, LSP server, CLI toolchain

**Secondary:**
- WebAssembly (WASM) - Browser/Node.js runtime for contract evaluator (`crates/tenor-eval-wasm`)

## Runtime

**Environment:**
- Rust 1.93+ (stable toolchain via dtolnay/rust-toolchain)
- WASM target: wasm32-unknown-unknown (via wasm-pack)
- Node.js runtime for WASM bindings (wasm-bindgen targets nodejs)

**Package Manager:**
- Cargo (Rust package manager)
- Lockfile: `Cargo.lock` present

## Frameworks

**Core:**
- `serde` 1.0 with derive - Serialization/deserialization for JSON interchange
- `serde_json` 1.0 - JSON parsing and generation
- `rust_decimal` 1.36 with serde-with-str - Decimal arithmetic with string serialization

**HTTP & Networking:**
- `axum` 0.8 - Async HTTP framework for server endpoints
- `axum-server` 0.8 with tls-rustls - Optional TLS support (feature-gated)
- `tower-http` 0.6 with cors - CORS middleware
- `ureq` 3.0 with json - Synchronous HTTP client for Anthropic API calls

**Async Runtime:**
- `tokio` 1.0 with full features - Async task runtime and I/O

**Testing:**
- `jsonschema` 0.42 - Schema validation for interchange JSON
- `assert_cmd` 2.0 - Command-line integration testing
- `predicates` 3.0 - Assertion predicates
- `tempfile` 3.0 - Temporary file handling in tests
- `wasm-bindgen-test` 0.3 - WASM unit tests

**Time & Scheduling:**
- `time` 0.3 with parsing and macros - Date/time operations

**CLI & Command Dispatch:**
- `clap` 4.5 with derive - Command-line argument parsing and subcommand routing

**Utilities:**
- `sha2` 0.10 - SHA-2 hashing
- `rand` 0.8 - Pseudo-random number generation
- `async-trait` 0.1 - Async trait support
- `lsp-server` 0.7 - LSP protocol server framework
- `lsp-types` 0.97 - LSP type definitions
- `wasm-bindgen` 0.2 - JS/Rust bindings for WASM
- `slab` 0.4 - Slab allocator for WASM contract storage
- `getrandom` 0.2 with js feature - Secure random for WASM
- `thiserror` 2.0 - Error type derivation (storage crate)

## Key Dependencies

**Critical:**
- `tenor-core` (internal) - Elaboration pipeline: lexer, parser, 6-pass orchestrator, type checking, validation, JSON serialization
- `tenor-eval` (internal) - Contract evaluator: fact assembly, rule stratification, verdict derivation
- `tenor-interchange` (internal) - Shared JSON types for elaborated contracts
- `tenor-storage` (internal) - Storage trait and record types for execution backends
- `tenor-analyze` (internal) - Static analysis pass over typed constructs
- `tenor-codegen` (internal) - Code generation from interchange JSON
- `tenor-lsp` (internal) - Language Server Protocol implementation

**Infrastructure:**
- `serde` ecosystem - Required for all JSON interchange; used in every crate
- `tokio` - Powers async HTTP server, concurrent elaboration, flow simulation
- `axum` - REST API surface for elaborate, evaluate, explain, flow simulation endpoints

## Configuration

**Environment:**
- `TENOR_API_KEY` (optional) - API key for server endpoints; if set, all non-health endpoints require key in Authorization header
- `TENOR_RATE_LIMIT` (optional) - Per-IP request limit in requests/minute; defaults to 60
- `ANTHROPIC_API_KEY` (optional) - Anthropic Claude API key for ambiguity testing via `tenor test ambiguity`

**Build:**
- `Cargo.toml` (workspace root) - Workspace configuration with shared version 0.1.0 and shared dependencies
- `crates/*/Cargo.toml` - Individual crate manifests with local dependency paths
- `.github/workflows/ci.yml` - CI pipeline: build, conformance suite, schema validation, formatting, clippy with -D warnings

## Platform Requirements

**Development:**
- Rust 1.93+ (stable)
- cargo-fmt for code formatting
- cargo-clippy for linting (warnings treated as errors in CI)
- wasm-pack for WASM compilation (installed in CI pipeline)

**Production:**
- Docker image: `rust:1.93-slim` for build stage, `debian:trixie-slim` for runtime
- Multi-stage Dockerfile creates stripped binary at `/usr/local/bin/tenor`
- Container exposes port 8080 for HTTP server
- Volume at `/contracts` for contract file mounting

---

*Stack analysis: 2026-02-25*
