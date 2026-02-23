# Phase 15: TypeScript Agent SDK (Client to Rust Evaluator) - Context

**Gathered:** 2026-02-23
**Status:** Ready for planning

<domain>
## Phase Boundary

A TypeScript SDK that connects to the Rust evaluator (running as a service) and exposes the core agent skills — getOperations, invoke, explain — without reimplementing trust-critical logic. The SDK is a client. The evaluator is the trusted core.

</domain>

<decisions>
## Implementation Decisions

### Claude's Discretion

The user delegated all implementation decisions. Claude has full flexibility on:

**SDK surface & API shape:**
- Method signatures for getOperations, invoke, explain
- Contract loading approach (path-based, bundle-based, or both)
- Sync vs async API design
- Class-based vs functional style
- How to handle multi-contract System scenarios

**Evaluator connectivity:**
- Transport protocol (HTTP, gRPC, subprocess stdio)
- Connection lifecycle management
- `tenor serve` vs Docker vs hosted evaluator approach
- Health checks, reconnection, timeout behavior

**Error handling & types:**
- TypeScript type generation strategy from interchange format
- How evaluation errors are surfaced to SDK consumers
- Provenance data representation in TypeScript
- Verdict and outcome typing

**Package & distribution:**
- npm package naming and scope
- ESM/CJS dual-publish strategy
- Node.js version targets
- Dependency footprint and bundling approach
- Whether to support browser environments (probably not for v1 — evaluator is a server)

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches. The ROADMAP.md notes three getting-started options:
- Option A: `docker run tenor/evaluator` for local development
- Option B: `tenor serve` CLI command that starts a local evaluator process
- Option C: Hosted evaluator (future) for teams that don't want to manage infrastructure

The trust boundary constraint from PROJECT.md is the key design driver: the Rust evaluator is the trusted core, the TypeScript SDK is a client that does not reimplement evaluation logic.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 15-typescript-agent-sdk-client-to-rust-evaluator*
*Context gathered: 2026-02-23*
