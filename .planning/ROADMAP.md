# Roadmap: Tenor

## Overview

Tenor's remaining roadmap spans 8 phases: completing the core runtime (multi-instance entities, trust, policies), building developer experience (SDKs, auto UI, builder), and launching the commercial platform (hosted SaaS, marketplace).

## Completed

- **Phase 1: Migration** — shipped 2026-02-26
- **Phase 2: Source Declarations** — shipped 2026-02-27
- **Phase 3: Automated Fact Wiring** — shipped 2026-02-27

## Phases

### Phase 4: Multi-Instance Entities

**Goal**: Evaluator natively supports multiple runtime instances per entity type with independent state machines
**Repo**: Public + Private
**Plan**: `docs/plans/plan-4.md`
**Spec**: TENOR.md §6.5, §9, §11, §15, §16, §17, §20, §21, §22
**Depends on**: Phase 3
**Success Criteria**:

1. EntityStateMap keyed by (entity_id, instance_id)
2. Single-instance contracts work unchanged via `_default` instance ID
3. Operations target specific instances
4. Flows accept InstanceBindingMap
5. Action space reports per-instance availability
6. Provenance is instance-scoped
7. WASM API accepts both old flat and new nested format
8. All existing tests pass (660+), conformance (82+), clippy clean
   **Plans**: 5 (04-01 through 04-05)

- [x] 04-01: EntityStateMap type migration and existing test updates
- [x] 04-02: Instance-targeted operation and flow execution
- [x] 04-03: Per-instance action space and instance-scoped provenance
- [x] 04-04: New multi-instance tests
- [x] 04-05: WASM evaluator instance support

### Phase 5: Trust & Security

**Goal**: Contract signing, WASM bundle signing, provenance attestation, executor conformance suite
**Repo**: Public + Private
**Plan**: `docs/plans/plan-5.md`
**Spec**: TENOR.md §17.4 (E18-E20), §18, §19.1, §20 (AL80-AL84)
**Depends on**: Phase 4
**Success Criteria**:

1. `tenor sign` / `tenor verify` for interchange bundles
2. WASM binary signing and verification
3. Provenance records carry optional trust_domain and attestation
4. Manifest serves optional `trust` section
5. Executor conformance suite validates E1-E20
6. All trust metadata is optional — zero-config deployments unchanged
   **Plans**: 6 (05-01 through 05-06)

- [x] 05-01: Trust types in interchange crate
- [x] 05-02: Key generation + bundle signing/verification CLI
- [x] 05-03: WASM signing
- [x] 05-04: Executor conformance suite
- [x] 05-05: Trust feature tests
- [x] 05-06: Private repo trust integration

### Phase 6: Advanced Policies

**Goal**: HumanInTheLoopPolicy, LlmPolicy, CompositePolicy for agent runtime
**Repo**: Public only
**Plan**: `docs/plans/plan-6.md`
**Spec**: TENOR.md §15.6, AGENT_ORIENTATION.md
**Depends on**: Phase 4 (multi-instance action space)
**Success Criteria**:

1. HumanInTheLoopPolicy pauses for human approval
2. LlmPolicy serializes action space to LLM prompt, parses chosen action
3. CompositePolicy chains policies with configurable thresholds
4. All implement AgentPolicy trait, work with existing runtime
5. Edge cases handled: empty action space, timeout, invalid LLM response
   **Plans**: 4 (06-01 through 06-04)

- [x] 06-01: HumanInTheLoopPolicy
- [x] 06-02: LlmPolicy
- [x] 06-03: CompositePolicy
- [x] 06-04: Documentation and examples

### Phase 7: SDKs

**Goal**: Idiomatic TypeScript (Node.js), Python, and Go SDKs wrapping the evaluator
**Repo**: Public only
**Plan**: `docs/plans/plan-7.md`
**Depends on**: Phase 4 (instance-aware API surface)
**Success Criteria**:

1. TypeScript npm package wrapping WASM evaluator
2. Python PyPI package via PyO3 FFI
3. Go module via CGo FFI
4. Each SDK: evaluate, action space, execute flow, read interchange
5. Each SDK: test suite proving identical results to Rust evaluator
   **Plans**: 4 (07-01 through 07-04)

- [x] 07-01: TypeScript SDK (WASM wrapper)
- [x] 07-02: Python SDK (PyO3)
- [x] 07-03: Go SDK (wazero + WASI bridge)
- [x] 07-04: Cross-SDK conformance tests

### Phase 8: Automatic UI

**Goal**: `tenor ui <contract>` generates a complete themed React application from any contract
**Repo**: Public only
**Plan**: `docs/plans/plan-8.md`
**Depends on**: Phase 7 (TypeScript SDK), Phase 4 (multi-instance)
**Success Criteria**:

1. Generated React SPA from interchange bundle alone
2. Entity states, actions per persona, blocked actions, fact inputs, flow execution, provenance
3. Themed and professional — not a dev wireframe
4. Works against any executor's HTTP API (§19 endpoints)
5. Multi-instance entity browsing
   **Plans**: 4 (08-01 through 08-04)

- [ ] 08-01: UI architecture + CLI command + API client
- [ ] 08-02: Contract-driven UI generation
- [ ] 08-03: Theming
- [ ] 08-04: Tests

### Phase 9: Builder

**Goal**: Visual contract editor web application — author, visualize, simulate, export contracts
**Repo**: Public only
**Plan**: `docs/plans/plan-9.md`
**Depends on**: Phase 8 (UI components), Phase 7 (TypeScript SDK)
**Success Criteria**:

1. Visual editors for entities, facts, rules, operations, flows, personas, sources
2. Real-time elaboration via WASM
3. State machine and flow DAG visualization
4. Simulation with test facts
5. Import/export .tenor files and interchange bundles
   **Plans**: 7 (09-01 through 09-07)

- [ ] 09-01: Architecture + WASM integration
- [ ] 09-02: Entity and fact editors
- [ ] 09-03: Rule and operation editors
- [ ] 09-04: Flow editor
- [ ] 09-05: Simulation mode
- [ ] 09-06: Import/export + CLI command
- [ ] 09-07: Tests

### Phase 10: Hosted Platform

**Goal**: Multi-tenant hosted Tenor SaaS with provisioning, auth, API gateway, and billing
**Repo**: Private
**Plan**: `docs/plans/plan-10.md`
**Depends on**: Phase 5 (trust for multi-tenant), Phase 6 (policies for managed agents)
**Success Criteria**:

1. Multi-tenant with isolated contracts, entities, provenance
2. API key auth, org/user management
3. Contract provisioning from interchange bundle
4. Rate limiting, API gateway
5. Usage metering and billing tiers
6. Admin dashboard
   **Plans**: 7 (10-01 through 10-07)

- [x] 10-01: Multi-tenancy
- [x] 10-02: Authentication and authorization
- [x] 10-03: Contract deployment
- [x] 10-04: API gateway
- [ ] 10-05: Billing and metering
- [ ] 10-06: Admin dashboard
- [ ] 10-07: Integration tests

### Phase 11: Marketplace

**Goal**: Searchable contract template registry with publishing, discovery, and one-click deploy
**Repo**: Public + Private
**Plan**: `docs/plans/plan-11.md`
**Depends on**: Phase 10 (hosted platform for deployment target)
**Success Criteria**:

1. Template format (tenor-template.toml) with metadata
2. `tenor publish` CLI command
3. Searchable catalog with categories, tags, ratings
4. One-click deploy to hosted platform
5. Community contributions with review workflow
   **Plans**: 5 (11-01 through 11-05)

- [ ] 11-01: Template format + packaging CLI
- [ ] 11-02: Publishing, search, install CLI
- [ ] 11-03: Registry API + storage
- [ ] 11-04: Review workflow + marketplace UI
- [ ] 11-05: One-click deploy

## Progress

| Phase                      | Status   | Plans    | Completed  |
| -------------------------- | -------- | -------- | ---------- |
| 1. Migration               | Complete | —        | 2026-02-26 |
| 2. Source Declarations     | Complete | —        | 2026-02-27 |
| 3. Automated Fact Wiring   | Complete | —        | 2026-02-27 |
| 4. Multi-Instance Entities | 5/5      | Complete | 2026-02-27 |
| 5. Trust & Security        | 6/6      | Complete | 2026-02-27 |
| 6. Advanced Policies       | 4/4      | Complete | 2026-02-27 |
| 7. SDKs                    | 4/4      | Complete    | 2026-02-27 |
| 8. Automatic UI            | 4/4 | Complete    | 2026-02-27 |
| 9. Builder                 | 7/7 | Complete   | 2026-02-27 |
| 10. Hosted Platform        | 5/7 | In Progress|  |
| 11. Marketplace            | Planned  | 0/5      | —          |

**Total**: 42 plans across 8 remaining phases
