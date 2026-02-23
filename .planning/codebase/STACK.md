# Technology Stack

**Analysis Date:** 2026-02-23

## Languages

**Primary:**
- Rust (edition 2021, workspace version 0.1.0) — All production crates: core, cli, eval, analyze, codegen, lsp (~32,254 LOC)

**Secondary:**
- TypeScript ~5.7 — TypeScript SDK (`sdk/typescript/`, ~346 LOC) and VS Code extension (`editors/vscode/`, ~964 LOC)
- Tenor DSL (custom language) — Contract definition language in `.tenor` source files, parsed by `crates/core/`
- JSON Schema (Draft 2020-12) — Interchange format schema at `docs/interchange-schema.json` and `docs/manifest-schema.json`

## Runtime

**Environment:**
- Native binary (no VM/runtime dependency for Rust crates)
- Rust std only, no async runtime — all I/O is synchronous (including `tiny_http` HTTP server)
- Node.js >= 22.0.0 for TypeScript SDK (uses `--experimental-strip-types` for tests)
- VS Code >= 1.85.0 for editor extension

**Package Manager:**
- Cargo (Rust) — workspace root `Cargo.toml`
- Lockfile: `Cargo.lock` — present and committed
- npm (TypeScript SDK) — `sdk/typescript/package.json`
- npm (VS Code extension) — `editors/vscode/package.json`

## Frameworks

**Core:**
- No framework — pure Rust library/binary crates

**CLI:**
- `clap` 4.5 (derive feature) — argument parsing and subcommand dispatch in `crates/cli/src/main.rs`

**HTTP Server:**
- `tiny_http` 0.12 — synchronous HTTP server for `tenor serve` in `crates/cli/src/serve.rs`

**LSP:**
- `lsp-server` 0.7 — Language Server Protocol transport layer in `crates/lsp/src/server.rs`
- `lsp-types` 0.97 — LSP type definitions (requests, responses, capabilities)

**Editor:**
- `vscode-languageclient` ^9.0.0 — LSP client for VS Code extension in `editors/vscode/src/extension.ts`

**Testing:**
- `assert_cmd` 2 — CLI binary integration tests in `crates/cli/tests/cli_integration.rs`
- `predicates` 3 — composable assertions for `assert_cmd` tests
- `tempfile` 3 — temporary file/dir creation in integration tests
- Node.js built-in test runner (`node --test`) — TypeScript SDK tests in `sdk/typescript/tests/`

**Build/Dev:**
- `rustfmt` (stable toolchain) — enforced via `cargo fmt --all -- --check` in CI
- `clippy` (stable toolchain, `-D warnings`) — enforced in CI
- `tsc` (TypeScript compiler) — dual-emit ESM + CJS for SDK, extension compilation

## Key Dependencies

**Critical (used across multiple crates):**
- `serde` 1 (derive feature) — serialization/deserialization of AST and interchange JSON; used in every Rust crate
- `serde_json` 1 — JSON value manipulation, pretty-printing, and interchange serialization; used in every Rust crate

**Domain Logic:**
- `rust_decimal` 1.36 (serde-with-str feature) — arbitrary-precision decimal arithmetic for Money and Decimal types; used in `crates/eval/src/numeric.rs`
- `time` 0.3 (parsing, macros features) — date/time parsing for Tenor Date type; used in `crates/eval/src/types.rs`

**Validation:**
- `jsonschema` 0.42 — validates interchange JSON bundles against embedded JSON Schema; used in `crates/cli/src/main.rs` and `crates/core/tests/schema_validation.rs`

**Infrastructure:**
- `sha2` 0.10 — SHA-256 etag computation for TenorManifest envelopes in `crates/cli/src/manifest.rs`
- `ureq` 3 (json feature) — synchronous HTTP client for Anthropic API calls in `crates/cli/src/ambiguity/api.rs`
- `libc` 0.2 — low-level system calls (signal handling) in CLI

**Dev-only:**
- `assert_cmd` 2 — CLI integration tests (`crates/cli/tests/`)
- `predicates` 3 — assertion combinators (`crates/cli/tests/`)
- `tempfile` 3 — temp files/dirs (`crates/cli/tests/`, `crates/codegen/tests/`)

## Configuration

**Environment Variables:**
- `ANTHROPIC_API_KEY` — required only for `tenor ambiguity` subcommand; read via `std::env::var` in `crates/cli/src/ambiguity/api.rs`
- No other runtime environment variables; no `.env` files present

**Build Configuration:**
- `Cargo.toml` — workspace root; `[workspace.dependencies]` pins all shared dependency versions
- `Cargo.lock` — committed; ensures reproducible builds
- No build scripts (`build.rs`) in any crate

**Embedded Assets:**
- `docs/interchange-schema.json` — embedded into CLI binary at compile time via `include_str!()` in `crates/cli/src/main.rs`
- `docs/manifest-schema.json` — embedded into CLI binary at compile time via `include_str!()`

**TypeScript SDK Configuration:**
- `sdk/typescript/tsconfig.build.json` — ESM output build config
- `sdk/typescript/tsconfig.cjs.json` — CJS output build config
- Dual-package: ESM (`dist/esm/`) + CJS (`dist/cjs/`) + types (`dist/types/`)

**VS Code Extension Configuration:**
- `editors/vscode/tsconfig.json` — extension compilation
- `editors/vscode/language-configuration.json` — bracket/comment configuration for Tenor files
- `editors/vscode/syntaxes/tenor.tmLanguage.json` — TextMate grammar for syntax highlighting
- `editors/vscode/snippets/tenor.json` — code snippets

## CI/CD

**CI Pipeline:** `.github/workflows/ci.yml`
- Triggers: push/PR to `main` and `v1` branches
- Runner: `ubuntu-latest`
- Toolchain: `dtolnay/rust-toolchain@stable`
- Caching: `Swatinem/rust-cache@v2`

**CI Steps (in order):**
1. `cargo build --workspace` — compile all crates
2. `cargo run -p tenor-cli -- test conformance` — run conformance suite (73 tests)
3. `cargo test --workspace` — run all unit and integration tests (~398 tests)
4. `cargo fmt --all -- --check` — formatting check
5. `cargo clippy --workspace -- -D warnings` — lint check (warnings are errors)

**Docker:**
- `Dockerfile` — multi-stage build: `rust:1.93-slim` builder, `debian:trixie-slim` runtime
- `docker-compose.yml` — single `evaluator` service, exposes port 8080, mounts `domains/` as `/contracts`
- Default entrypoint: `tenor serve --port 8080`

## Platform Requirements

**Development:**
- Rust stable toolchain (tested at 1.93.1, `aarch64-apple-darwin`)
- No system libraries required beyond Rust std
- Node.js >= 22 (for TypeScript SDK development)
- VS Code >= 1.85 (for extension development)

**Production:**
- Single binary: `tenor` (compiled from `crates/cli/`)
- CI target: `ubuntu-latest` (GitHub Actions)
- Docker image: `debian:trixie-slim` (minimal runtime)
- No runtime database or external service dependencies (except `ANTHROPIC_API_KEY` for ambiguity testing)

## Codebase Size

| Component | LOC | Language |
|-----------|-----|----------|
| Rust crates | ~32,254 | Rust |
| Domain contracts | ~4,480 | Tenor DSL |
| TypeScript SDK | ~346 | TypeScript |
| VS Code extension | ~964 | TypeScript |
| Conformance fixtures | 112 `.tenor` files | Tenor DSL + JSON |

---

*Stack analysis: 2026-02-23*
