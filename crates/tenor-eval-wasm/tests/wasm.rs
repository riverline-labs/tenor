use wasm_bindgen_test::*;

// Uses the entity_operation_basic conformance fixture — it has
// Fact, Entity, Rule, Operation, and Flow constructs.
const BASIC_BUNDLE: &str = r#"{
  "constructs": [
    {
      "id": "is_active",
      "kind": "Fact",
      "provenance": { "file": "test.tenor", "line": 11 },
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
      "provenance": { "file": "test.tenor", "line": 16 },
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
      "provenance": { "file": "test.tenor", "line": 22 },
      "tenor": "1.0"
    },
    {
      "entry": "step_approve",
      "id": "approval_flow",
      "kind": "Flow",
      "provenance": { "file": "test.tenor", "line": 29 },
      "snapshot": "at_initiation",
      "steps": [
        {
          "id": "step_approve",
          "kind": "OperationStep",
          "on_failure": { "kind": "Terminate", "outcome": "approval_failed" },
          "op": "approve_order",
          "outcomes": {
            "success": { "kind": "Terminal", "outcome": "order_approved" }
          },
          "persona": "admin"
        }
      ],
      "tenor": "1.0"
    }
  ],
  "id": "entity_operation_basic",
  "kind": "Bundle",
  "tenor": "1.0",
  "tenor_version": "1.1.0"
}"#;

#[wasm_bindgen_test(unsupported = test)]
fn test_load_contract_success() {
    let result = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("handle").is_some(), "expected handle in: {}", result);
    assert!(parsed.get("error").is_none(), "unexpected error: {}", result);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_load_contract_invalid_json() {
    let result = tenor_eval_wasm::load_contract("not json");
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("error").is_some(), "expected error in: {}", result);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_load_contract_invalid_bundle() {
    let result = tenor_eval_wasm::load_contract(r#"{"not": "a bundle"}"#);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("error").is_some(), "expected error in: {}", result);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_evaluate_produces_verdicts() {
    let load_result = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let handle = serde_json::from_str::<serde_json::Value>(&load_result)
        .unwrap()["handle"]
        .as_u64()
        .unwrap() as u32;

    let result = tenor_eval_wasm::evaluate(handle, r#"{"is_active": true}"#);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let verdicts = parsed["verdicts"].as_array().expect("expected verdicts array");
    assert_eq!(verdicts.len(), 1);
    assert_eq!(verdicts[0]["type"], "account_active");

    // Verify provenance is present and complete
    let prov = &verdicts[0]["provenance"];
    assert_eq!(prov["rule"], "check_active");
    assert_eq!(prov["stratum"], 0);
    assert!(prov["facts_used"].as_array().unwrap().contains(&serde_json::json!("is_active")));
}

#[wasm_bindgen_test(unsupported = test)]
fn test_evaluate_no_verdict_when_false() {
    let load_result = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let handle = serde_json::from_str::<serde_json::Value>(&load_result)
        .unwrap()["handle"]
        .as_u64()
        .unwrap() as u32;

    let result = tenor_eval_wasm::evaluate(handle, r#"{"is_active": false}"#);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let verdicts = parsed["verdicts"].as_array().expect("expected verdicts array");
    assert_eq!(verdicts.len(), 0);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_evaluate_missing_required_fact() {
    let load_result = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let handle = serde_json::from_str::<serde_json::Value>(&load_result)
        .unwrap()["handle"]
        .as_u64()
        .unwrap() as u32;

    let result = tenor_eval_wasm::evaluate(handle, r#"{}"#);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("error").is_some(), "expected error for missing fact: {}", result);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_evaluate_invalid_handle() {
    let result = tenor_eval_wasm::evaluate(9999, r#"{"is_active": true}"#);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("error").is_some(), "expected error for invalid handle: {}", result);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_simulate_flow_success() {
    let load_result = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let handle = serde_json::from_str::<serde_json::Value>(&load_result)
        .unwrap()["handle"]
        .as_u64()
        .unwrap() as u32;

    let result = tenor_eval_wasm::simulate_flow(
        handle,
        "approval_flow",
        "admin",
        r#"{"is_active": true}"#,
        r#"{}"#,
    );
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(parsed["simulation"], true);
    assert_eq!(parsed["flow_id"], "approval_flow");
    assert_eq!(parsed["outcome"], "order_approved");
    assert!(parsed["path"].as_array().unwrap().len() > 0);
    assert!(parsed["would_transition"].as_array().unwrap().len() > 0);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_simulate_flow_precondition_fails() {
    let load_result = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let handle = serde_json::from_str::<serde_json::Value>(&load_result)
        .unwrap()["handle"]
        .as_u64()
        .unwrap() as u32;

    let result = tenor_eval_wasm::simulate_flow(
        handle,
        "approval_flow",
        "admin",
        r#"{"is_active": false}"#,
        r#"{}"#,
    );
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(parsed["simulation"], true);
    assert_eq!(parsed["outcome"], "approval_failed");
}

#[wasm_bindgen_test(unsupported = test)]
fn test_simulate_flow_initiating_persona_is_provenance_only() {
    // Per spec Section 11.4: initiating_persona is recorded for provenance.
    // Flow-level persona authorization is delegated to step-level Operation
    // persona checks. The step's persona ("admin") is used for the operation,
    // not the initiating persona ("guest"), so the flow still succeeds.
    let load_result = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let handle = serde_json::from_str::<serde_json::Value>(&load_result)
        .unwrap()["handle"]
        .as_u64()
        .unwrap() as u32;

    let result = tenor_eval_wasm::simulate_flow(
        handle,
        "approval_flow",
        "guest",
        r#"{"is_active": true}"#,
        r#"{}"#,
    );
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(parsed["simulation"], true);
    // Step-level persona is "admin" (from the step definition), so the
    // operation succeeds regardless of the initiating persona.
    assert_eq!(parsed["outcome"], "order_approved");
}

#[wasm_bindgen_test(unsupported = test)]
fn test_simulate_flow_not_found() {
    let load_result = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let handle = serde_json::from_str::<serde_json::Value>(&load_result)
        .unwrap()["handle"]
        .as_u64()
        .unwrap() as u32;

    let result = tenor_eval_wasm::simulate_flow(
        handle,
        "nonexistent_flow",
        "admin",
        r#"{"is_active": true}"#,
        r#"{}"#,
    );
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("error").is_some(), "expected error for missing flow: {}", result);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_inspect_contract() {
    let load_result = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let handle = serde_json::from_str::<serde_json::Value>(&load_result)
        .unwrap()["handle"]
        .as_u64()
        .unwrap() as u32;

    let result = tenor_eval_wasm::inspect_contract(handle);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(!parsed["facts"].as_array().unwrap().is_empty());
    assert!(!parsed["entities"].as_array().unwrap().is_empty());
    assert!(!parsed["rules"].as_array().unwrap().is_empty());
    assert!(!parsed["operations"].as_array().unwrap().is_empty());
    assert!(!parsed["flows"].as_array().unwrap().is_empty());

    let fact = &parsed["facts"][0];
    assert_eq!(fact["id"], "is_active");
    assert_eq!(fact["type"], "Bool");

    let entity = &parsed["entities"][0];
    assert_eq!(entity["id"], "Order");
    assert_eq!(entity["initial"], "pending");
}

#[wasm_bindgen_test(unsupported = test)]
fn test_inspect_invalid_handle() {
    let result = tenor_eval_wasm::inspect_contract(9999);
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("error").is_some());
}

#[wasm_bindgen_test(unsupported = test)]
fn test_free_and_reuse() {
    let r1 = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let h1 = serde_json::from_str::<serde_json::Value>(&r1).unwrap()["handle"]
        .as_u64().unwrap() as u32;

    let r2 = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let h2 = serde_json::from_str::<serde_json::Value>(&r2).unwrap()["handle"]
        .as_u64().unwrap() as u32;

    assert!(serde_json::from_str::<serde_json::Value>(
        &tenor_eval_wasm::evaluate(h1, r#"{"is_active": true}"#)
    ).unwrap().get("verdicts").is_some());

    assert!(serde_json::from_str::<serde_json::Value>(
        &tenor_eval_wasm::evaluate(h2, r#"{"is_active": true}"#)
    ).unwrap().get("verdicts").is_some());

    tenor_eval_wasm::free_contract(h1);

    let freed_result = serde_json::from_str::<serde_json::Value>(
        &tenor_eval_wasm::evaluate(h1, r#"{"is_active": true}"#)
    ).unwrap();
    assert!(freed_result.get("error").is_some(), "freed handle should error");

    let still_valid = serde_json::from_str::<serde_json::Value>(
        &tenor_eval_wasm::evaluate(h2, r#"{"is_active": true}"#)
    ).unwrap();
    assert!(still_valid.get("verdicts").is_some(), "other handle should still work");
}

#[wasm_bindgen_test(unsupported = test)]
fn test_free_invalid_handle_is_noop() {
    tenor_eval_wasm::free_contract(9999);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_compute_action_space_available() {
    let load_result = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let handle = serde_json::from_str::<serde_json::Value>(&load_result)
        .unwrap()["handle"]
        .as_u64()
        .unwrap() as u32;

    let result = tenor_eval_wasm::compute_action_space(
        handle,
        r#"{"is_active": true}"#,
        r#"{"Order": "pending"}"#,
        "admin",
    );
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed.get("error").is_none(), "unexpected error: {}", result);
    assert_eq!(parsed["persona_id"], "admin");

    let actions = parsed["actions"].as_array().unwrap();
    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0]["flow_id"], "approval_flow");
    assert_eq!(actions[0]["entry_operation_id"], "approve_order");

    let blocked = parsed["blocked_actions"].as_array().unwrap();
    assert_eq!(blocked.len(), 0);
}

#[wasm_bindgen_test(unsupported = test)]
fn test_compute_action_space_blocked_persona() {
    let load_result = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let handle = serde_json::from_str::<serde_json::Value>(&load_result)
        .unwrap()["handle"]
        .as_u64()
        .unwrap() as u32;

    let result = tenor_eval_wasm::compute_action_space(
        handle,
        r#"{"is_active": true}"#,
        r#"{"Order": "pending"}"#,
        "guest",
    );
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed.get("error").is_none(), "unexpected error: {}", result);
    let actions = parsed["actions"].as_array().unwrap();
    assert_eq!(actions.len(), 0);

    let blocked = parsed["blocked_actions"].as_array().unwrap();
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0]["reason"]["type"], "PersonaNotAuthorized");
}

#[wasm_bindgen_test(unsupported = test)]
fn test_compute_action_space_blocked_precondition() {
    let load_result = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let handle = serde_json::from_str::<serde_json::Value>(&load_result)
        .unwrap()["handle"]
        .as_u64()
        .unwrap() as u32;

    let result = tenor_eval_wasm::compute_action_space(
        handle,
        r#"{"is_active": false}"#,
        r#"{"Order": "pending"}"#,
        "admin",
    );
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed.get("error").is_none(), "unexpected error: {}", result);
    let actions = parsed["actions"].as_array().unwrap();
    assert_eq!(actions.len(), 0);

    let blocked = parsed["blocked_actions"].as_array().unwrap();
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0]["reason"]["type"], "PreconditionNotMet");
    let missing = blocked[0]["reason"]["missing_verdicts"].as_array().unwrap();
    assert!(missing.contains(&serde_json::json!("account_active")));
}

#[wasm_bindgen_test(unsupported = test)]
fn test_compute_action_space_blocked_entity_state() {
    let load_result = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let handle = serde_json::from_str::<serde_json::Value>(&load_result)
        .unwrap()["handle"]
        .as_u64()
        .unwrap() as u32;

    let result = tenor_eval_wasm::compute_action_space(
        handle,
        r#"{"is_active": true}"#,
        r#"{"Order": "approved"}"#,
        "admin",
    );
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert!(parsed.get("error").is_none(), "unexpected error: {}", result);
    let actions = parsed["actions"].as_array().unwrap();
    assert_eq!(actions.len(), 0);

    let blocked = parsed["blocked_actions"].as_array().unwrap();
    assert_eq!(blocked.len(), 1);
    assert_eq!(blocked[0]["reason"]["type"], "EntityNotInSourceState");
}

#[wasm_bindgen_test(unsupported = test)]
fn test_compute_action_space_invalid_handle() {
    let result = tenor_eval_wasm::compute_action_space(
        9999,
        r#"{"is_active": true}"#,
        r#"{}"#,
        "admin",
    );
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("error").is_some());
}

#[wasm_bindgen_test(unsupported = test)]
fn test_compute_action_space_current_verdicts() {
    let load_result = tenor_eval_wasm::load_contract(BASIC_BUNDLE);
    let handle = serde_json::from_str::<serde_json::Value>(&load_result)
        .unwrap()["handle"]
        .as_u64()
        .unwrap() as u32;

    let result = tenor_eval_wasm::compute_action_space(
        handle,
        r#"{"is_active": true}"#,
        r#"{}"#,
        "admin",
    );
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    let verdicts = parsed["current_verdicts"].as_array().unwrap();
    assert_eq!(verdicts.len(), 1);
    assert_eq!(verdicts[0]["verdict_type"], "account_active");
    assert_eq!(verdicts[0]["producing_rule"], "check_active");
}
