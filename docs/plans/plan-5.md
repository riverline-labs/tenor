# Phase 5: Trust & Security — Complete Implementation

The spec amendment for Trust & Security is in TENOR.md §17.4 (E18, E19, E20), with changes to §17.2 (obligation summary), §18 (trust metadata migration taxonomy), §19.1 (optional `trust` field in manifest), §20 (AL80-AL84), §22 (glossary), and provenance records (optional trust_domain and attestation fields).

The amendment is mechanism-agnostic by design. It defines what must be attestable, not how to attest it. The implementation chooses Ed25519 as the reference signing scheme but the architecture must support swapping it out.

**Repo:** Both. Public repo gets contract signing, WASM bundle signing, and executor conformance suite. Private repo gets trust wiring in the executor, provenance attestation, and manifest trust field.

**Source of truth:** TENOR.md §17.4 (E18-E20), the trust-security amendment document, and the roadmap Phase 8 items.

---

## What "done" means

1. `tenor sign <bundle.json> --key <key-file>` signs an interchange bundle and produces a signed bundle with attestation metadata
2. `tenor verify <signed-bundle.json> --pubkey <pubkey-file>` verifies bundle integrity
3. WASM evaluator binary can be signed and verified (bundle hash embedded, binary hash signed)
4. Executor records trust_domain and attestation on provenance records when trust is configured
5. Manifest serves optional `trust` section with bundle_attestation, trust_domain, attestation_format
6. Executor conformance suite exists (like storage conformance suite) — validates E1-E20 against any executor implementation
7. All trust metadata is optional — deployments without trust infrastructure work unchanged
8. Evaluator is completely unaffected (invariant I1)

---

## Part A: Public Repo — Contract Signing (~/src/riverline/tenor)

### A1: Key generation CLI

Add `tenor keygen` command:

```
tenor keygen [OPTIONS]

OPTIONS:
  --algorithm <ed25519>     Signing algorithm (default: ed25519, only option for v1)
  --output <prefix>         Output file prefix (default: tenor-key)
```

Produces:

- `<prefix>.secret` — Ed25519 secret key (64 bytes, base64 encoded)
- `<prefix>.pub` — Ed25519 public key (32 bytes, base64 encoded)

Use the `ed25519-dalek` crate (or `ring` — whichever is already in the dependency tree or lighter). Do NOT use a custom crypto implementation.

### A2: Bundle signing CLI

Add `tenor sign` command:

```
tenor sign <bundle.json> --key <secret-key-file> [OPTIONS]

OPTIONS:
  --output <file>           Output signed bundle (default: <bundle>.signed.json)
  --format <ed25519-detached>   Attestation format identifier (default: ed25519-detached)
```

The signed bundle is the original bundle JSON with a `trust` section added:

```json
{
  "bundle": { ... },
  "etag": "a1b2c3...",
  "tenor": "1.0",
  "trust": {
    "bundle_attestation": "<base64-encoded Ed25519 signature of canonical bundle bytes>",
    "attestation_format": "ed25519-detached",
    "signer_public_key": "<base64-encoded public key>"
  }
}
```

The attestation signs the canonical bundle bytes (same bytes used for etag computation — §19.2). This means: `sign(SHA-256(canonical_json_bytes(bundle)))` or `sign(canonical_json_bytes(bundle))` directly. Use the etag as the content binding — the signature covers the etag which covers the content.

### A3: Bundle verification CLI

Add `tenor verify` command:

```
tenor verify <signed-bundle.json> [OPTIONS]

OPTIONS:
  --pubkey <pubkey-file>    Public key to verify against (optional if signer_public_key in bundle)
```

Verification:

1. Extract `trust.bundle_attestation` and `trust.attestation_format`
2. If format is not recognized, report "unrecognized attestation format" and exit (per AL81)
3. For `ed25519-detached`: extract public key (from `--pubkey` flag or `trust.signer_public_key`), recompute canonical bundle bytes, verify signature
4. Report: `✓ Bundle verified: etag matches, signature valid, signer: <pubkey-fingerprint>` or `✗ Verification failed: <reason>`

### A4: WASM bundle signing

When the elaborator compiles a contract to WASM (`tenor compile --wasm`), the output WASM binary can also be signed:

```
tenor sign-wasm <evaluator.wasm> --key <secret-key-file> --bundle-etag <etag>
```

This produces a detached signature file (`evaluator.wasm.sig`) that attests:

- The WASM binary hash (SHA-256 of the binary bytes)
- The bundle etag it was compiled from (binding WASM → contract)
- The signer identity

Verification:

```
tenor verify-wasm <evaluator.wasm> --sig <evaluator.wasm.sig> --pubkey <pubkey-file>
```

This verifies the WASM binary hasn't been tampered with AND can be linked back to a specific contract bundle.

### A5: Trust types in interchange crate

Add trust types to the interchange crate:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustMetadata {
    pub bundle_attestation: Option<String>,
    pub trust_domain: Option<String>,
    pub attestation_format: Option<String>,
    pub signer_public_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceTrustFields {
    pub trust_domain: Option<String>,
    pub attestation: Option<String>,
}
```

Add `trust: Option<TrustMetadata>` to the manifest type.
Add `ProvenanceTrustFields` to all provenance record types (FactProvenance, OperationProvenance, VerdictProvenance, TriggerProvenance).

These are all optional. Existing serialization/deserialization must continue to work when trust fields are absent.

### A6: Executor conformance suite

This is the big deliverable. Like the storage conformance suite but for executors.

Create `crates/executor-conformance/` (or add to an existing conformance location). The suite defines a trait:

```rust
#[async_trait]
pub trait TestableExecutor: Send + Sync {
    async fn load_contract(&self, bundle: &InterchangeBundle) -> Result<()>;
    async fn execute_flow(&self, flow_id: &str, persona: &str, facts: &FactSet, instance_bindings: &InstanceBindingMap) -> Result<FlowResult>;
    async fn simulate_flow(&self, flow_id: &str, persona: &str, facts: &FactSet, instance_bindings: &InstanceBindingMap) -> Result<FlowResult>;
    async fn get_entity_state(&self, entity_id: &str, instance_id: &str) -> Result<Option<String>>;
    async fn get_action_space(&self, persona: &str, facts: &FactSet) -> Result<ActionSpace>;
    async fn get_manifest(&self) -> Result<TenorManifest>;
}
```

Then a macro or function that generates tests for any implementation:

```rust
pub fn executor_conformance_tests<E: TestableExecutor>(executor: E) -> Vec<TestCase> { ... }
```

The suite tests obligations E1-E20:

| Obligation | Test                                                                      |
| ---------- | ------------------------------------------------------------------------- |
| E1         | Facts from provided FactSet only, no internal derivation                  |
| E2         | Transition source validation — wrong source state → error                 |
| E3         | Atomicity — multi-effect operation: all succeed or all fail               |
| E4         | Snapshot isolation — facts changed mid-flow don't affect flow             |
| E5         | Sub-flow snapshot inheritance — sub-flow uses parent snapshot             |
| E6         | DateTime UTC normalization                                                |
| E7         | Numeric model — fixed-point decimal, round-half-even                      |
| E8         | Branch isolation — parallel branches don't see each other's state changes |
| E9         | Join after full branch completion                                         |
| E10        | Manifest served at /.well-known/tenor with ETag                           |
| E11        | Manifest bundle is complete                                               |
| E12        | Etag changes when and only when bundle changes                            |
| E13        | Dry-run: simulation=true, no state changes, no audit log writes           |
| E14        | Capability advertisement when dynamic                                     |
| E15        | Instance creation in initial state only                                   |
| E16        | Instance identity stability                                               |
| E17        | Instance enumeration complete                                             |
| E18        | Artifact integrity attestation capability (when trust configured)         |
| E19        | Provenance authenticity capability (when trust configured)                |
| E20        | Trust domain in provenance records (when declared)                        |

Each test uses a known contract, known facts, known entity states, and asserts specific outcomes. The contracts and expected outputs are fixtures bundled with the suite.

### A7: Tests

- Unit tests for key generation, signing, verification
- Unit tests for WASM signing/verification
- Test: sign a bundle, modify one byte, verify fails
- Test: sign a bundle, verify with wrong key, verify fails
- Test: bundle without trust section passes all existing tests (backward compat)
- Test: provenance records with and without trust fields both deserialize correctly
- Executor conformance suite tests themselves (run against a mock executor)

### Acceptance criteria — Part A

- [ ] `tenor keygen` generates Ed25519 keypair
- [ ] `tenor sign` signs bundle with detached attestation
- [ ] `tenor verify` verifies signed bundle
- [ ] `tenor sign-wasm` signs WASM binary with bundle binding
- [ ] `tenor verify-wasm` verifies WASM binary
- [ ] Trust types in interchange crate (optional on manifest, optional on provenance)
- [ ] Executor conformance suite: E1-E20 test coverage
- [ ] Conformance suite trait allows any executor implementation
- [ ] All trust metadata optional — existing tests pass unchanged
- [ ] Evaluator completely unaffected (no trust imports in eval crate)
- [ ] All workspace tests pass
- [ ] `cargo clippy` clean

---

## Part B: Private Repo — Executor Trust Integration (~/src/riverline/tenor-platform)

After Part A is pushed, update deps:

```
cargo update -p tenor-eval -p tenor-storage
```

### B1: Trust configuration

Add trust configuration to the executor:

```toml
# tenor-platform.toml
[trust]
enabled = true
domain = "acme.prod.us-east-1"
secret_key_path = "/path/to/tenor-key.secret"
public_key_path = "/path/to/tenor-key.pub"
attestation_format = "ed25519-detached"
```

When trust is not configured, the executor operates in unattested mode (per AL80). All trust fields are omitted from provenance and manifest.

### B2: Manifest trust field

When trust is configured, the manifest served at `/{contract_id}/.well-known/tenor` includes the `trust` section:

```json
{
  "trust": {
    "bundle_attestation": "<signature>",
    "trust_domain": "acme.prod.us-east-1",
    "attestation_format": "ed25519-detached",
    "signer_public_key": "<pubkey>"
  }
}
```

The bundle attestation is computed at manifest generation time — sign the canonical bundle bytes with the configured key.

Per §19.2: the `trust` field is excluded from etag computation. The etag covers only the bundle. This means changing the signing key doesn't change the etag and doesn't invalidate cached bundles.

### B3: Provenance attestation

When trust is configured, all provenance records gain trust fields:

```json
{
  "trust_domain": "acme.prod.us-east-1",
  "attestation": "<signature-of-provenance-record>"
}
```

The attestation signs the provenance record's content (excluding the attestation field itself). This makes provenance tamper-evident — modifying a provenance record after attestation is detectable.

Per E19: provenance attestation can be per-record or batched. For this implementation, use per-record signing (simpler, auditable). The record is serialized to canonical JSON, signed, and the signature is stored alongside the record.

### B4: Run executor conformance suite

Wire the public repo's executor conformance suite to run against the private repo's PostgresExecutor. This is like the storage conformance suite but for the full executor.

```rust
#[cfg(test)]
mod conformance {
    use tenor_executor_conformance::executor_conformance_tests;
    use crate::PostgresExecutor;

    // Generate all E1-E20 tests
    executor_conformance_tests!(PostgresExecutor::test_instance());
}
```

All E1-E20 tests must pass. If any fail, fix the executor — the conformance suite is the authority.

### B5: Manifest capability

Add `trust_attestation: true` (or similar) to the capabilities object when trust is configured:

```json
"capabilities": {
  "migration_analysis_mode": "conservative",
  "source_adapters": true,
  "multi_instance_entities": true,
  "trust_attestation": true
}
```

### B6: Integration tests

- Test: executor with trust configured → manifest has trust section
- Test: executor without trust → manifest has no trust section (backward compat)
- Test: provenance records with trust → trust_domain and attestation present
- Test: provenance records without trust → trust fields absent
- Test: verify bundle attestation in manifest matches actual bundle content
- Test: verify provenance attestation is valid signature of record content
- Test: tamper with provenance record → attestation verification fails

### Acceptance criteria — Part B

- [ ] Trust configuration via config file
- [ ] Manifest includes trust section when configured
- [ ] Manifest omits trust section when not configured
- [ ] Provenance records include trust fields when configured
- [ ] Provenance attestations are valid signatures
- [ ] Executor conformance suite (E1-E20) passes
- [ ] Capability advertisement includes trust_attestation
- [ ] Backward compat: no trust config → identical to pre-trust behavior
- [ ] Integration tests pass
- [ ] `cargo check` passes
- [ ] `cargo clippy` clean

---

## Final Report

```
## Phase 5: Trust & Security — COMPLETE

### Public repo
- Key generation: `tenor keygen` (Ed25519)
- Bundle signing: `tenor sign` / `tenor verify`
- WASM signing: `tenor sign-wasm` / `tenor verify-wasm`
- Trust types: TrustMetadata, ProvenanceTrustFields in interchange crate
- Executor conformance suite: E1-E20, [N] tests
- Tests: [total] passing

### Private repo
- Trust configuration: [config mechanism]
- Manifest trust field: bundle_attestation, trust_domain, attestation_format
- Provenance attestation: per-record Ed25519 signatures
- Conformance suite: all E1-E20 tests pass against PostgresExecutor
- Capability: trust_attestation advertised
- Integration tests: [N] passing

### Invariants verified
- Evaluator unaffected: no trust imports in eval crate
- Trust optional: all tests pass without trust configuration
- Etag unchanged by trust field: verified

### Commits
Public: [list]
Private: [list]
```

Phase 5 is done when bundles can be signed and verified, provenance is attestable, the executor conformance suite passes E1-E20, and every checkbox above is checked. Not before.
