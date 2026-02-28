//! Basic policy implementations: RandomPolicy, FirstAvailablePolicy, PriorityPolicy.

use async_trait::async_trait;
#[cfg(feature = "interactive")]
use rand::seq::SliceRandom;

use crate::action_space::{Action, ActionSpace};
use crate::policy::{AgentPolicy, AgentSnapshot};

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

    // -- RandomPolicy --

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

    // -- FirstAvailablePolicy --

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

    // -- PriorityPolicy --

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
}
