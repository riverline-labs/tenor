//! Agent policy trait and reference implementations.
//!
//! The `AgentPolicy` trait is the single point where "intelligence" lives.
//! Everything else -- action space computation, contract enforcement,
//! execution, provenance -- is mechanics. The policy just picks a legal move.
//!
//! Tenor does not care how the decision is made. It only cares that the
//! decision is an element of the action space.

mod basic;
mod composite;
#[cfg(feature = "interactive")]
mod human;
mod llm;

// Re-export everything so external callers don't break.
pub use basic::{FirstAvailablePolicy, PriorityPolicy};
pub use composite::{
    AlwaysApprove, ApprovalPredicate, CompositePolicy, EntityStatePredicate, FlowIdPredicate,
    NeverApprove,
};
#[cfg(feature = "interactive")]
pub use human::{CallbackApprovalChannel, HumanInTheLoopPolicy, StdinApprovalChannel};
pub use llm::{LlmClient, LlmError, LlmPolicy, Message};

#[cfg(feature = "interactive")]
pub use basic::RandomPolicy;

#[cfg(feature = "anthropic")]
pub use llm::AnthropicClient;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::action_space::{Action, ActionSpace};

/// Result of presenting a proposed action to a human for approval.
#[derive(Debug, Clone)]
pub enum ApprovalResult {
    /// Proceed with the proposed action.
    Approved,
    /// Abort -- return None from choose().
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
/// The `ActionSpace` is computed FROM this snapshot -- but the snapshot
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

/// Strip markdown code fences from a response string.
/// Returns a slice of the original string or a trimmed version.
pub(crate) fn strip_code_fences(s: &str) -> &str {
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
pub(crate) fn tracing_log(msg: &str) {
    // In production this would use tracing::warn!
    // Here we use eprintln so it doesn't affect test output
    eprintln!("[LlmPolicy] {}", msg);
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

    // -- AgentSnapshot --

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

    // -- Trait object safety --

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
}
