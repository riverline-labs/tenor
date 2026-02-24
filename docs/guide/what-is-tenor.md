# What Is Tenor?

Every business runs on decisions. Who can approve this payment? Under what conditions does escrow release? When a dispute arrives, whose system's word is final?

Today, the answers are scattered—some live in code, some in configuration files, some in spreadsheets, some in the heads of people who might leave. When a regulator asks "who authorized this and why?", the answer requires archaeology. When a compliance officer asks "could this ever happen?", the honest answer is usually "we think not, but we can't prove it."

Tenor eliminates both problems. It gives you a single, formally verifiable description of the behavioral contract governing your process—one that can be analyzed before it runs and audited long after.

---

## What Tenor Guarantees

A Tenor contract is not documentation. It is a formal specification that a machine can analyze statically. From the contract alone—without executing it—an analyzer can derive:

**Every possible state an entity can occupy**, and every state reachable from its starting point. If your escrow account has five states, the analyzer knows all five and exactly which sequences lead to each.

**Every operation every participant can perform in every state.** If only the escrow agent can release funds while an account is on hold, the analyzer confirms that no other participant, in any other state, can trigger that release. This is a structural property of the contract, not a test result.

**Every possible verdict your rules can produce**, and every possible outcome of every operation. The space of outcomes is finite, enumerable, and known before the first transaction executes.

**Every path through every workflow**—who acts at each step, what outcomes are possible, and where the workflow can terminate. When a compliance officer asks "how can this approval process end?", the answer is a complete list derived from the contract, not a best-effort trace of past executions.

**Bounded evaluation complexity** for every expression and workflow. Before execution, the system confirms that no rule evaluation exceeds a known computational bound. No runaway queries. No unbounded loops.

**Unique verdict provenance.** Each verdict type is produced by exactly one rule. If two rules could produce the same verdict, the contract is rejected at analysis time. When you see a verdict, you know precisely which rule produced it and which facts led there.

These are not aspirational properties. They are enforced by the language itself. A contract that violates any of them is rejected at analysis time.

---

## What Tenor Produces

Every decision in a Tenor contract carries its complete derivation. When a rule fires, the output includes the exact facts examined, the predicate evaluated, and the rule that applied. This is not a log entry appended after the fact—it is a structured proof generated as an intrinsic part of evaluation.

The chain of provenance is mathematical, not procedural. A traditional audit trail records *what happened*: this function was called, this value was checked, this result was returned. A Tenor derivation records *why a decision was reached*: these facts existed, this predicate held, this rule applied, this verdict followed. The distinction matters when the question is not "what did the system do?" but "was the system correct to do it?"

The derivation is deterministic. Given the same facts, the same contract always produces the same verdicts with the same proof structure—whether in production, a test environment, or a regulatory review.

---

## What Tenor Enables

Because a Tenor contract is a self-contained formal specification, anyone with access to it can independently verify its behavior. A regulator does not need access to your production system, your source code, or your engineering team. They can take the contract, run it through their own conforming analyzer, and confirm that the properties you claim actually hold.

If you say "no unauthorized participant can release a payment," a regulator can check that claim against the contract directly. If you say "every approval requires sign-off from at least two independent reviewers," that claim is either provable from the contract or it isn't. There is no ambiguity, no reliance on testing coverage, and no dependence on your particular implementation.

When both parties to a transaction can run the same contract against the same facts and produce identical provenance chains, there is no "our records say" versus "their records say." There is one answer, derivable by anyone.

This is what independent auditability means in practice. The contract is the specification. The specification is checkable. Anyone can check it.

---

## What Tenor Does Not Do

Tenor describes the behavioral contract: who can do what, under what conditions, with what guarantees, producing what verdicts. It does not build the application around that contract. The user interface, the database, the network layer, the deployment infrastructure—that remains engineering work.

What Tenor ensures is that the behavioral layer—the layer that determines whether a transaction is authorized, whether a claim is approved, whether a workflow can proceed—is formally specified, statically verifiable, and independently auditable. Everything else is built on a foundation whose correctness is proven, not assumed.