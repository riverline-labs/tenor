# Phase 1: Spec Completion - Research

**Researched:** 2026-02-21
**Domain:** Language specification formalization (CFFP), interchange format versioning, JSON Schema
**Confidence:** HIGH

## Summary

Phase 1 is a pure specification phase — no elaborator code changes, no tooling. The deliverables are updated `docs/TENOR.md` sections for three new/modified constructs (Persona, P7 outcome typing, P5 shared type library), three CFFP artifact documents, a formal JSON Schema for the interchange format, and a `tenor_version` field specification. The phase is constrained by the CFFP protocol (`docs/cffp.cue`) which mandates a six-phase process per construct: invariant declaration, candidate formalisms, pressure testing, survivor derivation, static analysis obligations, and canonicalization.

The serial ordering (Persona → P7 → P5) is locked by user decision and is well-motivated: Persona is simplest and establishes the CFFP execution pattern; P7 depends on understanding how Operations compose with Flows (persona references needed); P5 is hardest and may produce a scoped-down canonical form. Each CFFP run must compose-test against all previously canonicalized constructs (Fact, Entity, Rule, Operation, Flow, TypeDecl, and any constructs canonicalized earlier in this phase).

The primary risk is P5 complexity: the user flagged it as "needs authoring experience from Phase 2 to scope correctly." A scoped-down canonical form with acknowledged limitations is a valid CFFP outcome. The secondary risk is inadequate composition testing — each new construct must be tested against all six existing constructs plus any newly canonicalized ones.

**Primary recommendation:** Execute each CFFP run as a self-contained unit producing a JSON artifact conforming to `docs/cffp.cue`, then translate the canonical form into spec sections following the exact structure of existing construct sections (syntax, semantics, elaboration rules, interchange representation, validation rules). The JSON Schema should be authored from the spec, not reverse-engineered from elaborator output.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- CFFP execution order: Serial, not parallel: Persona → P7 → P5
- Persona first because it's simplest and P7/P5 composition tests may reference it
- P7 second because outcome typing affects how Operations compose with Flows — P5 needs this settled
- P5 last because shared types cross contract boundaries and composition tests need both Persona and P7 canonicalized
- Full CFFP rigor on all three constructs — these are language semantics, not features. Every construct gets: invariant declaration, candidate formalisms with proof sketches, counterexample pressure, composition testing against existing canonicalized constructs (Fact, Entity, Rule, Operation, Flow, TypeDecl), and canonicalization
- CFFP artifact format: Each construct produces a CFFP instance document (following `docs/cffp.cue` schema), stored as `docs/cffp/persona.json`, `docs/cffp/p7-outcome-typing.json`, `docs/cffp/p5-shared-types.json`
- `depends_on` field in each CFFP instance lists all previously canonicalized constructs
- Composition failures against existing constructs are tested explicitly in Phase 3 of each CFFP run
- Persona starting invariants: every persona reference in an Operation or Flow must resolve to a declared Persona; persona ids are unique within a contract
- P7 starting invariants: every Flow transition must reference a valid outcome of the referenced Operation; the set of outcomes is statically determinable; no implicit/catch-all outcomes
- P5 starting invariants: type resolution terminates; imported types compose with local types without ambiguity; circular type imports are detected and rejected
- Interchange versioning: `tenor_version` field at bundle top-level, string, semver format. v0.3 → v1.0 is a major version bump. Formal JSON Schema at `docs/interchange-schema.json` generated from spec, not reverse-engineered. Schema validation becomes a conformance test.
- Spec freeze: "Frozen" means no breaking changes to existing construct semantics after Phase 1. Additive changes allowed but documented. Changes after freeze require new CFFP run. `docs/TENOR.md` gets version header "Tenor Language Specification v1.0". New sections follow same structure as existing construct sections.

### Claude's Discretion
- Internal structure of CFFP artifact documents (JSON vs markdown — follow what's natural for the schema)
- Specific invariant classifications (termination, determinism, etc.) — derive from the construct's nature
- Number of candidate formalisms per construct — generate as many as the design space warrants
- Counterexample generation strategy — target weak points in candidate claims
- JSON Schema tooling (hand-authored vs generated) — whatever produces a correct, maintainable schema
- Whether P5 needs sub-phases within its CFFP run (type identity is a hard subproblem)

### Deferred Ideas (OUT OF SCOPE)
- Expressing CFFP in Tenor itself — domain validation exercise for Phase 5
- P5 module federation (inter-org type sharing) — explicitly out of scope for 1.0, per PROJECT.md
- Generic type parameters for Records — v2 requirement (SPEC-07)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SPEC-01 | Persona declared as first-class construct with id and optional metadata in spec and elaborator | CFFP run 1 (Persona). Starting invariants provided. Current state: personas are bare string identifiers in Operations and Flows with no declaration construct. Research finding: the construct must integrate with Operation `allowed_personas`, Flow step `persona` fields, HandoffStep `from_persona`/`to_persona`, and S4 authority topology analysis. Spec section must define syntax, semantics, elaboration rules, interchange representation, validation rules. |
| SPEC-02 | Operation outcome typing — named outcome types on Operations, statically enumerable, specified via CFFP | CFFP run 2 (P7). Current state: outcomes are Flow-side classification only (AL13 in spec). The OperationStep `outcomes` map uses string labels ("success") that are not declared on the Operation itself. Research finding: this is a fundamental change — Operations must declare their outcome set, and Flows must reference valid outcomes. Affects Operation construct definition (Section 8), Flow step typing (Section 10), and interchange format for both. |
| SPEC-03 | Shared type library — cross-contract type reuse for Record and TaggedUnion with import semantics, specified via CFFP | CFFP run 3 (P5). Current state: TypeDecl is per-contract only (spec Section 4.5), inlined during Pass 3, does not appear in interchange. Research finding: P5 introduces cross-contract references which fundamentally affect the closed-world semantics (C5). Must resolve: structural vs nominal type identity, import mechanism (new syntax), cross-contract elaboration, and whether shared types appear in interchange or remain inlined. User flagged this as hardest — scoped-down canonical form is acceptable. |
| SPEC-04 | Interchange format versioned with `tenor_version` field and formal JSON Schema | Research finding: Current interchange uses `"tenor": "0.3"` on every construct document and at bundle level. Decision is to add `tenor_version` at top level (semver string). JSON Schema should use Draft 2020-12 (current stable standard). Schema must be authored from spec, covering all construct kinds, all field types, all serialization rules from Pass 6. Current conformance suite has 47 tests with exact expected JSON — these serve as validation corpus for the schema. |
| SPEC-05 | Each spec change (SPEC-01, SPEC-02, SPEC-03) run through CFFP with invariant declaration, candidate formalisms, pressure testing, and canonicalization before implementation | Research finding: CFFP protocol is fully defined in `docs/cffp.cue` (v0.2.1, 398 lines). Protocol has 6 phases per construct. Each produces a `#CFFPInstance` document. Three outcomes: canonical, collapse, or open. The `depends_on` field enables composition testing against prior constructs. No CFFP artifacts exist yet in the repo (`docs/cffp/` directory does not exist). |
</phase_requirements>

## Standard Stack

### Core
| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| CFFP protocol | v0.2.1 | Construct formalization protocol | Project's own protocol; schema at `docs/cffp.cue` |
| JSON Schema | Draft 2020-12 | Interchange format validation | Current stable standard; OpenAPI 3.1 compatible; recommended by json-schema.org |
| CUE | (reference only) | CFFP schema definition language | Already used for CFFP schema; CFFP artifacts are JSON conforming to CUE schema |

### Supporting
| Tool | Purpose | When to Use |
|------|---------|-------------|
| `ajv` or `jsonschema` (Rust) | JSON Schema validation in CI | For conformance test: every elaborator output validates against `docs/interchange-schema.json` |
| `cue vet` | Validate CFFP artifacts against schema | After producing each CFFP instance document |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| JSON Schema 2020-12 | TypeSchema or CUE schema for interchange | JSON Schema is universally supported; CUE adds a dependency for consumers; JSON Schema is the right choice for a published interchange format |
| Hand-authored JSON Schema | Generated from Rust types via `schemars` | Spec-first discipline means schema comes from spec, not implementation; hand-authoring is correct here since the schema defines what the implementation must produce |

## Architecture Patterns

### CFFP Execution Pattern

Each of the three CFFP runs follows the same structure:

```
docs/cffp/
├── persona.json              # CFFP instance for Persona construct
├── p7-outcome-typing.json    # CFFP instance for P7
└── p5-shared-types.json      # CFFP instance for P5
```

Each CFFP instance is a JSON document conforming to `docs/cffp.cue#CFFPInstance`:

1. **Phase 1 — Invariant Declaration**: Define testable, structural, falsifiable invariants. Starting invariants are provided in CONTEXT.md. Additional invariants may emerge from analysis.
2. **Phase 2 — Candidate Formalisms**: Define formal structures with evaluation rules, resolution rules, proof sketches, complexity bounds, and failure modes. At least one candidate required.
3. **Phase 3 — Pressure**: Generate counterexamples targeting specific invariant violations. Test composition against all `depends_on` constructs. Rebuttals are either refutations or scope-narrowings (the latter become acknowledged limitations).
4. **Survivor Derivation**: Explicitly populate `#Derived` — eliminated candidates and survivors with accumulated scope narrowings.
5. **Phase 4 — Collapse Test**: Only if multiple survivors. Attempt merge or select with rationale.
6. **Phase 5 — Static Analysis Obligations**: Prove properties that emerge from the complete evaluation model.
7. **Phase 6 — Canonicalization**: Produce formal statement, evaluation definition, satisfied invariants, acknowledged limitations.

### Spec Section Pattern

Every new construct section in `docs/TENOR.md` must follow the existing pattern:

```
## N. ConstructName

### N.1 Definition
[Formal definition with algebraic notation]

### N.2 Evaluation / Semantics
[Operational semantics with pseudo-code]

### N.3 Constraints
[Load-time validation rules, structural requirements]

### N.4 Provenance
[Provenance record structure if applicable]

### N.5 Interchange Representation
[How the construct serializes to JSON in Pass 6]
```

Sections 5 (Fact), 6 (Entity), 7 (Rule), 8 (Operation), 10 (Flow) all follow this template. New sections must be structurally parallel.

### Interchange Schema Pattern

The interchange format has a consistent structure observed across all conformance test expected outputs:

```json
{
  "constructs": [ ... ],  // Array of construct documents
  "id": "bundle_name",
  "kind": "Bundle",
  "tenor": "0.3"          // Current version field
}
```

Each construct document has:
- `"kind"`: One of "Fact", "Entity", "Rule", "Operation", "Flow"
- `"id"`: Construct identifier
- `"tenor"`: Version string
- `"provenance"`: `{"file": "...", "line": N}`
- Kind-specific fields (sorted lexicographically)

The `tenor_version` field will be added at the bundle level. The existing per-construct `"tenor"` field may be retained for backward compatibility or removed — this is a design decision for plan 01-04.

### Anti-Patterns to Avoid
- **Implementation-first schema**: Do not generate the JSON Schema from the elaborator's Rust types. The schema defines what the implementation must produce, not what it currently produces.
- **Incomplete composition testing**: Each CFFP run must test against ALL prior constructs, not just the ones that seem related.
- **Informal invariants**: Every invariant must be testable and falsifiable per `docs/cffp.cue#Invariant`. "Personas should be user-friendly" is not an invariant.
- **Skipping Phase 3b**: If all candidates are eliminated, the protocol requires diagnosis (invariants too strong, candidates too weak, or construct incoherent) — not ad-hoc workarounds.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CFFP artifact validation | Custom validation logic | `cue vet` against `docs/cffp.cue` | CUE schema is already defined; manual validation misses structural constraints |
| JSON Schema for interchange | Custom format validators | JSON Schema Draft 2020-12 | Industry standard, tooling ecosystem, cross-language validation |
| Semver comparison | String comparison | Semver parsing (when implementing, in Phase 2) | Semver has non-trivial comparison rules (pre-release, build metadata) |

**Key insight:** Phase 1 is specification work, not implementation. The temptation to "just code it" must be resisted. Every construct design decision must survive CFFP pressure before any implementation artifact is created.

## Common Pitfalls

### Pitfall 1: Persona Construct Overdesign
**What goes wrong:** Personas grow metadata fields, hierarchies, delegation chains, and role-based access control features during the CFFP run. The construct becomes a mini-RBAC system bolted onto a contract language.
**Why it happens:** The CONTEXT.md mentions "optional metadata fields" and "whether personas form hierarchies (delegation)." These are starting points for investigation, not requirements. The CFFP pressure phase should kill overdesign.
**How to avoid:** Apply the spec's own design constraints (C1-C7) as invariants. Persona must be decidable (C1), finite (termination — C2), deterministic (C3), and statically analyzable (C4). Delegation hierarchies violate C6 (explicit over implicit) unless every delegation relationship is explicitly declared. Start with the minimal Persona: `{id: PersonaId, metadata?: Record}`. Let counterexamples argue for more, not assumed need.
**Warning signs:** Candidate formalisms with more than 5 fields. Invariants that mention "flexible" or "extensible."

### Pitfall 2: P7 Outcome Typing Breaking Backward Compatibility
**What goes wrong:** Named outcome types on Operations change the Operation construct definition (Section 8), which means every existing conformance test with Operations becomes invalid. The `outcomes` field on OperationStep (currently `{"success": "step_ref"}`) must now reference declared outcomes.
**Why it happens:** P7 is not a new construct — it modifies two existing constructs (Operation and Flow/OperationStep). The change radiates through the interchange format, the elaboration passes, and the conformance suite.
**How to avoid:** The CFFP run for P7 must include composition testing against the existing Operation canonical form (Section 8) and the existing Flow canonical form (Section 10). The canonical form must specify a migration path: how do existing contracts (with ad-hoc "success"/"failure" outcome labels) coexist with or migrate to typed outcomes? The spec freeze definition says "no breaking changes to existing construct semantics" — P7 must be additive or provide explicit migration.
**Warning signs:** A P7 canonical form that invalidates AL13 ("Flow typed outcomes are Flow-side only") without addressing the transition.

### Pitfall 3: P5 Scope Explosion
**What goes wrong:** Shared type library touches type identity (structural vs nominal), import semantics (new syntax), cross-contract elaboration (new pass behavior), and closed-world semantics (C5). Each sub-problem is hard independently. Together they exceed a single CFFP run's capacity.
**Why it happens:** Type sharing across contract boundaries is a module system problem. Module systems are notoriously hard to formalize (ML modules took decades of research).
**How to avoid:** The user explicitly noted P5 "may produce a scoped-down canonical form with acknowledged limitations, which is a valid CFFP outcome." Embrace this. Consider sub-phasing P5's CFFP run: first CFFP for type identity (structural vs nominal), then CFFP for import semantics (given resolved type identity). If the full problem does not collapse to a single canonical form, `outcome: "open"` with documented limitations is valid. The CFFP protocol supports this explicitly.
**Warning signs:** More than 8 invariants declared for P5. Candidate formalisms that require runtime type checking. Composition failures against C5 (closed-world semantics).

### Pitfall 4: JSON Schema Drift from Spec
**What goes wrong:** The JSON Schema is written once, then the spec evolves (new constructs from CFFP runs), and the schema is not updated in lockstep. The schema becomes a stale artifact that validates old formats but rejects valid new output.
**Why it happens:** The schema and the spec are two separate documents describing the same thing. Keeping them synchronized requires discipline.
**How to avoid:** Write the schema AFTER all three CFFP runs complete and the spec is frozen. The schema captures the v1.0 interchange format — writing it before the spec is stable guarantees rework. If intermediate validation is needed, use the 47 existing conformance test expected outputs as a validation corpus.
**Warning signs:** Schema is authored before plans 01-01 through 01-03 complete.

### Pitfall 5: CFFP Cargo-Culting
**What goes wrong:** CFFP runs become performative — invariants are declared but not actually testable, proof sketches are hand-waved, counterexamples are trivial or absent, and the "canonical form" is whatever the author wanted all along.
**Why it happens:** CFFP is rigorous and slow. The temptation is to go through the motions without genuine pressure testing.
**How to avoid:** Each counterexample must be minimal (per `docs/cffp.cue#Counterexample`: `minimal: bool & true`). Each proof sketch must be precise enough that two independent agents would agree on its meaning. Composition failures against existing constructs must be checked — not assumed absent. If a CFFP run produces zero counterexamples, that is a red flag, not a sign of quality.
**Warning signs:** A CFFP instance with empty `phase3.counterexamples` and empty `phase3.composition_failures`. Proof sketches that begin with "obviously" or "trivially."

## Code Examples

### CFFP Instance Structure (Persona Example Skeleton)

```json
{
  "protocol": {
    "name": "Constraint-First Formalization Protocol",
    "version": "0.2.1",
    "description": "Invariant-driven semantic design. Candidates survive pressure or die."
  },
  "construct": {
    "name": "Persona",
    "description": "Declared identity construct for authority gating in Operations and Flows",
    "depends_on": ["Fact", "Entity", "Rule", "Operation", "Flow", "TypeDecl"]
  },
  "version": "1.0",
  "phase1": {
    "invariants": [
      {
        "id": "I1",
        "description": "Every persona reference in an Operation allowed_personas set or Flow step persona field must resolve to a declared Persona construct",
        "testable": true,
        "structural": true,
        "class": "soundness"
      },
      {
        "id": "I2",
        "description": "Persona ids are unique within a contract",
        "testable": true,
        "structural": true,
        "class": "soundness"
      }
    ]
  },
  "phase2": { "candidates": ["..."] },
  "phase3": { "counterexamples": [], "composition_failures": [] },
  "derived": { "eliminated": [], "survivors": [] },
  "phase5": { "obligations": [], "all_provable": true },
  "phase6": { "canonical": { "...": "..." } },
  "outcome": "canonical",
  "outcome_notes": "..."
}
```

### Existing Interchange Format — Operation (Current State)

Source: `conformance/positive/operation_basic.expected.json`

```json
{
  "allowed_personas": ["reviewer", "admin"],
  "effects": [
    { "entity_id": "Order", "from": "submitted", "to": "approved" }
  ],
  "error_contract": ["precondition_failed", "persona_rejected"],
  "id": "approve_order",
  "kind": "Operation",
  "precondition": { "verdict_present": "account_active" },
  "provenance": { "file": "operation_basic.tenor", "line": 33 },
  "tenor": "0.3"
}
```

Note: `allowed_personas` is an array of bare strings. After SPEC-01 (Persona construct), these must resolve to declared Persona constructs. The interchange format may or may not change — this is a design decision for the Persona CFFP run.

### Existing Interchange Format — Flow OperationStep (Current State)

Source: `conformance/positive/flow_basic.expected.json`

```json
{
  "id": "step_submit",
  "kind": "OperationStep",
  "on_failure": { "kind": "Terminate", "outcome": "failure" },
  "op": "submit_order",
  "outcomes": { "success": "step_check_review" },
  "persona": "buyer"
}
```

Note: `outcomes` uses ad-hoc string labels ("success"). After SPEC-02 (P7 outcome typing), these labels must reference declared outcome types on the referenced Operation. The current spec (AL13) says "typed outcome routing is Flow-side classification only."

### Current Spec — Pending Work Items Relevant to Phase 1

From `docs/TENOR.md` Section 16:

```
P5 — Shared type library. Record and TaggedUnion types are per-contract in v0.3.
     Cross-contract type reuse is deferred.

P7 — Operation outcome typing. Named outcome types on Operations are deferred.
     Current typed outcome routing is Flow-side classification only.
```

Both are marked "Deferred to v2" in the current spec. Phase 1 un-defers them into v1.0.

## State of the Art

| Current Tenor State | Phase 1 Target | Impact |
|---------------------|----------------|--------|
| Personas are bare strings | Persona is a declared construct with id + optional metadata | Enables S4 authority topology analysis; closes spec gap TS-10 |
| Operation outcomes are Flow-side labels | Named outcome types declared on Operations | Enables static enumeration of all possible operation outcomes; type-safe flow routing |
| TypeDecl is per-contract, inlined | Shared types importable across contracts | Enables cross-contract type reuse; addresses composition at scale |
| No `tenor_version` field | Semver version field + JSON Schema | Enables tooling compatibility detection; interchange validation |
| Spec is v0.3 (pre-release) | Spec is v1.0 (frozen) | All downstream phases build on stable spec |

**Deprecated/outdated:**
- AL13 ("Flow typed outcomes are Flow-side only") will be superseded by P7 outcome typing
- The "Deferred to v2" label on P5 and P7 in Section 16 will be updated to "Resolved in v1.0"

## Open Questions

1. **Persona metadata fields — what specifically?**
   - What we know: CONTEXT.md says "optional metadata fields." The starting invariants mention resolution and uniqueness but not what metadata a Persona carries.
   - What's unclear: Does a Persona have description, display name, organizational role, or is it purely an identity token? The CFFP run must decide.
   - Recommendation: Start with minimal metadata (just id, perhaps description). Let the CFFP counterexample phase argue for more. Metadata that cannot be consumed by any evaluation, analysis, or generation step has no semantic value and should not be in the construct.

2. **P7 — Are outcomes exhaustive or open-ended?**
   - What we know: Starting invariant says "no implicit/catch-all outcomes." The set of outcomes is statically determinable.
   - What's unclear: Does every operation invocation produce exactly one declared outcome? What about error conditions (persona_rejected, precondition_failed) — are these outcomes or a separate error channel?
   - Recommendation: The current spec separates error conditions (`error_contract`) from outcomes. P7 should formalize the success-path outcomes while preserving the error channel. Counterexamples should test whether conflating errors and outcomes creates invariant violations.

3. **P5 — Structural vs nominal type identity**
   - What we know: CONTEXT.md identifies this as a hard subproblem. The spec currently uses structural typing (TypeDecl inlines to full BaseType; two Records with identical fields are the same type).
   - What's unclear: When types are shared across contracts, is `Contract_A.LineItem` the same type as `Contract_B.LineItem` if they have identical fields? Nominal typing says no (different origin = different type). Structural says yes. The choice affects import semantics fundamentally.
   - Recommendation: This is the core design question for P5. It may warrant sub-phasing the CFFP run (type identity first, import semantics second). Structural typing is simpler and aligns with current Tenor behavior. Nominal typing is safer for cross-contract boundaries but adds complexity. Let the CFFP invariants and counterexamples decide.

4. **JSON Schema — should it cover only the bundle level or individual construct documents?**
   - What we know: The decision says `docs/interchange-schema.json` defines the canonical structure.
   - What's unclear: Is this one schema or a set of schemas (one per construct kind)? JSON Schema 2020-12 supports `$ref` and `$defs` for modular schemas.
   - Recommendation: Single schema file with `$defs` for each construct kind. The top-level schema validates the Bundle; construct-level schemas are referenceable sub-schemas. This is the standard JSON Schema pattern for complex documents.

5. **Backward compatibility of interchange format**
   - What we know: Spec freeze means no breaking changes after Phase 1. Current format uses `"tenor": "0.3"`. New format will use `"tenor_version": "1.0.0"` (semver).
   - What's unclear: Must v1.0 interchange be readable by v0.3 tooling? Or is v0.3 → v1.0 a clean break?
   - Recommendation: The decision explicitly says "v0.3 → v1.0 is a major version bump." This implies a clean break. However, the JSON Schema should be designed for forward evolution (additive fields in minor versions). The `tenor_version` field enables tooling to detect version mismatches.

6. **How do Persona constructs affect the interchange format?**
   - What we know: Currently personas are bare strings in `allowed_personas` arrays and `persona` fields in steps.
   - What's unclear: Does adding a Persona construct mean: (a) Persona constructs appear as new items in the `constructs` array, AND existing persona string references remain unchanged? Or (b) persona string references are replaced with structured references? Or (c) some hybrid?
   - Recommendation: Let the CFFP run decide. Option (a) is most backward-compatible — Persona constructs are new declarations, existing string references become validated against declared Personas. This parallels how Fact references work (bare string fact_ref validated against declared Facts).

## Sources

### Primary (HIGH confidence)
- `docs/TENOR.md` v0.3 — Complete formal specification, 1,780+ lines. Source of truth for all existing construct definitions, evaluation model, elaboration spec, and acknowledged limitations.
- `docs/cffp.cue` v0.2.1 — CFFP protocol schema, 398 lines. Defines the exact structure every CFFP artifact must conform to.
- `conformance/` — 47 passing elaborator conformance tests. Provides the validation corpus for JSON Schema development and demonstrates current interchange format structure.
- `.planning/phases/01-spec-completion/01-CONTEXT.md` — User decisions including CFFP execution order, starting invariants, interchange versioning decisions, and spec freeze definition.

### Secondary (MEDIUM confidence)
- [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12) — Current stable JSON Schema standard. Recommended for new schema development.
- [JSON Schema Specification](https://json-schema.org/specification) — Official specification links and adoption guidance.
- `.planning/research/PITFALLS.md` — Project-level pitfalls research identifying spec drift, interchange ossification, and other risks directly relevant to Phase 1.
- `.planning/research/ARCHITECTURE.md` — Architecture patterns identifying the interchange format as the integration boundary between all tooling components.
- `.planning/research/SUMMARY.md` — Research summary confirming Phase 1 ordering rationale and spec-first discipline.

### Tertiary (LOW confidence)
- General ADT/tagged union formal specification patterns — training data, not verified against a specific standard. Relevant to P7 outcome typing design space.

## Metadata

**Confidence breakdown:**
- CFFP execution: HIGH — protocol is fully defined in `docs/cffp.cue`, project has clear documentation, all starting invariants provided
- Persona construct: HIGH — current state well-understood (bare strings in elaborator, no declaration), design space is bounded
- P7 outcome typing: HIGH — current state documented (AL13 in spec, Flow-side classification only), design space understood
- P5 shared types: MEDIUM — design space is large (structural vs nominal, import semantics, cross-contract elaboration), user acknowledges this is hardest, scoped-down outcome acceptable
- JSON Schema: HIGH — standard tooling, well-documented, interchange format is observable from 47 conformance tests
- Interchange versioning: HIGH — decision is locked (semver, `tenor_version` field, major version bump)

**Research date:** 2026-02-21
**Valid until:** 2026-03-21 (stable domain — specification work, not library versions)
