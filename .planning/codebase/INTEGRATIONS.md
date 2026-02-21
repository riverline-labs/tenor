# External Integrations

**Analysis Date:** 2026-02-21

## APIs & External Services

**None.**

This codebase is a standalone DSL elaborator. It has no external service integrations.

## Data Storage

**Databases:**
- None - Not used

**File Storage:**
- Local filesystem only
- Input: Reads `.tenor` DSL source files from the filesystem
- Output: Writes JSON interchange format to stdout, errors to stderr

**Caching:**
- None - Not implemented

## Authentication & Identity

**None.**

The elaborator is a stateless, single-invocation CLI tool with no authentication requirements.

## Monitoring & Observability

**Error Tracking:**
- None - Errors are formatted as structured JSON (matching `ElabError` schema in `elaborator/src/error.rs`) and written to stderr

**Logs:**
- Custom error output via `eprintln!()` in `elaborator/src/main.rs`
- TAP (Test Anything Protocol) output for test suite runs via `elaborator/src/tap.rs`
- No structured logging framework

## CI/CD & Deployment

**Hosting:**
- None required - Compiled binary runs locally
- No deployment platform integration

**CI Pipeline:**
- Not present in this repository

## Environment Configuration

**Required environment variables:**
- None

**Required configuration files:**
- None

**Secrets:**
- None required

## Webhooks & Callbacks

**Incoming:**
- None

**Outgoing:**
- None

## Interchange Format

The only "external" concern is the output format:

**JSON Interchange (Tenor Interchange Format):**
- Produced by `elaborate()` in `elaborator/src/elaborate.rs`
- Serialized via `serde_json` in Pass 6 (serialization)
- Used by: Test fixtures (`.expected.json` files for conformance testing)
- Schema: Structured JSON with:
  - Constructs (fact, entity, rule, operation, flow, type declarations)
  - Typed expressions with precedence and type information
  - Numeric values with precision metadata
  - Sorted keys (lexicographic ordering within objects)
  - See `docs/TENOR.md` for full interchange specification (v0.3)

---

*Integration audit: 2026-02-21*
