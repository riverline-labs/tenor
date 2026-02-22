# Phase 1: Spec Completion - Context

**Gathered:** 2026-02-21
**Status:** Ready for planning

<domain>
## Phase Boundary

Formalize three new constructs (Persona declaration, P7 Operation outcome typing, P5 Shared type library) through CFFP and version the interchange format. Output is updated `docs/TENOR.md`, CFFP artifacts per construct, and a formal JSON Schema for the interchange format. No elaborator implementation in this phase — that's Phase 2.

</domain>

<decisions>
## Implementation Decisions

### CFFP execution order
- Serial, not parallel: Persona → P7 → P5
- Persona first because it's simplest and P7/P5 composition tests may reference it
- P7 second because outcome typing affects how Operations compose with Flows — P5 needs this settled
- P5 last because shared types cross contract boundaries and composition tests need both Persona and P7 canonicalized
- Full CFFP rigor on all three constructs — these are language semantics, not features. Every construct gets: invariant declaration, candidate formalisms with proof sketches, counterexample pressure, composition testing against existing canonicalized constructs (Fact, Entity, Rule, Operation, Flow, TypeDecl), and canonicalization

### CFFP artifact format
- Each construct produces a CFFP instance document (following `docs/cffp.cue` schema)
- Artifacts stored as `docs/cffp/persona.json`, `docs/cffp/p7-outcome-typing.json`, `docs/cffp/p5-shared-types.json`
- The `depends_on` field in each CFFP instance lists all previously canonicalized constructs
- Composition failures against existing constructs are tested explicitly in Phase 3 of each CFFP run

### Construct design starting points
- **Persona**: Declared construct with `id` (string) and optional metadata fields. Currently implicit in Operation `authorized_by` and Flow persona references. The CFFP run formalizes: what metadata personas carry, whether personas form hierarchies (delegation), and how persona references are validated across constructs. Starting invariants: every persona reference in an Operation or Flow must resolve to a declared Persona; persona ids are unique within a contract.
- **P7 — Operation outcome typing**: Operations currently produce untyped outcomes that Flows classify ad-hoc. CFFP formalizes: named outcome types declared on Operations, static enumerability (the set of possible outcomes is known at elaboration time), and how Flow transitions reference specific outcomes. Starting invariants: every Flow transition must reference a valid outcome of the referenced Operation; the set of outcomes is statically determinable; no implicit/catch-all outcomes.
- **P5 — Shared type library**: Cross-contract type reuse for Record and TaggedUnion. CFFP formalizes: import semantics for types (how one contract references types from another), type identity (structural vs nominal), and cross-contract elaboration. Starting invariants: type resolution terminates; imported types compose with local types without ambiguity; circular type imports are detected and rejected.

### Interchange versioning
- Add `tenor_version` field to the interchange JSON bundle (top-level, string, semver format e.g. "1.0.0")
- Semantic versioning for the interchange format: major = breaking structural changes, minor = additive fields/constructs, patch = fixes to serialization behavior
- The v0.3 → v1.0 transition is a major version bump (new constructs change the format)
- Formal JSON Schema (`docs/interchange-schema.json`) defines the canonical structure — generated from spec, not reverse-engineered from serializer output
- Schema validation becomes a conformance test: every elaborator output must validate against the schema

### Spec freeze definition
- "Frozen" means: no breaking changes to existing construct semantics after Phase 1
- Additive changes (new analysis results, new interchange metadata) are allowed but must be documented
- Any change to construct semantics after Phase 1 requires a new CFFP run — the cost is intentionally high
- `docs/TENOR.md` gets a version header: "Tenor Language Specification v1.0"
- Spec sections for persona, P7, and P5 follow the same structure as existing construct sections (syntax, semantics, elaboration rules, interchange representation, validation rules)

### Claude's Discretion
- Internal structure of CFFP artifact documents (JSON vs markdown — follow what's natural for the schema)
- Specific invariant classifications (termination, determinism, etc.) — derive from the construct's nature
- Number of candidate formalisms per construct — generate as many as the design space warrants
- Counterexample generation strategy — target weak points in candidate claims
- JSON Schema tooling (hand-authored vs generated) — whatever produces a correct, maintainable schema
- Whether P5 needs sub-phases within its CFFP run (type identity is a hard subproblem)

</decisions>

<specifics>
## Specific Ideas

- CFFP protocol is defined in `docs/cffp.cue` — use its schema for all CFFP artifacts
- The user's long-term vision is to express CFFP itself in Tenor (dogfooding), so the CFFP artifacts produced here may later become a domain validation contract
- Existing spec is `docs/TENOR.md` (v0.3, ~1,780 lines) — the source of truth for all existing construct definitions
- P5 is explicitly the hardest and was flagged in the original roadmap as "needs authoring experience from Phase 2 to scope correctly" — the CFFP run may produce a scoped-down canonical form with acknowledged limitations, which is a valid CFFP outcome
- Research flagged spec drift as the #1 risk — the spec-first discipline established here sets the tone for all subsequent phases

</specifics>

<deferred>
## Deferred Ideas

- Expressing CFFP in Tenor itself — domain validation exercise for Phase 5
- P5 module federation (inter-org type sharing) — explicitly out of scope for 1.0, per PROJECT.md
- Generic type parameters for Records — v2 requirement (SPEC-07)

</deferred>

---

*Phase: 01-spec-completion*
*Context gathered: 2026-02-21*
