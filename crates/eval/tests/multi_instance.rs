//! Multi-instance entity integration tests.
//!
//! Exercises all multi-instance behavior from Plans 04-01 through 04-03.
//! Covers section A8 of docs/plans/plan-4.md:
//!
//! 1. Multiple instances same entity type — per-instance action space
//! 2. Action space per-instance — which instances are valid for which flows
//! 3. Execute with instance targeting — only targeted instance changes
//! 4. Flow with instance bindings — flow targets specific instances with provenance
//! 5. Missing instance binding → error (EntityNotFound via DEFAULT_INSTANCE_ID fallback)
//! 6. Single-instance degenerate case — backward compat proof
//! 7. Instance absence — absent instances (not in EntityStateMap) don't exist

use std::collections::{BTreeMap, BTreeSet};

use tenor_eval::{
    action_space::{compute_action_space, BlockedReason},
    flow::{execute_flow, Snapshot},
    operation::{
        execute_operation, get_instance_state, single_instance, EntityStateMap, InstanceBindingMap,
        OperationError, DEFAULT_INSTANCE_ID,
    },
    types::{
        Contract, Effect, Entity, FactSet, FailureHandler, Flow, FlowStep, Operation, Predicate,
        StepTarget, TypeSpec, Value, VerdictSet,
    },
};

// ──────────────────────────────────────────────
// Test fixtures
// ──────────────────────────────────────────────

/// Create a `TypeSpec` for Bool — used in operations that need a precondition literal.
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

/// Build a true-precondition operation (unconditionally executable by authorized persona).
fn make_operation(
    id: &str,
    personas: Vec<&str>,
    effects: Vec<Effect>,
    outcomes: Vec<&str>,
) -> Operation {
    Operation {
        id: id.to_string(),
        allowed_personas: personas.into_iter().map(|p| p.to_string()).collect(),
        precondition: Predicate::Literal {
            value: Value::Bool(true),
            type_spec: bool_type(),
        },
        effects,
        error_contract: vec![],
        outcomes: outcomes.into_iter().map(|o| o.to_string()).collect(),
    }
}

/// Build the Order entity used across all tests.
///
/// States: draft → submitted → approved | rejected
fn order_entity() -> Entity {
    Entity {
        id: "Order".to_string(),
        states: vec![
            "draft".to_string(),
            "submitted".to_string(),
            "approved".to_string(),
            "rejected".to_string(),
        ],
        initial: "draft".to_string(),
        transitions: vec![],
    }
}

/// Build standard Order operations.
///
/// - submit_order: draft → submitted  (allowed: buyer)
/// - approve_order: submitted → approved  (allowed: admin)
/// - reject_order: submitted → rejected  (allowed: admin)
fn order_operations() -> Vec<Operation> {
    vec![
        make_operation(
            "submit_order",
            vec!["buyer"],
            vec![Effect {
                entity_id: "Order".to_string(),
                from: "draft".to_string(),
                to: "submitted".to_string(),
                outcome: None,
            }],
            vec!["submitted"],
        ),
        make_operation(
            "approve_order",
            vec!["admin"],
            vec![Effect {
                entity_id: "Order".to_string(),
                from: "submitted".to_string(),
                to: "approved".to_string(),
                outcome: None,
            }],
            vec!["approved"],
        ),
        make_operation(
            "reject_order",
            vec!["admin"],
            vec![Effect {
                entity_id: "Order".to_string(),
                from: "submitted".to_string(),
                to: "rejected".to_string(),
                outcome: None,
            }],
            vec!["rejected"],
        ),
    ]
}

/// Build a minimal single-step flow for the given operation.
///
/// Flow: entry_step (OperationStep) → Terminal outcome
fn simple_flow(flow_id: &str, op_id: &str, persona: &str, op_outcome: &str) -> Flow {
    let mut outcomes = BTreeMap::new();
    outcomes.insert(
        op_outcome.to_string(),
        StepTarget::Terminal {
            outcome: format!("{}_done", op_id),
        },
    );

    Flow {
        id: flow_id.to_string(),
        snapshot: "at_initiation".to_string(),
        entry: "step_1".to_string(),
        steps: vec![FlowStep::OperationStep {
            id: "step_1".to_string(),
            op: op_id.to_string(),
            persona: persona.to_string(),
            outcomes,
            on_failure: FailureHandler::Terminate {
                outcome: format!("{}_failed", op_id),
            },
        }],
    }
}

/// Build a Contract with Order entity, operations, and flows.
fn make_order_contract(flows: Vec<Flow>) -> Contract {
    Contract::new(
        vec![],
        vec![order_entity()],
        vec![],
        order_operations(),
        flows,
        vec!["buyer".to_string(), "admin".to_string()],
    )
}

/// Build an action space bundle JSON for use with `compute_action_space`.
///
/// This is the interchange-JSON representation of the Order contract
/// with a submit flow (entry op: submit_order, allowed: buyer).
fn order_action_space_bundle() -> serde_json::Value {
    serde_json::json!({
        "id": "multi_instance_test",
        "kind": "Bundle",
        "tenor": "1.0",
        "tenor_version": "1.0.0",
        "constructs": [
            {
                "id": "Order",
                "initial": "draft",
                "kind": "Entity",
                "provenance": { "file": "test.tenor", "line": 1 },
                "states": ["draft", "submitted", "approved", "rejected"],
                "tenor": "1.0",
                "transitions": [
                    { "from": "draft", "to": "submitted" },
                    { "from": "submitted", "to": "approved" },
                    { "from": "submitted", "to": "rejected" }
                ]
            },
            {
                "allowed_personas": ["buyer"],
                "effects": [{ "entity_id": "Order", "from": "draft", "to": "submitted" }],
                "error_contract": [],
                "id": "submit_order",
                "kind": "Operation",
                "precondition": { "literal": true, "type": { "base": "Bool" } },
                "provenance": { "file": "test.tenor", "line": 10 },
                "tenor": "1.0",
                "outcomes": ["submitted"]
            },
            {
                "allowed_personas": ["admin"],
                "effects": [{ "entity_id": "Order", "from": "submitted", "to": "approved" }],
                "error_contract": [],
                "id": "approve_order",
                "kind": "Operation",
                "precondition": { "literal": true, "type": { "base": "Bool" } },
                "provenance": { "file": "test.tenor", "line": 15 },
                "tenor": "1.0",
                "outcomes": ["approved"]
            },
            {
                "allowed_personas": ["admin"],
                "effects": [{ "entity_id": "Order", "from": "submitted", "to": "rejected" }],
                "error_contract": [],
                "id": "reject_order",
                "kind": "Operation",
                "precondition": { "literal": true, "type": { "base": "Bool" } },
                "provenance": { "file": "test.tenor", "line": 20 },
                "tenor": "1.0",
                "outcomes": ["rejected"]
            },
            {
                "entry": "step_submit",
                "id": "submit_flow",
                "kind": "Flow",
                "provenance": { "file": "test.tenor", "line": 25 },
                "snapshot": "at_initiation",
                "steps": [{
                    "id": "step_submit",
                    "kind": "OperationStep",
                    "on_failure": { "kind": "Terminate", "outcome": "submit_failed" },
                    "op": "submit_order",
                    "outcomes": {
                        "submitted": { "kind": "Terminal", "outcome": "submit_done" }
                    },
                    "persona": "buyer"
                }],
                "tenor": "1.0"
            },
            {
                "entry": "step_approve",
                "id": "approve_flow",
                "kind": "Flow",
                "provenance": { "file": "test.tenor", "line": 35 },
                "snapshot": "at_initiation",
                "steps": [{
                    "id": "step_approve",
                    "kind": "OperationStep",
                    "on_failure": { "kind": "Terminate", "outcome": "approve_failed" },
                    "op": "approve_order",
                    "outcomes": {
                        "approved": { "kind": "Terminal", "outcome": "approve_done" }
                    },
                    "persona": "admin"
                }],
                "tenor": "1.0"
            },
            {
                "entry": "step_reject",
                "id": "reject_flow",
                "kind": "Flow",
                "provenance": { "file": "test.tenor", "line": 45 },
                "snapshot": "at_initiation",
                "steps": [{
                    "id": "step_reject",
                    "kind": "OperationStep",
                    "on_failure": { "kind": "Terminate", "outcome": "reject_failed" },
                    "op": "reject_order",
                    "outcomes": {
                        "rejected": { "kind": "Terminal", "outcome": "reject_done" }
                    },
                    "persona": "admin"
                }],
                "tenor": "1.0"
            }
        ]
    })
}

// ──────────────────────────────────────────────
// Test 1: Multiple instances of the same entity type
// §6.5 — multiple instances coexist with independent states
// ──────────────────────────────────────────────

/// Verifies that three Order instances can coexist in different states and the
/// action space correctly identifies per-instance availability.
///
/// ord-001 = draft   → submit_order available (draft → submitted)
/// ord-002 = submitted → submit_order NOT available (already submitted)
/// ord-003 = approved  → submit_order NOT available (wrong state)
#[test]
fn multiple_instances_same_entity_type() {
    let bundle = order_action_space_bundle();
    let contract = Contract::from_interchange(&bundle).unwrap();
    let facts = serde_json::json!({});

    let mut entity_states = EntityStateMap::new();
    entity_states.insert(
        ("Order".to_string(), "ord-001".to_string()),
        "draft".to_string(),
    );
    entity_states.insert(
        ("Order".to_string(), "ord-002".to_string()),
        "submitted".to_string(),
    );
    entity_states.insert(
        ("Order".to_string(), "ord-003".to_string()),
        "approved".to_string(),
    );

    let result = compute_action_space(&contract, &facts, &entity_states, "buyer").unwrap();

    // submit_order requires Order in "draft" state
    // ord-001 is draft → should be in valid instance bindings
    // ord-002, ord-003 are NOT draft → should NOT appear in valid bindings
    let submit_action = result.actions.iter().find(|a| a.flow_id == "submit_flow");
    assert!(
        submit_action.is_some(),
        "submit_flow should be available (ord-001 is in draft)"
    );

    let bindings = &submit_action.unwrap().instance_bindings;
    assert!(
        bindings.contains_key("Order"),
        "Order should appear in instance_bindings"
    );

    let valid_instances: &BTreeSet<String> = &bindings["Order"];
    assert!(
        valid_instances.contains("ord-001"),
        "ord-001 (draft) should be valid for submit_order"
    );
    assert!(
        !valid_instances.contains("ord-002"),
        "ord-002 (submitted) should NOT be valid for submit_order"
    );
    assert!(
        !valid_instances.contains("ord-003"),
        "ord-003 (approved) should NOT be valid for submit_order"
    );
    assert_eq!(
        valid_instances.len(),
        1,
        "exactly one instance (ord-001) should be valid"
    );
}

// ──────────────────────────────────────────────
// Test 2: Per-instance action space
// §15.6 — action space is instance-keyed
// ──────────────────────────────────────────────

/// Verifies that when Order instances are in different states, the action space
/// correctly partitions them: each flow only sees instances in the right state.
///
/// ord-001 = draft   → submit_flow available for ord-001
/// ord-002 = submitted → approve_flow and reject_flow available for ord-002
///
/// Key: a "buyer" sees submit_flow (ord-001 valid), an "admin" sees approve/reject
/// flows (ord-002 valid). This exercises per-flow, per-instance discrimination.
#[test]
fn action_space_per_instance() {
    let bundle = order_action_space_bundle();
    let contract = Contract::from_interchange(&bundle).unwrap();
    let facts = serde_json::json!({});

    let mut entity_states = EntityStateMap::new();
    entity_states.insert(
        ("Order".to_string(), "ord-001".to_string()),
        "draft".to_string(),
    );
    entity_states.insert(
        ("Order".to_string(), "ord-002".to_string()),
        "submitted".to_string(),
    );

    // Buyer: submit_flow should list ord-001 as valid (draft→submitted)
    let buyer_space = compute_action_space(&contract, &facts, &entity_states, "buyer").unwrap();

    let submit_action = buyer_space
        .actions
        .iter()
        .find(|a| a.flow_id == "submit_flow");
    assert!(
        submit_action.is_some(),
        "buyer should see submit_flow available"
    );
    let submit_bindings = &submit_action.unwrap().instance_bindings;
    assert!(
        submit_bindings["Order"].contains("ord-001"),
        "submit_flow instance_bindings should contain ord-001 (draft)"
    );
    assert!(
        !submit_bindings["Order"].contains("ord-002"),
        "submit_flow instance_bindings should NOT contain ord-002 (submitted)"
    );

    // Admin: approve_flow and reject_flow should list ord-002 as valid (submitted→...)
    let admin_space = compute_action_space(&contract, &facts, &entity_states, "admin").unwrap();

    let approve_action = admin_space
        .actions
        .iter()
        .find(|a| a.flow_id == "approve_flow");
    assert!(
        approve_action.is_some(),
        "admin should see approve_flow available"
    );
    let approve_bindings = &approve_action.unwrap().instance_bindings;
    assert!(
        approve_bindings["Order"].contains("ord-002"),
        "approve_flow should target ord-002 (submitted)"
    );
    assert!(
        !approve_bindings["Order"].contains("ord-001"),
        "approve_flow should NOT target ord-001 (draft)"
    );

    let reject_action = admin_space
        .actions
        .iter()
        .find(|a| a.flow_id == "reject_flow");
    assert!(
        reject_action.is_some(),
        "admin should see reject_flow available"
    );
    let reject_bindings = &reject_action.unwrap().instance_bindings;
    assert!(
        reject_bindings["Order"].contains("ord-002"),
        "reject_flow should target ord-002 (submitted)"
    );
    assert!(
        !reject_bindings["Order"].contains("ord-001"),
        "reject_flow should NOT target ord-001 (draft)"
    );
}

// ──────────────────────────────────────────────
// Test 3: Instance-targeted operation execution
// §9.2 — only targeted instance transitions
// ──────────────────────────────────────────────

/// Verifies that executing submit_order on ord-001 only transitions ord-001.
/// ord-002 (submitted) and ord-003 (approved) remain untouched.
///
/// This is the core multi-instance isolation guarantee.
#[test]
fn execute_with_instance_targeting() {
    let submit_op = make_operation(
        "submit_order",
        vec!["buyer"],
        vec![Effect {
            entity_id: "Order".to_string(),
            from: "draft".to_string(),
            to: "submitted".to_string(),
            outcome: None,
        }],
        vec!["submitted"],
    );

    let facts = FactSet::new();
    let verdicts = VerdictSet::new();

    let mut entity_states = EntityStateMap::new();
    entity_states.insert(
        ("Order".to_string(), "ord-001".to_string()),
        "draft".to_string(),
    );
    entity_states.insert(
        ("Order".to_string(), "ord-002".to_string()),
        "submitted".to_string(),
    );
    entity_states.insert(
        ("Order".to_string(), "ord-003".to_string()),
        "approved".to_string(),
    );

    // Target only ord-001
    let mut bindings = InstanceBindingMap::new();
    bindings.insert("Order".to_string(), "ord-001".to_string());

    let result = execute_operation(
        &submit_op,
        "buyer",
        &facts,
        &verdicts,
        &mut entity_states,
        &bindings,
    )
    .unwrap();

    assert_eq!(result.outcome, "submitted");
    assert_eq!(result.effects_applied.len(), 1);
    assert_eq!(result.effects_applied[0].instance_id, "ord-001");
    assert_eq!(result.effects_applied[0].from_state, "draft");
    assert_eq!(result.effects_applied[0].to_state, "submitted");

    // ord-001 transitioned to submitted
    assert_eq!(
        get_instance_state(&entity_states, "Order", "ord-001").unwrap(),
        "submitted",
        "ord-001 should be in submitted state"
    );

    // ord-002 UNCHANGED (was submitted, should still be submitted)
    assert_eq!(
        get_instance_state(&entity_states, "Order", "ord-002").unwrap(),
        "submitted",
        "ord-002 should be unchanged"
    );

    // ord-003 UNCHANGED (was approved, should still be approved)
    assert_eq!(
        get_instance_state(&entity_states, "Order", "ord-003").unwrap(),
        "approved",
        "ord-003 should be unchanged"
    );

    // Provenance: instance_binding should record ord-001
    assert_eq!(
        result
            .provenance
            .instance_binding
            .get("Order")
            .map(|s| s.as_str()),
        Some("ord-001"),
        "provenance.instance_binding should record Order -> ord-001"
    );

    // state_before: ord-001 was draft
    let before_key = ("Order".to_string(), "ord-001".to_string());
    assert_eq!(
        result
            .provenance
            .state_before
            .get(&before_key)
            .map(|s| s.as_str()),
        Some("draft"),
        "state_before should capture draft"
    );

    // state_after: ord-001 is now submitted
    let after_key = ("Order".to_string(), "ord-001".to_string());
    assert_eq!(
        result
            .provenance
            .state_after
            .get(&after_key)
            .map(|s| s.as_str()),
        Some("submitted"),
        "state_after should capture submitted"
    );
}

// ──────────────────────────────────────────────
// Test 4: Flow with instance bindings
// §11.1, §11.4 — flow targets specific instances; provenance traces them
// ──────────────────────────────────────────────

/// Executes a flow with an explicit InstanceBindingMap.
/// Verifies the flow targets the correct instance and provenance records it.
///
/// Setup: ord-001=draft, ord-002=submitted
/// Execute submit_flow targeting ord-001
/// Expected: ord-001→submitted, ord-002 unchanged, step record has ord-001 in instance_bindings
#[test]
fn flow_with_instance_bindings() {
    let submit_flow = simple_flow("submit_flow", "submit_order", "buyer", "submitted");
    let contract = make_order_contract(vec![submit_flow.clone()]);

    let snapshot = Snapshot {
        facts: FactSet::new(),
        verdicts: VerdictSet::new(),
    };

    let mut entity_states = EntityStateMap::new();
    entity_states.insert(
        ("Order".to_string(), "ord-001".to_string()),
        "draft".to_string(),
    );
    entity_states.insert(
        ("Order".to_string(), "ord-002".to_string()),
        "submitted".to_string(),
    );

    // Explicitly target ord-001 for the submit flow
    let mut bindings = InstanceBindingMap::new();
    bindings.insert("Order".to_string(), "ord-001".to_string());

    let result = execute_flow(
        &submit_flow,
        &contract,
        &snapshot,
        &mut entity_states,
        &bindings,
        None,
    )
    .unwrap();

    assert_eq!(result.outcome, "submit_order_done");
    assert_eq!(result.steps_executed.len(), 1);

    // The step record should carry instance_bindings indicating ord-001 was targeted
    let step = &result.steps_executed[0];
    assert_eq!(step.step_type, "operation");
    assert_eq!(
        step.instance_bindings.get("Order").map(|s| s.as_str()),
        Some("ord-001"),
        "step record instance_bindings should show Order -> ord-001"
    );

    // Entity state: ord-001 transitioned, ord-002 unchanged
    assert_eq!(
        get_instance_state(&entity_states, "Order", "ord-001").unwrap(),
        "submitted",
        "ord-001 should be submitted after flow execution"
    );
    assert_eq!(
        get_instance_state(&entity_states, "Order", "ord-002").unwrap(),
        "submitted",
        "ord-002 should be unchanged"
    );

    // Effect record: one change for ord-001
    assert_eq!(result.entity_state_changes.len(), 1);
    assert_eq!(result.entity_state_changes[0].entity_id, "Order");
    assert_eq!(result.entity_state_changes[0].instance_id, "ord-001");
    assert_eq!(result.entity_state_changes[0].from_state, "draft");
    assert_eq!(result.entity_state_changes[0].to_state, "submitted");
}

// ──────────────────────────────────────────────
// Test 5: Missing instance binding → error
// §9.2 — missing binding falls back to DEFAULT_INSTANCE_ID → EntityNotFound
// ──────────────────────────────────────────────

/// When no binding is provided for an entity referenced in effects AND the
/// DEFAULT_INSTANCE_ID instance does not exist in the EntityStateMap, the
/// operation step fails with EntityNotFound. The flow's `on_failure` handler
/// terminates with a failure outcome (not a panic, not a Rust Err).
///
/// This is the "missing instance binding = error" case from A8.
/// Note: the evaluator falls back to "_default" when no binding is provided;
/// if "_default" is not in the EntityStateMap, the operation step fails and
/// `on_failure` routes to a terminal failure outcome. The flow returns Ok but
/// with the failure outcome — this is the correct safe behavior.
///
/// For the direct-operation path (no flow on_failure wrapper), EntityNotFound
/// is returned as Err. See `explicit_missing_binding_returns_entity_not_found`.
#[test]
fn missing_instance_binding_error() {
    // Build a contract with a submit flow
    let submit_flow = simple_flow("submit_flow", "submit_order", "buyer", "submitted");
    let contract = make_order_contract(vec![submit_flow.clone()]);

    let snapshot = Snapshot {
        facts: FactSet::new(),
        verdicts: VerdictSet::new(),
    };

    // Only ord-001 exists, NO _default instance
    let mut entity_states = EntityStateMap::new();
    entity_states.insert(
        ("Order".to_string(), "ord-001".to_string()),
        "draft".to_string(),
    );

    // Empty bindings — falls back to DEFAULT_INSTANCE_ID "_default"
    // But "_default" is not in the map → the operation step fails
    let bindings = InstanceBindingMap::new();

    let result = execute_flow(
        &submit_flow,
        &contract,
        &snapshot,
        &mut entity_states,
        &bindings,
        None,
    );

    // The flow itself does not panic — it handles the error gracefully.
    // The on_failure handler terminates with the failure outcome.
    // This proves: missing binding → clear failure, not panic.
    assert!(
        result.is_ok(),
        "flow should handle missing binding gracefully via on_failure"
    );

    let flow_result = result.unwrap();
    assert_eq!(
        flow_result.outcome, "submit_order_failed",
        "flow should terminate with failure outcome when targeted instance is absent"
    );

    // ord-001 should be UNCHANGED (operation was never applied)
    assert_eq!(
        get_instance_state(&entity_states, "Order", "ord-001").unwrap(),
        "draft",
        "ord-001 should be untouched after failed attempt on missing _default instance"
    );
}

/// Direct operation test: explicitly provide a nonexistent binding → EntityNotFound.
#[test]
fn explicit_missing_binding_returns_entity_not_found() {
    let submit_op = make_operation(
        "submit_order",
        vec!["buyer"],
        vec![Effect {
            entity_id: "Order".to_string(),
            from: "draft".to_string(),
            to: "submitted".to_string(),
            outcome: None,
        }],
        vec!["submitted"],
    );

    let facts = FactSet::new();
    let verdicts = VerdictSet::new();

    // Only ord-001 in the map
    let mut entity_states = EntityStateMap::new();
    entity_states.insert(
        ("Order".to_string(), "ord-001".to_string()),
        "draft".to_string(),
    );

    // Bind to ord-999 which does not exist
    let mut bindings = InstanceBindingMap::new();
    bindings.insert("Order".to_string(), "ord-999".to_string());

    let result = execute_operation(
        &submit_op,
        "buyer",
        &facts,
        &verdicts,
        &mut entity_states,
        &bindings,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        OperationError::EntityNotFound {
            entity_id,
            instance_id,
        } => {
            assert_eq!(entity_id, "Order");
            assert_eq!(instance_id, "ord-999");
        }
        other => panic!("expected EntityNotFound, got {:?}", other),
    }
}

// ──────────────────────────────────────────────
// Test 6: Single-instance degenerate case
// §6.5 — one instance per entity with "_default" behaves as pre-multi-instance
// ──────────────────────────────────────────────

/// Verifies that single_instance() produces the same results as before the
/// multi-instance change. This is the backward compatibility proof.
///
/// Expected: empty InstanceBindingMap + single_instance() EntityStateMap →
/// operations behave exactly as they did before plan 04-01.
#[test]
fn single_instance_degenerate_case() {
    let submit_op = make_operation(
        "submit_order",
        vec!["buyer"],
        vec![Effect {
            entity_id: "Order".to_string(),
            from: "draft".to_string(),
            to: "submitted".to_string(),
            outcome: None,
        }],
        vec!["submitted"],
    );

    let facts = FactSet::new();
    let verdicts = VerdictSet::new();

    // Use single_instance() helper — creates ("Order", "_default") → "draft"
    let mut entity_states = single_instance(
        [("Order".to_string(), "draft".to_string())]
            .into_iter()
            .collect(),
    );

    // Empty bindings (pre-multi-instance behavior)
    let result = execute_operation(
        &submit_op,
        "buyer",
        &facts,
        &verdicts,
        &mut entity_states,
        &InstanceBindingMap::new(),
    )
    .unwrap();

    // Same behavior as before: succeeds, transitions the default instance
    assert_eq!(result.outcome, "submitted");
    assert_eq!(result.effects_applied.len(), 1);
    assert_eq!(
        result.effects_applied[0].instance_id, DEFAULT_INSTANCE_ID,
        "single-instance uses _default as instance_id"
    );
    assert_eq!(result.effects_applied[0].from_state, "draft");
    assert_eq!(result.effects_applied[0].to_state, "submitted");

    // Entity state updated for the default instance
    assert_eq!(
        get_instance_state(&entity_states, "Order", DEFAULT_INSTANCE_ID).unwrap(),
        "submitted",
        "default instance should be in submitted state"
    );

    // Provenance: instance_binding records _default
    assert_eq!(
        result
            .provenance
            .instance_binding
            .get("Order")
            .map(|s| s.as_str()),
        Some(DEFAULT_INSTANCE_ID),
        "provenance.instance_binding should record _default for single-instance case"
    );
}

/// Verify single-instance action space: single Order/_default → same as old behavior.
#[test]
fn single_instance_action_space_backward_compat() {
    let bundle = order_action_space_bundle();
    let contract = Contract::from_interchange(&bundle).unwrap();
    let facts = serde_json::json!({});

    // Single-instance via helper: Order/_default = draft
    let entity_states = single_instance(
        [("Order".to_string(), "draft".to_string())]
            .into_iter()
            .collect(),
    );

    let result = compute_action_space(&contract, &facts, &entity_states, "buyer").unwrap();

    // submit_flow should be available
    let submit_action = result.actions.iter().find(|a| a.flow_id == "submit_flow");
    assert!(
        submit_action.is_some(),
        "submit_flow should be available for buyer with draft order"
    );

    // instance_bindings should contain _default (not some new key)
    let bindings = &submit_action.unwrap().instance_bindings;
    assert!(
        bindings["Order"].contains(DEFAULT_INSTANCE_ID),
        "single-instance contract should use _default in instance_bindings"
    );
    assert_eq!(
        bindings["Order"].len(),
        1,
        "exactly one instance for single-instance contract"
    );
}

/// Verify single-instance flow execution is identical to pre-change behavior.
#[test]
fn single_instance_flow_backward_compat() {
    let submit_flow = simple_flow("submit_flow", "submit_order", "buyer", "submitted");
    let contract = make_order_contract(vec![submit_flow.clone()]);

    let snapshot = Snapshot {
        facts: FactSet::new(),
        verdicts: VerdictSet::new(),
    };

    let mut entity_states = single_instance(
        [("Order".to_string(), "draft".to_string())]
            .into_iter()
            .collect(),
    );

    // Empty bindings — uses _default per backward compat
    let result = execute_flow(
        &submit_flow,
        &contract,
        &snapshot,
        &mut entity_states,
        &InstanceBindingMap::new(),
        None,
    )
    .unwrap();

    assert_eq!(result.outcome, "submit_order_done");
    assert_eq!(result.entity_state_changes.len(), 1);
    assert_eq!(
        result.entity_state_changes[0].instance_id,
        DEFAULT_INSTANCE_ID
    );

    // Default instance now in submitted state
    assert_eq!(
        get_instance_state(&entity_states, "Order", DEFAULT_INSTANCE_ID).unwrap(),
        "submitted"
    );
}

// ──────────────────────────────────────────────
// Test 7: Instance absence
// §6.5 — instance not in EntityStateMap = doesn't exist
// ──────────────────────────────────────────────

/// Verifies that attempting to execute an operation on an instance that is not
/// present in EntityStateMap returns a clear error — not a panic, not a silent
/// fallback to another instance.
///
/// This is the "absent instances are non-existent" guarantee per §6.5.
#[test]
fn instance_absence() {
    let submit_op = make_operation(
        "submit_order",
        vec!["buyer"],
        vec![Effect {
            entity_id: "Order".to_string(),
            from: "draft".to_string(),
            to: "submitted".to_string(),
            outcome: None,
        }],
        vec!["submitted"],
    );

    let facts = FactSet::new();
    let verdicts = VerdictSet::new();

    // Only ord-001 exists in EntityStateMap
    let mut entity_states = EntityStateMap::new();
    entity_states.insert(
        ("Order".to_string(), "ord-001".to_string()),
        "draft".to_string(),
    );

    // Attempt to target ord-999 — not in EntityStateMap
    let mut bindings = InstanceBindingMap::new();
    bindings.insert("Order".to_string(), "ord-999".to_string());

    let result = execute_operation(
        &submit_op,
        "buyer",
        &facts,
        &verdicts,
        &mut entity_states,
        &bindings,
    );

    // Must fail with EntityNotFound — ord-999 does not exist per §6.5
    assert!(
        result.is_err(),
        "executing on absent instance should return error, not succeed"
    );

    match result.unwrap_err() {
        OperationError::EntityNotFound {
            entity_id,
            instance_id,
        } => {
            assert_eq!(entity_id, "Order", "error should name the entity");
            assert_eq!(
                instance_id, "ord-999",
                "error should name the absent instance"
            );
        }
        other => panic!(
            "expected EntityNotFound for absent instance, got {:?}",
            other
        ),
    }

    // ord-001 should be UNCHANGED (no side effects from failed attempt)
    assert_eq!(
        get_instance_state(&entity_states, "Order", "ord-001").unwrap(),
        "draft",
        "existing ord-001 should be untouched after failed operation on ord-999"
    );
}

/// Verifies that querying action space with an instance not in EntityStateMap
/// correctly excludes that instance from available actions.
///
/// When no instances of a required entity are in the correct source state,
/// the action is blocked.
#[test]
fn absent_instance_not_considered_for_action_space() {
    let bundle = order_action_space_bundle();
    let contract = Contract::from_interchange(&bundle).unwrap();
    let facts = serde_json::json!({});

    // EntityStateMap only has ord-002 (submitted) — no draft instances
    let mut entity_states = EntityStateMap::new();
    entity_states.insert(
        ("Order".to_string(), "ord-002".to_string()),
        "submitted".to_string(),
    );

    // For buyer: submit_flow requires Order in "draft" — no draft instance exists
    let result = compute_action_space(&contract, &facts, &entity_states, "buyer").unwrap();

    let submit_action = result.actions.iter().find(|a| a.flow_id == "submit_flow");
    assert!(
        submit_action.is_none(),
        "submit_flow should NOT be available when no Order is in draft state"
    );

    assert!(
        !result.blocked_actions.is_empty(),
        "submit_flow should appear in blocked_actions"
    );

    let submit_blocked = result
        .blocked_actions
        .iter()
        .find(|a| a.flow_id == "submit_flow");
    assert!(
        submit_blocked.is_some(),
        "submit_flow should be in blocked_actions"
    );

    match &submit_blocked.unwrap().reason {
        BlockedReason::EntityNotInSourceState {
            entity_id,
            required_state,
            ..
        } => {
            assert_eq!(entity_id, "Order");
            assert_eq!(required_state, "draft");
        }
        other => panic!("expected EntityNotInSourceState, got {:?}", other),
    }
}

// ──────────────────────────────────────────────
// Additional: state_before/state_after provenance
// §9.5 — instance-scoped state snapshots
// ──────────────────────────────────────────────

/// Verifies that when executing an operation on a specific instance,
/// state_before and state_after in provenance are scoped to that instance.
#[test]
fn operation_provenance_scoped_to_targeted_instance() {
    let approve_op = make_operation(
        "approve_order",
        vec!["admin"],
        vec![Effect {
            entity_id: "Order".to_string(),
            from: "submitted".to_string(),
            to: "approved".to_string(),
            outcome: None,
        }],
        vec!["approved"],
    );

    let facts = FactSet::new();
    let verdicts = VerdictSet::new();

    let mut entity_states = EntityStateMap::new();
    entity_states.insert(
        ("Order".to_string(), "ord-A".to_string()),
        "submitted".to_string(),
    );
    entity_states.insert(
        ("Order".to_string(), "ord-B".to_string()),
        "submitted".to_string(),
    );

    // Target ord-A
    let mut bindings = InstanceBindingMap::new();
    bindings.insert("Order".to_string(), "ord-A".to_string());

    let result = execute_operation(
        &approve_op,
        "admin",
        &facts,
        &verdicts,
        &mut entity_states,
        &bindings,
    )
    .unwrap();

    // instance_binding records ord-A
    assert_eq!(
        result
            .provenance
            .instance_binding
            .get("Order")
            .map(|s| s.as_str()),
        Some("ord-A")
    );

    // state_before: ord-A was submitted
    assert_eq!(
        result
            .provenance
            .state_before
            .get(&("Order".to_string(), "ord-A".to_string()))
            .map(|s| s.as_str()),
        Some("submitted"),
        "state_before should capture ord-A pre-transition state"
    );

    // state_after: ord-A is now approved
    assert_eq!(
        result
            .provenance
            .state_after
            .get(&("Order".to_string(), "ord-A".to_string()))
            .map(|s| s.as_str()),
        Some("approved"),
        "state_after should capture ord-A post-transition state"
    );

    // ord-B is NOT in state_before or state_after (untouched)
    assert!(
        !result
            .provenance
            .state_before
            .contains_key(&("Order".to_string(), "ord-B".to_string())),
        "state_before should NOT include untouched ord-B"
    );

    // ord-A transitioned; ord-B unchanged
    assert_eq!(
        get_instance_state(&entity_states, "Order", "ord-A").unwrap(),
        "approved"
    );
    assert_eq!(
        get_instance_state(&entity_states, "Order", "ord-B").unwrap(),
        "submitted"
    );
}

// ──────────────────────────────────────────────
// Test: Precondition failure on targeted instance
// ──────────────────────────────────────────────

/// Verifies that executing an operation whose precondition evaluates to false
/// returns PreconditionFailed, even when the targeted instance is in the
/// correct entity state and the persona is authorized.
///
/// This exercises the precondition check path in execute_operation for the
/// multi-instance case: the operation is structurally valid (correct entity
/// state, correct persona) but the business-logic precondition is not met.
#[test]
fn precondition_failure_on_targeted_instance() {
    // Build an operation with a FALSE precondition (always fails)
    let submit_op = Operation {
        id: "submit_order".to_string(),
        allowed_personas: vec!["buyer".to_string()],
        precondition: Predicate::Literal {
            value: Value::Bool(false),
            type_spec: bool_type(),
        },
        effects: vec![Effect {
            entity_id: "Order".to_string(),
            from: "draft".to_string(),
            to: "submitted".to_string(),
            outcome: None,
        }],
        error_contract: vec![],
        outcomes: vec!["submitted".to_string()],
    };

    let facts = FactSet::new();
    let verdicts = VerdictSet::new();

    let mut entity_states = EntityStateMap::new();
    entity_states.insert(
        ("Order".to_string(), "ord-001".to_string()),
        "draft".to_string(),
    );

    // Target ord-001 (correct state, authorized persona, but precondition is false)
    let mut bindings = InstanceBindingMap::new();
    bindings.insert("Order".to_string(), "ord-001".to_string());

    let result = execute_operation(
        &submit_op,
        "buyer",
        &facts,
        &verdicts,
        &mut entity_states,
        &bindings,
    );

    assert!(
        result.is_err(),
        "operation should fail when precondition evaluates to false"
    );

    match result.unwrap_err() {
        OperationError::PreconditionFailed {
            operation_id,
            condition_desc,
        } => {
            assert_eq!(
                operation_id, "submit_order",
                "error should name the operation"
            );
            assert!(
                !condition_desc.is_empty(),
                "condition_desc should describe the failure"
            );
        }
        other => panic!("expected PreconditionFailed, got {:?}", other),
    }

    // Entity state must be unchanged (no side effects from failed precondition)
    assert_eq!(
        get_instance_state(&entity_states, "Order", "ord-001").unwrap(),
        "draft",
        "ord-001 should remain in draft after precondition failure"
    );
}

// ──────────────────────────────────────────────
// Test: Unauthorized persona on instance operation
// ──────────────────────────────────────────────

/// Verifies that executing an operation with a persona not in the operation's
/// allowed_personas list returns PersonaRejected, even when the targeted
/// instance is in the correct entity state and the precondition would pass.
///
/// This exercises the persona authorization check in execute_operation for the
/// multi-instance case: structurally valid (correct entity state, true
/// precondition) but the caller is not authorized.
#[test]
fn unauthorized_persona_on_instance_operation() {
    let submit_op = make_operation(
        "submit_order",
        vec!["buyer"],
        vec![Effect {
            entity_id: "Order".to_string(),
            from: "draft".to_string(),
            to: "submitted".to_string(),
            outcome: None,
        }],
        vec!["submitted"],
    );

    let facts = FactSet::new();
    let verdicts = VerdictSet::new();

    let mut entity_states = EntityStateMap::new();
    entity_states.insert(
        ("Order".to_string(), "ord-001".to_string()),
        "draft".to_string(),
    );

    // Target ord-001 (correct state, true precondition)
    let mut bindings = InstanceBindingMap::new();
    bindings.insert("Order".to_string(), "ord-001".to_string());

    // Execute as "admin" — NOT in allowed_personas (only "buyer" is allowed)
    let result = execute_operation(
        &submit_op,
        "admin",
        &facts,
        &verdicts,
        &mut entity_states,
        &bindings,
    );

    assert!(
        result.is_err(),
        "operation should fail when persona is not authorized"
    );

    match result.unwrap_err() {
        OperationError::PersonaRejected {
            operation_id,
            persona,
        } => {
            assert_eq!(
                operation_id, "submit_order",
                "error should name the operation"
            );
            assert_eq!(persona, "admin", "error should name the rejected persona");
        }
        other => panic!("expected PersonaRejected, got {:?}", other),
    }

    // Entity state must be unchanged (no side effects from unauthorized attempt)
    assert_eq!(
        get_instance_state(&entity_states, "Order", "ord-001").unwrap(),
        "draft",
        "ord-001 should remain in draft after persona rejection"
    );
}
