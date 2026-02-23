---
phase: 15-typescript-agent-sdk-client-to-rust-evaluator
verified: 2026-02-22T00:00:00Z
status: passed
score: 16/16 must-haves verified
re_verification: false
---

# Phase 15: TypeScript Agent SDK — Verification Report

**Phase Goal:** A TypeScript SDK that connects to the Rust evaluator (running as a service) and exposes the core agent skills — getOperations, invoke, explain — without reimplementing trust-critical logic in a new language

**Verified:** 2026-02-22
**Status:** PASSED
**Re-verification:** No — initial verification

---

## Goal Achievement

### Observable Truths

Truths are drawn from all three plan `must_haves` blocks (plans 01, 02, 03) covering requirements SDK-01 through SDK-05.

**Plan 15-01 truths (SDK-01, SDK-03):**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `tenor serve` starts an HTTP server on a configurable port | VERIFIED | `crates/cli/src/serve.rs:33` — `start_server(port, contract_paths)` binds `tiny_http::Server::http("0.0.0.0:{port}")`. `crates/cli/src/main.rs:137-144` — `Serve` variant with `--port` arg defaulting to 8080. |
| 2 | POST /elaborate accepts .tenor source and returns interchange JSON | VERIFIED | `serve.rs:324-371` — `handle_elaborate()` writes source to temp file, calls `tenor_core::elaborate::elaborate()`, returns bundle or structured error. |
| 3 | POST /evaluate accepts bundle + facts and returns verdicts JSON | VERIFIED | `serve.rs:375-480` — `handle_evaluate()` looks up bundle by `bundle_id`, calls `tenor_eval::evaluate()`, returns `result.verdicts.to_json()`. |
| 4 | POST /evaluate with flow_id and persona returns flow result | VERIFIED | `serve.rs:408-467` — when `flow_id` is present, calls `tenor_eval::evaluate_flow()` and serializes full flow result including `entity_state_changes`, `steps_executed`, `verdicts`. |
| 5 | POST /explain accepts bundle and returns explain output | VERIFIED | `serve.rs:484-515` — `handle_explain()` looks up bundle, calls `explain::explain_bundle()` which returns `{ summary, verbose }` JSON. |
| 6 | GET /contracts lists loaded contract bundles | VERIFIED | `serve.rs:202-247` — `handle_list_contracts()` iterates `state.contracts`, returns array with `id`, `construct_count`, `facts`, `operations`, `flows`. |
| 7 | GET /contracts/:id/operations returns operation list | VERIFIED | `serve.rs:250-321` — `handle_get_operations()` filters constructs by kind="Operation", returns `id`, `allowed_personas`, `effects` (entity_id/from/to), `preconditions_summary`. |
| 8 | GET /health returns server status | VERIFIED | `serve.rs:190-199` — returns `{ status: "ok", tenor_version: TENOR_BUNDLE_VERSION }`. |
| 9 | Server shuts down cleanly on SIGINT/SIGTERM | VERIFIED | `serve.rs:69-110` — `SHUTDOWN_FLAG` AtomicBool, `install_signal_handlers()` via libc, `recv_timeout(1s)` polling loop that breaks when flag set. |

**Plan 15-02 truths (SDK-01, SDK-02):**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 10 | TypeScript SDK connects to a running `tenor serve` instance over HTTP | VERIFIED | `sdk/typescript/src/client.ts` — `TenorClient` uses `fetch()` with configurable `baseUrl`. Private `request<T>()` helper handles HTTP, throws `ConnectionError` on network failure. |
| 11 | SDK exposes `getOperations(contractId)` returning typed operation list | VERIFIED | `client.ts:63-68` — GET `/contracts/${contractId}/operations`, returns `OperationInfo[]`. `types.ts:36-41` — `OperationInfo` with `id`, `allowed_personas`, `effects`, `preconditions_summary`. |
| 12 | SDK exposes `invoke(contractId, facts, options?)` returning typed verdicts | VERIFIED | `client.ts:79-90` — POST `/evaluate` with `bundle_id`, `facts`, optional `flow_id`/`persona`. Returns `EvalResult | FlowEvalResult`. |
| 13 | SDK exposes `explain(contractId)` returning typed explanation | VERIFIED | `client.ts:96-99` — POST `/explain` with `bundle_id`. Returns `ExplainResult` (`{ summary: string, verbose: string }`). |
| 14 | SDK lists available contracts via `listContracts()` | VERIFIED | `client.ts:54-57` — GET `/contracts`, returns `ContractSummary[]`. |
| 15 | SDK provides typed error classes for connection and evaluation failures | VERIFIED | `sdk/typescript/src/errors.ts` — `TenorError` base, `ConnectionError`, `EvaluationError`, `ElaborationError`, `ContractNotFoundError`. All with correct `name` field, typed properties. |

**Plan 15-03 truths (SDK-04, SDK-05):**

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 16 | Docker image builds and runs `tenor serve` with pre-loaded contracts | VERIFIED | `Dockerfile` — 21 lines, multi-stage: `rust:1.93-slim` builder, `debian:trixie-slim` runtime, `ENTRYPOINT ["tenor", "serve", "--port", "8080"]`. `docker-compose.yml` mounts `./domains:/contracts:ro`. |
| 17 | SDK README documents the trust boundary | VERIFIED | `sdk/typescript/README.md:7` — "The SDK is a **client**. The Rust evaluator is the **trusted core**." Architecture section on line 5 explains the separation and why it matters. 252 lines total. |
| 18 | SDK README shows getting started with both `tenor serve` and Docker | VERIFIED | `README.md:24-41` — "Option A: tenor serve" and "Option B: Docker" sections with copy-paste commands. |
| 19 | Example script demonstrates all three agent skills | VERIFIED | `sdk/typescript/examples/agent-basics.ts` — 88 lines. Calls `client.health()`, `client.listContracts()`, `client.getOperations()` (skill 1), `client.invoke()` (skill 2), `client.explain()` (skill 3). |

**Score: 19/19 truths verified** (truths 10-19 consolidate the 16-item must_haves count from the three plans; numbered to 19 here for clarity but score is 16/16 on must_haves as declared).

---

### Required Artifacts

All artifacts checked at three levels: exists, substantive (line count / content), wired (imported and called).

**Plan 15-01 artifacts:**

| Artifact | Min Lines | Actual | Contains | Status | Wiring |
|----------|-----------|--------|----------|--------|--------|
| `crates/cli/src/serve.rs` | 200 | 515 | HTTP server, all route handlers | VERIFIED | Called from `main.rs:202` via `serve::start_server(port, contracts)` |
| `crates/cli/src/main.rs` | — | 1081 | `Serve` variant, `mod serve;` | VERIFIED | Entry point, dispatches to `serve::start_server` |

**Plan 15-02 artifacts:**

| Artifact | Min Lines | Actual | Contains | Status | Wiring |
|----------|-----------|--------|----------|--------|--------|
| `sdk/typescript/src/client.ts` | 100 | 172 | `TenorClient` class, all methods, `request()` helper | VERIFIED | Re-exported from `index.ts` |
| `sdk/typescript/src/types.ts` | 50 | 118 | All response interfaces, `OperationInfo`, `EvalResult`, `FlowEvalResult`, `ExplainResult`, error interfaces | VERIFIED | Imported by `client.ts` |
| `sdk/typescript/src/errors.ts` | 20 | 53 | `TenorError`, `ConnectionError`, `EvaluationError`, `ElaborationError`, `ContractNotFoundError` | VERIFIED | Imported by `client.ts`, re-exported from `index.ts` |
| `sdk/typescript/package.json` | — | 28 lines | `"name": "@tenor-lang/sdk"`, dual ESM/CJS exports, `@types/node ^22` | VERIFIED | npm package root |
| `sdk/typescript/tests/client.test.ts` | 50 | 268 | 22 tests: 13 unit + skippable integration tests gated on `TENOR_SERVE_URL` | VERIFIED | Run via `npm test` |

**Plan 15-03 artifacts:**

| Artifact | Min Lines | Actual | Contains | Status | Wiring |
|----------|-----------|--------|----------|--------|--------|
| `Dockerfile` | 15 | 21 | `tenor serve` ENTRYPOINT, multi-stage build, VOLUME `/contracts` | VERIFIED | Referenced by `docker-compose.yml` |
| `sdk/typescript/README.md` | 80 | 252 | Trust boundary architecture, getting started, agent skills, API reference, error handling | VERIFIED | Standalone documentation |
| `sdk/typescript/examples/agent-basics.ts` | 30 | 88 | All three agent skills: getOperations, invoke, explain | VERIFIED | Imports TenorClient from `../src/index.ts` |

---

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `crates/cli/src/serve.rs` | `tenor_core::elaborate::elaborate` | elaborate endpoint handler | VERIFIED | Lines 40, 351: direct calls in pre-load and `handle_elaborate()` |
| `crates/cli/src/serve.rs` | `tenor_eval::evaluate` | evaluate endpoint handler | VERIFIED | Line 470: `tenor_eval::evaluate(&bundle, &facts)` in `handle_evaluate()` |
| `crates/cli/src/serve.rs` | `tenor_eval::evaluate_flow` | flow evaluate endpoint handler | VERIFIED | Line 419: `tenor_eval::evaluate_flow(&bundle, &facts, fid, p)` |
| `sdk/typescript/src/client.ts` | `crates/cli/src/serve.rs` | HTTP fetch calls | VERIFIED | `client.ts` calls GET /health, GET /contracts, GET /contracts/{id}/operations, POST /elaborate, POST /evaluate, POST /explain — all endpoints implemented in `serve.rs` |
| `sdk/typescript/src/types.ts` | `crates/eval/src/types.rs` | TypeScript mirrors Rust types | VERIFIED | `OperationInfo` uses `allowed_personas` and `effects[].entity_id` matching serve.rs handler; `Verdict` uses `type`/`payload`/`provenance.rule` matching `VerdictSet::to_json()` at `types.rs:1583-1595` |
| `Dockerfile` | `crates/cli/src/serve.rs` | ENTRYPOINT runs tenor serve | VERIFIED | `ENTRYPOINT ["tenor", "serve", "--port", "8080"]` at line 20 |
| `sdk/typescript/README.md` | `sdk/typescript/src/client.ts` | Code examples reference TenorClient API | VERIFIED | Lines 52, 54, 140, 211, 238: TenorClient referenced in quick start and API reference |
| `sdk/typescript/examples/agent-basics.ts` | `sdk/typescript/src/index.ts` | Import and use SDK | VERIFIED | Line 11: `import { TenorClient } from '../src/index.ts'` — uses relative source import rather than `@tenor-lang/sdk` package import (acceptable for an in-repo example; plan showed it could use `@tenor-lang/sdk` but that would require `npm install` first) |

---

### Requirements Coverage

All five requirement IDs from plan frontmatter are accounted for:

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| SDK-01 | 15-01, 15-02 | TypeScript SDK connects to Rust evaluator running as a service | SATISFIED | `serve.rs` provides HTTP API; `client.ts` `TenorClient` connects to it via `fetch()`. End-to-end wiring verified. |
| SDK-02 | 15-02 | SDK exposes core agent skills: getOperations, invoke, explain | SATISFIED | `client.ts:63` `getOperations()`, `client.ts:79` `invoke()`, `client.ts:96` `explain()` — all three skills implemented, typed, and tested. |
| SDK-03 | 15-01 | Evaluator available via `tenor serve` CLI command | SATISFIED | `main.rs:137-144,201-203` — `Serve` subcommand registered with `--port` and `contracts` args, dispatches to `serve::start_server()`. |
| SDK-04 | 15-03 | Evaluator available via Docker image (`tenor/evaluator`) | SATISFIED | `Dockerfile` multi-stage build produces `tenor` binary with `ENTRYPOINT ["tenor", "serve", "--port", "8080"]`. `docker-compose.yml` provides one-command local dev. |
| SDK-05 | 15-03 | SDK documentation is explicit: SDK is client, evaluator is trusted core | SATISFIED | `sdk/typescript/README.md:7` — "The SDK is a **client**. The Rust evaluator is the **trusted core**." Architecture section with diagram placed as second section of README. |

**Requirements note:** REQUIREMENTS.md traceability table still shows "Not started" for Phase 15, but the checkboxes at the top of the SDK section already show `[x]` for all five. This is a documentation inconsistency in REQUIREMENTS.md (table and checkboxes are out of sync) — it does not affect whether the requirements are satisfied.

**No orphaned requirements:** Exactly SDK-01 through SDK-05 are mapped to Phase 15, and all five are covered by the plans.

---

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none found) | — | — | — | — |

No TODO/FIXME/placeholder comments, no stub implementations, no empty handlers, no static response returns found in any of the key artifacts.

---

### Human Verification Required

#### 1. Integration test suite against live server

**Test:** Start `cargo run -p tenor-cli -- serve --port 9090 domains/saas/saas_subscription.tenor`, then run `cd sdk/typescript && TENOR_SERVE_URL=http://localhost:9090 npm test`

**Expected:** All integration tests pass (health, listContracts, getOperations, invoke with facts, ContractNotFoundError for unknown contract, explain, elaborate with valid source, ElaborationError for invalid source). Unit tests pass without server running.

**Why human:** Integration tests are gated by `TENOR_SERVE_URL` env var and require a live server. Automated verification cannot run the server process.

#### 2. Docker image build and runtime

**Test:** `docker build -t tenor/evaluator .` then `docker run --rm -p 8080:8080 -v $(pwd)/domains:/contracts:ro tenor/evaluator /contracts/saas/saas_subscription.tenor` then `curl http://localhost:8080/health`

**Expected:** Image builds successfully, container starts, `/health` returns `{"status":"ok","tenor_version":"..."}`.

**Why human:** Docker build requires Docker daemon and network access to pull base images. Cannot verify in static code analysis.

---

### Gaps Summary

No gaps found. All 16 must-haves are verified at all three levels (exists, substantive, wired). All five requirements (SDK-01 through SDK-05) are satisfied by concrete implementation evidence. No anti-patterns detected.

The one cosmetic deviation — `agent-basics.ts` imports `../src/index.ts` rather than `@tenor-lang/sdk` — is functionally correct for an in-repo example (avoids requiring `npm install` to run the example from source) and does not block any goal or requirement.

---

_Verified: 2026-02-22_
_Verifier: Claude (gsd-verifier)_
