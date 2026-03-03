//! Action space extraction — computes the set of actions available to a persona.

use std::collections::{BTreeMap, BTreeSet};

use crate::assemble;
use crate::operation::{EntityStateMap, DEFAULT_INSTANCE_ID};
use crate::rules;
use crate::types::{Contract, EvalError, FlowStep, Predicate, VerdictSet};
use serde::{Deserialize, Serialize};

/// A single executable action available to a persona.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub flow_id: String,
    pub persona_id: String,
    pub entry_operation_id: String,
    pub enabling_verdicts: Vec<VerdictSummary>,
    pub affected_entities: Vec<EntitySummary>,
    pub description: String,
    /// Per §15.6: entity_id → set of instance_ids that are in a valid source state
    /// for this flow's entry operation effects. For single-instance contracts, this
    /// will contain `_default` for each entity.
    pub instance_bindings: BTreeMap<String, BTreeSet<String>>,
}

/// Summary of a verdict for action space context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerdictSummary {
    pub verdict_type: String,
    pub payload: serde_json::Value,
    pub producing_rule: String,
    pub stratum: u32,
}

/// Summary of an entity's current state and possible transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySummary {
    pub entity_id: String,
    pub current_state: String,
    pub possible_transitions: Vec<String>,
}

/// The complete action space for a persona at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionSpace {
    pub persona_id: String,
    pub actions: Vec<Action>,
    pub current_verdicts: Vec<VerdictSummary>,
    pub blocked_actions: Vec<BlockedAction>,
}

/// An action that exists but is NOT currently executable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedAction {
    pub flow_id: String,
    pub reason: BlockedReason,
    /// Per §15.6: entity_id → set of instance_ids that are blocking this action.
    /// Populated when the reason is EntityNotInSourceState. Empty otherwise.
    pub instance_bindings: BTreeMap<String, BTreeSet<String>>,
}

/// Why an action is blocked.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum BlockedReason {
    PersonaNotAuthorized,
    PreconditionNotMet {
        missing_verdicts: Vec<String>,
    },
    EntityNotInSourceState {
        entity_id: String,
        current_state: String,
        required_state: String,
    },
    MissingFacts {
        fact_ids: Vec<String>,
    },
}

/// Compute the complete action space for a persona given the current state of the world.
///
/// Pure function. No IO. No side effects. No state mutation.
///
/// Answers: "What can this persona do right now, and why?"
///
/// Per §15.6: the action space is instance-keyed. For each flow, we determine which
/// specific instances are in valid source states for the flow's entry operation effects.
/// The `instance_bindings` field on `Action` maps entity_id → set of valid instance_ids.
pub fn compute_action_space(
    contract: &Contract,
    facts: &serde_json::Value,
    entity_states: &EntityStateMap,
    persona_id: &str,
) -> Result<ActionSpace, EvalError> {
    // 1. Assemble facts
    let fact_set = assemble::assemble_facts(contract, facts)?;

    // 2. Evaluate rules to get verdicts
    let verdict_set = rules::eval_strata(contract, &fact_set)?;

    // 3. Build current_verdicts
    let current_verdicts = verdict_set_to_summaries(&verdict_set);

    // 4. Analyze each flow
    let mut actions = Vec::new();
    let mut blocked_actions = Vec::new();

    for flow in &contract.flows {
        // Find entry step
        let entry_step = match flow.steps.iter().find(|s| step_id(s) == flow.entry) {
            Some(s) => s,
            None => continue,
        };

        // Entry must be an OperationStep — otherwise not persona-initiable
        let op_id = match entry_step {
            FlowStep::OperationStep { op, .. } => op.as_str(),
            _ => continue,
        };

        // Look up the operation
        let operation = match contract.get_operation(op_id) {
            Some(op) => op,
            None => continue,
        };

        // Check 1: Persona authorization
        if !operation.allowed_personas.contains(&persona_id.to_string()) {
            blocked_actions.push(BlockedAction {
                flow_id: flow.id.clone(),
                reason: BlockedReason::PersonaNotAuthorized,
                instance_bindings: BTreeMap::new(),
            });
            continue;
        }

        // Check 2: Precondition verdicts
        let required_verdicts = extract_verdict_refs(&operation.precondition);
        let missing: Vec<String> = required_verdicts
            .iter()
            .filter(|v| !verdict_set.has_verdict(v))
            .cloned()
            .collect();

        if !missing.is_empty() {
            blocked_actions.push(BlockedAction {
                flow_id: flow.id.clone(),
                reason: BlockedReason::PreconditionNotMet {
                    missing_verdicts: missing,
                },
                instance_bindings: BTreeMap::new(),
            });
            continue;
        }

        // Check 3: Entity states — per §15.6, check per-instance availability.
        //
        // For each entity referenced by the entry operation's effects, collect
        // all instance_ids that are in the required source state. An action is
        // available if every required entity has at least one instance in the
        // correct source state. The instance_bindings map captures which instances
        // are valid.
        //
        // For single-instance contracts (only `_default` per entity), this behaves
        // identically to the pre-multi-instance check.
        let mut entity_blocked = false;
        let mut blocking_instance_bindings: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut valid_instance_bindings: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for effect in &operation.effects {
            let entity_id = &effect.entity_id;

            // Collect all instances of this entity and their states
            let mut valid_instances: BTreeSet<String> = BTreeSet::new();
            let mut blocking_instances: BTreeSet<String> = BTreeSet::new();
            let mut found_any = false;

            for ((eid, iid), state) in entity_states {
                if eid != entity_id {
                    continue;
                }
                found_any = true;
                if state == &effect.from {
                    valid_instances.insert(iid.clone());
                } else {
                    blocking_instances.insert(iid.clone());
                }
            }

            if !found_any {
                // No instances found for this entity — treat as blocked
                blocking_instance_bindings
                    .entry(entity_id.clone())
                    .or_default()
                    .insert(DEFAULT_INSTANCE_ID.to_string());
                blocked_actions.push(BlockedAction {
                    flow_id: flow.id.clone(),
                    reason: BlockedReason::EntityNotInSourceState {
                        entity_id: entity_id.clone(),
                        current_state: "(unknown)".to_string(),
                        required_state: effect.from.clone(),
                    },
                    instance_bindings: blocking_instance_bindings.clone(),
                });
                entity_blocked = true;
                break;
            }

            if valid_instances.is_empty() {
                // All instances of this entity are in wrong states
                if let Some(((_, iid), state)) =
                    entity_states.iter().find(|((eid, _), _)| eid == entity_id)
                {
                    blocking_instance_bindings
                        .entry(entity_id.clone())
                        .or_default()
                        .insert(iid.clone());
                    blocked_actions.push(BlockedAction {
                        flow_id: flow.id.clone(),
                        reason: BlockedReason::EntityNotInSourceState {
                            entity_id: entity_id.clone(),
                            current_state: state.clone(),
                            required_state: effect.from.clone(),
                        },
                        instance_bindings: blocking_instance_bindings.clone(),
                    });
                }
                entity_blocked = true;
                break;
            }

            // Some or all instances are valid — record them
            *valid_instance_bindings
                .entry(entity_id.clone())
                .or_default() = valid_instances;

            // Also record blocking instances for information
            if !blocking_instances.is_empty() {
                *blocking_instance_bindings
                    .entry(entity_id.clone())
                    .or_default() = blocking_instances;
            }
        }
        if entity_blocked {
            continue;
        }

        // All checks pass — build Action
        let enabling_verdicts: Vec<VerdictSummary> = required_verdicts
            .iter()
            .filter_map(|vt| {
                verdict_set.get_verdict(vt).map(|vi| VerdictSummary {
                    verdict_type: vi.verdict_type.clone(),
                    payload: vi.payload.to_json(),
                    producing_rule: vi.provenance.rule_id.clone(),
                    stratum: vi.provenance.stratum,
                })
            })
            .collect();

        let affected_entities: Vec<EntitySummary> = operation
            .effects
            .iter()
            .map(|effect| {
                // For affected_entities summary, pick any valid instance's state.
                // We use DEFAULT_INSTANCE_ID for single-instance backward compat,
                // but for multi-instance we pick the first valid instance.
                let valid_set = valid_instance_bindings.get(&effect.entity_id);
                let lookup_instance = valid_set
                    .and_then(|s| s.iter().next())
                    .map(|s| s.as_str())
                    .unwrap_or(DEFAULT_INSTANCE_ID);
                let key = (effect.entity_id.clone(), lookup_instance.to_string());
                let current_state = entity_states
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| "(unknown)".to_string());
                let possible_transitions = contract
                    .get_entity(&effect.entity_id)
                    .map(|e| {
                        e.transitions
                            .iter()
                            .filter(|t| t.from == current_state)
                            .map(|t| t.to.clone())
                            .collect()
                    })
                    .unwrap_or_default();
                EntitySummary {
                    entity_id: effect.entity_id.clone(),
                    current_state,
                    possible_transitions,
                }
            })
            .collect();

        let description = build_description(&flow.id, op_id, &operation.effects);

        // If no effects, instance_bindings is empty (no entity constraints)
        actions.push(Action {
            flow_id: flow.id.clone(),
            persona_id: persona_id.to_string(),
            entry_operation_id: op_id.to_string(),
            enabling_verdicts,
            affected_entities,
            description,
            instance_bindings: valid_instance_bindings,
        });
    }

    Ok(ActionSpace {
        persona_id: persona_id.to_string(),
        actions,
        current_verdicts,
        blocked_actions,
    })
}

/// Get the id of a flow step.
fn step_id(step: &FlowStep) -> &str {
    match step {
        FlowStep::OperationStep { id, .. }
        | FlowStep::BranchStep { id, .. }
        | FlowStep::HandoffStep { id, .. }
        | FlowStep::SubFlowStep { id, .. }
        | FlowStep::ParallelStep { id, .. } => id,
    }
}

/// Convert a VerdictSet to a Vec<VerdictSummary>.
fn verdict_set_to_summaries(vs: &VerdictSet) -> Vec<VerdictSummary> {
    vs.0.iter()
        .map(|vi| VerdictSummary {
            verdict_type: vi.verdict_type.clone(),
            payload: vi.payload.to_json(),
            producing_rule: vi.provenance.rule_id.clone(),
            stratum: vi.provenance.stratum,
        })
        .collect()
}

/// Build a human-readable description for an action.
fn build_description(flow_id: &str, op_id: &str, effects: &[crate::types::Effect]) -> String {
    if effects.is_empty() {
        return format!("Execute {}: {}", flow_id, op_id);
    }
    let transitions: Vec<String> = effects
        .iter()
        .map(|e| format!("{} from {} to {}", e.entity_id, e.from, e.to))
        .collect();
    format!(
        "Execute {}: {} transitions {}",
        flow_id,
        op_id,
        transitions.join(", ")
    )
}

/// Walk a predicate AST and collect all VerdictPresent node identifiers.
pub(crate) fn extract_verdict_refs(pred: &Predicate) -> Vec<String> {
    let mut refs = Vec::new();
    collect_verdict_refs(pred, &mut refs);
    refs
}

fn collect_verdict_refs(pred: &Predicate, refs: &mut Vec<String>) {
    match pred {
        Predicate::VerdictPresent(id) => {
            if !refs.contains(id) {
                refs.push(id.clone());
            }
        }
        Predicate::And { left, right } | Predicate::Or { left, right } => {
            collect_verdict_refs(left, refs);
            collect_verdict_refs(right, refs);
        }
        Predicate::Compare { left, right, .. } => {
            collect_verdict_refs(left, refs);
            collect_verdict_refs(right, refs);
        }
        Predicate::Not { operand } => {
            collect_verdict_refs(operand, refs);
        }
        Predicate::Forall { domain, body, .. } | Predicate::Exists { domain, body, .. } => {
            collect_verdict_refs(domain, refs);
            collect_verdict_refs(body, refs);
        }
        Predicate::Mul { left, .. } => {
            collect_verdict_refs(left, refs);
        }
        Predicate::FactRef(_) | Predicate::FieldRef { .. } | Predicate::Literal { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operation::EntityStateMap;
    use crate::types::{Contract, TypeSpec, Value};

    fn bool_type() -> TypeSpec {
        TypeSpec {
            base: "Bool".to_string(),
            precision: None,
            scale: None,
            currency: None,
            min: None,
            max: None,
            max_length: None,
            values: None,
            fields: None,
            element_type: None,
            unit: None,
            variants: None,
        }
    }

    #[test]
    fn extract_single_verdict_ref() {
        let pred = Predicate::VerdictPresent("account_active".to_string());
        let refs = extract_verdict_refs(&pred);
        assert_eq!(refs, vec!["account_active"]);
    }

    #[test]
    fn extract_verdict_refs_from_and() {
        let pred = Predicate::And {
            left: Box::new(Predicate::VerdictPresent("v1".to_string())),
            right: Box::new(Predicate::VerdictPresent("v2".to_string())),
        };
        let refs = extract_verdict_refs(&pred);
        assert_eq!(refs, vec!["v1", "v2"]);
    }

    #[test]
    fn extract_verdict_refs_from_nested() {
        let pred = Predicate::And {
            left: Box::new(Predicate::VerdictPresent("v1".to_string())),
            right: Box::new(Predicate::Or {
                left: Box::new(Predicate::VerdictPresent("v2".to_string())),
                right: Box::new(Predicate::Compare {
                    left: Box::new(Predicate::FactRef("x".to_string())),
                    op: "=".to_string(),
                    right: Box::new(Predicate::Literal {
                        value: Value::Bool(true),
                        type_spec: bool_type(),
                    }),
                    comparison_type: None,
                }),
            }),
        };
        let refs = extract_verdict_refs(&pred);
        assert_eq!(refs, vec!["v1", "v2"]);
    }

    #[test]
    fn extract_verdict_refs_none() {
        let pred = Predicate::Compare {
            left: Box::new(Predicate::FactRef("x".to_string())),
            op: "=".to_string(),
            right: Box::new(Predicate::Literal {
                value: Value::Bool(true),
                type_spec: bool_type(),
            }),
            comparison_type: None,
        };
        let refs = extract_verdict_refs(&pred);
        assert!(refs.is_empty());
    }

    #[test]
    fn extract_verdict_refs_deduplicates() {
        let pred = Predicate::And {
            left: Box::new(Predicate::VerdictPresent("v1".to_string())),
            right: Box::new(Predicate::VerdictPresent("v1".to_string())),
        };
        let refs = extract_verdict_refs(&pred);
        assert_eq!(refs, vec!["v1"]);
    }

    #[test]
    fn extract_verdict_refs_from_not() {
        let pred = Predicate::Not {
            operand: Box::new(Predicate::VerdictPresent("v1".to_string())),
        };
        let refs = extract_verdict_refs(&pred);
        assert_eq!(refs, vec!["v1"]);
    }

    #[test]
    fn extract_verdict_refs_from_forall() {
        let pred = Predicate::Forall {
            variable: "x".to_string(),
            variable_type: bool_type(),
            domain: Box::new(Predicate::FactRef("items".to_string())),
            body: Box::new(Predicate::VerdictPresent("item_valid".to_string())),
        };
        let refs = extract_verdict_refs(&pred);
        assert_eq!(refs, vec!["item_valid"]);
    }

    // ── Test helpers for compute_action_space ──

    fn test_bundle() -> serde_json::Value {
        serde_json::json!({
            "id": "action_space_test",
            "kind": "Bundle",
            "tenor": "1.0",
            "tenor_version": "1.0.0",
            "constructs": [
                {
                    "id": "is_active",
                    "kind": "Fact",
                    "provenance": { "file": "test.tenor", "line": 1 },
                    "source": { "field": "active", "system": "account" },
                    "tenor": "1.0",
                    "type": { "base": "Bool" }
                },
                {
                    "id": "Order",
                    "initial": "pending",
                    "kind": "Entity",
                    "provenance": { "file": "test.tenor", "line": 3 },
                    "states": ["pending", "approved"],
                    "tenor": "1.0",
                    "transitions": [{ "from": "pending", "to": "approved" }]
                },
                {
                    "body": {
                        "produce": {
                            "payload": { "type": { "base": "Bool" }, "value": true },
                            "verdict_type": "account_active"
                        },
                        "when": {
                            "left": { "fact_ref": "is_active" },
                            "op": "=",
                            "right": { "literal": true, "type": { "base": "Bool" } }
                        }
                    },
                    "id": "check_active",
                    "kind": "Rule",
                    "provenance": { "file": "test.tenor", "line": 5 },
                    "stratum": 0,
                    "tenor": "1.0"
                },
                {
                    "allowed_personas": ["admin"],
                    "effects": [{ "entity_id": "Order", "from": "pending", "to": "approved" }],
                    "error_contract": ["precondition_failed"],
                    "id": "approve_order",
                    "kind": "Operation",
                    "precondition": { "verdict_present": "account_active" },
                    "provenance": { "file": "test.tenor", "line": 10 },
                    "tenor": "1.0"
                },
                {
                    "entry": "step_approve",
                    "id": "approval_flow",
                    "kind": "Flow",
                    "provenance": { "file": "test.tenor", "line": 15 },
                    "snapshot": "at_initiation",
                    "steps": [{
                        "id": "step_approve",
                        "kind": "OperationStep",
                        "on_failure": { "kind": "Terminate", "outcome": "approval_failed" },
                        "op": "approve_order",
                        "outcomes": {
                            "success": { "kind": "Terminal", "outcome": "order_approved" }
                        },
                        "persona": "admin"
                    }],
                    "tenor": "1.0"
                }
            ]
        })
    }

    fn make_contract(bundle: &serde_json::Value) -> Contract {
        Contract::from_interchange(bundle).unwrap()
    }

    // ── compute_action_space tests ──

    #[test]
    fn action_available_when_all_checks_pass() {
        let bundle = test_bundle();
        let contract = make_contract(&bundle);
        let facts = serde_json::json!({ "is_active": true });
        let entity_states = crate::operation::single_instance(
            [("Order".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        let result = compute_action_space(&contract, &facts, &entity_states, "admin").unwrap();

        assert_eq!(result.persona_id, "admin");
        assert_eq!(result.actions.len(), 1);
        assert_eq!(result.actions[0].flow_id, "approval_flow");
        assert_eq!(result.actions[0].entry_operation_id, "approve_order");
        assert_eq!(result.blocked_actions.len(), 0);

        // Instance bindings: Order -> {_default}
        let bindings = &result.actions[0].instance_bindings;
        assert_eq!(bindings.len(), 1);
        assert!(bindings.contains_key("Order"));
        assert!(bindings["Order"].contains(DEFAULT_INSTANCE_ID));
    }

    #[test]
    fn blocked_when_persona_not_authorized() {
        let bundle = test_bundle();
        let contract = make_contract(&bundle);
        let facts = serde_json::json!({ "is_active": true });
        let entity_states = crate::operation::single_instance(
            [("Order".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        let result = compute_action_space(&contract, &facts, &entity_states, "guest").unwrap();

        assert_eq!(result.actions.len(), 0);
        assert_eq!(result.blocked_actions.len(), 1);
        assert_eq!(result.blocked_actions[0].flow_id, "approval_flow");
        assert!(matches!(
            &result.blocked_actions[0].reason,
            BlockedReason::PersonaNotAuthorized
        ));
        // No instance bindings for persona-blocked actions
        assert!(result.blocked_actions[0].instance_bindings.is_empty());
    }

    #[test]
    fn blocked_when_precondition_not_met() {
        let bundle = test_bundle();
        let contract = make_contract(&bundle);
        let facts = serde_json::json!({ "is_active": false });
        let entity_states = crate::operation::single_instance(
            [("Order".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        let result = compute_action_space(&contract, &facts, &entity_states, "admin").unwrap();

        assert_eq!(result.actions.len(), 0);
        assert_eq!(result.blocked_actions.len(), 1);
        assert_eq!(result.blocked_actions[0].flow_id, "approval_flow");
        match &result.blocked_actions[0].reason {
            BlockedReason::PreconditionNotMet { missing_verdicts } => {
                assert_eq!(missing_verdicts, &vec!["account_active".to_string()]);
            }
            other => panic!("expected PreconditionNotMet, got {:?}", other),
        }
    }

    #[test]
    fn blocked_when_entity_not_in_source_state() {
        let bundle = test_bundle();
        let contract = make_contract(&bundle);
        let facts = serde_json::json!({ "is_active": true });
        let entity_states = crate::operation::single_instance(
            [("Order".to_string(), "approved".to_string())]
                .into_iter()
                .collect(),
        );

        let result = compute_action_space(&contract, &facts, &entity_states, "admin").unwrap();

        assert_eq!(result.actions.len(), 0);
        assert_eq!(result.blocked_actions.len(), 1);
        match &result.blocked_actions[0].reason {
            BlockedReason::EntityNotInSourceState {
                entity_id,
                current_state,
                required_state,
            } => {
                assert_eq!(entity_id, "Order");
                assert_eq!(current_state, "approved");
                assert_eq!(required_state, "pending");
            }
            other => panic!("expected EntityNotInSourceState, got {:?}", other),
        }
    }

    #[test]
    fn current_verdicts_populated() {
        let bundle = test_bundle();
        let contract = make_contract(&bundle);
        let facts = serde_json::json!({ "is_active": true });
        let entity_states = EntityStateMap::new();

        let result = compute_action_space(&contract, &facts, &entity_states, "admin").unwrap();

        assert_eq!(result.current_verdicts.len(), 1);
        assert_eq!(result.current_verdicts[0].verdict_type, "account_active");
        assert_eq!(result.current_verdicts[0].producing_rule, "check_active");
        assert_eq!(result.current_verdicts[0].stratum, 0);
    }

    #[test]
    fn enabling_verdicts_precise() {
        let bundle = test_bundle();
        let contract = make_contract(&bundle);
        let facts = serde_json::json!({ "is_active": true });
        let entity_states = crate::operation::single_instance(
            [("Order".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        let result = compute_action_space(&contract, &facts, &entity_states, "admin").unwrap();

        assert_eq!(result.actions[0].enabling_verdicts.len(), 1);
        assert_eq!(
            result.actions[0].enabling_verdicts[0].verdict_type,
            "account_active"
        );
    }

    #[test]
    fn affected_entities_populated() {
        let bundle = test_bundle();
        let contract = make_contract(&bundle);
        let facts = serde_json::json!({ "is_active": true });
        let entity_states = crate::operation::single_instance(
            [("Order".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        let result = compute_action_space(&contract, &facts, &entity_states, "admin").unwrap();

        assert_eq!(result.actions[0].affected_entities.len(), 1);
        assert_eq!(result.actions[0].affected_entities[0].entity_id, "Order");
        assert_eq!(
            result.actions[0].affected_entities[0].current_state,
            "pending"
        );
        assert_eq!(
            result.actions[0].affected_entities[0].possible_transitions,
            vec!["approved"]
        );
    }

    #[test]
    fn description_generated() {
        let bundle = test_bundle();
        let contract = make_contract(&bundle);
        let facts = serde_json::json!({ "is_active": true });
        let entity_states = crate::operation::single_instance(
            [("Order".to_string(), "pending".to_string())]
                .into_iter()
                .collect(),
        );

        let result = compute_action_space(&contract, &facts, &entity_states, "admin").unwrap();

        assert!(result.actions[0].description.contains("approval_flow"));
        assert!(result.actions[0].description.contains("approve_order"));
    }

    #[test]
    fn empty_facts_causes_error() {
        let bundle = test_bundle();
        let contract = make_contract(&bundle);
        let facts = serde_json::json!({});
        let entity_states = EntityStateMap::new();

        // is_active has no default, so assemble_facts should fail
        let result = compute_action_space(&contract, &facts, &entity_states, "admin");
        assert!(result.is_err());
    }

    // ── Per-instance action space tests ──

    #[test]
    fn multi_instance_action_shows_valid_instances() {
        // Two Order instances: order-1 in pending (valid), order-2 in approved (blocked).
        // Action should be available with instance_bindings["Order"] = {"order-1"}.
        let bundle = test_bundle();
        let contract = make_contract(&bundle);
        let facts = serde_json::json!({ "is_active": true });

        let mut entity_states = EntityStateMap::new();
        entity_states.insert(
            ("Order".to_string(), "order-1".to_string()),
            "pending".to_string(),
        );
        entity_states.insert(
            ("Order".to_string(), "order-2".to_string()),
            "approved".to_string(),
        );

        let result = compute_action_space(&contract, &facts, &entity_states, "admin").unwrap();

        // Action should be available because order-1 is in "pending"
        assert_eq!(result.actions.len(), 1, "action should be available");
        let bindings = &result.actions[0].instance_bindings;
        assert!(
            bindings.contains_key("Order"),
            "Order should be in instance_bindings"
        );
        assert!(
            bindings["Order"].contains("order-1"),
            "order-1 should be valid"
        );
        assert!(
            !bindings["Order"].contains("order-2"),
            "order-2 should not be valid"
        );
    }

    #[test]
    fn multi_instance_blocked_when_no_valid_instance() {
        // Two Order instances: both in "approved" (not "pending"). Action should be blocked.
        let bundle = test_bundle();
        let contract = make_contract(&bundle);
        let facts = serde_json::json!({ "is_active": true });

        let mut entity_states = EntityStateMap::new();
        entity_states.insert(
            ("Order".to_string(), "order-1".to_string()),
            "approved".to_string(),
        );
        entity_states.insert(
            ("Order".to_string(), "order-2".to_string()),
            "approved".to_string(),
        );

        let result = compute_action_space(&contract, &facts, &entity_states, "admin").unwrap();

        assert_eq!(
            result.actions.len(),
            0,
            "no valid actions when all instances wrong state"
        );
        assert_eq!(result.blocked_actions.len(), 1);
        match &result.blocked_actions[0].reason {
            BlockedReason::EntityNotInSourceState {
                entity_id,
                required_state,
                ..
            } => {
                assert_eq!(entity_id, "Order");
                assert_eq!(required_state, "pending");
            }
            other => panic!("expected EntityNotInSourceState, got {:?}", other),
        }
    }

    #[test]
    fn multi_instance_all_instances_valid() {
        // Two Order instances: both in "pending". Both should appear in instance_bindings.
        let bundle = test_bundle();
        let contract = make_contract(&bundle);
        let facts = serde_json::json!({ "is_active": true });

        let mut entity_states = EntityStateMap::new();
        entity_states.insert(
            ("Order".to_string(), "order-1".to_string()),
            "pending".to_string(),
        );
        entity_states.insert(
            ("Order".to_string(), "order-2".to_string()),
            "pending".to_string(),
        );

        let result = compute_action_space(&contract, &facts, &entity_states, "admin").unwrap();

        assert_eq!(result.actions.len(), 1, "action available");
        let bindings = &result.actions[0].instance_bindings;
        assert_eq!(bindings["Order"].len(), 2, "both instances valid");
        assert!(bindings["Order"].contains("order-1"));
        assert!(bindings["Order"].contains("order-2"));
    }
}
