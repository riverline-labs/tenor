# Tenor — The Behavioral Contract Calculus

Your business logic lives in five places. The API spec describes endpoints. The RBAC config describes permissions. The state machine library describes transitions. The workflow engine describes orchestration. The implementation code describes what actually happens — and it disagrees with the other four.

When a regulator asks "who authorized this?", the answer takes a week. When an engineer asks "can this ever happen?", the honest answer is "we think not." When you attach an AI agent to the system, it hallucinates authority because the rules aren't stated anywhere it can read.

Tenor makes business processes into something you can prove things about.

A Tenor contract is a single formal artifact that declares the complete observable behavior of a system: what exists, who can act, under what conditions, with what effects. From the contract alone — without executing anything — a machine can derive every reachable state, every authority boundary, every possible outcome, and the complete chain of reasoning behind any decision.

The contract is not documentation about a system. It *is* the system's behavioral specification. Application code renders state and relays decisions. The runtime enforces the contract the way a chess board enforces the rules of chess. You don't ask the pieces to follow the rules. The board only permits legal moves.

---

## The Five-Step Journey

Tenor is useful at every step. You don't need to adopt the full stack to get value. Each step stands alone, and each makes the next one possible.

### Step 1: Understand Your Domain

Before writing a contract, you need to answer four questions about your process:

**What are the things?** Every domain has entities — objects that move through states. An escrow account goes from held to released or refunded. A subscription goes from trial to active to suspended to cancelled. A prior authorization goes from submitted to under review to approved or denied. These are your entities.

**What does the world tell you?** Every decision depends on external data. The delivery service says the package arrived. The billing system says the payment cleared. The inspection report says the cargo passed. These are your facts — typed, sourced, declared. The contract doesn't compute them. It accepts them from named systems and reasons about them.

**Who are the decision-makers?** Every action needs someone authorized to take it. The escrow agent releases funds. The account admin upgrades a plan. The medical director approves an authorization. These are your personas — named authorities with bounded power.

**What are the rules?** Given the facts, what conclusions follow? If delivery is confirmed and all line items are valid, then release is approved. If the account is past due and the grace period has expired, then suspension is warranted. These are your rules — stratified, deterministic, producing named verdicts that operations check before executing.

This is the domain model. Every Tenor contract starts here. If you can answer these four questions, you can write a contract.

### Step 2: Write the Contract

A Tenor contract is a `.tenor` file. Here is a real one — a SaaS subscription lifecycle:

```tenor
entity Subscription {
  states:  [trial, active, suspended, cancelled]
  initial: trial
  transitions: [
    (trial, active), (trial, cancelled),
    (active, suspended), (active, cancelled),
    (suspended, active), (suspended, cancelled)
  ]
}

fact payment_ok {
  type:    Bool
  source:  "billing_service.payment_status"
  default: true
}

fact current_seat_count {
  type:   Int(min: 0, max: 10000)
  source: "identity_service.active_users"
}

fact plan_features {
  type:   PlanFeatures
  source: "plan_service.features"
}

rule seats_within_limit {
  stratum: 0
  when:    current_seat_count <= plan_features.max_seats
  produce: verdict seats_ok { payload: Bool = true }
}

rule payment_current {
  stratum: 0
  when:    payment_ok = true
  produce: verdict payment_verified { payload: Bool = true }
}

rule can_activate {
  stratum: 1
  when:    verdict_present(seats_ok)
         ∧ verdict_present(payment_verified)
  produce: verdict activation_approved { payload: Bool = true }
}

operation activate_subscription {
  allowed_personas: [account_admin, billing_system]
  precondition:     verdict_present(activation_approved)
  effects:          [(Subscription, trial, active)]
}
```

Notice what this says without any implementation code:

- The subscription has exactly four states. No hidden states emerge at runtime.
- Only `account_admin` and `billing_system` can activate. No other persona, regardless of how the application is built, can trigger this transition.
- Activation requires both seat compliance and payment verification. Both facts come from named external systems. Both rules are at stratum 0 (they read facts directly). The activation rule is at stratum 1 (it reads verdicts from stratum 0). There are no circular dependencies. Evaluation always terminates.
- The effect is a single transition: trial to active. If the subscription is in any other state, the operation cannot execute. This is structural — not a runtime check that might be skipped.

The elaborator compiles this to a canonical JSON interchange format. That JSON is the single source of truth for every downstream tool: the evaluator, the static analyzer, the executor, the LSP, the WASM module. Six validated domain contracts ship with Tenor — SaaS, healthcare, supply chain, energy procurement, trade finance, and a multi-contract system composition — each proven end-to-end.

**What makes this different from a state machine library?** The contract includes the authorization model, the decision logic, the workflow orchestration, and the provenance semantics — all in one formally verified artifact. A state machine library gives you transitions. Tenor gives you the complete behavioral specification.

**What makes this different from a policy DSL?** Tenor contracts are closed-world. If it's not declared, it doesn't exist. There are no implicit behaviors, no ambient authorities, no escape hatches. The static analyzer can enumerate every possible outcome because the contract is the complete description of the system.

### Step 3: Simulate

Before deploying anything, you can prove your contract does what you intend.

**Static analysis** runs eight checks against any contract:

| Check | What it proves |
|-------|---------------|
| State space enumeration | Every entity state, total state count |
| Reachability | Every declared state is reachable from the initial state |
| Admissibility | Which operations are available in which state combinations |
| Authority mapping | What each persona can do in every reachable state |
| Verdict enumeration | Every verdict the rules can produce |
| Flow path enumeration | Every execution path through every workflow |
| Complexity bounds | Predicate depth, branching factors, evaluation bounds |
| Verdict uniqueness | No two rules produce the same verdict type |

If your compliance officer asks "can a suspended account ever be activated without payment verification?", you don't grep through code. You run `tenor check` and read the authority mapping. The answer is structural — it comes from the contract, not from testing.

**Evaluation** takes a contract, a set of facts, and current entity states, then produces verdicts and an action space. The action space answers three questions simultaneously:

1. **What can I do?** Every operation executable right now by this persona, with the verdicts that enable it.
2. **What's blocked and why?** Every operation that exists but can't execute, with the specific reason — missing verdict, wrong entity state, unauthorized persona.
3. **What's true?** The full verdict set from evaluation.

This is pure computation. No database, no side effects, no network. You can run it in a Rust process, in a WASM module in the browser, or via the HTTP API. The same contract with the same inputs produces the same output on any platform — determinism is a structural guarantee of the language.

**Simulation** executes a flow end-to-end against test data and returns the full outcome with provenance. You see exactly which facts led to which verdicts, which verdicts satisfied which preconditions, which operations executed, and which entity states changed. The chain is unbroken from the terminal outcome back to the raw fact values that caused it.

You can simulate before you have a database. Before you have an application. Before you have users. The contract is testable the moment it compiles.

### Step 4: Deploy

Deployment means connecting the contract to real state and real data.

The **evaluator** (open source, Apache 2.0) is the read path. It takes a contract, facts, and entity states and computes verdicts and action spaces. It is pure — no database, no side effects. It runs as a Rust library, a WASM module, or an HTTP server. The evaluator answers questions. It never changes anything.

The **executor** (commercial) is the write path. It takes the evaluator's output and makes it real. When a persona invokes an operation, the executor:

1. Loads current entity states from storage
2. Calls the evaluator to verify preconditions still hold (snapshot isolation — no TOCTOU races)
3. Executes the operation atomically — all transitions succeed or none commit
4. Records complete provenance: which operation, which persona, which verdicts, which facts, when

The executor enforces what the evaluator advises. The separation is the most important architectural boundary in Tenor. The evaluator is open so anyone can verify contract behavior independently. The executor is commercial because it requires durable state and atomic execution.

**What does "deploy" look like concretely?**

You load a contract into the platform. The platform exposes HTTP endpoints: evaluate, inspect, simulate, execute, and actions. Your application calls these endpoints. It never implements business logic — it renders state (showing the user what's true and what's possible) and relays decisions (forwarding the user's chosen action to the executor). The contract is the authority. Your application is the interface.

Entity states live in Postgres with optimistic concurrency control. Every transition is atomic. Every execution is recorded with its full provenance chain. The storage interface is defined as a trait with a conformance suite — any conforming storage backend is interchangeable.

### Step 5: Attach Decision-Makers

Once the runtime is deployed, anything that can read an action space and choose an action can operate the system. The decision-maker is a persona. It could be a human using a UI. It could be an AI agent. It could be both, working together.

The agent loop is four steps: **observe** current state, **evaluate** the contract to get the action space, **choose** an action, **execute** it. The loop is identical regardless of who's choosing. A random agent and a strategic agent go through the same enforcement path. Safety doesn't vary with intelligence because safety is in the contract, not in the agent.

The `AgentPolicy` trait is the only part that changes:

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

The policy sees the full action space — available actions, blocked actions with reasons, and current verdicts. A policy that only sees available actions is reactive. A policy that also sees what's blocked and why can plan toward unblocking those actions. The contract provides both views.

Reference policies ship with Tenor: random selection, first-available, priority-ordered. Production policies can be as sophisticated as needed — including LLM-backed policies that serialize the action space into a prompt and return a reasoned choice.

**The critical property:** an agent operating inside Tenor cannot take an impermissible action. Not because it's well-behaved, but because impermissible actions don't exist in the action space. The contract constrains the environment. The agent picks from legal moves. This is the difference between trusting an AI to follow rules and building an environment where the rules are physics.

---

## Three Perspectives

### For the Architect

You write the contract. Tenor gives you a language where authorization, state management, decision logic, and workflow orchestration are one artifact — not scattered across config files, code, and documentation that fall out of sync.

The language is non-Turing complete by design. That's not a limitation — it's how you get static analysis. Every claim your contract makes is checkable before deployment. Every state is enumerable. Every authority boundary is derivable. The elaborator rejects contracts that violate these properties. You can't accidentally write an unanalyzable contract because the language won't let you.

The six validated domain contracts demonstrate this across real industries: SaaS subscription lifecycle, healthcare prior authorization, supply chain inspection, energy procurement RFP, trade finance letter of credit, and a multi-contract system composition that coordinates across contract boundaries.

### For the Engineering Team

Your application doesn't implement business logic. It calls the evaluator to learn what's true and what's possible, renders that information for users, and forwards their decisions to the executor. The contract is the single source of truth. Your code is the delivery mechanism.

The evaluator is a Rust library you can embed directly, a WASM module you can run in the browser, or an HTTP server you can call from any language. The executor provides atomic execution with Postgres-backed storage and complete provenance. The storage trait has a conformance suite — you can verify any backend implementation against the spec.

When the contract changes, your application doesn't change. The new contract produces new verdicts, new action spaces, new authority boundaries. Your UI adapts because it renders contract output, not hardcoded logic. The contract is the API.

### For the Organization

Every decision made through Tenor carries its complete derivation. Not a log entry appended after the fact — a structured proof generated as part of evaluation. When a regulator asks "who authorized this and why?", the answer is a provenance chain: this persona invoked this operation, enabled by these verdicts, produced by these rules, evaluated against these facts with these values at this time.

The chain is deterministic. Given the same facts, the same contract always produces the same verdicts with the same proof structure. A regulator doesn't need access to your production system. They can take the contract, run it through their own conforming evaluator, and verify that the properties you claim actually hold.

Static analysis means you can answer "could this ever happen?" with mathematical certainty. Not "we tested for it." Not "we haven't seen it." The contract's state space is finite and fully enumerable. If an outcome is possible, the analyzer finds it. If the analyzer doesn't find it, it's impossible — by construction, not by testing.

---

## What Tenor Is Not

**Tenor is not a programming language.** You cannot write loops, allocate memory, or call functions. The language is non-Turing complete. That is the mechanism that enables formal guarantees.

**Tenor is not a workflow engine.** Workflow engines orchestrate steps but cannot verify that the orchestration is complete or correct. Tenor flows are in the contract — every path is statically enumerable, every outcome is provenance-complete.

**Tenor is not a smart contract platform.** Smart contracts execute on a blockchain with consensus mechanisms. Tenor contracts execute anywhere a conforming evaluator runs — your laptop, a server, a browser, an edge node. No blockchain required. No gas fees. No consensus delays.

**Tenor does not replace your application.** The user interface, the database schema, the network layer, the deployment infrastructure — that remains engineering work. Tenor replaces the behavioral layer: the scattered logic that determines whether a transaction is authorized, whether a claim is approved, whether a workflow can proceed. That layer becomes formally specified, statically verifiable, and independently auditable. Everything else is built on a foundation whose correctness is proven, not assumed.

---

## The End State

When everything works:

1. A domain expert describes a process. An LLM, guided by the closed-world spec, writes the contract. The spec's constraints make LLM-authored contracts more reliable than LLM-authored general-purpose code — the language rejects anything that violates formal properties.
2. The team simulates until the contract matches intent. Static analysis proves the properties they care about. No deployment required.
3. They deploy the runtime. The contract connects to real data sources and real storage. The application renders state and relays decisions.
4. They attach decision-makers. Humans see what's possible and what's blocked. AI agents operate within the same boundaries. Both go through the same enforcement path.
5. Every action is pre-authorized by the contract. Every transition is atomic. Every outcome carries its complete provenance chain. A random agent and a strategic agent have identical safety guarantees — because safety is structural, not behavioral.

The contract is the boundary between intent and execution. The spec guarantees that boundary is formally enforced.
