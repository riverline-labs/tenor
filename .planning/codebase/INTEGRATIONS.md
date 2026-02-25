# External Integrations

**Analysis Date:** 2026-02-25

## APIs & External Services

**Anthropic Claude (Optional):**
- Claude Sonnet 4.5 - Ambiguity testing for contract DSL
  - SDK/Client: `ureq` 3.0 (synchronous HTTP)
  - Endpoint: `https://api.anthropic.com/v1/messages`
  - Auth: `ANTHROPIC_API_KEY` environment variable
  - Version header: `anthropic-version: 2023-06-01`
  - Usage: `cargo run -p tenor-cli -- test ambiguity` (optional, gracefully skips if key not set)
  - Retry logic: Exponential backoff on 429, 500, 503; default 3 retries, 1000ms initial backoff
  - Model parameter: Configurable via `--model` flag, defaults to `claude-sonnet-4-5`

## Data Storage

**Databases:**
- Not applicable - Tenor is a stateless elaborator and evaluator

**File Storage:**
- Local filesystem only
  - Elaboration: Reads `.tenor` source files from disk
  - Contracts: Mounted at `/contracts` in Docker container
  - Conformance suite: Uses `tempfile` for temporary test fixtures

**Caching:**
- In-memory only:
  - Per-IP rate limit tracker (HashMap<IpAddr, (count, Instant)>) in `RateLimiter`
  - WASM contract storage: Thread-local `Slab<StoredContract>` in `crates/tenor-eval-wasm/src/lib.rs`
  - No persistent cache

## Authentication & Identity

**Auth Provider:**
- Custom API key scheme
  - Implementation: Optional header-based authentication via `TENOR_API_KEY`
  - All endpoints except `/health` require key if `TENOR_API_KEY` is set
  - Header format: `Authorization: Bearer <key>` (verified in `serve.rs`)
  - No external OAuth, JWT, or identity provider

## Monitoring & Observability

**Error Tracking:**
- Not detected - Errors are returned in JSON responses, no external error tracking

**Logs:**
- stdout/stderr only
  - `eprintln!()` for diagnostics (ambiguity test results, API errors)
  - Server logs request/response via CORS and error middleware
  - No external logging service

## CI/CD & Deployment

**Hosting:**
- Docker (containerized deployment)
- Local filesystem or manual binary execution

**CI Pipeline:**
- GitHub Actions (`.github/workflows/ci.yml`)
  - Triggers: Push to `main` branch, pull requests to `main`
  - Stages:
    1. Checkout code (`actions/checkout@v4`)
    2. Setup Rust toolchain (`dtolnay/rust-toolchain@stable`)
    3. Cache Rust build artifacts (`Swatinem/rust-cache@v2`)
    4. `cargo build --workspace`
    5. `cargo run -p tenor-cli -- test conformance` (72/72 tests)
    6. `cargo test --workspace` (schema validation + unit tests)
    7. `cargo fmt --all -- --check` (formatting enforcement)
    8. `cargo clippy --workspace -- -D warnings` (linting with error on warnings)
    9. `wasm-pack build --target nodejs` (WASM compilation)
    10. `wasm-pack test --node` (WASM unit tests)

## Environment Configuration

**Required env vars:**
- None - Tenor runs with defaults

**Optional env vars:**
- `TENOR_RATE_LIMIT` - Requests per minute per IP (defaults to 60)
- `TENOR_API_KEY` - API key for endpoint authentication (defaults to no auth)
- `ANTHROPIC_API_KEY` - Anthropic API key for ambiguity testing (defaults to skipped tests)

**Secrets location:**
- Environment variables only; no .env files committed to repo
- `.env` and `.env.*` patterns in `.gitignore`

## Webhooks & Callbacks

**Incoming:**
- None - Tenor is request/response only

**Outgoing:**
- None - No callbacks or webhooks

## HTTP Server

**Endpoints:**
- `GET /health` - Health check (exempt from auth and rate limiting)
- `GET /contracts` - List loaded contract bundles
- `GET /contracts/{id}/operations` - Operations for a specific contract
- `GET /.well-known/tenor` - Contract manifest with ETag (Tenor spec §19)
- `GET /inspect` - Structured contract summary
- `POST /elaborate` - Elaborate `.tenor` source text to interchange JSON
- `POST /evaluate` - Evaluate a contract against fact set
- `POST /explain` - Explain a contract bundle (verbose analysis)
- `POST /flows/{flow_id}/simulate` - Stateless flow simulation
- `POST /actions` - Action space for a persona

**Security:**
- Per-IP rate limiting (default 60 req/min, configurable via `TENOR_RATE_LIMIT`)
- Optional API key authentication (via `TENOR_API_KEY`)
- CORS headers on all responses (permissive for local dev via `tower-http::cors`)
- Input validation on elaborate endpoint: max 1 MB source, max 10 MB body, filename/import checks
- TLS support optional (feature-gated `tls` feature via `axum-server` with rustls)

**Response Format:**
- All responses: `Content-Type: application/json`
- Error responses: JSON error objects with human-readable messages
- Status codes: 200 OK, 400 Bad Request, 401 Unauthorized, 429 Too Many Requests, 500 Internal Server Error

---

*Integration audit: 2026-02-25*
