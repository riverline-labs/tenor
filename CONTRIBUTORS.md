# Contributors

## People

| Name | Role | Contributions |
|------|------|---------------|
| Brandon W. Bush | Creator | Language design, spec authoring, elaborator implementation, decision protocol design, toolchain architecture |
| Javier Muniz | Core contributor | Domain expertise, contract versioning/migration requirements, flow migration compatibility analysis |

## AI Contributors

Tenor was designed through human-AI collaborative sessions. The following AI systems participated in spec design, pressure testing, and convergence validation.

| System | Provider | Contributions |
|--------|----------|---------------|
| Claude | Anthropic | Spec co-authoring, [CFFP](https://github.com/riverline-labs/iap) runs (persona, outcome typing, shared types, migration semantics), elaborator implementation, conformance suite, toolchain development |
| ChatGPT | OpenAI | Spec pressure testing, convergence validation, entity authority semantics debate (see Appendix B) |
| DeepSeek | DeepSeek | Spec pressure testing, convergence validation, entity hierarchy semantics debate (see Appendix B) |
| Gemini | Google | Spec pressure testing, convergence validation |
| Grok | xAI | Spec pressure testing |

## Method

This project uses protocols from the [Interpretive Adjudiciation Protocols](https://github.com/riverline-labs/iap) suite:

- **[CFFP](https://github.com/riverline-labs/iap)** (Constraint-First Formalization Protocol) — invariant declaration, candidate formalisms, pressure testing via counterexamples, canonical form selection. Used for all construct additions (persona, outcome typing, shared types, migration semantics).
- **[ADP](https://github.com/riverline-labs/iap)** (Adversarial Design Protocol) — design space mapping before formalization. Used to arrive at the §18 Contract Discovery & Agent Orientation design (Phase 3.4).
- **[RCP](https://github.com/riverline-labs/iap)** (Reconciliation Protocol) — verifies consistency across multiple protocol run outputs. Used to reconcile §18 design decisions with existing spec commitments.
