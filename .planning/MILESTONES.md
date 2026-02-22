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

