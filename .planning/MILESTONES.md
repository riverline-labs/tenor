# Milestones

## v0.9 Core (Shipped: 2026-02-22)

**Phases completed:** 14 phases, 46 plans
**Timeline:** 2 days (2026-02-21 → 2026-02-22)
**Codebase:** 24,543 LOC Rust · 2,719 LOC spec · 8,304 LOC conformance · 6,441 LOC domain contracts

**Key accomplishments:**
- CFFP-formalized v0.9 spec: 10 constructs, migration semantics, contract discovery (§1-§22, 2,719 lines)
- 6-pass elaborator producing deterministic interchange JSON validated against JSON Schema
- Provenance-traced evaluator with fixed-point decimal arithmetic, ParallelStep/Compensate/Escalate flow handlers
- S1-S8 static analysis suite: entity state enumeration, rule reachability, authority topology, flow path analysis, complexity bounds, `tenor diff --breaking`
- Five-domain validation: SaaS subscriptions, healthcare prior auth, supply chain inspection, energy procurement, trade finance
- AI ambiguity testing harness comparing LLM verdicts against elaborator ground truth
- Effect-to-outcome mapping, exists quantifier, S6 escalation fix, CLI flow evaluation (gap closure)

---

## v1.0 System Construct + Documentation (Shipped: 2026-02-22)

**Phases completed:** 4 phases, 17 plans
**Timeline:** 1 day (2026-02-22)
**Spec:** Frozen at v1.0 including System construct (§15), AAP audited

**Key accomplishments:**
- System construct for multi-contract composition: shared persona identity, cross-contract flow triggers, cross-contract entity relationships
- CFFP-derived design (Candidate A: dedicated .tenor file with centralized member declaration)
- 4 executor obligations (E-SYS-01 to E-SYS-04) for cross-contract coordination
- AAP spec audit: all hidden assumptions surfaced, resolved or documented as acknowledged limitations
- All 5 domain contracts re-validated for v1.0 spec; multi-contract System scenario (supply chain + trade finance) validated end-to-end
- Author guide (5 parts, real domain contract patterns), one-page explainer for decision makers, README v1.0 update
- Logic conformance vs. operational conformance distinction added to trust model (§17.1.1)
- 72 conformance tests passing, 384 Rust tests passing

---

