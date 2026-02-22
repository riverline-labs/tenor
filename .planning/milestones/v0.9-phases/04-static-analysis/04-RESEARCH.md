# Phase 4: Static Analysis — Research

**Researched:** 2026-02-22
**Status:** Complete

## Spec Requirements Summary

Section 15 of TENOR.md defines eight static analyses (S1-S8). Each derives information from a contract alone, without execution. S8 is already enforced in Pass 5 (verdict uniqueness). S3b is explicitly qualified as "not always computationally feasible" and is out of scope.

### S1 — Complete State Space
- For each Entity, enumerate all declared states.
- Straightforward extraction from interchange JSON: `entity.states` array.
- Output: `Map<EntityId, Vec<State>>`

### S2 — Reachable States
- For each Entity, derive states reachable from initial via declared transition relation.
- Graph traversal: BFS/DFS from `entity.initial` following `entity.transitions`.
- Output: `Map<EntityId, ReachabilityResult>` where result includes reachable states AND unreachable states (dead states).
- Dead states (declared but unreachable from initial) are a finding worth reporting.

### S3a — Structural Admissibility Per State
- For each Entity state and persona, find Operations whose preconditions are structurally satisfiable AND whose effects include a transition from that state.
- Type-level analysis only: check if fact types in preconditions are compatible with literal comparison values.
- E.g., `Enum(["pending","confirmed"]) == "approved"` is structurally unsatisfiable by type inspection.
- O(|expression tree|) per precondition -- always computationally feasible.
- Output: `Map<(EntityId, State, PersonaId), Vec<OperationId>>`

### S4 — Authority Topology
- For any Persona P and Entity state S, derive which Operations P can invoke in S.
- Cross-reference: `operation.allowed_personas` contains P, AND operation has an effect transitioning from S.
- Whether P can cause a transition S->S' is answerable (path analysis through S3a results).
- Output: `Map<PersonaId, AuthorityMap>` where AuthorityMap = `Map<EntityId, Map<State, Vec<OperationId>>>`

### S5 — Verdict and Outcome Space
- Enumerate all possible verdict types from rules: `rule.body.produce.verdict_type`.
- Enumerate all possible outcomes for each Operation: `operation.outcomes` (if present).
- Output: `VerdictSpace { verdict_types, operation_outcomes }`

### S6 — Flow Path Enumeration
- For each Flow, enumerate all possible execution paths.
- Track: personas at each step, Operation outcomes at each OperationStep, entity states reachable, terminal outcomes.
- OperationStep outcome handling is exhaustive (Section 11.5) -- set of paths = product of declared outcomes.
- BranchStep creates two paths (true/false). ParallelStep branches and rejoins.
- SubFlowStep recurses into sub-flow paths.
- Output: `Map<FlowId, FlowPaths>` where FlowPaths contains all enumerated paths with per-step metadata.
- Must handle: step cycles (detected in Pass 5, but defend against), maximum depth bounds.

### S7 — Evaluation Complexity Bounds
- PredicateExpression: count nodes, classify quantifiers (forall is O(n) per list element).
- Flow max execution depth: longest path in step graph (from S6).
- Output: `Map<ExprId, ComplexityBound>` and `Map<FlowId, DepthBound>`

### S8 — Verdict Uniqueness
- Already enforced in Pass 5 of the elaborator.
- Analyzer can confirm "pre-verified by elaborator" rather than re-implementing.
- No new code needed in tenor-analyze; just note it in the report.

## Codebase Analysis

### Existing Infrastructure

**tenor-analyze crate:** Stub with `tenor-core` dependency. Needs `serde_json` and `serde` added.

**Interchange JSON structure** (from interchange-schema.json):
- Bundle: `{ constructs: [...], id, kind, tenor, tenor_version }`
- Each construct: `{ kind, id, ... }` -- kind discriminator selects type
- Entity: `{ kind: "Entity", id, states: [...], initial, transitions: [{from, to}], parent? }`
- Rule: `{ kind: "Rule", id, stratum, body: {when: <predexpr>, produce: {verdict_type, payload}} }`
- Operation: `{ kind: "Operation", id, allowed_personas: [...], precondition?, effects: [...], outcomes?, error_contract? }`
- Flow: `{ kind: "Flow", id, entry, steps: [...], snapshot }`

**Existing consumers of interchange JSON:**
- `tenor-eval` (`crates/eval/src/`) -- parses bundle JSON into its own types, evaluates. Good pattern to follow.
- `diff.rs` (in `crates/cli/src/diff.rs`) -- operates on raw `serde_json::Value`, keyed by (kind, id). BundleDiff struct pattern.

**CLI pattern** (`crates/cli/src/main.rs`):
- Subcommands via clap derive, existing `check` stub returns exit 2 (not implemented).
- `cmd_elaborate` shows the elaborate-then-process pattern that `cmd_check` needs.
- `OutputFormat::Text` / `OutputFormat::Json` global flag.

### Breaking Change Analysis (`tenor diff --breaking`)

**Existing diff infrastructure:**
- `BundleDiff` with `added`, `removed`, `changed` construct lists
- `ConstructChange` with `kind`, `id`, and `Vec<FieldDiff>`
- `FieldDiff` with `field`, `before`, `after`

**Section 17.2 taxonomy:**
- Classification is a pure function: `classify(kind, field, change_type) -> Severity`
- change_type for field changes: add_value, remove_value, change_value (with sub-types like widen/narrow)
- Each (construct_kind, field, change_type) triple has a fixed classification
- Some classifications are REQUIRES_ANALYSIS -- need S1-S7 to resolve

**What `--breaking` needs:**
1. Compute diff (existing)
2. For each diff entry, classify using taxonomy table
3. For REQUIRES_ANALYSIS entries, run relevant S1-S7 analyses on both bundles
4. Final classification: BREAKING, NON_BREAKING, or REQUIRES_ANALYSIS (if static analysis can't resolve)

### Test Fixtures

**Existing conformance fixtures:**
- `integration_escrow.tenor` -- comprehensive contract with Entities, Facts, Rules, Operations, Flows, Personas
- The worked example in Appendix D provides expected S1, S2, S3, S4, S6 derivations for this contract
- `entity_basic.tenor`, `flow_basic.tenor`, `operation_basic.tenor` -- simpler contracts for unit testing

**Test strategy:**
- Use `integration_escrow` as primary known-good fixture (spec provides expected outputs)
- Create dedicated analysis fixtures for edge cases (dead states, unreachable strata, etc.)
- Unit tests per analysis module + integration tests via `tenor check`

## Wave Structure Recommendation

The roadmap suggests 8 plans. Given dependency analysis:

- **Wave 1:** Plan 01 (crate structure + S1) + Plan 02 (S2 + S3a) -- S1 is independent, S2 depends on S1's entity extraction, S3a depends on S2's reachability
- **Wave 2:** Plan 03 (S4 + S5) -- S4 depends on S3a for per-state admissibility, S5 is independent
- **Wave 3:** Plan 04 (S6 + S7) -- S6 depends on S5's outcome space, S7 depends on S6's path enumeration
- **Wave 4:** Plan 05 (S8) -- trivial, can run in parallel with Plan 06
- **Wave 4:** Plan 06 (structured output + `tenor check` CLI) -- depends on all S1-S7 being available
- **Wave 5:** Plan 07 (test suite) -- depends on Plan 06 for CLI integration
- **Wave 5:** Plan 08 (`tenor diff --breaking`) -- depends on all analyses and diff infrastructure

**Optimization:** Plans 01+02 can be Wave 1 (S1 is trivial, S2 and S3a build on entity extraction). Plans 03+05 can be Wave 2 (S4/S5 + S8 are independent). Plan 04 is Wave 3. Plan 06 is Wave 4. Plans 07+08 are Wave 5.

## Key Technical Decisions

1. **Analyzer consumes interchange JSON** (not raw AST) -- established pattern from tenor-eval
2. **Each analysis is a separate module** in tenor-analyze with its own result type
3. **AnalysisReport aggregates** all individual results -- CLI `tenor check` calls `analyze(bundle)` and formats
4. **Breaking change classifier** is a pure lookup function applied to BundleDiff entries
5. **S8 is pre-verified** by Pass 5 -- no new analysis code, just report confirmation
6. **S3b is out of scope** -- only S3a (structural admissibility) implemented

## Risks and Mitigations

- **S6 flow path explosion:** Flows with many outcomes per step create exponential paths. Mitigation: cap enumeration at 10,000 paths, report "path limit exceeded" rather than OOM.
- **S3a type-level satisfiability:** Need to walk predicate expression trees and check type compatibility. The spec says O(|tree|) per precondition -- manageable.
- **Breaking change taxonomy completeness:** The spec says taxonomy is exhaustive (MI4). Must implement every (kind, field, change_type) triple. This is a large lookup table but mechanically straightforward.
- **tenor-analyze dependency on serde_json:** Needs adding to Cargo.toml (currently only has tenor-core).

---

*Phase: 04-static-analysis*
*Researched: 2026-02-22*
