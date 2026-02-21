# Codebase Concerns

**Analysis Date:** 2026-02-21

## Pre-Release Status

**Current State:** Pre-alpha (v0.3). This is a pre-stable specification and implementation. The project status explicitly warns against production use. See `STABILITY.md`.

**Test Coverage:** 47/47 conformance suite tests passing as of v0.3.

**Scope:** This codebase contains:
- Tenor DSL specification (`docs/TENOR.md`)
- Reference elaborator in Rust (`elaborator/src/`)
- Conformance test suite (`conformance/`)

**No evaluator, executor, or code generation yet.** See ROADMAP.md for Phase 1–5 work.

---

## File Size and Complexity

**elaborate.rs — Primary Bottleneck:**
- **Location:** `elaborator/src/elaborate.rs`
- **Lines:** 2,066
- **Functions:** 58 (see detailed list below)
- **Impact:** Monolithic single-file module containing all 6 elaboration passes + validation + serialization
- **Risk:** High cognitive load. Changes in one pass affect scope and line-number tracking across all downstream passes
- **Safe modification:** Use `grep -n "^fn "` to understand function boundaries. Each pass is roughly sequential; mutation in Pass N can silently break Passes N+1 through N+6

**Functions in elaborate.rs (major ones):**
- `load_bundle()` / `load_file()` — Pass 0+1: import resolution, cycle detection
- `build_index()` / `check_cross_file_dups()` — Pass 2: construct indexing, duplicate detection
- `build_type_env()` — Pass 3: type environment construction, TypeDecl cycle detection
- `resolve_types()` / `resolve_typedecl()` — Pass 4a: TypeRef → concrete BaseType resolution
- `type_check_rules()` / `type_check_expr()` — Pass 4b: expression type-checking
- `validate()` / `validate_entity()` / `validate_operation()` / `validate_flow()` — Pass 5: structural validation
- `serialize()` / `serialize_type()` / `serialize_expr()` / `serialize_step()` — Pass 6: JSON output

**Recommendation:** Break elaborate.rs into modules:
- `passes/pass0.rs` (lex)
- `passes/pass1.rs` (import + bundle)
- `passes/pass2.rs` (indexing)
- `passes/pass3.rs` (type env)
- `passes/pass4.rs` (type-check)
- `passes/pass5.rs` (validate)
- `passes/pass6.rs` (serialize)

No immediate blocking concern since tests pass, but refactoring before feature work is cheaper than after.

---

## Technical Debt

**Missing Features (Deferred to v2):**

1. **P5 — Shared Type Library**
   - **Issue:** Record and TaggedUnion types are per-contract only in v0.3
   - **Files:** `docs/TENOR.md` (§16, §AL4)
   - **Impact:** No cross-contract type reuse. Requires inter-contract elaboration in v1.0+
   - **Current Mitigation:** Design is constrained; full scope deferred

2. **P7 — Operation Outcome Typing**
   - **Issue:** Named outcome types on Operations are not yet part of the language
   - **Files:** `docs/TENOR.md` (§16)
   - **Impact:** Outcome classification happens in Flow (§AL13), not in Operation construct
   - **Current Mitigation:** Spec explicitly documents this as deferred; Flow-side routing is the workaround

**Specification Ambiguity (Intentional Design Decisions):**

These are documented limitations in `docs/TENOR.md` Appendix A, not bugs:

- **AL1 — Fact ground property boundary:** Facts must come from external sources, not from internal evaluations. Executors enforce this.
- **AL11 — Operation source-state validation:** Not language-enforced; executor obligation.
- **AL12 — Operation atomicity:** Executor obligation, not language-enforceable.
- **AL21 — Parallel branch cancellation:** `ParallelStep` runs all branches to completion; early-exit not supported.
- **AL22 — Post-parallel verdict re-evaluation:** Frozen verdict semantics within parallel blocks; new Flow required for re-evaluation.

---

## Elaborator Code Quality Concerns

**Error Message Line Number Accuracy:**

`elaborate.rs` tracks line numbers throughout all 6 passes. Concerns:

1. **Pass 1 Import Cycle Detection (`load_file()`)**
   - Files: `elaborator/src/elaborate.rs` lines 119–128
   - Current: Cycle path reported with arrows, but line reported as 0 (file-open line)
   - Correct: Line should be the line of the `import` statement that closes the cycle
   - **Status:** FIXED in recent commits; current tests passing

2. **Pass 3 TypeDecl Cycle Detection**
   - Files: `elaborator/src/elaborate.rs` `detect_typedecl_cycle()`
   - Current: Reports the entry-point TypeDecl, not the one that closes the cycle
   - Expected: Should report the construct_id and line of the TypeDecl that closes the cycle
   - **Status:** FIXED; v0.3 all tests passing

3. **Pass 5 Entity Validation**
   - Files: `elaborator/src/elaborate.rs` `validate_entity()`
   - Concern: Line numbers for `initial:` and `transitions:` field errors — must point to field, not `entity` keyword
   - **Status:** FIXED; v0.3 all tests passing

4. **Pass 5 Verdict Stratum Violations**
   - Files: `elaborator/src/elaborate.rs` `validate_verdict_refs_in_expr()`
   - Concern: Line must point to the `verdict_present()` reference, not the rule keyword
   - **Status:** FIXED; v0.3 all tests passing

**All known line-number issues are fixed in current codebase. No regressions detected in conformance suite.**

---

## Type-Checking Gaps

**Currently Implemented:**

- Fact type validation (Pass 4)
- Verdict presence checking (Pass 5)
- Literal type coercion (Pass 4)
- Numeric type promotion (Pass 4)

**Type-Checking Coverage:**

All 6 type-checking tests in negative/pass4/ are now passing (v0.3):
- `unresolved_fact_ref` — fact references must be declared
- `bool_int_comparison` — Bool only supports = and ≠
- `cross_currency_compare` — Money comparisons require matching currency
- `quantifier_scalar_domain` — Quantifier domain must be List-typed
- `var_var_in_predexpr` — Variable × variable not allowed in PredicateExpression
- `rule_body_var_var_range_exceeded` — Product range must fit in declared verdict type

**Status:** All type checks fully implemented and tested.

---

## Numeric Type System Concerns

**Fixed-Point Decimal Precision:**

- **Files:** `elaborator/src/elaborate.rs` serialization; `docs/TENOR.md` §11 NumericModel
- **Risk:** Decimal values serialize using declared type's (precision, scale), not inferred from literal
- **Example:** `Decimal(precision: 10, scale: 2)` with value `"3.14"` serializes as `{"kind": "decimal_value", "precision": 10, "scale": 2, "value": "3.14"}`
- **Current Implementation:** Correct per conformance tests
- **Tests:** `numeric/decimal_large_precise`, `numeric/decimal_trailing_zero` both passing

**Numeric Promotion:**

- **Files:** `docs/TENOR.md` §11.3, conformance tests in `promotion/`
- **Rule:** Int × Decimal → Decimal; cross-type comparisons emit `comparison_type` in interchange
- **Current Implementation:** Correct
- **Tests:** `promotion/int_decimal_comparison`, `promotion/int_literal_multiply` both passing

**No regressions detected in numeric handling.**

---

## Parser Completeness

**Complete Feature Set (v0.3):**

All language constructs are now recognized:

1. **Fact, Entity, Rule, Operation, Flow** — fully implemented
2. **TypeDecl** — type aliases with cycle detection
3. **OperationStep, ConditionalStep** — both step kinds
4. **SubFlowStep** — cross-flow step invocation with failure handling
5. **ParallelStep** — fork/join with branch entity conflict detection
6. **All BaseTypes** — Bool, Int, Decimal, Text, Date, DateTime, Money, Duration, Enum, Record, TaggedUnion, List
7. **Arrow operators** — both `→` and `->` recognized, comma also accepted as separator

**Parser Coverage:**

- `positive/integration_escrow` — comprehensive integration test, passing
- `parallel/conflict_direct`, `parallel/conflict_transitive` — ParallelStep entity conflict detection, passing
- `cross_file/bundle` — multi-file import, passing

**No parser gaps detected.**

---

## Validation Completeness

**Pass 5 Structural Validation:**

All 14 validation tests in negative/pass5/ passing:

1. `entity_initial_not_in_states` — initial state must be declared
2. `entity_transition_unknown_endpoint` — transition endpoints must be declared
3. `entity_hierarchy_cycle` — entity parent relationships must be acyclic
4. `rule_negative_stratum` — stratum must be non-negative
5. `rule_forward_stratum_ref` — stratum violation: lower rules cannot reference higher-stratum verdicts
6. `operation_empty_personas` — operations must have at least one allowed persona
7. `operation_effect_unknown_entity` — effects must reference declared entities
8. `operation_effect_unknown_transition` — effects must reference declared transitions
9. `flow_missing_entry` — entry step must be declared
10. `flow_unresolved_step_ref` — step references must be declared
11. `flow_step_cycle` — step graph must be acyclic
12. `flow_reference_cycle` — cross-flow SubFlowStep references must be acyclic
13. `flow_missing_failure_handler` — OperationStep must have failure handler
14. `unresolved_verdict_ref` — verdict must be produced by some rule

**Parallel Branch Conflict Detection:**

- `conflict_direct` — branches directly affecting same entity, passing
- `conflict_transitive` — branches indirectly affecting same entity through SubFlowStep, passing

**No validation gaps detected.**

---

## Known Limitations (Intentional Design)

**Flow Execution Model (AL21, AL22):**

**Concern:** ParallelStep has non-obvious semantics around verdict freezing:

- **AL21:** All branches run to completion; no early cancellation on any-branch-failure
- **AL22:** Frozen verdict semantics within parallel blocks; verdict re-evaluation requires initiating a new Flow after parallel completion

**Implication:** Executors must be careful not to evaluate verdicts against parallel branch intermediate states. Contract authors must place verdict-dependent operations outside parallel blocks.

**Mitigation:** Spec clearly documents both (§10.2, Appendix A). No code change needed; documentation is sufficient.

**Duration Semantics (AL18):**

- **Risk:** Duration("day") = exactly 86,400 seconds
- **No DST, leap-second, or calendar-span support**
- **Mitigation:** Adapters must convert calendar times to Duration before Fact assertion
- **Status:** By design; spec is explicit

---

## Build and Deployment

**Cargo Configuration:**

- **File:** `elaborator/Cargo.toml`
- **Dependencies:** serde, serde_json only (minimal)
- **Binary:** `tenor-elaborator` (CLI tool)
- **Commands:** `run` (conformance suite), `elaborate` (single file)

**No dependency vulnerabilities detected.** Only stable, widely-used crates.

**Platform Support:** Rust 2021 edition; tested on macOS, should work on Linux/Windows.

---

## Documentation Gaps

**Missing Documentation:**

1. **Authoring Guide** — No examples beyond worked example in Appendix C
   - **Planned:** Phase 5 deliverable per ROADMAP.md
   - **Current:** Spec is formal and implementation-focused, not author-facing

2. **Executor Implementation Guide** — Reference executor does not exist yet
   - **Planned:** Phase 2 deliverable
   - **Current:** Spec defines evaluation model, but no reference implementation

3. **Code Generation Guide** — No generator yet
   - **Planned:** Phase 4 deliverable

4. **VS Code Extension** — Not started
   - **Planned:** Phase 5 deliverable

**No immediate code concerns; these are roadmap items, not issues.**

---

## Static Analysis Obligations (S1–S7)

**Current Status:** Elaborator is a conformance-tested DSL-to-JSON compiler. Static analysis subsystem (`tenor check` command, §12 and §14 in spec) does not exist yet.

**Missing Analyses:**

- **S1:** Reachable entity states
- **S2:** Authority topology (persona per state)
- **S3:** Flow path enumeration
- **S4:** Verdict space
- **S5:** Complexity bounds
- **S6:** Unreachable verdict detection
- **S7:** Stratification validation (beyond what elaborator checks)

**Implication:** `tenor check` command does not yet exist. Phase 2 deliverable per ROADMAP.md.

---

## Test Suite Coverage

**47/47 Passing:**

- Positive tests: `positive/` + `cross_file/` + `parallel/` + `numeric/` + `promotion/` + `shorthand/`
- Negative tests (by pass): `negative/pass0/` through `negative/pass5/`, plus `parallel/` entity conflict tests

**Coverage Gaps:**

1. **No fuzzing or property-based testing** — conformance tests are hand-written
2. **No performance/scalability tests** — no suite for large contracts
3. **No end-to-end evaluator tests** — elaborator is tested; evaluator is not written yet

**Recommendation:** Add performance benchmarks (contract size vs elaboration time) before code generation begins in Phase 4.

---

## Pre-Release Readiness Checklist

Per `ROADMAP.md` §1.0 declaration criteria:

- [x] Core constructs canonicalized
- [x] Syntax defined (Tenor v1.0)
- [x] Elaborator conformance suite (47/47)
- [ ] **Persona declaration** — P1 task, not yet in spec
- [ ] **P7 outcome typing** — deferred to v2
- [ ] **P5 type library** — deferred to v2
- [ ] CLI with subcommands (`tenor elaborate`, `tenor check`, `tenor eval`, etc.)
- [ ] Evaluator wired and conformance-tested
- [ ] Static analyzer (S1–S7)
- [ ] 5+ real contracts validated end-to-end
- [ ] Code generation for TypeScript
- [ ] VS Code extension

**Status:** Elaborator is feature-complete for v0.3. Remaining work is phases 2–5.

---

## Summary of Actionable Concerns

**No blocking issues in current codebase.** All tests pass.

**Recommended pre-1.0 work (in roadmap order):**

1. **Phase 1 — Persona declaration:** Add to spec and elaborator before Phase 2 depends on it
2. **Phase 2 — Evaluator + Static analyzer:** Largest implementation effort
3. **Phase 3 — Domain validation:** Author real contracts; discover spec gaps
4. **Phase 4 — Code generation:** After Phase 3 validates language stability
5. **Phase 5 — Developer experience:** Polish (VS Code, docs) last

**Code quality:** One monolithic `elaborate.rs` file should be modularized by pass before adding major features. No technical debt blocking roadmap; refactor during Phase 2.

---

*Concerns audit: 2026-02-21*
