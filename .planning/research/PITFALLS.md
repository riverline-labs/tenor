# Pitfalls Research

**Domain:** DSL / behavioral contract language toolchain (v0.3 elaborator to 1.0 full toolchain)
**Researched:** 2026-02-21
**Confidence:** HIGH (code-examined, spec-examined, domain-verified)

## Critical Pitfalls

### Pitfall 1: Spec Drift Under Tooling Pressure

**What goes wrong:**
Once CLI, evaluator, static analyzer, and code generation are all in flight, pressure to "just make it work" causes the spec (TENOR.md) and the implementation to diverge. Someone adds a feature to the elaborator to unblock code generation, forgets to update the spec. The interchange format grows a field that the spec does not describe. The evaluator assumes semantics the spec never codified. Within months the spec becomes aspirational rather than authoritative, and the project loses its core value proposition: a single formal description that all tooling is derived from.

**Why it happens:**
Tenor's spec is 1,780 lines of dense formal text. Updating it is slow and careful work. Implementation changes are fast. When a code generation target needs a piece of information the interchange does not carry, the temptation is to add it to the serializer and move on. When the evaluator needs to handle an edge case, the temptation is to decide the behavior in code and document it later.

**How to avoid:**
Enforce a "spec-first" discipline: every change to interchange schema, evaluation semantics, or construct behavior must have a corresponding spec section written **before** the implementation lands. In practice this means:
- Spec PR must merge before or simultaneously with implementation PR.
- The conformance suite is the executable spec. Any behavior not covered by a conformance test does not exist.
- Interchange schema should be formally defined (JSON Schema or similar) and generated from the spec section, not reverse-engineered from what the serializer produces.

**Warning signs:**
- Someone says "I'll update the spec later."
- A conformance test is added without a corresponding spec section justifying the behavior.
- The interchange JSON contains fields that do not appear in any spec section.
- The evaluator handles a case that Section 13 (Evaluation Model) does not describe.

**Phase to address:**
Phase 1 (Spec Completion). The spec must be complete and frozen for 1.0 constructs before tooling phases begin. P7 (outcome typing) and persona declaration must be fully specified before any evaluator or code generator touches them.

---

### Pitfall 2: Monolithic Elaborate.rs Becomes Load-Bearing Spaghetti

**What goes wrong:**
The current `elaborate.rs` is 2,066 lines containing all six elaboration passes in a single file. It works (47/47 tests), but adding new constructs (persona declarations, P7 outcome types), implementing the evaluator that shares type structures, building an LSP server that needs incremental re-elaboration, and creating a code generator that reads the typed AST all require reaching into this file. Without modularization, every new tool couples to the internal structure of `elaborate.rs`. The file grows to 4,000+ lines. Refactoring becomes terrifying because the conformance suite only tests final output, not intermediate representations.

**Why it happens:**
The elaborator was built as a proof of concept with a linear pipeline. That was the right call for v0.3. But production tooling needs to:
- Share the type environment (Pass 3 output) with the evaluator, static analyzer, and code generator.
- Share the construct index (Pass 2 output) with the static analyzer.
- Run partial re-elaboration for the LSP server (re-run Pass 4+ when a single file changes).
- Surface intermediate pass results for debugging and tooling.

None of this is possible when the passes are private functions in a single file with no defined intermediate data types.

**How to avoid:**
Refactor elaborate.rs into separate modules **before** building any new tooling, while the conformance suite still passes 47/47. Specifically:
- Extract each pass into its own module (`pass0_parse.rs`, `pass1_bundle.rs`, ..., `pass6_serialize.rs`).
- Define explicit intermediate data types for each pass boundary (ParseTree, ConstructIndex, TypeEnv, TypedAST, ValidationReport, InterchangeBundle).
- Make the top-level `elaborate()` function a composition of pass functions with well-typed boundaries.
- Keep the conformance suite as the regression gate throughout.

This is a refactoring, not a rewrite. The logic does not change; only the module boundaries do.

**Warning signs:**
- Evaluator or code generator imports internal helper functions from elaborate.rs.
- Adding a new construct (persona) requires touching more than 6 locations in elaborate.rs (one per pass).
- LSP server duplicates type-checking logic because it cannot call Pass 4 independently.
- `pub(crate)` visibility creep -- functions that should be pass-internal become crate-public.

**Phase to address:**
Must happen at the start of Phase 2, before the evaluator and CLI are built. The evaluator needs typed AST and type environment as input; if those are not extractable from elaborate.rs, the evaluator will either duplicate the work or couple to internals.

---

### Pitfall 3: Interchange Format Ossifies Without Versioning

**What goes wrong:**
The TenorInterchange JSON format is the canonical representation that all downstream tooling consumes. Adding persona declarations, P7 outcome types, and static analysis metadata to it without a versioning strategy means that:
- Old interchange files silently fail in new tooling.
- New interchange files silently fail in old tooling.
- Code generators targeting "TenorInterchange" cannot specify which version they support.
- No migration path exists for contracts elaborated under v0.3 when v1.0 changes the format.

This is worse than a breaking change in an API because interchange files are persisted artifacts, not transient wire formats.

**Why it happens:**
The interchange format is currently defined implicitly by what `serialize()` in Pass 6 produces. There is no schema, no version field, and no compatibility contract. The conformance suite tests exact byte-for-byte JSON output, which is great for correctness but means any additive change breaks all existing expected-output files.

**How to avoid:**
- Add a `"tenor_version"` field to the interchange bundle root. The elaborator already emits `"tenor"` but it is the bundle id, not a format version.
- Define the interchange schema formally (JSON Schema draft 2020-12). Generate the schema from spec sections, validate conformance test outputs against it.
- Establish a compatibility contract: additive fields are backward-compatible (old consumers ignore them). Removing or renaming fields is a major version bump.
- Version the conformance suite expected outputs alongside the interchange version.

**Warning signs:**
- The code generator breaks after a seemingly minor elaborator change.
- Someone adds a field to the interchange and has to update 40+ expected JSON files.
- Two tools disagree on the interchange format because they were built against different elaborator commits.

**Phase to address:**
Phase 1 (Spec Completion). The interchange schema must be versioned and formally defined before Phase 2 tooling depends on it.

---

### Pitfall 4: Evaluator Assumes Elaborator Correctness Without Validation

**What goes wrong:**
The evaluator receives an interchange bundle and executes it. If it trusts the bundle blindly (because "the elaborator already validated it"), it is vulnerable to:
- Hand-edited interchange files with invalid transitions.
- Interchange from non-conforming third-party elaborators.
- Interchange from older elaborator versions that did not check a constraint added later.
- Malformed interchange produced by bugs in the serializer.

The evaluator panics or silently produces wrong verdicts. Worse: the provenance chain says "correct" because the evaluator does not re-validate its inputs.

**Why it happens:**
The elaborator's six passes are designed to reject invalid contracts. It is natural to assume the evaluator can skip validation. But the evaluator is a separate trust boundary (Section 15 of the spec makes this explicit). The interchange format is the contract between elaborator and evaluator, and contracts are only as strong as the validation at the boundary.

**How to avoid:**
The evaluator must validate interchange at load time:
- Structural validation: JSON schema conformance, all required fields present.
- Semantic validation: entity transitions form valid state machines, rule strata are monotonically ordered, operation effects reference declared entities, flow step graphs are acyclic.
- This is a subset of the elaborator's checks (not a re-run of Pass 3/4 type checking, but the structural invariants from Pass 5).
- Define a `validate` subcommand (`tenor validate`) that runs this check independently.

**Warning signs:**
- Evaluator panics on a `unwrap()` when processing interchange.
- Test suite only tests evaluator with elaborator-produced interchange, never with hand-crafted or adversarial inputs.
- Evaluator's error messages reference internal Rust structures rather than contract constructs.

**Phase to address:**
Phase 2 (CLI and Evaluator). The evaluator must have its own validation layer from day one.

---

### Pitfall 5: Building Code Generation Before Domain Validation

**What goes wrong:**
Code generation is architected and partially built. Then real-world contracts reveal that the language needs changes: healthcare contracts need a construct for time-windowed approvals, supply chain contracts need conditional propagation that the current entity model does not support, financial contracts need outcome types that P7 has not fully specified yet. The code generator must be substantially reworked because it was built against a language that was not yet stable.

**Why it happens:**
Code generation is the most visible, demo-able feature. It is tempting to build it early to show progress. But code generation is the furthest downstream consumer of the spec -- it depends on every layer (interchange format, evaluation semantics, type system, construct set) being stable.

**How to avoid:**
This is already in the ROADMAP.md (Phase 3 precedes Phase 4). The discipline is: do not start code generation until at least 5 real contracts across distinct domains have been authored, elaborated, evaluated, and analyzed without hitting spec gaps. Domain validation is the stability gate.

The specific criterion: if any domain validation contract requires a spec change, the clock resets. Code generation begins only after the spec has been stable through a full domain validation cycle.

**Warning signs:**
- "Let's just start the TypeScript target while we wait for domain validation."
- Domain validation surfaces a spec gap but code generation work continues anyway.
- Code generator targets an interchange format that is still changing.

**Phase to address:**
Phase 3 (Domain Validation) must complete before Phase 4 (Code Generation) begins. The ROADMAP already enforces this ordering, but the temptation to parallelize is the pitfall.

---

### Pitfall 6: Evaluator Does Not Enforce Frozen Verdict Semantics

**What goes wrong:**
Section 10.4 of the spec defines frozen verdict semantics: within a Flow, the ResolvedVerdictSet is computed once at initiation and not recomputed after intermediate Operations execute. This is a "fundamental semantic commitment" (spec language). An evaluator that accidentally re-evaluates rules after each operation step produces different results than a conforming evaluator. The divergence is subtle -- it only manifests when:
- An Operation changes entity state mid-Flow.
- A Rule references a verdict that depends on a Fact whose meaning is affected by entity state changes.
- The frozen vs. live verdict sets differ for the specific inputs.

This means the bug passes most tests and only surfaces in production with specific contract/input combinations.

**Why it happens:**
The most natural evaluator implementation is: "for each step, evaluate the current state, execute the operation, move to the next step." Re-evaluation feels like the correct thing to do because it keeps the verdict set "fresh." The frozen snapshot semantic is counterintuitive until you understand the formal reasoning (Flows are decision graphs over a stable logical universe, per spec Section 10.4).

**How to avoid:**
- Make the snapshot explicit in the evaluator's data model: `FlowExecution { snapshot: FrozenSnapshot, current_step: StepId }` where `FrozenSnapshot` is immutable after construction.
- Write conformance tests that specifically detect frozen vs. live divergence: construct a Flow where mid-Flow entity state changes would cause a verdict to flip, and verify the Flow takes the path dictated by the frozen verdicts.
- Include these tests in the evaluator conformance suite (which does not yet exist).

**Warning signs:**
- Evaluator re-evaluates rules inside `execute_flow()`.
- FlowExecution struct has a mutable verdict set field.
- Evaluator tests only cover single-step Flows where frozen/live distinction is invisible.

**Phase to address:**
Phase 2 (Evaluator). This must be designed into the evaluator from the start, not retrofitted.

---

### Pitfall 7: NumericModel Conformance Gap Between Elaborator and Evaluator

**What goes wrong:**
The elaborator (Rust) and evaluator (may target TypeScript/other languages) implement the NumericModel differently. The elaborator uses Rust's native integer arithmetic for range computation. The evaluator uses JavaScript's `Number` type, or a decimal library with different rounding behavior. The spec mandates fixed-point decimal with round-half-to-even (IEEE 754 roundTiesToEven). A 1-cent difference in a Money comparison flips a verdict. The elaborator says the contract is valid; the evaluator produces a different result for the same inputs.

**Why it happens:**
Section 11 (NumericModel) is the most technically demanding part of the spec. The promotion rules, overflow semantics, and rounding mode are fully specified but require careful implementation in every language target. JavaScript's `Number` is IEEE 754 float64 -- it silently loses precision for large decimals. Python's `decimal` module defaults to round-half-even but has configurable precision. Rust's `rust_decimal` crate uses 128-bit representation with its own rounding defaults.

**How to avoid:**
- Create a dedicated numeric conformance test suite (separate from the elaborator conformance suite) with edge-case fixtures: values at precision boundaries, round-half-to-even tie-breaking, overflow at declared range limits, cross-type promotion (Int x Decimal).
- Every evaluator implementation (Rust, TypeScript, future targets) must pass this suite.
- For TypeScript: use a fixed-point decimal library (e.g., `decimal.js` with explicit precision/rounding config), not native `Number`. Document this as a hard requirement in the code generation guide.
- The elaborator already has `conformance/numeric/` fixtures. Expand this substantially.

**Warning signs:**
- Evaluator uses `f64` or JavaScript `Number` for Money arithmetic.
- Numeric test suite has fewer than 50 edge-case fixtures.
- TypeScript code generator emits `number` type for Decimal fields.
- No test for round-half-to-even specifically (e.g., 2.5 rounds to 2, 3.5 rounds to 4).

**Phase to address:**
Phase 2 (Evaluator) for the numeric conformance suite. Phase 4 (Code Generation) for each language target.

---

## Technical Debt Patterns

Shortcuts that seem reasonable but create long-term problems.

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Keeping all 6 passes in one file | Easier to trace data flow | Every new tool couples to elaborate.rs internals; LSP impossible without refactor | Never past v0.3 -- refactor before Phase 2 |
| Using `serde_json::Value` as the typed AST | Fast to implement, no new types needed | No compile-time guarantees on AST shape; downstream tools must pattern-match on JSON | Acceptable only for Pass 6 output; internal passes need typed Rust structs |
| Skipping evaluator input validation | Faster evaluator, less code | Silent wrong results from malformed interchange; debugging nightmares | Never -- evaluator is a trust boundary |
| Hard-coding TypeScript as the only codegen target | Ships faster | Architecture couples to TS idioms; adding Rust/Python target requires rewrite of codegen | Acceptable for MVP if port interfaces are language-agnostic |
| Running conformance suite only against final JSON output | Simpler test infrastructure | Cannot catch regressions in intermediate passes; refactoring is high-risk | Current approach is fine for elaborator; evaluator needs its own suite |
| Sharing `RawType` enum between parser and evaluator | Less code duplication | Parser-specific variants (TypeRef) leak into evaluator; evaluator must handle cases that should be impossible | Never -- define separate type for interchange layer |

## Integration Gotchas

Common mistakes when connecting components of the toolchain.

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Elaborator to Evaluator | Passing Rust structs directly instead of going through interchange | Serialize to interchange JSON, deserialize in evaluator; the interchange is the contract |
| Evaluator to Provenance | Building provenance as a side effect rather than part of the return type | `eval_strata()` returns `Set<VerdictInstance x Provenance>` per spec Section 7.2; provenance is the evaluation relation, not logging |
| Code Generator to Interchange | Generating code by walking the Rust AST instead of the interchange JSON | Code generator must consume interchange only; it must work with any conforming elaborator |
| LSP Server to Elaborator | Re-running full elaboration on every keystroke | Incremental: re-lex changed file (Pass 0), rebuild bundle (Pass 1), re-index (Pass 2), re-check types for changed constructs only (Pass 3-4) |
| CLI to Conformance Suite | Testing CLI by shelling out and comparing stdout | Test the library functions directly; CLI is a thin shell around library |
| Static Analyzer to Construct Index | Reimplementing construct indexing instead of reusing Pass 2 output | Extract Pass 2 as a shared module; static analyzer imports the same index builder |

## Performance Traps

Patterns that work at small scale but fail as usage grows.

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Cloning the entire FactSet for each rule evaluation | Slow evaluation, high memory use | Use immutable reference to shared FactSet (already implied by spec: FactSet is immutable) | Contracts with 100+ Facts and 20+ Rules |
| O(n^2) parallel branch conflict detection | Elaboration slows on complex flows | Current pairwise check is O(b^2 * e) where b=branches, e=entities; fine for <10 branches. If parallel flows grow larger, precompute entity-to-branch index | Flows with 20+ parallel branches |
| LSP full re-elaboration on every change | Editor becomes sluggish | Incremental elaboration: cache Pass 2 index, invalidate only changed files | Contracts spanning 10+ files |
| Evaluator re-derives type environment per evaluation | Slow for repeated evaluations of the same contract | Cache type environment per contract bundle; only rebuild when bundle changes | Running the same contract against many fact sets (batch evaluation) |
| Conformance suite runs all tests serially | CI takes minutes as suite grows | Tests are independent; run in parallel. Rust's `cargo test` does this by default if tests are proper `#[test]` functions instead of a custom runner | Suite exceeds 200 tests |

## Security Mistakes

Domain-specific security issues for a contract evaluation system.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Evaluator accepts interchange from untrusted sources without validation | Malformed interchange causes panics, wrong verdicts, or infinite loops (spec says evaluation terminates, but a malformed Flow DAG could contain a cycle the evaluator does not detect) | Validate interchange at load time: check all structural invariants, including DAG acyclicity |
| Provenance chain is forgeable | An adversary supplies a provenance chain that claims a verdict was derived from facts that were never asserted | Provenance is derived by the evaluator, not supplied by the caller. Never accept provenance as input. |
| Fact source declarations are not enforced | An executor populates a Fact from an internal computation instead of the declared source, corrupting the provenance root | This is an executor obligation (E1) that the language cannot enforce. Document it clearly; build attestation mechanisms in the executor layer. |
| Code-generated adapters expose internal state | Generated port interfaces leak entity state machine internals to adapter implementors who should only see the operation contract | Code generator emits port interfaces that expose operations and facts, not entity state. State is internal to the generated core. |

## UX Pitfalls

Common user experience mistakes for a DSL toolchain.

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Error messages reference elaborator internals ("Pass 4 type error in RawExpr::Compare") | Contract authors do not know what Pass 4 is or what RawExpr means | Error messages reference contract constructs: "in rule 'can_release', the comparison 'escrow_amount <= threshold' has incompatible types: Money(USD) vs Bool" |
| VS Code extension only shows first error | Author fixes one error, saves, gets next error; fix-save-fix-save cycle for 10 errors | Show all errors at once; group by construct |
| `tenor check` output is machine-readable JSON only | Humans cannot quickly scan results | Default to human-readable table output; `--json` flag for machine consumption |
| No "explain" mode for verdicts | Author cannot understand why a verdict was or was not produced | `tenor eval --explain` traces the derivation: "refund_approved NOT produced because delivery_failed NOT produced because delivery_status = 'confirmed' (not 'failed')" |
| Generated code has no comments linking back to contract constructs | Developer maintaining generated code cannot trace back to the Tenor source | Every generated function/type includes a comment: `// Generated from Tenor rule 'can_release_without_compliance' (stratum 1)` |

## "Looks Done But Isn't" Checklist

Things that appear complete but are missing critical pieces.

- [ ] **Elaborator refactoring:** "All tests pass" -- but are intermediate pass outputs typed and extractable? Can the evaluator import the type environment without reaching into elaborate.rs internals?
- [ ] **Evaluator:** "Produces correct verdicts" -- but does it handle frozen verdict semantics in multi-step Flows? Test with a Flow where mid-step entity changes would flip a verdict.
- [ ] **Static analyzer:** "Reports reachable states" -- but does it implement S3b (domain satisfiability) with documented thresholds, or just S3a (structural admissibility)? The spec distinguishes these.
- [ ] **Code generator:** "Generates TypeScript that compiles" -- but does the generated evaluator pass the numeric conformance suite? Does it use fixed-point decimal, not float64?
- [ ] **VS Code extension:** "Syntax highlighting works" -- but does it handle `.tenor` files with `import` statements? Does it resolve cross-file references for go-to-definition?
- [ ] **Domain validation:** "5 contracts elaborate without error" -- but have they been evaluated against realistic fact sets? Do the provenance chains make sense to a domain expert?
- [ ] **Conformance suite:** "47/47 passing" -- but are there tests for the evaluator? For code-generated evaluators? The current suite only covers the elaborator.
- [ ] **Interchange format:** "All tools consume it" -- but is there a formal schema? Can you validate an interchange file without running the elaborator?

## Recovery Strategies

When pitfalls occur despite prevention, how to recover.

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Spec drift | MEDIUM | Audit all interchange fields against spec sections. Write spec sections for undocumented fields. Add conformance tests. Freeze spec for 2 weeks while catching up. |
| Monolithic elaborate.rs coupling | HIGH | Extract modules one pass at a time, starting from Pass 6 (serialization) which has the fewest dependencies. Run conformance suite after each extraction. Budget 2 weeks. |
| Interchange versioning gap | MEDIUM | Add version field. Write migration tool for existing interchange files. Define compatibility rules. Budget 1 week. |
| Evaluator without input validation | MEDIUM | Add structural validation pass at evaluator entry point. Write adversarial interchange fixtures. Budget 1 week. |
| Code generation built on unstable spec | HIGH | Freeze code generator development. Complete domain validation. Identify spec changes needed. Budget depends on scope of changes -- could be 1-4 weeks. |
| Frozen verdict bug in evaluator | LOW-MEDIUM | If caught by conformance tests: fix the evaluator. If caught in production: trace the divergence, write a regression test, fix the snapshot implementation. Budget 2-3 days. |
| Numeric conformance gap | MEDIUM | Write comprehensive numeric edge-case suite. Run against all evaluator implementations. Fix each implementation's decimal handling. Budget 1 week per target language. |

## Pitfall-to-Phase Mapping

How roadmap phases should address these pitfalls.

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| Spec drift | Phase 1 (Spec Completion) | Every implementation PR has a corresponding spec section. CI checks that conformance tests cover all interchange fields. |
| Monolithic elaborate.rs | Phase 2 start (before evaluator) | Each pass is a separate module with typed input/output. Evaluator imports type environment without touching elaborate.rs internals. |
| Interchange versioning | Phase 1 (Spec Completion) | `tenor_version` field in interchange. JSON Schema for interchange exists and is CI-validated. |
| Evaluator input validation | Phase 2 (Evaluator) | Evaluator conformance suite includes adversarial interchange fixtures (malformed JSON, invalid transitions, cyclic flows). |
| Code gen before domain validation | Phase 3 gate (Domain Validation) | Phase 4 does not begin until 5+ contracts pass full lifecycle (elaborate, evaluate, analyze) without spec changes. |
| Frozen verdict semantics | Phase 2 (Evaluator) | Evaluator conformance suite includes at least 3 multi-step Flow tests where frozen vs. live verdicts diverge. |
| Numeric conformance gap | Phase 2 + Phase 4 | Numeric edge-case suite with 50+ fixtures. Every evaluator (Rust, TypeScript) passes it. CI runs it. |

## Sources

- Direct code examination: `/Users/bwb/src/rll/tenor/elaborator/src/elaborate.rs` (2,066 lines, all 6 passes in one file)
- Direct code examination: `/Users/bwb/src/rll/tenor/elaborator/src/parser.rs` (1,300 lines, AST definitions)
- Direct spec examination: `/Users/bwb/src/rll/tenor/docs/TENOR.md` (v0.3 formal specification)
- Direct roadmap examination: `/Users/bwb/src/rll/tenor/ROADMAP.md`
- [Semantic Types for Money in Rust with Fixed-point Decimal](https://crustyengineer.com/blog/semantic-types-for-money-in-rust-with-fastnum/) -- fixed-point arithmetic pitfalls in Rust
- [JSON Schema Evolution](https://www.creekservice.org/articles/2024/01/08/json-schema-evolution-part-1.html) -- interchange versioning patterns
- [Ports and Fat Adapters](https://blog.ploeh.dk/2025/04/01/ports-and-fat-adapters/) -- code generation architecture pitfalls
- [VS Code Language Server Extension Guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide) -- LSP implementation patterns
- [Spec-Driven Development 2025](https://www.thoughtworks.com/en-us/insights/blog/agile-engineering-practices/spec-driven-development-unpacking-2025-new-engineering-practices) -- spec-first discipline
- [Compiler Test Suite Strategy](https://solidsands.com/a-compiler-test-suite-thats-built-for-the-job) -- conformance testing at scale

---
*Pitfalls research for: Tenor DSL toolchain v0.3-to-1.0*
*Researched: 2026-02-21*
