# External Integrations

**Analysis Date:** 2026-02-22

## APIs & External Services

**AI / LLM:**
- Anthropic Claude API — used exclusively by the `ambiguity` subcommand to test whether an LLM can evaluate Tenor contracts and produce the same verdicts as the reference elaborator
  - SDK/Client: `ureq` 3.2.0 (raw HTTP, no official Anthropic Rust SDK)
  - Endpoint: `https://api.anthropic.com/v1/messages`
  - API version header: `anthropic-version: 2023-06-01`
  - Default model: `claude-sonnet-4-5-20250514` (overridable via `--model` flag)
  - Auth: `ANTHROPIC_API_KEY` environment variable
  - Implementation: `crates/cli/src/ambiguity/api.rs`
  - Retry policy: 3 retries with exponential backoff (starting 1000ms, doubling) on HTTP 429/500/502/503 and network errors

## Data Storage

**Databases:**
- None — no database of any kind

**File Storage:**
- Local filesystem only
  - Source files: `.tenor` files read from user-specified paths
  - Interchange bundles: `.json` files read/written to user-specified paths
  - Conformance suite: `conformance/` directory tree read by `cargo run -p tenor-cli -- test conformance`
  - Schema files: embedded at compile time (not read at runtime)

**Caching:**
- None

## Authentication & Identity

**Auth Provider:**
- None — the CLI has no user authentication
- The only credential in the system is `ANTHROPIC_API_KEY` for the AI ambiguity testing subcommand (optional, feature degrades gracefully when absent)

## Monitoring & Observability

**Error Tracking:**
- None — no error reporting service integrated

**Logs:**
- `stderr` only — errors and diagnostic messages go to stderr; structured output goes to stdout
- No log framework (no `tracing`, `log`, `env_logger`)
- TAP v14 format output for conformance and ambiguity test runs (implemented in `crates/cli/src/tap.rs`)

## CI/CD & Deployment

**Hosting:**
- No deployment target — this is a CLI toolchain, not a hosted service

**CI Pipeline:**
- GitHub Actions — `.github/workflows/ci.yml`
- Triggers: push/PR to `main` and `v1` branches
- Runner: `ubuntu-latest`
- Steps (in order):
  1. `cargo build --workspace`
  2. `cargo run -p tenor-cli -- test conformance` (conformance suite)
  3. `cargo test --workspace` (schema validation + unit tests)
  4. `cargo fmt --all -- --check`
  5. `cargo clippy --workspace -- -D warnings`
- Rust toolchain action: `dtolnay/rust-toolchain@stable`
- Build cache: `Swatinem/rust-cache@v2`

## Environment Configuration

**Required env vars:**
- None for normal operation (`elaborate`, `validate`, `eval`, `test`, `diff`, `check`, `explain`)

**Optional env vars:**
- `ANTHROPIC_API_KEY` — enables `tenor ambiguity` subcommand; the command skips gracefully with an informational message if absent

**Secrets location:**
- No secrets committed to repo; `ANTHROPIC_API_KEY` is a runtime environment variable only

## Webhooks & Callbacks

**Incoming:**
- None

**Outgoing:**
- None (the Anthropic API call from `tenor ambiguity` is a user-initiated CLI invocation, not a webhook)

## JSON Schema Registry

**Schema $id URIs** (used for `$ref` resolution, not live network calls):
- `https://tenor-lang.org/schemas/interchange/v1.0.0` — interchange bundle schema in `docs/interchange-schema.json`
- `https://tenor-lang.org/schemas/manifest/v1.1.0` — manifest envelope schema in `docs/manifest-schema.json`

These URIs are registered in-process at validation time via `jsonschema::options().with_resource(...)`. No live network requests are made to `tenor-lang.org` at runtime.

---

*Integration audit: 2026-02-22*
