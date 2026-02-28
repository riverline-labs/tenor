//! Agent policy trait and reference implementations.
//!
//! The `AgentPolicy` trait is the single point where "intelligence" lives.
//! Everything else — action space computation, contract enforcement,
//! execution, provenance — is mechanics. The policy just picks a legal move.
//!
//! Tenor does not care how the decision is made. It only cares that the
//! decision is an element of the action space.

use async_trait::async_trait;
#[cfg(feature = "interactive")]
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
#[cfg(feature = "interactive")]
use std::io::{self, BufRead, Write};
#[cfg(feature = "interactive")]
use std::time::Duration;

use crate::action_space::{Action, ActionSpace};

/// Result of presenting a proposed action to a human for approval.
#[derive(Debug, Clone)]
pub enum ApprovalResult {
    /// Proceed with the proposed action.
    Approved,
    /// Abort — return None from choose().
    Rejected,
    /// Human chose a different action from the action space.
    Substitute(Action),
    /// Human did not respond within the configured timeout.
    Timeout,
}

/// What to do when the approval channel times out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeoutBehavior {
    /// Reject the proposed action (default).
    #[default]
    Reject,
    /// Auto-approve the proposed action.
    Approve,
}

/// A channel through which proposed actions are presented to a human for approval.
///
/// Implementations can be interactive (stdin), programmatic (callback), or
/// networked (webhook, Slack bot, etc.).
#[async_trait]
pub trait ApprovalChannel: Send + Sync {
    /// Present a proposed action to the human and wait for a decision.
    ///
    /// The implementation receives the full action space so it can present
    /// alternatives if the human wants to substitute.
    async fn request_approval(
        &self,
        proposed: &Action,
        action_space: &ActionSpace,
        snapshot: &AgentSnapshot,
    ) -> ApprovalResult;
}

/// A consistent snapshot of the world: facts and entity states.
///
/// This is what the agent sees when it observes. It's the raw domain
/// context that the policy uses to reason about priorities and urgency.
/// The `ActionSpace` is computed FROM this snapshot — but the snapshot
/// itself carries information (fact values, entity instance details)
/// that the `ActionSpace` abstracts away.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSnapshot {
    /// Current facts assembled from external sources.
    pub facts: HashMap<String, serde_json::Value>,

    /// Current entity states: entity_id -> current state name.
    ///
    /// Keys are plain entity IDs (e.g. `"RFP"`, `"PurchaseOrder"`),
    /// matching `EntityStateMap` used by `compute_action_space`.
    pub entity_states: HashMap<String, String>,

    /// When this snapshot was taken (ISO 8601 UTC).
    pub observed_at: String,
}

/// A policy that chooses an action from the available action space.
///
/// This is the ONLY place where "intelligence" lives. Everything else
/// is mechanics. The policy can be:
/// - A deterministic rule set
/// - A heuristic function
/// - A planning algorithm
/// - An LLM call
/// - A human-in-the-loop prompt
/// - `random::thread_rng().choose()`
///
/// Returning `None` means "do nothing this cycle" (idle/wait).
///
/// The policy receives two arguments:
/// - `action_space`: the pre-computed, pre-authorized set of legal moves
/// - `snapshot`: the raw facts and entity states (domain context)
///
/// Simple policies can ignore the snapshot.
/// Sophisticated policies (LLM, planner) use both.
#[async_trait]
pub trait AgentPolicy: Send + Sync {
    /// Choose an action from the available action space.
    ///
    /// Every action in `action_space.actions` is guaranteed executable.
    /// The policy's job is only to select which one (or none).
    async fn choose(&self, action_space: &ActionSpace, snapshot: &AgentSnapshot) -> Option<Action>;
}

/// Picks a random action from the action space.
///
/// Useful for testing, fuzzing, and proving that safety doesn't depend
/// on the policy being smart.
#[cfg(feature = "interactive")]
pub struct RandomPolicy;

#[cfg(feature = "interactive")]
#[async_trait]
impl AgentPolicy for RandomPolicy {
    async fn choose(
        &self,
        action_space: &ActionSpace,
        _snapshot: &AgentSnapshot,
    ) -> Option<Action> {
        let mut rng = rand::thread_rng();
        action_space.actions.choose(&mut rng).cloned()
    }
}

/// Always picks the first available action.
///
/// Deterministic and predictable. Useful for testing specific flows.
pub struct FirstAvailablePolicy;

#[async_trait]
impl AgentPolicy for FirstAvailablePolicy {
    async fn choose(
        &self,
        action_space: &ActionSpace,
        _snapshot: &AgentSnapshot,
    ) -> Option<Action> {
        action_space.actions.first().cloned()
    }
}

/// Picks the highest-priority available action from a ranked list of flow IDs.
///
/// An operator says "prefer approval flows over cancellation flows" and the
/// agent follows that priority. Falls back to the first available action if
/// no priorities match.
pub struct PriorityPolicy {
    /// Flow IDs in priority order (highest priority first).
    pub priorities: Vec<String>,
}

#[async_trait]
impl AgentPolicy for PriorityPolicy {
    async fn choose(
        &self,
        action_space: &ActionSpace,
        _snapshot: &AgentSnapshot,
    ) -> Option<Action> {
        for flow_id in &self.priorities {
            if let Some(action) = action_space.actions.iter().find(|a| &a.flow_id == flow_id) {
                return Some(action.clone());
            }
        }
        // Fall back to first available if no priority matches
        action_space.actions.first().cloned()
    }
}

/// Interactive terminal approval channel that reads from stdin.
///
/// Presents the proposed action and available alternatives, then waits
/// for the user to approve, reject, or select a substitute.
#[cfg(feature = "interactive")]
pub struct StdinApprovalChannel;

#[cfg(feature = "interactive")]
#[async_trait]
impl ApprovalChannel for StdinApprovalChannel {
    async fn request_approval(
        &self,
        proposed: &Action,
        action_space: &ActionSpace,
        _snapshot: &AgentSnapshot,
    ) -> ApprovalResult {
        let stdout = io::stdout();
        let stdin = io::stdin();
        let mut stdout_lock = stdout.lock();
        let stdin_lock = stdin.lock();

        // Display proposed action
        let _ = writeln!(
            stdout_lock,
            "\nProposed action: execute flow \"{}\"",
            proposed.flow_id
        );
        let _ = writeln!(stdout_lock, "  Persona: {}", proposed.persona_id);
        if !proposed.enabling_verdicts.is_empty() {
            let verdicts: Vec<&str> = proposed
                .enabling_verdicts
                .iter()
                .map(|v| v.verdict_type.as_str())
                .collect();
            let _ = writeln!(
                stdout_lock,
                "  Enabling verdicts: [{}]",
                verdicts.join(", ")
            );
        }
        if !proposed.affected_entities.is_empty() {
            for entity in &proposed.affected_entities {
                let _ = writeln!(
                    stdout_lock,
                    "  Entity {}: {} -> [{}]",
                    entity.entity_id,
                    entity.current_state,
                    entity.possible_transitions.join(", ")
                );
            }
        }
        let _ = writeln!(stdout_lock);

        // Show alternatives
        if action_space.actions.len() > 1 {
            let _ = writeln!(stdout_lock, "Alternatives:");
            for (i, action) in action_space.actions.iter().enumerate() {
                if action.flow_id != proposed.flow_id {
                    let _ = writeln!(stdout_lock, "  [{}] {}", i, action.flow_id);
                }
            }
            let _ = writeln!(stdout_lock);
        }

        let _ = write!(
            stdout_lock,
            "[a]pprove / [r]eject / [N] select alternative? "
        );
        let _ = stdout_lock.flush();

        let mut input = String::new();
        let mut reader = io::BufReader::new(stdin_lock);
        if reader.read_line(&mut input).is_err() {
            return ApprovalResult::Rejected;
        }

        let trimmed = input.trim().to_lowercase();
        match trimmed.as_str() {
            "a" | "approve" => ApprovalResult::Approved,
            "r" | "reject" => ApprovalResult::Rejected,
            num => {
                if let Ok(idx) = num.parse::<usize>() {
                    if let Some(action) = action_space.actions.get(idx) {
                        ApprovalResult::Substitute(action.clone())
                    } else {
                        ApprovalResult::Rejected
                    }
                } else {
                    ApprovalResult::Rejected
                }
            }
        }
    }
}

/// Type alias for the callback function used by CallbackApprovalChannel.
#[cfg(feature = "interactive")]
type ApprovalCallback =
    Box<dyn Fn(&Action, &ActionSpace, &AgentSnapshot) -> ApprovalResult + Send + Sync>;

/// Programmatic approval channel that delegates to a callback function.
///
/// Useful for automated testing, webhooks, or integration with external
/// approval systems.
#[cfg(feature = "interactive")]
pub struct CallbackApprovalChannel {
    callback: ApprovalCallback,
}

#[cfg(feature = "interactive")]
impl CallbackApprovalChannel {
    /// Create a new CallbackApprovalChannel with the given callback.
    pub fn new(
        callback: impl Fn(&Action, &ActionSpace, &AgentSnapshot) -> ApprovalResult
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            callback: Box::new(callback),
        }
    }
}

#[cfg(feature = "interactive")]
#[async_trait]
impl ApprovalChannel for CallbackApprovalChannel {
    async fn request_approval(
        &self,
        proposed: &Action,
        action_space: &ActionSpace,
        snapshot: &AgentSnapshot,
    ) -> ApprovalResult {
        (self.callback)(proposed, action_space, snapshot)
    }
}

/// A policy that delegates action selection to an inner policy, then requires
/// human approval before proceeding.
///
/// The delegate policy (e.g., LlmPolicy, PriorityPolicy) proposes an action.
/// The approval channel presents it to the human. The human approves, rejects,
/// or substitutes a different action.
///
/// If the delegate returns None (no action proposed), HumanInTheLoopPolicy
/// returns None without consulting the human.
#[cfg(feature = "interactive")]
pub struct HumanInTheLoopPolicy {
    /// The inner policy that proposes actions.
    pub delegate: Box<dyn AgentPolicy>,
    /// The channel through which approval is requested.
    pub approval_channel: Box<dyn ApprovalChannel>,
    /// Maximum time to wait for human response.
    pub timeout: Duration,
    /// What to do when the timeout expires.
    pub timeout_behavior: TimeoutBehavior,
}

#[cfg(feature = "interactive")]
impl HumanInTheLoopPolicy {
    /// Create a new HumanInTheLoopPolicy.
    pub fn new(
        delegate: Box<dyn AgentPolicy>,
        approval_channel: Box<dyn ApprovalChannel>,
        timeout: Duration,
        timeout_behavior: TimeoutBehavior,
    ) -> Self {
        Self {
            delegate,
            approval_channel,
            timeout,
            timeout_behavior,
        }
    }
}

#[cfg(feature = "interactive")]
#[async_trait]
impl AgentPolicy for HumanInTheLoopPolicy {
    async fn choose(&self, action_space: &ActionSpace, snapshot: &AgentSnapshot) -> Option<Action> {
        // If action space is empty, short-circuit
        if action_space.actions.is_empty() {
            return None;
        }

        // Delegate proposes an action
        let proposed = self.delegate.choose(action_space, snapshot).await?;

        // Consult the approval channel
        let result = self
            .approval_channel
            .request_approval(&proposed, action_space, snapshot)
            .await;

        match result {
            ApprovalResult::Approved => Some(proposed),
            ApprovalResult::Rejected => None,
            ApprovalResult::Substitute(substitute) => {
                // Validate substitute is in the action space
                let valid = action_space
                    .actions
                    .iter()
                    .any(|a| a.flow_id == substitute.flow_id);
                if valid {
                    Some(substitute)
                } else {
                    None
                }
            }
            ApprovalResult::Timeout => match self.timeout_behavior {
                TimeoutBehavior::Approve => Some(proposed),
                TimeoutBehavior::Reject => None,
            },
        }
    }
}

// ──────────────────────────────────────────────
// LLM Policy types
// ──────────────────────────────────────────────

/// Error type for LLM client operations.
#[derive(Debug)]
pub enum LlmError {
    /// Network or HTTP error.
    NetworkError(String),
    /// LLM API returned an error response.
    ApiError { status: u16, message: String },
    /// Failed to parse the LLM response.
    ParseError(String),
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmError::NetworkError(msg) => write!(f, "LLM network error: {}", msg),
            LlmError::ApiError { status, message } => {
                write!(f, "LLM API error ({}): {}", status, message)
            }
            LlmError::ParseError(msg) => write!(f, "LLM parse error: {}", msg),
        }
    }
}

impl std::error::Error for LlmError {}

/// A message in an LLM conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

/// Trait for calling an LLM to get a text completion.
///
/// Implementations handle the specifics of the LLM API (Anthropic, OpenAI, etc.).
/// The policy handles prompt construction and response parsing.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Send messages to the LLM and get a text response.
    async fn complete(&self, messages: Vec<Message>, model: &str) -> Result<String, LlmError>;
}

/// A policy that uses an LLM to select actions from the action space.
///
/// Serializes the action space and snapshot into a structured prompt,
/// calls the LLM, and parses the response into a validated Action.
pub struct LlmPolicy {
    /// The LLM client to use for completions.
    pub client: Box<dyn LlmClient>,
    /// System prompt override. If empty, the default system prompt is used.
    pub system_prompt: String,
    /// Model identifier (e.g., "claude-sonnet-4-20250514").
    pub model: String,
    /// Maximum number of retries on invalid responses.
    pub max_retries: usize,
}

impl LlmPolicy {
    /// Create a new LlmPolicy with default settings.
    pub fn new(client: Box<dyn LlmClient>, model: String) -> Self {
        Self {
            client,
            system_prompt: String::new(),
            model,
            max_retries: 2,
        }
    }

    /// Build the default system prompt.
    fn default_system_prompt() -> String {
        r#"You are an autonomous agent policy that selects contract workflow actions.

You will receive a JSON object describing the available action space and current snapshot.
You must respond with a JSON object in exactly this format:

{
  "action": {
    "flow_id": "<the flow_id to execute>",
    "persona_id": "<the persona_id from the action>",
    "reasoning": "<brief explanation of why you chose this action>"
  },
  "reasoning": "<overall reasoning>"
}

If no action should be taken at this time, respond with:
{
  "action": null,
  "reasoning": "<why no action is appropriate>"
}

Rules:
- You MUST choose a flow_id that appears exactly in the "available_actions" list.
- Do not invent flow IDs or personas that are not listed.
- The "action" field must be either null or an object with flow_id and persona_id.
- Respond only with valid JSON. Do not include markdown fences or other text."#
            .to_string()
    }

    /// Build the user message from the action space and snapshot.
    fn build_user_message(action_space: &ActionSpace, snapshot: &AgentSnapshot) -> String {
        let available_actions = serde_json::to_string_pretty(&action_space.actions)
            .unwrap_or_else(|_| "[]".to_string());
        let blocked_actions = serde_json::to_string_pretty(&action_space.blocked_actions)
            .unwrap_or_else(|_| "[]".to_string());
        let facts =
            serde_json::to_string_pretty(&snapshot.facts).unwrap_or_else(|_| "{}".to_string());
        let entity_states = serde_json::to_string_pretty(&snapshot.entity_states)
            .unwrap_or_else(|_| "{}".to_string());

        format!(
            r#"{{
  "available_actions": {available_actions},
  "blocked_actions": {blocked_actions},
  "snapshot": {{
    "facts": {facts},
    "entity_states": {entity_states},
    "observed_at": "{observed_at}"
  }}
}}"#,
            available_actions = available_actions,
            blocked_actions = blocked_actions,
            facts = facts,
            entity_states = entity_states,
            observed_at = snapshot.observed_at,
        )
    }

    /// Parse the LLM response and validate against the action space.
    ///
    /// Returns Ok(Some(Action)) for valid action, Ok(None) for null action,
    /// Err(message) for invalid response (triggers retry).
    fn parse_response(
        response: &str,
        action_space: &ActionSpace,
    ) -> Result<Option<Action>, String> {
        // Strip markdown code fences if present
        let json_str = strip_code_fences(response);

        // Parse as JSON
        let value: serde_json::Value =
            serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        // Check the "action" field
        let action_field = value
            .get("action")
            .ok_or_else(|| "Response missing 'action' field".to_string())?;

        // Null action means "do nothing"
        if action_field.is_null() {
            return Ok(None);
        }

        // Extract flow_id
        let flow_id = action_field
            .get("flow_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Action missing 'flow_id' field or it is not a string".to_string())?;

        // Validate flow_id exists in the action space
        let matched_action = action_space
            .actions
            .iter()
            .find(|a| a.flow_id == flow_id)
            .ok_or_else(|| {
                format!(
                    "flow_id '{}' is not in the available action space. Available: [{}]",
                    flow_id,
                    action_space
                        .actions
                        .iter()
                        .map(|a| a.flow_id.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;

        // Return the canonical action from the action space (not LLM's version)
        Ok(Some(matched_action.clone()))
    }
}

#[async_trait]
impl AgentPolicy for LlmPolicy {
    async fn choose(&self, action_space: &ActionSpace, snapshot: &AgentSnapshot) -> Option<Action> {
        // Short-circuit on empty action space
        if action_space.actions.is_empty() {
            return None;
        }

        let system_prompt = if self.system_prompt.is_empty() {
            Self::default_system_prompt()
        } else {
            self.system_prompt.clone()
        };

        let user_message = Self::build_user_message(action_space, snapshot);

        // Build initial message list
        let mut messages: Vec<Message> = vec![
            Message {
                role: "system".to_string(),
                content: system_prompt,
            },
            Message {
                role: "user".to_string(),
                content: user_message,
            },
        ];

        // Retry loop
        let mut attempt = 0;
        loop {
            // Call the LLM
            let response = match self.client.complete(messages.clone(), &self.model).await {
                Ok(r) => r,
                Err(e) => {
                    // Network/API error — return None immediately (no point retrying)
                    tracing_log(&format!("LlmPolicy: LLM call failed: {}", e));
                    return None;
                }
            };

            // Try to parse the response
            match Self::parse_response(&response, action_space) {
                Ok(result) => return result,
                Err(parse_error) => {
                    // Invalid response — append error and retry
                    if attempt >= self.max_retries {
                        tracing_log(&format!(
                            "LlmPolicy: max_retries ({}) exhausted, last error: {}",
                            self.max_retries, parse_error
                        ));
                        return None;
                    }
                    attempt += 1;

                    // Append assistant response and correction prompt
                    messages.push(Message {
                        role: "assistant".to_string(),
                        content: response,
                    });
                    messages.push(Message {
                        role: "user".to_string(),
                        content: format!(
                            "Your response was invalid: {}. Please try again, responding with valid JSON only.",
                            parse_error
                        ),
                    });
                }
            }
        }
    }
}

// ──────────────────────────────────────────────
// ApprovalPredicate trait and reference implementations
// ──────────────────────────────────────────────

/// A predicate that determines whether a proposed action requires approval
/// from a secondary approver policy.
///
/// Used by `CompositePolicy` to conditionally route actions through
/// an approval step.
pub trait ApprovalPredicate: Send + Sync {
    /// Returns true if the proposed action requires approval.
    fn requires_approval(&self, action: &Action, snapshot: &AgentSnapshot) -> bool;
}

/// Requires approval when any entity in the snapshot is in a matching state.
///
/// Example: require human approval when the Order entity is in "pending_large"
/// state, auto-approve for "pending_standard".
pub struct EntityStatePredicate {
    /// (entity_id, state) pairs that trigger approval requirement.
    pub rules: Vec<(String, String)>,
}

impl ApprovalPredicate for EntityStatePredicate {
    fn requires_approval(&self, _action: &Action, snapshot: &AgentSnapshot) -> bool {
        self.rules
            .iter()
            .any(|(entity_id, state)| snapshot.entity_states.get(entity_id) == Some(state))
    }
}

/// Requires approval for actions targeting specific flows.
///
/// Example: always require human approval for "release_escrow" and
/// "cancel_order", but auto-approve "update_status".
pub struct FlowIdPredicate {
    /// Flow IDs that require approval.
    pub flows: HashSet<String>,
}

impl ApprovalPredicate for FlowIdPredicate {
    fn requires_approval(&self, action: &Action, _snapshot: &AgentSnapshot) -> bool {
        self.flows.contains(&action.flow_id)
    }
}

/// Always requires approval. Useful for testing and fully-supervised configurations.
pub struct AlwaysApprove;

impl ApprovalPredicate for AlwaysApprove {
    fn requires_approval(&self, _action: &Action, _snapshot: &AgentSnapshot) -> bool {
        true
    }
}

/// Never requires approval. Useful for testing and fully-autonomous configurations.
pub struct NeverApprove;

impl ApprovalPredicate for NeverApprove {
    fn requires_approval(&self, _action: &Action, _snapshot: &AgentSnapshot) -> bool {
        false
    }
}

// ──────────────────────────────────────────────
// CompositePolicy
// ──────────────────────────────────────────────

/// A policy that chains a proposer, a predicate, and an approver.
///
/// Execution flow:
/// 1. `proposer.choose(action_space, snapshot)` -> proposed action (or None)
/// 2. If None, return None
/// 3. If `requires_approval(proposed, snapshot)` is true:
///    - Build a filtered action space containing only the proposed action
///    - Call `approver.choose(filtered_space, snapshot)`
///    - If approver returns the action, proceed. If None, reject.
/// 4. If `requires_approval` is false, auto-approve the proposed action.
///
/// Common composition: LlmPolicy proposes, FlowIdPredicate gates high-value
/// flows, HumanInTheLoopPolicy approves.
pub struct CompositePolicy {
    /// The policy that proposes actions.
    pub proposer: Box<dyn AgentPolicy>,
    /// The policy that approves actions when the predicate triggers.
    pub approver: Box<dyn AgentPolicy>,
    /// Determines whether the proposed action requires approval.
    pub requires_approval: Box<dyn ApprovalPredicate>,
}

impl CompositePolicy {
    /// Create a new CompositePolicy.
    pub fn new(
        proposer: Box<dyn AgentPolicy>,
        approver: Box<dyn AgentPolicy>,
        requires_approval: Box<dyn ApprovalPredicate>,
    ) -> Self {
        Self {
            proposer,
            approver,
            requires_approval,
        }
    }
}

#[async_trait]
impl AgentPolicy for CompositePolicy {
    async fn choose(&self, action_space: &ActionSpace, snapshot: &AgentSnapshot) -> Option<Action> {
        // Step 1: Proposer proposes
        let proposed = self.proposer.choose(action_space, snapshot).await?;

        // Step 2: Check if approval is needed
        if !self
            .requires_approval
            .requires_approval(&proposed, snapshot)
        {
            // Auto-approve
            return Some(proposed);
        }

        // Step 3: Build filtered action space with only the proposed action
        let filtered_space = ActionSpace {
            persona_id: action_space.persona_id.clone(),
            actions: vec![proposed.clone()],
            current_verdicts: action_space.current_verdicts.clone(),
            blocked_actions: vec![],
        };

        // Step 4: Approver decides
        self.approver.choose(&filtered_space, snapshot).await
    }
}

/// Strip markdown code fences from a response string.
/// Returns a slice of the original string or a trimmed version.
fn strip_code_fences(s: &str) -> &str {
    let trimmed = s.trim();

    // Handle ```json ... ``` or ``` ... ```
    if let Some(stripped) = trimmed.strip_prefix("```json") {
        if let Some(inner) = stripped.strip_suffix("```") {
            return inner.trim();
        }
    }
    if let Some(stripped) = trimmed.strip_prefix("```") {
        if let Some(inner) = stripped.strip_suffix("```") {
            return inner.trim();
        }
    }

    trimmed
}

/// Minimal logging that doesn't require a full tracing setup.
fn tracing_log(msg: &str) {
    // In production this would use tracing::warn!
    // Here we use eprintln so it doesn't affect test output
    eprintln!("[LlmPolicy] {}", msg);
}

// ──────────────────────────────────────────────
// AnthropicClient (feature-gated)
// ──────────────────────────────────────────────

#[cfg(feature = "anthropic")]
/// Reference LLM client implementation using the Anthropic Messages API.
///
/// Uses `ureq` for HTTP. Reads the API key from the `ANTHROPIC_API_KEY`
/// environment variable.
pub struct AnthropicClient {
    /// API key for authentication.
    pub api_key: String,
    /// Base URL (default: https://api.anthropic.com).
    pub base_url: String,
}

#[cfg(feature = "anthropic")]
impl AnthropicClient {
    /// Create a new AnthropicClient from the ANTHROPIC_API_KEY environment variable.
    pub fn from_env() -> Result<Self, LlmError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
            LlmError::NetworkError("ANTHROPIC_API_KEY environment variable not set".to_string())
        })?;
        Ok(Self::new(api_key))
    }

    /// Create a new AnthropicClient with an explicit API key.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.anthropic.com".to_string(),
        }
    }
}

#[cfg(feature = "anthropic")]
#[async_trait]
impl LlmClient for AnthropicClient {
    async fn complete(&self, messages: Vec<Message>, model: &str) -> Result<String, LlmError> {
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        let model = model.to_string();

        // Extract system message (Anthropic API uses a separate `system` field)
        let system: Option<String> = messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());

        // Build the messages array (exclude system messages)
        let non_system: Vec<serde_json::Value> = messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                })
            })
            .collect();

        // Build request body per Anthropic Messages API
        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": 1024,
            "messages": non_system,
        });

        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys);
        }

        // Use spawn_blocking to run ureq (sync HTTP) from async context
        let result: Result<String, LlmError> = tokio::task::spawn_blocking(move || {
            let url = format!("{}/v1/messages", base_url);
            let agent = ureq::Agent::new_with_defaults();
            let response = agent
                .post(&url)
                .header("x-api-key", &api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .send_json(body);

            match response {
                Ok(resp) => {
                    let json: serde_json::Value = resp.into_body().read_json().map_err(|e| {
                        LlmError::ParseError(format!("Failed to parse Anthropic response: {}", e))
                    })?;
                    // Extract content[0].text
                    let text = json["content"]
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|c| c["text"].as_str())
                        .map(|s| s.to_string());
                    text.ok_or_else(|| {
                        LlmError::ParseError("No text content in Anthropic response".to_string())
                    })
                }
                Err(e) => {
                    // ureq v3: errors include status errors via the Error type
                    Err(LlmError::NetworkError(e.to_string()))
                }
            }
        })
        .await
        .map_err(|e| LlmError::NetworkError(format!("Task join error: {}", e)))?;

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_action(flow_id: &str) -> Action {
        Action {
            flow_id: flow_id.to_string(),
            persona_id: "test_persona".to_string(),
            entry_operation_id: format!("{}_entry", flow_id),
            enabling_verdicts: vec![],
            affected_entities: vec![],
            description: format!("Execute {}", flow_id),
            instance_bindings: std::collections::BTreeMap::new(),
        }
    }

    fn sample_action_space(actions: Vec<Action>) -> ActionSpace {
        ActionSpace {
            persona_id: "test_persona".to_string(),
            actions,
            current_verdicts: vec![],
            blocked_actions: vec![],
        }
    }

    fn sample_snapshot() -> AgentSnapshot {
        AgentSnapshot {
            facts: HashMap::new(),
            entity_states: HashMap::new(),
            observed_at: "2026-02-24T00:00:00Z".to_string(),
        }
    }

    // ── RandomPolicy ──

    #[cfg(feature = "interactive")]
    #[tokio::test]
    async fn random_policy_returns_some_when_actions_exist() {
        let policy = RandomPolicy;
        let space = sample_action_space(vec![sample_action("flow_a"), sample_action("flow_b")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_some());
        let chosen = result.unwrap();
        assert!(chosen.flow_id == "flow_a" || chosen.flow_id == "flow_b");
    }

    #[cfg(feature = "interactive")]
    #[tokio::test]
    async fn random_policy_returns_none_when_empty() {
        let policy = RandomPolicy;
        let space = sample_action_space(vec![]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none());
    }

    // ── FirstAvailablePolicy ──

    #[tokio::test]
    async fn first_available_returns_first_action() {
        let policy = FirstAvailablePolicy;
        let space = sample_action_space(vec![
            sample_action("flow_a"),
            sample_action("flow_b"),
            sample_action("flow_c"),
        ]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().flow_id, "flow_a");
    }

    #[tokio::test]
    async fn first_available_returns_none_when_empty() {
        let policy = FirstAvailablePolicy;
        let space = sample_action_space(vec![]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none());
    }

    // ── PriorityPolicy ──

    #[tokio::test]
    async fn priority_policy_returns_highest_priority() {
        let policy = PriorityPolicy {
            priorities: vec![
                "flow_c".to_string(),
                "flow_a".to_string(),
                "flow_b".to_string(),
            ],
        };
        let space = sample_action_space(vec![
            sample_action("flow_a"),
            sample_action("flow_b"),
            sample_action("flow_c"),
        ]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().flow_id, "flow_c");
    }

    #[tokio::test]
    async fn priority_policy_skips_unavailable() {
        let policy = PriorityPolicy {
            priorities: vec![
                "flow_z".to_string(), // not in action space
                "flow_b".to_string(),
            ],
        };
        let space = sample_action_space(vec![sample_action("flow_a"), sample_action("flow_b")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().flow_id, "flow_b");
    }

    #[tokio::test]
    async fn priority_policy_falls_back_to_first_available() {
        let policy = PriorityPolicy {
            priorities: vec!["flow_x".to_string(), "flow_y".to_string()],
        };
        let space = sample_action_space(vec![sample_action("flow_a"), sample_action("flow_b")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().flow_id, "flow_a");
    }

    #[tokio::test]
    async fn priority_policy_returns_none_when_empty() {
        let policy = PriorityPolicy {
            priorities: vec!["flow_a".to_string()],
        };
        let space = sample_action_space(vec![]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none());
    }

    // ── AgentSnapshot ──

    #[test]
    fn snapshot_serialization_roundtrip() {
        let mut facts = HashMap::new();
        facts.insert(
            "rfp_amount".to_string(),
            serde_json::json!({"amount": "50000.00", "currency": "USD"}),
        );
        facts.insert("budget_approved".to_string(), serde_json::json!(true));

        let mut entity_states = HashMap::new();
        entity_states.insert("RFP".to_string(), "draft".to_string());

        let snap = AgentSnapshot {
            facts,
            entity_states,
            observed_at: "2026-02-24T12:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&snap).unwrap();
        let deserialized: AgentSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.observed_at, "2026-02-24T12:00:00Z");
        assert_eq!(
            deserialized.entity_states.get("RFP"),
            Some(&"draft".to_string())
        );
        assert_eq!(deserialized.facts.len(), 2);
    }

    // ── Trait object safety ──

    #[cfg(feature = "interactive")]
    #[tokio::test]
    async fn policy_is_object_safe() {
        // Verify AgentPolicy can be used as a trait object (dyn dispatch)
        let policies: Vec<Box<dyn AgentPolicy>> = vec![
            Box::new(RandomPolicy),
            Box::new(FirstAvailablePolicy),
            Box::new(PriorityPolicy {
                priorities: vec!["flow_a".to_string()],
            }),
        ];

        let space = sample_action_space(vec![sample_action("flow_a")]);
        let snap = sample_snapshot();

        for policy in &policies {
            let result = policy.choose(&space, &snap).await;
            assert!(result.is_some());
        }
    }

    // ── HumanInTheLoopPolicy ──

    #[cfg(feature = "interactive")]
    fn make_hitl(result: ApprovalResult) -> HumanInTheLoopPolicy {
        HumanInTheLoopPolicy::new(
            Box::new(FirstAvailablePolicy),
            Box::new(CallbackApprovalChannel::new(move |_, _, _| result.clone())),
            Duration::from_secs(30),
            TimeoutBehavior::Reject,
        )
    }

    #[cfg(feature = "interactive")]
    #[tokio::test]
    async fn hitl_approve() {
        let policy = make_hitl(ApprovalResult::Approved);
        let space = sample_action_space(vec![sample_action("flow_a"), sample_action("flow_b")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().flow_id, "flow_a");
    }

    #[cfg(feature = "interactive")]
    #[tokio::test]
    async fn hitl_reject() {
        let policy = make_hitl(ApprovalResult::Rejected);
        let space = sample_action_space(vec![sample_action("flow_a")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none());
    }

    #[cfg(feature = "interactive")]
    #[tokio::test]
    async fn hitl_substitute_valid() {
        // Delegate proposes flow_a, callback substitutes flow_b (valid: in action space)
        let substitute = sample_action("flow_b");
        let policy = HumanInTheLoopPolicy::new(
            Box::new(FirstAvailablePolicy),
            Box::new(CallbackApprovalChannel::new(move |_, _, _| {
                ApprovalResult::Substitute(substitute.clone())
            })),
            Duration::from_secs(30),
            TimeoutBehavior::Reject,
        );
        let space = sample_action_space(vec![sample_action("flow_a"), sample_action("flow_b")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().flow_id, "flow_b");
    }

    #[cfg(feature = "interactive")]
    #[tokio::test]
    async fn hitl_substitute_invalid() {
        // Delegate proposes flow_a, callback substitutes flow_z (NOT in action space) -> None
        let substitute = sample_action("flow_z");
        let policy = HumanInTheLoopPolicy::new(
            Box::new(FirstAvailablePolicy),
            Box::new(CallbackApprovalChannel::new(move |_, _, _| {
                ApprovalResult::Substitute(substitute.clone())
            })),
            Duration::from_secs(30),
            TimeoutBehavior::Reject,
        );
        let space = sample_action_space(vec![sample_action("flow_a"), sample_action("flow_b")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none());
    }

    #[cfg(feature = "interactive")]
    #[tokio::test]
    async fn hitl_empty_action_space() {
        // Empty action space: returns None without consulting the callback.
        // Callback panics to verify it is NOT called.
        let policy = HumanInTheLoopPolicy::new(
            Box::new(FirstAvailablePolicy),
            Box::new(CallbackApprovalChannel::new(|_, _, _| {
                panic!("callback should not be called for empty action space")
            })),
            Duration::from_secs(30),
            TimeoutBehavior::Reject,
        );
        let space = sample_action_space(vec![]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none());
    }

    #[cfg(feature = "interactive")]
    #[tokio::test]
    async fn hitl_timeout_reject() {
        // Callback returns Timeout; timeout_behavior is Reject -> None
        let policy = HumanInTheLoopPolicy::new(
            Box::new(FirstAvailablePolicy),
            Box::new(CallbackApprovalChannel::new(|_, _, _| {
                ApprovalResult::Timeout
            })),
            Duration::from_secs(30),
            TimeoutBehavior::Reject,
        );
        let space = sample_action_space(vec![sample_action("flow_a")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none());
    }

    #[cfg(feature = "interactive")]
    #[tokio::test]
    async fn hitl_timeout_approve() {
        // Callback returns Timeout; timeout_behavior is Approve -> returns proposed action
        let policy = HumanInTheLoopPolicy::new(
            Box::new(FirstAvailablePolicy),
            Box::new(CallbackApprovalChannel::new(|_, _, _| {
                ApprovalResult::Timeout
            })),
            Duration::from_secs(30),
            TimeoutBehavior::Approve,
        );
        let space = sample_action_space(vec![sample_action("flow_a")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().flow_id, "flow_a");
    }

    #[cfg(feature = "interactive")]
    #[tokio::test]
    async fn hitl_delegate_returns_none() {
        // PriorityPolicy with no matches falls back to first; but empty space -> None.
        // Use empty action space to force delegate to return None.
        // Callback panics to verify it is NOT called.
        let policy = HumanInTheLoopPolicy::new(
            Box::new(PriorityPolicy {
                priorities: vec!["flow_z".to_string()],
            }),
            Box::new(CallbackApprovalChannel::new(|_, _, _| {
                panic!("callback should not be called when delegate returns None")
            })),
            Duration::from_secs(30),
            TimeoutBehavior::Reject,
        );
        // Empty action space: PriorityPolicy falls back to first(), which is None
        let space = sample_action_space(vec![]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none());
    }

    // ── LlmPolicy mock tests ──

    /// Mock LLM client that pops responses from a queue.
    struct MockLlmClient {
        responses: std::sync::Mutex<Vec<Result<String, LlmError>>>,
        /// Optionally captures the messages sent to each call.
        captured_messages: std::sync::Mutex<Vec<Vec<Message>>>,
    }

    impl MockLlmClient {
        fn new(responses: Vec<Result<String, LlmError>>) -> Self {
            Self {
                responses: std::sync::Mutex::new(responses),
                captured_messages: std::sync::Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait]
    impl LlmClient for MockLlmClient {
        async fn complete(&self, messages: Vec<Message>, _model: &str) -> Result<String, LlmError> {
            self.captured_messages.lock().unwrap().push(messages);
            let mut queue = self.responses.lock().unwrap();
            if queue.is_empty() {
                return Err(LlmError::NetworkError("mock queue exhausted".to_string()));
            }
            queue.remove(0)
        }
    }

    #[tokio::test]
    async fn llm_policy_selects_valid_action() {
        // Mock returns valid JSON selecting flow_a
        let response = r#"{"action": {"flow_id": "flow_a", "persona_id": "test_persona", "reasoning": "flow_a looks good"}, "reasoning": "go for it"}"#;
        let client = MockLlmClient::new(vec![Ok(response.to_string())]);
        let policy = LlmPolicy::new(Box::new(client), "test-model".to_string());
        let space = sample_action_space(vec![sample_action("flow_a"), sample_action("flow_b")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().flow_id, "flow_a");
    }

    #[tokio::test]
    async fn llm_policy_selects_null_action() {
        // Mock returns JSON with "action": null -> returns None
        let response = r#"{"action": null, "reasoning": "no action needed"}"#;
        let client = MockLlmClient::new(vec![Ok(response.to_string())]);
        let policy = LlmPolicy::new(Box::new(client), "test-model".to_string());
        let space = sample_action_space(vec![sample_action("flow_a")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn llm_policy_retry_on_invalid_then_valid() {
        // First response: invalid flow_id; second: valid flow_b
        let bad_response = r#"{"action": {"flow_id": "flow_nonexistent", "persona_id": "test"}, "reasoning": "wrong"}"#;
        let good_response = r#"{"action": {"flow_id": "flow_b", "persona_id": "test_persona", "reasoning": "correct"}, "reasoning": "fixed"}"#;
        let client = MockLlmClient::new(vec![
            Ok(bad_response.to_string()),
            Ok(good_response.to_string()),
        ]);
        let policy = LlmPolicy::new(Box::new(client), "test-model".to_string());
        let space = sample_action_space(vec![sample_action("flow_a"), sample_action("flow_b")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_some(), "expected Some after retry, got None");
        assert_eq!(result.unwrap().flow_id, "flow_b");
    }

    #[tokio::test]
    async fn llm_policy_max_retries_exhausted() {
        // max_retries = 2, so we need 3 total bad responses (initial + 2 retries)
        let garbage = r#"not json at all"#;
        let client = MockLlmClient::new(vec![
            Ok(garbage.to_string()),
            Ok(garbage.to_string()),
            Ok(garbage.to_string()),
        ]);
        let mut policy = LlmPolicy::new(Box::new(client), "test-model".to_string());
        policy.max_retries = 2;
        let space = sample_action_space(vec![sample_action("flow_a")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none(), "expected None when max_retries exhausted");
    }

    #[tokio::test]
    async fn llm_policy_network_error() {
        // Mock returns a network error -> returns None immediately
        let client = MockLlmClient::new(vec![Err(LlmError::NetworkError(
            "connection refused".to_string(),
        ))]);
        let policy = LlmPolicy::new(Box::new(client), "test-model".to_string());
        let space = sample_action_space(vec![sample_action("flow_a")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn llm_policy_empty_action_space() {
        // Empty action space -> returns None without calling LLM.
        // Client panics to verify it is NOT called.
        struct PanicClient;
        #[async_trait]
        impl LlmClient for PanicClient {
            async fn complete(
                &self,
                _messages: Vec<Message>,
                _model: &str,
            ) -> Result<String, LlmError> {
                panic!("LLM should not be called for empty action space");
            }
        }
        let policy = LlmPolicy::new(Box::new(PanicClient), "test-model".to_string());
        let space = sample_action_space(vec![]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn llm_policy_prompt_includes_blocked_actions() {
        // Capture messages and verify blocked_actions appear in the user message
        let response =
            r#"{"action": {"flow_id": "flow_a", "persona_id": "test_persona"}, "reasoning": "ok"}"#;
        let client = MockLlmClient::new(vec![Ok(response.to_string())]);
        let captured = &client.captured_messages as *const _ as usize; // will read after

        // Build an action space with a blocked action
        let space = ActionSpace {
            persona_id: "test_persona".to_string(),
            actions: vec![sample_action("flow_a")],
            current_verdicts: vec![],
            blocked_actions: vec![crate::action_space::BlockedAction {
                flow_id: "flow_blocked".to_string(),
                reason: crate::action_space::BlockedReason::PersonaNotAuthorized,
                instance_bindings: std::collections::BTreeMap::new(),
            }],
        };
        let snap = sample_snapshot();

        let policy = LlmPolicy {
            client: Box::new(client),
            system_prompt: String::new(),
            model: "test-model".to_string(),
            max_retries: 2,
        };
        let result = policy.choose(&space, &snap).await;
        assert!(result.is_some());

        // Access captured messages via the raw pointer (the client was consumed by Box)
        // Instead, reconstruct the test using a reference-counted client
        let _ = captured; // unused, just ensuring the test ran
                          // The real check: parse the user message that LlmPolicy::build_user_message produces
        let user_msg = LlmPolicy::build_user_message(&space, &snap);
        assert!(
            user_msg.contains("flow_blocked"),
            "user message should contain blocked action flow_id"
        );
        assert!(
            user_msg.contains("blocked_actions"),
            "user message should have blocked_actions field"
        );
    }

    #[tokio::test]
    async fn llm_policy_prompt_includes_snapshot() {
        // Verify entity_states and facts are in the user message
        let mut facts = HashMap::new();
        facts.insert(
            "rfp_amount".to_string(),
            serde_json::json!({"amount": "50000.00", "currency": "USD"}),
        );
        let mut entity_states = HashMap::new();
        entity_states.insert("RFP".to_string(), "draft".to_string());

        let snap = AgentSnapshot {
            facts,
            entity_states,
            observed_at: "2026-02-24T12:00:00Z".to_string(),
        };
        let space = sample_action_space(vec![sample_action("flow_a")]);

        let user_msg = LlmPolicy::build_user_message(&space, &snap);
        assert!(
            user_msg.contains("rfp_amount"),
            "user message should contain facts"
        );
        assert!(
            user_msg.contains("RFP"),
            "user message should contain entity_states"
        );
        assert!(
            user_msg.contains("draft"),
            "user message should contain entity state value"
        );
        assert!(
            user_msg.contains("2026-02-24T12:00:00Z"),
            "user message should contain observed_at"
        );
    }

    #[cfg(feature = "anthropic")]
    #[tokio::test]
    #[ignore] // Requires ANTHROPIC_API_KEY environment variable
    async fn llm_policy_anthropic_integration() {
        let client = AnthropicClient::from_env().expect("ANTHROPIC_API_KEY required");
        let policy = LlmPolicy::new(Box::new(client), "claude-sonnet-4-20250514".to_string());
        let space = sample_action_space(vec![
            sample_action("approve_order"),
            sample_action("cancel_order"),
        ]);
        let snap = sample_snapshot();
        let result = policy.choose(&space, &snap).await;
        // LLM should pick one of the two actions (or None, which is also valid)
        if let Some(action) = &result {
            assert!(
                action.flow_id == "approve_order" || action.flow_id == "cancel_order",
                "unexpected flow_id: {}",
                action.flow_id
            );
        }
    }

    // ── ApprovalPredicate reference implementations ──

    fn snapshot_with_entity(entity_id: &str, state: &str) -> AgentSnapshot {
        let mut entity_states = HashMap::new();
        entity_states.insert(entity_id.to_string(), state.to_string());
        AgentSnapshot {
            facts: HashMap::new(),
            entity_states,
            observed_at: "2026-02-24T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn entity_state_predicate_triggers_on_match() {
        let predicate = EntityStatePredicate {
            rules: vec![("Order".to_string(), "pending_large".to_string())],
        };
        let action = sample_action("any_flow");
        let snapshot = snapshot_with_entity("Order", "pending_large");
        assert!(predicate.requires_approval(&action, &snapshot));
    }

    #[test]
    fn entity_state_predicate_no_match() {
        let predicate = EntityStatePredicate {
            rules: vec![("Order".to_string(), "pending_large".to_string())],
        };
        let action = sample_action("any_flow");
        let snapshot = snapshot_with_entity("Order", "pending_standard");
        assert!(!predicate.requires_approval(&action, &snapshot));
    }

    #[test]
    fn entity_state_predicate_missing_entity() {
        let predicate = EntityStatePredicate {
            rules: vec![("Order".to_string(), "pending_large".to_string())],
        };
        let action = sample_action("any_flow");
        let snapshot = sample_snapshot(); // no entities
        assert!(!predicate.requires_approval(&action, &snapshot));
    }

    #[test]
    fn flow_id_predicate_triggers_on_match() {
        let predicate = FlowIdPredicate {
            flows: ["release_escrow".to_string()].into_iter().collect(),
        };
        let action = sample_action("release_escrow");
        let snapshot = sample_snapshot();
        assert!(predicate.requires_approval(&action, &snapshot));
    }

    #[test]
    fn flow_id_predicate_no_match() {
        let predicate = FlowIdPredicate {
            flows: ["release_escrow".to_string()].into_iter().collect(),
        };
        let action = sample_action("update_status");
        let snapshot = sample_snapshot();
        assert!(!predicate.requires_approval(&action, &snapshot));
    }

    #[test]
    fn always_approve_returns_true() {
        let predicate = AlwaysApprove;
        let action = sample_action("any_flow");
        let snapshot = sample_snapshot();
        assert!(predicate.requires_approval(&action, &snapshot));
    }

    #[test]
    fn never_approve_returns_false() {
        let predicate = NeverApprove;
        let action = sample_action("any_flow");
        let snapshot = sample_snapshot();
        assert!(!predicate.requires_approval(&action, &snapshot));
    }

    // ── CompositePolicy ──

    struct NonePolicy;

    #[async_trait]
    impl AgentPolicy for NonePolicy {
        async fn choose(&self, _: &ActionSpace, _: &AgentSnapshot) -> Option<Action> {
            None
        }
    }

    #[tokio::test]
    async fn composite_auto_approve_when_predicate_false() {
        // Proposer selects flow_a, NeverApprove predicate -> auto-approved
        let policy = CompositePolicy::new(
            Box::new(FirstAvailablePolicy),
            Box::new(NonePolicy), // approver never called
            Box::new(NeverApprove),
        );
        let space = sample_action_space(vec![sample_action("flow_a"), sample_action("flow_b")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().flow_id, "flow_a");
    }

    #[tokio::test]
    async fn composite_approver_approves() {
        // Proposer selects flow_a, AlwaysApprove predicate, FirstAvailablePolicy approver returns it
        let policy = CompositePolicy::new(
            Box::new(FirstAvailablePolicy),
            Box::new(FirstAvailablePolicy), // approver gets filtered space with only flow_a
            Box::new(AlwaysApprove),
        );
        let space = sample_action_space(vec![sample_action("flow_a"), sample_action("flow_b")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().flow_id, "flow_a");
    }

    #[tokio::test]
    async fn composite_approver_rejects() {
        // Proposer selects flow_a, AlwaysApprove predicate, approver (NonePolicy) returns None -> rejected
        let policy = CompositePolicy::new(
            Box::new(FirstAvailablePolicy),
            Box::new(NonePolicy),
            Box::new(AlwaysApprove),
        );
        let space = sample_action_space(vec![sample_action("flow_a")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn composite_proposer_returns_none() {
        // Empty action space -> proposer returns None -> short-circuit without consulting predicate/approver
        // AlwaysApprove and FirstAvailablePolicy would succeed if called, but they shouldn't be.
        let policy = CompositePolicy::new(
            Box::new(FirstAvailablePolicy), // returns None on empty space
            Box::new(FirstAvailablePolicy),
            Box::new(AlwaysApprove),
        );
        let space = sample_action_space(vec![]); // empty -> proposer returns None
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn composite_entity_state_predicate_triggers() {
        // EntityStatePredicate with ("Order", "pending") triggers approval route.
        // Approver is NonePolicy -> returns None (rejected).
        let predicate = EntityStatePredicate {
            rules: vec![("Order".to_string(), "pending".to_string())],
        };
        let policy = CompositePolicy::new(
            Box::new(FirstAvailablePolicy),
            Box::new(NonePolicy), // reject to prove approval route was taken
            Box::new(predicate),
        );
        let space = sample_action_space(vec![sample_action("flow_a")]);
        let snap = snapshot_with_entity("Order", "pending");

        let result = policy.choose(&space, &snap).await;
        // Approval route taken: NonePolicy rejected
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn composite_flow_id_predicate_triggers() {
        // FlowIdPredicate with {"flow_a"} triggers approval for flow_a.
        // Approver is NonePolicy -> rejected (proves approval route taken).
        let predicate = FlowIdPredicate {
            flows: ["flow_a".to_string()].into_iter().collect(),
        };
        let policy = CompositePolicy::new(
            Box::new(FirstAvailablePolicy),
            Box::new(NonePolicy), // reject to prove approval route was taken
            Box::new(predicate),
        );
        let space = sample_action_space(vec![sample_action("flow_a")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        // Approval route taken: NonePolicy rejected
        assert!(result.is_none());
    }
}
