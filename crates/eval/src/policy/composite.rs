//! CompositePolicy and ApprovalPredicate trait with reference implementations.

use async_trait::async_trait;
use std::collections::HashSet;

use crate::action_space::{Action, ActionSpace};
use crate::policy::{AgentPolicy, AgentSnapshot};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::basic::FirstAvailablePolicy;
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

    fn snapshot_with_entity(entity_id: &str, state: &str) -> AgentSnapshot {
        let mut entity_states = HashMap::new();
        entity_states.insert(entity_id.to_string(), state.to_string());
        AgentSnapshot {
            facts: HashMap::new(),
            entity_states,
            observed_at: "2026-02-24T00:00:00Z".to_string(),
        }
    }

    // -- ApprovalPredicate reference implementations --

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

    // -- CompositePolicy --

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
