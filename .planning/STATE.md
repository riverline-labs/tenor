# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-02-22)

**Core value:** A contract authored in TenorDSL must be statically verifiable, evaluable against facts, and generatable into working code -- the full lifecycle from specification to execution with provenance at every step.
**Current focus:** Milestone 1 (v0.9) complete. Next: Milestone 2 — System construct (Phase 12) then Documentation (Phase 6).

## Current Position

Milestone: v1.0 — System Construct + Documentation
Phase: Not started (defining requirements)
Status: Defining requirements for v1.0 milestone
Last activity: 2026-02-22 — Milestone v1.0 started

## Performance Metrics

**Velocity:**
- Total plans completed: 32
- Average duration: ~8.2min
- Total execution time: ~4.4 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1. Spec Completion | 5 | 40min | 8.0min |
| 1.1. Spec CI: AI Ambiguity Testing | 2 | 7min | 3.5min |
| 2. Foundation | 4 | 58min | 14.5min |
| 3. CLI + Evaluator | 7/7 | ~57min | ~8.1min |
| 3.1. CFFP Migration Semantics | 2/2 | 13min | 6.5min |
| 3.3. Flow Migration Compatibility | 2/2 | 21min | 10.5min |
| 3.4. Contract Discovery | 2/2 | 13min | 6.5min |
| 4. Static Analysis | 8/8 | ~65min | ~8.1min |

**Recent Trend:**
- Last 5 plans: 05-02 (healthcare), 05-05 (trade finance), 05-04 (energy), 05-07 (gap report), 05-08 (executor conformance)
- Trend: Stable ~8min per plan; Phase 5 domain contracts averaged ~17min due to contract complexity; testing plans faster (~4min)

*Updated after each plan completion*
| Phase 03 P03 | 6min | 2 tasks | 4 files |
| Phase 03 P04 | 4min | 2 tasks | 2 files |
| Phase 03 P05 | 5min | 2 tasks | 8 files |
| Phase 03 P07 | 7min | 2 tasks | 13 files |
| Phase 03.1 P01 | 8min | 2 tasks | 1 files |
| Phase 03.1 P02 | 5min | 2 tasks | 1 files |
| Phase 03.2 P01 | 7min | 2 tasks | 5 files |
| Phase 03.2 P02 | 5min | 2 tasks | 11 files |
| Phase 03.2 P03 | 13min | 2 tasks | 15 files |
| Phase 03.3 P01 | 7min | 2 tasks | 1 files |
| Phase 03.3 P02 | 14min | 2 tasks | 1 files |
| Phase 03.4 P01 | 8min | 2 tasks | 3 files |
| Phase 03.4 P02 | 5min | 2 tasks | 3 files |
| Phase 05 P01 | 12min | 2 tasks | 11 files |
| Phase 05 P03 | 16min | 2 tasks | 10 files |
| Phase 05 P02 | 18min | 2 tasks | 11 files |
| Phase 05 P05 | 16min | 2 tasks | 9 files |
| Phase 05 P04 | 25 | 2 tasks | 10 files |
| Phase 05 P07 | 4min | 1 tasks | 1 files |
| Phase 05 P08 | 4min | 2 tasks | 2 files |
| Phase 05 P06 | 7min | 2 tasks | 3 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [CFFP](https://github.com/riverline-labs/iap) required for SPEC-01, SPEC-02, SPEC-03 before any implementation
- Domain validation (Phase 5) is a hard gate before code generation (Phase 6)
- Spec frozen after Phase 1 -- no language changes during tooling phases
- Persona is a pure identity token (no metadata, no delegation) -- CFFP Candidate A selected
- Persona section placed as Section 8 in TENOR.md, renumbering all subsequent sections
- Persona references in interchange remain as validated strings (parallels fact_ref pattern)
- P7 outcomes are Operation-local string sets (Candidate A) -- not shared constructs or typed variants
- Typed outcome payloads rejected (violate closed-world semantics C7)
- Flow OperationStep outcome handling must be exhaustive (all declared outcomes handled)
- Effect-to-outcome association explicit in contract for multi-outcome Operations
- Outcomes and error_contract are disjoint channels
- AL13 (Flow-side-only outcomes) superseded by P7
- P5 shared type library: structural typing preserved (no nominal identity change) -- CFFP Candidate C selected
- Type library files are self-contained leaves (no imports within type libraries) -- prevents transitive type propagation
- Nominal typing (Candidate B) eliminated: incompatible with interchange self-containedness
- Shared types placed as Section 4.6 under BaseType -- extends TypeDecl, not a new construct
- Scoped-down P5 canonical form: module federation, generics, import depth, flat namespace, type extension, selective import all deferred to v2
- Interchange schema authored from spec (not reverse-engineered from elaborator output) per user decision
- Single schema file with $defs for all construct kinds (not split per construct)
- Bundle-level tenor_version (semver) is canonical; per-construct tenor field is short version
- Spec frozen as v0.9 with CFFP provenance -- any semantic change requires a new CFFP run. v1.0 requires System construct (multi-contract composition layer) via dedicated CFFP run
- v0.9 → v1.0 transition: System construct is the only additive change gating v1.0 freeze. All existing v0.9 content remains frozen. System adds: spec section, elaborator passes, interchange representation, static analysis extensions (cross-contract authority topology, flow path enumeration), executor obligations (cross-contract snapshot coordination, persona resolution), conformance tests
- Worked example (Appendix C) updated for v1.0: persona declarations, outcomes fields, outcome-based routing
- ureq v3 (synchronous) chosen for ambiguity harness HTTP client -- no tokio async runtime
- Fixture triplet convention: .tenor + .facts.json + .verdicts.json per ambiguity test case
- Spec section extraction by numbered heading for targeted LLM context injection
- Hard errors (API failure, missing files) cause exit 1; LLM mismatches are informational (exit 0)
- Optional --model and --spec CLI flags for overriding ambiguity harness defaults
- Short directory names under crates/ (core, cli) while Cargo package names use tenor- prefix (tenor-core, tenor-cli)
- AST types extracted to ast.rs with pub use re-exports from parser.rs for backward compatibility
- Thin elaborate.rs orchestrator delegates to pass modules (pass1_bundle through pass6_serialize)
- Per-pass module convention: pass{N}_{name}.rs files in tenor-core
- Conservative public API: only re-export types and per-pass entry functions from tenor-core root, not internal helpers
- Stub crates (eval, analyze, codegen, lsp) depend on tenor-core from creation
- Bundle envelope carries both tenor (short "1.0") and tenor_version (semver "1.0.0")
- Expected JSON fixtures regenerated from elaborator output for exact canonical match
- Persona validation conditional on persona constructs existing in index (backward-compatible)
- outcomes_error_contract_collision test chosen over outcomes_missing since outcomes are optional on Operations
- jsonschema 0.42 for schema validation integration test (latest stable, validator_for + validate API)
- CI pipeline on push/PR to both main and v1 branches with build, conformance, test, fmt, clippy
- clap 4.5 derive API for CLI subcommand dispatch (replacing hand-rolled args parsing)
- jsonschema 0.42 with iter_errors API for schema validation error collection
- include_str! to embed interchange schema at compile time (binary is self-contained)
- CLI exit code convention: 0=success, 1=error, 2=not-implemented for stub subcommands
- CI conformance command updated from `run` to `test` to match new clap subcommand
- Evaluator types are DISTINCT from tenor-core AST types -- evaluator consumes interchange JSON, not raw DSL
- All evaluator numeric arithmetic uses rust_decimal::Decimal with MidpointNearestEven -- no f64 in evaluation paths
- Predicate expressions parsed from interchange JSON into evaluator's own Predicate enum (not tenor-core RawExpr)
- ProvenanceCollector threaded through eval_pred to track all fact/verdict references for provenance chains
- Short-circuit evaluation for And/Or operators in predicate evaluator
- Entity state changes tracked in mutable EntityStateMap, completely separate from immutable Snapshot
- Sub-flows inherit parent Snapshot by reference -- no new snapshot creation
- OperationError is a separate enum from EvalError to distinguish operation-specific failures
- Flow execution uses step index BTreeMap for O(1) step lookup by ID
- Max step count (1000) prevents infinite flow loops
- Outcome routing: effect-to-outcome mapping for multi-outcome ops, first declared outcome for single-outcome
- Hand-built domain-aware diff (not generic JSON diff) keyed by (kind, id) not array position
- Provenance and line fields excluded from diff comparison (noise fields)
- Primitive arrays normalized as sets for comparison (states, allowed_personas order-insensitive)
- Diff exit code convention: 0=identical, 1=different (matches Unix diff)
- workspace_root() helper (CARGO_MANIFEST_DIR -> parent -> parent) for integration test fixture path resolution
- Static fixture files preferred over tempfile-generated for eval tests (reproducibility)
- Eval text output: [verdict_type] payload (rule: id, stratum: N) format for human readability
- File-based numeric fixtures adapted to elaborator output constraints (no result_type for Decimal Mul, comparison_type only for cross-type comparisons)
- Elaborator type checker is sound for Int range arithmetic -- runtime overflow impossible for declared ranges, so Int overflow fixtures not feasible through file-based pipeline
- Hybrid diff representation (Candidate C) selected via CFFP collapse -- DiffEntry JSON is primary authoritative output, Tenor migration contract is supplementary (tenor diff --migration)
- Migration contracts in v1.0 are classification-only (Facts + Rules) -- orchestration (Operations/Flows) deferred to v2 due to meta-level construct requirements
- Migration contracts are self-contained -- no imports of source contracts due to namespace collision in Pass 1 import resolution
- REQUIRES_ANALYSIS is valid third classification for predicate expression changes where strength comparison is undecidable
- In-flight flow migration policy (blue_green, force_migrate, abort) is executor deployment obligation, not contract-level construct
- Checked Decimal arithmetic for precision bounds: use Decimal::TEN.checked_mul() loop instead of i64.pow() to prevent panics on precision > 18
- Currency validation in Money comparison: coerce_to_money returns (Decimal, &str) forcing callers to validate currency match
- Multi-outcome Operations REQUIRE explicit effect-to-outcome mapping; silent fallback to outcomes[0] eliminated
- ISO 8601 date format validation is pattern-only (YYYY-MM-DD) without calendar correctness -- simple format check sufficient
- Duration cross-unit comparison returns TypeError; full unit promotion deferred to Phase 5
- EvalError::FlowError variant used for flow execution errors (step limit, etc.) instead of TypeError
- SAFETY comment convention: every .unwrap() in non-test tenor-core code must have // SAFETY: annotation explaining the invariant
- Structured HTTP status extraction for API retry logic (extract_http_status) replaces fragile string matching
- Workspace dependency normalization: shared deps pinned once in root Cargo.toml, crates reference via workspace = true
- BranchOutcome enum for parallel branch classification (Success/Failure) instead of string-matching
- handle_failure() shared helper for OperationStep and SubFlowStep failure handling (returns Option<FlowResult>)
- initiating_persona set on FlowResult after execute_flow returns (not threaded through execution)
- Escalate failure handler added to core AST/parser/serializer (was eval-only; needed for conformance tests)
- §18 Contract Discovery & Agent Orientation: executor obligations E10-E13 (manifest serving, cold-start, change detection, dry-run). No new language constructs. Inserted before Pending Work, renumbered §19-§22.
- TenorManifest: { bundle (inlined interchange), etag (SHA-256 of canonical bundle bytes), tenor (spec version) }. Keys sorted lexicographically. Static artifact, no live server required.
- Stdlib artifact dropped: Tenor has no user-invokable functions; closed function set already formally specified in §4.2/§12 and enforced by elaborator. Machine-readable encoding would be redundant.
- Elaboration validation gaps (FactSource format, operation I/O) → Phase 4 static analysis, not a new decimal phase
- Phase 3.4 scoped to: manifest-schema.json + `tenor elaborate --manifest` flag only
- E10-E13 executor conformance testing → Phase 5 (tested against real domain contracts)
- Flow migration compatibility: three-layer analysis (Layer 1 verdict isolation, Layer 2 entity state, Layer 3 operation/flow structure) with Candidate C (layered) selected via CFFP collapse over graph-theoretic (A) and predicate-based (B)
- Reachable path computation uses v2's step graph from current position -- routing after migration follows v2 semantics
- Flow-level compatibility refines construct-level breaking change taxonomy: a BREAKING construct change may be COMPATIBLE at specific flow positions
- Conservative data dependency analysis (frozen snapshot + current entity states) is REQUIRED; aggressive analysis with intra-path production (path dominance) is OPTIONAL
- Coexistence layer (v1.5) is executor implementation strategy (MAY), not spec obligation (MUST)
- Breaking change taxonomy exhaustive: every (construct_kind, field, change_type) triple classified across all six construct kinds
- Non-normative spec content separated: changelog, Appendix B (convergence record), Pending Work sections, CFFP provenance blocks removed from TENOR.md
- Appendix A AL numbering preserved with gaps after trimming to avoid cascading cross-reference breakage
- Section 17.6 Flow Migration Compatibility added with formal FMC1-FMC7 conditions and three-layer analysis model
- Eight migration obligations (M1-M8) in TENOR.md Section 17.3 -- M7 and M8 added beyond CFFP minimum for implementation precision
- AL37-AL43 capture all seven CFFP scope narrowings as acknowledged limitations in Appendix A
- TENOR.md Section 17 cross-references: Section 13.2.1 (format vs content versioning), Section 16 (migration obligations callout), Section 17.5 (acknowledged limitations back-references)
- [Phase 05]: Evaluator FieldRef resolution falls back to facts when binding not found -- enables Record-typed fact field access in predicates
- [Phase 05]: Domain contract fixture pattern: .tenor + .facts.json + .verdicts.json triplets in domains/{name}/ with domains_dir() helper for test resolution
- [Phase 05]: Split InspectionLot into QualityLot + ComplianceLot to satisfy parallel branch disjoint entity constraint (GAP-004)
- [Phase 05]: No bounded existential quantification in Tenor v1.0 (GAP-003); De Morgan workaround using negated universal
- [Phase 05]: Domain contract operations in flows use verdict_present() preconditions (not evolving fact values) due to frozen snapshot model (GAP-009)
- [Phase 05]: Multi-outcome operations require BranchStep workaround until effect-to-outcome DSL syntax is added (GAP-006/GAP-010)
- [Phase 05]: Verdict-based preconditions in flows for frozen snapshot compatibility (GAP-009)
- [Phase 05]: No TaggedUnion type in DSL -- use Enum for simple discrimination, separate Record facts for variant payloads (GAP-005)
- [Phase 05]: Escalation handler targets reported as unreachable by S6 -- informational only, not a blocker (GAP-007)
- [Phase 05]: No flow-level eval in CLI -- flow evaluation only via Rust API evaluate_flow() (GAP-008)
- [Phase 05]: Healthcare prior auth is the "wow" showcase: 465 lines, 6 personas, 4 strata, SubFlowStep, Escalate, HandoffStep, bounded quantification
- [Phase 05]: Split multi-outcome award_contract into separate award_rfp and reject_rfp operations due to DSL lacking effect-to-outcome mapping syntax (GAP-013)
- [Phase 05]: run_domain_flow_fixture() helper decouples contract file name from fixture name for multi-file contracts
- [Phase 05]: Approval tier routing via BranchStep (not multi-outcome operation) models real procurement approval chains more accurately
- [Phase 05]: ANSI escape codes used directly for terminal styling in explain subcommand instead of crossterm/textwrap dependencies -- simpler, no new deps needed
- [Phase 05]: Spec gap report: 13 gaps across 5 domains -- spec cleared for Phase 6 code generation with no blockers. Effect-to-outcome mapping (GAP-006/010/013) is highest-priority v1.x improvement.
- [Phase 05]: E10-E13 executor conformance tested against real domain contracts (healthcare for E11 cold-start, SaaS+healthcare for E13 dry-run). Recursive predicate walker validates all fact_ref/verdict_present references from consumer JSON perspective.

### Roadmap Evolution

- Phase 01.1 inserted after Phase 1: Spec CI — AI Ambiguity Testing (URGENT)
- Phase 3.1 inserted after Phase 3: CFFP — Migration Semantics. Javier Muniz identified contract versioning/migration as a business-readiness gap. Resolution: L1 structural diff (`tenor diff`) in Phase 3; CFFP round on breaking change taxonomy + versioning spec prose in Phase 3.1; L2 breaking change analysis (`tenor diff --breaking`) in Phase 4 using S1-S7 building blocks. In-flight flow migration: spec requires executors to declare policy (blue-green/force-migrate/abort), not prescribe which one.
- Phase 4 (code generation): DeepSeek raised "compiled to data + generic interpreter" vs "compiled to code + bindings" as a fundamental architectural decision — needs dedicated design pass during Phase 4 context/planning, not Phase 1
- Phase 3.2 inserted after Phase 3: Technical Debt, Bug Fixes & Missing Eval Features (URGENT). Codebase concerns audit identified critical bugs (precision overflow panic, silent incorrect behavior in Money/Duration/Date types, multi-outcome fallback), unimplemented eval features (ParallelStep, Compensate, Escalate failure handlers, flow-level persona auth), and tech debt (unwrap fragility, string-match retry logic, dead code, workspace dep drift). All must be resolved before Phase 4.
- Phase 3.3 inserted after Phase 3.2: CFFP — Flow Migration Compatibility (URGENT). Javier Muniz and Brandon Bush identified that flows outlive contracts — business processes spanning weeks/months will inevitably cross contract versions. The three compatibility conditions for force-migration: (1) forward path existence (every future step has v2 equivalent), (2) backward data dependency satisfaction (v2 steps' prerequisites satisfiable from existing provenance/snapshot/defaults), (3) directional asymmetry (v2 may require state v1 never established). Coexistence layer pattern (v1.5): new flows on v2, incompatible in-progress flows on v1 runtime, translate on exit. Must be formalized in spec before Phase 4's `tenor diff --breaking` can implement flow-level analysis.
- Phase 5.1 inserted after Phase 5: Fix critical DSL gaps before v1-complete (URGENT)
- Standing preference: stay in Rust as much as possible across all phases

### Pending Todos

None yet.

### Blockers/Concerns

- Requirements count: traceability table has 66 entries but REQUIREMENTS.md states 62. Actual count is 66. Updated during roadmap creation.

## Session Continuity

Last session: 2026-02-22
Stopped at: Phase 05.1 complete (3/3 plans). Next: Phase 6 Documentation.
Resume file: .planning/ROADMAP.md
