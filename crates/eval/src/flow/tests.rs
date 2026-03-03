use super::*;
use crate::operation::EntityStateMap;
use crate::types::*;

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

fn make_contract_with(
    entities: Vec<Entity>,
    operations: Vec<Operation>,
    flows: Vec<Flow>,
) -> Contract {
    Contract::new(
        vec![],
        entities,
        vec![],
        operations,
        flows,
        vec!["admin".to_string(), "system".to_string()],
    )
}

// ──────────────────────────────────────
// Simple flow: entry -> operation -> terminal
// ──────────────────────────────────────

#[test]
fn simple_flow_entry_operation_terminal() {
    let operation = Operation {
        id: "approve_order".to_string(),
        allowed_personas: vec!["admin".to_string()],
        precondition: Predicate::Literal {
            value: Value::Bool(true),
            type_spec: bool_type(),
        },
        effects: vec![Effect {
            entity_id: "order".to_string(),
            from: "pending".to_string(),
            to: "approved".to_string(),
            outcome: None,
        }],
        error_contract: vec![],
        outcomes: vec!["approved".to_string()],
    };

    let flow = Flow {
        id: "approval_flow".to_string(),
        snapshot: "at_initiation".to_string(),
        entry: "step_approve".to_string(),
        steps: vec![FlowStep::OperationStep {
            id: "step_approve".to_string(),
            op: "approve_order".to_string(),
            persona: "admin".to_string(),
            outcomes: {
                let mut m = BTreeMap::new();
                m.insert(
                    "approved".to_string(),
                    StepTarget::Terminal {
                        outcome: "order_approved".to_string(),
                    },
                );
                m
            },
            on_failure: FailureHandler::Terminate {
                outcome: "approval_failed".to_string(),
            },
        }],
    };

    let contract = make_contract_with(
        vec![Entity {
            id: "order".to_string(),
            states: vec!["pending".to_string(), "approved".to_string()],
            initial: "pending".to_string(),
            transitions: vec![],
        }],
        vec![operation],
        vec![flow.clone()],
    );

    let snapshot = Snapshot {
        facts: FactSet::new(),
        verdicts: VerdictSet::new(),
    };

    let mut entity_states = crate::operation::single_instance(
        [("order".to_string(), "pending".to_string())]
            .into_iter()
            .collect(),
    );

    let result = execute_flow(
        &flow,
        &contract,
        &snapshot,
        &mut entity_states,
        &InstanceBindingMap::new(),
        None,
    )
    .unwrap();
    assert_eq!(result.outcome, "order_approved");
    assert_eq!(result.steps_executed.len(), 1);
    assert_eq!(result.steps_executed[0].step_type, "operation");
    assert_eq!(result.steps_executed[0].result, "approved");
    assert_eq!(result.entity_state_changes.len(), 1);
    assert_eq!(
        crate::operation::get_instance_state(
            &entity_states,
            "order",
            crate::operation::DEFAULT_INSTANCE_ID
        )
        .unwrap(),
        "approved"
    );
}

// ──────────────────────────────────────
// Branch flow: entry -> branch -> terminal_a | terminal_b
// ──────────────────────────────────────

#[test]
fn branch_flow_true_path() {
    let flow = Flow {
        id: "branch_flow".to_string(),
        snapshot: "at_initiation".to_string(),
        entry: "step_branch".to_string(),
        steps: vec![FlowStep::BranchStep {
            id: "step_branch".to_string(),
            condition: Predicate::VerdictPresent("is_valid".to_string()),
            persona: "system".to_string(),
            if_true: StepTarget::Terminal {
                outcome: "valid_path".to_string(),
            },
            if_false: StepTarget::Terminal {
                outcome: "invalid_path".to_string(),
            },
        }],
    };

    let contract = make_contract_with(vec![], vec![], vec![flow.clone()]);

    // Verdict is present -> should take true path
    let mut verdicts = VerdictSet::new();
    verdicts.push(VerdictInstance {
        verdict_type: "is_valid".to_string(),
        payload: Value::Bool(true),
        provenance: crate::provenance::VerdictProvenance {
            rule_id: "check".to_string(),
            stratum: 0,
            facts_used: vec![],
            verdicts_used: vec![],
        },
    });

    let snapshot = Snapshot {
        facts: FactSet::new(),
        verdicts,
    };
    let mut entity_states = EntityStateMap::new();

    let result = execute_flow(
        &flow,
        &contract,
        &snapshot,
        &mut entity_states,
        &InstanceBindingMap::new(),
        None,
    )
    .unwrap();
    assert_eq!(result.outcome, "valid_path");
    assert_eq!(result.steps_executed[0].result, "true");
}

#[test]
fn branch_flow_false_path() {
    let flow = Flow {
        id: "branch_flow".to_string(),
        snapshot: "at_initiation".to_string(),
        entry: "step_branch".to_string(),
        steps: vec![FlowStep::BranchStep {
            id: "step_branch".to_string(),
            condition: Predicate::VerdictPresent("is_valid".to_string()),
            persona: "system".to_string(),
            if_true: StepTarget::Terminal {
                outcome: "valid_path".to_string(),
            },
            if_false: StepTarget::Terminal {
                outcome: "invalid_path".to_string(),
            },
        }],
    };

    let contract = make_contract_with(vec![], vec![], vec![flow.clone()]);

    // No verdict -> should take false path
    let snapshot = Snapshot {
        facts: FactSet::new(),
        verdicts: VerdictSet::new(),
    };
    let mut entity_states = EntityStateMap::new();

    let result = execute_flow(
        &flow,
        &contract,
        &snapshot,
        &mut entity_states,
        &InstanceBindingMap::new(),
        None,
    )
    .unwrap();
    assert_eq!(result.outcome, "invalid_path");
    assert_eq!(result.steps_executed[0].result, "false");
}

// ──────────────────────────────────────────────────────────
// CRITICAL: Frozen verdict test
//
// Scenario: A flow where an OperationStep changes entity
// state, then a subsequent BranchStep checks a verdict that
// WOULD change if verdicts were recomputed. The branch MUST
// use the frozen (original) verdict, NOT a recomputed one.
// ──────────────────────────────────────────────────────────

#[test]
fn frozen_verdict_semantics_entity_change_does_not_affect_verdicts() {
    // Setup:
    // - Entity "order" starts in "pending" state
    // - Verdict "order_eligible" was computed at flow initiation
    //   (based on facts/verdicts at that time)
    // - Step 1: OperationStep transitions order from "pending" to "rejected"
    //   (In a recomputing system, this might invalidate "order_eligible")
    // - Step 2: BranchStep checks if verdict "order_eligible" is present
    //   (MUST be true because verdicts are FROZEN at initiation)
    //
    // If frozen semantics are correct: branch takes TRUE path
    // If verdicts were recomputed: branch would take FALSE path

    let operation = Operation {
        id: "reject_order".to_string(),
        allowed_personas: vec!["admin".to_string()],
        precondition: Predicate::Literal {
            value: Value::Bool(true),
            type_spec: bool_type(),
        },
        effects: vec![Effect {
            entity_id: "order".to_string(),
            from: "pending".to_string(),
            to: "rejected".to_string(),
            outcome: None,
        }],
        error_contract: vec![],
        outcomes: vec!["rejected".to_string()],
    };

    let flow = Flow {
        id: "test_frozen_flow".to_string(),
        snapshot: "at_initiation".to_string(),
        entry: "step_reject".to_string(),
        steps: vec![
            // Step 1: Change entity state (pending -> rejected)
            FlowStep::OperationStep {
                id: "step_reject".to_string(),
                op: "reject_order".to_string(),
                persona: "admin".to_string(),
                outcomes: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        "rejected".to_string(),
                        StepTarget::StepRef("step_check".to_string()),
                    );
                    m
                },
                on_failure: FailureHandler::Terminate {
                    outcome: "flow_error".to_string(),
                },
            },
            // Step 2: Check verdict that was set at initiation
            // Even though entity state changed, verdict MUST still be present
            FlowStep::BranchStep {
                id: "step_check".to_string(),
                condition: Predicate::VerdictPresent("order_eligible".to_string()),
                persona: "system".to_string(),
                if_true: StepTarget::Terminal {
                    outcome: "frozen_verdict_confirmed".to_string(),
                },
                if_false: StepTarget::Terminal {
                    outcome: "frozen_verdict_violated".to_string(),
                },
            },
        ],
    };

    let contract = make_contract_with(
        vec![Entity {
            id: "order".to_string(),
            states: vec!["pending".to_string(), "rejected".to_string()],
            initial: "pending".to_string(),
            transitions: vec![],
        }],
        vec![operation],
        vec![flow.clone()],
    );

    // Create snapshot WITH "order_eligible" verdict
    // (In a real system, this would have been computed from rules)
    let mut verdicts = VerdictSet::new();
    verdicts.push(VerdictInstance {
        verdict_type: "order_eligible".to_string(),
        payload: Value::Bool(true),
        provenance: crate::provenance::VerdictProvenance {
            rule_id: "eligibility_check".to_string(),
            stratum: 0,
            facts_used: vec!["order_status".to_string()],
            verdicts_used: vec![],
        },
    });

    let snapshot = Snapshot {
        facts: FactSet::new(),
        verdicts,
    };

    let mut entity_states = crate::operation::single_instance(
        [("order".to_string(), "pending".to_string())]
            .into_iter()
            .collect(),
    );

    let result = execute_flow(
        &flow,
        &contract,
        &snapshot,
        &mut entity_states,
        &InstanceBindingMap::new(),
        None,
    )
    .unwrap();

    // CRITICAL ASSERTION: The branch must use the FROZEN verdict
    // Even though the operation changed entity state to "rejected",
    // the verdict "order_eligible" from the snapshot is still present.
    assert_eq!(
        result.outcome, "frozen_verdict_confirmed",
        "Frozen verdict semantics violated! Branch should have used the original \
         snapshot verdict, not a recomputed one."
    );

    // Verify the entity state DID change (operation was executed)
    assert_eq!(
        crate::operation::get_instance_state(
            &entity_states,
            "order",
            crate::operation::DEFAULT_INSTANCE_ID
        )
        .unwrap(),
        "rejected"
    );

    // Verify both steps executed
    assert_eq!(result.steps_executed.len(), 2);
    assert_eq!(result.steps_executed[0].step_type, "operation");
    assert_eq!(result.steps_executed[1].step_type, "branch");
    assert_eq!(result.steps_executed[1].result, "true");
}

// ──────────────────────────────────────
// Sub-flow inherits parent snapshot
// ──────────────────────────────────────

#[test]
fn sub_flow_inherits_parent_snapshot() {
    // Parent flow calls a sub-flow. The sub-flow should see
    // the same verdicts from the parent's snapshot.

    let operation = Operation {
        id: "finalize".to_string(),
        allowed_personas: vec!["admin".to_string()],
        precondition: Predicate::Literal {
            value: Value::Bool(true),
            type_spec: bool_type(),
        },
        effects: vec![Effect {
            entity_id: "order".to_string(),
            from: "approved".to_string(),
            to: "finalized".to_string(),
            outcome: None,
        }],
        error_contract: vec![],
        outcomes: vec!["done".to_string()],
    };

    // Sub-flow: branch on verdict, then terminal
    let sub_flow = Flow {
        id: "sub_check_flow".to_string(),
        snapshot: "at_initiation".to_string(),
        entry: "sub_branch".to_string(),
        steps: vec![FlowStep::BranchStep {
            id: "sub_branch".to_string(),
            condition: Predicate::VerdictPresent("parent_verdict".to_string()),
            persona: "system".to_string(),
            if_true: StepTarget::Terminal {
                outcome: "sub_success".to_string(),
            },
            if_false: StepTarget::Terminal {
                outcome: "sub_failure".to_string(),
            },
        }],
    };

    // Parent flow: op -> sub-flow -> terminal
    let parent_flow = Flow {
        id: "parent_flow".to_string(),
        snapshot: "at_initiation".to_string(),
        entry: "step_op".to_string(),
        steps: vec![
            FlowStep::OperationStep {
                id: "step_op".to_string(),
                op: "finalize".to_string(),
                persona: "admin".to_string(),
                outcomes: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        "done".to_string(),
                        StepTarget::StepRef("step_sub".to_string()),
                    );
                    m
                },
                on_failure: FailureHandler::Terminate {
                    outcome: "parent_error".to_string(),
                },
            },
            FlowStep::SubFlowStep {
                id: "step_sub".to_string(),
                flow: "sub_check_flow".to_string(),
                persona: "admin".to_string(),
                on_success: StepTarget::Terminal {
                    outcome: "parent_success".to_string(),
                },
                on_failure: FailureHandler::Terminate {
                    outcome: "parent_error".to_string(),
                },
            },
        ],
    };

    let contract = make_contract_with(
        vec![Entity {
            id: "order".to_string(),
            states: vec!["approved".to_string(), "finalized".to_string()],
            initial: "approved".to_string(),
            transitions: vec![],
        }],
        vec![operation],
        vec![parent_flow.clone(), sub_flow],
    );

    // Snapshot with parent_verdict present
    let mut verdicts = VerdictSet::new();
    verdicts.push(VerdictInstance {
        verdict_type: "parent_verdict".to_string(),
        payload: Value::Bool(true),
        provenance: crate::provenance::VerdictProvenance {
            rule_id: "parent_rule".to_string(),
            stratum: 0,
            facts_used: vec![],
            verdicts_used: vec![],
        },
    });

    let snapshot = Snapshot {
        facts: FactSet::new(),
        verdicts,
    };

    let mut entity_states = crate::operation::single_instance(
        [("order".to_string(), "approved".to_string())]
            .into_iter()
            .collect(),
    );

    let result = execute_flow(
        &parent_flow,
        &contract,
        &snapshot,
        &mut entity_states,
        &InstanceBindingMap::new(),
        None,
    )
    .unwrap();

    // Sub-flow should have seen "parent_verdict" from the inherited snapshot
    assert_eq!(result.outcome, "parent_success");
    assert_eq!(result.steps_executed.len(), 2); // op + sub_flow
    assert_eq!(result.steps_executed[1].step_type, "sub_flow");
    assert_eq!(result.steps_executed[1].result, "sub_success");
}

// ──────────────────────────────────────
// Operation on_failure routing
// ──────────────────────────────────────

#[test]
fn operation_step_on_failure_terminates() {
    // Operation will fail because entity is in wrong state
    let operation = Operation {
        id: "approve".to_string(),
        allowed_personas: vec!["admin".to_string()],
        precondition: Predicate::Literal {
            value: Value::Bool(true),
            type_spec: bool_type(),
        },
        effects: vec![Effect {
            entity_id: "order".to_string(),
            from: "pending".to_string(),
            to: "approved".to_string(),
            outcome: None,
        }],
        error_contract: vec![],
        outcomes: vec!["approved".to_string()],
    };

    let flow = Flow {
        id: "fail_flow".to_string(),
        snapshot: "at_initiation".to_string(),
        entry: "step_approve".to_string(),
        steps: vec![FlowStep::OperationStep {
            id: "step_approve".to_string(),
            op: "approve".to_string(),
            persona: "admin".to_string(),
            outcomes: {
                let mut m = BTreeMap::new();
                m.insert(
                    "approved".to_string(),
                    StepTarget::Terminal {
                        outcome: "success".to_string(),
                    },
                );
                m
            },
            on_failure: FailureHandler::Terminate {
                outcome: "failure_handled".to_string(),
            },
        }],
    };

    let contract = make_contract_with(
        vec![Entity {
            id: "order".to_string(),
            states: vec![
                "draft".to_string(),
                "pending".to_string(),
                "approved".to_string(),
            ],
            initial: "draft".to_string(),
            transitions: vec![],
        }],
        vec![operation],
        vec![flow.clone()],
    );

    let snapshot = Snapshot {
        facts: FactSet::new(),
        verdicts: VerdictSet::new(),
    };

    // Entity is in "draft", not "pending" -- operation will fail
    let mut entity_states = crate::operation::single_instance(
        [("order".to_string(), "draft".to_string())]
            .into_iter()
            .collect(),
    );

    let result = execute_flow(
        &flow,
        &contract,
        &snapshot,
        &mut entity_states,
        &InstanceBindingMap::new(),
        None,
    )
    .unwrap();
    assert_eq!(result.outcome, "failure_handled");
}

// ──────────────────────────────────────
// Multi-step flow: operation -> branch -> terminal
// ──────────────────────────────────────

#[test]
fn multi_step_flow_operation_then_branch() {
    let operation = Operation {
        id: "submit".to_string(),
        allowed_personas: vec!["buyer".to_string()],
        precondition: Predicate::Literal {
            value: Value::Bool(true),
            type_spec: bool_type(),
        },
        effects: vec![Effect {
            entity_id: "order".to_string(),
            from: "draft".to_string(),
            to: "submitted".to_string(),
            outcome: None,
        }],
        error_contract: vec![],
        outcomes: vec!["submitted".to_string()],
    };

    let flow = Flow {
        id: "submit_flow".to_string(),
        snapshot: "at_initiation".to_string(),
        entry: "step_submit".to_string(),
        steps: vec![
            FlowStep::OperationStep {
                id: "step_submit".to_string(),
                op: "submit".to_string(),
                persona: "buyer".to_string(),
                outcomes: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        "submitted".to_string(),
                        StepTarget::StepRef("step_check".to_string()),
                    );
                    m
                },
                on_failure: FailureHandler::Terminate {
                    outcome: "submit_failed".to_string(),
                },
            },
            FlowStep::BranchStep {
                id: "step_check".to_string(),
                condition: Predicate::VerdictPresent("order_valid".to_string()),
                persona: "system".to_string(),
                if_true: StepTarget::Terminal {
                    outcome: "submitted_valid".to_string(),
                },
                if_false: StepTarget::Terminal {
                    outcome: "submitted_invalid".to_string(),
                },
            },
        ],
    };

    let contract = make_contract_with(
        vec![Entity {
            id: "order".to_string(),
            states: vec!["draft".to_string(), "submitted".to_string()],
            initial: "draft".to_string(),
            transitions: vec![],
        }],
        vec![operation],
        vec![flow.clone()],
    );

    // Snapshot with "order_valid" verdict
    let mut verdicts = VerdictSet::new();
    verdicts.push(VerdictInstance {
        verdict_type: "order_valid".to_string(),
        payload: Value::Bool(true),
        provenance: crate::provenance::VerdictProvenance {
            rule_id: "validate".to_string(),
            stratum: 0,
            facts_used: vec![],
            verdicts_used: vec![],
        },
    });

    let snapshot = Snapshot {
        facts: FactSet::new(),
        verdicts,
    };

    let mut entity_states = crate::operation::single_instance(
        [("order".to_string(), "draft".to_string())]
            .into_iter()
            .collect(),
    );

    let result = execute_flow(
        &flow,
        &contract,
        &snapshot,
        &mut entity_states,
        &InstanceBindingMap::new(),
        None,
    )
    .unwrap();
    assert_eq!(result.outcome, "submitted_valid");
    assert_eq!(result.steps_executed.len(), 2);
    assert_eq!(
        crate::operation::get_instance_state(
            &entity_states,
            "order",
            crate::operation::DEFAULT_INSTANCE_ID
        )
        .unwrap(),
        "submitted"
    );
}

// ──────────────────────────────────────
// C9: FlowError variant test (E1 fix)
// ──────────────────────────────────────

#[test]
fn flow_step_limit_returns_flow_error() {
    // Create a circular flow that will exceed the step limit.
    // step_a -> step_b -> step_a (infinite loop)
    let operation_a = Operation {
        id: "op_a".to_string(),
        allowed_personas: vec!["admin".to_string()],
        precondition: Predicate::Literal {
            value: Value::Bool(true),
            type_spec: bool_type(),
        },
        effects: vec![],
        error_contract: vec![],
        outcomes: vec!["done".to_string()],
    };

    let operation_b = Operation {
        id: "op_b".to_string(),
        allowed_personas: vec!["admin".to_string()],
        precondition: Predicate::Literal {
            value: Value::Bool(true),
            type_spec: bool_type(),
        },
        effects: vec![],
        error_contract: vec![],
        outcomes: vec!["done".to_string()],
    };

    let flow = Flow {
        id: "loop_flow".to_string(),
        snapshot: "at_initiation".to_string(),
        entry: "step_a".to_string(),
        steps: vec![
            FlowStep::OperationStep {
                id: "step_a".to_string(),
                op: "op_a".to_string(),
                persona: "admin".to_string(),
                outcomes: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        "done".to_string(),
                        StepTarget::StepRef("step_b".to_string()),
                    );
                    m
                },
                on_failure: FailureHandler::Terminate {
                    outcome: "error".to_string(),
                },
            },
            FlowStep::OperationStep {
                id: "step_b".to_string(),
                op: "op_b".to_string(),
                persona: "admin".to_string(),
                outcomes: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        "done".to_string(),
                        StepTarget::StepRef("step_a".to_string()), // circular!
                    );
                    m
                },
                on_failure: FailureHandler::Terminate {
                    outcome: "error".to_string(),
                },
            },
        ],
    };

    let contract = make_contract_with(vec![], vec![operation_a, operation_b], vec![flow.clone()]);

    let snapshot = Snapshot {
        facts: FactSet::new(),
        verdicts: VerdictSet::new(),
    };

    let mut entity_states = EntityStateMap::new();

    let result = execute_flow(
        &flow,
        &contract,
        &snapshot,
        &mut entity_states,
        &InstanceBindingMap::new(),
        None,
    );
    assert!(result.is_err());
    match result.unwrap_err() {
        EvalError::FlowError { flow_id, message } => {
            assert_eq!(flow_id, "loop_flow");
            assert!(
                message.contains("maximum step count"),
                "error should mention step count, got: {}",
                message
            );
        }
        other => panic!("expected FlowError, got {:?}", other),
    }
}

// ──────────────────────────────────────
// Configurable max_steps parameter test
// ──────────────────────────────────────

// ──────────────────────────────────────────────────────────
// BEHAVIORAL CHARACTERIZATION: Parallel branch entity state conflict
//
// This test documents the ACTUAL behavior when two parallel branches
// modify the same entity to different states. The spec (Section 10.5,
// Section 12.2 Pass 5) requires non-overlapping entity effect sets
// across parallel branches, and the elaborator enforces this at
// compile time (Pass 5). However, the evaluator does NOT enforce
// this constraint at runtime -- it trusts the elaborator.
//
// This test constructs a scenario that would be rejected by the
// elaborator but is valid interchange JSON, to characterize the
// evaluator's merge semantics when two branches write conflicting
// entity states.
//
// Observed behavior: LAST-WRITER-WINS (silent).
// - Both branches execute independently from cloned entity state.
// - Both succeed (each sees the entity in "initial" state).
// - During merge, branch outcomes are processed in declaration order.
// - The last branch's entity_states.insert() overwrites the first's.
// - entity_changes_all contains BOTH effect records (no dedup).
// - No error is raised. No warning is emitted.
//
// This is a characterization test, not a prescriptive test. If the
// evaluator is changed to detect conflicts, this test should be
// updated to reflect the new behavior.
// ──────────────────────────────────────────────────────────

#[test]
fn parallel_branch_entity_conflict_last_writer_wins() {
    // Entity: Order with states [initial, state_a, state_b]
    let entity = Entity {
        id: "Order".to_string(),
        states: vec![
            "initial".to_string(),
            "state_a".to_string(),
            "state_b".to_string(),
        ],
        initial: "initial".to_string(),
        transitions: vec![],
    };

    // op_to_a: transitions Order from initial -> state_a
    let op_a = Operation {
        id: "op_to_a".to_string(),
        allowed_personas: vec!["system".to_string()],
        precondition: Predicate::Literal {
            value: Value::Bool(true),
            type_spec: bool_type(),
        },
        effects: vec![Effect {
            entity_id: "Order".to_string(),
            from: "initial".to_string(),
            to: "state_a".to_string(),
            outcome: None,
        }],
        error_contract: vec![],
        outcomes: vec!["success".to_string()],
    };

    // op_to_b: transitions Order from initial -> state_b
    let op_b = Operation {
        id: "op_to_b".to_string(),
        allowed_personas: vec!["system".to_string()],
        precondition: Predicate::Literal {
            value: Value::Bool(true),
            type_spec: bool_type(),
        },
        effects: vec![Effect {
            entity_id: "Order".to_string(),
            from: "initial".to_string(),
            to: "state_b".to_string(),
            outcome: None,
        }],
        error_contract: vec![],
        outcomes: vec!["success".to_string()],
    };

    // Flow with a parallel step: branch_a runs op_to_a, branch_b runs op_to_b
    let flow = Flow {
        id: "conflict_flow".to_string(),
        snapshot: "at_initiation".to_string(),
        entry: "par_step".to_string(),
        steps: vec![FlowStep::ParallelStep {
            id: "par_step".to_string(),
            branches: vec![
                ParallelBranch {
                    id: "branch_a".to_string(),
                    entry: "step_a".to_string(),
                    steps: vec![FlowStep::OperationStep {
                        id: "step_a".to_string(),
                        op: "op_to_a".to_string(),
                        persona: "system".to_string(),
                        outcomes: {
                            let mut m = BTreeMap::new();
                            m.insert(
                                "success".to_string(),
                                StepTarget::Terminal {
                                    outcome: "branch_a_done".to_string(),
                                },
                            );
                            m
                        },
                        on_failure: FailureHandler::Terminate {
                            outcome: "branch_a_failed".to_string(),
                        },
                    }],
                },
                ParallelBranch {
                    id: "branch_b".to_string(),
                    entry: "step_b".to_string(),
                    steps: vec![FlowStep::OperationStep {
                        id: "step_b".to_string(),
                        op: "op_to_b".to_string(),
                        persona: "system".to_string(),
                        outcomes: {
                            let mut m = BTreeMap::new();
                            m.insert(
                                "success".to_string(),
                                StepTarget::Terminal {
                                    outcome: "branch_b_done".to_string(),
                                },
                            );
                            m
                        },
                        on_failure: FailureHandler::Terminate {
                            outcome: "branch_b_failed".to_string(),
                        },
                    }],
                },
            ],
            join: JoinPolicy {
                on_all_success: Some(StepTarget::Terminal {
                    outcome: "all_done".to_string(),
                }),
                on_any_failure: Some(FailureHandler::Terminate {
                    outcome: "some_failed".to_string(),
                }),
                on_all_complete: None,
            },
        }],
    };

    let contract = make_contract_with(vec![entity], vec![op_a, op_b], vec![flow.clone()]);

    let snapshot = Snapshot {
        facts: FactSet::new(),
        verdicts: VerdictSet::new(),
    };

    let mut entity_states = crate::operation::single_instance(
        [("Order".to_string(), "initial".to_string())]
            .into_iter()
            .collect(),
    );

    let result = execute_flow(
        &flow,
        &contract,
        &snapshot,
        &mut entity_states,
        &InstanceBindingMap::new(),
        None,
    );

    // ── Characterize actual behavior ──

    // The flow should succeed (no conflict detection at the evaluator level).
    assert!(
        result.is_ok(),
        "Expected flow to succeed (no runtime conflict detection), but got error: {:?}",
        result.err()
    );

    let flow_result = result.unwrap();

    // Both branches succeed, so on_all_success fires -> outcome "all_done"
    assert_eq!(
        flow_result.outcome, "all_done",
        "Expected on_all_success outcome since both branches succeed independently"
    );

    // CRITICAL OBSERVATION: The final entity state is determined by
    // branch processing order. Since branches are iterated in declaration
    // order and the merge loop does entity_states.insert() for each,
    // the LAST branch's state wins.
    //
    // branch_a sets Order -> state_a
    // branch_b sets Order -> state_b (overwrites state_a)
    let final_state = crate::operation::get_instance_state(
        &entity_states,
        "Order",
        crate::operation::DEFAULT_INSTANCE_ID,
    )
    .unwrap();
    assert_eq!(
        final_state, "state_b",
        "Last-writer-wins: branch_b (declared second) overwrites branch_a's state. \
         Final entity state is '{}', expected 'state_b'.",
        final_state
    );

    // Both effect records are collected (extend, not replace).
    // This means entity_changes_all contains contradictory records:
    // [Order: initial->state_a, Order: initial->state_b]
    assert_eq!(
        flow_result.entity_state_changes.len(),
        2,
        "Both branches' effect records are collected (no dedup). Got: {:?}",
        flow_result.entity_state_changes
    );

    // Verify the individual effect records
    let change_a = &flow_result.entity_state_changes[0];
    assert_eq!(change_a.entity_id, "Order");
    assert_eq!(change_a.from_state, "initial");
    assert_eq!(change_a.to_state, "state_a");

    let change_b = &flow_result.entity_state_changes[1];
    assert_eq!(change_b.entity_id, "Order");
    assert_eq!(change_b.from_state, "initial");
    assert_eq!(change_b.to_state, "state_b");

    // Verify steps executed: parallel step + 2 branch operation steps
    assert_eq!(flow_result.steps_executed.len(), 3);
    assert_eq!(flow_result.steps_executed[0].step_type, "parallel");
    assert_eq!(flow_result.steps_executed[1].step_type, "operation");
    assert_eq!(flow_result.steps_executed[2].step_type, "operation");

    // The parallel step's result shows both branches succeeded
    assert!(
        flow_result.steps_executed[0]
            .result
            .contains("branch_a:branch_a_done"),
        "branch_a should report success"
    );
    assert!(
        flow_result.steps_executed[0]
            .result
            .contains("branch_b:branch_b_done"),
        "branch_b should report success"
    );
}

#[test]
fn configurable_max_steps_triggers_at_custom_limit() {
    // A circular flow: step_a -> step_b -> step_a
    // With max_steps=5, should fail after 5 steps.
    let operation_a = Operation {
        id: "op_a".to_string(),
        allowed_personas: vec!["admin".to_string()],
        precondition: Predicate::Literal {
            value: Value::Bool(true),
            type_spec: bool_type(),
        },
        effects: vec![],
        error_contract: vec![],
        outcomes: vec!["done".to_string()],
    };

    let operation_b = Operation {
        id: "op_b".to_string(),
        allowed_personas: vec!["admin".to_string()],
        precondition: Predicate::Literal {
            value: Value::Bool(true),
            type_spec: bool_type(),
        },
        effects: vec![],
        error_contract: vec![],
        outcomes: vec!["done".to_string()],
    };

    let flow = Flow {
        id: "custom_limit_flow".to_string(),
        snapshot: "at_initiation".to_string(),
        entry: "step_a".to_string(),
        steps: vec![
            FlowStep::OperationStep {
                id: "step_a".to_string(),
                op: "op_a".to_string(),
                persona: "admin".to_string(),
                outcomes: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        "done".to_string(),
                        StepTarget::StepRef("step_b".to_string()),
                    );
                    m
                },
                on_failure: FailureHandler::Terminate {
                    outcome: "error".to_string(),
                },
            },
            FlowStep::OperationStep {
                id: "step_b".to_string(),
                op: "op_b".to_string(),
                persona: "admin".to_string(),
                outcomes: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        "done".to_string(),
                        StepTarget::StepRef("step_a".to_string()),
                    );
                    m
                },
                on_failure: FailureHandler::Terminate {
                    outcome: "error".to_string(),
                },
            },
        ],
    };

    let contract = make_contract_with(vec![], vec![operation_a, operation_b], vec![flow.clone()]);

    let snapshot = Snapshot {
        facts: FactSet::new(),
        verdicts: VerdictSet::new(),
    };

    // With max_steps=5, should fail
    let mut entity_states = EntityStateMap::new();
    let result = execute_flow(
        &flow,
        &contract,
        &snapshot,
        &mut entity_states,
        &InstanceBindingMap::new(),
        Some(5),
    );
    assert!(result.is_err());
    match result.unwrap_err() {
        EvalError::FlowError { flow_id, message } => {
            assert_eq!(flow_id, "custom_limit_flow");
            assert!(
                message.contains("5"),
                "error should mention the custom limit 5, got: {}",
                message
            );
        }
        other => panic!("expected FlowError, got {:?}", other),
    }

    // With None (default 1000), the same flow would also eventually fail
    // but at step 1000 -- just verify it returns a FlowError
    let mut entity_states2 = EntityStateMap::new();
    let result2 = execute_flow(
        &flow,
        &contract,
        &snapshot,
        &mut entity_states2,
        &InstanceBindingMap::new(),
        None,
    );
    assert!(result2.is_err());
    match result2.unwrap_err() {
        EvalError::FlowError { message, .. } => {
            assert!(
                message.contains("1000"),
                "default limit should be 1000, got: {}",
                message
            );
        }
        other => panic!("expected FlowError with default limit, got {:?}", other),
    }
}
