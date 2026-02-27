# Agent Policies

The `AgentPolicy` trait is the single decision point in the agent loop. Everything else
— action space computation, contract enforcement, execution, provenance — is mechanics.
The policy just picks a legal move.

Tenor ships with three reference policies for simple use cases (`RandomPolicy`,
`FirstAvailablePolicy`, `PriorityPolicy`) and three advanced policies for production
deployments:

- **HumanInTheLoopPolicy** — pauses the loop and waits for a human to approve, reject,
  or substitute each proposed action
- **LlmPolicy** — asks a large language model to reason about the action space and pick
  the best move
- **CompositePolicy** — chains a proposer and an approver with a conditional predicate,
  enabling risk-based routing

All three implement the same trait:

```rust
#[async_trait]
pub trait AgentPolicy: Send + Sync {
    /// Choose an action from the available action space.
    ///
    /// Every action in `action_space.actions` is guaranteed executable.
    /// The policy's job is only to select which one (or none).
    async fn choose(&self, action_space: &ActionSpace, snapshot: &AgentSnapshot) -> Option<Action>;
}
```

Returning `None` means "do nothing this cycle" — the agent idles and will call `choose`
again on the next tick. Returning `Some(action)` causes the evaluator to execute that
action against the contract.

`AgentSnapshot` carries the raw context that sophisticated policies use to reason:

```rust
pub struct AgentSnapshot {
    /// Current facts assembled from external sources.
    pub facts: HashMap<String, serde_json::Value>,

    /// Current entity states: entity_id -> current state name.
    pub entity_states: HashMap<String, String>,

    /// When this snapshot was taken (ISO 8601 UTC).
    pub observed_at: String,
}
```

Simple policies (random, first-available) can ignore `snapshot`. Sophisticated policies
(LLM, planner) use both arguments.

---

## HumanInTheLoopPolicy

### Purpose

`HumanInTheLoopPolicy` pauses the agent loop before executing an action. A delegate
policy proposes an action; the policy presents it to a human via an approval channel;
the human decides whether to proceed.

This is the right choice when you need a human in the critical path — for compliance,
audit requirements, or simply because the stakes are high enough to warrant review.

### How it works

1. If the action space is empty, returns `None` immediately (no proposal, no prompt).
2. Delegates to the inner policy to propose an action. If the delegate returns `None`,
   returns `None` without consulting the human.
3. Passes the proposed action to the `ApprovalChannel`, which presents it to a human and
   waits for a decision.
4. Acts on the `ApprovalResult`:
   - `Approved` — returns the proposed action
   - `Rejected` — returns `None`
   - `Substitute(action)` — validates the substitute is in the action space, returns it
     if valid, `None` otherwise
   - `Timeout` — applies `TimeoutBehavior` (see below)

### ApprovalChannel trait

```rust
#[async_trait]
pub trait ApprovalChannel: Send + Sync {
    async fn request_approval(
        &self,
        proposed: &Action,
        action_space: &ActionSpace,
        snapshot: &AgentSnapshot,
    ) -> ApprovalResult;
}
```

The channel receives the full action space so it can present alternatives when the human
wants to substitute.

### Timeout behavior

```rust
pub enum TimeoutBehavior {
    /// Reject the proposed action (default).
    Reject,
    /// Auto-approve the proposed action.
    Approve,
}
```

`TimeoutBehavior::Reject` is the safe default. Use `Approve` only when an SLA requires
the agent to keep moving and the risk of proceeding is acceptable.

### Configuration

```rust
pub struct HumanInTheLoopPolicy {
    pub delegate: Box<dyn AgentPolicy>,
    pub approval_channel: Box<dyn ApprovalChannel>,
    pub timeout: Duration,
    pub timeout_behavior: TimeoutBehavior,
}
```

The `timeout` field records the intended maximum wait time and is available to approval
channel implementations that want to enforce it (e.g., via `tokio::time::timeout`). When
the channel returns `ApprovalResult::Timeout`, `HumanInTheLoopPolicy` applies the
`timeout_behavior` rule. The `StdinApprovalChannel` does not enforce the timeout itself;
for production use, wrap the channel or use `CallbackApprovalChannel` with a custom
implementation that races the approval against a deadline.

### Example: Interactive terminal approval

```rust
use std::time::Duration;
use tenor_eval::{
    HumanInTheLoopPolicy, FirstAvailablePolicy, StdinApprovalChannel, TimeoutBehavior,
};

let policy = HumanInTheLoopPolicy::new(
    Box::new(FirstAvailablePolicy),
    Box::new(StdinApprovalChannel),
    Duration::from_secs(60),   // one-minute timeout
    TimeoutBehavior::Reject,   // reject if no response
);
```

`StdinApprovalChannel` prints the proposed action and alternatives to stdout, reads a
line from stdin, and returns:
- `"a"` or `"approve"` → `ApprovalResult::Approved`
- `"r"` or `"reject"` → `ApprovalResult::Rejected`
- A numeric index → `ApprovalResult::Substitute(action_space.actions[idx])`

### Example: Programmatic callback (for testing or webhooks)

`CallbackApprovalChannel` accepts a synchronous closure, making it easy to wire up
automated testing or external approval systems:

```rust
use tenor_eval::{
    HumanInTheLoopPolicy, FirstAvailablePolicy, CallbackApprovalChannel,
    ApprovalResult, TimeoutBehavior,
};
use std::time::Duration;

// Automated test: always approve
let policy = HumanInTheLoopPolicy::new(
    Box::new(FirstAvailablePolicy),
    Box::new(CallbackApprovalChannel::new(|proposed, _space, _snapshot| {
        // inspect proposed.flow_id and decide
        ApprovalResult::Approved
    })),
    Duration::from_secs(30),
    TimeoutBehavior::Reject,
);
```

For webhook integration, the callback can send a request to an external service and
block until a response is received (or use an async channel with `tokio::task::block_in_place`
if the approval process is itself async).

### Edge cases

- **Empty action space**: Short-circuits immediately — delegate and channel are never
  called.
- **Delegate returns None**: Short-circuits — channel is never called.
- **Invalid substitute**: If the human names a `flow_id` not in the action space,
  `HumanInTheLoopPolicy` returns `None` (safe rejection).

---

## LlmPolicy

### Purpose

`LlmPolicy` uses a large language model to reason about the available action space and
select the best action. It serializes the action space and snapshot into a structured
JSON prompt, calls the LLM, parses the response, and validates the result against the
action space.

This is the right choice when you want autonomous operation but need something smarter
than priority lists — when context matters and a fixed ordering can't capture the right
decision.

### How it works

1. If the action space is empty, returns `None` immediately — the LLM is never called.
2. Builds a system prompt (default or custom) explaining the task and response format.
3. Builds a user message containing the available actions, blocked actions (for context),
   entity states, facts, and observation timestamp as a JSON object.
4. Calls the LLM client.
5. Parses the JSON response — extracts `action.flow_id` and validates it against the
   action space.
6. Returns the canonical `Action` from the action space (not the LLM's version).
7. On parse failure, appends the error to the conversation and retries (up to
   `max_retries`).

### Configuration

```rust
pub struct LlmPolicy {
    pub client: Box<dyn LlmClient>,
    pub system_prompt: String,  // empty = use default
    pub model: String,          // e.g., "claude-sonnet-4-20250514"
    pub max_retries: usize,     // default: 2
}
```

The `LlmPolicy::new(client, model)` constructor sets `system_prompt = ""` (uses default)
and `max_retries = 2`.

### LlmClient trait

```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, messages: Vec<Message>, model: &str) -> Result<String, LlmError>;
}
```

Implement this trait to connect any LLM backend. `Message` is a simple struct:

```rust
pub struct Message {
    pub role: String,    // "system", "user", or "assistant"
    pub content: String,
}
```

### AnthropicClient setup

The `AnthropicClient` is provided as a reference implementation and is feature-gated:

```toml
# Cargo.toml
tenor-eval = { version = "*", features = ["anthropic"] }
```

It requires the `ANTHROPIC_API_KEY` environment variable:

```rust
#[cfg(feature = "anthropic")]
use tenor_eval::AnthropicClient;

// Read from ANTHROPIC_API_KEY environment variable
let client = AnthropicClient::from_env()
    .expect("ANTHROPIC_API_KEY must be set");

// Or provide the key explicitly
let client = AnthropicClient::new("sk-ant-...".to_string());
```

### Example: Creating an LlmPolicy with AnthropicClient

```rust
#[cfg(feature = "anthropic")]
use tenor_eval::{LlmPolicy, AnthropicClient};

let client = AnthropicClient::from_env().expect("ANTHROPIC_API_KEY required");

let policy = LlmPolicy::new(
    Box::new(client),
    "claude-sonnet-4-20250514".to_string(),
);
```

### Example: Custom LlmClient (e.g., OpenAI)

```rust
use async_trait::async_trait;
use tenor_eval::{LlmClient, LlmError, Message, LlmPolicy};

struct OpenAiClient {
    api_key: String,
}

#[async_trait]
impl LlmClient for OpenAiClient {
    async fn complete(&self, messages: Vec<Message>, model: &str) -> Result<String, LlmError> {
        // Map messages to OpenAI format, call API, extract content
        // ...
        todo!("implement OpenAI API call")
    }
}

let policy = LlmPolicy::new(
    Box::new(OpenAiClient { api_key: "sk-...".to_string() }),
    "gpt-4o".to_string(),
);
```

### Retry behavior

When the LLM returns a response that cannot be parsed or references a `flow_id` not in
the action space, `LlmPolicy` appends the error to the conversation and asks the LLM to
try again:

```
attempt 0: bad JSON -> append (assistant: bad_response) + (user: "Your response was invalid: ... Please try again.")
attempt 1: wrong flow_id -> append correction pair again
attempt 2 (max_retries): exhausted -> return None
```

Network or API errors (`LlmError::NetworkError`, `LlmError::ApiError`) cause an
immediate `None` return without retrying — there is no point retrying a server-side
failure.

### Prompt format

The **system prompt** instructs the LLM to:
- Select a `flow_id` from `available_actions`
- Respond with a specific JSON schema containing `action.flow_id`, `action.persona_id`,
  and `reasoning`
- Respond with `{"action": null, ...}` if no action is appropriate
- Never invent flow IDs or personas not listed

The **user message** is a JSON object:

```json
{
  "available_actions": [ ... ],
  "blocked_actions": [ ... ],
  "snapshot": {
    "facts": { ... },
    "entity_states": { ... },
    "observed_at": "2026-02-27T10:00:00Z"
  }
}
```

`blocked_actions` is included so the LLM understands the full contract context — it can
reason about why certain actions are unavailable.

---

## CompositePolicy

### Purpose

`CompositePolicy` chains a proposer policy with a conditional approval gate. The proposer
suggests an action; a predicate decides whether that action needs secondary review; if so,
an approver policy makes the final call.

This enables risk-based routing: low-risk actions are auto-approved, high-risk actions
are routed to a human or secondary LLM for review.

### How it works

1. `proposer.choose(action_space, snapshot)` → proposed action. If `None`, return `None`
   immediately.
2. Call `requires_approval.requires_approval(proposed, snapshot)`.
3. If `false`: auto-approve and return the proposed action.
4. If `true`: build a filtered `ActionSpace` containing only the proposed action, then
   call `approver.choose(filtered_space, snapshot)`. If the approver returns `None`, the
   action is rejected.

### Configuration

```rust
pub struct CompositePolicy {
    pub proposer: Box<dyn AgentPolicy>,
    pub approver: Box<dyn AgentPolicy>,
    pub requires_approval: Box<dyn ApprovalPredicate>,
}
```

### ApprovalPredicate trait

```rust
pub trait ApprovalPredicate: Send + Sync {
    fn requires_approval(&self, action: &Action, snapshot: &AgentSnapshot) -> bool;
}
```

### Reference predicates

| Predicate | Triggers when |
|-----------|---------------|
| `FlowIdPredicate { flows }` | `action.flow_id` is in the set |
| `EntityStatePredicate { rules }` | Any `(entity_id, state)` rule matches snapshot |
| `AlwaysApprove` | Always (useful for fully-supervised configurations) |
| `NeverApprove` | Never (useful for fully-autonomous testing) |

### Example: LlmPolicy proposes, FlowIdPredicate gates, FirstAvailablePolicy approves

```rust
use std::collections::HashSet;
use tenor_eval::{
    CompositePolicy, LlmPolicy, FirstAvailablePolicy, FlowIdPredicate,
};
#[cfg(feature = "anthropic")]
use tenor_eval::AnthropicClient;

#[cfg(feature = "anthropic")]
let policy = CompositePolicy::new(
    Box::new(LlmPolicy::new(
        Box::new(AnthropicClient::from_env().unwrap()),
        "claude-sonnet-4-20250514".to_string(),
    )),
    Box::new(FirstAvailablePolicy),  // approver: auto-approve by selecting first
    Box::new(FlowIdPredicate {
        flows: ["release_escrow".to_string(), "cancel_order".to_string()]
            .into_iter()
            .collect::<HashSet<_>>(),
    }),
);
```

The LLM proposes; if it selects `"release_escrow"` or `"cancel_order"`, those are
routed to the approver. All other flows are auto-approved.

### Example: EntityStatePredicate for state-dependent approval

```rust
use tenor_eval::{CompositePolicy, FirstAvailablePolicy, HumanInTheLoopPolicy,
    StdinApprovalChannel, EntityStatePredicate, TimeoutBehavior};
use std::time::Duration;

let policy = CompositePolicy::new(
    Box::new(FirstAvailablePolicy),
    Box::new(HumanInTheLoopPolicy::new(
        Box::new(FirstAvailablePolicy),
        Box::new(StdinApprovalChannel),
        Duration::from_secs(120),
        TimeoutBehavior::Reject,
    )),
    Box::new(EntityStatePredicate {
        // Require human approval whenever the Order is in "pending_large" state
        rules: vec![("Order".to_string(), "pending_large".to_string())],
    }),
);
```

When `Order` is in `"pending_large"`, all proposed actions are routed to the
`HumanInTheLoopPolicy` approver. When `Order` is in any other state (e.g.,
`"pending_standard"`), proposed actions are auto-approved.

---

## Composition Patterns

### Example 1: Fully Autonomous Agent

Use an `LlmPolicy` directly with no approval gate. Appropriate for low-risk contract
processing, development, and testing.

```rust
#[cfg(feature = "anthropic")]
use tenor_eval::{LlmPolicy, AnthropicClient};

#[cfg(feature = "anthropic")]
let policy = LlmPolicy::new(
    Box::new(AnthropicClient::from_env().expect("ANTHROPIC_API_KEY required")),
    "claude-sonnet-4-20250514".to_string(),
);

// Use as the policy in your agent loop:
// let action = policy.choose(&action_space, &snapshot).await;
```

**Use cases**: Development and testing environments, low-risk batch processing, contracts
where all flows have equivalent risk profiles.

---

### Example 2: Human-Supervised Agent

Every action proposed by the LLM is reviewed by a human before execution. No action
is ever taken without explicit human sign-off.

```rust
use std::collections::HashSet;
use std::time::Duration;
use tenor_eval::{
    CompositePolicy, LlmPolicy, HumanInTheLoopPolicy, StdinApprovalChannel,
    AlwaysApprove, TimeoutBehavior,
};
#[cfg(feature = "anthropic")]
use tenor_eval::AnthropicClient;

#[cfg(feature = "anthropic")]
let policy = CompositePolicy::new(
    // Proposer: LLM picks the best action
    Box::new(LlmPolicy::new(
        Box::new(AnthropicClient::from_env().expect("ANTHROPIC_API_KEY required")),
        "claude-sonnet-4-20250514".to_string(),
    )),
    // Approver: human reviews and approves via stdin
    Box::new(HumanInTheLoopPolicy::new(
        Box::new(LlmPolicy::new(
            Box::new(AnthropicClient::from_env().expect("ANTHROPIC_API_KEY required")),
            "claude-sonnet-4-20250514".to_string(),
        )),
        Box::new(StdinApprovalChannel),
        Duration::from_secs(300),  // 5-minute window
        TimeoutBehavior::Reject,   // reject if operator walks away
    )),
    // Predicate: always require approval
    Box::new(AlwaysApprove),
);
```

**Use cases**: Production deployments with compliance requirements, onboarding new
contract types, high-value transactions where auditability is critical.

---

### Example 3: Risk-Based Approval Threshold

The LLM operates autonomously for routine flows. High-value flows (e.g., `release_escrow`,
`cancel_order`) are automatically routed to a human for review. Everything else is
auto-approved.

```rust
use std::collections::HashSet;
use std::time::Duration;
use tenor_eval::{
    CompositePolicy, LlmPolicy, HumanInTheLoopPolicy, StdinApprovalChannel,
    FlowIdPredicate, TimeoutBehavior,
};
#[cfg(feature = "anthropic")]
use tenor_eval::AnthropicClient;

#[cfg(feature = "anthropic")]
let high_value_flows: HashSet<String> = [
    "release_escrow",
    "cancel_order",
    "issue_refund",
]
.iter()
.map(|s| s.to_string())
.collect();

#[cfg(feature = "anthropic")]
let policy = CompositePolicy::new(
    // Proposer: LLM evaluates the full context
    Box::new(LlmPolicy::new(
        Box::new(AnthropicClient::from_env().expect("ANTHROPIC_API_KEY required")),
        "claude-sonnet-4-20250514".to_string(),
    )),
    // Approver: human reviews only when the predicate triggers
    Box::new(HumanInTheLoopPolicy::new(
        Box::new(LlmPolicy::new(
            Box::new(AnthropicClient::from_env().expect("ANTHROPIC_API_KEY required")),
            "claude-sonnet-4-20250514".to_string(),
        )),
        Box::new(StdinApprovalChannel),
        Duration::from_secs(180),  // 3-minute review window
        TimeoutBehavior::Reject,
    )),
    // Predicate: gate only high-value flows
    Box::new(FlowIdPredicate { flows: high_value_flows }),
);
```

**Use cases**: Production deployments where most decisions are routine but a subset of
flows carry material financial or operational risk. This pattern reduces operator toil
while preserving oversight where it matters.

**How it behaves at runtime:**
- LLM proposes `"update_status"` → not in `FlowIdPredicate` → auto-approved, executed immediately
- LLM proposes `"release_escrow"` → in `FlowIdPredicate` → routed to `HumanInTheLoopPolicy`
  → operator sees the proposed action on their terminal and approves or rejects
- LLM proposes `"cancel_order"` but operator is away → timeout → `TimeoutBehavior::Reject`
  → action rejected, agent idles until next cycle

---

## Reference policy quick-reference

| Policy | When to use |
|--------|-------------|
| `RandomPolicy` | Testing safety invariants — if the contract is safe, it's safe with a random policy |
| `FirstAvailablePolicy` | Deterministic testing of specific flows |
| `PriorityPolicy` | Simple rule-based ordering with a fallback |
| `HumanInTheLoopPolicy` | Human review required for every proposed action |
| `LlmPolicy` | Context-sensitive autonomous decision-making |
| `CompositePolicy` | Risk-based routing — different approval paths for different actions |
