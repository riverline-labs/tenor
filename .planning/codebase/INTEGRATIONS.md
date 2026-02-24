# External Integrations

**Analysis Date:** 2026-02-23

## Internal Integration Architecture

### The Interchange Boundary

The interchange JSON bundle is the single integration point between all major components. Every consumer deserializes it independently into its own typed structs:

```
                    .tenor source
                         |
                    [tenor-core]
                    elaborate()
                         |
                         v
              TenorInterchange JSON  <-- the boundary
              /      |       |      \
             /       |       |       \
     [tenor-eval] [tenor-analyze] [tenor-codegen] [tenor-lsp]
     evaluate()   analyze()       generate_ts()   diagnostics
```

Each crate deserializes the interchange JSON into its own typed structs:
- `tenor-eval`: `Contract::from_interchange()` in `crates/eval/src/types.rs`
- `tenor-analyze`: `AnalysisBundle::from_interchange()` in `crates/analyze/src/bundle.rs`
- `tenor-codegen`: `CodegenBundle::from_interchange()` in `crates/codegen/src/bundle.rs`

### Protocol Summary

| Integration | Protocol | Transport | Sync/Async | Implementation |
|------------|----------|-----------|------------|----------------|
| CLI -> Core | Rust function call | In-process | Sync | `tenor_core::elaborate()` |
| CLI -> Eval | Rust function call | In-process | Sync | `tenor_eval::evaluate()` |
| CLI -> Analyze | Rust function call | In-process | Sync | `tenor_analyze::analyze()` |
| CLI -> Codegen | Rust function call | In-process | Sync | `tenor_codegen::generate_typescript()` |
| CLI -> LSP | Rust function call | In-process | Sync | `tenor_lsp::run()` (then stdio) |
| TS SDK -> Serve | HTTP/JSON | TCP (localhost) | Async (fetch) | `TenorClient` -> `tiny_http` |
| VS Code -> LSP | JSON-RPC | stdio | Sync (crossbeam) | `vscode-languageclient` -> `lsp-server` |
| Ambiguity -> Claude | HTTP/JSON | TCP (internet) | Sync (ureq) | `crates/cli/src/ambiguity/api.rs` |

## APIs & External Services

**AI / LLM:**
- Anthropic Claude API -- used exclusively by the `ambiguity` subcommand to test whether an LLM can evaluate Tenor contracts and produce the same verdicts as the reference elaborator
  - SDK/Client: `ureq` 3 (raw HTTP, no official Anthropic Rust SDK)
  - Endpoint: `https://api.anthropic.com/v1/messages`
  - API version header: `anthropic-version: 2023-06-01`
  - Default model: `claude-sonnet-4-5-20250514` (overridable via `--model` flag)
  - Auth: `ANTHROPIC_API_KEY` environment variable
  - Implementation: `crates/cli/src/ambiguity/api.rs`
  - Retry policy: 3 retries with exponential backoff (starting 1000ms, doubling) on HTTP 429/500/502/503 and network errors

## HTTP API Server

**`tenor serve` -- Evaluator HTTP API:**
- Implementation: `crates/cli/src/serve.rs`
- Framework: `tiny_http` 0.12 (synchronous, no async runtime)
- Default port: 8080 (configurable via `--port`)
- State: `Arc<Mutex<HashMap<String, serde_json::Value>>>` for loaded contracts
- Pre-loading: contracts can be pre-loaded at startup via positional args
- Graceful shutdown: SIGINT/SIGTERM handlers via `libc::signal()`

**Endpoints:**

| Method | Path | Request Body | Response |
|--------|------|-------------|----------|
| GET | `/health` | -- | `{ status, tenor_version }` |
| GET | `/contracts` | -- | `{ contracts: [{ id, construct_count, facts, operations, flows }] }` |
| GET | `/contracts/{id}/operations` | -- | `{ operations: [{ id, allowed_personas, effects, preconditions_summary }] }` |
| POST | `/elaborate` | `{ source, filename? }` | Interchange JSON bundle or `{ error, details }` |
| POST | `/evaluate` | `{ bundle_id, facts, flow_id?, persona? }` | Verdict set or flow result |
| POST | `/explain` | `{ bundle_id }` | Explanation object |

- All responses: `Content-Type: application/json`
- Max request body: 10 MB
- `POST /elaborate` writes source to `tempfile::tempdir()` then runs `tenor_core::elaborate::elaborate()` (because the elaborator expects a file path for import resolution)
- `POST /evaluate` releases the mutex lock before running evaluation to avoid blocking

**Docker deployment:**
- `Dockerfile`: multi-stage build (`rust:1.93-slim` builder -> `debian:trixie-slim` runtime)
- `docker-compose.yml`: single `evaluator` service, port 8080, mounts `domains/` as `/contracts:ro`
- Entrypoint: `tenor serve --port 8080`

## TypeScript SDK Client

**`@tenor-lang/sdk` -- TypeScript client for evaluator API:**
- Implementation: `sdk/typescript/src/client.ts`
- Purpose: HTTP client wrapper for the `tenor serve` API
- Types: `sdk/typescript/src/types.ts` (HealthResponse, ContractSummary, OperationInfo, EvalResult, FlowEvalResult, ExplainResult, InterchangeBundle, etc.)
- Errors: `sdk/typescript/src/errors.ts` (TenorError, ConnectionError, ContractNotFoundError, EvaluationError, ElaborationError)
- No external HTTP library -- uses Node.js 22+ built-in `fetch`
- Package: `@tenor-lang/sdk` (npm, dual ESM + CJS)
- Default: `http://localhost:8080`, 30s timeout via `AbortSignal.timeout()`

**Client Methods:**

| Method | HTTP | Path | Purpose |
|--------|------|------|---------|
| `health()` | GET | `/health` | Check server reachability |
| `listContracts()` | GET | `/contracts` | List loaded contracts |
| `getOperations(id)` | GET | `/contracts/{id}/operations` | Get operations for a contract |
| `invoke(id, facts, options?)` | POST | `/evaluate` | Evaluate contract (rules or flow) |
| `explain(id)` | POST | `/explain` | Get human-readable explanation |
| `elaborate(source)` | POST | `/elaborate` | Elaborate .tenor source text |

**Error Classification:**
- HTTP 404 with contract path in URL -> `ContractNotFoundError`
- HTTP 404 with "contract '...' not found" in error -> `ContractNotFoundError`
- Any error on `/evaluate` path -> `EvaluationError`
- Any error on `/elaborate` path -> `ElaborationError`
- All others -> `TenorError`

## Editor Integration

**VS Code Extension (`tenor-lang`):**
- Implementation: `editors/vscode/src/extension.ts`
- LSP client: `vscode-languageclient` ^9.0.0 (connects to `tenor lsp` over stdio)
- TextMate grammar: `editors/vscode/syntaxes/tenor.tmLanguage.json`
- Commands: elaborate file, validate project, open agent capabilities, new file from template, show elaboration output, run conformance tests, open docs
- Configuration: `tenor.path` (custom binary path), `tenor.checkOnType` (debounced on-type checking)

**LSP Server:**
- Implementation: `crates/lsp/src/server.rs`
- Transport: stdio (launched by editor as `tenor lsp`)
- Protocol: `lsp-server` 0.7 + `lsp-types` 0.97 (synchronous, crossbeam channels)
- Capabilities: diagnostics (on open/save), semantic tokens (full), completion (triggers: `:`, ` `), hover, go-to-definition, references, document symbols
- Custom request: `tenor/agentCapabilities` -- extract agent-usable capabilities from a contract
- Custom notification: `tenor/agentCapabilitiesUpdated` -- sent after save with updated capabilities
- State: `DocumentState` (open documents), `ProjectIndex` (workspace-wide construct index, rebuilt on save)

## Code Generation Integration

```
.tenor source or interchange JSON
       |
       v
[tenor-core] elaborate (if .tenor input)
       |
       v
interchange JSON
       |
       v
[tenor-codegen] generate_typescript()
       |
       v
{out_dir}/{kebab-id}/
  +-- types.ts       (TypeScript interfaces)
  +-- schemas.ts     (runtime validation schemas)
  +-- client.ts      (typed contract-specific client wrapper)
  +-- index.ts       (barrel exports)
```

- Invoked via `tenor generate typescript <input> --out <dir> --sdk-import <path>`
- `tenor-codegen` operates on `serde_json::Value` only -- no dependency on `tenor-core`
- The CLI handles `.tenor` -> interchange conversion before passing to codegen

## Data Storage

**Databases:**
- None -- no database of any kind

**File Storage:**
- Local filesystem only
  - Source files: `.tenor` files read from user-specified paths
  - Interchange bundles: `.json` files read/written to user-specified paths
  - Conformance suite: `conformance/` directory tree read by `cargo run -p tenor-cli -- test conformance`
  - Domain contracts: `contracts/` directory with 5 validated domain contracts
  - Schema files: embedded at compile time via `include_str!` (not read at runtime)
  - Generated code: TypeScript files written to user-specified `--out` directory by `tenor generate typescript`
  - Temp files: `handle_elaborate()` in serve.rs writes source to `tempfile::tempdir()` for elaboration

**Caching:**
- None

## Authentication & Identity

**Auth Provider:**
- None -- the CLI and HTTP server have no user authentication
- The only credential in the system is `ANTHROPIC_API_KEY` for the AI ambiguity testing subcommand (optional, feature degrades gracefully when absent)
- The HTTP API (`tenor serve`) has no authentication -- intended for local development use

## Monitoring & Observability

**Error Tracking:**
- None -- no error reporting service integrated

**Logs:**
- `stderr` only -- errors and diagnostic messages go to stderr; structured output goes to stdout
- No log framework (no `tracing`, `log`, `env_logger`)
- TAP v14 format output for conformance and ambiguity test runs (implemented in `crates/cli/src/tap.rs`)
- HTTP server logs: startup message, loaded contract names, and shutdown message to stderr

## CI/CD & Deployment

**Hosting:**
- Docker image for evaluator service (see HTTP API Server section)
- CLI binary: standalone, no hosting required

**CI Pipeline:**
- GitHub Actions -- `.github/workflows/ci.yml`
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
- None for normal operation (`elaborate`, `validate`, `eval`, `test`, `diff`, `check`, `explain`, `serve`, `lsp`, `generate`)

**Optional env vars:**
- `ANTHROPIC_API_KEY` -- enables `tenor ambiguity` subcommand; the command skips gracefully with an informational message if absent

**Secrets location:**
- No secrets committed to repo; `ANTHROPIC_API_KEY` is a runtime environment variable only

## Webhooks & Callbacks

**Incoming:**
- HTTP API endpoints on `tenor serve` (see HTTP API Server section)

**Outgoing:**
- None (the Anthropic API call from `tenor ambiguity` is a user-initiated CLI invocation, not a webhook)

## JSON Schema Registry

**Schema $id URIs** (used for `$ref` resolution, not live network calls):
- `https://tenor-lang.org/schemas/interchange/v1.0.0` -- interchange bundle schema in `docs/interchange-schema.json`
- `https://tenor-lang.org/schemas/manifest/v1.1.0` -- manifest envelope schema in `docs/manifest-schema.json`

These URIs are registered in-process at validation time via `jsonschema::options().with_resource(...)`. No live network requests are made to `tenor-lang.org` at runtime.

---

*Integration audit: 2026-02-23*
