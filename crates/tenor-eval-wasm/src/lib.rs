use slab::Slab;
use std::cell::RefCell;
use std::collections::BTreeMap;
use tenor_eval::Contract;
use wasm_bindgen::prelude::*;

mod inspect;

struct StoredContract {
    contract: Contract,
    bundle: serde_json::Value,
}

thread_local! {
    static CONTRACTS: RefCell<Slab<StoredContract>> = const { RefCell::new(Slab::new()) };
}

fn error_json(msg: &str) -> String {
    serde_json::json!({ "error": msg }).to_string()
}

fn with_contract<F>(handle: u32, f: F) -> String
where
    F: FnOnce(&StoredContract) -> String,
{
    CONTRACTS.with(|contracts| {
        let contracts = contracts.borrow();
        match contracts.get(handle as usize) {
            Some(stored) => f(stored),
            None => error_json(&format!("invalid contract handle: {}", handle)),
        }
    })
}

/// Parse entity_states JSON with auto-detection of old and new formats.
///
/// Old format (single-instance, flat):
/// ```json
/// { "Order": "draft", "DeliveryRecord": "pending" }
/// ```
///
/// New format (multi-instance, nested):
/// ```json
/// {
///   "Order": { "ord-001": "draft", "ord-002": "submitted" },
///   "DeliveryRecord": { "del-001": "pending" }
/// }
/// ```
///
/// Mixed format is supported: per entity_id, if the value is a string it is
/// treated as old format (converted to _default instance), if it is an object
/// it is treated as new format (parsed directly).
///
/// Per plan-4.md §A9: detect by checking whether values are strings (old) or
/// objects (new). Convert old format by assigning DEFAULT_INSTANCE_ID.
fn parse_entity_states(json: &serde_json::Value) -> Result<tenor_eval::EntityStateMap, String> {
    let obj = match json.as_object() {
        Some(o) => o,
        None => return Err("entity_states must be a JSON object".to_string()),
    };

    let mut entity_states = tenor_eval::EntityStateMap::new();

    for (entity_id, value) in obj {
        if let Some(state_str) = value.as_str() {
            // Old format: entity_id -> state string; convert to single instance
            entity_states.insert(
                (
                    entity_id.clone(),
                    tenor_eval::DEFAULT_INSTANCE_ID.to_string(),
                ),
                state_str.to_string(),
            );
        } else if let Some(instance_map) = value.as_object() {
            // New format: entity_id -> { instance_id -> state }
            for (instance_id, state_val) in instance_map {
                let state_str = match state_val.as_str() {
                    Some(s) => s,
                    None => {
                        return Err(format!(
                            "entity_states[{}][{}] must be a string state",
                            entity_id, instance_id
                        ))
                    }
                };
                entity_states.insert(
                    (entity_id.clone(), instance_id.clone()),
                    state_str.to_string(),
                );
            }
        } else {
            return Err(format!(
                "entity_states[{}] must be a string (old format) or object (new format)",
                entity_id
            ));
        }
    }

    Ok(entity_states)
}

/// Parse instance_bindings JSON: entity_id -> instance_id.
///
/// Format: `{ "Order": "ord-001", "DeliveryRecord": "del-001" }`
///
/// Returns an empty map if the JSON is null, empty object, or empty string.
fn parse_instance_bindings(json_str: &str) -> Result<tenor_eval::InstanceBindingMap, String> {
    if json_str.trim().is_empty() || json_str.trim() == "null" || json_str.trim() == "{}" {
        return Ok(BTreeMap::new());
    }

    let val: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("invalid instance_bindings JSON: {}", e))?;

    match val {
        serde_json::Value::Null => Ok(BTreeMap::new()),
        serde_json::Value::Object(map) => {
            let mut result = BTreeMap::new();
            for (entity_id, instance_val) in map {
                let instance_id = match instance_val.as_str() {
                    Some(s) => s.to_string(),
                    None => {
                        return Err(format!(
                            "instance_bindings[{}] must be a string instance_id",
                            entity_id
                        ))
                    }
                };
                result.insert(entity_id, instance_id);
            }
            Ok(result)
        }
        _ => Err("instance_bindings must be a JSON object or null".to_string()),
    }
}

#[wasm_bindgen]
pub fn load_contract(interchange_json: &str) -> String {
    let bundle: serde_json::Value = match serde_json::from_str(interchange_json) {
        Ok(v) => v,
        Err(e) => return error_json(&format!("invalid JSON: {}", e)),
    };

    let contract = match Contract::from_interchange(&bundle) {
        Ok(c) => c,
        Err(e) => return error_json(&format!("invalid contract: {}", e)),
    };

    let handle = CONTRACTS.with(|contracts| {
        contracts
            .borrow_mut()
            .insert(StoredContract { contract, bundle })
    });

    serde_json::json!({ "handle": handle }).to_string()
}

#[wasm_bindgen]
pub fn free_contract(handle: u32) {
    CONTRACTS.with(|contracts| {
        let mut contracts = contracts.borrow_mut();
        if contracts.contains(handle as usize) {
            contracts.remove(handle as usize);
        }
    });
}

#[wasm_bindgen]
pub fn evaluate(handle: u32, facts_json: &str) -> String {
    let facts: serde_json::Value = match serde_json::from_str(facts_json) {
        Ok(v) => v,
        Err(e) => return error_json(&format!("invalid facts JSON: {}", e)),
    };

    with_contract(handle, |stored| {
        let fact_set = match tenor_eval::assemble::assemble_facts(&stored.contract, &facts) {
            Ok(fs) => fs,
            Err(e) => return error_json(&format!("fact assembly error: {}", e)),
        };

        let verdict_set = match tenor_eval::rules::eval_strata(&stored.contract, &fact_set) {
            Ok(vs) => vs,
            Err(e) => return error_json(&format!("evaluation error: {}", e)),
        };

        verdict_set.to_json().to_string()
    })
}

#[wasm_bindgen]
pub fn simulate_flow(
    handle: u32,
    flow_id: &str,
    persona_id: &str,
    facts_json: &str,
    entity_states_json: &str,
) -> String {
    simulate_flow_with_bindings(
        handle,
        flow_id,
        persona_id,
        facts_json,
        entity_states_json,
        "",
    )
}

/// Extended simulate_flow that accepts instance_bindings.
///
/// `entity_states_json` accepts both old flat format and new nested format.
/// `instance_bindings_json` maps entity_id → instance_id; if empty/null, uses _default.
#[wasm_bindgen]
pub fn simulate_flow_with_bindings(
    handle: u32,
    flow_id: &str,
    persona_id: &str,
    facts_json: &str,
    entity_states_json: &str,
    instance_bindings_json: &str,
) -> String {
    let facts: serde_json::Value = match serde_json::from_str(facts_json) {
        Ok(v) => v,
        Err(e) => return error_json(&format!("invalid facts JSON: {}", e)),
    };

    let entity_states_val: serde_json::Value = match serde_json::from_str(entity_states_json) {
        Ok(v) => v,
        Err(e) => return error_json(&format!("invalid entity states JSON: {}", e)),
    };

    let instance_bindings = match parse_instance_bindings(instance_bindings_json) {
        Ok(b) => b,
        Err(e) => return error_json(&e),
    };

    with_contract(handle, |stored| {
        let entity_states = match parse_entity_states(&entity_states_val) {
            Ok(s) => s,
            Err(e) => return error_json(&format!("invalid entity states: {}", e)),
        };

        let fact_set = match tenor_eval::assemble::assemble_facts(&stored.contract, &facts) {
            Ok(fs) => fs,
            Err(e) => return error_json(&format!("fact assembly error: {}", e)),
        };

        let verdict_set = match tenor_eval::rules::eval_strata(&stored.contract, &fact_set) {
            Ok(vs) => vs,
            Err(e) => return error_json(&format!("evaluation error: {}", e)),
        };

        let snapshot = tenor_eval::Snapshot {
            facts: fact_set,
            verdicts: verdict_set.clone(),
        };

        // Merge contract defaults with provided states
        let mut merged_entity_states = tenor_eval::operation::init_entity_states(&stored.contract);
        for (key, state) in entity_states {
            merged_entity_states.insert(key, state);
        }

        let target_flow = match stored.contract.get_flow(flow_id) {
            Some(f) => f,
            None => return error_json(&format!("flow '{}' not found", flow_id)),
        };

        let flow_result = match tenor_eval::flow::execute_flow(
            target_flow,
            &stored.contract,
            &snapshot,
            &mut merged_entity_states,
            &instance_bindings,
            None,
        ) {
            Ok(r) => r,
            Err(e) => return error_json(&format!("flow execution error: {}", e)),
        };

        let path: Vec<serde_json::Value> = flow_result
            .steps_executed
            .iter()
            .map(|s| {
                let mut step_json = serde_json::json!({
                    "step_id": s.step_id,
                    "step_type": s.step_type,
                    "result": s.result,
                });
                // Include instance_bindings on each step if non-empty
                if !s.instance_bindings.is_empty() {
                    step_json["instance_bindings"] = serde_json::to_value(&s.instance_bindings)
                        .unwrap_or(serde_json::Value::Null);
                }
                step_json
            })
            .collect();

        let would_transition: Vec<serde_json::Value> = flow_result
            .entity_state_changes
            .iter()
            .map(|e| {
                serde_json::json!({
                    "entity_id": e.entity_id,
                    "instance_id": e.instance_id,
                    "from_state": e.from_state,
                    "to_state": e.to_state,
                })
            })
            .collect();

        serde_json::json!({
            "simulation": true,
            "flow_id": flow_id,
            "persona": persona_id,
            "outcome": flow_result.outcome,
            "path": path,
            "would_transition": would_transition,
            "verdicts": verdict_set.to_json()["verdicts"],
            "instance_bindings": instance_bindings,
        })
        .to_string()
    })
}

#[wasm_bindgen]
pub fn inspect_contract(handle: u32) -> String {
    with_contract(handle, |stored| {
        match inspect::build_inspect(&stored.bundle) {
            Ok(json) => json.to_string(),
            Err(e) => error_json(&format!("inspect error: {}", e)),
        }
    })
}

#[wasm_bindgen]
pub fn compute_action_space(
    handle: u32,
    facts_json: &str,
    entity_states_json: &str,
    persona_id: &str,
) -> String {
    let facts: serde_json::Value = match serde_json::from_str(facts_json) {
        Ok(v) => v,
        Err(e) => return error_json(&format!("invalid facts JSON: {}", e)),
    };

    let entity_states_val: serde_json::Value = match serde_json::from_str(entity_states_json) {
        Ok(v) => v,
        Err(e) => return error_json(&format!("invalid entity states JSON: {}", e)),
    };

    with_contract(handle, |stored| {
        let entity_states = match parse_entity_states(&entity_states_val) {
            Ok(s) => s,
            Err(e) => return error_json(&format!("invalid entity states: {}", e)),
        };

        let result = tenor_eval::action_space::compute_action_space(
            &stored.contract,
            &facts,
            &entity_states,
            persona_id,
        );

        match result {
            Ok(action_space) => match serde_json::to_string(&action_space) {
                Ok(json) => json,
                Err(e) => error_json(&format!("serialization error: {}", e)),
            },
            Err(e) => error_json(&format!("action space error: {}", e)),
        }
    })
}
