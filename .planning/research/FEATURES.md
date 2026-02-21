# Feature Landscape

**Domain:** DSL toolchain for operational contract language (behavioral contract calculus)
**Researched:** 2026-02-21
**Overall confidence:** HIGH (well-established patterns from OPA, Cedar, CUE, Pkl, Alloy; Tenor spec is detailed)

---

## Comparable Systems and What They Ship

Before categorizing features, here is what the relevant ecosystem looks like. Tenor occupies a unique niche -- it is not exactly a policy language (OPA/Rego, Cedar), not a configuration language (CUE, Pkl, Dhall, Nickel), not a smart contract language (Solidity, Vyper, Move), and not a formal specification language (Alloy, TLA+). It borrows from all four categories. The feature expectations are a union of the table stakes across these categories, filtered by Tenor's specific constraints (decidable, deterministic, non-Turing-complete, closed-world).

| System | CLI | Evaluator | Static Analysis | Code Gen | IDE | Formatter | REPL | Docs |
|--------|-----|-----------|-----------------|----------|-----|-----------|------|------|
| OPA/Rego | eval, test, fmt, check, bench, build, parse, deps | Yes (core) | Type checking | Wasm bundle | VS Code | opa fmt | Yes | Extensive |
| Cedar | validate, evaluate, analyze | Yes (core) | SMT-based formal verification | Rust/Java SDKs | No official | No | No | Good |
| CUE | eval, vet, export, fmt, def, trim | Yes (constraint unification) | Type/constraint checking | JSON/YAML/Go | VS Code (community) | cue fmt | No | Good |
| Pkl | eval, test, project | Yes (core) | Type checking + constraints | Java, Kotlin, Swift, Go | VS Code, IntelliJ, Neovim | Built-in | Yes | Excellent |
| Alloy | N/A (GUI-based) | Model finder | Bounded model checking (SAT) | No | Alloy Analyzer (Swing) | No | No | Academic |
| Move | compile, test, prove | Yes (VM) | Move Prover (formal) | Bytecode | VS Code | move fmt | No | Good |

**Key observation:** Every mature DSL in this space ships a CLI with at minimum: elaborate/compile, validate/check, evaluate, and format. The ones that succeed in adoption also ship IDE support and testing infrastructure. Tenor's elaborator already covers compile; the gap is everything else.

---

## Table Stakes

Features users expect. Missing = product feels incomplete or unprofessional.

### TS-1: Unified CLI Binary

| Aspect | Detail |
|--------|--------|
| Feature | Single `tenor` binary with subcommands |
| Why Expected | Every comparable system (OPA, CUE, Pkl, Cedar, Move) ships one binary. Users expect `tenor elaborate`, `tenor check`, `tenor eval` -- not separate executables. |
| Complexity | Medium |
| Dependencies | None (wraps existing elaborator) |
| Notes | Must support `--output` format flags, `--quiet` for CI, exit codes for scripting. Stdin/stdout piping for unix composability. |

### TS-2: Elaboration (DSL to Interchange)

| Aspect | Detail |
|--------|--------|
| Feature | `tenor elaborate <file.tenor>` producing JSON interchange |
| Why Expected | This is the core pipeline. Already implemented. |
| Complexity | Done (47/47 conformance) |
| Dependencies | None |
| Notes | Already exists. Needs wrapping in CLI binary. |

### TS-3: Evaluation Against Facts

| Aspect | Detail |
|--------|--------|
| Feature | `tenor eval <bundle.json> --facts <facts.json>` producing verdict set |
| Why Expected | OPA has `opa eval`, Cedar has evaluate, CUE has `cue eval`. A contract language that cannot execute contracts is a specification, not a tool. Users need to test contracts against real data. |
| Complexity | High |
| Dependencies | TS-2 (interchange must exist to evaluate against) |
| Notes | Must implement the full evaluation model from spec sections 13.1-13.4: FactSet assembly, stratified rule evaluation, verdict resolution. Output must include the verdict set. |

### TS-4: Error Messages with Source Location

| Aspect | Detail |
|--------|--------|
| Feature | Every error identifies: file, line, construct, field, human-readable message |
| Why Expected | Spec section 12.3 mandates this. Every language users have ever used does this. Errors without locations are unusable. |
| Complexity | Done (elaborator already does this) |
| Dependencies | None |
| Notes | Already implemented per conformance suite negative tests. Maintain quality as new passes are added. |

### TS-5: Conformance Test Suite Runner

| Aspect | Detail |
|--------|--------|
| Feature | `tenor test` running the conformance suite with pass/fail reporting |
| Why Expected | OPA has `opa test`, Move has `move test`, Pkl has `pkl test`. Language implementers and contract authors both need automated testing. |
| Complexity | Low (runner.rs already exists) |
| Dependencies | TS-1 |
| Notes | Already implemented as the conformance runner. Needs: user-authored test support (contract + facts + expected verdicts), not just elaborator conformance. |

### TS-6: Schema/Interchange Validation

| Aspect | Detail |
|--------|--------|
| Feature | `tenor validate <bundle.json>` checking interchange JSON against schema |
| Why Expected | CUE has `cue vet`, OPA has `opa check`. Users receiving interchange from other tools need to validate it independently of elaboration. |
| Complexity | Medium |
| Dependencies | Interchange schema definition |
| Notes | Validates structure, type well-formedness, reference integrity -- without re-elaborating from DSL source. |

### TS-7: Syntax Highlighting (VS Code)

| Aspect | Detail |
|--------|--------|
| Feature | TextMate grammar for .tenor files providing keyword, type, string, comment highlighting |
| Why Expected | Every language with >10 users ships syntax highlighting. OPA, CUE, Pkl, Cedar all have VS Code extensions. Without it, the language looks amateurish regardless of its formal properties. |
| Complexity | Low |
| Dependencies | None (TextMate grammars are standalone) |
| Notes | TextMate grammar only -- no LSP required for basic highlighting. Can ship independently of other VS Code features. |

### TS-8: Inline Diagnostics (VS Code)

| Aspect | Detail |
|--------|--------|
| Feature | Elaboration errors shown inline in the editor as red/yellow squiggles |
| Why Expected | Users expect real-time feedback. Pkl, OPA, and CUE all provide inline diagnostics. Running CLI manually to see errors is a 2010s workflow. |
| Complexity | Medium |
| Dependencies | TS-1 (CLI binary the extension invokes), TS-7 |
| Notes | Can start with "run elaborator on save, parse error JSON, show diagnostics" without a full LSP. Upgrade to LSP later if needed. |

### TS-9: Language Reference Documentation

| Aspect | Detail |
|--------|--------|
| Feature | Author-facing documentation distinct from the formal spec |
| Why Expected | The formal spec (TENOR.md) is an implementer document. Authors need: "how do I write a Rule?", "what types are available?", worked examples per construct. Every language ships this separately. |
| Complexity | Medium (writing effort, not engineering) |
| Dependencies | Stable spec |
| Notes | Spec is the source of truth; reference docs are the human-readable projection. Must stay in sync. |

### TS-10: Persona Declaration

| Aspect | Detail |
|--------|--------|
| Feature | `persona` as a first-class construct with declared id |
| Why Expected | Personas are used throughout the language (Operation allowed_personas, Flow steps, HandoffStep) but never declared. This is a spec gap that makes contracts incomplete -- you cannot enumerate all personas from the contract alone, violating the closed-world property. |
| Complexity | Medium |
| Dependencies | Spec update, elaborator changes, conformance tests |
| Notes | This is a spec completion item, not a tooling feature. But it is table stakes for 1.0 because without it, S4 (authority topology) cannot be fully derived. |

---

## Differentiators

Features that set Tenor apart. Not expected by default, but create competitive advantage.

### D-1: Provenance-Traced Evaluation

| Aspect | Detail |
|--------|--------|
| Feature | `tenor eval` produces not just verdicts but complete provenance chains: which facts fed which rules, which rules produced which verdicts, full derivation tree |
| Value Proposition | No comparable system does this natively. OPA traces are debug logging. Cedar's analysis is about policy correctness, not execution traceability. Tenor's provenance is a formal semantic property (C7). This is the killer feature for regulated industries (healthcare, finance, compliance). |
| Complexity | High |
| Dependencies | TS-3 (evaluator) |
| Notes | Provenance is not an add-on -- it is baked into the evaluation model per spec section 7.2, 8.5, 10.6. Every VerdictInstance carries VerdictProvenance. Every OperationProvenance carries facts_used and verdicts_used. |

### D-2: Static Analysis Suite (S1-S7)

| Aspect | Detail |
|--------|--------|
| Feature | `tenor check` implementing all seven static analysis obligations from spec section 14 |
| Value Proposition | Cedar has SMT-based analysis but only for authorization properties. OPA has no static analysis beyond type checking. Alloy has bounded model checking but no state machine analysis. Tenor's S1-S7 suite is unique: complete state space enumeration (S1), reachability (S2), structural admissibility per state (S3a), authority topology (S4), verdict space (S5), flow path enumeration (S6), complexity bounds (S7). No other contract/policy language offers all of these from a single command. |
| Complexity | High (S1-S2: Medium, S3a: Medium, S4: Medium, S5: Low, S6: High, S7: Medium) |
| Dependencies | TS-2, TS-10 (persona declaration needed for S4) |
| Notes | S3b (domain satisfiability) is explicitly qualified in the spec as "not always computationally feasible" -- implement S3a first, S3b as an opt-in for small domains. |

### D-3: Human-Readable Explain

| Aspect | Detail |
|--------|--------|
| Feature | `tenor explain <bundle.json>` producing a human-readable contract summary: what entities exist, what operations are available to whom, what state transitions are possible, what rules determine outcomes |
| Value Proposition | This directly addresses the preamble's core claim: "Any agent that can read this specification can fully understand a system described in it." Explain makes the contract legible to non-technical stakeholders -- lawyers, compliance officers, business analysts. No comparable system generates plain-English summaries of contract behavior. |
| Complexity | Medium |
| Dependencies | TS-2 |
| Notes | Output should be structured (Markdown or similar), not free-form prose. Sections: Personas and their authorities, Entity state machines and transitions, Decision rules and what triggers them, Workflows and their paths. |

### D-4: Code Generation (Ports and Adapters)

| Aspect | Detail |
|--------|--------|
| Feature | `tenor generate --target typescript` producing a complete executor skeleton with port interfaces and generated core domain logic |
| Value Proposition | CUE generates data formats. Pkl generates data bindings. Move compiles to bytecode. None of them generate full application skeletons. Tenor's code generation produces the entire execution engine -- entity store, rule engine, operation handlers, flow orchestrator, provenance collector -- as generated core code, with developer-authored adapters for external integration. This is the bridge from "specification language" to "implementation accelerator." |
| Complexity | Very High |
| Dependencies | TS-3 (evaluator validates correctness), D-1 (provenance model informs generated code), domain validation (real contracts prove the pattern works) |
| Notes | First target: TypeScript (widest adoption). Second target: Rust (natural fit given elaborator). Ship with `@tenor/adapters-local` for in-memory dev/test. |

### D-5: Operation Outcome Typing (P7)

| Aspect | Detail |
|--------|--------|
| Feature | Named outcome types on Operations, statically enumerable, replacing flow-side classification |
| Value Proposition | Makes flows type-safe end-to-end. Without this, flow outcome routing is stringly-typed and error-prone. With it, the static analyzer can verify that every possible operation outcome is handled by the flow, and code generation can produce typed result types. |
| Complexity | High (spec change + elaborator + conformance) |
| Dependencies | TS-10 (persona declaration should land first to avoid multiple spec revisions) |
| Notes | Currently acknowledged as deferred (P7 in spec). Critical for clean code generation. |

### D-6: Contract-Level Testing

| Aspect | Detail |
|--------|--------|
| Feature | User-authored test cases: `.tenor` contract + facts JSON + expected verdicts JSON. `tenor test` runs them alongside conformance suite. |
| Value Proposition | OPA's `opa test` with `test_` prefixed rules is the gold standard for policy testing. Tenor needs equivalent: "given these facts, this contract should produce these verdicts." This is how contract authors validate correctness before deployment. |
| Complexity | Medium |
| Dependencies | TS-3 (evaluator), TS-5 (test runner) |
| Notes | Test format should mirror conformance suite conventions: `<name>.tenor` + `<name>.facts.json` + `<name>.expected-verdicts.json`. Include negative tests: "given these facts, this operation should fail with this error." |

### D-7: Go-to-Definition (VS Code)

| Aspect | Detail |
|--------|--------|
| Feature | Click on a fact_ref, entity_ref, or verdict_ref and jump to its declaration |
| Value Proposition | Essential for navigating non-trivial contracts. OPA's VS Code extension supports this. With cross-file imports, navigation becomes critical. |
| Complexity | Medium |
| Dependencies | TS-7, TS-8 (diagnostics infrastructure) |
| Notes | Requires building a symbol table from elaboration Pass 2 output. Can reuse construct index. |

### D-8: Shared Type Library (P5)

| Aspect | Detail |
|--------|--------|
| Feature | Cross-contract type reuse for Record and TaggedUnion definitions |
| Value Proposition | Real-world contracts in the same domain share types (LineItem, Address, Money types). Without shared types, every contract redeclares identical structures. This is especially important for organizations with multiple contracts in the same domain. |
| Complexity | High (import semantics, inter-contract elaboration, versioning) |
| Dependencies | Domain validation (need real-world contracts to scope correctly) |
| Notes | Acknowledged in spec as P5, deferred to v2. The roadmap correctly identifies this as "do last -- needs authoring experience from Phase 2 to scope correctly." |

---

## Anti-Features

Features to explicitly NOT build. Each represents a common request that would violate Tenor's design constraints or dilute its value proposition.

### AF-1: REPL / Interactive Evaluation

| Anti-Feature | REPL for interactive contract exploration |
|--------------|------------------------------------------|
| Why Avoid | Tenor contracts are not expressions to evaluate incrementally. Evaluation requires a complete FactSet, a complete contract, and produces a complete verdict set. Partial evaluation is not meaningful in a closed-world system. A REPL implies incremental, exploratory computation -- the opposite of Tenor's batch, deterministic, total evaluation model. OPA and CUE have REPLs because they evaluate expressions; Tenor evaluates contracts. |
| What to Do Instead | `tenor eval` with different fact sets. `tenor explain` for exploration. Test cases for "what if" scenarios. |

### AF-2: Formatter / Auto-Format

| Anti-Feature | `tenor fmt` that rewrites source files |
|--------------|----------------------------------------|
| Why Avoid | Premature optimization of authoring ergonomics. The DSL syntax is not yet battle-tested across diverse real-world contracts. Formatting rules require consensus on style that does not exist yet. OPA and CUE shipped formatters after significant community usage established conventions. Formatting also requires a CST (concrete syntax tree) that preserves comments and whitespace -- the current parser produces an AST that discards this. Adding CST preservation is a significant refactor with no payoff until the syntax is stable and widely used. |
| What to Do Instead | Establish style conventions in documentation. Revisit after 1.0 when real usage patterns emerge. |

### AF-3: GUI Contract Editor

| Anti-Feature | Visual/graphical contract editor |
|--------------|----------------------------------|
| Why Avoid | Explicitly listed in "Not in scope for 1.0" in the roadmap. GUI editors for DSLs have a poor track record (JetBrains MPS projectional editing is powerful but niche). The audience for Tenor 1.0 is developers and technical domain experts who are comfortable with text editors. A GUI editor would consume enormous engineering effort for a tiny audience. |
| What to Do Instead | VS Code extension with good syntax highlighting, diagnostics, and go-to-definition provides 90% of the value at 10% of the cost. |

### AF-4: Runtime Monitoring / Enforcement

| Anti-Feature | Production runtime that enforces contracts in real-time |
|--------------|--------------------------------------------------------|
| Why Avoid | Explicitly listed in "Not in scope for 1.0." Tenor is a specification and verification tool, not a runtime. The executor obligations (E1-E9) are trust boundaries -- the language explicitly acknowledges it cannot enforce them from within. Building a runtime conflates the contract (what should happen) with the enforcement mechanism (making it happen). This is a different product. |
| What to Do Instead | Code generation (D-4) produces executor skeletons. The generated code is the bridge to runtime. But the runtime itself is the developer's responsibility. |

### AF-5: Module Federation / Package Registry

| Anti-Feature | Cross-organization type sharing, package versioning, registry |
|--------------|---------------------------------------------------------------|
| Why Avoid | P5 (shared type library) is already scoped to cross-contract within a single project. Module federation adds distribution, versioning, conflict resolution, and trust -- each a deep problem. CUE has a module registry; it took years to mature. Pkl has packages; they are still evolving. Attempting this before the language is stable guarantees breaking changes propagating across organizations. |
| What to Do Instead | File-level imports with explicit paths (already supported). Shared type library (D-8) for intra-project reuse. Federation after 2.0. |

### AF-6: Additional Code Generation Targets Beyond TypeScript + Rust

| Anti-Feature | Generating Python, Java, Go, C# executors |
|--------------|-------------------------------------------|
| Why Avoid | Each code generation target requires: understanding the target language's type system, numeric libraries (fixed-point decimal), concurrency model, and testing ecosystem. TypeScript and Rust are chosen for clear reasons (wide adoption; natural fit with elaborator). Adding targets without demand dilutes quality. |
| What to Do Instead | Ship TypeScript and Rust well. Add targets based on user demand post-1.0. |

### AF-7: Aggregate Functions / Computed Intermediate Values

| Anti-Feature | Adding `sum()`, `count()`, `avg()` to the language |
|--------------|-----------------------------------------------------|
| Why Avoid | Spec section 5.5 explicitly prohibits this with detailed rationale. Aggregates are derived values, not verdicts. Tenor's evaluation model is Facts -> Rules -> Verdicts, with no intermediate computation layer. Adding aggregates would require a new construct kind, blur the Fact/Verdict distinction, and complicate provenance. |
| What to Do Instead | Computed values arrive as Facts from external systems. The contract takes them as given. This is a feature, not a limitation -- it preserves the ground property and keeps provenance chains clean. |

### AF-8: Formal Proof of Soundness

| Anti-Feature | Lean/Coq mechanized proof of language properties |
|--------------|---------------------------------------------------|
| Why Avoid | Explicitly listed in "Not in scope for 1.0" as a separate research track. Valuable but orthogonal to shipping a usable toolchain. Cedar's Lean proofs took a dedicated team at Amazon. |
| What to Do Instead | Conformance suite coverage, extensive domain validation, and static analysis (D-2) provide practical confidence. Formal proofs are a post-1.0 research track. |

---

## Feature Dependencies

```
TS-10 (Persona Declaration)
  |
  v
D-2 (Static Analysis S1-S7) --- specifically S4 (authority topology) requires declared personas
  |
  v
D-3 (Explain) --- uses static analysis results for human-readable output

TS-2 (Elaboration) [DONE]
  |
  +---> TS-6 (Interchange Validation)
  |
  +---> TS-3 (Evaluation)
  |       |
  |       +---> D-1 (Provenance-Traced Evaluation)
  |       |
  |       +---> D-6 (Contract-Level Testing)
  |       |
  |       +---> Domain Validation (5-10 real contracts)
  |               |
  |               +---> D-8 (Shared Type Library) --- scoped by domain experience
  |               |
  |               +---> D-4 (Code Generation) --- validated by real contracts
  |
  +---> D-5 (Operation Outcome Typing) --- spec change
          |
          +---> D-4 (Code Generation) --- needs typed outcomes for clean output

TS-1 (CLI Binary)
  |
  +---> TS-5 (Test Runner)
  +---> All subcommands

TS-7 (Syntax Highlighting)
  |
  +---> TS-8 (Inline Diagnostics)
          |
          +---> D-7 (Go-to-Definition)
```

---

## MVP Recommendation

### Must ship for 1.0 (in priority order):

1. **TS-1: Unified CLI** -- The entry point for everything. Without it, nothing else is accessible.
2. **TS-10: Persona Declaration** -- Spec gap that blocks static analysis (S4). Small scope, high leverage.
3. **D-5: Operation Outcome Typing** -- Spec completion (P7). Blocks clean code generation and flow type-safety. Better to do spec changes early.
4. **TS-3: Evaluator** -- A contract language that cannot execute contracts is a spec, not a tool. This is the moment the language becomes useful.
5. **D-1: Provenance-Traced Evaluation** -- Build provenance into the evaluator from day one, not as an afterthought. This is Tenor's unique value.
6. **D-2: Static Analysis (S1-S7)** -- The second unique value proposition. Start with S1, S2, S5 (low complexity), then S3a, S4, S6, S7.
7. **D-6: Contract-Level Testing** -- Authors need to test their contracts. Builds on evaluator.
8. **D-3: Explain** -- Low-complexity differentiator with enormous value for non-technical stakeholders.
9. **Domain Validation (5-10 contracts)** -- Proves the language works on real problems. Surfaces remaining spec gaps before 1.0 freeze.
10. **D-4: Code Generation (TypeScript)** -- The bridge from specification to implementation. Ship after domain validation confirms stability.
11. **D-8: Shared Type Library** -- Scope from domain validation experience.
12. **TS-7 + TS-8: VS Code Extension** -- Syntax highlighting and diagnostics. Can be built in parallel with other work.
13. **TS-9: Language Reference Docs** -- Write after spec is stable, not before.

### Defer post-1.0:

- **D-4 Rust target** -- Ship TypeScript first, Rust second. TypeScript has wider adoption for the target audience (business system developers).
- **AF-2: Formatter** -- Wait for community style conventions.
- **AF-5: Module federation** -- Wait for multi-org usage patterns.
- **AF-8: Formal proofs** -- Separate research track.

---

## Complexity Budget

| Feature | Complexity | Estimated Effort | Phase |
|---------|------------|------------------|-------|
| TS-1: CLI Binary | Medium | 1-2 weeks | Phase 2 |
| TS-10: Persona Declaration | Medium | 1-2 weeks | Phase 1 |
| D-5: Operation Outcome Typing | High | 2-3 weeks | Phase 1 |
| D-8: Shared Type Library | High | 2-3 weeks | Phase 1 (late) |
| TS-3: Evaluator | High | 3-4 weeks | Phase 2 |
| D-1: Provenance Evaluation | High | 2-3 weeks (integrated with TS-3) | Phase 2 |
| D-2: Static Analysis S1-S7 | High | 4-6 weeks total | Phase 2 |
| D-6: Contract Testing | Medium | 1-2 weeks | Phase 2 |
| D-3: Explain | Medium | 1-2 weeks | Phase 2 |
| TS-6: Interchange Validation | Medium | 1 week | Phase 2 |
| Domain Validation | Medium | 3-4 weeks | Phase 3 |
| D-4: Code Gen (TypeScript) | Very High | 4-6 weeks | Phase 4 |
| TS-7: Syntax Highlighting | Low | 3-5 days | Phase 5 (or parallel) |
| TS-8: Inline Diagnostics | Medium | 1-2 weeks | Phase 5 |
| D-7: Go-to-Definition | Medium | 1-2 weeks | Phase 5 |
| TS-9: Language Reference | Medium | 2-3 weeks writing | Phase 5 |

---

## Sources

- [OPA CLI Reference](https://www.openpolicyagent.org/docs/cli) -- comprehensive CLI subcommand patterns
- [Cedar Policy Language](https://github.com/cedar-policy/cedar) -- static analysis with SMT
- [Cedar Analysis Tools](https://aws.amazon.com/blogs/opensource/introducing-cedar-analysis-open-source-tools-for-verifying-authorization-policies/) -- formal verification approach
- [CUE CLI Commands](https://cuelang.org/docs/reference/command/) -- eval/vet/export/fmt pattern
- [Pkl Documentation](https://pkl-lang.org/index.html) -- code generation and IDE integration model
- [VS Code Language Extensions](https://code.visualstudio.com/api/language-extensions/overview) -- extension feature tiers
- [VS Code LSP Guide](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide) -- diagnostics and go-to-definition
- [Move Language 2025](https://aptoslabs.medium.com/move-in-2025-building-a-modern-smart-contract-language-391fc8ce0fe8) -- smart contract tooling maturity
- [Martin Fowler DSL Catalog](https://martinfowler.com/dslCatalog/) -- code generation patterns
- [Language Workbench Wikipedia](https://en.wikipedia.org/wiki/Language_workbench) -- feature taxonomy
- [Alloy 6](https://www.hillelwayne.com/post/alloy6/) -- bounded model checking approach
- [Audit Trails and Explainability](https://lawrence-emenike.medium.com/audit-trails-and-explainability-for-compliance-building-the-transparency-layer-financial-services-d24961bad987) -- regulatory requirements for provenance
- [NIST Access Control Verification](https://tsapps.nist.gov/publication/get_pdf.cfm?pub_id=921189) -- state machine reachability methods
