# Phase 3.1 Context: CFFP — Migration Semantics

## Critical Design Question (MUST CFFP)

**Should Tenor diffs be expressed as Tenor contracts?**

This is not a minor formatting decision — it's a foundational architectural question that must go through the full CFFP consensus protocol before any implementation.

### The Question

The current `tenor diff` (MIGR-01, Phase 3) outputs a structural JSON diff of two interchange bundles: added/removed/changed constructs with field-level before/after values. This is a *data format about* Tenor contracts.

The alternative: the diff output is itself **a Tenor contract** — a migration contract that uses the DSL's own constructs to describe the transformation between two contract versions.

### Why This Matters

If diffs are Tenor contracts, then:

1. **Tenor describes itself.** The language can express its own evolution. A migration is not metadata *about* a contract — it *is* a contract.
2. **Migrations are evaluable.** The same evaluator that processes business contracts can evaluate migration contracts. You can ask "is this migration valid?" using the same toolchain.
3. **Migrations are diffable.** You can diff two migrations. You can diff a migration against its inverse. The toolchain is recursive.
4. **Migrations are composable.** Chain migration contracts together. Compose v1→v2 with v2→v3 to get v1→v3. This is contract composition applied to versioning.
5. **Migrations are validatable.** Static analysis (S1-S7) applies to migration contracts too. You can check reachability, authority, completeness of a migration just like any other contract.
6. **Migrations are testable.** Write conformance fixtures for migrations using the same fixture format. The entire test infrastructure applies.
7. **Self-hosting potential.** If Tenor can describe its own structure and transformations, that's a path toward Tenor-in-Tenor — spec sections expressible in the language they define.

### What the CFFP Run Must Evaluate

This is not just "what format should diff output?" — it's "what is the ontological status of a migration in Tenor?"

**Candidate A: Interchange diff (current)**
- Output is structured JSON describing structural changes
- Simple, predictable, easy to implement
- Migration is metadata *about* contracts, not a contract itself
- Phase 4's `--breaking` adds classification labels to the same JSON structure
- No recursion, no self-reference, no composition

**Candidate B: Migration as Tenor contract**
- Output is a `.tenor` file (or equivalent DSL) describing the transformation
- Entities could represent construct lifecycle (Added, Removed, Changed)
- Rules could express breaking-change classification
- Operations could describe the migration steps
- Flows could sequence multi-step migrations
- Facts could capture the before/after state
- The evaluator produces verdicts about whether a migration is safe/complete/valid
- Unlocks: diffing diffs, composing migrations, validating migrations, self-hosting

**Candidate C: Hybrid**
- Structural diff for raw output (`tenor diff`)
- Migration contract generation for `tenor diff --migration`
- Both representations, different use cases

### Pressure Test Questions

The CFFP run should stress-test Candidate B especially hard:

1. Can every construct change type (entity state add/remove, fact type widen/narrow, rule add/remove, verdict removal, flow step changes) be naturally expressed in Tenor's existing constructs?
2. What new constructs (if any) would be needed? Does this violate the v1.0 freeze?
3. Is there a clean mapping from "diff of two contracts" to "contract describing the diff"?
4. Can migration contracts reference the contracts they migrate? What are the import semantics?
5. Does this create circular dependencies? (A migration contract imports the thing it's migrating, which may itself have been produced by a migration.)
6. Is the evaluator powerful enough to evaluate migration contracts, or would it need new capabilities?
7. What does "compose two migration contracts" actually look like in the DSL?

### Decision Owner

This decision shapes MIGR-02, MIGR-03, MIGR-04, MIGR-05, and potentially the entire Phase 4 `--breaking` implementation. It must be resolved in Phase 3.1 before Phase 4 planning begins.
