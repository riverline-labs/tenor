//! LLM-based policy: LlmPolicy, LlmClient trait, AnthropicClient.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::action_space::{Action, ActionSpace};
use crate::policy::{strip_code_fences, tracing_log, AgentPolicy, AgentSnapshot};

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
                    // Network/API error -- return None immediately (no point retrying)
                    tracing_log(&format!("LlmPolicy: LLM call failed: {}", e));
                    return None;
                }
            };

            // Try to parse the response
            match Self::parse_response(&response, action_space) {
                Ok(result) => return result,
                Err(parse_error) => {
                    // Invalid response -- append error and retry
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

// -- AnthropicClient (feature-gated) --

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
    use std::collections::HashMap;

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
}
