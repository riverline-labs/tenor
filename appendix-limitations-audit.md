# Appendix Limitations Audit

Date: 2026-02-25
Spec location: docs/TENOR.md, lines 2404–2545

---

## RESOLVED (limitation no longer applies)

**AL52 — Concurrent Operation isolation unspecified** _(Executor, §17)_
Claimed: E3 requires atomicity for a single Operation's effect set but does not specify isolation semantics for concurrent invocation of multiple Operations against the same entity.
Evidence: OCC with version-based conflict detection is fully implemented in the storage conformance suite at `crates/storage/src/conformance/concurrent.rs`. Four test cases enforce at-most-one-winner semantics: `concurrent_updates_exactly_one_wins`, `concurrent_initialize_exactly_one_wins`, `concurrent_updates_different_entities_all_succeed`, `concurrent_updates_final_state_consistent`. All use snapshot-based versioning with `ConcurrentConflict` error handling.
Recommendation: Remove from appendix or mark as resolved. The storage layer now specifies and enforces OCC isolation semantics.

---

## PARTIALLY RESOLVED

**AL5 — TaggedUnion absence semantics** _(BaseType)_
Claimed: Mismatched tag access produces a typed absence value, which evaluates to false in predicate context.
Evidence: TaggedUnion deserialization in `crates/eval/src/types.rs:725-758` validates the tag against declared variants. An unknown tag causes `EvalError::DeserializeError`, not a typed absence value. Predicate evaluation in `crates/eval/src/predicate.rs:65-96` handles only Record field access; TaggedUnion field access falls through to `NotARecord` error.
Remaining: The spec describes absence semantics for mismatched tag access, but the implementation errors instead of producing absence. This is a spec-implementation mismatch, not just a limitation.
Recommendation: Either update the spec to clarify that mismatched tag access is an error at the evaluator level, or implement the absence value semantics in the evaluator.

**AL30 — Operation outcome declarations are mandatory in v1.0** _(Operation)_
Claimed: Contracts must add outcomes declarations to Operations.
Evidence: Pass 5 validation at `crates/core/src/pass5_validate.rs:472-509` enforces outcome labels only on effects in multi-outcome operations (2+ outcomes). Single-outcome operations or operations with empty `outcomes` vectors are accepted without error.
Remaining: The spec says outcomes are "mandatory in v1.0" but the elaborator only enforces when there are 2+ outcomes. An Operation with zero declared outcomes is valid.
Recommendation: Update the limitation text to reflect that outcomes are enforced only for multi-outcome operations, or tighten the elaborator to require outcomes on all Operations.

**AL56 — Shared entity state set equality required** _(System, §12)_
Claimed: Shared entity state sets must be identical across all member contracts.
Evidence: C-SYS-14 is listed in `crates/core/src/pass5_validate.rs:1019-1022` as a "cross-contract deep validation" constraint. However, this validation "requires elaborated member contracts, which are not available in the standard single-file pipeline." The check is specified but not executed in the current Pass 5 validation flow.
Remaining: C-SYS-14 state set equality enforcement is not active in the current single-file elaboration pipeline.
Recommendation: Update the limitation to note that the validation constraint exists but is only enforced when the System is elaborated with all member contracts loaded (not in single-file mode).

**AL61 — No recursive System embedding** _(System, §12)_
Claimed: A System member must be a contract file, not another System file.
Evidence: C-SYS-03 is documented in the spec. Pass 5 validates System constructs and `crates/core/src/pass1_bundle.rs:93` tracks System constructs in the index. However, explicit enforcement code that rejects a System file used as a member file was not found during code review.
Remaining: The spec constraint exists but the enforcement path is not clearly visible in the codebase.
Recommendation: Verify whether C-SYS-03 is actually enforced. If not, add the check to Pass 1 or Pass 5.

---

## STILL TRUE

**AL1 — Fact ground property boundary** _(Fact 1.0)_
Claimed: Facts are ground within the evaluation model. Whether the source populating them is itself derived is outside the language's enforcement scope.
Verified: The FactProvider trait at `crates/eval/src/fact_provider.rs:38-41` accepts any HashMap with no source validation. No code path verifies that facts came from external sources rather than internal rule evaluation. The CONCERNS document in tenor-platform explicitly lists E1 (external source integrity) as untested.

**AL8 — List max is a conservative static bound** _(Fact extension)_
Claimed: Runtime lists may be smaller. Static complexity analysis uses the declared max.
Verified: List assembly at `crates/eval/src/assemble.rs:105-114` enforces max as a runtime boundary via `ListOverflow` error. No static complexity analysis engine exists that uses declared max for worst-case analysis. Static analysis (Phase 4) is not yet implemented.

**AL17 — Branch decision provenance** _(Flow)_
Claimed: Branch decisions are recorded in Flow provenance but not in the Operation provenance chain.
Verified: Branch execution at `crates/eval/src/flow.rs:375-382` emits a `StepRecord` with `step_type: "branch"` into `steps_executed` (flow-level provenance). No Operation-level provenance records are created for branch decisions. The separation is maintained as designed.

**AL18 — Duration calendar independence** _(Duration)_
Claimed: Duration "day" means exactly 86,400 seconds. DST transitions, leap seconds, and calendar spans are not representable.
Verified: Duration is stored as `{ value: i64, unit: String }` at `crates/eval/src/types.rs:143-145`. Comparison at `crates/eval/src/numeric.rs:150-166` requires exact unit match with no calendar-aware conversion. No DST, leap second, or calendar logic exists anywhere in the codebase.

**AL22 — Post-parallel verdict re-evaluation requires new Flow** _(ParallelStep)_
Claimed: Frozen verdict semantics apply within parallel blocks.
Verified: `crates/eval/src/flow.rs:1-4` states "frozen verdict snapshot semantics." The `Snapshot` struct at lines 25-33 is immutable: "Per spec Section 11.4, this snapshot is NEVER recomputed during flow execution." ParallelStep execution at lines 491-520 passes shared immutable snapshot to all branches.

**AL24 — Persona declaration is mandatory in v1.0** _(Persona)_
Claimed: Contracts must add explicit persona declarations.
Verified: Pass 5 validation at `crates/core/src/pass5_validate.rs:432-452` checks every persona in `allowed_personas` against declared personas. Undeclared personas produce an error. Conformance test `conformance/negative/pass5/persona_undeclared.tenor` confirms enforcement.

**AL28 — Outcome labels carry no typed payload** _(Operation)_
Claimed: Outcome labels are bare strings with no associated payload data.
Verified: AST at `crates/core/src/ast.rs:188-189` defines outcomes as `Vec<String>`. Interchange at `crates/interchange/src/types.rs:127-148` defines `outcome: Option<String>` on Effect — bare string, no payload field.

**AL31 — Module federation deferred to v2** _(P5 Shared Type Library)_
Claimed: Inter-organization type sharing is out of scope for v1.0.
Verified: Import mechanism in `crates/core/src/pass1_bundle.rs` supports only local file paths via `import "path.tenor"`. No registry, versioning, or cross-repository distribution mechanism exists.

**AL32 — Generic type parameters deferred to v2** _(P5 Shared Type Library)_
Claimed: Shared type libraries cannot define parameterized types.
Verified: `RawType` enum in `crates/core/src/ast.rs:23-60` contains only concrete types. No generic parameter syntax in parser. TypeRef resolution handles bare identifier refs only, not parameterized refs.

**AL33 — Type library files may not import other files** _(P5 Shared Type Library)_
Claimed: Type library files are self-contained leaf files.
Verified: No explicit enforcement code prevents a type library file from importing. The restriction is upheld by convention (conformance test `shared_types_lib.tenor` has no imports) but not enforced by the elaborator.

**AL34 — TypeDecl flat namespace across imports** _(P5 Shared Type Library)_
Claimed: TypeDecl names occupy a flat namespace; duplicates cause error.
Verified: `check_cross_file_dups()` at `crates/core/src/pass1_bundle.rs:82-116` uses `HashMap<(&str, &str), &Provenance>` keyed by `(kind, id)`. Cross-file TypeDecl duplicates produce a Pass 1 error.

**AL35 — No type extension or inheritance across libraries** _(P5 Shared Type Library)_
Claimed: A contract cannot import a type and extend it.
Verified: `RawConstruct::TypeDecl` in `crates/core/src/ast.rs:140-144` has no `parent` or `extends` field. Parser does not recognize extension syntax. Pass 3 type environment maps names to concrete `RawType` with no super-type tracking.

**AL36 — No selective type import** _(P5 Shared Type Library)_
Claimed: Importing a type library loads all its TypeDecl definitions.
Verified: `parse_import()` at `crates/core/src/parser.rs:624-630` takes only a file path string. Load mechanism in Pass 1 flattens all constructs from imported file into the construct list. No filtering or selector syntax exists.

**AL37 — Migration contracts cannot express complex type changes** _(Migration)_
Claimed: Only simple type parameter changes are faithfully representable as Tenor Facts.
Verified: `classify_type_change()` in `crates/cli/src/diff.rs:857-945` handles Int range and Enum value changes concretely. Record, TaggedUnion, and nested List changes all return `REQUIRES_ANALYSIS`. No migration contract generation code exists.

**AL38 — Migration contracts cannot compare predicate strength** _(Migration)_
Claimed: All predicate changes classified as REQUIRES_ANALYSIS.
Verified: In `crates/cli/src/diff.rs`, Rule body changes (lines 694-698) and Operation precondition changes (lines 702-707) both return `REQUIRES_ANALYSIS`. No predicate strength comparison is implemented.

**AL39 — Migration contracts are self-contained** _(Migration)_
Claimed: Migration contracts do not import the contracts they migrate.
Verified: `crates/cli/src/diff.rs` implements purely structural diffing with no import generation. No migration contract generation code exists yet. The constraint holds by design.

**AL40 — Migration contracts are not composable in v1.0** _(Migration)_
Claimed: Transitive migration requires directly diffing endpoint versions' bundles.
Verified: No composition or chaining mechanism exists in the codebase. Diff operates on exactly two bundles.

**AL41 — Migration contracts are classification-only in v1.0** _(Migration)_
Claimed: No migration orchestration (Operations + Flows).
Verified: `tenor diff` outputs only classification (BREAKING/NON_BREAKING/REQUIRES_ANALYSIS). No Operation or Flow generation for migration orchestration exists.

**AL42 — Migration contract source bindings are conventionalized** _(Migration)_
Claimed: Migration contract Facts use conventionalized source bindings (`system: "tenor-diff"`).
Verified: No migration contract generation code exists. The constraint is documented but the feature is not yet implemented.

**AL43 — Migration contract determinism requires canonical ordering** _(Migration)_
Claimed: Must follow canonical ordering rules for deterministic output.
Verified: `crates/cli/src/diff.rs` uses `BTreeMap` for construct indexing (lines 146-158), sorted/deduplicated keys for field diffs (lines 210-212), and `BTreeSet` for set change classification (lines 747-805). The underlying diff mechanism ensures determinism. Migration contract generation is not yet implemented.

**AL43a — Construct-level classification does not model semantic interaction between co-occurring field changes** _(Migration, §18.5)_
Claimed: Supremum composition treats each field change independently.
Verified: Each `(kind, field, change_type)` triple maps to a single classification per §18.2 in `crates/cli/src/diff.rs`. No cross-field interaction modeling or semantic composition exists.

**AL44 — Flow compatibility does not model time-based constraints** _(Flow Migration, §18.6)_
Claimed: Timeout changes not accounted for in compatibility analysis.
Verified: No flow migration compatibility checker exists in the codebase. The three-layer analysis model (FMC1-FMC7) specified in §18.6 contains no timeout layer.

**AL45 — Recursive sub-flow compatibility depth is unbounded** _(Flow Migration, §18.6)_
Claimed: No depth limit on sub-flow recursion in compatibility analysis.
Verified: No flow migration compatibility checker exists. The spec permits arbitrary depth; acyclic DAG guarantees termination but no practical depth limit is imposed.

**AL46 — ParallelStep compatibility requires all branches compatible** _(Flow Migration, §18.6)_
Claimed: No partial parallel branch migration.
Verified: No flow migration compatibility checker exists. The spec at §18.6.6 requires all branches to pass independently.

**AL47 — Conservative data dependency analysis may produce false negatives** _(Flow Migration, §18.6)_
Claimed: Conservative analysis may reject safe migrations.
Verified: No flow migration compatibility checker exists. The spec defines conservative as required and aggressive as optional (§18.6.11).

**AL48 — Entity parent changes require transitive analysis** _(Flow Migration, §18.6)_
Claimed: Full impact of parent changes may require transitive analysis not specified in v1.0.
Verified: No flow migration compatibility checker exists. The spec defers full propagation analysis.

**AL49 — Reachable path computation uses v2's step graph** _(Flow Migration, §18.6)_
Claimed: Compatibility analysis uses v2's step graph for reachable path computation.
Verified: S6 path analysis exists at `crates/analyze/src/s6_flow_paths.rs` for single-contract path enumeration. Compatibility checking that would apply this to v1→v2 migration is not implemented.

**AL50 — User-defined verdict precedence deferred to v2** _(Rule, §7)_
Claimed: v1 contracts must use distinct VerdictType names per rule.
Verified: S8 enforcement at `crates/core/src/pass5_validate.rs:386-413` and `crates/analyze/src/s8_verdict_uniqueness.rs` strictly prevents two Rules from producing the same VerdictType.

**AL51 — Single contract per discovery endpoint** _(Contract Discovery, §19)_
Claimed: The well-known endpoint serves a single TenorManifest.
Verified: Route at `tenor-platform/crates/platform-serve/src/routes.rs:54` defines `GET /{contract_id}/.well-known/tenor`. Handler at `handlers.rs:229-266` returns a single contract manifest. No multi-contract discovery endpoint exists.

**AL53 — Text equality uses byte-exact comparison** _(BaseType, §4)_
Claimed: No Unicode normalization for text equality.
Verified: `compare_strings()` at `crates/eval/src/numeric.rs:300-310` uses direct Rust `==` on `&str`. No Unicode normalization, NFC/NFD conversion, or collation logic exists.

**AL54 — Sub-flow cross-version invocation unspecified** _(Flow Migration, §18.6)_
Claimed: v1 parent flow invoking v2 sub-flow not addressed.
Verified: `SubFlowStep` at `crates/core/src/ast.rs:264-273` has no version field. Flow execution at `crates/eval/src/flow.rs:400` invokes sub-flows by id only, no version checks.

**AL55 — Per-flow-type capability advertisement not supported in v1.1** _(Contract Discovery, §19)_
Claimed: migration_analysis_mode is per-executor, not per-flow.
Verified: Capabilities at `tenor-platform/crates/platform-serve/src/handlers.rs:250-252` returns executor-level `migration_analysis_mode: "conservative"`. No per-flow-type granularity.

**AL57 — Shared persona identity by exact id only** _(System, §12)_
Claimed: No persona aliasing mechanism.
Verified: C-SYS-16 validation at `crates/core/src/pass5_validate.rs:1073-1107` uses exact string id comparison. No aliasing or identity mapping exists.

**AL58 — Cross-contract triggers fire on terminal outcomes only** _(System, §12)_
Claimed: No intermediate-step triggering.
Verified: C-SYS-11 at `crates/core/src/pass5_validate.rs:1110-1162` validates trigger outcomes against `["success", "failure", "escalation"]` only.

**AL59 — System member file path resolution is relative** _(System, §12)_
Claimed: No absolute path resolution.
Verified: Import resolution in `crates/core/src/pass1_bundle.rs:22-79` resolves paths relative to the file's directory. No absolute path mechanism.

**AL60 — One System per file** _(System, §12)_
Claimed: Only one System declaration per file.
Verified: C-SYS-05 is specified. Parser at `crates/core/src/parser.rs:596-622` does not enforce single-System-per-file. Pass 1 `check_cross_file_dups()` checks cross-file duplicates but not per-file System count. The constraint is specified but enforcement is unclear.

**AL62 — Shared entity transition compatibility not checked** _(System, §12)_
Claimed: Only state set equality is checked, not transition compatibility.
Verified: C-SYS-14 (state set equality) is listed as a deep validation constraint. No transition compatibility analysis exists across contracts.

**AL63 — System-level migration policy not specified** _(System, §12, §18)_
Claimed: No formal migration obligations for Systems.
Verified: No System-level migration policy in either codebase. Migration analysis mode is executor-wide only.

**AL64 — Trigger persona authorization at non-OperationStep entry steps** _(System, §12)_
Claimed: Can't check persona auth at BranchStep/HandoffStep entries.
Verified: C-SYS-12 at `crates/core/src/pass5_validate.rs:1219-1246` checks persona authorization only when entry step is an `OperationStep`. BranchStep, HandoffStep, SubFlowStep entries are not checked.

**AL65 — System-level static analysis obligations not defined** _(System, §12, §16)_
Claimed: S1-S8 apply to individual contracts only.
Verified: No System-level static analysis constraints exist in either codebase. Only individual contract validation (S1-S8) is implemented.

**AL66 — Trigger at-most-once delivery mechanism is implementation-defined** _(System, §12)_
Claimed: Enforcement mechanism not prescribed.
Verified: No trigger delivery mechanism or deduplication logic exists in either codebase. The guarantee is stated but the mechanism is left to executors.

**AL67 — Cross-contract provenance retention policy not prescribed** _(System, §12)_
Claimed: Storage/retention not prescribed.
Verified: Provenance types exist at `crates/eval/src/provenance.rs` and conformance tests at `crates/storage/src/conformance/provenance.rs` cover single-contract provenance. No cross-contract or System-level provenance retention policy exists.

---

## Summary

- Total AL items: 44
- Resolved: 1 (AL52)
- Partially resolved: 4 (AL5, AL30, AL56, AL61)
- Still true: 39
