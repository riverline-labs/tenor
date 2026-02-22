# External Integrations

**Analysis Date:** 2026-02-21

## APIs & External Services

**AI / LLM:**
- Anthropic Messages API - Used by the `ambiguity` CLI subcommand to test whether an LLM can unambiguously evaluate Tenor contracts against fact sets and produce correct verdicts
  - SDK/Client: `ureq` 3.2.0 (synchronous HTTP, no official Anthropic Rust SDK used)
  - Endpoint: `https://api.anthropic.com/v1/messages`
  - API Version header: `anthropic-version: 2023-06-01`
  - Auth: `ANTHROPIC_API_KEY` environment variable
  - Default model: `claude-sonnet-4-5-20250514` (overridable via `--model` CLI flag)
  - Implementation: `crates/cli/src/ambiguity/api.rs`
  - Retry logic: exponential backoff (3 retries, starting 1000ms) on 429, 500, 503 responses

## Data Storage

**Databases:**
- None — no database used anywhere in the codebase

**File Storage:**
- Local filesystem only — contracts read from `.tenor` files, interchange JSON written to stdout or read from disk
- Conformance suite fixtures stored as files under `conformance/`

**Caching:**
- None

## Authentication & Identity

**Auth Provider:**
- None — the tool is a local CLI; no user authentication
- The only auth is the `ANTHROPIC_API_KEY` env var for the AI ambiguity testing subcommand (see above)

## Monitoring & Observability

**Error Tracking:**
- None — errors are printed to stderr and exit codes signal failure

**Logs:**
- `eprintln!` to stderr for errors and informational messages; no structured logging library

## CI/CD & Deployment

**Hosting:**
- Not deployed as a service; distributed as a compiled binary

**CI Pipeline:**
- GitHub Actions — `.github/workflows/ci.yml`
- Triggers: push and PR to `main` and `v1` branches
- Runner: `ubuntu-latest`
- Steps:
  1. `cargo build --workspace`
  2. `cargo run -p tenor-cli -- test conformance` (conformance suite, 55 tests)
  3. `cargo test --workspace` (schema validation + unit tests)
  4. `cargo fmt --all -- --check`
  5. `cargo clippy --workspace -- -D warnings`
- Build caching: `Swatinem/rust-cache@v2`

## Environment Configuration

**Required env vars:**
- `ANTHROPIC_API_KEY` - Only required for `tenor ambiguity` subcommand; the command skips gracefully (exits 0 with message) if not set

**Secrets location:**
- No secrets files committed; API key sourced from environment only

## Webhooks & Callbacks

**Incoming:**
- None

**Outgoing:**
- None — the only outbound HTTP is the Anthropic API call in `crates/cli/src/ambiguity/api.rs`, which is user-initiated via CLI command

---

*Integration audit: 2026-02-21*
