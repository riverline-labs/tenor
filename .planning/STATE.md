# Project State

## Current Position

**Phase**: 6 of 11 — Advanced Policies
**Plan**: 4 of 4 completed in current phase
**Status**: Phase 6 complete
**Last activity**: 2026-02-27 — Completed plan 06-04 (Advanced Policies documentation)

Progress: ████████████░░░░░░░░ 55% (Phases 1-6 complete)

## Decisions

- Phase plans authored by PM-level Claude (spec/vision access, no codebase access)
- Flag code-level discrepancies between plans and actual codebase to user
- Phase 4 Part A (public repo) first; Part B (private repo) after push
- EntityStateMap uses (entity_id, instance_id) composite key per §6.5
- `_default` instance ID for backward compat
- WASM auto-detects old vs new format
- Use DEFAULT_INSTANCE_ID = "_default" for all single-instance backward-compat paths
- Public API (WASM, HTTP serve) accepts flat entity_id->state JSON and converts via single_instance()
- single_instance() and get_instance_state() re-exported from crates/eval/src/lib.rs
- No function signature changes in plan 04-01 (reserved for plan 04-02)
- InstanceBindingMap is BTreeMap<String, String> (entity_id to instance_id)
- Empty InstanceBindingMap falls back to DEFAULT_INSTANCE_ID for full backward compat
- EffectRecord gains instance_id field per §9.5 provenance requirements
- evaluate_flow() public API gains instance_bindings parameter (empty map = backward compat)
- Action.instance_bindings is BTreeMap<String, BTreeSet<String>> (entity_id to set of valid instance_ids) per §15.6
- OperationProvenance.state_before/state_after use BTreeMap<(String,String),String> tuple keys (internal type, not serialized)
- Two-pass effect loop: validate+capture state_before first, then apply, then capture state_after
- StepRecord.instance_bindings empty for non-operation steps (branch, handoff, escalation)
- parse_entity_states() WASM helper: string value = old flat (-> _default), object value = new nested (-> direct parse)
- simulate_flow_with_bindings() is new 6-arg WASM export; simulate_flow() kept as 5-arg backward-compat wrapper
- missing_instance_binding at flow level: on_failure terminates with failure outcome (Ok result), not Rust Err; direct execute_operation returns OperationError::EntityNotFound
- [05-01] All trust fields are Option<String> — backward compat via field absence, not sentinel values
- [05-01] TrustMetadata and ProvenanceTrustFields live in interchange crate; evaluator has zero trust imports (invariant I1)
- [05-01] parse_trust_metadata() returns None for both absent field and explicit null
- [05-02] Keygen --output renamed to --prefix; Sign --output renamed to --out to avoid clap global arg naming conflict
- [05-02] Sign covers etag bytes (SHA-256 of canonical compact bundle JSON), not raw bundle bytes
- [05-02] verify_strict() used for stricter Ed25519 signature checking vs verify()
- [05-02] Both bare bundle and manifest-wrapped inputs supported by tenor sign
- [05-03] Attestation payload is wasm_hash:bundle_etag (colon-separated); signed directly, not re-hashed
- [05-03] Detached .sig file fields sorted lexicographically (attestation_format, bundle_etag, signature, signer_public_key, wasm_hash)
- [05-03] read_secret_key() expects 32-byte seed (not 64-byte expanded key) for ed25519-dalek 2.x
- [05-03] trust module (keygen.rs) ready for 05-02-style reuse; ed25519-dalek/base64 in workspace
- [05-04] executor_conformance_tests! macro uses #[macro_export] — accessible at crate root, not suite:: path
- [05-04] E18-E20 trust tests conditional via is_trust_configured() — pass unconditionally when trust absent (AL80)
- [05-04] Graceful skip pattern for optional features (E6/E7/E8/E9): match error message for "unsupported"/"not implemented"
- [05-04] TestableExecutor uses &self (not &mut self) — executor internal state managed by implementor
- [05-05] Expose thin public helpers (sign_bundle, verify_bundle, sign_wasm_bytes, verify_wasm_bytes) so cmd_* functions delegate and tests call helpers directly without process::exit
- [05-05] VerifyResult / VerifyWasmResult enums give structured failure reasons for unit-test assertions
- [05-05] Backward compat test scans conformance/positive/*.expected.json files at runtime via CARGO_MANIFEST_DIR navigation
- [05-05] clippy::manual_map requires `else if let Some(x) = opt { Some(x) } else { None }` -> `opt.cloned()`
- [05-06] Private repo: all git deps switched to local path deps (executor-conformance not yet on GitHub)
- [05-06] strip_attestation_field must strip both 'attestation' AND 'trust_domain' for sign/verify content consistency
- [05-06] Conformance fixture step refs must be bare strings (not {step: id} objects) for parse_step_target
- [05-06] normalize_rule must fix both when-clause (type error) and produce payload (format error)
- [05-06] Random suffix for conformance test DB names — now_v7() causes collision when 20 tests start simultaneously
- [Phase 06-01]: TimeoutBehavior uses #[derive(Default)] with #[default] on Reject variant (clippy::derivable_impls)
- [Phase 06-01]: ApprovalCallback type alias resolves clippy::type_complexity for CallbackApprovalChannel field
- [Phase 06-01]: HumanInTheLoopPolicy short-circuits on empty action space before consulting delegate or channel
- [Phase 06-01]: CallbackApprovalChannel is the test harness for HITL policy unit tests (avoids stdin dependency)
- [Phase 06-02]: LlmPolicy short-circuits on empty action space before calling client
- [Phase 06-02]: On LlmError (network/API), return None immediately — no retry (server-side failure)
- [Phase 06-02]: Retry loop appends assistant+user correction pair on parse failure (up to max_retries)
- [Phase 06-02]: Return canonical Action from action space after flow_id validation (not LLM-generated)
- [Phase 06-02]: AnthropicClient uses ureq v3 API (Agent::new_with_defaults, .header(), .send_json(body)) matching HttpAdapter pattern
- [Phase 06-02]: anthropic feature requires both ureq and tokio (spawn_blocking needs tokio runtime)
- [Phase 06-03]: EntityStatePredicate uses direct == Some(state) comparison (clippy::unnecessary_map_or)
- [Phase 06-03]: CompositePolicy filtered ActionSpace has empty blocked_actions (approver only needs the proposed action)
- [Phase 06-03]: Short-circuit on proposer None via ? operator — no predicate/approver consulted
- [Phase 06-04]: HumanInTheLoopPolicy.timeout field stores intended duration but delegates enforcement to the ApprovalChannel implementation — choose() does not enforce via tokio::time::timeout

## Blockers/Concerns

- Part B (private repo) depends on Part A being pushed to main first
- WASM crate excluded from workspace — needs separate build/test

## Performance Metrics

| Phase | Plan | Duration (s) | Tasks | Files |
|-------|------|-------------|-------|-------|
| 04 | 01 | 740 | 2 | 7 |
| 04 | 02 | 633 | 2 | 11 |
| 04 | 03 | 536 | 2 | 4 |
| 04 | 04 | 280 | 2 | 1 |
| 04 | 05 | 248 | 2 | 2 |
| 05 | 01 | 1041 | 5 | 2 |
| 05 | 02 | 443 | 6 | 8 |
| 05 | 03 | 423 | 5 | 8 |
| 05 | 04 | 385 | 7 | 24 |
| 05 | 05 | 847 | 7 | 7 |
| 05 | 06 | ~10800 | 9 | 15 |

## Performance Metrics (continued)

| 06 | 01 | 318 | 9 | 2 |
| 06 | 02 | 311 | 8 | 3 |
| 06 | 03 | 257 | 7 | 2 |
| 06 | 04 | 171 | 6 | 1 |

## Session Continuity

Last session: 2026-02-27
Stopped at: Completed plan 06-04 (Advanced Policies documentation — Phase 6 complete)
Next action: Begin Phase 7 when scheduled
