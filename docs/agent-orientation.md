# Tenor — Agent Orientation

You are working on Tenor, a behavioral contract calculus. This document is the context you need before writing any code, reviewing any plan, or making any architectural decision. Read it fully. Internalize the constraints. Every design choice in the codebase traces back to what's described here.

---

## What Tenor Is

Tenor is a formal language and runtime that turns business processes into physics. A Tenor contract declares the complete observable behavior of a system: what entities exist and their state machines, what facts the world can observe, what rules (stratified by dependency) produce what verdicts, who can act (personas), what operations are available with what preconditions and effects, and what flows orchestrate them.

The contract is not a description of a system. It _is_ the system. Application code doesn't implement business logic — it renders state and relays decisions. The runtime enforces the contract the way a chess board enforces the rules of chess. You don't ask the pieces to follow the rules. The board only permits legal moves.

This means: an agent (human or AI) operating inside Tenor doesn't need to be trusted to follow rules. The rules are the environment. Impermissible actions don't exist in the action space. A random agent and a genius agent have identical safety guarantees because safety is structural, not behavioral.

---

## The Seven Constraints

Everything in Tenor derives from seven formal constraints. These are not implementation preferences. They are the physics. If you find yourself making a decision that violates one, stop — you're wrong, even if the code would work.

**C1 — Closed world.** The contract declares everything that exists. If it's not in the contract, it doesn't exist. No implicit state, no ambient authority, no external side effects during evaluation.

**C2 — Stratified rules.** Rules are organized into strata where each stratum depends only on facts and verdicts from prior strata. No circular dependencies. Evaluation order is deterministic and finite.

**C3 — Deterministic evaluation.** Given the same contract, facts, and entity states, any conforming evaluator produces the same verdicts. Always. On any platform.

**C4 — Finite state machines.** Entities have declared states and declared transitions. You cannot invent a state at runtime. You cannot transition to a state that isn't declared as a valid target.

**C5 — Atomic transitions.** State transitions are all-or-nothing. If any step in a flow fails, nothing commits. There is no partial execution.

**C6 — Complete provenance.** Every state transition records: which operation executed it, which verdicts enabled it, which facts those verdicts examined, which persona initiated it, and when. The chain is unbroken from transition back to raw fact values.

**C7 — Formal verifiability.** The contract's behavior can be statically analyzed. The state graph is finite and enumerable. The authority topology (who can do what under what conditions) is computable from the contract alone.

---

## The Architectural Split

Tenor has two layers. The boundary between them is the most important line in the architecture. Every file you touch lives on one side or the other. Never cross it.

### Read Path — Advisory (Open Source, Apache 2.0)

Repository: `riverline-labs/tenor` (public)
Local path: `~/src/riverline/tenor`

The evaluator takes a contract, facts, and entity states. It produces verdicts and an action space. It is **pure computation** — no database, no side effects, no network calls, no authority. It runs as:

- A Rust crate (`tenor-eval`)
- A WASM module (`tenor-eval-wasm`) for browsers and edge
- An OSS HTTP server (`tenor serve`) for development and testing

The evaluator answers: "What's true? What can I do? What's blocked and why?" These are advisory answers. The evaluator doesn't change anything. It observes and computes.

**What lives here:**

- The spec and formal language definition
- The elaborator (DSL → interchange JSON)
- The evaluator (contract + facts + entity states → verdicts + action space)
- The WASM evaluator
- The storage trait and conformance suite (the _interface_, not an implementation)
- The `AgentPolicy` trait and reference policies (random, first-available, priority)
- The `ActionSpace`, `Action`, `BlockedAction` types
- The OSS serve (advisory HTTP endpoints, no database)
- The LSP, CLI tools (`tenor check`, `tenor explain`)
- All documentation

**What must never be here:**

- Any database driver or connection logic (no `sqlx`, no `postgres`, no `DATABASE_URL`)
- Any code that commits state transitions
- Any reference to the private repo
- Any executor logic
- Any platform-specific deployment code

**The one-question test:** Does this code need a database to function? If yes, it doesn't belong here.

### Write Path — Authoritative (Commercial, ELv2)

Repository: `riverline-labs/tenor-platform` (private)
Local path: `~/src/riverline/tenor-platform`

The executor takes the evaluator's output and makes it real. It commits atomic state transitions to Postgres with full provenance. It enforces what the evaluator advises.

**What lives here:**

- The executor (`tenor-executor`) — atomic flow execution with provenance
- The storage implementation (`tenor-storage-postgres`) — Postgres backend
- The platform serve (`tenor-platform-serve`) — authoritative HTTP endpoints
- The platform CLI (`tenor-platform`) — deployment, migration, entity management
- The agent runtime (`tenor-agent-runtime`) — observe/evaluate/choose/execute loop

**What must never be here:**

- Evaluation logic (that's the public evaluator's job)
- Rule evaluation, verdict production, action space computation (call the public crate)
- Any re-implementation of something that exists in the public evaluator

**The one-question test:** Does this code commit state changes or require a database? If yes, it belongs here.

### Dependency Direction

Private depends on public. Public has zero knowledge of private. The private repo references `tenor-eval` and `tenor-storage` via git dependency:

```
tenor-eval = { git = "https://github.com/riverline-labs/tenor.git", path = "crates/eval" }
tenor-storage = { git = "https://github.com/riverline-labs/tenor.git", path = "crates/storage" }
```

After pushing changes to the public repo, run `cargo update -p tenor-eval -p tenor-storage` in the private repo. Never the reverse — the public repo never pulls from private.

---

## The Action Space

The action space is the central abstraction for agents. It answers three questions simultaneously:

1. **What can I do?** — `actions`: every flow executable right now by this persona, with the verdicts that enable it and the entities it affects.
2. **What's blocked and why?** — `blocked_actions`: every flow that exists but can't execute, with the specific reason (missing verdict, wrong entity state, unauthorized persona).
3. **What's true?** — `current_verdicts`: the full verdict set from evaluation.

An agent that only sees available actions is reactive. An agent that also sees blocked actions and their reasons can plan toward unblocking them. The contract provides both views. This is the difference between a button-pusher and a strategist.

`compute_action_space()` lives in the public evaluator. It is pure — no database, no side effects. The platform serve's `POST /actions` endpoint wires it to the database (reads current entity states, calls the pure function, returns the result). The agent runtime's observe/evaluate cycle does the same thing through the Rust API instead of HTTP.

---

## The Agent Runtime

The agent loop is: **observe → evaluate → choose → execute**.

- **Observe:** Read current entity states from storage. Read or receive facts. Build the snapshot.
- **Evaluate:** Call `compute_action_space()` with the contract, facts, entity states, and persona. Get back the full action space.
- **Choose:** The `AgentPolicy` trait picks an action (or returns `None` to stop). The policy receives both the action space and the snapshot — it knows what's possible, what's blocked and why, and the full domain context.
- **Execute:** Call the executor with the chosen action. The executor runs the flow, checks preconditions authoritatively, commits atomic transitions, records provenance.

The `AgentPolicy` trait:

```rust
#[async_trait]
pub trait AgentPolicy: Send + Sync {
    async fn choose(
        &self,
        action_space: &ActionSpace,
        snapshot: &AgentSnapshot,
    ) -> Option<Action>;
}
```

The policy is the only part that varies between agents. Everything else — observation, evaluation, execution, provenance — is identical. A random policy and an LLM policy go through the same enforcement path. Safety doesn't change because safety is in the contract, not the policy.

The `AgentPolicy` trait and reference implementations live in the **public repo**. The agent runtime loop that wires them to the executor lives in the **private repo**. The trait is open so anyone can implement policies. The loop is commercial because it requires the executor.

---

## Provenance

Every state transition must be traceable back to the raw facts that caused it. The provenance chain is:

```
Entity transition
  ← caused by operation execution
    ← authorized by persona
    ← preconditions met because verdicts were true
      ← verdicts produced by rules in specific strata
        ← rules evaluated specific facts with specific values
```

The evaluator (public) produces verdict provenance: which rule, which stratum, which facts were examined, which other verdicts were used. The executor (private) records this alongside the transition: execution ID, operation, persona, timestamp, verdict snapshot, facts used.

If `facts_used` is empty anywhere in the chain, provenance is incomplete. This is a bug, not a feature gap.

---

## Working Across Repos

When a change spans both repos (common for evaluator API changes):

1. Make the change in the public repo first
2. Run all quality gates in the public repo
3. Commit and push to GitHub
4. In the private repo: `cargo update -p tenor-eval -p tenor-storage`
5. Update callers in the private repo
6. Run all quality gates in the private repo
7. Commit

Quality gates for both repos:

```bash
cargo fmt --all
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

The private repo additionally requires:

```
DATABASE_URL=postgres://postgres:postgres@localhost/tenor
DOCKER_API_VERSION=1.43
```

### WASM Crate

`tenor-eval-wasm` is NOT a workspace member (WASM crates break `cargo build --workspace` because the target is different). It has its own build and test commands:

```bash
cd crates/tenor-eval-wasm
wasm-pack build --target nodejs
wasm-pack test --node
```

---

## What Not To Do

These are invariants. If you find yourself doing any of these, you've gone wrong somewhere.

- **Don't put database code in the public repo.** Not even behind a feature flag. The evaluator is pure.
- **Don't reimplement evaluation in the private repo.** Call `tenor-eval`. If the evaluator doesn't expose what you need, add it to the evaluator and push.
- **Don't make the evaluator impure.** No network calls, no filesystem access, no database reads during evaluation. Facts come in as parameters. Verdicts come out as return values.
- **Don't treat safety as a policy concern.** Safety is environmental. The contract constrains the action space. The policy picks from legal moves. If you're writing code that checks "is this agent allowed to do X" — that check belongs in the contract as a precondition, not in the runtime as a guard.
- **Don't break determinism.** Same contract + same facts + same entity states = same verdicts. Always. If you're introducing randomness, timestamps, or external state into evaluation, stop.
- **Don't create partial execution paths.** Flows are atomic. Every step succeeds or nothing commits. If you're writing code that commits some transitions but not others, stop.
- **Don't invent types that exist.** `EntityStateMap`, `VerdictSet`, `ActionSpace`, `FlowEvalResult`, `AgentSnapshot` — these are defined. Use them. Don't create parallel types in the private repo.
- **Don't skip provenance.** Every operation execution records the full chain. If a code path commits a transition without recording which verdicts enabled it and which facts those verdicts examined, it's broken.
- **Don't confuse advisory and authoritative.** The evaluator advises. The executor enforces. A simulation (advisory) must never write to the database. An execution (authoritative) must always go through the executor, never by directly writing to storage.

---

## The End State

When everything works:

1. An organization writes a contract (with LLM assistance — the closed-world spec is easier for LLMs to author correctly than general-purpose code)
2. They simulate until it matches intent
3. They deploy the runtime
4. They attach decision-makers — human UIs, AI agents, or both
5. Every action is pre-authorized by the contract, every transition is atomic, every outcome is provenanced
6. A random agent and a genius agent have identical safety guarantees

The LLM is both the author and the actor. The contract is the boundary between the two roles. The spec guarantees that boundary is formally enforced.
