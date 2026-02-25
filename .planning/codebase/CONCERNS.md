# Codebase Concerns

**Analysis Date:** 2026-02-25

## Tech Debt

**Pre-release Status & API Stability:**
- Issue: Project is at v0.1.0 (pre-stable per STABILITY.md). Spec is v1.0 complete, but implementation may still evolve
- Files: `STABILITY.md`, `Cargo.toml` (workspace.package version = "0.1.0")
- Impact: Public consumers have no guarantees about breaking changes. Any crate using Tenor should pin to specific versions
- Fix approach: Monitor progress toward v1.0 declaration. Implement semantic versioning once stable. Consider breaking change indicators in changelog

**LSP Navigation Path Handling:**
- Issue: Path canonicalization and URI parsing in LSP has multiple fallback layers with potential edge cases
- Files: `crates/lsp/src/navigation.rs:174-188` (parse/canonicalize chain with fallback panics)
- Impact: Malformed URIs or paths on Windows could cause panic in LSP server. Client loses language server
- Fix approach: Replace `panic!()` at line 188 with graceful error handling returning a default URI. Add comprehensive URI test suite for Windows paths

**Unused Clone Operations in Flow Execution:**
- Issue: 141 instances of `.clone()` throughout eval crate, many in hot paths like flow steps
- Files: `crates/eval/src/flow.rs` (lines 289, 418, 422, etc.), `crates/eval/src/operation.rs`, `crates/eval/src/rules.rs`
- Impact: Each clone of complex types (FactSet, VerdictSet, EntityStateMap) wastes CPU/memory. Compounds with nested flows and parallel branches
- Fix approach: Profile memory usage in large flows (1000+ steps). Consider cow-wrapped types for immutable snapshots. Use references where ownership not required

**Massive Parser and Evaluation Files:**
- Issue: Single-file implementations for complex functionality: parser.rs (1820 lines), flow.rs (1518 lines), types.rs (1825 lines)
- Files: `crates/core/src/parser.rs`, `crates/eval/src/flow.rs`, `crates/eval/src/types.rs`
- Impact: Hard to navigate, test individual modules, and isolate bugs. Makes code reviews slow
- Fix approach: Break parser into lexer/parser/ast modules. Move flow variants into separate handling modules. Split types into value types and contract types

## Known Bugs

**LSP Server URI Parsing Panic:**
- Symptoms: LSP server panics on certain URI formats, particularly Windows UNC paths or non-ASCII characters
- Files: `crates/lsp/src/navigation.rs:187-191`
- Trigger: Provide a workspace folder with unusual path format or non-standard characters to LSP initialize
- Workaround: Use ASCII-only normalized paths in workspace roots

**Sub-flow Snapshot Inheritance Error Message:**
- Symptoms: If a sub-flow references undefined operations/flows, error message reports wrong flow ID
- Files: `crates/eval/src/flow.rs:412-413` (message reports sub_flow_id not parent flow context)
- Trigger: Execute nested flow with invalid sub-flow reference
- Workaround: Check all sub-flow IDs match contract operations before execution

## Security Considerations

**Unvalidated JSON Deserialization:**
- Risk: Interchange JSON is deserialized without bounds checking. Extremely large lists or nested structures could cause OOM
- Files: `crates/interchange/src/deserialize.rs`, `crates/eval/src/types.rs:205-291`
- Current mitigation: List max_length enforced at eval time, but no JSON-level limits
- Recommendations: Add deserializer bounds (max list size 1M, max record depth 100). Validate during from_interchange() before creating Contract

**Entity State Mutation During Flow Execution:**
- Risk: While snapshot is frozen, entity states are mutably borrowed and modified. If parallel branches modify shared state without proper isolation, race condition possible
- Files: `crates/eval/src/flow.rs:499` (branch_entity_states cloned but parent state unmerged on error), `crates/eval/src/operation.rs` (effects applied directly)
- Current mitigation: Branch states are cloned at entry, merged on success. But failed branch changes are discarded without logging
- Recommendations: Add audit trail for state rollbacks. Test concurrent entity state modifications with parallel branches. Document isolation guarantees

**Numeric Overflow in Precision Checks:**
- Risk: Building upper bounds with checked_mul in check_precision() at line 95-103. If precision > 28, loop multiplies by 10 up to 28 times. Could still theoretically overflow
- Files: `crates/eval/src/numeric.rs:70-116`
- Current mitigation: Uses checked_mul with explicit error
- Recommendations: Add explicit upper bounds test. Document assumption that precision <= 38 (Decimal max). Add test cases for precision(99, 0)

## Performance Bottlenecks

**Flow Step Lookup Creates Index Every Execution:**
- Problem: execute_flow() rebuilds step_index and op_index as BTreeMap/HashMap every time
- Files: `crates/eval/src/flow.rs:226-237` (index creation in execute_flow)
- Cause: No caching of indices. With large flows (100+ steps), creates indices per branch execution
- Improvement path: Pre-compute step indices during Contract deserialization. Pass index as immutable reference to avoid rebuild. Cache operation index in Contract struct

**FactSet/VerdictSet Cloning in Branches:**
- Problem: Each parallel branch clones the entire frozen snapshot (facts + verdicts) and entity states
- Files: `crates/eval/src/flow.rs:499` (branch gets full clone of entity_states)
- Cause: BTreeMap/HashMap clone is O(n). With large fact/verdict sets, this becomes expensive at 10+ branches
- Improvement path: Use Arc<> for immutable snapshot. Only branch_entity_states needs cloning. Test with 100-branch parallel step

**Verbose Serialization with Sorted Keys:**
- Problem: Pass 6 serializes all JSON with lexicographically sorted keys (interchange spec requirement). For large bundles with 1000+ constructs, sorting is O(n log n)
- Files: `crates/core/src/pass6_serialize.rs:790-791` (BTreeMap ensures sorted but could be deferred)
- Cause: Specification compliance requires exact key ordering. No lazy evaluation
- Improvement path: Accept sorted output as spec requirement. Profile to see if actual bottleneck. If needed, use serde-json ordering features

## Fragile Areas

**LSP Document Synchronization State Machine:**
- Files: `crates/lsp/src/document.rs`, `crates/lsp/src/server.rs` (request/notification dispatch)
- Why fragile: Multiple overlapping state transitions (DidOpen, DidChange, DidSave, DidClose) with no explicit state validation. Out-of-order notifications could cause stale diagnostics
- Safe modification: Add FSM validation layer that enforces allowed transitions (Open -> Change -> Save -> Close). Add state assertions. Test with rapid fire notifications
- Test coverage: Gaps in DidClose after DidChange without DidSave. No tests for duplicate DidOpen

**Parser Error Recovery:**
- Files: `crates/core/src/parser.rs` (no recovery from syntax errors beyond top-level construct boundaries)
- Why fragile: Single parsing error in multi-file bundle stops elaboration. No partial parsing fallback
- Safe modification: Parser currently validates strictly. Avoid adding "optional" constructs that change parsing flow. Always explicit bounds
- Test coverage: Good negative test suite in conformance/. But no incremental parsing tests

**Parallel Branch Outcome Merging:**
- Files: `crates/eval/src/flow.rs:535-630` (parallel step join logic)
- Why fragile: Join policy (All/Any/First) merges branch outcomes without detecting conflicting entity state changes. Two branches modifying same entity without explicit lock will silently merge
- Safe modification: Add explicit entity conflict detection. Build entity change map per branch, check for overlaps before merge. Fail flow if conflicting changes detected
- Test coverage: Gap - no tests for two branches modifying the same entity with different final states

**Flow Entry Point Validation:**
- Files: `crates/eval/src/flow.rs:215-262` (entry step lookup in step_index)
- Why fragile: If flow.entry doesn't exist in step_index, returns DeserializeError. But if steps array is incomplete during deserialization, entry could be silently skipped
- Safe modification: Validate during Flow deserialization that entry exists in steps array. Add explicit entry validation in Contract::from_json
- Test coverage: Missing test for flow with entry ID that doesn't match any step

## Scaling Limits

**Flow Step Execution Recursion:**
- Current capacity: Hardcoded max_steps = 1000 (line 243 in flow.rs). Configurable via parameter but defaults conservatively
- Limit: Flows with cycles or long chains hit limit. Parallel branches multiply step count (10 branches with 100 steps each = 1000 total)
- Scaling path: Make max_steps configurable via CLI/API. Add adaptive limit based on available memory. For truly long flows, require explicit per-step instrumentation

**Interchange Bundle Size:**
- Current capacity: No limits on construct count or total JSON size. A contract with 10,000 entities/rules deserializes but becomes slow
- Limit: Deserialization and indexing become O(n) bottlenecks. BTreeMap/HashMap construction dominates with 10k+ constructs
- Scaling path: Add streaming deserializer for large bundles. Lazy-load constructs by kind. Partition indices by module

**Conformance Test Suite Execution:**
- Current capacity: 113 .tenor files in conformance/. Full suite runs in ~5s on modern hardware
- Limit: Adding domain contracts doubles test time. No parallelization
- Scaling path: Implement test parallelization (currently runner.rs is synchronous). Cache elaboration results. Use rayon for parallel conformance runs

## Dependencies at Risk

**rust_decimal Version Lock:**
- Risk: Pinned to v1.36. Numeric precision is critical to spec compliance. Any future major version bump requires full precision audit
- Impact: If decimal behavior changes, contract evaluation could produce different results
- Migration plan: When v2.x released, audit all numeric test cases. Verify rounding strategy (MidpointNearestEven) still default. Test precision bounds (28 digits) behavior

**LSP Types & Server Versions:**
- Risk: lsp-types 0.97 and lsp-server 0.7 are mature but may diverge from LSP spec evolution
- Impact: If new LSP features needed (e.g., inline values, type hints), would need major version bumps
- Migration plan: Monitor LSP spec updates. Consider upgrading lsp-types yearly. Test with latest VS Code LSP client

## Missing Critical Features

**No Incremental Elaboration:**
- Problem: Full contract re-elaboration on every change. For large bundles (1000+ constructs), this is slow for IDE usage
- Blocks: Real-time LSP diagnostics, hot-reload development workflow, IDE performance

**No Contract Versioning/Backwards Compatibility:**
- Problem: If contract schema changes, no way to express version compatibility. Evaluators must use exact bundle version
- Blocks: API evolution, safe schema migration, multi-version support

**Flow Debugging & Tracing:**
- Problem: No execution trace output. Flow errors report only final failure, not step-by-step execution for diagnosis
- Blocks: Diagnosing complex flow bugs, understanding entity state evolution, auditing step execution path

**Persona Impersonation Audit Trail:**
- Problem: Persona changes during handoff steps recorded but not formatted for audit purposes
- Blocks: Compliance reporting, persona chain audit, detecting unauthorized handoffs

## Test Coverage Gaps

**Cross-File Import Cycles:**
- What's not tested: Cycle detection with more than 2 files in cycle (A -> B -> C -> A)
- Files: `crates/core/src/pass1_bundle.rs` (cycle detection tested in conformance/negative but needs exhaustive cross_file variants)
- Risk: Complex import cycles might slip through. Error message could be unintelligible
- Priority: Medium - cycle detection is fairly robust, but message clarity could be better

**Parallel Branch State Conflicts:**
- What's not tested: Two parallel branches modifying the same entity to different states
- Files: `crates/eval/src/flow.rs` (parallel execution and merge logic)
- Risk: Silent data loss on branch merge. Entity ends up in unexpected state
- Priority: High - correctness issue

**Numeric Precision at Extremes:**
- What's not tested: Decimal(precision: 38, scale: 37) with operations that produce all-fractional values
- Files: `crates/eval/src/numeric.rs`, conformance/numeric/ (has some but not exhaustive)
- Risk: Edge case in precision checking could allow invalid values through
- Priority: Medium - spec compliance risk

**LSP Concurrent Request Handling:**
- What's not tested: Multiple completion/hover requests in rapid succession while document is changing
- Files: `crates/lsp/src/server.rs` (synchronous single-threaded design)
- Risk: Race condition in document state if requests arrive out of order
- Priority: Low - single-threaded design avoids races, but document state could become stale

**Entity Transition Invalid States:**
- What's not tested: Effect transitions entity to state that doesn't exist in entity definition
- Files: `crates/core/src/pass5_validate.rs` (validate_entity checks transitions but eval doesn't re-check)
- Risk: If bundle was elaborated on older version, eval accepts invalid transitions
- Priority: Medium - eval should re-validate state names

---

*Concerns audit: 2026-02-25*
