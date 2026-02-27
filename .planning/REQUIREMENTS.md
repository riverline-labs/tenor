# Requirements: Tenor

**Defined:** 2026-02-27
**Core Value:** Deterministic contract evaluation

## Cross-Cutting Requirements

- [x] **QLT-01**: All conformance tests pass (82+)
- [x] **QLT-02**: All workspace tests pass (660+)
- [x] **QLT-03**: cargo clippy --workspace clean (-D warnings)
- **BWC-01**: Existing contracts work unchanged through all phases
- **SPC-01**: All changes match TENOR.md spec sections

## Phase 4: Multi-Instance Entities

### Entity State Model
- [x] **ESM-01**: EntityStateMap keyed by (entity_id, instance_id) composite key
- [x] **ESM-02**: single_instance() helper converts entity_id→state maps to instance-keyed form with `_default` ID
- [x] **ESM-03**: All existing eval crate call sites compile and pass with new EntityStateMap type

### Operation Execution
- [x] **OPX-01**: execute_operation() takes (entity_id, instance_id) for effect targets
- [x] **OPX-02**: E2 transition source validation checks targeted instance's current state

### Flow Execution
- [x] **FLX-01**: InstanceBindingMap type maps entity_id → instance_id
- [x] **FLX-02**: execute_flow() takes InstanceBindingMap parameter
- [x] **FLX-03**: resolve_bindings() maps operation effect targets to specific instances from bindings
- [x] **FLX-04**: Sub-flows inherit parent's instance bindings
- [x] **FLX-05**: Missing binding for entity referenced in effects = execution error

### Action Space
- [ ] **ACT-01**: compute_action_space() returns per-instance results
- [ ] **ACT-02**: Action struct includes instance_bindings (entity_id → set of valid instance_ids)
- [ ] **ACT-03**: BlockedAction includes per-instance blocking info

### Provenance
- [ ] **PRV-01**: OperationProvenance records instance_binding map
- [ ] **PRV-02**: OperationProvenance has per-instance state_before/state_after
- [ ] **PRV-03**: FlowProvenance carries instance bindings through step records

### Testing
- [x] **TST-01**: All existing tests updated to use single_instance() helper
- [ ] **TST-02**: Multi-instance action space test
- [ ] **TST-03**: Instance-targeted execution test
- [ ] **TST-04**: Flow with instance bindings test
- [ ] **TST-05**: Missing instance binding error test
- [ ] **TST-06**: Single-instance degenerate case backward compat test
- [ ] **TST-07**: Instance absence test

### WASM
- [ ] **WSM-01**: WASM API accepts new nested entity_states format
- [ ] **WSM-02**: WASM API accepts old flat format with `_default` fallback
- [ ] **WSM-03**: WASM action space output includes instance bindings
- [ ] **WSM-04**: WASM flow execution accepts instance bindings

## Phase 5: Trust & Security

- [ ] **TRS-01**: `tenor sign` / `tenor verify` for interchange bundles (Ed25519)
- [ ] **TRS-02**: `tenor sign-wasm` / `tenor verify-wasm` for WASM binaries
- [ ] **TRS-03**: TrustMetadata struct in interchange crate (optional fields)
- [ ] **TRS-04**: Provenance records carry optional trust_domain and attestation
- [ ] **TRS-05**: Executor conformance suite (E1-E20 test macro)
- [ ] **TRS-06**: All trust metadata optional — zero-config unchanged
- [ ] **TRS-07**: Private repo: TrustConfig, manifest trust section, provenance attestation

## Phase 6: Advanced Policies

- [ ] **POL-01**: HumanInTheLoopPolicy with ApprovalChannel trait
- [ ] **POL-02**: LlmPolicy with LlmClient trait and retry logic
- [ ] **POL-03**: CompositePolicy with ApprovalPredicate trait
- [ ] **POL-04**: All implement AgentPolicy trait
- [ ] **POL-05**: Edge cases: empty action space, timeout, invalid LLM response

## Phase 7: SDKs

- [ ] **SDK-01**: TypeScript npm package wrapping WASM evaluator
- [ ] **SDK-02**: Python PyPI package via PyO3 FFI
- [ ] **SDK-03**: Go module via CGo FFI
- [ ] **SDK-04**: Each SDK: evaluate, action space, execute flow, read interchange
- [ ] **SDK-05**: Cross-SDK conformance test proving identical results

## Phase 8: Automatic UI

- [ ] **AUI-01**: `tenor ui <contract>` CLI command
- [ ] **AUI-02**: Generated React SPA from interchange bundle
- [ ] **AUI-03**: Entity states, actions, blocked actions, fact inputs, flows, provenance
- [ ] **AUI-04**: Professional theming (not a wireframe)
- [ ] **AUI-05**: Multi-instance entity browsing

## Phase 9: Builder

- [ ] **BLD-01**: Visual editors for all construct types
- [ ] **BLD-02**: Real-time elaboration via WASM
- [ ] **BLD-03**: State machine and flow DAG visualization
- [ ] **BLD-04**: Simulation with test facts
- [ ] **BLD-05**: Import/export .tenor files and interchange bundles

## Phase 10: Hosted Platform

- [ ] **PLT-01**: Multi-tenant isolation (contracts, entities, provenance)
- [ ] **PLT-02**: API key auth, org/user management
- [ ] **PLT-03**: Contract provisioning from interchange bundle
- [ ] **PLT-04**: Rate limiting and API gateway
- [ ] **PLT-05**: Usage metering and billing tiers
- [ ] **PLT-06**: Admin dashboard

## Phase 11: Marketplace

- [ ] **MKT-01**: Template format (tenor-template.toml) with metadata
- [ ] **MKT-02**: `tenor publish` / `tenor pack` CLI commands
- [ ] **MKT-03**: Searchable catalog with categories, tags, ratings
- [ ] **MKT-04**: One-click deploy to hosted platform
- [ ] **MKT-05**: Community contributions with review workflow

## Out of Scope

| Feature | Reason |
|---------|--------|
| Dynamic entity type creation at runtime | AL74: multiplicity is runtime-only |
| Cross-instance preconditions in rules | AL75: not modeled |
| Instance count constraints | Instances are purely runtime, not declared |

## Traceability (Phase 4 — Active)

| Requirement | Plan | Status |
|-------------|------|--------|
| ESM-01 | 04-01 | Complete |
| ESM-02 | 04-01 | Complete |
| ESM-03 | 04-01 | Complete |
| OPX-01 | 04-02 | Complete |
| OPX-02 | 04-02 | Complete |
| FLX-01 | 04-02 | Complete |
| FLX-02 | 04-02 | Complete |
| FLX-03 | 04-02 | Complete |
| FLX-04 | 04-02 | Complete |
| FLX-05 | 04-02 | Complete |
| ACT-01 | 04-03 | Pending |
| ACT-02 | 04-03 | Pending |
| ACT-03 | 04-03 | Pending |
| PRV-01 | 04-03 | Pending |
| PRV-02 | 04-03 | Pending |
| PRV-03 | 04-03 | Pending |
| TST-01 | 04-01 | Complete |
| TST-02 | 04-04 | Pending |
| TST-03 | 04-04 | Pending |
| TST-04 | 04-04 | Pending |
| TST-05 | 04-04 | Pending |
| TST-06 | 04-04 | Pending |
| TST-07 | 04-04 | Pending |
| WSM-01 | 04-05 | Pending |
| WSM-02 | 04-05 | Pending |
| WSM-03 | 04-05 | Pending |
| WSM-04 | 04-05 | Pending |

---
*Requirements defined: 2026-02-27*
