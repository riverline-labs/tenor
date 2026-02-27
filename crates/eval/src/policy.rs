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
use std::io::{self, BufRead, Write};
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

/// Interactive terminal approval channel that reads from stdin.
///
/// Presents the proposed action and available alternatives, then waits
/// for the user to approve, reject, or select a substitute.
pub struct StdinApprovalChannel;

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
type ApprovalCallback =
    Box<dyn Fn(&Action, &ActionSpace, &AgentSnapshot) -> ApprovalResult + Send + Sync>;

/// Programmatic approval channel that delegates to a callback function.
///
/// Useful for automated testing, webhooks, or integration with external
/// approval systems.
pub struct CallbackApprovalChannel {
    callback: ApprovalCallback,
}

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

    // ── HumanInTheLoopPolicy ──

    fn make_hitl(result: ApprovalResult) -> HumanInTheLoopPolicy {
        HumanInTheLoopPolicy::new(
            Box::new(FirstAvailablePolicy),
            Box::new(CallbackApprovalChannel::new(move |_, _, _| result.clone())),
            Duration::from_secs(30),
            TimeoutBehavior::Reject,
        )
    }

    #[tokio::test]
    async fn hitl_approve() {
        let policy = make_hitl(ApprovalResult::Approved);
        let space = sample_action_space(vec![sample_action("flow_a"), sample_action("flow_b")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().flow_id, "flow_a");
    }

    #[tokio::test]
    async fn hitl_reject() {
        let policy = make_hitl(ApprovalResult::Rejected);
        let space = sample_action_space(vec![sample_action("flow_a")]);
        let snap = sample_snapshot();

        let result = policy.choose(&space, &snap).await;
        assert!(result.is_none());
    }

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
}
