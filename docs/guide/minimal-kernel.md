# Tenor — Minimal Kernel

The minimum an AI agent or human needs to know to read, write, audit, and reason about a Tenor contract correctly. Everything else in the spec is precision and edge cases. This is the mental model.

## 1. Facts Are the Ground Truth

Every value the system reasons about comes from outside. Facts are declared, typed, and sourced — never computed internally. If a value isn't declared as a Fact, it doesn't exist in the evaluation. There is no hidden state. There are no implicit inputs. The FactSet is assembled once and is immutable from that point forward.

**What to look for:** `fact` blocks. Check that every value the contract reasons about has a declared source. If something seems to appear from nowhere, the contract is wrong.

## 2. Rules Build Upward, Never Sideways

Rules are stratified. Stratum 0 reads Facts. Stratum 1 reads Stratum 0 verdicts. Stratum N reads verdicts from strata strictly below N. No rule ever reads from its own stratum or above. This guarantees termination and eliminates circular reasoning. Each rule produces exactly one verdict type — no two rules produce the same verdict.

**What to look for:** `rule` blocks and their stratum numbers. Trace the dependency chain downward. If a rule at stratum 2 references a verdict, that verdict must come from stratum 0 or 1. If two rules produce the same verdict type, the contract is invalid.

## 3. Entities Are Finite State Machines

Every domain object has a fixed set of states and a fixed set of legal transitions between them. No state is implied. No transition is implicit. If a transition isn't declared, it cannot happen. The entity graph is a DAG — no cycles in the hierarchy. Entity state is the only mutable thing in the system, and only Operations can change it.

**What to look for:** `entity` blocks. Read the states and transitions. Then check every operation's effects — each effect must be a declared transition. If an operation tries to move an entity along a path that isn't in the transition list, it's invalid.

## 4. Operations Are the Only Way to Change State

An Operation is a persona-gated, precondition-guarded state transition. Every operation declares: who can invoke it (personas), what must be true for it to execute (precondition over verdicts), and what changes (entity state transitions). Effects are atomic — all transitions in an operation happen together or none do. Operations also declare their possible outcomes, which flows use for routing.

**What to look for:** `operation` blocks. For each one, ask three questions: Who can do this? What has to be true first? What changes? If any of those is missing or too broad, the contract has an authority gap.

## 5. Flows Are Frozen

A Flow orchestrates Operations into a DAG of steps. At flow initiation, the system takes a snapshot: the complete FactSet and the complete VerdictSet computed from those facts. This snapshot is frozen for the entire flow. Operations within the flow change entity state, but the verdicts are never recomputed. Every branch condition, every precondition check, every routing decision within the flow operates against the snapshot taken at initiation — not against live state.

This is the most important thing to internalize. If an operation at step 3 transitions an entity from "draft" to "published," a branch at step 4 that checks a verdict depending on entity state will NOT see "published." It sees whatever the verdicts were at initiation. Mid-flow re-evaluation does not happen. If you need verdicts that reflect state changes, you need a new flow.

**What to look for:** Branch conditions in flows that seem to depend on state changes made by earlier steps in the same flow. That's a bug — the branch will evaluate against the frozen snapshot, not current state.

## 6. Provenance Is the Proof

Every verdict records which facts and lower-stratum verdicts produced it. Every operation records which verdicts satisfied its precondition, which persona invoked it, and which entity states changed. Every flow records the complete sequence of steps taken. The provenance chain is not a log — it's a structural derivation. You can trace any terminal outcome backward through operations, through verdicts, through rules, all the way down to the ground facts that caused it.

**What to look for:** When reviewing a contract's behavior, don't just ask "what happened." Ask "can I trace this outcome back to specific facts?" If you can't, something is missing from the contract — either a fact isn't declared, a rule doesn't produce a verdict it should, or an operation's precondition doesn't reference the right verdicts.

## The 30-Second Audit

Given a new Tenor contract, check these six things:

1. **Facts complete?** Does every value the system needs have a declared, typed fact with a source?
2. **Strata clean?** Do rules build strictly upward with no same-stratum or cross-stratum cycles?
3. **Transitions legal?** Does every operation effect match a declared entity transition?
4. **Authority gated?** Does every operation have a non-empty persona set and a meaningful precondition?
5. **Flow frozen?** Do any branch conditions in flows assume mid-flow state changes? (If yes: bug.)
6. **Provenance traceable?** Can every possible terminal outcome be traced back through verdicts to facts?

If all six pass, the contract is structurally sound. If any fails, you've found a defect without executing a single line of code.

## The Closed-World Guarantee

The reason this minimal kernel works is that Tenor is closed-world. If something isn't declared in the contract, it doesn't exist. There are no ambient authorities, no implicit behaviors, no external references that affect evaluation. The contract is the complete description of the system. An agent that knows these six concepts can fully understand any Tenor contract without reading implementation code, configuration files, or infrastructure definitions.