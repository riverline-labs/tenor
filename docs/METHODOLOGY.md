# Design Methodology

When Tenor confronts a major linguistic decision where multiple candidate formalisms could work and the winner is not obvious, we use protocols from the [Dialectics](https://github.com/riverline-labs/dialectics) suite.

## CFFP — Constraint-First Formalization Protocol

Used when we need to add or modify a construct and there are several viable candidate formalisms.

How it works: declare invariants up front, propose candidate formalisms, pressure-test each candidate with counterexamples, and let the candidates compete. One survivor emerges — either as-is, collapsed with another, or they all die and we start over. This is dialectics, not design. The protocol doesn't produce solutions; it selects them.

Used for: persona semantics, outcome typing, shared types, migration semantics, source declarations, multi-instance entities, trust obligations.

## ADP — Adversarial Design Protocol

Used when the solution space is unknown and must be explored before formalization. Maps the design space adversarially before committing to candidates.

Used for: contract discovery and agent orientation (§19).

## RCP — Reconciliation Protocol

Used to verify consistency across multiple protocol run outputs — ensures decisions from separate runs don't contradict each other.

Used for: reconciling §19 design decisions with existing spec commitments.
