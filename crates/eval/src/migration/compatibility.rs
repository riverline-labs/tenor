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
use crate::types::{Contract, Flow, FlowStep, StepTarget};

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
    // Look up the flow in v1's contract
    let v1_flow = match find_flow(v1, flow_id) {
        Some(f) => f,
        None => {
            return FlowCompatibilityResult {
                flow_id: flow_id.to_string(),
                position: Some(position.to_string()),
                compatible: false,
                layer_results: LayerResults {
                    layer1_verdict_isolation: false,
                    layer2_entity_state: false,
                    layer3_structure: false,
                },
                reasons: vec![],
            };
        }
    };

    let v2_flow = find_flow(v2, flow_id);

    // Compute reachable steps from position in v1's flow graph
    let reachable = compute_reachable_steps(v1_flow, position);

    // Layer 1: Verdict isolation -- always passes.
    // Frozen snapshot is immutable per Section 15. No test needed for failure,
    // but the layer must be present in code for auditability.
    let layer1 = true;

    // Layer 2: Entity state equivalence (FMC3)
    let (layer2, mut reasons) = check_layer2_entity_states(v1, v2, &reachable, entity_states);

    // Layer 3: Structure (FMC1 + FMC2 minus verdicts) -- short-circuit if Layer 2 fails
    let layer3 = if layer2 {
        let (l3_pass, l3_reasons) = check_layer3_structure(v1, v2, v1_flow, v2_flow, &reachable);
        reasons.extend(l3_reasons);
        l3_pass
    } else {
        false
    };

    let compatible = layer1 && layer2 && layer3;

    FlowCompatibilityResult {
        flow_id: flow_id.to_string(),
        position: Some(position.to_string()),
        compatible,
        layer_results: LayerResults {
            layer1_verdict_isolation: layer1,
            layer2_entity_state: layer2,
            layer3_structure: layer3,
        },
        reasons,
    }
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
    // Get the flow's entry point from v1
    let entry = match find_flow(v1, flow_id) {
        Some(f) => f.entry.clone(),
        None => {
            return FlowCompatibilityResult {
                flow_id: flow_id.to_string(),
                position: None,
                compatible: false,
                layer_results: LayerResults {
                    layer1_verdict_isolation: false,
                    layer2_entity_state: false,
                    layer3_structure: false,
                },
                reasons: vec![],
            };
        }
    };

    // Build default entity states from v1's entity initial states
    let mut entity_states = EntityStateMap::new();
    for entity in &v1.entities {
        entity_states.insert(entity.id.clone(), entity.initial.clone());
    }

    check_flow_compatibility(v1, v2, flow_id, &entry, &entity_states)
}

/// Find a flow by ID in a contract.
fn find_flow<'a>(contract: &'a Contract, flow_id: &str) -> Option<&'a Flow> {
    contract
        .flow_index
        .get(flow_id)
        .map(|&idx| &contract.flows[idx])
}

/// Compute all step IDs reachable from the given position via DFS through
/// the flow's step graph.
///
/// Follows all outcome edges: OperationStep outcomes, BranchStep if_true/if_false,
/// SubFlowStep on_success, ParallelStep branches. The flow DAG is acyclic per
/// spec Section 11.5.
fn compute_reachable_steps(flow: &Flow, position: &str) -> Vec<String> {
    let mut visited = HashSet::new();
    let mut result = Vec::new();
    let mut stack = vec![position.to_string()];

    while let Some(step_id) = stack.pop() {
        if !visited.insert(step_id.clone()) {
            continue;
        }

        // Find this step in the flow
        let step = flow.steps.iter().find(|s| step_id_of(s) == step_id);
        let step = match step {
            Some(s) => s,
            None => continue,
        };

        result.push(step_id.clone());

        // Follow all outgoing edges
        match step {
            FlowStep::OperationStep { outcomes, .. } => {
                for target in outcomes.values() {
                    if let StepTarget::StepRef(next) = target {
                        stack.push(next.clone());
                    }
                }
            }
            FlowStep::BranchStep {
                if_true, if_false, ..
            } => {
                if let StepTarget::StepRef(next) = if_true {
                    stack.push(next.clone());
                }
                if let StepTarget::StepRef(next) = if_false {
                    stack.push(next.clone());
                }
            }
            FlowStep::HandoffStep { next, .. } => {
                stack.push(next.clone());
            }
            FlowStep::SubFlowStep { on_success, .. } => {
                if let StepTarget::StepRef(next) = on_success {
                    stack.push(next.clone());
                }
            }
            FlowStep::ParallelStep { branches, .. } => {
                for branch in branches {
                    stack.push(branch.entry.clone());
                    // Also traverse steps within branches
                    for branch_step in &branch.steps {
                        let bid = step_id_of(branch_step);
                        stack.push(bid);
                    }
                }
            }
        }
    }

    result
}

/// Extract the step ID from a FlowStep.
fn step_id_of(step: &FlowStep) -> String {
    match step {
        FlowStep::OperationStep { id, .. } => id.clone(),
        FlowStep::BranchStep { id, .. } => id.clone(),
        FlowStep::HandoffStep { id, .. } => id.clone(),
        FlowStep::SubFlowStep { id, .. } => id.clone(),
        FlowStep::ParallelStep { id, .. } => id.clone(),
    }
}

/// Layer 2: Entity state equivalence (FMC3).
///
/// For each entity referenced in reachable operation effects:
/// - Check that the entity's current state exists in v2's entity definition
/// - Check that transition targets from operations exist in v2
fn check_layer2_entity_states(
    v1: &Contract,
    v2: &Contract,
    reachable_steps: &[String],
    entity_states: &EntityStateMap,
) -> (bool, Vec<IncompatibilityReason>) {
    let mut reasons = Vec::new();

    // Check all entities that have current states in entity_states
    for (entity_id, current_state) in entity_states {
        // Find the entity in v2
        let v2_entity = v2.entities.iter().find(|e| e.id == *entity_id);

        match v2_entity {
            Some(v2_ent) => {
                // Check current state exists in v2
                if !v2_ent.states.contains(current_state) {
                    reasons.push(IncompatibilityReason::EntityStateNotInV2 {
                        entity_id: entity_id.clone(),
                        state: current_state.clone(),
                    });
                }

                // Check that transition targets from reachable operations exist in v2
                for step_id in reachable_steps {
                    // Find v1 operations referenced by this step
                    if let Some(v1_flow) = v1
                        .flows
                        .iter()
                        .find(|f| f.steps.iter().any(|s| step_id_of(s) == *step_id))
                    {
                        if let Some(FlowStep::OperationStep { op, .. }) =
                            v1_flow.steps.iter().find(|s| step_id_of(s) == *step_id)
                        {
                            // Find the operation in v1
                            if let Some(v1_op) = v1.operations.iter().find(|o| o.id == *op) {
                                for effect in &v1_op.effects {
                                    if effect.entity_id == *entity_id {
                                        // Check target state exists in v2
                                        if !v2_ent.states.contains(&effect.to) {
                                            reasons.push(
                                                IncompatibilityReason::TransitionNotInV2 {
                                                    entity_id: entity_id.clone(),
                                                    from: effect.from.clone(),
                                                    to: effect.to.clone(),
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            None => {
                // Entity doesn't exist in v2 at all
                reasons.push(IncompatibilityReason::EntityStateNotInV2 {
                    entity_id: entity_id.clone(),
                    state: current_state.clone(),
                });
            }
        }
    }

    let pass = reasons.is_empty();
    (pass, reasons)
}

/// Layer 3: Operation/flow structure (FMC1 + FMC2 minus verdicts).
///
/// For each reachable step from position:
/// - Verify the step exists in v2's flow with the same step ID
/// - Verify the step references the same operation ID in v2
/// - Verify the step's persona is authorized under v2's operation
fn check_layer3_structure(
    _v1: &Contract,
    v2: &Contract,
    v1_flow: &Flow,
    v2_flow: Option<&Flow>,
    reachable_steps: &[String],
) -> (bool, Vec<IncompatibilityReason>) {
    let mut reasons = Vec::new();

    let v2_flow = match v2_flow {
        Some(f) => f,
        None => {
            // Flow doesn't exist in v2 at all
            for step_id in reachable_steps {
                reasons.push(IncompatibilityReason::StepNotInV2 {
                    step_id: step_id.clone(),
                });
            }
            return (false, reasons);
        }
    };

    for step_id in reachable_steps {
        // Find the step in v1's flow
        let v1_step = v1_flow.steps.iter().find(|s| step_id_of(s) == *step_id);
        // Find the corresponding step in v2's flow
        let v2_step = v2_flow.steps.iter().find(|s| step_id_of(s) == *step_id);

        match (v1_step, v2_step) {
            (Some(v1_s), Some(v2_s)) => {
                // Check operation ID matches for OperationSteps
                if let (
                    FlowStep::OperationStep {
                        op: v1_op,
                        persona: v1_persona,
                        ..
                    },
                    FlowStep::OperationStep { op: v2_op, .. },
                ) = (v1_s, v2_s)
                {
                    // Check operation reference hasn't changed
                    if v1_op != v2_op {
                        reasons.push(IncompatibilityReason::OperationChangedInV2 {
                            operation_id: v1_op.clone(),
                        });
                    }

                    // Check persona is authorized under v2's operation definition
                    if let Some(v2_operation) = v2.operations.iter().find(|o| o.id == *v2_op) {
                        if !v2_operation.allowed_personas.contains(v1_persona) {
                            reasons.push(IncompatibilityReason::PersonaNotAuthorized {
                                step_id: step_id.clone(),
                                persona: v1_persona.clone(),
                            });
                        }
                    }
                }
            }
            (Some(_), None) => {
                // Step exists in v1 but not in v2
                reasons.push(IncompatibilityReason::StepNotInV2 {
                    step_id: step_id.clone(),
                });
            }
            _ => {}
        }
    }

    let pass = reasons.is_empty();
    (pass, reasons)
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
