//! Agent policy trait and reference implementations.
//!
//! The `AgentPolicy` trait is the single point where "intelligence" lives.
//! Everything else — action space computation, contract enforcement,
//! execution, provenance — is mechanics. The policy just picks a legal move.
//!
//! Tenor does not care how the decision is made. It only cares that the
//! decision is an element of the action space.

use async_trait::async_trait;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::action_space::{Action, ActionSpace};

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
pub struct RandomPolicy;

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
