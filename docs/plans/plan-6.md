# Phase 6: Advanced Policies — Complete Implementation

The roadmap specifies three production-ready agent policies beyond the reference implementations (RandomPolicy, FirstAvailablePolicy, PriorityPolicy) that already exist.

**Repo:** Public only (`~/src/riverline/tenor`). All policies implement the `AgentPolicy` trait which lives in the public repo. The private repo's agent runtime already consumes policies via the trait — no private changes needed.

**Source of truth:** TENOR.md §15.6 (action space), AGENT_ORIENTATION.md (agent runtime section, AgentPolicy trait), and the roadmap Phase 9 items.

---

## What "done" means

1. `HumanInTheLoopPolicy` — pauses execution, presents the proposed action to a human, waits for confirmation, proceeds or aborts
2. `LlmPolicy` — serializes the action space and snapshot into a prompt, calls an LLM, parses the chosen action
3. `CompositePolicy` — chains policies with configurable thresholds (e.g., LLM proposes, human approves for high-value actions)
4. All three implement `AgentPolicy` and work with the existing agent runtime
5. LLM policy has a reference implementation against the Anthropic API
6. All policies handle edge cases: empty action space, policy timeout, invalid LLM response

---

## Step 1: Review existing policy infrastructure

Before writing anything, read:

- The `AgentPolicy` trait definition (should be in `crates/eval/src/policy.rs` or similar)
- The existing reference implementations (RandomPolicy, FirstAvailablePolicy, PriorityPolicy)
- The `ActionSpace`, `Action`, `BlockedAction`, `AgentSnapshot` types
- How the agent runtime calls `policy.choose(action_space, snapshot)`

Report what you find. The new policies must follow the exact same patterns.

---

## Step 2: HumanInTheLoopPolicy

### 2A: Design

The HumanInTheLoopPolicy pauses the agent loop and presents the proposed action to a human for review. The human confirms, rejects, or selects a different action.

```rust
pub struct HumanInTheLoopPolicy {
    delegate: Box<dyn AgentPolicy>,
    approval_channel: Box<dyn ApprovalChannel>,
    timeout: Duration,
}

#[async_trait]
pub trait ApprovalChannel: Send + Sync {
    /// Present an action to the human and wait for approval
    async fn request_approval(&self, proposed: &Action, action_space: &ActionSpace, snapshot: &AgentSnapshot) -> ApprovalResult;
}

pub enum ApprovalResult {
    Approved,                    // proceed with proposed action
    Rejected,                    // abort, return None from choose()
    Substitute(Action),          // human chose a different action
    Timeout,                     // human didn't respond in time
}
```

The delegate policy (e.g., LlmPolicy) proposes an action. The approval channel presents it to the human. The human decides.

### 2B: Reference approval channels

**StdinApprovalChannel** — interactive terminal:

```
Proposed action: execute flow "release_escrow" for Order/ord-001
  Instance bindings: { Order: "ord-001", DeliveryRecord: "del-001" }
  Personas: [escrow_agent]
  Enabling verdicts: [all_line_items_valid, delivery_confirmed, compliance_approved]

[a]pprove / [r]eject / [l]ist alternatives? _
```

**CallbackApprovalChannel** — for programmatic integration:

```rust
pub struct CallbackApprovalChannel {
    callback: Box<dyn Fn(&Action, &ActionSpace) -> ApprovalResult + Send + Sync>,
}
```

### 2C: Policy behavior

- If delegate returns `None` (no action proposed), HumanInTheLoopPolicy returns `None` without consulting the human
- If approval times out, behavior is configurable: `TimeoutBehavior::Reject` (default) or `TimeoutBehavior::Approve`
- If human selects a Substitute, validate it's in the action space before returning it

### 2D: Tests

- Test: delegate proposes action, human approves → action returned
- Test: delegate proposes action, human rejects → None returned
- Test: delegate proposes action, human substitutes → substitute returned (if valid)
- Test: delegate proposes action, human substitutes invalid action → error
- Test: empty action space → None returned without consulting human
- Test: timeout → configured behavior (reject by default)

### Acceptance criteria — Step 2

- [ ] HumanInTheLoopPolicy implements AgentPolicy
- [ ] ApprovalChannel trait defined
- [ ] StdinApprovalChannel for terminal use
- [ ] CallbackApprovalChannel for programmatic use
- [ ] Timeout handling
- [ ] Tests pass

---

## Step 3: LlmPolicy

### 3A: Design

The LlmPolicy serializes the action space and snapshot into a prompt, calls an LLM, and parses the response into an Action.

```rust
pub struct LlmPolicy {
    client: Box<dyn LlmClient>,
    system_prompt: String,
    model: String,
    max_retries: usize,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, messages: Vec<Message>, model: &str) -> Result<String, LlmError>;
}

pub struct Message {
    pub role: String,  // "system", "user", "assistant"
    pub content: String,
}
```

### 3B: Prompt construction

The prompt must include:

- The contract's domain context (entity types, current states, fact values)
- The available actions with their instance bindings, enabling verdicts, and personas
- The blocked actions with their reasons (so the LLM can reason about unblocking)
- Instructions to return a JSON response selecting one action by flow_id and instance_bindings

System prompt template:

```
You are an agent operating within a Tenor contract. You must select the best action
from the available action space, or return null to take no action.

The action space shows:
- Available actions: flows you can execute right now, with which entity instances
- Blocked actions: flows that exist but can't execute, with the specific reason

You MUST return valid JSON in this format:
{
  "action": {
    "flow_id": "<flow_id>",
    "instance_bindings": { "<entity_id>": "<instance_id>", ... },
    "persona": "<persona_id>"
  },
  "reasoning": "<brief explanation>"
}

Or if no action should be taken:
{ "action": null, "reasoning": "<why>" }
```

The user message contains the serialized action space and snapshot.

### 3C: Anthropic API reference implementation

Create `AnthropicClient` implementing `LlmClient`:

```rust
pub struct AnthropicClient {
    api_key: String,
    base_url: String,  // default: https://api.anthropic.com
}
```

- Uses the Messages API (`/v1/messages`)
- Model default: `claude-sonnet-4-20250514`
- API key from `ANTHROPIC_API_KEY` environment variable
- Handles rate limits with exponential backoff
- Handles malformed responses by retrying up to `max_retries` times

Use `ureq` (already in the dependency tree from tenor connect) for HTTP. Feature-gate behind `#[cfg(feature = "anthropic")]` so the dependency is optional.

### 3D: Response parsing

Parse the LLM's JSON response:

1. Extract `action` field
2. If null, return `None`
3. If present, validate the selected action is in the action space (flow_id exists, instance_bindings are valid, persona is authorized)
4. If validation fails, retry with an error message appended to the conversation
5. After max_retries, return `None` with a warning log

### 3E: Tests

- Test: LLM selects valid action → action returned (mock LlmClient)
- Test: LLM selects null → None returned
- Test: LLM returns invalid action → retry → eventually valid → action returned
- Test: LLM returns garbage → retry → max retries → None returned
- Test: LLM client error → None returned with error log
- Test: prompt construction includes all action space fields
- Test: prompt construction includes blocked actions with reasons
- Integration test (requires ANTHROPIC_API_KEY, mark as ignored): LLM policy against escrow contract

### Acceptance criteria — Step 3

- [ ] LlmPolicy implements AgentPolicy
- [ ] LlmClient trait defined
- [ ] AnthropicClient implements LlmClient (feature-gated)
- [ ] Prompt includes action space, blocked actions, snapshot context
- [ ] Response parsing with validation and retry
- [ ] Graceful degradation (returns None on persistent failure)
- [ ] Tests pass (mock and integration)

---

## Step 4: CompositePolicy

### 4A: Design

The CompositePolicy chains policies with configurable logic. The primary use case: LLM proposes, human approves for high-value actions, auto-approves for low-value.

```rust
pub struct CompositePolicy {
    proposer: Box<dyn AgentPolicy>,
    approver: Box<dyn AgentPolicy>,
    requires_approval: Box<dyn ApprovalPredicate>,
}

pub trait ApprovalPredicate: Send + Sync {
    /// Returns true if the proposed action requires approval from the approver
    fn requires_approval(&self, action: &Action, snapshot: &AgentSnapshot) -> bool;
}
```

Execution:

1. `proposer.choose(action_space, snapshot)` → proposed action (or None)
2. If None, return None
3. If `requires_approval(proposed, snapshot)` is true, call `approver.choose(filtered_action_space, snapshot)` where the filtered action space contains only the proposed action
4. If approver returns the action, proceed. If approver returns None, reject.
5. If `requires_approval` is false, auto-approve.

### 4B: Reference predicates

**EntityStatePredicate** — require approval when specific entities are in specific states:

```rust
pub struct EntityStatePredicate {
    rules: Vec<(String, String)>,  // (entity_id, state) pairs that trigger approval
}
```

**FlowIdPredicate** — require approval for specific flows:

```rust
pub struct FlowIdPredicate {
    flows: HashSet<String>,
}
```

**AlwaysApprove / NeverApprove** — for testing and simple configurations.

### 4C: Tests

- Test: proposer proposes, predicate says no approval needed → auto-approved
- Test: proposer proposes, predicate says approval needed, approver approves → action returned
- Test: proposer proposes, predicate says approval needed, approver rejects → None returned
- Test: proposer returns None → None returned without consulting approver
- Test: entity state predicate triggers on correct state
- Test: flow id predicate triggers on correct flow

### Acceptance criteria — Step 4

- [ ] CompositePolicy implements AgentPolicy
- [ ] ApprovalPredicate trait defined
- [ ] EntityStatePredicate and FlowIdPredicate implemented
- [ ] Chain: proposer → predicate → approver works correctly
- [ ] Tests pass

---

## Step 5: Documentation and examples

Add a `docs/policies.md` or update existing documentation:

- How to use each policy
- How to configure the LLM policy (API key, model)
- How to compose policies (LLM + human-in-the-loop)
- Example: fully autonomous agent (LlmPolicy alone)
- Example: human-supervised agent (LlmPolicy + HumanInTheLoopPolicy via CompositePolicy)
- Example: approval threshold (auto-approve low-value, human-approve high-value)

### Acceptance criteria — Step 5

- [ ] Documentation exists
- [ ] Examples cover all three policies
- [ ] Examples show composition

---

## Final Report

```
## Phase 6: Advanced Policies — COMPLETE

### Policies implemented
- HumanInTheLoopPolicy: delegate + approval channel, timeout handling
- LlmPolicy: Anthropic API reference impl, structured prompt, retry with validation
- CompositePolicy: proposer + predicate + approver chain

### Infrastructure
- ApprovalChannel trait: StdinApprovalChannel, CallbackApprovalChannel
- LlmClient trait: AnthropicClient (feature-gated)
- ApprovalPredicate trait: EntityStatePredicate, FlowIdPredicate

### Tests
- Unit tests: [N] passing
- Integration (LLM, requires API key): [N] passing
- All existing tests: PASS

### Commits
- [hash] [message]
- ...
```

Phase 6 is done when all three policies work, the LLM integration calls the Anthropic API, the composite policy chains correctly, and every checkbox above is checked. Not before.
