//! Three-layer flow compatibility checker per spec Section 18.6.
//!
//! Checks whether in-flight flows can safely continue after a contract
//! version change. Decomposes into three analysis layers that short-circuit
//! on first failure:
//!
//! - Layer 1: Verdict isolation (always passes -- frozen snapshot is immutable)
//! - Layer 2: Entity state equivalence (FMC3)
//! - Layer 3: Operation/flow structure (FMC1 + FMC2 minus verdicts)

use std::collections::HashSet;

use crate::operation::EntityStateMap;
use crate::types::{Contract, Entity, Flow, FlowStep, StepTarget};

use super::plan::{FlowCompatibilityResult, IncompatibilityReason, LayerResults};

/// Check flow compatibility between v1 and v2 contracts at a given position.
///
/// The three layers are evaluated in order with short-circuit on failure:
/// - Layer 1: Verdict isolation -- always passes (frozen snapshot semantics)
/// - Layer 2: Entity state equivalence -- checks entity states against v2
/// - Layer 3: Structure -- checks steps, operations, personas against v2
pub fn check_flow_compatibility(
    v1: &Contract,
    v2: &Contract,
    flow_id: &str,
    position: &str,
    entity_states: &EntityStateMap,
) -> FlowCompatibilityResult {
    let _ = (v1, v2, flow_id, position, entity_states);
    todo!("Implement in GREEN phase")
}

/// Static entry-point check: checks from entry point without entity states.
///
/// Used for pre-migration analysis where we don't know per-instance entity states.
/// Uses the flow's entry point as position and default initial states from v1.
pub fn check_flow_compatibility_static(
    v1: &Contract,
    v2: &Contract,
    flow_id: &str,
) -> FlowCompatibilityResult {
    let _ = (v1, v2, flow_id);
    todo!("Implement in GREEN phase")
}

/// Compute all step IDs reachable from the given position via DFS through
/// the flow's step graph.
///
/// Follows all outcome edges: OperationStep outcomes, BranchStep if_true/if_false,
/// SubFlowStep on_success, ParallelStep branches. The flow DAG is acyclic per
/// spec Section 11.5.
fn compute_reachable_steps(_flow: &Flow, _position: &str) -> Vec<String> {
    todo!("Implement in GREEN phase")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Contract, Effect, Entity, FactDecl, Flow, FlowStep, Operation, Predicate, StepTarget,
        Transition, TypeSpec, Value,
    };
    use std::collections::BTreeMap;

    // ────────────────────────────────────────────
    // Test helpers
    // ────────────────────────────────────────────

    fn make_type_spec() -> TypeSpec {
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

    fn make_fact(id: &str) -> FactDecl {
        FactDecl {
            id: id.to_string(),
            fact_type: make_type_spec(),
            default: None,
        }
    }

    fn make_entity(
        id: &str,
        states: &[&str],
        initial: &str,
        transitions: &[(&str, &str)],
    ) -> Entity {
        Entity {
            id: id.to_string(),
            states: states.iter().map(|s| s.to_string()).collect(),
            initial: initial.to_string(),
            transitions: transitions
                .iter()
                .map(|(from, to)| Transition {
                    from: from.to_string(),
                    to: to.to_string(),
                })
                .collect(),
        }
    }

    fn make_operation(
        id: &str,
        personas: &[&str],
        effects: Vec<Effect>,
        outcomes: &[&str],
    ) -> Operation {
        Operation {
            id: id.to_string(),
            allowed_personas: personas.iter().map(|s| s.to_string()).collect(),
            precondition: Predicate::Literal {
                value: Value::Bool(true),
                type_spec: make_type_spec(),
            },
            effects,
            error_contract: vec![],
            outcomes: outcomes.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn make_effect(entity_id: &str, from: &str, to: &str) -> Effect {
        Effect {
            entity_id: entity_id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            outcome: None,
        }
    }

    fn make_operation_step(
        id: &str,
        op: &str,
        persona: &str,
        outcomes: Vec<(&str, StepTarget)>,
    ) -> FlowStep {
        let mut outcome_map = BTreeMap::new();
        for (k, v) in outcomes {
            outcome_map.insert(k.to_string(), v);
        }
        FlowStep::OperationStep {
            id: id.to_string(),
            op: op.to_string(),
            persona: persona.to_string(),
            outcomes: outcome_map,
            on_failure: crate::types::FailureHandler::Terminate {
                outcome: "failed".to_string(),
            },
        }
    }

    fn make_flow(id: &str, entry: &str, steps: Vec<FlowStep>) -> Flow {
        Flow {
            id: id.to_string(),
            snapshot: "initiation".to_string(),
            entry: entry.to_string(),
            steps,
        }
    }

    fn make_contract(
        entities: Vec<Entity>,
        operations: Vec<Operation>,
        flows: Vec<Flow>,
    ) -> Contract {
        Contract::new(
            vec![make_fact("dummy_fact")],
            entities,
            vec![],
            operations,
            flows,
            vec!["admin".to_string(), "user".to_string()],
        )
    }

    // ────────────────────────────────────────────
    // Layer 1: Verdict isolation (always passes)
    // ────────────────────────────────────────────

    #[test]
    fn layer1_always_passes() {
        // Layer 1 (verdict isolation) must always return true due to
        // frozen snapshot semantics (Section 15).
        let entity = make_entity(
            "Order",
            &["draft", "submitted"],
            "draft",
            &[("draft", "submitted")],
        );
        let op = make_operation(
            "submit",
            &["user"],
            vec![make_effect("Order", "draft", "submitted")],
            &["success"],
        );
        let flow = make_flow(
            "order_flow",
            "step1",
            vec![make_operation_step(
                "step1",
                "submit",
                "user",
                vec![(
                    "success",
                    StepTarget::Terminal {
                        outcome: "done".to_string(),
                    },
                )],
            )],
        );

        let v1 = make_contract(vec![entity.clone()], vec![op.clone()], vec![flow.clone()]);
        let v2 = make_contract(vec![entity], vec![op], vec![flow]);

        let mut entity_states = EntityStateMap::new();
        entity_states.insert("Order".to_string(), "draft".to_string());

        let result = check_flow_compatibility(&v1, &v2, "order_flow", "step1", &entity_states);

        assert!(result.layer_results.layer1_verdict_isolation);
    }

    // ────────────────────────────────────────────
    // Layer 2: Entity state equivalence
    // ────────────────────────────────────────────

    #[test]
    fn layer2_passes_when_all_states_exist_in_v2() {
        let v1_entity = make_entity(
            "Order",
            &["draft", "submitted", "approved"],
            "draft",
            &[("draft", "submitted"), ("submitted", "approved")],
        );
        let v2_entity = make_entity(
            "Order",
            &["draft", "submitted", "approved"],
            "draft",
            &[("draft", "submitted"), ("submitted", "approved")],
        );
        let op = make_operation(
            "submit",
            &["user"],
            vec![make_effect("Order", "draft", "submitted")],
            &["success"],
        );
        let flow = make_flow(
            "order_flow",
            "step1",
            vec![make_operation_step(
                "step1",
                "submit",
                "user",
                vec![(
                    "success",
                    StepTarget::Terminal {
                        outcome: "done".to_string(),
                    },
                )],
            )],
        );

        let v1 = make_contract(vec![v1_entity], vec![op.clone()], vec![flow.clone()]);
        let v2 = make_contract(vec![v2_entity], vec![op], vec![flow]);

        let mut entity_states = EntityStateMap::new();
        entity_states.insert("Order".to_string(), "draft".to_string());

        let result = check_flow_compatibility(&v1, &v2, "order_flow", "step1", &entity_states);

        assert!(result.layer_results.layer2_entity_state);
        assert!(result.compatible);
    }

    #[test]
    fn layer2_fails_when_entity_in_removed_state() {
        // V2 removed "pending_review" state. Entity currently in "pending_review" -> incompatible.
        let v1_entity = make_entity(
            "Order",
            &["draft", "pending_review", "approved"],
            "draft",
            &[("draft", "pending_review"), ("pending_review", "approved")],
        );
        let v2_entity = make_entity(
            "Order",
            &["draft", "approved"],
            "draft",
            &[("draft", "approved")],
        );
        let op = make_operation(
            "review",
            &["user"],
            vec![make_effect("Order", "pending_review", "approved")],
            &["success"],
        );
        let flow = make_flow(
            "review_flow",
            "step1",
            vec![make_operation_step(
                "step1",
                "review",
                "user",
                vec![(
                    "success",
                    StepTarget::Terminal {
                        outcome: "done".to_string(),
                    },
                )],
            )],
        );

        let v1 = make_contract(vec![v1_entity], vec![op.clone()], vec![flow.clone()]);
        let v2 = make_contract(vec![v2_entity], vec![op], vec![flow]);

        let mut entity_states = EntityStateMap::new();
        entity_states.insert("Order".to_string(), "pending_review".to_string());

        let result = check_flow_compatibility(&v1, &v2, "review_flow", "step1", &entity_states);

        assert!(!result.layer_results.layer2_entity_state);
        assert!(!result.compatible);
        assert!(result.reasons.iter().any(|r| matches!(
            r,
            IncompatibilityReason::EntityStateNotInV2 { entity_id, state }
            if entity_id == "Order" && state == "pending_review"
        )));
    }

    // ────────────────────────────────────────────
    // Layer 3: Operation/flow structure
    // ────────────────────────────────────────────

    #[test]
    fn layer3_fails_when_step_missing_in_v2() {
        let entity = make_entity(
            "Order",
            &["draft", "submitted"],
            "draft",
            &[("draft", "submitted")],
        );
        let op = make_operation(
            "submit",
            &["user"],
            vec![make_effect("Order", "draft", "submitted")],
            &["success"],
        );

        let v1_flow = make_flow(
            "order_flow",
            "step1",
            vec![
                make_operation_step(
                    "step1",
                    "submit",
                    "user",
                    vec![("success", StepTarget::StepRef("step2".to_string()))],
                ),
                make_operation_step(
                    "step2",
                    "submit",
                    "user",
                    vec![(
                        "success",
                        StepTarget::Terminal {
                            outcome: "done".to_string(),
                        },
                    )],
                ),
            ],
        );
        // V2 flow only has step1 -- step2 is missing
        let v2_flow = make_flow(
            "order_flow",
            "step1",
            vec![make_operation_step(
                "step1",
                "submit",
                "user",
                vec![(
                    "success",
                    StepTarget::Terminal {
                        outcome: "done".to_string(),
                    },
                )],
            )],
        );

        let v1 = make_contract(vec![entity.clone()], vec![op.clone()], vec![v1_flow]);
        let v2 = make_contract(vec![entity], vec![op], vec![v2_flow]);

        let mut entity_states = EntityStateMap::new();
        entity_states.insert("Order".to_string(), "draft".to_string());

        let result = check_flow_compatibility(&v1, &v2, "order_flow", "step1", &entity_states);

        assert!(!result.layer_results.layer3_structure);
        assert!(!result.compatible);
        assert!(result.reasons.iter().any(|r| matches!(
            r,
            IncompatibilityReason::StepNotInV2 { step_id } if step_id == "step2"
        )));
    }

    #[test]
    fn layer3_fails_when_operation_changed_in_v2() {
        let entity = make_entity(
            "Order",
            &["draft", "submitted"],
            "draft",
            &[("draft", "submitted")],
        );
        let v1_op = make_operation(
            "submit",
            &["user"],
            vec![make_effect("Order", "draft", "submitted")],
            &["success"],
        );
        let v2_op = make_operation(
            "submit_v2",
            &["user"],
            vec![make_effect("Order", "draft", "submitted")],
            &["success"],
        );

        let v1_flow = make_flow(
            "order_flow",
            "step1",
            vec![make_operation_step(
                "step1",
                "submit",
                "user",
                vec![(
                    "success",
                    StepTarget::Terminal {
                        outcome: "done".to_string(),
                    },
                )],
            )],
        );
        // V2 step references "submit_v2" instead of "submit"
        let v2_flow = make_flow(
            "order_flow",
            "step1",
            vec![make_operation_step(
                "step1",
                "submit_v2",
                "user",
                vec![(
                    "success",
                    StepTarget::Terminal {
                        outcome: "done".to_string(),
                    },
                )],
            )],
        );

        let v1 = make_contract(vec![entity.clone()], vec![v1_op], vec![v1_flow]);
        let v2 = make_contract(vec![entity], vec![v2_op], vec![v2_flow]);

        let mut entity_states = EntityStateMap::new();
        entity_states.insert("Order".to_string(), "draft".to_string());

        let result = check_flow_compatibility(&v1, &v2, "order_flow", "step1", &entity_states);

        assert!(!result.layer_results.layer3_structure);
        assert!(!result.compatible);
        assert!(result.reasons.iter().any(|r| matches!(
            r,
            IncompatibilityReason::OperationChangedInV2 { operation_id } if operation_id == "submit"
        )));
    }

    #[test]
    fn layer3_fails_when_persona_not_authorized_in_v2() {
        let entity = make_entity(
            "Order",
            &["draft", "submitted"],
            "draft",
            &[("draft", "submitted")],
        );
        let v1_op = make_operation(
            "submit",
            &["user", "admin"],
            vec![make_effect("Order", "draft", "submitted")],
            &["success"],
        );
        // V2 removes "user" from allowed personas
        let v2_op = make_operation(
            "submit",
            &["admin"],
            vec![make_effect("Order", "draft", "submitted")],
            &["success"],
        );

        // Flow step uses persona "user" -- which v2's operation no longer allows
        let flow = make_flow(
            "order_flow",
            "step1",
            vec![make_operation_step(
                "step1",
                "submit",
                "user",
                vec![(
                    "success",
                    StepTarget::Terminal {
                        outcome: "done".to_string(),
                    },
                )],
            )],
        );

        let v1 = make_contract(vec![entity.clone()], vec![v1_op], vec![flow.clone()]);
        let v2 = make_contract(vec![entity], vec![v2_op], vec![flow]);

        let mut entity_states = EntityStateMap::new();
        entity_states.insert("Order".to_string(), "draft".to_string());

        let result = check_flow_compatibility(&v1, &v2, "order_flow", "step1", &entity_states);

        assert!(!result.layer_results.layer3_structure);
        assert!(!result.compatible);
        assert!(result.reasons.iter().any(|r| matches!(
            r,
            IncompatibilityReason::PersonaNotAuthorized { step_id, persona }
            if step_id == "step1" && persona == "user"
        )));
    }

    // ────────────────────────────────────────────
    // Short-circuit behavior
    // ────────────────────────────────────────────

    #[test]
    fn layer2_failure_short_circuits_layer3() {
        // When Layer 2 fails, Layer 3 should not run (set to false in results).
        let v1_entity = make_entity(
            "Order",
            &["draft", "pending_review", "approved"],
            "draft",
            &[("draft", "pending_review"), ("pending_review", "approved")],
        );
        let v2_entity = make_entity(
            "Order",
            &["draft", "approved"],
            "draft",
            &[("draft", "approved")],
        );
        let op = make_operation(
            "review",
            &["user"],
            vec![make_effect("Order", "pending_review", "approved")],
            &["success"],
        );
        let flow = make_flow(
            "review_flow",
            "step1",
            vec![make_operation_step(
                "step1",
                "review",
                "user",
                vec![(
                    "success",
                    StepTarget::Terminal {
                        outcome: "done".to_string(),
                    },
                )],
            )],
        );

        let v1 = make_contract(vec![v1_entity], vec![op.clone()], vec![flow.clone()]);
        let v2 = make_contract(vec![v2_entity], vec![op], vec![flow]);

        let mut entity_states = EntityStateMap::new();
        entity_states.insert("Order".to_string(), "pending_review".to_string());

        let result = check_flow_compatibility(&v1, &v2, "review_flow", "step1", &entity_states);

        assert!(result.layer_results.layer1_verdict_isolation);
        assert!(!result.layer_results.layer2_entity_state);
        assert!(!result.layer_results.layer3_structure); // short-circuited
        assert!(!result.compatible);
    }

    // ────────────────────────────────────────────
    // Identical contracts
    // ────────────────────────────────────────────

    #[test]
    fn identical_contracts_are_compatible() {
        let entity = make_entity(
            "Order",
            &["draft", "submitted", "approved"],
            "draft",
            &[("draft", "submitted"), ("submitted", "approved")],
        );
        let op = make_operation(
            "submit",
            &["user"],
            vec![make_effect("Order", "draft", "submitted")],
            &["success"],
        );
        let flow = make_flow(
            "order_flow",
            "step1",
            vec![make_operation_step(
                "step1",
                "submit",
                "user",
                vec![(
                    "success",
                    StepTarget::Terminal {
                        outcome: "done".to_string(),
                    },
                )],
            )],
        );

        let v1 = make_contract(vec![entity.clone()], vec![op.clone()], vec![flow.clone()]);
        let v2 = make_contract(vec![entity], vec![op], vec![flow]);

        let mut entity_states = EntityStateMap::new();
        entity_states.insert("Order".to_string(), "draft".to_string());

        let result = check_flow_compatibility(&v1, &v2, "order_flow", "step1", &entity_states);

        assert!(result.compatible);
        assert!(result.layer_results.layer1_verdict_isolation);
        assert!(result.layer_results.layer2_entity_state);
        assert!(result.layer_results.layer3_structure);
        assert!(result.reasons.is_empty());
    }

    // ────────────────────────────────────────────
    // Position sensitivity
    // ────────────────────────────────────────────

    #[test]
    fn position_sensitivity_same_flow_different_results() {
        // Same flow pair returns different results at different positions.
        // V2 removes step2's operation. At step1, step2 is reachable -> incompatible.
        // At step3 (past step2), step2 is NOT reachable -> compatible.
        let entity = make_entity(
            "Order",
            &["draft", "submitted", "approved"],
            "draft",
            &[("draft", "submitted"), ("submitted", "approved")],
        );
        let submit_op = make_operation(
            "submit",
            &["user"],
            vec![make_effect("Order", "draft", "submitted")],
            &["success"],
        );
        let approve_op = make_operation(
            "approve",
            &["admin"],
            vec![make_effect("Order", "submitted", "approved")],
            &["success"],
        );
        let finalize_op = make_operation("finalize", &["admin"], vec![], &["success"]);

        let v1_flow = make_flow(
            "order_flow",
            "step1",
            vec![
                make_operation_step(
                    "step1",
                    "submit",
                    "user",
                    vec![("success", StepTarget::StepRef("step2".to_string()))],
                ),
                make_operation_step(
                    "step2",
                    "approve",
                    "admin",
                    vec![("success", StepTarget::StepRef("step3".to_string()))],
                ),
                make_operation_step(
                    "step3",
                    "finalize",
                    "admin",
                    vec![(
                        "success",
                        StepTarget::Terminal {
                            outcome: "done".to_string(),
                        },
                    )],
                ),
            ],
        );

        // V2 replaces step2's operation with a different one
        let approve_v2_op = make_operation(
            "approve_v2",
            &["admin"],
            vec![make_effect("Order", "submitted", "approved")],
            &["success"],
        );
        let v2_flow = make_flow(
            "order_flow",
            "step1",
            vec![
                make_operation_step(
                    "step1",
                    "submit",
                    "user",
                    vec![("success", StepTarget::StepRef("step2".to_string()))],
                ),
                make_operation_step(
                    "step2",
                    "approve_v2",
                    "admin",
                    vec![("success", StepTarget::StepRef("step3".to_string()))],
                ),
                make_operation_step(
                    "step3",
                    "finalize",
                    "admin",
                    vec![(
                        "success",
                        StepTarget::Terminal {
                            outcome: "done".to_string(),
                        },
                    )],
                ),
            ],
        );

        let v1 = make_contract(
            vec![entity.clone()],
            vec![submit_op.clone(), approve_op, finalize_op.clone()],
            vec![v1_flow],
        );
        let v2 = make_contract(
            vec![entity],
            vec![submit_op, approve_v2_op, finalize_op],
            vec![v2_flow],
        );

        // At step1: step2 is reachable, step2 uses "approve" but v2 uses "approve_v2" -> incompatible
        let mut entity_states = EntityStateMap::new();
        entity_states.insert("Order".to_string(), "draft".to_string());
        let result_at_step1 =
            check_flow_compatibility(&v1, &v2, "order_flow", "step1", &entity_states);

        // At step3: step2 is NOT reachable (already past it) -> compatible
        let mut entity_states_at_step3 = EntityStateMap::new();
        entity_states_at_step3.insert("Order".to_string(), "approved".to_string());
        let result_at_step3 =
            check_flow_compatibility(&v1, &v2, "order_flow", "step3", &entity_states_at_step3);

        assert!(
            !result_at_step1.compatible,
            "At step1, step2 is reachable with changed operation -- should be incompatible"
        );
        assert!(
            result_at_step3.compatible,
            "At step3, step2 is not reachable -- should be compatible"
        );
    }

    // ────────────────────────────────────────────
    // V2 adds states/transitions (compatible)
    // ────────────────────────────────────────────

    #[test]
    fn v2_adds_new_states_is_compatible() {
        let v1_entity = make_entity(
            "Order",
            &["draft", "submitted"],
            "draft",
            &[("draft", "submitted")],
        );
        // V2 adds "approved" state and transition
        let v2_entity = make_entity(
            "Order",
            &["draft", "submitted", "approved"],
            "draft",
            &[("draft", "submitted"), ("submitted", "approved")],
        );
        let op = make_operation(
            "submit",
            &["user"],
            vec![make_effect("Order", "draft", "submitted")],
            &["success"],
        );
        let flow = make_flow(
            "order_flow",
            "step1",
            vec![make_operation_step(
                "step1",
                "submit",
                "user",
                vec![(
                    "success",
                    StepTarget::Terminal {
                        outcome: "done".to_string(),
                    },
                )],
            )],
        );

        let v1 = make_contract(vec![v1_entity], vec![op.clone()], vec![flow.clone()]);
        let v2 = make_contract(vec![v2_entity], vec![op], vec![flow]);

        let mut entity_states = EntityStateMap::new();
        entity_states.insert("Order".to_string(), "draft".to_string());

        let result = check_flow_compatibility(&v1, &v2, "order_flow", "step1", &entity_states);

        assert!(result.compatible);
        assert!(result.reasons.is_empty());
    }

    // ────────────────────────────────────────────
    // Static entry-point check
    // ────────────────────────────────────────────

    #[test]
    fn static_check_uses_entry_point_and_initial_states() {
        let entity = make_entity(
            "Order",
            &["draft", "submitted"],
            "draft",
            &[("draft", "submitted")],
        );
        let op = make_operation(
            "submit",
            &["user"],
            vec![make_effect("Order", "draft", "submitted")],
            &["success"],
        );
        let flow = make_flow(
            "order_flow",
            "step1",
            vec![make_operation_step(
                "step1",
                "submit",
                "user",
                vec![(
                    "success",
                    StepTarget::Terminal {
                        outcome: "done".to_string(),
                    },
                )],
            )],
        );

        let v1 = make_contract(vec![entity.clone()], vec![op.clone()], vec![flow.clone()]);
        let v2 = make_contract(vec![entity], vec![op], vec![flow]);

        let result = check_flow_compatibility_static(&v1, &v2, "order_flow");

        assert!(result.compatible);
        assert_eq!(result.position, Some("step1".to_string())); // entry point
    }

    // ────────────────────────────────────────────
    // Multi-step reachability
    // ────────────────────────────────────────────

    #[test]
    fn multi_step_flow_checks_all_reachable_steps() {
        let entity = make_entity(
            "Order",
            &["draft", "submitted", "approved"],
            "draft",
            &[("draft", "submitted"), ("submitted", "approved")],
        );
        let submit_op = make_operation(
            "submit",
            &["user"],
            vec![make_effect("Order", "draft", "submitted")],
            &["success"],
        );
        let approve_op = make_operation(
            "approve",
            &["admin"],
            vec![make_effect("Order", "submitted", "approved")],
            &["success"],
        );

        let flow = make_flow(
            "order_flow",
            "step1",
            vec![
                make_operation_step(
                    "step1",
                    "submit",
                    "user",
                    vec![("success", StepTarget::StepRef("step2".to_string()))],
                ),
                make_operation_step(
                    "step2",
                    "approve",
                    "admin",
                    vec![(
                        "success",
                        StepTarget::Terminal {
                            outcome: "done".to_string(),
                        },
                    )],
                ),
            ],
        );

        let v1 = make_contract(
            vec![entity.clone()],
            vec![submit_op.clone(), approve_op.clone()],
            vec![flow.clone()],
        );
        let v2 = make_contract(vec![entity], vec![submit_op, approve_op], vec![flow]);

        let mut entity_states = EntityStateMap::new();
        entity_states.insert("Order".to_string(), "draft".to_string());

        let result = check_flow_compatibility(&v1, &v2, "order_flow", "step1", &entity_states);

        assert!(result.compatible);
    }
}
