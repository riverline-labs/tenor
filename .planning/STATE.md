# Project State

## Current Position

**Phase**: 9 of 11 — Builder SPA — COMPLETE
**Plan**: 7 of 7 completed in current phase
**Status**: Phase 9 complete, ready for Phase 10
**Last activity**: 2026-02-27 — Phase 9 Plan 7 complete (Builder test suite: 153 tests, production build verified)

Progress: ████████████████████░░ 78% (Phases 1-9 complete)

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
- [Phase 07-01]: TenorEvaluator.fromJson/fromBundle are synchronous (not async) — wasm-pack --target nodejs produces synchronous CommonJS module
- [Phase 07-01]: FactValue uses interface-based recursion (FactRecord, FactList) to avoid TypeScript circular type alias error
- [Phase 07-01]: executeFlowWithBindings() added to expose simulate_flow_with_bindings WASM export for multi-instance use
- [Phase 07]: [07-03] Use wasm32-wasip1 target (not wasm32-unknown-unknown) for Go SDK bridge — avoids wasm-bindgen imports that wazero cannot satisfy
- [Phase 07]: [07-03] wasi_snapshot_preview1.MustInstantiate(ctx, r) must precede module load; WithStartFunctions() prevents wazero calling _start on library-style WASM
- [Phase 07]: pyo3 0.23 with abi3-py39: stable ABI for Python 3.9+ compatibility, required when Python 3.14 on dev machine exceeds pyo3 max version
- [Phase 07]: [07-02] execute_flow uses init_entity_states() + overlay pattern (not evaluate_flow() top-level); mirrors WASM for empty entity state handling
- [Phase 07]: [07-02] PyO3 cdylib needs [workspace] table in Cargo.toml to exclude from root workspace
- [Phase 07-04] WASM simulate_flow_with_bindings output is canonical format; Python SDK execute_flow must include simulation:true and instance_bindings:{} to match
- [Phase 07-04] Go SDK omitempty removed from VerdictProvenance.VerdictsUsed, BlockedAction.InstanceBindings, FlowResult.InstanceBindings — Rust always emits these even when empty
- [Phase 07-04] Conformance fixture-gen Cargo.toml uses [workspace] stub to avoid workspace conflict (same as Python SDK)
- [Phase 07-04] Python runner uses .venv (maturin develop) if available, falls back to PYTHONPATH for pre-built .so
- [Phase 08-01] Ui subcommand uses --out flag (not --output) to avoid clap global arg conflict with --output OutputFormat
- [Phase 08-01] types.ts uses plain string for Decimal/Money/Date/DateTime (not branded types) for simpler UI usage
- [Phase 08-01] theme.ts uses djb2 hash of contract_id for hue derivation — deterministic HSL color palette
- [Phase 08-01] generate_ui_project uses vec![] macro to build file list (clippy::vec_init_then_push)
- [Phase 08-02] emit_action_space and emit_fact_input use r##"..."## raw strings (no format!()) — clippy::useless_format fires when format! used with {{}} escaping but no Rust variable substitution
- [Phase 08-02] FactInput dispatches on FACTS metadata from types.ts at runtime (no static codegen of type info in component)
- [Phase 08-02] ProvenanceDrill uses r## delimiter since color="#7c3aed" contains "# which terminates r# raw strings
- [Phase 08-02] Entity transitions field absent in CodegenEntity — ENTITIES const emits transitions: [] as placeholder
- [Phase 08-03] theme.rs extracted from generate.rs as dedicated module with 6 unit tests
- [Phase 08-03] Custom theme uses per-color merge (not full replacement): {"colors": {"primary": "#ff0000"}} overrides just primary
- [Phase 08-03] textPrimary/textSecondary rename: matches new theme.ts output keys; old text/textMuted removed from all generated TypeScript
- [Phase 08-03] sidebar color removed from theme: Layout now uses theme.colors.surface for sidebar background
- [Phase 08-03] styles.css emitted as standalone global reset: imported in main.tsx, complements per-component inline styles
- [Phase 08]: [08-04] Minimal contract written inline as const (not a fixture file) for self-contained tests
- [Phase 08]: [08-04] TypeScript compilation tests marked #[ignore] — avoid CI dependency on Node.js
- [Phase 08]: [08-04] Fact ID assertions use OR patterns (isActive || is_active) to tolerate camelCase conversion

- [Phase 09-01] Internal model is always interchange JSON; DSL generated only at export time
- [Phase 09-01] WASM pkg committed via git force-add (wasm-pack generates .gitignore with * that blocks tracking)
- [Phase 09-01] zundo used for undo/redo (zustand temporal middleware, 50-state history)
- [Phase 09-01] Simulation store reads contractHandle from elaboration store (no duplicate WASM state)
- [Phase 09-02] FactDefault plain strings for Text/Date/DateTime/Enum (no text_literal/enum_literal in interchange type)
- [Phase 09-02] Rename pattern: removeConstruct(oldId) + addConstruct({...updated}) since updateConstruct matches (id, kind)
- [Phase 09-02] Source description field reused for base_url/connection string (no dedicated field in SourceConstruct)
- [Phase 09-02] StateMachine nodePosOverrides reset via useEffect on states/transitions change to avoid stale drag positions
- [Phase 09]: PredicateBuilder uses ExprTypeSelector inline (not a separate dialog) to minimize friction for expression type changes
- [Phase 09]: StratumView compact mode auto-triggers when > 8 rules to avoid horizontal overflow
- [Phase 09]: Rule conditions filter availableVerdicts to strata < current rule stratum; operation mode passes all verdicts
- [Phase 09]: AuthorityMatrix toggles dispatch directly to updateConstruct on OperationConstruct (operation owns allowed_personas)
- [Phase 09-04]: FlowDag uses SVG viewBox pan/zoom — no external graph library dependency
- [Phase 09-04]: Step detail panel shown as right sidebar when step selected in DAG
- [Phase 09-04]: FlowConstruct.steps is FlowStep[] (array) not Record<string,FlowStep> — PM plan had stale assumption
- [Phase 09-05]: Client-side step replay: simulateFlow() runs full WASM simulation at once; stepFlowForward() reveals steps from stored result — no per-step WASM calls
- [Phase 09-05]: FactInputPanel derives zero-defaults from type.base when no explicit fact.default declared
- [Phase 09-05]: ActionSpacePanel computes "unauthorized" client-side by diffing persona allowed_personas vs WASM action space result
- [Phase 09-04]: ParallelStep branches rendered as swim lanes within a single node (not recursive sub-DAG)
- [Phase 09-06]: ZIP export without JSZip dependency — dynamic import() with `new Function()` escape hatch, falls back to combined text blob
- [Phase 09-06]: importTenorFile raises descriptive error directing to CLI (no client-side Rust parser available)
- [Phase 09-06]: ContractPreLoader placed inside BrowserRouter to access useNavigate() — checks ?contract= then VITE_TENOR_CONTRACT_PATH
- [Phase 09-06]: tenor builder Ctrl+C relies on OS SIGINT propagation to child process group; no ctrlc crate added
- [Phase 09-06]: BuilderCommands::Build uses --out flag to avoid clap global arg conflict with --output OutputFormat
- [Phase 09]: happy-dom used over jsdom: jsdom v27 requires Node >=20.19; happy-dom works on Node 20.17
- [Phase 09]: [09-07] WASM evaluator mocked in setup.ts via vi.mock() — WASM cannot run in jsdom/happy-dom test environment

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
| Phase 09 P03 | 408 | 6 tasks | 6 files |
| Phase 09 P07 | 754 | 8 tasks | 12 files |

## Performance Metrics (continued)

| 06 | 01 | 318 | 9 | 2 |
| 06 | 02 | 311 | 8 | 3 |
| 06 | 03 | 257 | 7 | 2 |
| 06 | 04 | 171 | 6 | 1 |
| 07 | 01 | 1393 | 8 | 12 |
| 07 | 02 | 1997 | 8 | 12 |
| 07 | 03 | 1695 | 8 | 11 |
| 07 | 04 | 686 | 6 | 23 |
| 08 | 01 | 438 | 6 | 5 |
| 08 | 02 | 882 | 7 | 6 |
| 08 | 03 | 291 | 5 | 5 |
| 08 | 04 | 216 | 6 | 1 |
| 09 | 01 | 746 | 10 | 21 |
| 09 | 02 | 403 | 6 | 6 |
| 09 | 03 | 408 | 6 | 6 |
| 09 | 04 | 474 | 5 | 4 |
| 09 | 05 | 437 | 7 | 8 |
| 09 | 06 | 592 | 7 | 8 |

| 09 | 07 | 754 | 8 | 12 |

## Session Continuity

Last session: 2026-02-27
Stopped at: Completed 09-07-PLAN.md (Builder test suite: 153 tests, vitest, happy-dom, WASM mocks, production build verified)
Next action: Phase 10
